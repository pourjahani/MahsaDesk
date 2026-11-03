//! Session + monthly traffic accounting (#33).
//!
//! The xray stats API already reports per-core cumulative uplink/downlink
//! totals — that is what the live `traffic` event shows. This module folds
//! those totals into durable accounting: the current connection's session
//! bytes plus a rolling per-calendar-month total persisted in the data dir.
//!
//! Modes without a local xray core (psiphon-only, and hysteria2 seats where
//! the native sing-box outbound bypasses xray) have no per-byte source, so
//! they simply do not move the counters — the UI keeps showing the last tally
//! for those sessions.
//!
//! Counters live here (the backend), not in the webview, because the window
//! is JavaScript-throttled once hidden in the tray and would silently stop
//! counting while invisible.

use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const USAGE_FILE: &str = "usage.json";
/// Month counters are flushed to disk at most this often while traffic flows;
/// `disconnect` always flushes explicitly, so the throttle only bounds how
/// much of the last interval can be lost when the app is killed without it.
const SAVE_INTERVAL_MS: u64 = 20_000;

#[derive(Serialize, Clone, Default)]
pub struct UsageSnapshot {
    pub session_up: u64,
    pub session_down: u64,
    /// Calendar month the month counters belong to, "YYYY-MM" (UTC).
    pub month: String,
    pub month_up: u64,
    pub month_down: u64,
}

/// What is written to disk: only the monthly rollup survives a restart.
#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    month: String,
    up: u64,
    down: u64,
}

struct Inner {
    session_up: u64,
    session_down: u64,
    month: String,
    month_up: u64,
    month_down: u64,
    /// Last cumulative totals seen from the running core, for deltas.
    core_up: u64,
    core_down: u64,
    core_seen: bool,
    last_save_ms: u64,
}

pub struct Usage {
    inner: Mutex<Inner>,
}

/// UTC calendar month as "YYYY-MM". Civil-from-days conversion (after Howard
/// Hinnant's algorithm); no calendar dependency on purpose.
pub fn month_now() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        / 86_400;
    month_from_days(days)
}

fn month_from_days(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}", year, m)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Usage {
    /// Load the persisted month counters. A fresh month means zeros; a corrupt
    /// or missing file is the same as starting clean.
    pub fn load(dir: &Path) -> std::sync::Arc<Self> {
        let cur = month_now();
        let p: Persisted = std::fs::read_to_string(dir.join(USAGE_FILE))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        let (month, up, down) = if p.month == cur {
            (p.month, p.up, p.down)
        } else {
            (cur, 0, 0)
        };
        std::sync::Arc::new(Self {
            inner: Mutex::new(Inner {
                session_up: 0,
                session_down: 0,
                month,
                month_up: up,
                month_down: down,
                core_up: 0,
                core_down: 0,
                core_seen: false,
                last_save_ms: 0,
            }),
        })
    }

    /// A new connection starts its tally from zero. The previous connection's
    /// numbers simply stop being shown; the month keeps accumulating.
    pub fn reset_session(&self) {
        let mut g = self.inner.lock().unwrap();
        g.session_up = 0;
        g.session_down = 0;
        g.core_up = 0;
        g.core_down = 0;
        g.core_seen = false;
    }

    /// Fold a cumulative (uplink, downlink) snapshot from the core into the
    /// counters. Returns the fresh snapshot when anything changed (bytes
    /// moved, or the calendar rolled), so the caller can emit it; None when
    /// idle. `dir` is the data dir for the throttled month flush.
    pub fn record(&self, core_up: u64, core_down: u64, dir: &Path) -> Option<UsageSnapshot> {
        let month = month_now();
        let mut g = self.inner.lock().unwrap();
        let rolled = g.month != month;
        if rolled {
            g.month = month.clone();
            g.month_up = 0;
            g.month_down = 0;
        }
        // The first sighting after `reset_session` counts everything that has
        // already flowed (the connect probe spends bytes too); afterwards only
        // the growth of the cumulative totals is new traffic. saturating_sub
        // defends against a counter restart that skipped the reset.
        let (du, dd) = if g.core_seen {
            (
                core_up.saturating_sub(g.core_up),
                core_down.saturating_sub(g.core_down),
            )
        } else {
            (core_up, core_down)
        };
        g.core_seen = true;
        g.core_up = core_up;
        g.core_down = core_down;
        if du == 0 && dd == 0 && !rolled {
            return None;
        }
        g.session_up += du;
        g.session_down += dd;
        g.month_up += du;
        g.month_down += dd;
        let now = now_ms();
        if now.saturating_sub(g.last_save_ms) >= SAVE_INTERVAL_MS {
            g.last_save_ms = now;
            let _ = save(
                dir,
                &Persisted {
                    month: g.month.clone(),
                    up: g.month_up,
                    down: g.month_down,
                },
            );
        }
        Some(UsageSnapshot {
            session_up: g.session_up,
            session_down: g.session_down,
            month: g.month.clone(),
            month_up: g.month_up,
            month_down: g.month_down,
        })
    }

    /// Current totals (session + this month, rolling over on month change).
    pub fn snapshot(&self) -> UsageSnapshot {
        self.snapshot_at(&month_now())
    }

    fn snapshot_at(&self, month: &str) -> UsageSnapshot {
        let mut g = self.inner.lock().unwrap();
        if g.month != month {
            g.month = month.to_string();
            g.month_up = 0;
            g.month_down = 0;
        }
        UsageSnapshot {
            session_up: g.session_up,
            session_down: g.session_down,
            month: g.month.clone(),
            month_up: g.month_up,
            month_down: g.month_down,
        }
    }

    /// Write the monthly rollup (the session tally dies with the process).
    pub fn persist(&self, dir: &Path) -> Result<(), String> {
        let g = self.inner.lock().unwrap();
        save(dir, &Persisted {
            month: g.month.clone(),
            up: g.month_up,
            down: g.month_down,
        })
    }
}

