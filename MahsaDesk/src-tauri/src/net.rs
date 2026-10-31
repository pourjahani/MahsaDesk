//! Outbound HTTP for every download the app performs: engine upgrades, geo
//! data, subscription links and its own updates.
//!
//! A single "just build the default client" approach fails on real machines.
//! The Windows proxy may still point at a port a previous VPN client set and
//! then closed, the direct route to GitHub may be filtered, or a security
//! product may intercept TLS. reqwest then reports the whole thing as one
//! opaque `error sending request for url (...)` and every feature that
//! downloads something breaks at once - which is exactly what the portable
//! build hit on a fresh PC.
//!
//! So each request walks a short ordered list of transports and returns the
//! first success. If all of them fail the error names every route that was
//! tried and the underlying cause, so the UI can say what to change.

use std::io::Write;
use std::path::Path;
use std::sync::RwLock;
use std::time::Duration;

use sha2::{Digest, Sha256};

/// Extra transports, highest priority first, e.g. `http://127.0.0.1:10808`.
/// Filled from the user's download-proxy setting and the app's own tunnel by
/// `refresh_download_proxies` in `lib.rs`.
static PROXIES: RwLock<Vec<String>> = RwLock::new(Vec::new());

const USER_AGENT: &str = "MahsaDesk";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Route {
    /// An explicit proxy: the user's setting or the app's own tunnel.
    Proxy(String),
    /// Whatever the OS and environment configure, plus `HTTP(S)_PROXY`.
    System,
    /// Ignore the OS proxy entirely and dial straight out.
    Direct,
}

impl Route {
    fn label(&self) -> String {
        match self {
            Route::Proxy(p) => format!("proxy {}", p),
            Route::System => "system proxy settings".into(),
            Route::Direct => "direct connection".into(),
        }
    }
}

/// Replace the extra transports. Entries are normalised and de-duplicated;
/// junk (empty, spaces, unknown scheme, no port) is dropped rather than
/// turned into a route that can only fail.
pub fn set_proxies(list: Vec<String>) {
    let mut clean: Vec<String> = Vec::new();
    for raw in list {
        let p = normalize_proxy(&raw);
        if p.is_empty() || clean.contains(&p) {
            continue;
        }
        clean.push(p);
    }
    *PROXIES.write().unwrap_or_else(|e| e.into_inner()) = clean;
}

/// Accept `127.0.0.1:10808`, `host:port`, `http://host:port`,
/// `socks5://host:port` and `socks5h://host:port`. Anything else is "".
pub fn normalize_proxy(raw: &str) -> String {
    let t = raw.trim();
    if t.is_empty() || t.chars().any(|c| c.is_whitespace()) {
        return String::new();
    }
    let lower = t.to_ascii_lowercase();
    for scheme in [
        "http://",
        "https://",
        "socks5h://",
        "socks5://",
        "socks4://",
    ] {
        if lower.starts_with(scheme) {
            return t.to_string();
        }
    }
    if t.contains("://") || !t.contains(':') {
        return String::new();
    }
    format!("http://{}", t)
}

fn routes() -> Vec<Route> {
    let proxies = PROXIES.read().unwrap_or_else(|e| e.into_inner()).clone();
    let mut out: Vec<Route> = proxies.into_iter().map(Route::Proxy).collect();
    out.push(Route::System);
    // Last resort: a stale or unreachable system proxy must never be able to
    // block a download that the plain direct route would have completed.
    out.push(Route::Direct);
    out
}

// ---------------------------------------------------------------------------
// Error detail
// ---------------------------------------------------------------------------

/// `outer <- cause <- root cause`: reqwest's `Display` only shows the first
/// layer, which is why "error sending request for url (...)" told the user
/// nothing about what actually went wrong.
fn cause_chain(err: &(dyn std::error::Error + 'static)) -> String {
    let mut out = String::new();
    let mut cur = Some(err);
    let mut depth = 0;
    while let Some(e) = cur {
        if depth > 0 {
            out.push_str(" <- ");
        }
        out.push_str(&e.to_string());
        cur = e.source();
        depth += 1;
        if depth > 6 {
            break;
        }
    }
    out
}

fn describe(e: &reqwest::Error) -> String {
    let mut kind: Vec<&str> = Vec::new();
    if e.is_connect() {
        kind.push("connect");
    }
    if e.is_timeout() {
        kind.push("timeout");
    }
    if e.is_body() {
        kind.push("body");
    }
    if e.is_decode() {
        kind.push("decode");
    }
    let chain = cause_chain(e);
    if kind.is_empty() {
        chain
    } else {
        format!("{} [{}]", chain, kind.join("/"))
    }
}

fn http_hint(status: u16) -> &'static str {
    match status {
        403 => " (access denied: GitHub rate limit, or this route is blocked)",
        404 => " (the file or release does not exist)",
        407 => " (the proxy wants authentication)",
        429 => " (too many requests, try again in a minute)",
        _ => "",
    }
}

