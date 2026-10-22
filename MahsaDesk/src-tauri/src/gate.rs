//! Per-app routing for system-proxy mode (#20) — the PID-aware gate.
//!
//! Windows has no per-app system proxy: WinINET settings are machine-wide,
//! and a WFP/WinDivert driver would need admin rights plus a kernel module.
//! Instead, this module owns the port the system proxy points at. For every
//! incoming HTTP/CONNECT request it identifies the *client process* by
//! looking its connection up in the Windows TCP table
//! (`GetExtendedTcpTable`, owner-PID variant), maps the PID to an executable
//! name, and applies the same `per_app_mode` / `per_app_list` rules the TUN
//! view already edits:
//!
//!   off      -> everything tunnels (today's behavior)
//!   bypass   -> listed processes connect directly, the rest tunnel
//!   allow    -> only listed processes tunnel, the rest go direct
//!
//! Proxied traffic is forwarded untouched to Xray's HTTP inbound (which is
//! moved to a private loopback port while the gate is up); direct traffic is
//! dialled locally. No admin rights, no driver, transparent to every app that
//! honors the system proxy.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::core::push_log;
use tauri::AppHandle;

pub struct GateHandle {
    stop: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

impl GateHandle {
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.accept_thread.take() {
            let _ = t.join();
        }
    }
}

// ---------------------------------------------------------------------------
// Policy (pure, unit-tested)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Route {
    Direct,
    Proxy,
}

/// Lowercase exe name without extension: "Chrome.EXE" == "chrome".
pub fn normalize_proc(name: &str) -> String {
    name.trim()
        .to_ascii_lowercase()
        .trim_end_matches(".exe")
        .to_string()
}

/// Decide where one process's traffic goes. Unknown process names count as
/// not-listed, which fails *into the tunnel* for bypass mode and *out of it*
/// for allow mode — matching what each mode promises the user.
pub fn route_for(mode: &str, list: &str, proc: &str) -> Route {
    let p = normalize_proc(proc);
    let listed = list
        .lines()
        .map(normalize_proc)
        .filter(|l| !l.is_empty())
        .any(|l| l == p && !p.is_empty());
    match mode {
        "bypass" => {
            if listed {
                Route::Direct
            } else {
                Route::Proxy
            }
        }
        "allow" => {
            if listed {
                Route::Proxy
            } else {
                Route::Direct
            }
        }
        _ => Route::Proxy,
    }
}

// ---------------------------------------------------------------------------
// Request head parsing (pure, unit-tested)
// ---------------------------------------------------------------------------

pub enum ReqKind {
    Connect,
    Forward,
}

pub struct ReqHead {
    pub kind: ReqKind,
    pub host: String,
    pub port: u16,
    /// Bytes to forward upstream (request line rewritten to origin-form when
    /// the client used absolute-form against a direct target).
    pub out: Vec<u8>,
}

fn split_host_port(s: &str, default_port: u16) -> Option<(String, u16)> {
    let s = s.trim();
    if let Some(rest) = s.strip_prefix('[') {
        // IPv6 literal: [::1]:8080
        let end = rest.find(']')?;
        let host = &rest[..end];
        let after = &rest[end + 1..];
        let port = if let Some(p) = after.strip_prefix(':') {
            p.parse().ok()?
        } else {
            default_port
        };
        return Some((host.to_string(), port));
    }
    match s.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            Some((h.to_string(), p.parse().ok()?))
        }
        Some(_) => None,
        None if default_port > 0 && !s.is_empty() => Some((s.to_string(), default_port)),
        None => None,
    }
}

