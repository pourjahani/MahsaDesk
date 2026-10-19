use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::config_gen;
use crate::settings::Settings;
use crate::servers::ServerProfile;

pub const LOG_CAP: usize = 800;
pub const STATS_API_PORT: u16 = 10130;

pub struct CoreHandle {
    pub pid: u32,
    stop_flag: Arc<AtomicBool>,
    stats_thread: Option<JoinHandle<()>>,
}

impl CoreHandle {
    pub fn shutdown(mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        if let Some(t) = self.stats_thread.take() {
            let _ = t.join();
        }
        unsafe_terminate(self.pid);
    }
}

#[cfg(windows)]
fn unsafe_terminate(pid: u32) {
    let _ = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(windows))]
fn unsafe_terminate(pid: u32) {
    let _ = Command::new("kill").arg(pid.to_string()).status();
}

fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        PathBuf::from(format!("/proc/{}", pid)).exists()
    }
}

/// Spawn xray core with the generated config; returns the handle. Logs are pushed to `logs`.
pub fn start_core(
    xray_path: PathBuf,
    workdir: PathBuf,
    cfg: &Value,
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
) -> Result<CoreHandle, String> {
    let cfg_path = workdir.join("xray-config.json");
    let json_text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| format!("config write: {}", e))?;

    let mut cmd = Command::new(&xray_path);
    cmd.args(["run", "-c"])
        .arg(&cfg_path)
        .current_dir(&workdir)
        .env("XRAY_LOCATION", &workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child: Child = cmd.spawn().map_err(|e| format!("cannot start core: {}", e))?;
    let pid = child.id();

    if let Some(stdout) = child.stdout.take() {
        let logs = logs.clone();
        let area = area.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::<String, _>::ok) {
                push_log(&logs, &area, line);
            }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let logs = logs.clone();
        let area = area.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::<String, _>::ok) {
                push_log(&logs, &area, line);
            }
        });
    }
    push_log(logs, area, format!("[core] xray started, pid={}", pid));

    // stats polling thread
    let stop_flag = Arc::new(AtomicBool::new(false));
    let flag = stop_flag.clone();
    let area2 = area.clone();
    let stats_thread = std::thread::spawn(move || {
        loop {
            if flag.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_secs(2));
            if flag.load(Ordering::SeqCst) {
                break;
            }
            match query_stats() {
                Ok((up, down)) => {
                    let _ = area2.emit("traffic", json!({"up": up, "down": down}));
                }
                Err(_) => {
                    // core API may not be ready yet or stats disabled; back off
                    std::thread::sleep(Duration::from_secs(6));
                }
            }
        }
    });

    // verify the core is still alive after initial spin-up
    std::thread::sleep(Duration::from_millis(700));
    if child.try_wait().map(|o| o.is_some()).unwrap_or(false) || !process_alive(pid) {
        let handle = CoreHandle {
            pid,
            stop_flag,
            stats_thread: Some(stats_thread),
        };
        handle.shutdown();
        return Err("core exited immediately (check logs)".into());
    }

    Ok(CoreHandle {
        pid,
        stop_flag,
        stats_thread: Some(stats_thread),
    })
}

