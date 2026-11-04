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
/// A failed read must not block the disable - ProxyEnable=0 is what matters.
pub fn disable_system_proxy() -> Result<(), String> {
    let mut sp = Sysproxy::get_system_proxy().unwrap_or(Sysproxy {
        enable: false,
        host: "127.0.0.1".into(),
        port: 0,
        bypass: "localhost;127.*;10.*;172.16.*;192.168.*;<local>".into(),
    });
    sp.enable = false;
    sp.set_system_proxy().map_err(|e| e.to_string())
}

fn is_loopback_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "localhost" || host == "::1"
}

/// Clear the system proxy ONLY when it points at one of this app's own
/// loopback inbounds (its HTTP or psiphon HTTP port). A left-over from a
/// killed/crashed/failed-disconnect session blackholes the user's normal
/// traffic; a proxy set by other software must survive untouched.
pub fn clear_own_leftover(http_port: u16, psiphon_http_port: u16) -> bool {
    let Ok(sp) = Sysproxy::get_system_proxy() else {
        return false;
    };
    if sp.enable
        && is_loopback_host(&sp.host)
        && (sp.port == http_port || sp.port == psiphon_http_port)
    {
        return disable_system_proxy().is_ok();
    }
    false
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