/// Parse the first request of a proxied connection.
pub fn parse_head(raw: &[u8]) -> Option<ReqHead> {
    let first_end = raw
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(raw.len());
    let first = String::from_utf8_lossy(&raw[..first_end]);
    let mut it = first.split_whitespace();
    let method = it.next()?;
    let target = it.next()?;
    let version = it.next().unwrap_or("HTTP/1.1");

    if method.eq_ignore_ascii_case("CONNECT") {
        // CONNECT always carries authority-form host:port
        let (host, port) = split_host_port(target, 0)?;
        return Some(ReqHead {
            kind: ReqKind::Connect,
            host,
            port,
            out: raw.to_vec(),
        });
    }

    if let Some(rest) = target.strip_prefix("http://") {
        // absolute-form (what browsers send to a proxy): convert to
        // origin-form so a *direct* origin server accepts it
        let (authority, path) = match rest.find('/') {
            Some(i) => (&rest[..i], &rest[i..]),
            None => (rest, "/"),
        };
        let (host, port) = split_host_port(authority, 80)?;
        let new_first = format!("{} {} {}", method, path, version);
        let mut out = new_first.into_bytes();
        out.extend_from_slice(&raw[first_end..]);
        return Some(ReqHead {
            kind: ReqKind::Forward,
            host,
            port,
            out,
        });
    }

    // origin-form: take the authority from the Host header
    let text = String::from_utf8_lossy(raw);
    let host_hdr = text
        .lines()
        .map(str::trim)
        .find_map(|l| l.strip_prefix("Host:").or_else(|| l.strip_prefix("host:")))
        .map(str::trim)?;
    let (host, port) = split_host_port(host_hdr, 80)?;
    Some(ReqHead {
        kind: ReqKind::Forward,
        host,
        port,
        out: raw.to_vec(),
    })
}

// ---------------------------------------------------------------------------
// Windows: connection -> owning PID -> process name
// ---------------------------------------------------------------------------

#[cfg(windows)]
#[link(name = "iphlpapi")]
extern "system" {
    fn GetExtendedTcpTable(
        p_tcp_table: *mut core::ffi::c_void,
        pdw_size: *mut u32,
        b_order: i32,
        ul_af: u32,
        table_class: u32,
        reserved: u32,
    ) -> u32;
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn OpenProcess(
        dw_desired_access: u32,
        b_inherit_handle: i32,
        dw_process_id: u32,
    ) -> *mut core::ffi::c_void;
    fn QueryFullProcessImageNameW(
        h_process: *mut core::ffi::c_void,
        dw_flags: u32,
        lp_exe_name: *mut u16,
        lpdw_size: *mut u32,
    ) -> i32;
    fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
}

/// Owning PID of the ESTABLISHED connection `client_ephemeral_port -> gate_port`.
#[cfg(windows)]
fn pid_for_connection(client_ephemeral_port: u16, gate_port: u16) -> Option<u32> {
    use std::ptr;
    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_ALL: u32 = 5;
    unsafe {
        let mut size: u32 = 0;
        // first call: query required buffer size
        GetExtendedTcpTable(
            ptr::null_mut(),
            &mut size,
            0,
            AF_INET,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
        if size == 0 {
            return None;
        }
        let mut buf: Vec<u8> = vec![0u8; size as usize + 64];
        let ret = GetExtendedTcpTable(
            buf.as_mut_ptr() as *mut _,
            &mut size,
            0,
            AF_INET,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        );
        if ret != 0 {
            return None;
        }
        let num = *(buf.as_ptr() as *const u32) as usize;
        // MIB_TCPROW_OWNER_PID: state, localAddr, localPort, remoteAddr,
        // remotePort, owningPid — 6 x u32 = 24 bytes, header is one u32.
        let base = 4usize;
        for i in 0..num {
            let row = buf.as_ptr().wrapping_add(base + i * 24) as *const u32;
            if row.add(5).read() == 0 {
                continue;
            }
            let local_port = u16::from_be(row.add(2).read() as u16);
            let remote_port = u16::from_be(row.add(4).read() as u16);
            if local_port == client_ephemeral_port && remote_port == gate_port {
                return Some(row.add(5).read());
            }
        }
        None
    }
}

#[cfg(not(windows))]
fn pid_for_connection(_client_ephemeral_port: u16, _gate_port: u16) -> Option<u32> {
    None
}

static NAME_CACHE: Mutex<Option<HashMap<u32, (String, Instant)>>> = Mutex::new(None);
const NAME_CACHE_TTL: Duration = Duration::from_secs(60);

/// Executable file name for a PID (cached briefly; PID reuse is bounded by
/// the TTL so a recycled PID can only skew one decision for a minute).
#[cfg(windows)]
fn process_name(pid: u32) -> Option<String> {
    {
        let cache = NAME_CACHE.lock().ok()?;
        if let Some(cache) = cache.as_ref() {
            if let Some((name, at)) = cache.get(&pid) {
                if at.elapsed() < NAME_CACHE_TTL {
                    return Some(name.clone());
                }
            }
        }
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return None;
        }
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(handle);
        if ok == 0 || size == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        let name = path
            .rsplit(['\\', '/'])
            .next()
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            return None;
        }
        if let Ok(mut slot) = NAME_CACHE.lock() {
            let cache = slot.get_or_insert_with(HashMap::new);
            if cache.len() > 2048 {
                cache.clear();
            }
            cache.insert(pid, (name.clone(), Instant::now()));
        }
        Some(name)
    }
}

