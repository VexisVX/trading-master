//! Account state — snapshot + derived metrics. Pure struct; no I/O.
//! All money fields are Decimal strings (Decimal-safe over the wire).

pub use super::state_machine::State;
use chrono::{DateTime, Utc};
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
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
    fn default() -> Self {
        AccountPhase::Normal
    }
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
    pub net_profit: Decimal,       // P_t
    pub dd_pct: Decimal,           // DD_t (0.0–1.0)
    pub profit_retention: Decimal, // PR_t
    pub f_profit: Decimal,         // F_profit
    pub f_trail: Decimal,          // F_trail
    pub f_prop: Decimal,           // F_prop (independent, S6)
    pub f_internal: Decimal,       // F_internal = max(f_profit, f_trail) (S6)
    // ---- New fields (S6) ----
    pub state_enum: crate::equity::state_machine::State,
    pub buffer_r: Decimal,
    pub m_preserve: Decimal,
    // ---- Legacy aliases (S6 transition; removed in S8) ----
    pub f_hybrid: Decimal,      // legacy = f_internal.max(f_prop)
    pub loss_capacity: Decimal, // legacy = equity - f_internal
    pub phase: AccountPhase,    // legacy = map from state_enum
}

impl AccountState {
    /// Build state from a parsed snapshot + config. No I/O.
    pub fn from_snapshot(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
    ) -> Result<Self, String> {
        Self::from_snapshot_with_profile(snapshot, config, None)
    }

    /// Profile-aware variant: if `profile` is Some, use its derived values
    /// for profit protection / trailing DD instead of the global config.
    pub fn from_snapshot_with_profile(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
        profile: Option<&crate::db::profiles::AccountProfileRow>,
    ) -> Result<Self, String> {
        let d = |s: &str, name: &str| -> Result<Decimal, String> {
            Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
        };
        let d_prop = d(&config.d_prop, "d_prop").unwrap_or(Decimal::ZERO);

        // Net profit
        let net_profit = snapshot.equity - snapshot.initial_balance;

        // Drawdown percentage
        let dd_pct = if snapshot.high_watermark > Decimal::ZERO {
            (snapshot.high_watermark - snapshot.equity) / snapshot.high_watermark
        } else {
            Decimal::ZERO
        };

        // Profit retention: only positive HWM minus initial counts
        // S11: use protection module (smoothstep) for F_internal.
        // Compute nav and nav_hwm relative to initial_balance.
        let nav = if snapshot.initial_balance > Decimal::ZERO {
            snapshot.equity / snapshot.initial_balance
        } else {
            Decimal::ONE
        };
        let nav_hwm = if snapshot.initial_balance > Decimal::ZERO {
            snapshot.high_watermark / snapshot.initial_balance
        } else {
            Decimal::ONE
        };

        let mut prot_cfg = crate::equity::protection::ProtectionConfig::default_v1();
        if let Some(p) = profile {
            if let Ok(v) = Decimal::from_str(&p.profit_protect_activation) {
                prot_cfg.profit_protect_activation = v;
            }
            if let Ok(v) = Decimal::from_str(&p.profit_protect_full) {
                prot_cfg.profit_protect_full = v;
            }
            if let Ok(v) = Decimal::from_str(&p.max_profit_retention) {
                prot_cfg.max_profit_retention = v;
            }
            if let Ok(v) = Decimal::from_str(&p.trailing_dd_pct) {
                prot_cfg.trailing_dd_pct = v;
            }
        }
        let prot = crate::equity::protection::compute(snapshot.initial_balance, nav_hwm, &prot_cfg);

        let f_profit = prot.f_profit.unwrap_or(Decimal::ZERO);
        let f_trail = prot.f_trail;
        let f_internal = prot.f_internal;
        let profit_retention = prot.q_retention
            * (snapshot.high_watermark - snapshot.initial_balance).max(Decimal::ZERO);

        // F_prop: still uses legacy D_prop config (S12 will replace).
        let f_prop = if d_prop > Decimal::ZERO {
            snapshot.initial_balance - d_prop
        } else {
            Decimal::ZERO
        };

        // R_base for buffer_r computation.
        let r0 = d(&config.r0, "r0").unwrap_or(Decimal::new(1, 2));
        let r_base = snapshot.equity * r0;

        // Buffer: equity - f_internal - open_risk (execution buffer default 0).
        let raw_b = snapshot.equity - f_internal - snapshot.open_portfolio_risk;
        let b = if raw_b > Decimal::ZERO {
            raw_b
        } else {
            Decimal::ZERO
        };
        let buffer_r = if r_base.is_zero() {
            Decimal::ZERO
        } else {
            b / r_base
        };

        // M_preserve piecewise (spec §6).
        let m_preserve = if buffer_r >= Decimal::from(8) {
            Decimal::ONE
        } else if buffer_r > Decimal::from(2) {
            let frac = (buffer_r - Decimal::from(2)) / Decimal::from(6);
            Decimal::new(25, 2) + Decimal::new(75, 2) * frac
        } else {
            Decimal::ZERO
        };

        // State machine.
        use crate::equity::state_machine::{next, State, TransitionInput};
        let previous = State::NormalGrowth;
        let prop_breached = snapshot.equity <= f_prop && f_prop > Decimal::ZERO;
        let daily_locked = false;
        let ti = TransitionInput {
            previous,
            prop_breached,
            daily_locked,
            buffer_r,
            recovery_authorized: false,
        };
        let state_enum = next(&ti);

        // Legacy aliases.
        let f_hybrid = f_internal.max(f_prop);
        let loss_capacity = (snapshot.equity - f_internal).max(Decimal::ZERO);
        let phase = match state_enum {
            State::PropHardLock => AccountPhase::Disabled,
            State::PreservationPause => AccountPhase::Paused,
            State::RecoveryMode => AccountPhase::Recovery,
            State::Protected => AccountPhase::Caution,
            State::DailyLock => AccountPhase::Paused,
            State::NormalGrowth => AccountPhase::Normal,
        };

        Ok(Self {
            snapshot,
            net_profit,
            dd_pct,
            profit_retention,
            f_profit,
            f_trail,
            f_prop,
            f_internal,
            state_enum,
            buffer_r,
            m_preserve,
            // legacy aliases
            f_hybrid,
            loss_capacity,
            phase,
        })
    }
}

