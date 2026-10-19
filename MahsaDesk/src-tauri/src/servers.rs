use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_id() -> String {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros();
    format!("srv-{}", n)
}

/// A single proxy profile, decoded from a share link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub id: String,
    pub sub_id: String,
    pub remark: String,
    pub protocol: String, // vmess | vless | trojan | shadowsocks | socks | http
    pub raw: String,
    pub address: String,
    pub port: u16,
    pub auth: String,
    pub method: String,
    pub alter_id: u32,
    pub encryption: String,
    pub net: String,
    pub header_type: String,
    pub host: String,
    pub path: String,
    pub security: String, // "" | tls | reality
    pub sni: String,
    pub fingerprint: String,
    pub pbk: String,
    pub sid: String,
    pub spx: String,
    pub flow: String,
    pub alpn: String,
    pub delay: Option<u32>,
}

impl ServerProfile {
    pub fn empty() -> Self {
        ServerProfile {
            id: now_id(),
            sub_id: String::new(),
            remark: String::new(),
            protocol: String::new(),
            raw: String::new(),
            address: String::new(),
            port: 443,
            auth: String::new(),
            method: String::new(),
            alter_id: 0,
            encryption: String::new(),
            net: "tcp".into(),
            header_type: String::new(),
            host: String::new(),
            path: String::new(),
            security: String::new(),
            sni: String::new(),
            fingerprint: String::new(),
            pbk: String::new(),
            sid: String::new(),
            spx: String::new(),
            flow: String::new(),
            alpn: String::new(),
            delay: None,
        }
    }
}

fn b64_decode(input: &str) -> Option<Vec<u8>> {
    let s: String = input
        .trim()
        .replace('\r', "")
        .replace('\n', "")
        .replace('-', "+")
        .replace('_', "/");
    let pad = |t: &str| {
        let rem = t.len() % 4;
        if rem > 0 {
            format!("{}{}", t, "=".repeat(4 - rem))
        } else {
            t.to_string()
        }
    };
    STANDARD
        .decode(&s)
        .or_else(|_| STANDARD.decode(pad(&s)))
        .or_else(|_| URL_SAFE_NO_PAD.decode(input.trim()))
        .ok()
}

/// Parse an entire subscription payload: either a base64 blob or plain lines of share links.
pub fn parse_payload(payload: &str) -> Vec<ServerProfile> {
    let payload = payload.trim();
    let mut out = Vec::new();

    // plain text list of links first
    for line in payload.lines().map(str::trim) {
        if !line.is_empty() {
            if let Some(p) = parse_link(line) {
                out.push(p);
            }
        }
    }
    if !out.is_empty() {
        return out;
    }

    // try base64 blob
    if let Some(bytes) = b64_decode(payload) {
        if let Ok(text) = String::from_utf8(bytes) {
            for line in text.lines().map(str::trim) {
                if !line.is_empty() {
                    if let Some(p) = parse_link(line) {
                        out.push(p);
                    }
                }
            }
        }
    }
    out
}

/// Parse a single share link into a profile. Returns None for unknown schemes.
pub fn parse_link(link: &str) -> Option<ServerProfile> {
    let link = link.trim();
    if let Some(rest) = link.strip_prefix("vmess://") {
        parse_vmess(rest)
    } else if let Some(rest) = link.strip_prefix("vless://") {
        parse_userinfo_link(rest, "vless")
    } else if let Some(rest) = link.strip_prefix("trojan://") {
        parse_userinfo_link(rest, "trojan")
    } else if let Some(rest) = link.strip_prefix("ss://") {
        parse_ss(rest)
    } else if let Some(rest) = link.strip_prefix("socks://") {
        parse_url_scheme(rest, "socks")
    } else if let Some(rest) = link.strip_prefix("http://") {
        parse_url_scheme(rest, "http")
    } else if let Some(rest) = link.strip_prefix("https://") {
        parse_url_scheme(rest, "http")
    } else {
        None
    }
}

fn parse_vmess(b64: &str) -> Option<ServerProfile> {
    let bytes = b64_decode(b64)?;
    let text = String::from_utf8(bytes).ok()?;
    let v: serde_json::Value = serde_json::from_str(text.trim()).ok()?;
    let mut p = ServerProfile::empty();
    p.protocol = "vmess".into();
    p.raw = format!("vmess://{}", b64);
    p.remark = v.get("ps").and_then(|x| x.as_str()).unwrap_or("").to_string();
    p.address = v
        .get("add")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.port = v
        .get("port")
        .map(|x| {
            x.as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| x.as_u64().unwrap_or(0) as u16)
        })
        .unwrap_or(0);
    if p.port == 0 {
        return None;
    }
    p.auth = v
        .get("id")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.alter_id = v
        .get("aid")
        .map(|x| {
            x.as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| x.as_u64().unwrap_or(0) as u32)
        })
        .unwrap_or(0);
    p.encryption = v
        .get("scy")
        .and_then(|x| x.as_str())
        .unwrap_or("auto")
        .to_string();
    p.net = v
        .get("net")
        .and_then(|x| x.as_str())
        .unwrap_or("tcp")
        .to_string();
    p.header_type = v
        .get("type")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.host = v
        .get("host")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.path = v
        .get("path")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.security = v
        .get("tls")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if p.security == "true" {
        p.security = "tls".into();
    }
    p.sni = v
        .get("sni")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.alpn = v
        .get("alpn")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    p.fingerprint = v
        .get("fp")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    if p.remark.is_empty() {
        p.remark = p.address.clone();
    }
    Some(p)
}

