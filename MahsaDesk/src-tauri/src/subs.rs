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
    let resp = client
        .get(url)
        .header("User-Agent", "MahsaDesk/0.1")
        .send()
        .await
        .map_err(|e| format!("fetch: {}", e))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    Ok(resp.text().await.map_err(|e| e.to_string())?)
}
