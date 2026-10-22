//! Per-ISP anti-filtering presets (#22).
//!
//! MahsaNG ships its per-ISP values inside `remote_config_update_v*.json`,
//! but that payload is base64 + encrypted and its decryptor
//! (`config_pkg_reader`) is deliberately closed-source — verified against the
//! upstream repo on 2026-10-22. What *is* public is MahsaNet's published
//! fragment guidance (github.com/mahsanet/MahsaFreeConfig `fragment/`), which
//! these presets encode, plus the differences users actually observe per
//! network. The UI labels this honestly instead of pretending to decrypt
//! MahsaNG's config.

use crate::settings::Settings;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct IspPreset {
    pub id: String,
    pub fragment_enabled: bool,
    pub packets: String,
    pub length: String,
    pub interval: String,
}

/// Fragment presets keyed by ISP (labels/notes come from i18n by id).
///
/// - mci/rightel: TLS-heavy DPI -> MahsaNet's published TLS recommendation
///   (tlshello, 10-20, 10-20).
/// - tci: the app's wider proven default (80-200 first fragment) which has
///   historically worked on TCI-family networks.
/// - mtn: Irancell generally does not throttle by SNI -> fragmentation off
///   avoids needless handshake overhead.
/// - off: no fragmentation at all.
pub fn presets() -> Vec<IspPreset> {
    vec![
        IspPreset {
            id: "mci".into(),
            fragment_enabled: true,
            packets: "tlshello".into(),
            length: "10-20".into(),
            interval: "10-20".into(),
        },
        IspPreset {
            id: "mtn".into(),
            fragment_enabled: false,
            packets: "tlshello".into(),
            length: "10-20".into(),
            interval: "10-20".into(),
        },
        IspPreset {
            id: "tci".into(),
            fragment_enabled: true,
            packets: "tlshello".into(),
            length: "80-200".into(),
            interval: "10-20".into(),
        },
        IspPreset {
            id: "rightel".into(),
            fragment_enabled: true,
            packets: "tlshello".into(),
            length: "10-20".into(),
            interval: "10-20".into(),
        },
        IspPreset {
            id: "off".into(),
            fragment_enabled: false,
            packets: "tlshello".into(),
            length: "80-200".into(),
            interval: "10-20".into(),
        },
    ]
}

/// Write one preset's fragment fields into settings. Returns false for an
/// unknown id (nothing is changed in that case).
pub fn apply(settings: &mut Settings, id: &str) -> bool {
    match presets().into_iter().find(|p| p.id == id) {
        Some(p) => {
            settings.fragment_enabled = p.fragment_enabled;
            settings.fragment_packets = p.packets;
            settings.fragment_length = p.length;
            settings.fragment_interval = p.interval;
            true
        }
        None => false,
    }
}

/// Best-effort ISP id from an ip-api response (`isp`/`org` fields), used by
/// the UI to preselect a preset chip. Returns None when nothing matches.
pub fn detect_from_ipinfo(isp: &str, org: &str) -> Option<&'static str> {
    let hay = format!("{} {}", isp, org).to_lowercase();
    if hay.contains("mci") || hay.contains("hiweb") {
        Some("mci")
    } else if hay.contains("irancell") || hay.contains("mtn iran") || hay.contains("mobil ir") {
        Some("mtn")
    } else if hay.contains("rightel") {
        Some("rightel")
    } else if hay.contains("tci")
        || hay.contains("telecommunication")
        || hay.contains("irantel")
    {
        Some("tci")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_writes_fragment_fields() {
        let mut s = Settings::default();
        assert!(apply(&mut s, "mci"));
        assert!(s.fragment_enabled);
        assert_eq!(s.fragment_packets, "tlshello");
        assert_eq!(s.fragment_length, "10-20");
        assert_eq!(s.fragment_interval, "10-20");

        assert!(apply(&mut s, "mtn"));
        assert!(!s.fragment_enabled, "Irancell preset turns fragmentation off");

        assert!(apply(&mut s, "tci"));
        assert!(s.fragment_enabled);
        assert_eq!(s.fragment_length, "80-200");
    }

    #[test]
    fn unknown_preset_is_rejected_without_changes() {
        let mut s = Settings::default();
        s.fragment_length = "77-88".into();
        assert!(!apply(&mut s, "nope"));
        assert_eq!(s.fragment_length, "77-88");
        assert!(!s.fragment_enabled);
    }

    #[test]
    fn detect_maps_known_isps() {
        assert_eq!(detect_from_ipinfo("MCI", ""), Some("mci"));
        assert_eq!(detect_from_ipinfo("MTN Irancell", ""), Some("mtn"));
        assert_eq!(detect_from_ipinfo("RighTel", ""), Some("rightel"));
        assert_eq!(
            detect_from_ipinfo("Telecommunication Infrastructure Company", ""),
            Some("tci")
        );
        assert_eq!(detect_from_ipinfo("Some Foreign ISP", "Hosting"), None);
    }
}
