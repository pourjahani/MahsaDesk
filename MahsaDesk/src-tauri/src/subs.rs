use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub update_interval_hours: u32,
    pub last_update: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SubsFile {
    pub subs: Vec<Subscription>,
}

pub const SUBS_FILE: &str = "subscriptions.json";

pub fn load_subs(dir: &PathBuf) -> SubsFile {
    let path = dir.join(SUBS_FILE);
    fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_subs(dir: &PathBuf, subs: &SubsFile) -> Result<(), String> {
    let path = dir.join(SUBS_FILE);
    let json = serde_json::to_string_pretty(subs).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

/// Fetch a subscription URL and return the raw payload (base64 or plain link list).
pub async fn fetch_payload(url: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let mut resp = client
        .get(url)
        .header("User-Agent", "MahsaDesk/0.1")
        .send()
        .await
        .map_err(|e| {
            format!(
                "subscription fetch failed: {}. Check internet access or connect through a working proxy, then retry.",
                e
            )
        })?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    // Cap the body: a hostile or broken endpoint must not be able to exhaust
    // memory - the biggest real subscriptions are a few MB.
    const MAX_BODY: usize = 32 * 1024 * 1024;
    if resp.content_length().unwrap_or(0) as usize > MAX_BODY {
        return Err("subscription too large (over 32 MB)".into());
    }
    let mut buf: Vec<u8> = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
        if buf.len() + chunk.len() > MAX_BODY {
            return Err("subscription too large (over 32 MB)".into());
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}