#[cfg(not(windows))]
fn process_name(_pid: u32) -> Option<String> {
    None
}

// ---------------------------------------------------------------------------
// Connection handling
// ---------------------------------------------------------------------------

fn read_head(client: &mut TcpStream) -> Option<Vec<u8>> {
    client
        .set_read_timeout(Some(Duration::from_secs(15)))
        .ok()?;
    let mut head = Vec::with_capacity(1024);
    let mut buf = [0u8; 4096];
    loop {
        if head.len() > 65536 {
            return None;
        }
        match client.read(&mut buf) {
            Ok(0) => return None,
            Ok(n) => head.extend_from_slice(&buf[..n]),
            Err(_) => return None,
        }
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    client.set_read_timeout(None).ok()?;
    Some(head)
}

fn splice(mut a: TcpStream, mut b: TcpStream) {
    let mut a2 = match a.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let mut b2 = match b.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    };
    let up = std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match a2.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if b.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = b.shutdown(std::net::Shutdown::Write);
    });
    let mut buf = [0u8; 8192];
    loop {
        match b2.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if a.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
        }
    }
    let _ = a.shutdown(std::net::Shutdown::Write);
    let _ = up.join();
}

fn handle_conn(
    mut client: TcpStream,
    gate_port: u16,
    upstream_port: u16,
    mode: &str,
    list: &str,
) {
    let Some(raw) = read_head(&mut client) else { return };
    let Some(req) = parse_head(&raw) else { return };

    // identify the client process, then apply the per-app policy
    let route = match client.peer_addr() {
        Ok(peer) => {
            let name = pid_for_connection(peer.port(), gate_port).and_then(process_name);
            match name {
                Some(n) => route_for(mode, list, &n),
                None => route_for(mode, list, ""),
            }
        }
        Err(_) => Route::Proxy,
    };

    // dial the chosen path
    let target: Option<TcpStream> = match route {
        Route::Proxy => TcpStream::connect(("127.0.0.1", upstream_port)).ok(),
        Route::Direct => TcpStream::connect((req.host.as_str(), req.port)).ok(),
    };
    let Some(mut target) = target else { return };

    match req.kind {
        ReqKind::Connect => {
            // tunnel established for the client either way
            let _ = client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n");
        }
        ReqKind::Forward => {
            let bytes = match route {
                // proxied: forward the client's original proxy request as-is
                Route::Proxy => raw,
                // direct: origin-form request line (rewritten by parse_head)
                Route::Direct => req.out,
            };
            if target.write_all(&bytes).is_err() {
                return;
            }
        }
    }

    splice(client, target);
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Start the gate on `gate_port`, forwarding proxied traffic to the core's
/// private HTTP port `upstream_port`. Settings are snapshotted at connect
/// time (changing them requires a reconnect, same as TUN rules).
pub fn start(
    gate_port: u16,
    upstream_port: u16,
    mode: &str,
    list: &str,
    logs: &std::sync::Arc<Mutex<std::collections::VecDeque<String>>>,
    area: &AppHandle,
) -> Result<GateHandle, String> {
    let listener =
        TcpListener::bind(("127.0.0.1", gate_port)).map_err(|e| format!("cannot bind {}: {}", gate_port, e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;

    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let rules = Arc::new((mode.to_string(), list.to_string()));
    push_log(
        logs,
        area,
        format!(
            "[gate] per-app rules active: 127.0.0.1:{} -> upstream {} (mode: {})",
            gate_port, upstream_port, mode
        ),
    );

    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let rules = rules.clone();
                    std::thread::spawn(move || {
                        let (mode, list) = &*rules;
                        handle_conn(stream, gate_port, upstream_port, mode, list);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(60));
                }
                Err(_) => {
                    if flag.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
            }
        }
    });

    Ok(GateHandle {
        stop,
        accept_thread: Some(join),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bypass_mode_sends_listed_apps_direct() {
        let list = "chrome.exe\nfirefox\n";
        assert_eq!(route_for("bypass", list, "chrome.exe"), Route::Direct);
        assert_eq!(route_for("bypass", list, "CHROME.EXE"), Route::Direct);
        assert_eq!(route_for("bypass", list, "firefox"), Route::Direct);
        assert_eq!(route_for("bypass", list, "discord.exe"), Route::Proxy);
        assert_eq!(route_for("bypass", list, ""), Route::Proxy);
    }

    #[test]
    fn allow_mode_tunnels_only_listed_apps() {
        let list = "myapp.exe";
        assert_eq!(route_for("allow", list, "myapp.exe"), Route::Proxy);
        assert_eq!(route_for("allow", list, "anything.exe"), Route::Direct);
        assert_eq!(route_for("allow", list, ""), Route::Direct);
    }

    #[test]
    fn off_mode_proxies_everything() {
        assert_eq!(route_for("", "", "x.exe"), Route::Proxy);
        assert_eq!(route_for("off", "x.exe", "y.exe"), Route::Proxy);
    }

    #[test]
    fn normalize_handles_case_and_extension() {
        assert_eq!(normalize_proc("  Notepad.EXE "), "notepad");
        assert_eq!(normalize_proc("notepad"), "notepad");
        assert_eq!(normalize_proc(""), "");
    }

    #[test]
    fn parse_connect_request() {
        let raw = b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n";
        let req = parse_head(raw).unwrap();
        assert!(matches!(req.kind, ReqKind::Connect));
        assert_eq!(req.host, "example.com");
        assert_eq!(req.port, 443);
    }

    #[test]
    fn absolute_form_rewrites_to_origin_form() {
        let raw = b"GET http://example.com/some/path?q=1 HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let req = parse_head(raw).unwrap();
        assert!(matches!(req.kind, ReqKind::Forward));
        assert_eq!(req.host, "example.com");
        assert_eq!(req.port, 80);
        let text = String::from_utf8_lossy(&req.out).to_string();
        assert!(text.starts_with("GET /some/path?q=1 HTTP/1.1\r\n"), "{}", text);
        assert!(text.contains("Host: example.com"));
    }

    #[test]
    fn origin_form_uses_host_header() {
        let raw = b"GET /index.html HTTP/1.1\r\nHost: site.test:8080\r\nAccept: */*\r\n\r\n";
        let req = parse_head(raw).unwrap();
        assert_eq!(req.host, "site.test");
        assert_eq!(req.port, 8080);
        // unchanged bytes for the direct path
        assert_eq!(req.out, raw);
    }

    #[test]
    fn ipv6_literal_parses() {
        let raw = b"GET http://[2001:db8::1]:8080/ HTTP/1.1\r\nHost: x\r\n\r\n";
        let req = parse_head(raw).unwrap();
        assert_eq!(req.host, "2001:db8::1");
        assert_eq!(req.port, 8080);
    }

    #[test]
    fn garbage_is_rejected() {
        assert!(parse_head(b"\x00\x01\x02junk").is_none() || parse_head(b"junk").is_none());
    }
}
