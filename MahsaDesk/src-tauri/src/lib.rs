pub mod config_gen;
pub mod core;
pub mod gate;
pub mod hy2;
pub mod isp;
pub mod ports;
pub mod psiphon;
pub mod proc;
pub mod requirements;
pub mod share;
pub mod servers;
pub mod settings;
pub mod subs;
pub mod sysproxy;
pub mod tools;
pub mod tun;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder};
use tauri::{AppHandle, Emitter, Manager, State};

use servers::ServerProfile;
use settings::Settings;
use subs::{Subscription, SubsFile};

fn binaries_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .resolve("binaries", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// True when this process runs as the portable build: CI renames the exe to
/// `MahsaDeskPortable.exe` and ships its `binaries/` folder next to it, so an
/// exe whose file stem ends in "portable" is that build (the installed app is
/// always `MahsaDesk.exe`).
fn is_portable_exe(path: &std::path::Path) -> bool {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_lowercase().ends_with("portable"))
        .unwrap_or(false)
}

/// Where settings, servers, subscriptions, generated configs and downloaded
/// geo data live.
///
/// The installed app and dev builds keep Tauri's profile directory
/// (`%APPDATA%\com.mahsadesk.desktop`). The portable build instead owns
/// `<exe dir>\data`, so extracting it next to an installed copy can never read
/// or overwrite that profile.
fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(dir) = portable_data_dir() {
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        return Ok(dir);
    }
    app.path().app_config_dir().map_err(|e| e.to_string())
}

/// `<exe dir>\data` when running the portable exe, `None` otherwise.
fn portable_data_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    if !is_portable_exe(&exe) {
        return None;
    }
    exe.parent().map(|d| d.join("data"))
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(PartialEq, Clone, Debug)]
enum Mode {
    SystemProxy,
    Tun,
    PsiphonOnly,
    PsiphonAfter,
}

struct AppState {
    settings: Settings,
    servers: Vec<ServerProfile>,
    subs: SubsFile,
    mode: Option<Mode>,
    core_handle: Option<core::CoreHandle>,
    psiphon_child: Option<std::process::Child>,
    /// Non-elevated sing-box hysteria2 sidecar (#7)
    hy2_child: Option<std::process::Child>,
    /// PID-aware per-app proxy gate for system-proxy mode (#20)
    gate: Option<gate::GateHandle>,
    /// Local subscription-share endpoint (#16); independent of connection state
    share: Option<share::ShareHandle>,
    logs: Arc<Mutex<VecDeque<String>>>,
}

struct ManagedState(pub Arc<Mutex<AppState>>);