impl AccountState {
    /// Build state with explicit recovery authorization (PHASE 5).
    pub fn from_snapshot_with_recovery(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
        profile: Option<&crate::db::profiles::AccountProfileRow>,
        recovery_authorized: bool,
    ) -> Result<Self, String> {
        use crate::equity::state_machine::{next, State, TransitionInput};

        // Call base constructor
        let mut state = Self::from_snapshot_with_profile(snapshot, config, profile)?;

        // Compute buffer_r for state machine
        let r0 = config.r0.parse::<Decimal>().unwrap_or(Decimal::new(1, 2));
        let r_base = state.snapshot.equity * r0;
        let raw_b = state.snapshot.equity - state.f_internal - state.snapshot.open_portfolio_risk;
        let b = if raw_b > Decimal::ZERO { raw_b } else { Decimal::ZERO };
        let buffer_r = if r_base.is_zero() { Decimal::ZERO } else { b / r_base };

        // Run state machine with recovery_authorized flag
        let previous = state.state_enum;
        let prop_breached = state.snapshot.equity <= state.f_prop && state.f_prop > Decimal::ZERO;
        let ti = TransitionInput {
            previous,
            prop_breached,
            daily_locked: false,
            buffer_r,
            recovery_authorized,
        };
        state.state_enum = next(&ti);

        // PHASE 5-C: In RecoveryMode, override m_preserve with probe_risk_pct (0.25)
        // to allow small probe trades
        if state.state_enum == State::RecoveryMode {
            state.m_preserve = Decimal::new(25, 2);  // 0.25
        }

        // Legacy phase map
        state.phase = match state.state_enum {
            State::PropHardLock => AccountPhase::Disabled,
            State::PreservationPause => AccountPhase::Paused,
            State::RecoveryMode => AccountPhase::Recovery,
            State::Protected => AccountPhase::Caution,
            State::DailyLock => AccountPhase::Paused,
            State::NormalGrowth => AccountPhase::Normal,
        };

        Ok(state)
    }
}

