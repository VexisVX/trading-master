//! User-tunable parameters for the v2 equity engine.
//!
//! All Decimals are stored as strings in SQLite, parsed to Decimal in Rust.
//! Legacy fields from v1 (r_min, delta_r, q_retention, d_trailing, dd_1..3,
//! m_dd_1..4, m_loss_table, n_max, mu_max, recovery_target) are silently
//! ignored by serde when present in old JSON configs.

use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityConfig {
    // ---- Risk baseline ----
    /// R_0 — base risk fraction of equity per trade (0.01 = 1%).
    pub r0: String,

    // ---- Profit protection (smoothstep) ----
    /// NAV growth at which F_profit activates (e.g. 0.05 = 5%).
    #[serde(default = "def_profit_activation")]
    pub profit_protect_activation: String,
    /// NAV growth at which F_profit reaches max retention (e.g. 0.10 = 10%).
    #[serde(default = "def_profit_full")]
    pub profit_protect_full: String,
    /// Max fraction of profit kept as protected (e.g. 0.40 = 40%).
    #[serde(default = "def_max_retention")]
    pub max_profit_retention: String,
    /// Internal trailing DD from NAV HWM (e.g. 0.10 = 10%).
    #[serde(default = "def_trailing_dd")]
    pub trailing_dd_pct: String,

    // ---- Prop-firm drawdown (independent of F_internal) ----
    /// Legacy D_prop (money). Kept for backward compat; 0 = disabled.
    #[serde(default)]
    pub d_prop: String,

    // ---- Risk capacity ----
    /// Alpha — fraction of loss capacity that can be risked.
    #[serde(default = "def_alpha")]
    pub alpha: String,
    /// Daily loss limit as fraction of equity (0 = disabled).
    #[serde(default)]
    pub daily_loss_limit: String,
    /// Portfolio max risk as fraction of equity (0 = disabled).
    #[serde(default)]
    pub r_portfolio_max: String,
    /// Per-trade prop max as fraction of equity (0 = disabled).
    #[serde(default)]
    pub r_prop_max: String,

    // ---- Execution buffer (audit §6) ----
    /// Slippage reserve as fraction of equity (e.g. 0.001 = 0.1%).
    #[serde(default = "def_exec_buffer")]
    pub execution_buffer_pct: String,

    // ---- Recovery ----
    /// Probe risk fraction of R_base when in RecoveryMode (e.g. 0.25 = 25%).
    #[serde(default = "def_recovery_probe")]
    pub recovery_probe_pct: String,

    // ---- Snapshot policy ----
    #[serde(default = "def_max_age")]
    pub max_snapshot_age_hours: i64,

    // ---- PHASE 2.2: market data freshness ----
    /// Max age (minutes) for current_price before considered stale.
    #[serde(default = "def_max_price_age")]
    pub max_price_age_minutes: i64,

    // ---- PHASE 2.2: daily reset ----
    /// IANA timezone for daily reset (e.g. "UTC", "Europe/London").
    #[serde(default = "def_reset_tz")]
    pub daily_reset_tz: String,
    /// Daily reset time in HH:MM (local to daily_reset_tz).
    #[serde(default = "def_reset_time")]
    pub daily_reset_time: String,

    // ---- Legacy fields (accepted but ignored) ----
    #[serde(default, skip_serializing)]
    pub r_min: String,
    #[serde(default, skip_serializing)]
    pub delta_r: String,
    #[serde(default, skip_serializing)]
    pub dd_1: String,
    #[serde(default, skip_serializing)]
    pub dd_2: String,
    #[serde(default, skip_serializing)]
    pub dd_3: String,
    #[serde(default, skip_serializing)]
    pub m_dd_1: String,
    #[serde(default, skip_serializing)]
    pub m_dd_2: String,
    #[serde(default, skip_serializing)]
    pub m_dd_3: String,
    #[serde(default, skip_serializing)]
    pub m_dd_4: String,
    #[serde(default, skip_serializing)]
    pub m_loss_table: String,
    #[serde(default, skip_serializing)]
    pub n_max: i64,
    #[serde(default, skip_serializing)]
    pub q_retention: String,
    #[serde(default, skip_serializing)]
    pub d_trailing: String,
    #[serde(default, skip_serializing)]
    pub mu_max: String,
    #[serde(default, skip_serializing)]
    pub recovery_target: String,
}

fn def_profit_activation() -> String {
    "0.05".into()
}
fn def_profit_full() -> String {
    "0.10".into()
}
fn def_max_retention() -> String {
    "0.40".into()
}
fn def_trailing_dd() -> String {
    "0.10".into()
}
fn def_alpha() -> String {
    "0.5".into()
}
fn def_exec_buffer() -> String {
    "0.001".into()
}
fn def_recovery_probe() -> String {
    "0.25".into()
}
fn def_max_age() -> i64 {
    24
}
fn def_max_price_age() -> i64 {
    60
}
fn def_reset_tz() -> String {
    "UTC".into()
}
fn def_reset_time() -> String {
    "00:00".into()
}