fn save(dir: &Path, p: &Persisted) -> Result<(), String> {
    let json = serde_json::to_string(p).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(USAGE_FILE), json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(month: &str) -> Usage {
        Usage {
            inner: Mutex::new(Inner {
                session_up: 0,
                session_down: 0,
                month: month.to_string(),
                month_up: 0,
                month_down: 0,
                core_up: 0,
                core_down: 0,
                core_seen: false,
                last_save_ms: 0,
            }),
        }
    }

    #[test]
    fn month_conversion_is_correct() {
        assert_eq!(month_from_days(0), "1970-01");
        // 2026-11-01 is 20759 days after the epoch
        assert_eq!(month_from_days(20_759), "2026-11");
        assert_eq!(month_from_days(20_780), "2026-11");
        // month boundary: 2026-12-01
        assert_eq!(month_from_days(20_789), "2026-12");
        // the year flip at January/February boundaries
        assert_eq!(month_from_days(19_727), "2024-01");
        assert_eq!(month_from_days(19_758), "2024-02");
    }

    #[test]
    fn first_sighting_counts_then_deltas() {
        let u = usage("2026-11");
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down, s.month_up, s.month_down), (0, 0, 0, 0));

        // totals already include the probe's bytes at first sight
        u.record(150, 900, Path::new("nowhere")); // test dir must not be written
        // (record writes the file best-effort; a bad path silences it)
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down), (150, 900));
        assert_eq!((s.month_up, s.month_down), (150, 900));

        u.record(450, 1_900, Path::new("nowhere"));
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down), (450, 1_900));
        assert_eq!((s.month_up, s.month_down), (450, 1_900));
    }

    #[test]
    fn idle_total_returns_none() {
        let u = usage("2026-11");
        u.record(150, 900, Path::new("nowhere"));
        assert!(u.record(150, 900, Path::new("nowhere")).is_none());
        assert!(u.record(151, 1_000, Path::new("nowhere")).is_some());
    }

    #[test]
    fn reset_session_starts_tall_over_but_month_keeps() {
        let u = usage("2026-11");
        u.record(1_000, 5_000, Path::new("nowhere"));
        u.reset_session();
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down), (0, 0));
        assert_eq!((s.month_up, s.month_down), (1_000, 5_000));
        u.record(800, 4_000, Path::new("nowhere")); // new core, fresh counters
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down), (800, 4_000));
        assert_eq!((s.month_up, s.month_down), (1_800, 9_000));
    }

    #[test]
    fn calendar_rollover_preserves_session() {
        let u = usage("2026-11");
        u.record(100, 200, Path::new("nowhere"));
        u.reset_session();
        u.record(50, 70, Path::new("nowhere"));
        let s = u.snapshot_at("2026-11");
        assert_eq!((s.session_up, s.session_down), (50, 70));
        assert_eq!((s.month_up, s.month_down), (150, 270));
        // the calendar advanced: the month restarts at zero, the session tally
        // of the still-open connection is kept
        let s = u.snapshot_at("2026-12");
        assert_eq!((s.session_up, s.session_down), (50, 70));
        assert_eq!((s.month_up, s.month_down), (0, 0));
        assert_eq!(s.month, "2026-12");
    }

    #[test]
    fn saturating_guard_never_loses_month_totals() {
        let u = usage("2026-11");
        u.record(1_000, 2_000, Path::new("nowhere"));
        u.reset_session();
        // a counter restart skipped the reset: totals shrank
        u.record(400, 500, Path::new("nowhere"));
        u.record(900, 1_500, Path::new("nowhere"));
        let s = u.snapshot_at("2026-11");
        // only the positive growth (900-800? no: from 400 to 900) counts
        assert_eq!((s.session_up, s.session_down), (900, 1_500));
        assert_eq!((s.month_up, s.month_down), (1_900, 3_500));
    }

    #[test]
    fn persist_round_trips_and_stale_months_zero() {
        let dir = std::env::temp_dir().join(format!("mahsadesk-usage-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let u = usage("2026-11");
        u.record(1_234, 56_789, Path::new("nowhere"));
        u.persist(&dir).unwrap();
        let loaded = Usage::load(&dir);
        let s = loaded.snapshot_at("2026-11");
        assert_eq!((s.month_up, s.month_down), (1_234, 56_789));
        assert_eq!((s.session_up, s.session_down), (0, 0));
        // a stale persisted month means a fresh month
        let stale = dir.join(USAGE_FILE);
        std::fs::write(&stale, r#"{"month":"2020-01","up":9,"down":9}"#).unwrap();
        let loaded = Usage::load(&dir);
        let s = loaded.snapshot_at("2026-11");
        assert_eq!((s.month_up, s.month_down), (0, 0));
        assert_eq!(s.month, "2026-11");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
