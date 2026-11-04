//! Decision engine — evaluates a proposed trade in fail-fast order.
//! Never panics. Every path returns a decision or a structured error.

use chrono::Utc;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

use super::buffer::{self, BufferInput};
use super::config::EquityConfig;
use super::error::{DecisionReason, EquityError};
use super::risk_engine::{self, RiskInput, RiskResult};
use super::sizing::{compute_size, SizingInput};
use super::state::{AccountState, State};
use super::state_machine;

#[derive(Debug, Clone)]
pub struct TradeProposal {
    pub entry: String,
    pub stop: String,
    pub take_profit: Option<String>,
    pub contract_multiplier: String,
    pub commission_per_unit: String,
    pub slippage_buffer: String,
}

#[derive(Debug, Clone)]
pub struct TradeDecision {
    pub allowed: bool,
    pub reason: DecisionReason,
    pub r_allowed: String,
    pub size: Option<String>,
    pub rr: Option<String>,
    pub risk_detail: Option<RiskResult>,
    pub state: String,
    pub buffer_r: String,
    pub m_preserve: String,
    pub binding_constraint: String,
}

fn parse_dec(s: &str, name: &str) -> Result<Decimal, EquityError> {
    Decimal::from_str(s).map_err(|e| EquityError::InvalidInput(format!("{name}: {e}")))
}