fn state<'r>(s: &State<'r, ManagedState>) -> Arc<Mutex<AppState>> {
    s.0.clone()
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

const SERVERS_FILE: &str = "servers.json";

fn load_servers(dir: &PathBuf) -> Vec<ServerProfile> {
    let path = dir.join(SERVERS_FILE);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_servers(dir: &PathBuf, servers: &[ServerProfile]) -> Result<(), String> {
    let path = dir.join(SERVERS_FILE);
    let json = serde_json::to_string_pretty(servers).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Core lifecycle
// ---------------------------------------------------------------------------

fn disconnect(app: &AppHandle) -> Result<(), String> {
    let arc = state(&app.state::<ManagedState>());
    let mut st = arc.lock().unwrap();

    let had_psiphon = st.psiphon_child.is_some();
    if let Some(gate) = st.gate.take() {
        gate.shutdown();
    }
    if let Some(mut child) = st.psiphon_child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    if let Some(mut child) = st.hy2_child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let prev_mode = st.mode.take();
    if prev_mode.is_some() {
        let _ = sysproxy::disable_system_proxy();
    }
    if let Some(handle) = st.core_handle.take() {
        handle.shutdown();
    }
    // Targeted cleanup, only for engines this session actually ran (and always
    // without flashing a console window). Orphans from a previous run are
    // cleaned once at startup instead of on every disconnect.
    if prev_mode == Some(Mode::Tun) {
        // the elevated sing-box is not our child, so it needs taskkill
        let mut cmd = Command::new("taskkill");
        cmd.args(["/IM", "sing-box.exe", "/F"]);
        let _ = proc::output(cmd);
    }
    if had_psiphon
        || prev_mode == Some(Mode::PsiphonOnly)
        || prev_mode == Some(Mode::PsiphonAfter)
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/IM", psiphon::EXE_NAME, "/F"]);
        let _ = proc::output(cmd);
    }
    Ok(())
}

fn connect_mode(app: &AppHandle, mode: Mode) -> Result<(), String> {
    let dir = data_dir(app)?;

    let (mut settings, servers, logs) = {
        let arc = state(&app.state::<ManagedState>());
        let g = arc.lock().unwrap();
        (g.settings.clone(), g.servers.clone(), g.logs.clone())
    };

    // ---- Psiphon only: the tunnel-core engine runs alone, no xray core ----
    if mode == Mode::PsiphonOnly {
        let _ = disconnect(app);
        // #6: shift the psiphon pair off busy ports for this session
        let port_changes = ports::resolve_psiphon(&mut settings);
        report_ports(app, &logs, &port_changes);
        let bin_dir = app
            .path()
            .resolve("binaries", tauri::path::BaseDirectory::Resource)
            .map_err(|e| e.to_string())?;
        let exe = bin_dir.join(psiphon::EXE_NAME);
        if !exe.exists() {
            return Err(format!(
                "{} not found in app binaries folder (place it next to xray.exe)",
                psiphon::EXE_NAME
            ));
        }
        let workdir = dir.join("psiphon");
        let _ = std::fs::create_dir_all(&workdir);
        // optional user-supplied base config from their own provider
        let base_cfg = dir.join("psiphon-user.json");
        let cfg = psiphon::build_config(&settings, None, &workdir, &base_cfg);
        let mut child = psiphon::spawn_tunnel(&exe, &workdir, &cfg, &logs, app)?;
        if psiphon::wait_for_socks(settings.psiphon_socks_port).is_err() {
            let _ = child.kill();
            return Err("Psiphon tunnel did not come up in time (see logs)".into());
        }
        sysproxy::enable_system_proxy(settings.psiphon_http_port)?;
        let arc = state(&app.state::<ManagedState>());
        let mut g = arc.lock().unwrap();
        g.psiphon_child = Some(child);
        g.mode = Some(mode);
        return Ok(());
    }

    let (_, server) = servers
        .iter()
        .map(|p| (p.id.clone(), clone_profile(p)))
        .find(|(id, _)| *id == settings.active_server)
        .ok_or_else(|| "no active server selected".to_string())?;

    if server.protocol == "hysteria2" && mode == Mode::PsiphonAfter {
        return Err(
            "hysteria2 profiles don't work in Psiphon-after mode - use System or TUN mode"
                .into(),
        );
    }

    // stop anything running first
    let _ = disconnect(app);

    // #6: shift SOCKS/HTTP (and psiphon ports when needed) off busy ports for
    // this session only - stored settings are never rewritten
    let mut port_changes = ports::resolve_socks_http(&mut settings);
    if mode == Mode::PsiphonAfter {
        port_changes.extend(ports::resolve_psiphon(&mut settings));
    }
    report_ports(app, &logs, &port_changes);

    // #20: per-app rules in system-proxy mode run through the PID-aware gate.
    // The gate owns the public HTTP port; the core gets a private loopback one.
    // (With LAN proxy sharing on, the port must stay the core's, so no gate.)
    let mut gate_wanted =
        mode == Mode::SystemProxy && settings.per_app_mode != "off" && !settings.proxy_sharing;
    let mut xray_settings = settings.clone();
    let mut gate_upstream: Option<u16> = None;
    if gate_wanted {
        match ports::find_free(
            settings.http_port.saturating_add(10),
            &[settings.socks_port, settings.http_port, core::STATS_API_PORT],
        ) {
            Some(up) => {
                gate_upstream = Some(up);
                xray_settings.http_port = up;
            }
            None => {
                gate_wanted = false;
                core::push_log(
                    &logs,
                    app,
                    "[gate] no free upstream port - per-app rules skipped for this session".into(),
                );
            }
        }
    }

    let bin_dir = app
        .path()
        .resolve("binaries", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    // put geo files into a writable dir so XRAY_LOCATION finds them after install
    let _ = copy_if_needed(&bin_dir, &dir);

    // ---- #7 hysteria2 in system-proxy mode: non-elevated sing-box sidecar ----
    if server.protocol == "hysteria2" && mode == Mode::SystemProxy {
        let singbox = bin_dir.join("sing-box.exe");
        if !singbox.exists() {
            return Err("sing-box binary not found (hysteria2 sidecar unavailable)".into());
        }
        let mut child = hy2::spawn_sidecar(
            &singbox,
            &server,
            settings.socks_port,
            gate_upstream.unwrap_or(settings.http_port),
            &dir,
            &logs,
            app,
        )?;
        let setup = hy2::wait_for_socks(settings.socks_port).and_then(|_| {
            arm_system_proxy(
                app,
                &logs,
                gate_wanted,
                settings.http_port,
                gate_upstream,
                &settings.per_app_mode,
                &settings.per_app_list,
            )
        });
        if let Err(e) = setup {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        let arc = state(&app.state::<ManagedState>());
        let mut g = arc.lock().unwrap();
        g.hy2_child = Some(child);
        g.mode = Some(mode);
        return Ok(());
    }

    // Xray cannot speak hysteria2: only the TUN path remains, where sing-box
    // tunnels it natively - so for hy2 we simply skip starting xray.
    let mut handle: Option<core::CoreHandle> = None;
    if server.protocol != "hysteria2" {
        let xray_path = bin_dir.join("xray.exe");
        if !xray_path.exists() {
            return Err("xray core binary not found in app binaries folder".into());
        }
        handle = Some(core::generate_and_start(
            dir.clone(),
            xray_path,
            &xray_settings,
            &server,
            &logs,
            app,
        )?);
    }

    let arc = state(&app.state::<ManagedState>());
    let mut g = arc.lock().unwrap();

    match mode {
        Mode::SystemProxy => {
            arm_system_proxy(
                app,
                &logs,
                gate_wanted,
                settings.http_port,
                gate_upstream,
                &settings.per_app_mode,
                &settings.per_app_list,
            )?;
        }
        Mode::PsiphonAfter => {
            // xray keeps running for the inner leg; psiphon sits on top of it
            let exe = bin_dir.join(psiphon::EXE_NAME);
            if !exe.exists() {
                if let Some(h) = handle.take() {
                    h.shutdown();
                }
                let _ = disconnect(app);
                return Err(format!(
                    "{} not found in app binaries folder (place it next to xray.exe)",
                    psiphon::EXE_NAME
                ));
            }
            let workdir = dir.join("psiphon");
            let _ = std::fs::create_dir_all(&workdir);
            let base_cfg = dir.join("psiphon-user.json");
            let cfg = psiphon::build_config(&settings, Some(settings.socks_port), &workdir, &base_cfg);
            let mut child = psiphon::spawn_tunnel(&exe, &workdir, &cfg, &logs, app)?;
            if psiphon::wait_for_socks(settings.psiphon_socks_port).is_err() {
                let _ = child.kill();
                if let Some(h) = handle.take() {
                    h.shutdown();
                }
                return Err("Psiphon tunnel did not come up in time (see logs)".into());
            }
            sysproxy::enable_system_proxy(settings.psiphon_http_port)?;
            g.psiphon_child = Some(child);
        }
        Mode::PsiphonOnly => { unreachable!("handled above") }
        Mode::Tun => {
            let singbox = bin_dir.join("sing-box.exe");
            if !singbox.exists() {
                if let Some(h) = handle.take() {
                    h.shutdown();
                }
                let _ = disconnect(app);
                return Err("sing-box binary not found (TUN mode unavailable)".into());
            }
            let cfg = if server.protocol == "hysteria2" {
                // native sing-box outbound; xray was never started for hy2
                tun::build_singbox_config_native(
                    &server,
                    &settings.per_app_mode,
                    &settings.per_app_list,
                )
            } else {
                // xray keeps running under the TUN
                tun::build_singbox_config(
                    settings.socks_port,
                    &settings.per_app_mode,
                    &settings.per_app_list,
                )
            };
            if let Err(e) = tun::start_tun_elevated(&cfg, &singbox, &dir) {
                if let Some(h) = handle.take() {
                    h.shutdown();
                }
                let _ = disconnect(app);
                return Err(e);
            }
        }
    }

    g.core_handle = handle;
    g.mode = Some(mode);
    Ok(())
}

/// Log + emit port shifts so the UI can toast them (#6).
fn report_ports(app: &AppHandle, logs: &Arc<Mutex<VecDeque<String>>>, changes: &[(String, u16, u16)]) {
    if changes.is_empty() {
        return;
    }
    for (name, old, new) in changes {
        core::push_log(
            logs,
            app,
            format!(
                "[ports] {} port {} was busy - using {} for this session",
                name, old, new
            ),
        );
    }
    let _ = app.emit(
        "ports-shifted",
        serde_json::json!({ "changes": changes }),
    );
}

/// Start the per-app gate when its rules are enabled (#20) and point the
/// Windows system proxy at the right port: the gate when rules are on, the
/// core's HTTP port otherwise. Only a system-proxy failure is an error; a
/// gate that cannot start degrades to direct core access with a log line.
fn arm_system_proxy(
    app: &AppHandle,
    logs: &Arc<Mutex<VecDeque<String>>>,
    gate_wanted: bool,
    public_port: u16,
    upstream: Option<u16>,
    per_app_mode: &str,
    per_app_list: &str,
) -> Result<(), String> {
    let mut target = public_port;
    if gate_wanted {
        if let Some(up) = upstream {
            match gate::start(
                public_port,
                up,
                per_app_mode,
                per_app_list,
                logs,
                app,
            ) {
                Ok(handle) => {
                    let arc = state(&app.state::<ManagedState>());
                    arc.lock().unwrap().gate = Some(handle);
                }
                Err(e) => {
                    core::push_log(
                        logs,
                        app,
                        format!("[gate] disabled ({}); system proxy goes straight to the core", e),
                    );
                    target = up;
                }
            }
        }
    }
    sysproxy::enable_system_proxy(target)
}

fn clone_profile(p: &ServerProfile) -> ServerProfile {
    p.clone()
}

fn copy_if_needed(src_dir: &PathBuf, dst_dir: &PathBuf) -> Result<(), String> {
    for f in ["geoip.dat", "geosite.dat"] {
        let from = src_dir.join(f);
        let to = dst_dir.join(f);
        if from.exists() && !to.exists() {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct StatusInfo {
    running: bool,
    mode: Option<String>,
    active: Option<String>,
    psiphon_running: bool,
    hy2_running: bool,
}

#[tauri::command]
fn get_settings(st: State<ManagedState>) -> Settings {
    state(&st).lock().unwrap().settings.clone()
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    st: State<ManagedState>,
    new_settings: Settings,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    let old_autostart = g.settings.launch_at_login;
    let new_autostart = new_settings.launch_at_login;
    new_settings.save(&dir)?;
    g.settings = new_settings;
    drop(g);

    if new_autostart != old_autostart {
        let mgr = app.autolaunch();
        if new_autostart {
            let _ = mgr.enable();
        } else {
            let _ = mgr.disable();
        }
    }
    Ok(())
}

#[tauri::command]
fn list_servers(st: State<ManagedState>) -> Vec<ServerProfile> {
    state(&st).lock().unwrap().servers.clone()
}

#[tauri::command]
fn save_server(app: AppHandle, st: State<ManagedState>, dir_server: ServerProfile) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    if let Some(pos) = g.servers.iter().position(|p| p.id == dir_server.id) {
        g.servers[pos] = dir_server;
    } else {
        g.servers.push(dir_server);
    }
    save_servers(&dir, &g.servers)
}

/// Remove servers by id, keeping the active selection valid (#14 bulk path).
fn remove_servers(g: &mut AppState, dir: &PathBuf, ids: &[String]) -> Result<(), String> {
    g.servers.retain(|p| !ids.contains(&p.id));
    if !g.servers.iter().any(|p| p.id == g.settings.active_server) {
        g.settings.active_server = g
            .servers
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_default();
        let _ = g.settings.save(dir);
    }
    save_servers(dir, &g.servers)
}

#[tauri::command]
fn delete_server(app: AppHandle, st: State<ManagedState>, id: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    remove_servers(&mut g, &dir, std::slice::from_ref(&id))
}

/// Bulk delete (#14): one IPC call for the whole selection.
#[tauri::command]
fn delete_servers(app: AppHandle, st: State<ManagedState>, ids: Vec<String>) -> Result<(), String> {
    if ids.is_empty() {
        return Ok(());
    }
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    remove_servers(&mut g, &dir, &ids)
}

/// Share links for the selected ids, in selection order (#14 bulk export).
#[tauri::command]
fn export_links(st: State<ManagedState>, ids: Vec<String>) -> Vec<String> {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    g.servers
        .iter()
        .filter(|p| ids.contains(&p.id))
        .map(|p| servers::build_link(p))
        .collect()
}

// ---------------------------------------------------------------------------
// Subscription share endpoint (#16)
// ---------------------------------------------------------------------------

#[tauri::command]
fn share_server_start(
    app: AppHandle,
    st: State<ManagedState>,
    port: u16,
    lan: bool,
    ids: Option<Vec<String>>,
) -> Result<share::ShareInfo, String> {
    let arc = state(&st);
    if let Some(prev) = arc.lock().unwrap().share.take() {
        prev.shutdown();
    }
    let logs = { arc.lock().unwrap().logs.clone() };
    let st2 = arc.clone();
    // None/empty = serve the whole pool; otherwise only the selected ids
    // (#14 bulk share), so a selection can be shared without exposing the rest
    let scope = ids.filter(|v| !v.is_empty());
    let scoped = scope.clone();
    let payload: Arc<dyn Fn() -> String + Send + Sync> = Arc::new(move || {
        let g = st2.lock().unwrap();
        let joined: Vec<String> = g
            .servers
            .iter()
            .filter(|p| scoped.as_ref().map_or(true, |ids| ids.contains(&p.id)))
            .map(|p| servers::build_link(p))
            .collect();
        STANDARD.encode(joined.join("\n"))
    });
    let handle = share::start(port, lan, scope, payload, &logs, &app)?;
    let info = handle.info().clone();
    arc.lock().unwrap().share = Some(handle);
    Ok(info)
}

#[tauri::command]
fn share_server_stop(st: State<ManagedState>) {
    let arc = state(&st);
    let prev = arc.lock().unwrap().share.take();
    if let Some(h) = prev {
        h.shutdown();
    }
}

// ---------------------------------------------------------------------------
// Connection doctor (#21)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct DoctorCheck {
    id: String,
    ok: bool,
    detail: String,
}

struct DoctorSnapshot {
    settings: Settings,
    mode: Option<String>,
    running: bool,
}

fn socks_client(port: u16, timeout: std::time::Duration) -> Option<reqwest::blocking::Client> {
    let proxy = reqwest::Proxy::all(&format!("socks5h://127.0.0.1:{}", port)).ok()?;
    reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(timeout)
        .user_agent("MahsaDesk/1.0")
        .build()
        .ok()
}

fn plain_client(timeout: std::time::Duration) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent("MahsaDesk/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn doctor_work(snap: DoctorSnapshot) -> Vec<DoctorCheck> {
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    let mut checks: Vec<DoctorCheck> = Vec::with_capacity(6);

    // 1. engine alive
    checks.push(DoctorCheck {
        id: "engine".into(),
        ok: snap.running,
        detail: match &snap.mode {
            Some(m) => m.clone(),
            None => "not connected".into(),
        },
    });

    // ports the system proxy actually points at
    let psiphon_mode = snap.mode.as_deref().unwrap_or("").starts_with("psiphon");
    let socks_port = if psiphon_mode {
        snap.settings.psiphon_socks_port
    } else {
        snap.settings.socks_port
    };
    let http_port = if psiphon_mode {
        snap.settings.psiphon_http_port
    } else {
        snap.settings.http_port
    };

    // 2. socks listening
    let socks_ok = TcpStream::connect(("127.0.0.1", socks_port)).is_ok();
    checks.push(DoctorCheck {
        id: "socks".into(),
        ok: socks_ok,
        detail: format!("127.0.0.1:{}", socks_port),
    });

    // 3. http listening (what WinINET points at)
    let http_ok = TcpStream::connect(("127.0.0.1", http_port)).is_ok();
    checks.push(DoctorCheck {
        id: "http".into(),
        ok: http_ok,
        detail: format!("127.0.0.1:{}", http_port),
    });

    let tun = socks_client(socks_port, Duration::from_secs(12));

    // 4. real request through the tunnel + latency
    match &tun {
        Some(c) => {
            let start = Instant::now();
            let res = c
                .get("https://www.gstatic.com/generate_204")
                .send()
                .map(|r| r.status().as_u16());
            match res {
                Ok(code) if code == 204 || (200..400).contains(&code) => {
                    checks.push(DoctorCheck {
                        id: "tunnel".into(),
                        ok: true,
                        detail: format!("HTTP {} in {} ms", code, start.elapsed().as_millis()),
                    });
                }
                Ok(code) => checks.push(DoctorCheck {
                    id: "tunnel".into(),
                    ok: false,
                    detail: format!("HTTP {}", code),
                }),
                Err(e) => checks.push(DoctorCheck {
                    id: "tunnel".into(),
                    ok: false,
                    detail: e.to_string(),
                }),
            }
        }
        None => checks.push(DoctorCheck {
            id: "tunnel".into(),
            ok: false,
            detail: "socks client unavailable".into(),
        }),
    }

    // 5. egress IP: direct baseline vs through the tunnel (leak check)
    let fetch_ip = |client: &reqwest::blocking::Client| -> Option<String> {
        let r = client.get("https://api.ipify.org").send().ok()?;
        if !r.status().is_success() {
            return None;
        }
        let ip = r.text().ok()?.trim().to_string();
        if ip.is_empty() {
            None
        } else {
            Some(ip)
        }
    };
    let direct_ip = fetch_ip(&plain_client(Duration::from_secs(6)));
    let tun_ip = tun.as_ref().and_then(fetch_ip);
    let ip_ok = tun_ip.is_some() && direct_ip.as_deref() != tun_ip.as_deref();
    checks.push(DoctorCheck {
        id: "ip".into(),
        ok: ip_ok,
        detail: format!(
            "direct {} -> tunnel {}",
            direct_ip.as_deref().unwrap_or("?"),
            tun_ip.as_deref().unwrap_or("unreachable")
        ),
    });

    // 6. DNS: DoH through the tunnel vs the system resolver (poisoning/leak hint)
    let domain = "www.microsoft.com";
    let mut doh_ips: Vec<String> = Vec::new();
    if let Some(c) = &tun {
        if let Ok(r) = c
            .get("https://cloudflare-dns.com/dns-query")
            .query(&[("name", domain), ("type", "A")])
            .header("Accept", "application/dns-json")
            .send()
        {
            if let Ok(v) = r.json::<serde_json::Value>() {
                if let Some(answers) = v.get("Answer").and_then(|a| a.as_array()) {
                    for a in answers {
                        if a.get("type").and_then(|t| t.as_u64()) == Some(1) {
                            if let Some(data) = a.get("data").and_then(|d| d.as_str()) {
                                doh_ips.push(data.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    let system_ips: Vec<String> = std::net::ToSocketAddrs::to_socket_addrs(&(domain, 443))
        .map(|it| it.map(|a| a.ip().to_string()).collect())
        .unwrap_or_default();
    let dns_ok = !doh_ips.is_empty();
    checks.push(DoctorCheck {
        id: "dns".into(),
        ok: dns_ok,
        detail: format!(
            "via tunnel: {}; system: {}",
            if doh_ips.is_empty() { "failed".to_string() } else { doh_ips.join(", ") },
            if system_ips.is_empty() {
                "n/a".to_string()
            } else {
                system_ips.join(", ")
            }
        ),
    });

    checks
}

#[tauri::command]
async fn connection_doctor(st: State<'_, ManagedState>) -> Result<Vec<DoctorCheck>, String> {
    let snap = {
        let arc = state(&st);
        let g = arc.lock().unwrap();
        DoctorSnapshot {
            settings: g.settings.clone(),
            mode: g.mode.as_ref().map(|m| match m {
                Mode::SystemProxy => "system".into(),
                Mode::Tun => "tun".into(),
                Mode::PsiphonOnly => "psiphon-only".into(),
                Mode::PsiphonAfter => "psiphon-after".into(),
            }),
            running: g.mode.is_some() || g.core_handle.is_some(),
        }
    };
    tauri::async_runtime::spawn_blocking(move || doctor_work(snap))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn share_server_status(st: State<ManagedState>) -> Option<share::ShareInfo> {
    let arc = state(&st);
    let info = arc
        .lock()
        .unwrap()
        .share
        .as_ref()
        .map(|h| h.info().clone());
    info
}

#[tauri::command]
fn set_active(app: AppHandle, st: State<ManagedState>, id: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    g.settings.active_server = id;
    g.settings.save(&dir)
}

#[tauri::command]
async fn import_links(
    app: AppHandle,
    st: State<'_, ManagedState>,
    text: String,
    sub_id: Option<String>,
) -> Result<usize, String> {
    let trimmed = text.trim().to_string();
    // a lone http(s) URL is a subscription link: fetch its payload first
    let payload = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if trimmed.split_whitespace().count() != 1 {
            return Err("invalid share link".into());
        }
        subs::fetch_payload(&trimmed).await?
    } else {
        trimmed
    };
    let dir = data_dir(&app)?;
    let profiles = servers::parse_payload(&payload);
    let n = profiles.len();
    if n == 0 {
        return Err("no valid share links found".into());
    }
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    let subid = sub_id.unwrap_or_default();
    for mut p in profiles {
        p.sub_id = subid.clone();
        if p.remark.is_empty() {
            p.remark = p.address.clone();
        }
        g.servers.push(p);
    }
    if g.settings.active_server.is_empty() {
        g.settings.active_server = g.servers[0].id.clone();
        let _ = g.settings.save(&dir);
    }
    save_servers(&dir, &g.servers)?;
    Ok(n)
}

#[tauri::command]
async fn add_subscription(
    app: AppHandle,
    st: State<'_, ManagedState>,
    name: String,
    url: String,
    update_interval_hours: u32,
    update_now: bool,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let sub = Subscription {
        id: format!("sub-{}", chrono_secs()),
        name,
        url,
        enabled: true,
        update_interval_hours: update_interval_hours.max(1),
        last_update: None,
    };
    let arc = state(&st);
    {
        let mut g = arc.lock().unwrap();
        g.subs.subs.push(sub.clone());
        subs::save_subs(&dir, &g.subs)?;
    }
    if update_now {
        update_subscription_impl(&app, st, sub.id, dir).await?;
    }
    Ok(())
}

async fn update_subscription_impl(
    _app: &AppHandle,
    st: State<'_, ManagedState>,
    id: String,
    dir: PathBuf,
) -> Result<usize, String> {
    let url = {
        let arc = state(&st);
        let g = arc.lock().unwrap();
        g.subs
            .subs
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.url.clone())
            .ok_or("subscription not found")?
    };
    let payload = subs::fetch_payload(&url).await?;
    let profiles = servers::parse_payload(&payload);
    let n = profiles.len();
    if n == 0 {
        return Err("subscription returned no servers".into());
    }
    if !id.is_empty() {
        let arc = state(&st);
        let mut g = arc.lock().unwrap();
        g.servers.retain(|p| p.sub_id != id);
    }
    // dropping state lock before fs work
    let n_set = {
        let arc = state(&st);
        let mut g = arc.lock().unwrap();
        for mut p in profiles {
            p.sub_id = id.clone();
            if p.remark.is_empty() {
                p.remark = p.address.clone();
            }
            g.servers.push(p);
        }
        if !g.servers.iter().any(|p| p.id == g.settings.active_server) {
            g.settings.active_server = g
                .servers
                .first()
                .map(|s| s.id.clone())
                .unwrap_or_default();
        }
        save_servers(&dir, &g.servers)?;
        subs::save_subs(&dir, &g.subs)?;
        n
    };
    let _ = n_set;
    Ok(n)
}

#[tauri::command]
fn list_subscriptions(st: State<ManagedState>) -> Vec<Subscription> {
    state(&st).lock().unwrap().subs.subs.clone()
}

#[tauri::command]
fn edit_subscription(app: AppHandle, st: State<ManagedState>, sub: Subscription) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    for s in g.subs.subs.iter_mut() {
        if s.id == sub.id {
            *s = sub.clone();
        }
    }
    subs::save_subs(&dir, &g.subs)
}

#[tauri::command]
fn delete_subscription(
    app: AppHandle,
    st: State<ManagedState>,
    id: String,
) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    g.subs.subs.retain(|x| x.id != id);
    g.servers.retain(|p| p.sub_id != id);
    if !g.servers.iter().any(|p| p.id == g.settings.active_server) {
        g.settings.active_server = g
            .servers
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_default();
        let _ = g.settings.save(&dir);
    }
    save_servers(&dir, &g.servers)?;
    subs::save_subs(&dir, &g.subs)?;
    Ok(())
}

#[tauri::command]
async fn update_subscription(
    app: AppHandle,
    st: State<'_, ManagedState>,
    id: String,
) -> Result<usize, String> {
    let dir = data_dir(&app)?;
    update_subscription_impl(&app, st, id, dir).await
}

#[derive(Serialize)]
struct DelayResult {
    id: String,
    delay: Option<u32>,
}

#[tauri::command]
fn test_delay(st: State<ManagedState>, id: String) -> Option<u32> {
    let target = {
        let arc = state(&st);
        let g = arc.lock().unwrap();
        g.servers
            .iter()
            .find(|p| p.id == id)
            .map(|p| (p.address.clone(), p.port))?
    };
    let delay = tcp_ping(&target.0, target.1);
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    for p in g.servers.iter_mut() {
        if p.id == id {
            p.delay = delay;
        }
    }
    delay
}

#[tauri::command]
fn test_all_delays(st: State<ManagedState>, ids: Option<Vec<String>>) -> Vec<DelayResult> {
    let targets: Vec<(String, String, u16)> = {
        let arc = state(&st);
        let g = arc.lock().unwrap();
        g.servers
            .iter()
            .filter(|p| ids.as_ref().map_or(true, |sel| sel.contains(&p.id)))
            .map(|p| (p.id.clone(), p.address.clone(), p.port))
            .collect()
    };
    // parallel batches of 16 (was fully sequential) and no state lock held
    // while pings are in flight, so the UI keeps responding during the sweep
    let mut results: Vec<DelayResult> = Vec::with_capacity(targets.len());
    for chunk in targets.chunks(16) {
        std::thread::scope(|s| {
            let handles: Vec<_> = chunk
                .iter()
                .map(|(id, addr, port)| {
                    let id = id.clone();
                    let addr = addr.clone();
                    let port = *port;
                    s.spawn(move || DelayResult {
                        id,
                        delay: tcp_ping(&addr, port),
                    })
                })
                .collect();
            for h in handles {
                if let Ok(r) = h.join() {
                    results.push(r);
                }
            }
        });
    }
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    for r in &results {
        if let Some(p) = g.servers.iter_mut().find(|p| p.id == r.id) {
            p.delay = r.delay;
        }
    }
    results
}

fn tcp_ping(address: &str, port: u16) -> Option<u32> {
    use std::net::{TcpStream, ToSocketAddrs};
    use std::time::Instant;
    let addrs: Vec<std::net::SocketAddr> = if is_ip(address) {
        let ip: std::net::IpAddr = address.parse().ok()?;
        vec![std::net::SocketAddr::new(ip, port)]
    } else {
        (address, port).to_socket_addrs().ok()?.collect()
    };
    for a in addrs {
        let start = Instant::now();
        if TcpStream::connect_timeout(&a, std::time::Duration::from_secs(4)).is_ok() {
            return Some(start.elapsed().as_millis() as u32);
        }
    }
    None
}

fn is_ip(s: &str) -> bool {
    s.parse::<std::net::IpAddr>().is_ok()
}

#[tauri::command]
fn connect(app: AppHandle, mode: String) -> Result<(), String> {
    let mode = match mode.as_str() {
        "tun" => Mode::Tun,
        "psiphon-only" => Mode::PsiphonOnly,
        "psiphon-after" => Mode::PsiphonAfter,
        _ => Mode::SystemProxy,
    };
    connect_mode(&app, mode)
}

#[tauri::command]
fn disconnect_cmd(app: AppHandle) -> Result<(), String> {
    let _ = disconnect(&app);
    Ok(())
}

#[tauri::command]
fn status(st: State<ManagedState>) -> StatusInfo {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    StatusInfo {
        // mode is set on every successful connect (including the hy2 sidecar
        // and hy2 TUN paths where no xray CoreHandle exists)
        running: g.mode.is_some() || g.core_handle.is_some(),
        mode: g.mode.as_ref().map(|m| match m {
            Mode::SystemProxy => "system".into(),
            Mode::Tun => "tun".into(),
            Mode::PsiphonOnly => "psiphon-only".into(),
            Mode::PsiphonAfter => "psiphon-after".into(),
        }),
        active: Some(g.settings.active_server.clone()),
        psiphon_running: g.psiphon_child.is_some(),
        hy2_running: g.hy2_child.is_some(),
    }
}

#[tauri::command]
fn get_logs(st: State<ManagedState>) -> Vec<String> {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    let q = g
        .logs
        .lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<String>>();
    q
}

#[tauri::command]
fn clear_logs(st: State<ManagedState>) {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    g.logs.lock().unwrap().clear();
}

/// Rolling crash history written by the core monitor (#23).
#[tauri::command]
fn get_crash_history(app: AppHandle) -> Vec<serde_json::Value> {
    let Ok(dir) = data_dir(&app) else {
        return Vec::new();
    };
    std::fs::read_to_string(dir.join("crash-history.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Write the current log buffer to a file in Downloads and return its path
/// (#23: export to file; there is no fs plugin, so the backend owns the write).
#[tauri::command]
fn isp_presets() -> Vec<isp::IspPreset> {
    isp::presets()
}

/// Best-effort ISP guess from the IP-lookup response fields, for preset preselection (#22).
#[tauri::command]
fn detect_isp(isp: String, org: String) -> Option<String> {
    isp::detect_from_ipinfo(&isp, &org).map(|s| s.to_string())
}

#[tauri::command]
fn export_logs(text: String) -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "cannot locate home directory".to_string())?;
    let dir = PathBuf::from(&home).join("Downloads");
    let dir = if dir.is_dir() { dir } else { PathBuf::from(home) };
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let path = dir.join(format!("MahsaDesk-logs-{}.txt", secs));
    std::fs::write(&path, text).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
fn export_config(_app: AppHandle, st: State<ManagedState>) -> Result<String, String> {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    let payload = serde_json::json!({
        "servers": g.servers,
        "subs": g.subs.subs,
        "settings": g.settings
    });
    Ok(serde_json::to_string_pretty(&payload).unwrap_or_default())
}

#[tauri::command]
fn import_config(app: AppHandle, st: State<ManagedState>, json_text: String) -> Result<(), String> {
    let dir = data_dir(&app)?;
    let v: serde_json::Value =
        serde_json::from_str(&json_text).map_err(|e| format!("invalid JSON: {}", e))?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    if let Some(servers) = v
        .get("servers")
        .and_then(|s| serde_json::from_value::<Vec<ServerProfile>>(s.clone()).ok())
    {
        g.servers = servers;
    }
    if let Some(subs) = v
        .get("subs")
        .and_then(|s| serde_json::from_value::<Vec<Subscription>>(s.clone()).ok())
    {
        g.subs.subs = subs;
    }
    if let Some(settings_v) = v.get("settings") {
        let new_settings: Settings = serde_json::from_value(settings_v.clone())
            .map_err(|e| format!("invalid settings: {}", e))?;
        new_settings.save(&dir)?;
        g.settings = new_settings;
    }
    save_servers(&dir, &g.servers)?;
    subs::save_subs(&dir, &g.subs)?;
    Ok(())
}

#[tauri::command]
async fn download_geo(app: AppHandle) -> Result<(), String> {
    let dir = data_dir(&app)?;
    for (fname, url) in [
        (
            "geoip.dat",
            "https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geoip.dat",
        ),
        (
            "geosite.dat",
            "https://github.com/Loyalsoldier/v2ray-rules-dat/releases/latest/download/geosite.dat",
        ),
    ] {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {} while downloading {}", resp.status(), fname));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        std::fs::write(dir.join(fname), &bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn geo_files_exist(app: AppHandle) -> bool {
    data_dir(&app)
        .map(|dir| dir.join("geoip.dat").exists() && dir.join("geosite.dat").exists())
        .unwrap_or(false)
}

#[tauri::command]
fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn base64_encode(text: String) -> String {
    STANDARD.encode(text)
}

#[tauri::command]
fn base64_decode(text: String) -> Result<String, String> {
    let b = STANDARD.decode(text.trim()).map_err(|e| e.to_string())?;
    String::from_utf8(b).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Extra desktop tools (scan / share / test / assets / per-app)
// ---------------------------------------------------------------------------

#[tauri::command]
fn share_link(st: State<ManagedState>, id: String) -> Result<String, String> {
    let arc = state(&st);
    let g = arc.lock().unwrap();
    let p = g
        .servers
        .iter()
        .find(|p| p.id == id)
        .ok_or("server not found")?;
    Ok(servers::build_link(p))
}

fn locate_xray(app: &AppHandle) -> Result<PathBuf, String> {
    let bin_dir = app
        .path()
        .resolve("binaries", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    let p = bin_dir.join("xray.exe");
    if p.exists() {
        Ok(p)
    } else {
        Err("xray core binary not found".into())
    }
}

#[tauri::command]
fn run_real_test(app: AppHandle, st: State<ManagedState>, id: String) -> Result<Option<u32>, String> {
    let xray = locate_xray(&app)?;
    let arc = state(&st);
    let (settings, server) = {
        let g = arc.lock().unwrap();
        let p = g
            .servers
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .ok_or("server not found")?;
        (g.settings.clone(), p)
    };
    if server.protocol == "hysteria2" {
        return Err("hysteria2 real-test runs in the TUN engine; use TCP ping for it".into());
    }
    let delay = core::real_url_test(&xray, &settings, &server, 0).ok();
    let mut g = arc.lock().unwrap();
    for p in g.servers.iter_mut() {
        if p.id == id {
            p.delay = delay;
        }
    }
    Ok(delay)
}

#[tauri::command]
fn test_all_real(
    app: AppHandle,
    st: State<ManagedState>,
    ids: Option<Vec<String>>,
) -> Vec<DelayResult> {
    let xray = match locate_xray(&app) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let arc = state(&st);
    let (settings, targets) = {
        let g = arc.lock().unwrap();
        (
            g.settings.clone(),
            g.servers
                .iter()
                .filter(|p| p.protocol != "hysteria2")
                .filter(|p| ids.as_ref().map_or(true, |sel| sel.contains(&p.id)))
                .map(|p| (p.id.clone(), p.clone()))
                .collect::<Vec<_>>(),
        )
    };
    // batches of 8 concurrent throwaway cores
    let mut results = Vec::new();
    for chunk in targets.chunks(8) {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for (i, (id, server)) in chunk.iter().enumerate() {
                let x = xray.clone();
                let st2 = settings.clone();
                let (id, server) = (id.clone(), clone_profile(server));
                handles.push(s.spawn(move || {
                    let delay = core::real_url_test(&x, &st2, &server, i).ok();
                    (id, delay)
                }));
            }
            for h in handles {
                if let Ok((id, delay)) = h.join() {
                    results.push(DelayResult { id, delay });
                }
            }
        });
    }
    let mut g = arc.lock().unwrap();
    for r in &results {
        for p in g.servers.iter_mut() {
            if p.id == r.id {
                p.delay = r.delay;
            }
        }
    }
    results
}

#[tauri::command]
fn validate_servers_config(app: AppHandle, st: State<ManagedState>) -> Result<(), String> {
    let xray = locate_xray(&app)?;
    let arc = state(&st);
    let (settings, server) = {
        let g = arc.lock().unwrap();
        let p = g
            .servers
            .iter()
            .find(|p| p.id == g.settings.active_server)
            .cloned()
            .ok_or_else(|| "no active server selected".to_string())?;
        (g.settings.clone(), p)
    };
    let cfg = config_gen::build_config(&settings, &server);
    core::validate_config(&xray, &cfg, 0)
}

#[tauri::command]
async fn dns_scan_cmd(domain: String) -> Vec<tools::DnsResult> {
    tools::dns_scan(&domain).await
}

#[tauri::command]
async fn ip_scan_cmd(preset: String, custom: String, port: u16) -> Vec<tools::IpResult> {
    tauri::async_runtime::spawn_blocking(move || tools::ip_scan(&preset, &custom, port))
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn net_tool_cmd(kind: String, host: String) -> tools::NetToolOutput {
    let fallback = tools::NetToolOutput {
        ok: false,
        detail: "task failed".into(),
    };
    tauri::async_runtime::spawn_blocking(move || match kind.as_str() {
        "trace" => tools::net_traceroute(&host),
        "dns" => tools::net_dns_lookup(&host),
        _ => tools::net_ping(&host),
    })
    .await
    .unwrap_or(fallback)
}

#[tauri::command]
async fn port_check_cmd(host: String, port: u16) -> tools::PortCheckResult {
    tauri::async_runtime::spawn_blocking(move || tools::net_port_check(&host, port))
        .await
        .unwrap_or(tools::PortCheckResult {
            open: false,
            delay: None,
        })
}

#[tauri::command]
async fn my_ip_cmd() -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || tools::my_ip_info())
        .await
        .unwrap_or_else(|e| Err(e.to_string()))
}

#[tauri::command]
async fn net_speed_cmd(bytes: u64) -> Result<f64, String> {
    tauri::async_runtime::spawn_blocking(move || tools::net_speed_test(bytes))
        .await
        .unwrap_or_else(|e| Err(e.to_string()))
}

#[tauri::command]
async fn requirements_status(app: AppHandle, remote: Option<bool>) -> Vec<requirements::ReqInfo> {
    match binaries_dir(&app) {
        // remote=false keeps the local/boot check free of GitHub API calls
        Ok(dir) => requirements::status_all(&dir, remote.unwrap_or(true)).await,
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
async fn requirements_upgrade(app: AppHandle, key: String) -> Result<requirements::ReqInfo, String> {
    // engine files are locked while the cores run -> refuse politely
    let running = {
        let arc = state(&app.state::<ManagedState>());
        let g = arc.lock().unwrap();
        g.core_handle.is_some() || g.psiphon_child.is_some() || g.hy2_child.is_some()
    };
    if running {
        return Err("disconnect MahsaDesk before updating engine files".into());
    }
    let dir = binaries_dir(&app)?;
    requirements::upgrade(&dir, &key).await
}

#[tauri::command]
async fn app_update_check() -> requirements::AppUpdateInfo {
    requirements::check_app_update(env!("CARGO_PKG_VERSION")).await
}

#[tauri::command]
fn list_running_processes() -> Vec<String> {
    core::list_processes()
}

#[derive(Serialize)]
struct AssetInfo {
    name: String,
    exists: bool,
    size: u64,
}

#[tauri::command]
fn assets_info(app: AppHandle) -> Vec<AssetInfo> {
    let mut out = Vec::new();
    if let Ok(dir) = data_dir(&app) {
        for f in ["geoip.dat", "geosite.dat"] {
            let p = dir.join(f);
            out.push(AssetInfo {
                name: f.into(),
                exists: p.exists(),
                size: std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0),
            });
        }
    }
    out
}

#[tauri::command]
fn delete_geo(app: AppHandle, name: String) -> Result<(), String> {
    if name != "geoip.dat" && name != "geosite.dat" {
        return Err("bad asset name".into());
    }
    let dir = data_dir(&app)?;
    let p = dir.join(&name);
    if p.exists() {
        std::fs::remove_file(&p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn chrono_secs() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}

// ---------------------------------------------------------------------------
// Tray
// ---------------------------------------------------------------------------

fn build_tray(app: &AppHandle) -> Result<(), tauri::Error> {
    let toggle = MenuItem::with_id(app, "toggle", "Connect / Disconnect", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open MahsaDesk", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &sep1, &toggle, &sep2, &quit])?;

    let icon = tauri::include_image!("icons/32x32.png");
    let _tray = TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "toggle" => {
                let running = {
                    let st = state(&app.state::<ManagedState>());
                    let g = st.lock().unwrap();
                    g.mode.is_some() || g.core_handle.is_some()
                };
                if running {
                    let _ = disconnect(app);
                } else {
                    let _ = connect_mode(app, Mode::SystemProxy);
                }
            }
            "open" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }
            "quit" => {
                let _ = disconnect(app);
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let tauri::tray::TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Run
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let logs: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_deep_link::init())
        .setup(move |app| {
            let dir = data_dir(app.handle())
                .map_err(|e| format!("no app data dir: {}", e))?;
            std::fs::create_dir_all(&dir).ok();

            let settings = Settings::load(&dir);
            let servers = load_servers(&dir);
            let subs = subs::load_subs(&dir);

            app.manage(ManagedState(Arc::new(Mutex::new(AppState {
                settings,
                servers,
                subs,
                mode: None,
                core_handle: None,
                psiphon_child: None,
                hy2_child: None,
                gate: None,
                share: None,
                logs: logs.clone(),
            }))));

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }

            build_tray(&app.handle()).map_err(|e| e.to_string())?;

            // clean up engines orphaned by a previous run (once, hidden)
            for image in ["sing-box.exe", psiphon::EXE_NAME] {
                let mut cmd = Command::new("taskkill");
                cmd.args(["/IM", image, "/F"]);
                let _ = proc::output(cmd);
            }

            // auto connect if configured
            let auto = {
                let st = app.state::<ManagedState>();
                let g = st.0.lock().unwrap();
                g.settings.auto_connect && !g.servers.is_empty()
            };
            if auto {
                let h = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let _ = connect_mode(&h, Mode::SystemProxy);
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let minimize_to_tray = {
                    let st = app.state::<ManagedState>();
                    let g = st.0.lock().unwrap();
                    g.settings.minimize_to_tray
                };
                if minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    let _ = disconnect(&app.clone());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            list_servers,
            save_server,
            delete_server,
            delete_servers,
            export_links,
            share_server_start,
            share_server_stop,
            share_server_status,
            connection_doctor,
            get_crash_history,
            export_logs,
            isp_presets,
            detect_isp,
            set_active,
            import_links,
            add_subscription,
            list_subscriptions,
            edit_subscription,
            delete_subscription,
            update_subscription,
            test_delay,
            test_all_delays,
            connect,
            disconnect_cmd,
            status,
            get_logs,
            clear_logs,
            export_config,
            import_config,
            download_geo,
            geo_files_exist,
            app_version,
            base64_encode,
            base64_decode,
            share_link,
            run_real_test,
            test_all_real,
            validate_servers_config,
            list_running_processes,
            assets_info,
            delete_geo,
            dns_scan_cmd,
            ip_scan_cmd,
            net_tool_cmd,
            port_check_cmd,
            my_ip_cmd,
            net_speed_cmd,
            requirements_status,
            requirements_upgrade,
            app_update_check
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::is_portable_exe;
    use std::path::Path;

    #[test]
    fn portable_detection_keys_off_the_exe_name() {
        // what CI ships
        assert!(is_portable_exe(Path::new(
            r"C:\Users\x\Desktop\MahsaDesk\MahsaDeskPortable.exe"
        )));
        assert!(is_portable_exe(Path::new("MahsaDeskPORTABLE.exe")));
        // installed app and dev binary keep the shared profile
        assert!(!is_portable_exe(Path::new(
            r"C:\Program Files\MahsaDesk\MahsaDesk.exe"
        )));
        assert!(!is_portable_exe(Path::new(r"target\release\mahsadesk.exe")));
    }
}