fn failure_message(url: &str, failures: &[String]) -> String {
    format!(
        "could not download {}\nEvery route failed:\n{}\nConnect MahsaDesk to a working server so its own proxy can fetch, or set a download proxy in Settings (e.g. http://127.0.0.1:10808), then retry.",
        url,
        failures.join("\n")
    )
}

// ---------------------------------------------------------------------------
// Transport
// ---------------------------------------------------------------------------

fn client(route: &Route, timeout: Duration) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(timeout.min(Duration::from_secs(20)));
    builder = match route {
        Route::Proxy(p) => builder.proxy(
            reqwest::Proxy::all(p).map_err(|e| format!("invalid proxy {}: {}", p, e))?,
        ),
        Route::System => builder,
        Route::Direct => builder.no_proxy(),
    };
    builder
        .build()
        .map_err(|e| cause_chain(&e).to_string())
}

async fn request(
    route: &Route,
    url: &str,
    timeout: Duration,
) -> Result<reqwest::Response, String> {
    let c = client(route, timeout)?;
    c.get(url).timeout(timeout).send().await.map_err(|e| describe(&e))
}

/// GET `url`, trying each transport in turn. A transport that produces any
/// HTTP response is authoritative: the server answered, so another route
/// cannot do better.
async fn get_response(url: &str, timeout: Duration) -> Result<reqwest::Response, String> {
    let mut failures: Vec<String> = Vec::new();
    for route in routes() {
        match request(&route, url, timeout).await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(resp);
                }
                return Err(format!(
                    "HTTP {} from {}{}",
                    status.as_u16(),
                    url,
                    http_hint(status.as_u16())
                ));
            }
            Err(reason) => failures.push(format!("  - {}: {}", route.label(), reason)),
        }
    }
    Err(failure_message(url, &failures))
}

/// Whole body as bytes.
pub async fn get_bytes(url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let resp = get_response(url, timeout).await?;
    resp.bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| format!("download of {} was interrupted: {}", url, describe(&e)))
}

/// Whole body as bytes, reporting `(downloaded, total)` while streaming so the
/// UI can show a fill bar for the multi-MB engine and geo files.
pub async fn get_bytes_progress(
    url: &str,
    timeout: Duration,
    progress: &(dyn Fn(u64, Option<u64>) + Sync),
) -> Result<Vec<u8>, String> {
    let mut resp = get_response(url, timeout).await?;
    let total = resp.content_length();
    let mut buf: Vec<u8> = Vec::with_capacity(total.unwrap_or(0) as usize);
    let mut reported: u64 = 0;
    progress(0, total);
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download of {} was interrupted: {}", url, describe(&e)))?
    {
        buf.extend_from_slice(&chunk);
        // one UI event per 256 KB keeps the webview responsive
        if buf.len() as u64 - reported >= 256 * 1024 {
            reported = buf.len() as u64;
            progress(reported, total);
        }
    }
    progress(buf.len() as u64, total);
    Ok(buf)
}

