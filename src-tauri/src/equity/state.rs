//! Account state — snapshot + derived metrics. Pure struct; no I/O.
//! All money fields are Decimal strings (Decimal-safe over the wire).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccountPhase {
    Normal,
    Caution,
    Recovery,
    Paused,
    Disabled,
}

impl Default for AccountPhase {
    fn default() -> Self { AccountPhase::Normal }
}

/// Raw snapshot taken at a point in time. Values are Decimal strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquitySnapshot {
    pub id: i64,
    /// E_t — equity at snapshot time (money).
    pub equity: String,
    /// E_0 — starting balance (money).
    pub initial_balance: String,
    /// H_t — high watermark (money), maintained server-side.
    pub high_watermark: String,
    /// E_low — lowest equity since last HWM (money).
    pub low_watermark: String,
    /// E_day_start — equity at start of current trading day.
    pub day_start_equity: String,
    /// Sum of open-trade risk (money), maintained by engine.
    pub open_portfolio_risk: String,
    /// Sum of today's realized losses (money, positive number = loss).
    pub daily_loss: String,
    /// Consecutive losses (integer).
    pub consecutive_losses: i64,
    /// Snapshot timestamp (UTC).
    pub taken_at: String,
}

impl EquitySnapshot {
    pub fn parse(&self) -> Result<ParsedSnapshot, String> {
        let d = |s: &str, name: &str| -> Result<Decimal, String> {
            Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
        };
        let taken_at = DateTime::parse_from_rfc3339(&self.taken_at)
            .map_err(|e| format!("taken_at: {e}"))?
            .with_timezone(&Utc);
        Ok(ParsedSnapshot {
            id: self.id,
            equity: d(&self.equity, "equity")?,
            initial_balance: d(&self.initial_balance, "initial_balance")?,
            high_watermark: d(&self.high_watermark, "high_watermark")?,
            low_watermark: d(&self.low_watermark, "low_watermark")?,
            day_start_equity: d(&self.day_start_equity, "day_start_equity")?,
            open_portfolio_risk: d(&self.open_portfolio_risk, "open_portfolio_risk")?,
            daily_loss: d(&self.daily_loss, "daily_loss")?,
            consecutive_losses: self.consecutive_losses,
            taken_at,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedSnapshot {

    pub id: i64,
    pub equity: Decimal,
    pub initial_balance: Decimal,
    pub high_watermark: Decimal,
    pub low_watermark: Decimal,
    pub day_start_equity: Decimal,
    pub open_portfolio_risk: Decimal,
    pub daily_loss: Decimal,
    pub consecutive_losses: i64,
    pub taken_at: DateTime<Utc>,
}

/// Fully derived account state used by the engine. Everything numeric is Decimal.
#[derive(Debug, Clone)]
pub struct AccountState {
    pub snapshot: ParsedSnapshot,
    // ---- Derived metrics (all money except dd_pct) ----
    pub net_profit: Decimal,          // P_t
    pub dd_pct: Decimal,              // DD_t (0.0–1.0)
    pub profit_retention: Decimal,    // PR_t
    pub f_profit: Decimal,            // F_profit
    pub f_trail: Decimal,             // F_trail
    pub f_prop: Decimal,              // F_prop
    pub f_hybrid: Decimal,            // F_hybrid
    pub loss_capacity: Decimal,       // LC_t
    pub phase: AccountPhase,
}

impl AccountState {
    /// Build state from a parsed snapshot + config. No I/O.
    pub fn from_snapshot(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
    ) -> Result<Self, String> {
        let d = |s: &str, name: &str| -> Result<Decimal, String> {
            Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
        };
        let q = d(&config.q_retention, "q_retention")?;
        let d_trail = d(&config.d_trailing, "d_trailing")?;
        let d_prop = d(&config.d_prop, "d_prop")?;
        let dd_1 = d(&config.dd_1, "dd_1")?;
        let dd_2 = d(&config.dd_2, "dd_2")?;
        let dd_3 = d(&config.dd_3, "dd_3")?;

        // Net profit
        let net_profit = snapshot.equity - snapshot.initial_balance;

        // Drawdown percentage
        let dd_pct = if snapshot.high_watermark > Decimal::ZERO {
            (snapshot.high_watermark - snapshot.equity) / snapshot.high_watermark
        } else {
            Decimal::ZERO
        };

        // Profit retention: only positive HWM minus initial counts
        let positive_excess = (snapshot.high_watermark - snapshot.initial_balance).max(Decimal::ZERO);
        let profit_retention = positive_excess * q;

        // F_profit only active when we have positive profit (HWM > initial).
        // Otherwise it's Decimal::MIN so it doesn't pull the hybrid floor up.
        let f_profit = if snapshot.high_watermark > snapshot.initial_balance {
            snapshot.initial_balance + profit_retention
        } else {
            Decimal::MIN
        };
        // F_trail always active.
        let f_trail = snapshot.high_watermark * (Decimal::ONE - d_trail);
        // F_prop only active when D_prop is set (> 0).
        let f_prop = if d_prop > Decimal::ZERO {
            snapshot.initial_balance - d_prop
        } else {
            Decimal::MIN
        };
        let f_hybrid = f_profit.max(f_trail).max(f_prop);
        let loss_capacity = snapshot.equity - f_hybrid;

        // Phase
        let phase = if snapshot.equity <= f_hybrid {
            AccountPhase::Disabled
        } else if snapshot.consecutive_losses >= config.n_max {
            AccountPhase::Paused
        } else if dd_pct >= dd_3 {
            AccountPhase::Recovery
        } else if dd_pct >= dd_1 {
            AccountPhase::Caution
        } else {
            AccountPhase::Normal
        };

        let _ = (dd_2, dd_3); // available for future use

        Ok(Self {
            snapshot,
            net_profit,
            dd_pct,
            profit_retention,
            f_profit,
            f_trail,
            f_prop,
            f_hybrid,
            loss_capacity,
            phase,
        })
    }
}
