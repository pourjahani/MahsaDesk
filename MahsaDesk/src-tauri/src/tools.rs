//! Network diagnostic tools, the desktop equivalents of MahsaNG's
//! DNS scanner, Mahsa IP scanner and the network tools utilities.

use serde::Serialize;
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// DNS scanner
// ---------------------------------------------------------------------------

/// Minimal raw UDP DNS A-query builder (works over plain sockets, no deps).
pub fn dns_query_packet(domain: &str) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(64);
    let id: u16 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() & 0xFFFF) as u16;
    pkt.extend_from_slice(&id.to_be_bytes()); // id
    pkt.extend_from_slice(&[0x01, 0x00]); // flags: recursion desired
    pkt.extend_from_slice(&[0x00, 0x01]); // QDCOUNT
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // NS/AR/AD
    for label in domain.trim().trim_end_matches('.').split('.') {
        let label = label.as_bytes();
        pkt.push(label.len().min(63) as u8);
        pkt.extend_from_slice(&label[..label.len().min(63)]);
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x01]); // QTYPE = A
    pkt.extend_from_slice(&[0x00, 0x01]); // QCLASS = IN
    pkt
}

/// Parse A-records out of an answer section (compression-pointer aware).
pub fn parse_dns_answer(pkt: &[u8]) -> Vec<String> {
    let mut ips = Vec::new();
    // walk question section
    if pkt.len() < 12 {
        return ips;
    }
    let qdcount = u16::from_be_bytes([pkt[4], pkt[5]]) as usize;
    let mut pos = 12usize;
    for _ in 0..qdcount {
        pos = skip_name(pkt, pos);
        pos += 4; // qtype + qclass
        if pos >= pkt.len() {
            return ips;
        }
    }
    let ancount = u16::from_be_bytes([pkt[6], pkt[7]]) as usize;
    for _ in 0..ancount {
        if pos >= pkt.len() {
            break;
        }
        pos = skip_name(pkt, pos);
        if pos + 10 > pkt.len() {
            break;
        }
        let rtype = u16::from_be_bytes([pkt[pos], pkt[pos + 1]]);
        let rdlen = u16::from_be_bytes([pkt[pos + 8], pkt[pos + 9]]) as usize;
        pos += 10;
        if rtype == 1 && rdlen == 4 && pos + 4 <= pkt.len() {
            ips.push(format!(
                "{}.{}.{}.{}",
                pkt[pos], pkt[pos + 1], pkt[pos + 2], pkt[pos + 3]
            ));
        }
        pos += rdlen;
    }
    ips
}

/// Skip a DNS name starting at `pos` (handles compression pointers).
fn skip_name(pkt: &[u8], mut pos: usize) -> usize {
    loop {
        if pos >= pkt.len() {
            return pos;
        }
        let len = pkt[pos] as usize;
        if len == 0 {
            return pos + 1;
        }
        if len & 0xC0 == 0xC0 {
            return pos + 2;
        }
        pos += len + 1;
    }
}

#[derive(Serialize, Clone)]
pub struct DnsResult {
    pub server: String,
    pub kind: String, // "udp" | "doh"
    pub status: String, // "ok" | "timeout" | "error" | "differs"
    pub delay: Option<u32>,
    pub ips: Vec<String>,
}

/// Public DNS list used for the scanner: plain resolvers + Iran-local resolvers
/// + two DoH endpoints as a truth reference.
pub const DNS_SERVERS: &[(&str, &str, &str)] = &[
    ("system", "", "udp"),
    ("8.8.8.8", "Google", "udp"),
    ("8.8.4.4", "Google 2", "udp"),
    ("1.1.1.1", "Cloudflare", "udp"),
    ("9.9.9.9", "Quad9", "udp"),
    ("178.22.122.100", "Shecan", "udp"),
    ("185.51.200.2", "Shecan 2", "udp"),
    ("10.202.10.202", "403.online", "udp"),
    ("10.202.10.102", "403.online 2", "udp"),
    ("10.202.10.10", "Radar", "udp"),
    ("10.202.10.11", "Radar 2", "udp"),
    ("78.157.42.100", "Electro", "udp"),
    ("78.157.42.101", "Electro 2", "udp"),
    ("185.55.226.26", "Begzar", "udp"),
    ("185.55.225.25", "Begzar 2", "udp"),
    ("1.1.1.1", "Cloudflare DoH", "doh"),
    ("8.8.8.8", "Google DoH", "doh"),
];