/// Whole body as text, refusing anything over `max_bytes` so a hostile or
/// broken endpoint cannot exhaust memory.
pub async fn get_text(url: &str, timeout: Duration, max_bytes: usize) -> Result<String, String> {
    let mut resp = get_response(url, timeout).await?;
    let too_big = || format!("{} is larger than the {} MB limit", url, max_bytes / (1024 * 1024));
    if resp
        .content_length()
        .map(|n| n as usize > max_bytes)
        .unwrap_or(false)
    {
        return Err(too_big());
    }
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|e| format!("download of {} was interrupted: {}", url, describe(&e)))?
    {
        if buf.len() + chunk.len() > max_bytes {
            return Err(too_big());
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// JSON body decoded into `T`.
pub async fn get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    timeout: Duration,
) -> Result<T, String> {
    let bytes = get_bytes(url, timeout).await?;
    serde_json::from_slice(&bytes).map_err(|e| format!("unexpected response from {}: {}", url, e))
}

/// Stream `url` into `dest`, reporting `(downloaded, total)` as it goes and
/// verifying `expected_sha256` before returning. On any failure the partial
/// file is removed, so a broken download can never look like an installed one.
pub async fn download_file(
    url: &str,
    dest: &Path,
    expected_sha256: Option<&str>,
    // Sync as well as Fn: the returned future has to stay `Send` so a Tauri
    // command can await it.
    progress: &(dyn Fn(u64, Option<u64>) + Sync),
) -> Result<(), String> {
    let mut resp = get_response(url, Duration::from_secs(600)).await?;
    let total = resp.content_length();
    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("cannot write {}: {}", dest.display(), e))?;
    let mut hasher = Sha256::new();
    let mut done: u64 = 0;
    let mut reported: u64 = 0;
    progress(0, total);
    let interrupted = |e: reqwest::Error| format!("download interrupted: {}", describe(&e));
    loop {
        let chunk = match resp.chunk().await {
            Ok(Some(c)) => c,
            Ok(None) => break,
            Err(e) => {
                let _ = std::fs::remove_file(dest);
                return Err(interrupted(e));
            }
        };
        hasher.update(&chunk);
        if let Err(e) = file.write_all(&chunk) {
            let _ = std::fs::remove_file(dest);
            return Err(format!("cannot write {}: {}", dest.display(), e));
        }
        done += chunk.len() as u64;
        // one UI event per 256 KB keeps the webview responsive on a 30 MB file
        if done - reported >= 256 * 1024 || total == Some(done) {
            reported = done;
            progress(done, total);
        }
    }
    let _ = file.flush();
    drop(file);
    progress(done, total);
    if let Some(expected) = expected_sha256 {
        let actual = hex(&hasher.finalize());
        if !expected.eq_ignore_ascii_case(&actual) {
            let _ = std::fs::remove_file(dest);
            return Err(format!(
                "integrity check failed: expected sha256 {}, got {}",
                expected, actual
            ));
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_host_port_gets_a_scheme() {
        assert_eq!(normalize_proxy(" 127.0.0.1:10808 "), "http://127.0.0.1:10808");
        assert_eq!(normalize_proxy("10.0.0.2:3128"), "http://10.0.0.2:3128");
    }

    #[test]
    fn explicit_schemes_are_kept() {
        assert_eq!(
            normalize_proxy("socks5h://127.0.0.1:1080"),
            "socks5h://127.0.0.1:1080"
        );
        assert_eq!(normalize_proxy("HTTP://10.0.0.2:3128"), "HTTP://10.0.0.2:3128");
    }

    #[test]
    fn junk_is_rejected_instead_of_becoming_a_dead_route() {
        assert_eq!(normalize_proxy(""), "");
        assert_eq!(normalize_proxy("   "), "");
        assert_eq!(normalize_proxy("not a proxy"), "");
        assert_eq!(normalize_proxy("localhost"), "");
        assert_eq!(normalize_proxy("ftp://127.0.0.1:21"), "");
    }

    #[test]
    fn configured_proxies_come_first_and_direct_is_always_last() {
        set_proxies(vec![
            "127.0.0.1:10808".into(),
            "http://127.0.0.1:10808".into(),
            "".into(),
        ]);
        let r = routes();
        assert_eq!(r[0], Route::Proxy("http://127.0.0.1:10808".into()));
        // the duplicate and the empty entry were dropped
        assert_eq!(r.len(), 3);
        assert_eq!(r.last(), Some(&Route::Direct));
        set_proxies(Vec::new());
    }

    #[test]
    fn failure_message_names_each_route_and_the_next_step() {
        let msg = failure_message(
            "https://example.com/a.bin",
            &["  - direct connection: connection refused".into()],
        );
        assert!(msg.contains("https://example.com/a.bin"));
        assert!(msg.contains("direct connection: connection refused"));
        assert!(msg.contains("download proxy in Settings"));
    }
}
