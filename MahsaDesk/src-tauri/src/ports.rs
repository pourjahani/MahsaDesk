//! Local port availability (#6: auto-resolve conflicts before connecting).
//!
//! A fresh install that collides with another local service (a second proxy,
//! a dev server, a stale core) is one of the most common first-connect
//! failures. Before every connect we probe the ports the config is about to
//! bind and shift them to the next free port for the session — the stored
//! settings are never rewritten, and every shift is reported to the UI.

use crate::settings::Settings;

/// True when loopback cannot bind `port` right now (something is on it).
pub fn is_busy(port: u16) -> bool {
    if port == 0 {
        return false;
    }
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

/// First free port at or above `start`, skipping `taken`. Searches up to 200
/// candidates; returns None when the whole range is occupied.
pub fn find_free(start: u16, taken: &[u16]) -> Option<u16> {
    let base = start as u32;
    for p in base..base + 200 {
        if p > u16::MAX as u32 {
            break;
        }
        let p = p as u16;
        if p != 0 && !taken.contains(&p) && !is_busy(p) {
            return Some(p);
        }
    }
    None
}

fn shift(
    label: &str,
    value: &mut u16,
    taken: &mut Vec<u16>,
    changes: &mut Vec<(String, u16, u16)>,
) {
    if is_busy(*value) || taken.contains(value) {
        if let Some(fresh) = find_free(*value, taken) {
            changes.push((label.to_string(), *value, fresh));
            *value = fresh;
        }
    }
    if !taken.contains(value) {
        taken.push(*value);
    }
}

/// Shift SOCKS + HTTP (+ the stats API port if it is taken by another core)
/// off busy ports for this session. Returns the list of changes.
pub fn resolve_socks_http(settings: &mut Settings) -> Vec<(String, u16, u16)> {
    let mut taken = Vec::new();
    let mut changes = Vec::new();
    shift("socks", &mut settings.socks_port, &mut taken, &mut changes);
    shift("http", &mut settings.http_port, &mut taken, &mut changes);
    changes
}

/// Shift the Psiphon SOCKS/HTTP pair off busy ports for this session.
pub fn resolve_psiphon(settings: &mut Settings) -> Vec<(String, u16, u16)> {
    let mut taken = Vec::new();
    let mut changes = Vec::new();
    shift(
        "psiphon-socks",
        &mut settings.psiphon_socks_port,
        &mut taken,
        &mut changes,
    );
    shift(
        "psiphon-http",
        &mut settings.psiphon_http_port,
        &mut taken,
        &mut changes,
    );
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hold_ephemeral() -> std::net::TcpListener {
        std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap()
    }

    #[test]
    fn busy_port_is_detected() {
        let l = hold_ephemeral();
        let busy = l.local_addr().unwrap().port();
        assert!(is_busy(busy));
        drop(l);
    }

    #[test]
    fn busy_port_is_shifted_and_reported() {
        let l = hold_ephemeral();
        let busy = l.local_addr().unwrap().port();
        let mut s = Settings::default();
        s.socks_port = busy;
        s.http_port = busy; // both want the same occupied port
        let changes = resolve_socks_http(&mut s);
        assert!(
            changes
                .iter()
                .any(|(label, old, new)| label == "socks" && *old == busy && *new != busy),
            "socks should shift off the busy port: {:?}",
            changes
        );
        assert_ne!(s.socks_port, busy);
        assert_ne!(s.socks_port, s.http_port, "socks and http must differ");
        // every reported shift must have been a genuinely occupied port
        for (_, old, _) in &changes {
            assert!(is_busy(*old), "reported old port {} was not busy", old);
        }
        for (_, _, new) in &changes {
            assert_ne!(*new, busy, "must never shift onto the busy port");
        }
    }

    #[test]
    fn find_free_skips_busy_and_taken_ports() {
        let l = hold_ephemeral();
        let busy = l.local_addr().unwrap().port();
        assert_ne!(
            find_free(busy, &[]),
            Some(busy),
            "must not hand back a busy port"
        );
        if let Some(first) = find_free(busy, &[]) {
            assert_ne!(
                find_free(busy, &[first]),
                Some(first),
                "must skip ports from the taken list"
            );
        }
    }
}
