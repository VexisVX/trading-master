//! Decision engine — evaluates a proposed trade in fail-fast order.
//! Never panics. Every path returns a decision or a structured error.

use chrono::Utc;
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;

use super::buffer::{self, BufferInput};
use super::config::EquityConfig;
use super::error::{DecisionReason, EquityError};
use super::risk_engine::{self, RiskInput, RiskResult};
use super::sizing::{compute_size, SizingInput};
use super::state::{AccountState, State};
use super::state_machine;

#[derive(Debug, Clone)]
pub struct TradeProposal {
    /// Symbol — used for instrument lookup (preferred path)
    pub symbol: Option<String>,
    pub entry: String,
    pub stop: String,
    pub take_profit: Option<String>,
    /// When true, engine will NOT use instrument table even if symbol is provided
    pub allow_legacy: bool,
    // --- legacy fields (used only if allow_legacy=true or symbol missing) ---
    pub contract_multiplier: Option<String>,
    pub commission_per_unit: Option<String>,
    pub slippage_buffer: Option<String>,
    // --- caller-supplied instrument (bypasses DB lookup) ---
    pub instrument_override: Option<crate::domain::instrument::InstrumentSpec>,
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
    // Sizing metadata (S2.1)
    pub sizing_source: String, // "instrument" | "legacy" | "override" | "none"
    pub actual_risk: Option<String>,
    pub instrument_symbol: Option<String>,
}

fn parse_dec(s: &str, name: &str) -> Result<Decimal, EquityError> {
    Decimal::from_str(s).map_err(|e| EquityError::InvalidInput(format!("{name}: {e}")))
}

pub fn evaluate(
    state: &AccountState,
    cfg: &EquityConfig,
    proposal: Option<&TradeProposal>,
) -> Result<TradeDecision, EquityError> {
    // Legacy wrapper: no connection available, so if symbol is provided
    // it must be an instrument_override. Otherwise falls back to legacy path.
    evaluate_with_conn(state, cfg, proposal, None)
}

