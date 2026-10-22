//! Hysteria2 sidecar (#7).
//!
//! Xray cannot speak hysteria2, so the app used to hard-refuse hy2 profiles
//! in system-proxy mode and forced users into TUN (which needs admin rights).
//! This module runs sing-box as a plain, non-elevated child with ordinary
//! SOCKS/HTTP inbounds — the system proxy then points at it exactly like the
//! xray core, no UAC prompt, no TUN driver.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::AppHandle;

use crate::core::push_log;
use crate::proc;
use crate::servers::ServerProfile;

/// Test ports for throwaway hy2 instances (xray tests use 18800+).
const TEST_SOCKS_BASE: u16 = 18850;

/// The hysteria2 outbound shared by the TUN config and the sidecar config.
pub fn hysteria2_outbound(server: &ServerProfile, tag: &str) -> Value {
    let mut outbound = json!({
        "type": "hysteria2",
        "tag": tag,
        "server": server.address,
        "server_port": server.port,
        "password": server.auth
    });
    if !server.sni.is_empty() {
        outbound["tls"] = json!({ "enabled": true, "server_name": server.sni, "insecure": true });
    } else {
        outbound["tls"] = json!({ "enabled": true, "insecure": true });
    }
    if !server.method.is_empty() {
        // MahsaNG convention: `method` non-empty marks an obfs password
        outbound["obfs"] = json!({ "type": "salamander", "password": server.encryption });
    }
    outbound
}

/// Sidecar config: SOCKS + HTTP inbounds on the normal app ports, hysteria2
/// outbound, DNS through the tunnel. No TUN inbound => runs unelevated.
pub fn build_sidecar_config(server: &ServerProfile, socks_port: u16, http_port: u16) -> Value {
    json!({
        "log": { "level": "info" },
        "dns": {
            "servers": [{ "tag": "proxy-dns", "address": "https://1.1.1.1/dns-query", "detour": "proxy-out" }],
            "final": "proxy-dns",
            "strategy": "prefer_ipv4",
            "independent_cache": true
        },
        "inbounds": [
            { "type": "socks", "tag": "socks-in", "listen": "127.0.0.1", "listen_port": socks_port },
            { "type": "http", "tag": "http-in", "listen": "127.0.0.1", "listen_port": http_port }
        ],
        "outbounds": [
            hysteria2_outbound(server, "proxy-out"),
            { "type": "direct", "tag": "direct-out" },
            { "type": "dns", "tag": "dns-out" }
        ],
        "route": {
            "rules": [
                { "protocol": "dns", "outbound": "dns-out" },
                { "ip_is_private": true, "outbound": "direct-out" }
            ],
            "final": "proxy-out"
        }
    })
}

fn write_and_spawn(
    singbox_path: &PathBuf,
    cfg: &Value,
    cfg_name: &str,
    workdir: &PathBuf,
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
    log_tag: &str,
) -> Result<Child, String> {
    let cfg_path = workdir.join(cfg_name);
    let json_text = serde_json::to_string_pretty(cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| format!("config write: {}", e))?;

    let mut cmd = Command::new(singbox_path);
    cmd.args(["run", "-c"])
        .arg(&cfg_path)
        .current_dir(workdir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    proc::hide(&mut cmd);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("cannot start sing-box: {}", e))?;
    let pid = child.id();

    let prefix = format!("[{}] ", log_tag);
    if let Some(stream) = child.stdout.take() {
        pump(stream, logs.clone(), area.clone(), prefix.clone());
    }
    if let Some(stream) = child.stderr.take() {
        pump(stream, logs.clone(), area.clone(), prefix.clone());
    }
    push_log(
        logs,
        area,
        format!("[{}] sing-box started, pid={}", log_tag, pid),
    );
    Ok(child)
}

fn pump<R: std::io::Read + Send + 'static>(
    stream: R,
    logs: Arc<Mutex<VecDeque<String>>>,
    area: AppHandle,
    prefix: String,
) {
    std::thread::spawn(move || {
        let reader = BufReader::new(stream);
        for line in reader.lines().map_while(Result::<String, _>::ok) {
            push_log(&logs, &area, format!("{}{}", prefix, line));
        }
    });
}

/// Start the non-elevated hysteria2 sidecar for system-proxy mode.
pub fn spawn_sidecar(
    singbox_path: &PathBuf,
    server: &ServerProfile,
    socks_port: u16,
    http_port: u16,
    workdir: &PathBuf,
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
) -> Result<Child, String> {
    let cfg = build_sidecar_config(server, socks_port, http_port);
    write_and_spawn(
        singbox_path,
        &cfg,
        "singbox-hy2.json",
        workdir,
        logs,
        area,
        "hy2",
    )
}

/// Block until a local SOCKS port accepts connections (max ~12 s).
pub fn wait_for_socks(socks_port: u16) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", socks_port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(120));
    }
    Err("hysteria2 sidecar did not open its SOCKS port (see logs)".into())
}

/// Real URL test for a hysteria2 profile: boot a throwaway sidecar on a test
/// port and fetch a 204 through it, measuring elapsed time — the hy2 twin of
/// `core::real_url_test`.
pub fn real_test(
    singbox_path: &PathBuf,
    server: &ServerProfile,
    slot: usize,
) -> Result<u32, String> {
    let port = TEST_SOCKS_BASE + slot as u16;
    let cfg = json!({
        "log": { "level": "warn" },
        "inbounds": [
            { "type": "socks", "tag": "in", "listen": "127.0.0.1", "listen_port": port }
        ],
        "outbounds": [ hysteria2_outbound(server, "proxy-out") ],
        "route": { "final": "proxy-out" }
    });
    let tmp = std::env::temp_dir().join(format!("mahsahy2test-{}", slot));
    let _ = std::fs::create_dir_all(&tmp);
    let cfg_path = tmp.join("test-config.json");
    let json_text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(singbox_path);
    cmd.args(["run", "-c"])
        .arg(&cfg_path)
        .current_dir(&tmp)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    proc::hide(&mut cmd);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;

    let res = (|| {
        let mut open = false;
        for _ in 0..60 {
            if child.try_wait().map(|o| o.is_some()).unwrap_or(false) {
                return Err("hy2 test core exited immediately".to_string());
            }
            if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                open = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if !open {
            return Err("hy2 test core did not open its port".to_string());
        }
        let start = Instant::now();
        let proxy = reqwest::Proxy::all(&format!("socks5h://127.0.0.1:{}", port))
            .map_err(|e| e.to_string())?;
        let client = reqwest::blocking::Client::builder()
            .proxy(proxy)
            .timeout(Duration::from_secs(12))
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
