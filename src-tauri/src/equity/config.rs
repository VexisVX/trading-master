//! User-tunable parameters. All Decimals are stored as strings in SQLite,
//! parsed to Decimal in Rust. No floats anywhere.

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde::{Deserialize, Serialize};

/// All fields are Decimal-friendly. Percentages use 0.0–1.0 (e.g. 0.05 = 5%).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityConfig {
    // ---- Base risk ----
    /// R_0 — base risk fraction of equity per trade (e.g. 0.01 = 1%).
    pub r0: String,
    /// Minimum R (floor for step-down).
    pub r_min: String,
    /// Step size for step-up / step-down.
    pub delta_r: String,

    // ---- Drawdown risk multipliers ----
    /// DD_1 (inclusive upper bound for tier 1) e.g. 0.05.
    pub dd_1: String,
    /// DD_2.
    pub dd_2: String,
    /// DD_3.
    pub dd_3: String,
    /// Multiplier at each tier (tier above dd_3 → 0.25).
    pub m_dd_1: String,
    pub m_dd_2: String,
    pub m_dd_3: String,
    pub m_dd_4: String,

    // ---- Loss-streak multipliers ----
    /// Comma-separated list: `1.0,0.8,0.6,0.4,0.2,0.0` (index = consecutive losses).
    pub m_loss_table: String,
    /// N_max — consecutive losses that trigger Paused.
    pub n_max: i64,

    // ---- Floors ----
    /// Q — profit-retention fraction (0.0–1.0).
    pub q_retention: String,
    /// d — trailing-from-HWM fraction (0.0–1.0).
    pub d_trailing: String,
    /// D_prop — prop firm drawdown budget (money).
    pub d_prop: String,

    // ---- Risk capacity ----
    /// alpha — fraction of loss capacity that can be risked (0.0–1.0).
    pub alpha: String,
    /// Daily loss limit (money).
    pub daily_loss_limit: String,
    /// Maximum concurrent portfolio risk (money).
    pub r_portfolio_max: String,
    /// Prop-firm per-trade max risk (money), 0 = disabled.
    pub r_prop_max: String,

    // ---- Margin ----
    /// MU_max fraction (0.0–1.0).
    pub mu_max: String,

    // ---- Snapshot policy ----
    /// Max age in hours before blocking decisions.
    pub max_snapshot_age_hours: i64,

    // ---- Recovery ----
    /// Recovery target (0.0–1.0). When recovery >= target, exit recovery mode.
    pub recovery_target: String,
}

impl EquityConfig {
    /// Defaults — well-documented and conservative.
    pub fn default_v1() -> Self {
        Self {
            r0: "0.01".into(),
            r_min: "0.0025".into(),
            delta_r: "0.001".into(),
            dd_1: "0.05".into(),
            dd_2: "0.10".into(),
            dd_3: "0.15".into(),
            m_dd_1: "1.0".into(),
            m_dd_2: "0.75".into(),
            m_dd_3: "0.50".into(),
            m_dd_4: "0.25".into(),
            m_loss_table: "1.0,0.8,0.6,0.4,0.2,0.0".into(),
            n_max: 5,
            q_retention: "0.3".into(),
            d_trailing: "0.1".into(),
            d_prop: "0".into(),
            alpha: "0.5".into(),
            daily_loss_limit: "0".into(),   // 0 = disabled
            r_portfolio_max: "0".into(),    // 0 = disabled
            r_prop_max: "0".into(),         // 0 = disabled
            mu_max: "0.8".into(),
            max_snapshot_age_hours: 24,
            recovery_target: "0.8".into(),
        }
    }

    /// Validate numeric fields parse as Decimal and are within range.
    pub fn validate(&self) -> Result<(), String> {
        let d = |s: &str, name: &str| -> Result<Decimal, String> {
            Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
        };
        let _ = d(&self.r0, "r0")?;
        let _ = d(&self.r_min, "r_min")?;
        let _ = d(&self.delta_r, "delta_r")?;
        let _ = d(&self.dd_1, "dd_1")?;
        let _ = d(&self.dd_2, "dd_2")?;
        let _ = d(&self.dd_3, "dd_3")?;
        let _ = d(&self.m_dd_1, "m_dd_1")?;
        let _ = d(&self.m_dd_2, "m_dd_2")?;
        let _ = d(&self.m_dd_3, "m_dd_3")?;
        let _ = d(&self.m_dd_4, "m_dd_4")?;
        let _ = d(&self.q_retention, "q_retention")?;
        let _ = d(&self.d_trailing, "d_trailing")?;
        let _ = d(&self.d_prop, "d_prop")?;
        let _ = d(&self.alpha, "alpha")?;
        let _ = d(&self.daily_loss_limit, "daily_loss_limit")?;
        let _ = d(&self.r_portfolio_max, "r_portfolio_max")?;
        let _ = d(&self.r_prop_max, "r_prop_max")?;
        let _ = d(&self.mu_max, "mu_max")?;
        let _ = d(&self.recovery_target, "recovery_target")?;
        for (i, tok) in self.m_loss_table.split(',').enumerate() {
            Decimal::from_str(tok.trim()).map_err(|e| format!("m_loss_table[{i}]: {e}"))?;
        }
        if self.n_max < 1 {
            return Err("n_max must be >= 1".into());
        }
        Ok(())
    }

    /// Parse the loss-streak table into a Vec<Decimal>.
    pub fn loss_table(&self) -> Result<Vec<Decimal>, String> {
        self.m_loss_table
            .split(',')
            .map(|t| Decimal::from_str(t.trim()).map_err(|e| format!("loss_table: {e}")))
            .collect()
    }
}
