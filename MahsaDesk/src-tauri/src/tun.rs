use crate::proc;
use crate::servers::ServerProfile;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;

fn per_app_rules(mode: &str, list: &str) -> Vec<Value> {
    let names: Vec<String> = list
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.trim_end_matches(".exe").to_string())
        .collect();
    if names.is_empty() || mode == "off" {
        return Vec::new();
    }
    let outbound = if mode == "bypass" { "direct-out" } else { "socks-out" };
    vec![json!({
        "process_name": names,
        "outbound": outbound,
        "inbound": ["tun-in"]
    })]
}

/// Generate a sing-box TUN config that forwards all traffic into the local
/// Xray SOCKS inbound, so TUN mode automatically works with whatever server
/// profile the Xray config uses. Scaffolded & experimental.
pub fn build_singbox_config(socks_port: u16, per_app_mode: &str, per_app_list: &str) -> Value {
    let mut rules = per_app_rules(per_app_mode, per_app_list);
    rules.push(json!({"protocol": "dns", "outbound": "dns-out"}));
    rules.push(json!({"ip_is_private": true, "outbound": "direct-out"}));
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
            "rules": rules,
            "final": "socks-out",
            "auto_detect_interface": true
        }
    })
}

/// Full TUN config for a hysteria2 profile: sing-box handles the tunnel itself.
pub fn build_singbox_config_native(server: &ServerProfile, per_app_mode: &str, per_app_list: &str) -> Value {
    let mut rules = per_app_rules(per_app_mode, per_app_list);
    rules.push(json!({"protocol": "dns", "outbound": "dns-out"}));
    rules.push(json!({"ip_is_private": true, "outbound": "direct-out"}));
    let outbound = crate::hy2::hysteria2_outbound(server, "socks-out");
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
            outbound,
            {"type": "direct", "tag": "direct-out"},
            {"type": "dns", "tag": "dns-out"}
        ],
        "route": {
            "rules": rules,
            "final": "socks-out",
            "auto_detect_interface": true
        }
    })
}

/// Write the sing-box config and start it elevated (UAC prompt). Best-effort.
pub fn start_tun_elevated(config: &Value, singbox_path: &PathBuf, workdir: &PathBuf) -> Result<(), String> {
    let cfg = config;
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
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps]);
    proc::hide(&mut cmd);
    let out = cmd.output().map_err(|e| e.to_string())?;
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
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", ps]);
    proc::hide(&mut cmd);
    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "TUN stop failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}
