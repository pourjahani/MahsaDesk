use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Application settings, persisted to `settings.json` in the app config dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub language: String,        // "en" | "fa"
    pub theme: String,           // "auto" | "light" | "dark"
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
    pub mode: String,          // "systemProxy" | "tun" | "psiphon-only" | "psiphon-after"
    pub failover_mode: String, // "off" | "round-robin" | "failover"
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
    // Local subscription sharing (#16)
    pub share_port: u16,         // default 18081
    pub share_lan: bool,         // false = loopback only
    // First-run wizard (#24): None until decided. `load` resolves a missing
    // settings file (fresh install) to false and an existing file without the
    // key (upgrade from an older version) to true, so upgrades are never nagged.
    pub wizard_done: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: "en".into(),
            theme: "auto".into(),
            socks_port: 10808,
            http_port: 10809,
            loglevel: "warning".into(),
            sniffing: true,
            allow_insecure: false,
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
            mode: "systemProxy".into(),
            failover_mode: "off".into(),
            psiphon_socks_port: 7501,
            psiphon_http_port: 7502,
            psiphon_region: String::new(),
            psiphon_protocol: "auto".into(),
            psiphon_aggressive: false,
            ech_enabled: false,
            ech_config: String::new(),
            front_enabled: false,
            front_host: String::new(),
            share_port: 18081,
            share_lan: false,
            wizard_done: None,
        }
    }
}

impl Settings {
    pub fn load(dir: &PathBuf) -> Settings {
        let path = dir.join("settings.json");
        match fs::read_to_string(&path) {
            Ok(text) => {
                let mut s: Settings = serde_json::from_str(&text).unwrap_or_default();
                if s.wizard_done.is_none() {
                    // The settings file already existed: not a first run
                    // (upgrade from an older version, or a corrupt file).
                    s.wizard_done = Some(true);
                }
                s
            }
            Err(_) => {
                let mut s = Settings::default();
                // No settings file at all: this is a fresh install.
                s.wizard_done = Some(false);
                s
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mahsa-wizard-{}-{}", name, std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn fresh_install_asks_for_the_wizard() {
        let d = tmp("fresh");
        assert_eq!(Settings::load(&d).wizard_done, Some(false));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn upgrade_without_the_flag_never_nags() {
        let d = tmp("upgrade");
        fs::write(d.join("settings.json"), "{\"language\":\"fa\"}").unwrap();
        assert_eq!(Settings::load(&d).wizard_done, Some(true));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn recorded_choice_is_kept() {
        let d = tmp("recorded");
        fs::write(d.join("settings.json"), "{\"wizard_done\":false}").unwrap();
        assert_eq!(Settings::load(&d).wizard_done, Some(false));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn fresh_defaults_keep_certificate_verification() {
        // allow_insecure disables TLS certificate verification for every server
        // connection: a fresh install must fail closed, not open. Stored settings
        // files keep whatever the user already had.
        assert!(!Settings::default().allow_insecure);
    }
}
