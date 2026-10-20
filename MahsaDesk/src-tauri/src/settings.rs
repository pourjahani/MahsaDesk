use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings, persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,        // "en" | "fa"
    pub socks_port: u16,         // default 10808
    pub http_port: u16,          // default 10809
    pub loglevel: String,        // debug|info|warning|error|none
    pub sniffing: bool,
    pub allow_insecure: bool,
    pub mux: bool,
    pub ipv6: bool,
    pub remote_dns: String,      // e.g. "1.1.1.1"
    pub routing_preset: String,  // "global" | "ir" | "ir_cn"
    pub custom_proxy: String,    // newline separated domains
    pub custom_direct: String,
    pub custom_block: String,
    pub minimize_to_tray: bool,
    pub launch_at_login: bool,
    pub auto_connect: bool,
    pub active_server: String,   // server id
    pub fragment_enabled: bool,
    pub fragment_packets: String, // "tlshello" | "1-3" (first TLS+first N segments)
    pub fragment_length: String,  // e.g. "80-200"
    pub fragment_interval: String,// e.g. "10-20"
    pub cdn_mode: String,        // "off" | "manual"
    pub cdn_ip: String,          // e.g. "104.18.0.25"
    pub domain_strategy: String, // IPIfNonMatch
    pub proxy_sharing: bool,     // listen on 0.0.0.0 instead of 127.0.0.1
    pub per_app_mode: String,    // "off" | "bypass" | "allow" (TUN only)
    pub per_app_list: String,    // newline separated process names
    // Psiphon tunnel (psiphon only / psiphon after)
    pub psiphon_mode: String,    // "off" | "only" | "after"
    pub psiphon_socks_port: u16, // default 7501
    pub psiphon_http_port: u16,  // default 7502
    pub psiphon_region: String,  // "" = auto, else 2-letter country
    pub psiphon_protocol: String, // "auto" | "normal" | "conduit"
    pub psiphon_aggressive: bool,
    // Encrypted Client Hello (Xray tlsSettings.echConfigList)
    pub ech_enabled: bool,
    pub ech_config: String,      // base64 ECHConfigList or DoH URL; empty = auto
    // Domain fronting (fake host)
    pub front_enabled: bool,
    pub front_host: String,      // CDN domain to connect to
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "en".into(),
            socks_port: 10808,
            http_port: 10809,
            loglevel: "warning".into(),
            sniffing: true,
            allow_insecure: true,
            mux: false,
            ipv6: false,
            remote_dns: "1.1.1.1".into(),
            routing_preset: "global".into(),
            custom_proxy: String::new(),
            custom_direct: String::new(),
            custom_block: String::new(),
            minimize_to_tray: true,
            launch_at_login: false,
            auto_connect: false,
            active_server: String::new(),
            fragment_enabled: false,
            fragment_packets: "tlshello".into(),
            fragment_length: "80-200".into(),
            fragment_interval: "10-20".into(),
            cdn_mode: "off".into(),
            cdn_ip: String::new(),
            domain_strategy: "IPIfNonMatch".into(),
            proxy_sharing: false,
            per_app_mode: "off".into(),
            per_app_list: String::new(),
            psiphon_mode: "off".into(),
            psiphon_socks_port: 7501,
            psiphon_http_port: 7502,
            psiphon_region: String::new(),
            psiphon_protocol: "auto".into(),
            psiphon_aggressive: false,
            ech_enabled: false,
            ech_config: String::new(),
            front_enabled: false,
            front_host: String::new(),
        }
    }
}

impl Settings {
    pub fn load(dir: &PathBuf) -> Settings {
        let path = dir.join("settings.json");
        match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, dir: &PathBuf) -> Result<(), String> {
        let path = dir.join("settings.json");
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, json).map_err(|e| e.to_string())
    }

    /// Domains entered by the user split into non-empty trimmed entries.
    #[allow(dead_code)]
    pub fn split_list(&self, raw: &str) -> Vec<String> {
        raw.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| format!("domain:{}", l))
            .collect()
    }
}