pub fn evaluate(
    state: &AccountState,
    cfg: &EquityConfig,
    proposal: Option<&TradeProposal>,
) -> Result<TradeDecision, EquityError> {
    cfg.validate().map_err(EquityError::InvalidInput)?;

    // 1. Snapshot age
    let now = Utc::now();
    let age_hours = (now - state.snapshot.taken_at).num_seconds() / 3600;
    if age_hours > cfg.max_snapshot_age_hours {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::SnapshotTooOld {
                age_hours,
                max_hours: cfg.max_snapshot_age_hours,
            },
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
            state: state.state_enum.as_str().to_string(),
            buffer_r: state.buffer_r.to_string(),
            m_preserve: state.m_preserve.to_string(),
            binding_constraint: "".into(),
        });
    }

    // 2. State machine — blocking states
    let state_s = state.state_enum.as_str().to_string();
    let buffer_r_s = state.buffer_r.to_string();
    let m_preserve_s = state.m_preserve.to_string();

    if state.state_enum == State::PropHardLock {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::AccountDisabled,
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: "".into(),
        });
    }
    if state.state_enum == State::PreservationPause
        || state.state_enum == State::DailyLock
    {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::AccountPaused,
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: "".into(),
        });
    }

    // 3. Compute buffer
    let buf = buffer::compute(&BufferInput {
        equity: state.snapshot.equity,
        f_internal: state.f_internal,
        r_open: state.snapshot.open_portfolio_risk,
        execution_buffer: Decimal::ZERO,
        r_base: state.snapshot.equity * parse_dec(&cfg.r0, "r0")?,
    });

    if buf.pause {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::RAllowedZeroOrNegative,
            r_allowed: "0".into(),
            size: None,
            rr: None,
            risk_detail: None,
            state: "preservation_pause".into(),
            buffer_r: buf.buffer_r.to_string(),
            m_preserve: buf.m_preserve.to_string(),
            binding_constraint: "Buffer".into(),
        });
    }

    // 4. Compute risk
    let r0 = parse_dec(&cfg.r0, "r0")?;
    let portfolio_max = parse_dec(&cfg.r_portfolio_max, "r_portfolio_max")?;
    let daily_limit = parse_dec(&cfg.daily_loss_limit, "daily_loss_limit")?;
    let prop_max = parse_dec(&cfg.r_prop_max, "r_prop_max")?;

    let r_portfolio_remaining = if portfolio_max.is_zero() {
        Decimal::MAX
    } else {
        (state.snapshot.equity * portfolio_max - state.snapshot.open_portfolio_risk).max(Decimal::ZERO)
    };
    let r_daily_remaining = if daily_limit.is_zero() {
        Decimal::MAX
    } else {
        (state.snapshot.equity * daily_limit - state.snapshot.daily_loss).max(Decimal::ZERO)
    };
    let r_prop_remaining = if prop_max.is_zero() {
        Decimal::MAX
    } else {
        state.snapshot.equity * prop_max
    };

    let risk = risk_engine::compute(&RiskInput {
        equity: state.snapshot.equity,
        r_base_pct: r0,
        m_preserve: buf.m_preserve,
        r_buffer: buf.r_buffer,
        r_portfolio_remaining,
        r_daily_remaining,
        r_prop_remaining,
    });

    if risk.r_allowed <= Decimal::ZERO {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::RAllowedZeroOrNegative,
            r_allowed: "0".into(),
            size: None,
            rr: None,
            risk_detail: Some(risk),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: buf.m_preserve.to_string(),
            binding_constraint: "Risk".into(),
        });
    }

    // 5. No proposal → just R_allowed query
    let Some(p) = proposal else {
        let binding = risk.binding.to_string();
        return Ok(TradeDecision {
            allowed: true,
            reason: DecisionReason::Allowed,
            r_allowed: risk.r_allowed.to_string(),
            size: None,
            rr: None,
            risk_detail: Some(risk),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: binding,
        });
    };

    // 6. Sizing
    let sinput = SizingInput {
        entry: p.entry.clone(),
        stop: p.stop.clone(),
        contract_multiplier: p.contract_multiplier.clone(),
        commission_per_unit: p.commission_per_unit.clone(),
        slippage_buffer: p.slippage_buffer.clone(),
    };
    let sized = compute_size(risk.r_allowed, &sinput, p.take_profit.as_deref())
        .map_err(EquityError::InvalidInput)?;

    let binding = risk.binding.to_string();
    Ok(TradeDecision {
        allowed: true,
        reason: DecisionReason::Allowed,
        r_allowed: risk.r_allowed.to_string(),
        size: Some(sized.size.to_string()),
        rr: sized.rr.map(|r| r.to_string()),
        risk_detail: Some(risk),
        state: state_s,
        buffer_r: buffer_r_s,
        m_preserve: m_preserve_s,
        binding_constraint: binding,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equity::state::{AccountState, ParsedSnapshot};
    use chrono::{Duration, Utc};
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromStr;

    fn snap_fresh() -> ParsedSnapshot {
        ParsedSnapshot {
            id: 1,
            equity: Decimal::from_str("10000").unwrap(),
            initial_balance: Decimal::from_str("10000").unwrap(),
            high_watermark: Decimal::from_str("10000").unwrap(),
            low_watermark: Decimal::from_str("10000").unwrap(),
            day_start_equity: Decimal::from_str("10000").unwrap(),
            open_portfolio_risk: Decimal::ZERO,
            daily_loss: Decimal::ZERO,
            consecutive_losses: 0,
            taken_at: Utc::now(),
        }
    }

    #[test]
    fn allowed_fresh_account() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: Some("1.1150".into()),
            contract_multiplier: "100000".into(),
            commission_per_unit: "0".into(),
            slippage_buffer: "0".into(),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        assert!(d.size.is_some());
    }

    #[test]
    fn rejected_when_stale() {
        let cfg = EquityConfig::default_v2();
        let mut s = snap_fresh();
        s.taken_at = Utc::now() - Duration::hours(48);
        let st = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(!d.allowed);
    }

    #[test]
    fn rejected_when_equity_below_floor() {
        let cfg = EquityConfig::default_v2();
        let mut s = snap_fresh();
        s.equity = Decimal::from_str("100").unwrap();
        s.high_watermark = Decimal::from_str("10000").unwrap();
        let st = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(!d.allowed);
    }

    #[test]
    fn allowed_query_when_no_proposal() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(d.allowed);
        assert!(d.size.is_none());
    }
}
