//! Decision engine — evaluates a proposed trade in Fail-fast order.
//! Never panics. Every path either returns a decision or a structured error.

use chrono::Utc;

use super::config::EquityConfig;
use super::error::{DecisionReason, EquityError};
use super::risk_engine::{compute_risk, RiskResult};
use super::sizing::{compute_size, SizingInput};
use super::state::{AccountPhase, AccountState};

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
    pub r_allowed: String,  // Decimal string (empty if rejected before sizing)
    pub size: Option<String>, // Decimal string, None if rejected
    pub rr: Option<String>,
    pub risk_detail: Option<RiskResult>,
}

/// Evaluate a trade proposal. Fail-fast, side-effect free.
pub fn evaluate(
    state: &AccountState,
    cfg: &EquityConfig,
    proposal: Option<&TradeProposal>,
) -> Result<TradeDecision, EquityError> {
    // 0. Validate config
    cfg.validate().map_err(EquityError::InvalidInput)?;

    // 1. Config snapshot age
    let now = Utc::now();
    let age_secs = (now - state.snapshot.taken_at).num_seconds();
    let age_hours = age_secs / 3600;
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
        });
    }

    // 2. Account disabled?
    if state.phase == AccountPhase::Disabled {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::AccountDisabled,
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
        });
    }

    // 3. Account paused?
    if state.phase == AccountPhase::Paused {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::AccountPaused,
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
        });
    }

    // 4. Equity below floor (belt & suspenders — phase check above should catch)
    if state.snapshot.equity <= state.f_hybrid {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::EquityBelowFloor {
                equity: state.snapshot.equity.to_string(),
                floor: state.f_hybrid.to_string(),
            },
            r_allowed: "".into(),
            size: None,
            rr: None,
            risk_detail: None,
        });
    }

    // 5. Compute risk
    let risk = compute_risk(state, cfg).map_err(EquityError::InvalidInput)?;

    // 6. Enforce floors / caps zero
    if risk.r_adaptive.is_zero() {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::AdaptiveRiskZero,
            r_allowed: "0".into(),
            size: None,
            rr: None,
            risk_detail: Some(risk),
        });
    }
    if risk.r_allowed <= rust_decimal::Decimal::ZERO {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::RAllowedZeroOrNegative,
            r_allowed: risk.r_allowed.to_string(),
            size: None,
            rr: None,
            risk_detail: Some(risk),
        });
    }

    // 7. If no proposal → this is just a "what is R_allowed?" query
    let Some(p) = proposal else {
        return Ok(TradeDecision {
            allowed: true,
            reason: DecisionReason::Allowed,
            r_allowed: risk.r_allowed.to_string(),
            size: None,
            rr: None,
            risk_detail: Some(risk),
        });
    };

    // 8. Sizing
    let sinput = SizingInput {
        entry: p.entry.clone(),
        stop: p.stop.clone(),
        contract_multiplier: p.contract_multiplier.clone(),
        commission_per_unit: p.commission_per_unit.clone(),
        slippage_buffer: p.slippage_buffer.clone(),
    };
    let sized = compute_size(risk.r_allowed, &sinput, p.take_profit.as_deref())
        .map_err(EquityError::InvalidInput)?;

    Ok(TradeDecision {
        allowed: true,
        reason: DecisionReason::Allowed,
        r_allowed: risk.r_allowed.to_string(),
        size: Some(sized.size.to_string()),
        rr: sized.rr.map(|r| r.to_string()),
        risk_detail: Some(risk),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equity::state::ParsedSnapshot;
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
        let cfg = EquityConfig::default_v1();
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
        assert_eq!(d.rr.unwrap(), "3");
    }

    #[test]
    fn rejected_when_stale() {
        let cfg = EquityConfig::default_v1();
        let mut s = snap_fresh();
        s.taken_at = Utc::now() - Duration::hours(48);
        let st = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(!d.allowed);
        match d.reason {
            DecisionReason::SnapshotTooOld { .. } => {}
            other => panic!("wrong reason: {:?}", other),
        }
    }

    #[test]
    fn rejected_when_paused() {
        let mut cfg = EquityConfig::default_v1();
        cfg.n_max = 5;
        let mut s = snap_fresh();
        s.consecutive_losses = 5;
        let st = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(!d.allowed);
        matches!(d.reason, DecisionReason::AccountPaused);
    }

    #[test]
    fn allowed_query_when_no_proposal() {
        let cfg = EquityConfig::default_v1();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(d.allowed);
        // Compare as Decimal (normalizes trailing zeros)
        assert_eq!(
            Decimal::from_str(&d.r_allowed).unwrap(),
            Decimal::from_str("100").unwrap(),
        ); // 10000 × 0.01
        assert!(d.size.is_none());
    }
}