/// Query xray stats API via `xray api statsquery`; returns (uplink, downlink) totals in bytes.
fn query_stats() -> Result<(u64, u64), String> {
    let xray = locate_xray_guess();
    let out = Command::new(&xray)
        .args([
            "api",
            "statsquery",
            "--server",
            &format!("127.0.0.1:{}", STATS_API_PORT),
            "--pattern",
            "inbound>>>",
        ])
        .current_dir(xray.parent().unwrap_or(Path::new(".")))
        .env("XRAY_LOCATION", xray.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut up = 0u64;
    let mut down = 0u64;
    if let Some(stats) = v.pointer("/stat").and_then(|s| s.as_array()) {
        for s in stats {
            let name = s.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let val: u64 = s
                .get("value")
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            if name.contains(">>>uplink") {
                up += val;
            } else if name.contains(">>>downlink") {
                down += val;
            }
        }
    }
    Ok((up, down))
}

fn locate_xray_guess() -> PathBuf {
    if let Ok(loc) = std::env::var("XRAY_LOCATION") {
        let p = PathBuf::from(loc).join("xray.exe");
        if p.exists() {
            return p;
        }
    }
    PathBuf::from("xray.exe")
}

pub fn push_log(
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
    line: String,
) {
    {
        let mut q = logs.lock().unwrap();
        q.push_back(line.clone());
        while q.len() > LOG_CAP {
            q.pop_front();
        }
    }
    let _ = area.emit("core-log", json!({"line": line}));
}

pub fn generate_and_start(
    workdir: PathBuf,
    xray_path: PathBuf,
    settings: &Settings,
    server: &ServerProfile,
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
) -> Result<CoreHandle, String> {
    let cfg = config_gen::build_config(settings, server);
    start_core(xray_path, workdir, &cfg, logs, area)
}

// ---------------------------------------------------------------------------
// Test tools
// ---------------------------------------------------------------------------
const TEST_SOCKS_BASE: u16 = 18800;

/// Real URL test, same idea as v2rayNG's HttpTestService: spin up a throwaway
/// core instance for this single profile, then fetch http://www.gstatic.com/generate_204
/// through its SOCKS inbound and measure the time.
pub fn real_url_test(
    xray_path: &PathBuf,
    settings: &Settings,
    server: &ServerProfile,
    slot: usize,
) -> Result<u32, String> {
    let port = TEST_SOCKS_BASE + slot as u16;
    let mut s = settings.clone();
    s.socks_port = port;
    s.http_port = port + 1;
    s.loglevel = "none".into();
    s.proxy_sharing = false;
    let cfg = config_gen::build_config(&s, server);

    let tmp = std::env::temp_dir().join(format!("mahsatest-{}", slot));
    let _ = std::fs::create_dir_all(&tmp);
    let cfg_path = tmp.join("test-config.json");
    let json_text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| e.to_string())?;

    let mut child = Command::new(xray_path)
        .args(["run", "-c"])
        .arg(&cfg_path)
        .current_dir(&tmp)
        .env("XRAY_LOCATION", &tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;

    let res = (|| {
        // wait for the inbound port to open (max ~5s)
        let mut open = false;
        for _ in 0..50 {
            if child.try_wait().map(|o| o.is_some()).unwrap_or(false) {
                return Err("test core exited immediately".to_string());
            }
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                open = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !open {
            return Err("test core did not open its port".to_string());
        }
        let start = std::time::Instant::now();
        let proxy = reqwest::Proxy::all(&format!("socks5h://127.0.0.1:{}", port))
            .map_err(|e| e.to_string())?;
        let client = reqwest::blocking::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .get("http://www.gstatic.com/generate_204")
            .send()
            .map_err(|e| format!("request: {}", e))?;
        if !resp.status().is_success() && resp.status().as_u16() != 204 {
            return Err(format!("HTTP {}", resp.status()));
        }
        Ok(start.elapsed().as_millis() as u32)
    })();

    let _ = child.kill();
    res
}

/// Validate a generated config by booting the core briefly (headless).
pub fn validate_config(xray_path: &PathBuf, cfg: &Value, slot: usize) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("mahasacheck-{}", slot));
    let _ = std::fs::create_dir_all(&tmp);
    let cfg_path = tmp.join("check-config.json");
    let json_text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| e.to_string())?;
    let _ = copy_geo(&tmp);
    let mut child = Command::new(xray_path)
        .args(["run", "-c"])
        .arg(&cfg_path)
        .current_dir(&tmp)
        .env("XRAY_LOCATION", &tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut ok = false;
    let probe_port = cfg
        .pointer("/inbounds/0/port")
        .and_then(|p| p.as_u64())
        .unwrap_or(0) as u16;
    for _ in 0..25 {
        if child.try_wait().ok().flatten().is_some() {
            break;
        }
        if probe_port > 0 && std::net::TcpStream::connect(("127.0.0.1", probe_port)).is_ok() {
            ok = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    let _ = child.kill();
    if ok {
        Ok(())
    } else {
        Err("core refused the config (exit during startup)".into())
    }
}

fn copy_geo(dst: &PathBuf) -> Result<(), String> {
    if let Ok(loc) = std::env::var("XRAY_LOCATION") {
        for f in ["geoip.dat", "geosite.dat"] {
            let from = PathBuf::from(&loc).join(f);
            let to = dst.join(f);
            if from.exists() && !to.exists() {
                let _ = std::fs::copy(&from, &to);
            }
        }
    }
    Ok(())
}

/// List user-visible running processes (for per-app proxy rules).
pub fn list_processes() -> Vec<String> {
    #[cfg(windows)]
    {
        let out = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output();
        if let Ok(o) = out {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut names: Vec<String> = text
                .lines()
                .filter_map(|l| {
                    let stripped = l.trim().trim_start_matches('"');
                    let n = stripped.split_once('"').map(|(a, _)| a)?;
                    if n.is_empty() || n.eq_ignore_ascii_case("system") || n.eq_ignore_ascii_case("svchost.exe") {
                        None
                    } else {
                        Some(n.to_string())
                    }
                })
                .collect();
            names.sort();
            names.dedup();
            return names;
        }
        Vec::new()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}
