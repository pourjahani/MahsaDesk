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

/// Cap the body: a hostile or broken endpoint must not be able to exhaust
/// memory - the biggest real subscriptions are a few MB.
const MAX_BODY: usize = 32 * 1024 * 1024;

/// Fetch a subscription URL and return the raw payload (base64 or plain link
/// list).
///
/// Goes through `net::` like every other download, so a subscription link still
/// loads when the Windows proxy is stale but the direct route works, or when it
/// only works through the app's own tunnel.
pub async fn fetch_payload(url: &str) -> Result<String, String> {
    let url = url.trim();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("subscription URL must start with http:// or https://".into());
    }
    crate::net::get_text(url, std::time::Duration::from_secs(20), MAX_BODY).await
}