/// Generic user[:pass]@host:port?query#remark parser (vless / trojan).
fn parse_userinfo_link(rest: &str, protocol: &str) -> Option<ServerProfile> {
    let (main, params, remark) = split_uri(rest)?;
    let (auth, hostport) = match main.rsplit_once('@') {
        Some((a, h)) => (a.to_string(), h.to_string()),
        None => (String::new(), main.to_string()),
    };
    let (host, port) = parse_hostport(&hostport)?;
    let mut p = ServerProfile::empty();
    p.protocol = protocol.into();
    p.raw = format!("{}://{}", protocol, rest);
    p.remark = remark.unwrap_or_else(|| host.clone());
    p.auth = percent_decode(&auth);
    p.address = host;
    p.port = port;
    p.net = params.get("type").cloned().unwrap_or_else(|| "tcp".into());
    p.header_type = params.get("headerType").cloned().unwrap_or_default();
    p.host = params.get("host").cloned().unwrap_or_default();
    p.path = params
        .get("path")
        .map(|s| percent_decode(s))
        .unwrap_or_default();
    p.security = params.get("security").cloned().unwrap_or_default();
    if protocol == "trojan" && p.security.is_empty() {
        p.security = "tls".into();
    }
    p.sni = params.get("sni").cloned().unwrap_or_default();
    p.fingerprint = params.get("fp").cloned().unwrap_or_default();
    p.pbk = params.get("pbk").cloned().unwrap_or_default();
    p.sid = params.get("sid").cloned().unwrap_or_default();
    p.spx = params.get("spx").cloned().unwrap_or_default();
    p.flow = params.get("flow").cloned().unwrap_or_default();
    p.alpn = params.get("alpn").cloned().unwrap_or_default();
    if protocol == "vless" {
        p.encryption = params
            .get("encryption")
            .cloned()
            .unwrap_or_else(|| "none".into());
    }
    Some(p)
}

fn parse_ss(rest: &str) -> Option<ServerProfile> {
    let (main, _params, remark) = split_uri(rest)?;
    let (userinfo, hostport) = match main.rsplit_once('@') {
        Some((u, h)) => (u.to_string(), h.to_string()),
        // legacy fully-encoded form: ss://base64(method:pass@host:port)
        None => {
            let bytes = b64_decode(&main)?;
            let text = String::from_utf8(bytes).ok()?;
            match text.rsplit_once('@') {
                Some((u, h)) => (u.to_string(), h.to_string()),
                None => return None,
            }
        }
    };
    let (host, port) = parse_hostport(&hostport)?;
    let mut p = ServerProfile::empty();
    p.protocol = "shadowsocks".into();
    p.raw = format!("ss://{}", rest);
    p.remark = remark.unwrap_or_else(|| host.clone());
    p.address = host;
    p.port = port;
    // method:password may be plain or base64 encoded
    let decoded = b64_decode(&userinfo)
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| percent_decode(&userinfo));
    if let Some((method, pass)) = decoded.split_once(':') {
        p.method = method.to_string();
        p.auth = pass.to_string();
    } else {
        p.auth = decoded;
    }
    p.net = "tcp".into();
    Some(p)
}

fn parse_url_scheme(rest: &str, protocol: &str) -> Option<ServerProfile> {
    let rest = rest.trim_start_matches("//");
    let (main, _params, remark) = split_uri(rest)?;
    let (auth, hostport) = match main.rsplit_once('@') {
        Some((a, h)) => (a.to_string(), h.to_string()),
        None => (String::new(), main.to_string()),
    };
    let (host, port) = parse_hostport(&hostport)?;
    let mut p = ServerProfile::empty();
    p.protocol = protocol.into();
    p.raw = format!("{}://{}", protocol, rest);
    p.remark = remark.unwrap_or_else(|| host.clone());
    p.address = host;
    p.port = port;
    p.auth = percent_decode(&auth);
    p.net = "tcp".into();
    Some(p)
}

/// Split "mainpart?query#remark".
fn split_uri(rest: &str) -> Option<(String, HashMap<String, String>, Option<String>)> {
    let (main_part, fragment) = match rest.split_once('#') {
        Some((m, f)) => (m.to_string(), Some(percent_decode(f))),
        None => (rest.to_string(), None),
    };
    let (main_part, query) = match main_part.split_once('?') {
        Some((m, q)) => (m.to_string(), Some(q.to_string())),
        None => (main_part, None),
    };
    let mut params: HashMap<String, String> = HashMap::new();
    if let Some(q) = query {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                params.insert(k.to_string(), percent_decode(v));
            } else if !pair.is_empty() {
                params.insert(pair.to_string(), String::new());
            }
        }
    }
    if main_part.is_empty() {
        return None;
    }
    Some((main_part, params, fragment))
}

fn parse_hostport(s: &str) -> Option<(String, u16)> {
    let s = s.trim();
    // IPv6 [::1]:443
    if s.starts_with('[') {
        let end = s.find(']')?;
        let host = s[1..end].to_string();
        let rest = &s[end + 1..];
        let port = rest.strip_prefix(':')?.parse().ok()?;
        return Some((host, port));
    }
    let (h, p) = s.rsplit_once(':')?;
    let port: u16 = p.parse().ok()?;
    Some((h.to_string(), port))
}

fn percent_decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
}
