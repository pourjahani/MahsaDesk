pub mod config_gen;
pub mod core;
pub mod servers;
pub mod settings;
pub mod subs;
pub mod sysproxy;
pub mod tun;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};

use servers::ServerProfile;
use settings::Settings;
use subs::{Subscription, SubsFile};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(PartialEq, Clone, Debug)]
enum Mode {
    SystemProxy,
    Tun,
}

struct AppState {
    settings: Settings,
    servers: Vec<ServerProfile>,
    subs: SubsFile,
    mode: Option<Mode>,
    core_handle: Option<core::CoreHandle>,
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

    if let Some(handle) = st.core_handle.take() {
        handle.shutdown();
    }
    if st.mode.take().is_some() {
        let _ = sysproxy::disable_system_proxy();
    }
    // best-effort cleanup of a running elevated sing-box (experimental TUN)
    let _ = Command::new("taskkill")
        .args(["/IM", "sing-box.exe", "/F"])
        .output();
    Ok(())
}

fn connect_mode(app: &AppHandle, mode: Mode) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;

    let (settings, servers, logs) = {
        let arc = state(&app.state::<ManagedState>());
        let g = arc.lock().unwrap();
        (g.settings.clone(), g.servers.clone(), g.logs.clone())
    };

    let (_, server) = servers
        .iter()
        .map(|p| (p.id.clone(), clone_profile(p)))
        .find(|(id, _)| *id == settings.active_server)
        .ok_or_else(|| "no active server selected".to_string())?;

    // stop anything running first
    let _ = disconnect(app);

    let bin_dir = app
        .path()
        .resolve("binaries", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    let xray_path = bin_dir.join("xray.exe");
    if !xray_path.exists() {
        return Err("xray core binary not found in app binaries folder".into());
    }
    // put geo files into a writable dir so XRAY_LOCATION finds them after install
    let _ = copy_if_needed(&bin_dir, &dir);

    let handle = core::generate_and_start(
        dir.clone(),
        xray_path,
        &settings,
        &server,
        &logs,
        app,
    )?;

    let arc = state(&app.state::<ManagedState>());
    let mut g = arc.lock().unwrap();

    match mode {
        Mode::SystemProxy => {
            sysproxy::enable_system_proxy(settings.http_port)?;
        }
        Mode::Tun => {
            let singbox = bin_dir.join("sing-box.exe");
            if !singbox.exists() {
                let _ = disconnect(app);
                return Err("sing-box binary not found (TUN mode unavailable)".into());
            }
            let _ = &handle; // xray keeps running under the TUN
            if let Err(e) = tun::start_tun_elevated(&singbox, &dir, settings.socks_port) {
                let _ = disconnect(app);
                return Err(e);
            }
        }
    }

    g.core_handle = Some(handle);
    g.mode = Some(mode);
    Ok(())
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    if let Some(pos) = g.servers.iter().position(|p| p.id == dir_server.id) {
        g.servers[pos] = dir_server;
    } else {
        g.servers.push(dir_server);
    }
    save_servers(&dir, &g.servers)
}

#[tauri::command]
fn delete_server(app: AppHandle, st: State<ManagedState>, id: String) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    g.servers.retain(|p| p.id != id);
    if !g.servers.iter().any(|p| p.id == g.settings.active_server) {
        g.settings.active_server = g
            .servers
            .first()
            .map(|s| s.id.clone())
            .unwrap_or_default();
        let _ = g.settings.save(&dir);
    }
    save_servers(&dir, &g.servers)
}

#[tauri::command]
fn set_active(app: AppHandle, st: State<ManagedState>, id: String) -> Result<(), String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    g.settings.active_server = id;
    g.settings.save(&dir)
}

#[tauri::command]
fn import_links(
    app: AppHandle,
    st: State<ManagedState>,
    text: String,
    sub_id: Option<String>,
) -> Result<usize, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    let profiles = servers::parse_payload(&text);
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
fn test_all_delays(st: State<ManagedState>) -> Vec<DelayResult> {
    let targets: Vec<(String, String, u16)> = {
        let arc = state(&st);
        let g = arc.lock().unwrap();
        g.servers
            .iter()
            .map(|p| (p.id.clone(), p.address.clone(), p.port))
            .collect()
    };
    let arc = state(&st);
    let mut g = arc.lock().unwrap();
    let mut out = Vec::new();
    for (id, addr, port) in targets {
        let delay = tcp_ping(&addr, port);
        for p in g.servers.iter_mut() {
            if p.id == id {
                p.delay = delay;
            }
        }
        out.push(DelayResult { id, delay });
    }
    out
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
        running: g.core_handle.is_some(),
        mode: g.mode.as_ref().map(|m| match m {
            Mode::SystemProxy => "system".into(),
            Mode::Tun => "tun".into(),
        }),
        active: Some(g.settings.active_server.clone()),
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
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
    app.path()
        .app_config_dir()
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
                    g.core_handle.is_some()
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
        .setup(move |app| {
            let dir = app
                .path()
                .app_config_dir()
                .map_err(|e| format!("no app config dir: {}", e))?;
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
                logs: logs.clone(),
            }))));

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.set_focus();
            }

            build_tray(&app.handle()).map_err(|e| e.to_string())?;

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
            base64_decode
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