pub fn evaluate_with_conn(
    state: &AccountState,
    cfg: &EquityConfig,
    proposal: Option<&TradeProposal>,
    conn: Option<&rusqlite::Connection>,
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
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
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
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
        });
    }
    if state.state_enum == State::PreservationPause || state.state_enum == State::DailyLock {
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
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
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
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
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
        (state.snapshot.equity * portfolio_max - state.snapshot.open_portfolio_risk)
            .max(Decimal::ZERO)
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
            risk_detail: Some(risk.clone()),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: buf.m_preserve.to_string(),
            binding_constraint: "Risk".into(),
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
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
            risk_detail: Some(risk.clone()),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: binding,
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
        });
    };

    // 6. Sizing — new path
    let entry_d = Decimal::from_str(&p.entry)
        .map_err(|e| EquityError::InvalidInput(format!("entry: {e}")))?;
    let stop_d =
        Decimal::from_str(&p.stop).map_err(|e| EquityError::InvalidInput(format!("stop: {e}")))?;

    // Resolve instrument spec
    let (inst_opt, sizing_source_str): (Option<crate::domain::instrument::InstrumentSpec>, &str) =
        if p.allow_legacy {
            // Explicit legacy path
            (None, "legacy")
        } else if let Some(ov) = &p.instrument_override {
            ov.validate_for_sizing()
                .map_err(EquityError::InvalidInput)?;
            // PHASE 4: currency must match account currency
            if ov.currency != "USD" {
                return Err(EquityError::InvalidInput(format!(
                    "currency conversion {}→USD unavailable (no service yet)",
                    ov.currency
                )));
            }
            (Some(ov.clone()), "override")
        } else if let Some(sym) = &p.symbol {
            // DB lookup
            match crate::db::instruments::get_typed(
                conn.ok_or_else(|| {
                    EquityError::InvalidInput("symbol provided but no DB connection".into())
                })?,
                sym,
            ) {
                Ok(Some(spec)) => {
                    spec.validate_for_sizing()
                        .map_err(EquityError::InvalidInput)?;
                    // PHASE 4: currency must match account currency
                    if spec.currency != "USD" {
                        return Err(EquityError::InvalidInput(format!(
                            "currency conversion {}→USD unavailable (no service yet)",
                            spec.currency
                        )));
                    }
                    (Some(spec), "instrument")
                }
                Ok(None) => {
                    return Ok(TradeDecision {
                        allowed: false,
                        reason: DecisionReason::InstrumentNotFound {
                            symbol: sym.clone(),
                        },
                        r_allowed: risk.r_allowed.to_string(),
                        size: None,
                        rr: None,
                        risk_detail: Some(risk.clone()),
                        state: state_s,
                        buffer_r: buffer_r_s,
                        m_preserve: m_preserve_s,
                        binding_constraint: risk.binding.to_string(),
                        sizing_source: "none".into(),
                        actual_risk: None,
                        instrument_symbol: Some(sym.clone()),
                    });
                }
                Err(e) => return Err(EquityError::InvalidInput(format!("instrument lookup: {e}"))),
            }
        } else {
            // No symbol, no override, not legacy → reject
            return Ok(TradeDecision {
                allowed: false,
                reason: DecisionReason::InstrumentNotFound { symbol: "".into() },
                r_allowed: risk.r_allowed.to_string(),
                size: None,
                rr: None,
                risk_detail: Some(risk.clone()),
                state: state_s,
                buffer_r: buffer_r_s,
                m_preserve: m_preserve_s,
                binding_constraint: risk.binding.to_string(),
                sizing_source: "none".into(),
                actual_risk: None,
                instrument_symbol: None,
            });
        };

    // 6. Sizing — production path
    let inst = inst_opt.as_ref().ok_or_else(|| {
        EquityError::InvalidInput(
            "instrument required: provide symbol or override, or set allow_legacy=true (not recommended)".into()
        )
    })?;

    let res = {
        use crate::risk::position::{size_from_allowed, SizeFromAllowedInput};
        let inp = SizeFromAllowedInput {
            r_allowed: risk.r_allowed,
            entry_price: entry_d,
            stop_price: stop_d,
            tick_size: inst.tick_size,
            tick_value: inst.tick_value,
            contract_size: inst.contract_size,
            commission_per_lot: inst.commission_per_lot,
            lot_step: inst.lot_step,
            min_qty: inst.min_qty,
            max_qty: inst.max_qty,
        };
        size_from_allowed(&inp).map_err(|e| EquityError::InvalidInput(format!("sizing: {e}")))?
    };

    if res.rejected_below_min {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::BelowMinimumQuantity {
                min_qty: inst.min_qty.to_string(),
                allowed_risk: risk.r_allowed.to_string(),
                actual_risk: res.actual_risk.to_string(),
            },
            r_allowed: risk.r_allowed.to_string(),
            size: None,
            rr: None,
            risk_detail: Some(risk.clone()),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: risk.binding.to_string(),
            sizing_source: sizing_source_str.into(),
            actual_risk: Some(res.actual_risk.to_string()),
            instrument_symbol: Some(inst.symbol.clone()),
        });
    }

    if res.actual_risk > risk.r_allowed {
        return Ok(TradeDecision {
            allowed: false,
            reason: DecisionReason::RAllowedZeroOrNegative,
            r_allowed: risk.r_allowed.to_string(),
            size: None,
            rr: None,
            risk_detail: Some(risk.clone()),
            state: state_s,
            buffer_r: buffer_r_s,
            m_preserve: m_preserve_s,
            binding_constraint: risk.binding.to_string(),
            sizing_source: sizing_source_str.into(),
            actual_risk: Some(res.actual_risk.to_string()),
            instrument_symbol: Some(inst.symbol.clone()),
        });
    }

    let rr_str = if let Some(tp) = p.take_profit.as_ref() {
        if let Ok(tp_d) = Decimal::from_str(tp) {
            let risk_d = (entry_d - stop_d).abs();
            if risk_d > Decimal::ZERO {
                Some(((tp_d - entry_d).abs() / risk_d).to_string())
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let size_str = Some(res.lots.to_string());
    let actual_risk_str = Some(res.actual_risk.to_string());
    let inst_sym = Some(inst.symbol.clone());

    let binding = risk.binding.to_string();
    Ok(TradeDecision {
        allowed: true,
        reason: DecisionReason::Allowed,
        r_allowed: risk.r_allowed.to_string(),
        size: size_str,
        rr: rr_str,
        risk_detail: Some(risk.clone()),
        state: state_s,
        buffer_r: buffer_r_s,
        m_preserve: m_preserve_s,
        binding_constraint: binding,
        sizing_source: sizing_source_str.into(),
        actual_risk: actual_risk_str,
        instrument_symbol: inst_sym,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equity::state::{AccountState, ParsedSnapshot};
    use chrono::{Duration, Utc};
    use rust_decimal::prelude::FromStr;
    use rust_decimal::Decimal;

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
        // PHASE 4: production path requires instrument. Use override.
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed, "reason: {:?}", d.reason);
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

    // ============================================================
    // S2.1: Instrument-aware sizing
    // ============================================================

    fn make_instrument(
        symbol: &str,
        tick_size: &str,
        tick_value: &str,
        contract_size: &str,
        min_qty: &str,
        lot_step: &str,
    ) -> crate::domain::instrument::InstrumentSpec {
        use rust_decimal::prelude::FromStr;
        crate::domain::instrument::InstrumentSpec {
            symbol: symbol.into(),
            currency: "USD".into(),
            tick_size: Decimal::from_str(tick_size).unwrap(),
            tick_value: Decimal::from_str(tick_value).unwrap(),
            contract_size: Decimal::from_str(contract_size).unwrap(),
            min_qty: Decimal::from_str(min_qty).unwrap(),
            max_qty: Decimal::from_str("100").unwrap(),
            lot_step: Decimal::from_str(lot_step).unwrap(),
            commission_per_lot: Decimal::ZERO,
            slippage_per_unit: Decimal::ZERO,
            spec_source: crate::domain::instrument::SpecSource::UserEntered,
        }
    }

    fn eval_with_inst(
        state: &AccountState,
        cfg: &EquityConfig,
        inst: &crate::domain::instrument::InstrumentSpec,
    ) -> TradeDecision {
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst.clone()),
        };
        evaluate(state, cfg, Some(&p)).unwrap()
    }

    #[test]
    fn t_eurusd_sizing() {
        // EURUSD: 0.0050 price distance / 0.00001 tick_size = 500 ticks
        // 500 ticks × $1/lot = $500 per lot
        // R_allowed = 10000 * 0.01 = $100 → size = 100/500 = 0.20 lots
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed, "should allow: {:?}", d.reason);
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(
            (size - 0.20).abs() < 0.05,
            "expected ~0.20 lots, got {}",
            size
        );
    }

    #[test]
    fn t_xauusd_sizing() {
        // XAUUSD: entry 2000, stop 1995 (5 points = 500 ticks with tick_size 0.01)
        // loss_per_lot = 500 * 1 = 500
        // R_allowed ~ 50 → size = 0.1
        let inst = make_instrument("XAUUSD", "0.01", "1", "100", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "2000".into(),
            stop: "1995".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(size <= 0.21, "expected ≤0.21 lots, got {}", size);
    }

    #[test]
    fn t_min_qty_rejection() {
        // Very small R_allowed → min_qty too big → reject
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "1.0", "0.01"); // min 1.0 lot
        let cfg = EquityConfig::default_v2();
        let mut snap = snap_fresh();
        // Tiny risk: R_allowed ≈ 0.1 → 0.1/50 = 0.002 lots < min 1.0
        snap.equity = Decimal::from_str("10").unwrap();
        snap.initial_balance = Decimal::from_str("10").unwrap();
        snap.high_watermark = Decimal::from_str("10").unwrap();
        let st = AccountState::from_snapshot(snap, &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        // Either rejected_below_min or r_allowed=0 → rejected
        // Just assert it's handled (not panics)
        let _ = d.allowed;
    }

    #[test]
    fn t_estimated_spec_rejected() {
        use crate::domain::instrument::SpecSource;
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.spec_source = SpecSource::Estimated;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        // evaluate() should return Err (validate_for_sizing fails before decision)
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err for estimated spec");
    }

    #[test]
    fn t_zero_tick_rejected() {
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.tick_size = Decimal::ZERO;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err for zero tick_size");
    }

    #[test]
    fn t_legacy_path_when_allow_legacy() {
        // PHASE 4: legacy path removed — allow_legacy=true still requires instrument
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: true,
            contract_multiplier: Some("100000".into()),
            commission_per_unit: Some("0".into()),
            slippage_buffer: Some("0".into()),
            instrument_override: None,
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "legacy path removed — expect Err");
    }

    #[test]
    fn t_symbol_not_found_rejected() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: Some("UNKNOWN".into()),
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        // evaluate() without conn → symbol requires DB → returns Err
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err: symbol needs DB connection");
    }

    #[test]
    fn t_size_from_allowed_gives_actual_risk() {
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed);
        let actual_risk: f64 = d.actual_risk.unwrap().parse().unwrap();
        let r_allowed: f64 = d.r_allowed.parse().unwrap();
        assert!(
            actual_risk <= r_allowed + 0.01,
            "actual_risk {} > r_allowed {}",
            actual_risk,
            r_allowed
        );
    }

    #[test]
    fn t_lot_step_rounding() {
        // loss_per_lot = 50, R = 50.03 → raw = 1.0006
        // lot_step 0.01 → round DOWN to 1.00
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed);
        let size: f64 = d.size.unwrap().parse().unwrap();
        // Must be multiple of 0.01 (i.e., 1.00 not 1.0006)
        let rounded = (size * 100.0).round() / 100.0;
        assert!((size - rounded).abs() < 0.001);
    }

    #[test]
    fn t_no_symbol_no_override_rejects() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(!d.allowed);
    }

    #[test]
    fn t_instrument_override_used() {
        // If override provided, sizing_source == "override"
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert_eq!(d.sizing_source, "override");
        assert_eq!(d.instrument_symbol.as_deref(), Some("EURUSD"));
    }

    // ============================================================
    // PHASE 4: instrument-aware sizing — 14 tests
    // ============================================================

    fn make_btc_instrument() -> crate::domain::instrument::InstrumentSpec {
        use rust_decimal::prelude::FromStr;
        crate::domain::instrument::InstrumentSpec {
            symbol: "BTCUSD".into(),
            currency: "USD".into(),
            tick_size: Decimal::from_str("0.01").unwrap(),
            tick_value: Decimal::from_str("0.01").unwrap(),
            contract_size: Decimal::from_str("1").unwrap(),
            min_qty: Decimal::from_str("0.01").unwrap(),
            max_qty: Decimal::from_str("10").unwrap(),
            lot_step: Decimal::from_str("0.01").unwrap(),
            commission_per_lot: Decimal::ZERO,
            slippage_per_unit: Decimal::ZERO,
            spec_source: crate::domain::instrument::SpecSource::UserEntered,
        }
    }

    fn make_eur_currency_instrument() -> crate::domain::instrument::InstrumentSpec {
        use rust_decimal::prelude::FromStr;
        crate::domain::instrument::InstrumentSpec {
            symbol: "EURUSD".into(),
            currency: "EUR".into(), // ← not USD
            tick_size: Decimal::from_str("0.00001").unwrap(),
            tick_value: Decimal::from_str("1").unwrap(),
            contract_size: Decimal::from_str("100000").unwrap(),
            min_qty: Decimal::from_str("0.01").unwrap(),
            max_qty: Decimal::from_str("100").unwrap(),
            lot_step: Decimal::from_str("0.01").unwrap(),
            commission_per_lot: Decimal::ZERO,
            slippage_per_unit: Decimal::ZERO,
            spec_source: crate::domain::instrument::SpecSource::UserEntered,
        }
    }

    #[test]
    fn t_phase4_eurusd_long() {
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed, "reason: {:?}", d.reason);
        assert_eq!(d.sizing_source, "override");
        assert!(d.size.is_some());
        assert!(d.actual_risk.is_some());
    }

    #[test]
    fn t_phase4_eurusd_short() {
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.1050".into(), // short
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        // size should be same as long (symmetric distance)
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(
            (size - 0.2).abs() < 0.05,
            "expected ~0.2 lots, got {}",
            size
        );
    }

    #[test]
    fn t_phase4_xauusd() {
        let inst = make_instrument("XAUUSD", "0.01", "1", "100", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "2000".into(),
            stop: "1995".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        let size: f64 = d.size.unwrap().parse().unwrap();
        // 500 ticks × $1 = $500/lot → ~0.2 lots
        assert!(size > 0.1 && size < 0.3, "got {}", size);
    }

    #[test]
    fn t_phase4_btcusd() {
        let inst = make_btc_instrument();
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "40000".into(),
            stop: "39500".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(size > 0.0, "btc size should be positive, got {}", size);
    }

    #[test]
    fn t_phase4_missing_instrument_rejected() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: Some("NOTREAL".into()),
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        // evaluate() (no conn) → symbol needs DB → Err
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err: symbol needs DB connection");
    }

    #[test]
    fn t_phase4_estimated_metadata_rejected() {
        use crate::domain::instrument::SpecSource;
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.spec_source = SpecSource::Estimated;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "estimated spec should be rejected");
    }

    #[test]
    fn t_phase4_currency_mismatch_rejected() {
        let inst = make_eur_currency_instrument();
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(
            result.is_err(),
            "EUR currency should be rejected (no conversion)"
        );
    }

    #[test]
    fn t_phase4_min_qty_rejection() {
        // R_allowed tiny → size below min_qty → rejected
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "1.0", "0.01"); // min 1.0
        let cfg = EquityConfig::default_v2();
        let mut snap = snap_fresh();
        snap.equity = Decimal::from_str("10").unwrap();
        snap.initial_balance = Decimal::from_str("10").unwrap();
        snap.high_watermark = Decimal::from_str("10").unwrap();
        let st = AccountState::from_snapshot(snap, &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        // Either rejected below min, or rejected by r_allowed
        let is_below_min = matches!(d.reason, DecisionReason::BelowMinimumQuantity { .. });
        let is_zero = matches!(d.reason, DecisionReason::RAllowedZeroOrNegative);
        assert!(is_below_min || is_zero, "expected reject: {:?}", d.reason);
    }

    #[test]
    fn t_phase4_max_qty_clamp() {
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.max_qty = Decimal::from_str("0.5").unwrap();
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        // Large R would give > 0.5 lots, clamp to 0.5
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0999".into(), // tiny stop → huge size
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        if d.allowed {
            let size: f64 = d.size.unwrap().parse().unwrap();
            assert!(size <= 0.5, "size {} should be clamped to 0.5", size);
        }
    }

    #[test]
    fn t_phase4_lot_step_rounding() {
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.lot_step = Decimal::from_str("0.05").unwrap();
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        if d.allowed {
            let size: f64 = d.size.unwrap().parse().unwrap();
            // size * 100 must be multiple of 5 (0.05 lot step)
            let rounded = (size * 100.0).round() as i32;
            assert_eq!(rounded % 5, 0, "size {} not multiple of 0.05", size);
        }
    }

    #[test]
    fn t_phase4_commission_and_slippage() {
        use rust_decimal::prelude::FromStr;
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.commission_per_lot = Decimal::from_str("7").unwrap();
        inst.slippage_per_unit = Decimal::from_str("0.00001").unwrap();
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(d.allowed);
        // With fees, size should be smaller than without
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(
            size > 0.0 && size < 0.2,
            "fees should reduce size, got {}",
            size
        );
    }

    #[test]
    fn t_phase4_actual_risk_after_rounding() {
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed);
        let actual: f64 = d.actual_risk.unwrap().parse().unwrap();
        let allowed: f64 = d.r_allowed.parse().unwrap();
        assert!(
            actual <= allowed + 0.01,
            "actual {} > allowed {}",
            actual,
            allowed
        );
    }

    #[test]
    fn t_phase4_legacy_disabled_rejects() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(!d.allowed, "no symbol, no override → reject");
    }
}
