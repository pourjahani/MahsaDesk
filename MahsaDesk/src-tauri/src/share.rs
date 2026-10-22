//! Share the current server pool as a subscription URL (#16).
//!
//! A tiny loopback (or LAN) HTTP endpoint that answers `GET /` with the base64
//! of every share link the app holds, so a phone on the same network can point
//! any subscription client at it — or scan the QR the UI shows. Uses only the
//! standard library: no web framework, one accept thread, one thread per
//! request, payload generated fresh on every hit.

use std::collections::VecDeque;
use std::net::{Ipv4Addr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde::Serialize;
use tauri::AppHandle;

use crate::core::push_log;

#[derive(Serialize, Clone)]
pub struct ShareInfo {
    /// URL subscription clients should use (LAN IP when sharing on LAN).
    pub url: String,
    /// Loopback URL (always present, useful for local tests).
    pub local_url: String,
    /// LAN URL when sharing on LAN, else None.
    pub lan_url: Option<String>,
    pub port: u16,
    pub lan: bool,
}

pub struct ShareHandle {
    info: ShareInfo,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl ShareHandle {
    pub fn info(&self) -> &ShareInfo {
        &self.info
    }

    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.join.take() {
            let _ = t.join();
        }
    }
}

/// Best-effort LAN IPv4 of this machine (route chosen towards a public IP;
/// the socket never actually sends packets when using connect on UDP).
pub fn lan_ip() -> Option<String> {
    let sock = UdpSocket::bind(("0.0.0.0", 0)).ok()?;
    sock.connect(("1.1.1.1", 53)).ok()?;
    let addr = sock.local_addr().ok()?;
    let ip = addr.ip();
    if let std::net::IpAddr::V4(v4) = ip {
        if v4.is_private() || v4.is_loopback() {
            return None;
        }
        return Some(v4.to_string());
    }
    None
}

fn respond(mut stream: TcpStream, body: &str, status: &str) {
    let mut head = format!(
        "HTTP/1.1 {}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        body.len()
    );
    head.push_str(body);
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.flush();
}

fn handle(mut stream: TcpStream, payload: &dyn Fn() -> String) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut buf = [0u8; 2048];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };
    let text = String::from_utf8_lossy(&buf[..n]);
    let first = text.lines().next().unwrap_or("");
    // only serve GET/HEAD of the document; anything else gets a hint
    if first.starts_with("GET ") || first.starts_with("HEAD ") {
        let path = first.split_whitespace().nth(1).unwrap_or("/");
        if path == "/" || path.starts_with("/sub") {
            respond(stream, &payload(), "200 OK");
        } else {
            respond(
                stream,
                "MahsaDesk share endpoint. Point your subscription client at /\n",
                "404 Not Found",
            );
        }
    } else {
        respond(stream, "method not allowed\n", "405 Method Not Allowed");
    }
}

use std::io::{Read, Write};

/// Start the share endpoint. `payload` is called on every request so edits to
/// the pool show up immediately. Returns the handle plus the URLs to show.
pub fn start(
    port: u16,
    lan: bool,
    payload: Arc<dyn Fn() -> String + Send + Sync>,
    logs: &Arc<Mutex<VecDeque<String>>>,
    area: &AppHandle,
) -> Result<ShareHandle, String> {
    let host: Ipv4Addr = if lan {
        Ipv4Addr::UNSPECIFIED
    } else {
        Ipv4Addr::LOCALHOST
    };
    let listener = TcpListener::bind((host, port))
        .map_err(|e| format!("cannot share on port {}: {}", port, e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| e.to_string())?;

    let local_url = format!("http://127.0.0.1:{}", port);
    let lan_url = if lan {
        lan_ip().map(|ip| format!("http://{}:{}", ip, port))
    } else {
        None
    };
    let url = lan_url.clone().unwrap_or_else(|| local_url.clone());
    let info = ShareInfo {
        url,
        local_url,
        lan_url,
        port,
        lan,
    };

    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    push_log(
        logs,
        area,
        format!(
            "[share] serving subscription on {}",
            if lan { info.url.clone() } else { info.local_url.clone() }
        ),
    );

    let join = std::thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let payload = payload.clone();
                    std::thread::spawn(move || handle(stream, &*payload));
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(120));
                }
                Err(_) => {
                    if flag.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(300));
                }
            }
        }
    });

    Ok(ShareHandle {
        info,
        stop,
        join: Some(join),
    })
}
