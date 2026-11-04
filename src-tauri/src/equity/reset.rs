//! Daily reset boundary (PHASE 2.2).
//!
//! Precedence (highest wins):
//!   1. prop_firm_rule_sets.reset_tz / reset_time  (if a prop rule exists)
//!   2. account_profiles.daily_reset_tz / daily_reset_time
//!   3. EquityConfig.daily_reset_tz / daily_reset_time
//!   4. UTC / 00:00
//!
//! Per-account, never global.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

use crate::equity::config::EquityConfig;

#[derive(Debug, Clone)]
pub struct ResetBoundary {
    pub tz_name: String,
    pub reset_time: String, // HH:MM
}

/// Parse a timezone name safely. Returns UTC if invalid.
fn parse_tz(name: &str) -> Tz {
    name.parse::<Tz>().unwrap_or(chrono_tz::UTC)
}

/// Parse "HH:MM". Returns (0,0) if invalid.
fn parse_hhmm(s: &str) -> (u32, u32) {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return (0, 0);
    }
    let hh = parts[0].parse().unwrap_or(0);
    let mm = parts[1].parse().unwrap_or(0);
    (hh.min(23), mm.min(59))
}

/// Compute the most recent reset boundary at or before `now`,
/// in the configured timezone and time.
pub fn last_boundary(b: &ResetBoundary, now: DateTime<Utc>) -> DateTime<Utc> {
    let tz = parse_tz(&b.tz_name);
    let (hh, mm) = parse_hhmm(&b.reset_time);

    let now_local = now.with_timezone(&tz);
    let today = now_local.date_naive();
    let candidate = today
        .and_hms_opt(hh, mm, 0)
        .and_then(|naive: NaiveDateTime| tz.from_local_datetime(&naive).single())
        .map(|dt| dt.with_timezone(&Utc));

    match candidate {
        Some(utc) if utc <= now => utc,
        _ => {
            // Fall back to yesterday
            let yesterday = today.pred_opt().unwrap_or(today);
            yesterday
                .and_hms_opt(hh, mm, 0)
                .and_then(|naive| tz.from_local_datetime(&naive).single())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or(now)
        }
    }
}

/// Resolve the boundary from a config fallback. (Precedence resolution
/// happens in the caller, which has access to prop/profile rows.)
pub fn from_config(cfg: &EquityConfig) -> ResetBoundary {
    ResetBoundary {
        tz_name: if cfg.daily_reset_tz.trim().is_empty() {
            "UTC".into()
        } else {
            cfg.daily_reset_tz.clone()
        },
        reset_time: if cfg.daily_reset_time.trim().is_empty() {
            "00:00".into()
        } else {
            cfg.daily_reset_time.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn utc_midnight() {
        let now = Utc.with_ymd_and_hms(2026, 3, 15, 14, 0, 0).unwrap();
        let b = ResetBoundary { tz_name: "UTC".into(), reset_time: "00:00".into() };
        let boundary = last_boundary(&b, now);
        assert_eq!(boundary, Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap());
    }

    #[test]
    fn london_before_utc_midnight() {
        // At 23:30 UTC on Mar 15 (London = UTC during winter, BST during summer)
        // Let's pick winter: Jan 15 2026 — London = UTC
        let now = Utc.with_ymd_and_hms(2026, 1, 15, 23, 30, 0).unwrap();
        let b = ResetBoundary { tz_name: "Europe/London".into(), reset_time: "00:00".into() };
        let boundary = last_boundary(&b, now);
        assert_eq!(boundary, Utc.with_ymd_and_hms(2026, 1, 15, 0, 0, 0).unwrap());
    }

    #[test]
    fn new_york_after_utc_midnight() {
        // At 02:00 UTC on Jan 16, New York (UTC-5) = Jan 15 21:00
        // So boundary should be Jan 15 05:00 UTC (midnight NY)
        let now = Utc.with_ymd_and_hms(2026, 1, 16, 2, 0, 0).unwrap();
        let b = ResetBoundary { tz_name: "America/New_York".into(), reset_time: "00:00".into() };
        let boundary = last_boundary(&b, now);
        // Jan 15 00:00 NY = Jan 15 05:00 UTC
        assert_eq!(boundary, Utc.with_ymd_and_hms(2026, 1, 15, 5, 0, 0).unwrap());
    }

    #[test]
    fn custom_reset_time() {
        // Reset at 17:00 UTC. At 18:00 UTC, boundary is today 17:00
        let now = Utc.with_ymd_and_hms(2026, 3, 15, 18, 0, 0).unwrap();
        let b = ResetBoundary { tz_name: "UTC".into(), reset_time: "17:00".into() };
        let boundary = last_boundary(&b, now);
        assert_eq!(boundary, Utc.with_ymd_and_hms(2026, 3, 15, 17, 0, 0).unwrap());
    }

    #[test]
    fn invalid_tz_falls_back_to_utc() {
        let now = Utc.with_ymd_and_hms(2026, 3, 15, 10, 0, 0).unwrap();
        let b = ResetBoundary { tz_name: "Invalid/Zone".into(), reset_time: "00:00".into() };
        let boundary = last_boundary(&b, now);
        assert_eq!(boundary, Utc.with_ymd_and_hms(2026, 3, 15, 0, 0, 0).unwrap());
    }
}