impl EquityConfig {
    pub fn default_v2() -> Self {
        Self {
            r0: "0.01".into(),
            profit_protect_activation: "0.05".into(),
            profit_protect_full: "0.10".into(),
            max_profit_retention: "0.40".into(),
            trailing_dd_pct: "0.10".into(),
            d_prop: "0".into(),
            alpha: "0.5".into(),
            daily_loss_limit: "0".into(),
            r_portfolio_max: "0".into(),
            r_prop_max: "0".into(),
            execution_buffer_pct: "0.001".into(),
            recovery_probe_pct: "0.25".into(),
            max_snapshot_age_hours: 24,
            max_price_age_minutes: 60,
            daily_reset_tz: "UTC".into(),
            daily_reset_time: "00:00".into(),
            // legacy
            r_min: String::new(),
            delta_r: String::new(),
            dd_1: String::new(),
            dd_2: String::new(),
            dd_3: String::new(),
            m_dd_1: String::new(),
            m_dd_2: String::new(),
            m_dd_3: String::new(),
            m_dd_4: String::new(),
            m_loss_table: String::new(),
            n_max: 0,
            q_retention: String::new(),
            d_trailing: String::new(),
            mu_max: String::new(),
            recovery_target: String::new(),
        }
    }

    /// Alias so old callers `EquityConfig::default_v1()` keep working.
    pub fn default_v1() -> Self {
        Self::default_v2()
    }

    pub fn validate(&self) -> Result<(), String> {
        let d = |s: &str, name: &str| -> Result<Decimal, String> {
            if s.is_empty() {
                return Ok(Decimal::ZERO);
            }
            Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
        };
        let _ = d(&self.r0, "r0")?;
        let _ = d(&self.profit_protect_activation, "profit_protect_activation")?;
        let _ = d(&self.profit_protect_full, "profit_protect_full")?;
        let _ = d(&self.max_profit_retention, "max_profit_retention")?;
        let _ = d(&self.trailing_dd_pct, "trailing_dd_pct")?;
        let _ = d(&self.d_prop, "d_prop")?;
        let _ = d(&self.alpha, "alpha")?;
        let _ = d(&self.daily_loss_limit, "daily_loss_limit")?;
        let _ = d(&self.r_portfolio_max, "r_portfolio_max")?;
        let _ = d(&self.r_prop_max, "r_prop_max")?;
        let _ = d(&self.execution_buffer_pct, "execution_buffer_pct")?;
        let _ = d(&self.recovery_probe_pct, "recovery_probe_pct")?;
        if self.max_snapshot_age_hours < 1 {
            return Err("max_snapshot_age_hours must be >= 1".into());
        }
        if self.max_price_age_minutes < 1 {
            return Err("max_price_age_minutes must be >= 1".into());
        }
        if self.daily_reset_tz.trim().is_empty() {
            return Err("daily_reset_tz must not be empty".into());
        }
        // Validate HH:MM format
        let parts: Vec<&str> = self.daily_reset_time.split(':').collect();
        if parts.len() != 2 {
            return Err("daily_reset_time must be HH:MM".into());
        }
        let hh: u32 = parts[0]
            .parse()
            .map_err(|_| "daily_reset_time hour invalid".to_string())?;
        let mm: u32 = parts[1]
            .parse()
            .map_err(|_| "daily_reset_time minute invalid".to_string())?;
        if hh > 23 || mm > 59 {
            return Err("daily_reset_time out of range".into());
        }
        Ok(())
    }

    /// Legacy accessor for old callers. Returns default loss table.
    pub fn loss_table(&self) -> Result<Vec<Decimal>, String> {
        Ok(vec![Decimal::ONE])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_v2_parses() {
        let c = EquityConfig::default_v2();
        c.validate().unwrap();
    }

    #[test]
    fn backward_compat_old_json() {
        // Legacy JSON with all old fields.
        let json = r#"{
            "r0": "0.01",
            "r_min": "0.0025",
            "delta_r": "0.001",
            "dd_1": "0.05",
            "dd_2": "0.10",
            "dd_3": "0.15",
            "m_dd_1": "1.0",
            "m_dd_2": "0.75",
            "m_dd_3": "0.50",
            "m_dd_4": "0.25",
            "m_loss_table": "1.0,0.8,0.6,0.4,0.2,0.0",
            "n_max": 5,
            "q_retention": "0.3",
            "d_trailing": "0.1",
            "d_prop": "0",
            "alpha": "0.5",
            "daily_loss_limit": "0",
            "r_portfolio_max": "0",
            "r_prop_max": "0",
            "mu_max": "0.8",
            "max_snapshot_age_hours": 24,
            "recovery_target": "0.8"
        }"#;
        let c: EquityConfig = serde_json::from_str(json).unwrap();
        assert_eq!(c.r0, "0.01");
        // New fields get defaults.
        assert_eq!(c.profit_protect_activation, "0.05");
        assert_eq!(c.max_profit_retention, "0.40");
        assert_eq!(c.recovery_probe_pct, "0.25");
        c.validate().unwrap();
    }

    #[test]
    fn round_trip_v2() {
        let c = EquityConfig::default_v2();
        let json = serde_json::to_string(&c).unwrap();
        let c2: EquityConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(c2.r0, c.r0);
        assert_eq!(c2.profit_protect_activation, c.profit_protect_activation);
    }
}