fn resolve_one_udp(server: &str, domain: &str) -> (DnsResult, Option<String>) {
    let pkt = dns_query_packet(domain);
    let mut out = DnsResult {
        server: server.into(),
        kind: "udp".into(),
        status: "error".into(),
        delay: None,
        ips: Vec::new(),
    };
    let bind: std::net::SocketAddr = if server.starts_with("10.") || server.starts_with("178.")
        || server.starts_with("185.") || server.starts_with("78.") || server.starts_with("192.")
    {
        // Iran-local resolvers typically answer any query; still bind LAN ip
        "0.0.0.0:0".parse().unwrap()
    } else {
        "0.0.0.0:0".parse().unwrap()
    };
    let _ = bind;
    let sock = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return (out, None),
    };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(1500)));
    let start = Instant::now();
    if sock
        .send_to(&pkt, format!("{}:53", server))
        .is_err()
    {
        out.status = "error".into();
        return (out, None);
    }
    let mut buf = [0u8; 2048];
    let recv = sock.recv_from(&mut buf);
    match recv {
        Ok((n, _)) => {
            out.delay = Some(start.elapsed().as_millis() as u32);
            out.ips = parse_dns_answer(&buf[..n]);
            if out.ips.is_empty() {
                out.status = "empty".into();
                (out, None)
            } else {
                out.status = "ok".into();
                let first = out.ips.first().cloned();
                (out, first)
            }
        }
        Err(_) => {
            out.status = "timeout".into();
            (out, None)
        }
    }
}

async fn resolve_one_doh(server_ip: &str, domain: &str) -> (DnsResult, Option<String>) {
    let out = DnsResult {
        server: server_ip.into(),
        kind: "doh".into(),
        status: "error".into(),
        delay: None,
        ips: Vec::new(),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new());
    let url = format!("https://{}/dns-query?name={}&type=A", server_ip, domain);
    let start = Instant::now();
    let resp = client
        .get(&url)
        .header("accept", "application/dns-json")
        .send()
        .await;
    match resp {
        Ok(r) => {
            if let Ok(v) = r.json::<serde_json::Value>().await {
                let mut ips = Vec::new();
                status_of_records(&v, &mut ips);
                let mut out = out;
                out.delay = Some(start.elapsed().as_millis() as u32);
                out.ips = ips.clone();
                out.status = if ips.is_empty() { "empty".into() } else { "ok".into() };
                (out, ips.first().cloned())
            } else {
                (out, None)
            }
        }
        Err(_) => (out, None),
    }
}

fn status_of_records(v: &serde_json::Value, ips: &mut Vec<String>) {
    if let Some(arr) = v.get("Answer").and_then(|a| a.as_array()) {
        for a in arr {
            if a.get("type").and_then(|t| t.as_u64()) == Some(1) {
                if let Some(ip) = a.get("data").and_then(|d| d.as_str()) {
                    ips.push(ip.to_string());
                }
            }
        }
    }
}

