use sysproxy::Sysproxy;

/// Enable the Windows system proxy to point at our local HTTP inbound.
pub fn enable_system_proxy(http_port: u16) -> Result<(), String> {
    let sp = Sysproxy {
        enable: true,
        host: "127.0.0.1".to_string(),
        port: http_port,
        bypass: "localhost;127.*;10.*;172.16.*;192.168.*;<local>".into(),
    };
    sp.set_system_proxy().map_err(|e| e.to_string())
}

/// Disable the system proxy (does not clear residual host configuration).
pub fn disable_system_proxy() -> Result<(), String> {
    let mut sp = Sysproxy::get_system_proxy().map_err(|e| e.to_string())?;
    sp.enable = false;
    sp.set_system_proxy().map_err(|e| e.to_string())
}

#[allow(dead_code)]
pub fn is_system_proxy_enabled() -> bool {
    Sysproxy::get_system_proxy().map(|p| p.enable).unwrap_or(false)
}

/// Preserve a snapshot of proxy settings before we override them.
#[allow(dead_code)]
pub fn save_snapshot() -> Option<Sysproxy> {
    Sysproxy::get_system_proxy().ok()
}

// simple passthrough retained for future use
