use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

/// Generate a sing-box TUN config that forwards all traffic into the local
/// Xray SOCKS inbound, so TUN mode automatically works with whatever server
/// profile the Xray config uses. Scaffolded & experimental.
pub fn build_singbox_config(socks_port: u16) -> Value {
    json!({
        "log": {"level": "info"},
        "dns": {
            "servers": [{"tag": "proxy-dns", "address": "https://1.1.1.1/dns-query", "detour": "socks-out"}],
            "final": "proxy-dns",
            "strategy": "prefer_ipv4",
            "independent_cache": true
        },
        "inbounds": [{
            "type": "tun",
            "tag": "tun-in",
            "address": ["172.19.0.1/28"],
            "mtu": 1400,
            "auto_route": true,
            "strict_route": true,
            "stack": "system",
            "sniff": true
        }],
        "outbounds": [
            {"type": "socks", "tag": "socks-out", "server": "127.0.0.1", "server_port": socks_port, "version": "5"},
            {"type": "direct", "tag": "direct-out"},
            {"type": "dns", "tag": "dns-out"}
        ],
        "route": {
            "rules": [
                {"protocol": "dns", "outbound": "dns-out"},
                {"ip_is_private": true, "outbound": "direct-out"}
            ],
            "final": "socks-out",
            "auto_detect_interface": true
        }
    })
}

/// Write the sing-box config and start it elevated (UAC prompt). Best-effort.
pub fn start_tun_elevated(singbox_path: &PathBuf, workdir: &PathBuf, socks_port: u16) -> Result<(), String> {
    let cfg = build_singbox_config(socks_port);
    let cfg_path = workdir.join("singbox-tun.json");
    let json_text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    std::fs::write(&cfg_path, json_text).map_err(|e| format!("config write: {}", e))?;

    let exe = singbox_path
        .to_string_lossy()
        .replace('\\', "\\\\");
    let ps = format!(
        "Start-Process -FilePath '{}' -ArgumentList 'run','-c','{}','-D','{}' -Verb RunAs -WindowStyle Hidden",
        exe,
        cfg_path.to_string_lossy().replace('\\', "\\\\"),
        workdir.to_string_lossy().replace('\\', "\\\\")
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "TUN start failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// Kill a running elevated sing-box (UAC prompt on stop).
#[allow(dead_code)]
pub fn stop_tun_elevated() -> Result<(), String> {
    let ps = "Start-Process -FilePath 'taskkill.exe' -ArgumentList '/IM','sing-box.exe','/F' -Verb RunAs -WindowStyle Hidden";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", ps])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "TUN stop failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}