/// Scan one domain through the local + public resolver list and DoH reference.
pub async fn dns_scan(domain: &str) -> Vec<DnsResult> {
    let domain = domain.trim().to_string();
    // "system" entry via std resolver
    let mut results: Vec<DnsResult> = Vec::new();
    {
        let d2 = domain.clone();
        let sys = tauri::async_runtime::spawn_blocking(move || {
            let mut out = DnsResult {
                server: "system".into(),
                kind: "udp".into(),
                status: "error".into(),
                delay: None,
                ips: Vec::new(),
            };
            let start = Instant::now();
            match (d2.as_str(), 53).to_socket_addrs() {
                Ok(addrs) => {
                    out.delay = Some(start.elapsed().as_millis() as u32);
                    let ips: Vec<String> = addrs
                        .map(|a| a.ip().to_string())
                        .filter(|ip| !ip.contains(':'))
                        .take(3)
                        .collect();
                    out.ips = ips.clone();
                    out.status = if ips.is_empty() { "empty".into() } else { "ok".into() };
                    out
                }
                Err(_) => {
                    out.status = "timeout".into();
                    out
                }
            }
        });
        if let Ok(r) = sys.await {
            results.push(r);
        }
    }
    // generic (non Tetris-bound) DNS lookups run in a small thread pool
    let mut handles = Vec::new();
    for (server, _name, kind) in DNS_SERVERS {
        if *server == "system" {
            continue;
        }
        let domain = domain.clone();
        let server = (*server).to_string();
        let kind = kind.to_string();
        handles.push((server.clone(), kind.clone(), async move {
            if kind == "doh" {
                resolve_one_doh(&server, &domain).await
            } else {
                let fallback_server = server.clone();
                tauri::async_runtime::spawn_blocking(move || resolve_one_udp(&server, &domain))
                    .await
                    .unwrap_or((
                        DnsResult {
                            server: fallback_server,
                            kind: "udp".into(),
                            status: "error".into(),
                            delay: None,
                            ips: Vec::new(),
                        },
                        None,
                    ))
            }
        }));
    }
    // Because async closures inside tuple lists are unwieldy, await them sequentially here.
    for (_server, kind, fut) in handles {
        let (res, _ip) = fut.await;
        let mut res = res;
        res.kind = kind;
        results.push(res);
    }

    // cross-check: flag entries that return an IP different from the DoH truth
    let truth = results
        .iter()
        .filter(|r| r.kind == "doh")
        .flat_map(|r| r.ips.iter().cloned())
        .collect::<Vec<String>>();
    for r in results.iter_mut() {
        if r.kind == "udp" && r.status == "ok" {
            if let Some(ip) = r.ips.first() {
                if !truth.is_empty() && !truth.contains(ip) {
                    r.status = "differs".into();
                }
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// Mahsa IP scanner (TCP availability of candidate CDN/server IPs)
// ---------------------------------------------------------------------------

#[derive(Serialize, Clone)]
pub struct IpResult {
    pub ip: String,
    pub ok: bool,
    pub delay: Option<u32>,
}

/// Candidate IP presets for the scanner.
pub const IP_PRESETS: &[(&str, &[&str])] = &[
    (
        "cloudflare",
        &["104.16.0.0/16", "104.17.0.0/16", "104.18.0.0/16", "172.64.0.0/16", "162.159.0.0/16"],
    ),
    (
        "google",
        &["142.250.0.0/18", "172.217.0.0/18", "216.58.192.0/18"],
    ),
    (
        "digitalocean",
        &["159.65.0.0/16", "139.59.0.0/16"],
    ),
];

pub fn expand_candidate_ips(preset: &str, custom: &str, max_per_cidr: usize) -> Vec<String> {
    let cids: Vec<String> = if preset == "custom" {
        custom
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| l.to_string())
            .collect()
    } else {
        IP_PRESETS
            .iter()
            .find(|(id, _)| *id == preset)
            .map(|(_, lst)| lst.iter().map(|s| s.to_string()).collect())
            .unwrap_or_default()
    };
    let mut out: Vec<String> = Vec::new();
    for cid in cids {
        if let Some(ip) = cid.parse::<std::net::Ipv4Addr>().ok().map(|i| i.to_string()) {
            out.push(ip);
            continue;
        }
        // CIDR form
        if let Some((addr, bits)) = cid.split_once('/') {
            let Ok(base) = addr.parse::<std::net::Ipv4Addr>() else {
                continue;
            };
            let Ok(bits) = bits.parse::<u8>() else { continue };
            if bits > 24 {
                continue;
            }
            let base_u = u32::from(base);
            let mask = if bits == 0 { 0 } else { u32::MAX << (32 - bits) };
            let host_count = 1u32 << (32 - bits.min(24));
            // stride so big blocks are sampled, always cap at max_per_cidr
            let stride = (host_count / max_per_cidr.max(1) as u32).max(1);
            for i in (1..host_count.saturating_sub(1)).step_by(stride as usize) {
                if out.len() >= 512 {
                    break;
                }
                let ip = std::net::Ipv4Addr::from(base_u & mask | i);
                out.push(ip.to_string());
            }
        }
    }
    out.truncate(512);
    out
}

/// TCP-check every candidate IP against `port` with a fixed timeout, running
/// in parallel batches.
pub fn ip_scan(preset: &str, custom: &str, port: u16) -> Vec<IpResult> {
    let ips = expand_candidate_ips(preset, custom, 64);
    let results: std::sync::Mutex<Vec<IpResult>> = std::sync::Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for chunk in ips.chunks(32) {
            let res = &results;
            s.spawn(move || {
                for ip in chunk {
                    let start = Instant::now();
                    let target: std::net::SocketAddr =
                        format!("{}:{}", ip, port).parse().unwrap_or("0.0.0.0:0".parse().unwrap());
                    let ok = TcpStream::connect_timeout(&target, Duration::from_secs(2)).is_ok();
                    let delay = if ok { Some(start.elapsed().as_millis() as u32) } else { None };
                    res.lock().unwrap().push(IpResult {
                        ip: ip.clone(),
                        ok,
                        delay,
                    });
                }
            });
        }
    });
    let mut list = results.into_inner().unwrap_or_default();
    list.sort_by(|a, b| {
        a.ok.cmp(&b.ok).then(a.delay.unwrap_or(999999).cmp(&b.delay.unwrap_or(999999)))
    });
    list
}

// ---------------------------------------------------------------------------
// Network tools
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct NetToolOutput {
    pub ok: bool,
    pub detail: String,
}

#[cfg(windows)]
fn windows_tool(exe: &str, args: &[&str]) -> NetToolOutput {
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    crate::proc::hide(&mut cmd);
    let out = cmd.output().map_err(|e| e.to_string());
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout).to_string()
                + &String::from_utf8_lossy(&o.stderr);
            NetToolOutput {
                ok: o.status.success(),
                detail: text.lines().take(60).collect::<Vec<_>>().join("\n"),
            }
        }
        Err(e) => NetToolOutput {
            ok: false,
            detail: e,
        },
    }
}

pub fn net_ping(host: &str) -> NetToolOutput {
    #[cfg(windows)]
    {
        windows_tool("ping", &["-n", "4", "-w", "3000", host])
    }
    #[cfg(not(windows))]
    {
        let out = std::process::Command::new("ping").args(["-c", "4", host]).output();
        match out {
            Ok(o) => NetToolOutput {
                ok: o.status.success(),
                detail: String::from_utf8_lossy(&o.stdout).to_string(),
            },
            Err(e) => NetToolOutput {
                ok: false,
                detail: e.to_string(),
            },
        }
    }
}

pub fn net_traceroute(host: &str) -> NetToolOutput {
    #[cfg(windows)]
    {
        windows_tool("tracert", &["-d", "-w", "700", "-h", "12", host])
    }
    #[cfg(not(windows))]
    {
        let out = std::process::Command::new("traceroute")
            .args(["-n", "-w", "1", "-m", "12", host])
            .output();
        match out {
            Ok(o) => NetToolOutput {
                ok: o.status.success(),
                detail: String::from_utf8_lossy(&o.stdout).to_string(),
            },
            Err(e) => NetToolOutput {
                ok: false,
                detail: e.to_string(),
            },
        }
    }
}

pub fn net_dns_lookup(host: &str) -> NetToolOutput {
    #[cfg(windows)]
    {
        windows_tool("nslookup", &[host])
    }
    #[cfg(not(windows))]
    {
        windows_tool("nslookup", &[host])
    }
}

#[derive(Serialize)]
pub struct PortCheckResult {
    pub open: bool,
    pub delay: Option<u32>,
}

pub fn net_port_check(host: &str, port: u16) -> PortCheckResult {
    let start = Instant::now();
    let target: std::net::SocketAddr =
        format!("{}:{}", host, port).parse().unwrap_or("0.0.0.0:0".parse().unwrap());
    let open = TcpStream::connect_timeout(&target, Duration::from_secs(4)).is_ok();
    let delay = if open { Some(start.elapsed().as_millis() as u32) } else { None };
    PortCheckResult { open, delay }
}

// ---------------------------------------------------------------------------
// IP / speed tools (plain HTTP, through the direct connection)
// ---------------------------------------------------------------------------

fn http_client(timeout: Duration) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .user_agent("MahsaDesk/1.0")
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// "What is my IP" + rough geolocation, the classic network-tools view.
pub fn my_ip_info() -> Result<serde_json::Value, String> {
    let client = http_client(Duration::from_secs(6));
    let ip_raw = client
        .get("http://ip-api.com/json/?fields=status,message,country,countryCode,regionName,city,isp,org,as,timezone,query")
        .send()
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .map_err(|e| e.to_string())?;
    Ok(ip_raw)
}

/// Download-based speed test: pulls `bytes` from Cloudflare's speed endpoint
/// and returns measured Mbit/s.
pub fn net_speed_test(bytes: u64) -> Result<f64, String> {
    let client = http_client(Duration::from_secs(30));
    let start = Instant::now();
    let resp = client
        .get(format!("https://speed.cloudflare.com/__down?bytes={}", bytes))
        .send()
        .map_err(|e| e.to_string())?;
    let body = resp.bytes().map_err(|e| e.to_string())?;
    let secs = start.elapsed().as_secs_f64();
    let got = body.len() as f64 * 8.0 / 1_000_000.0;
    Ok(got / secs.max(0.001))
}