//! Daily P&L computation (PHASE 2.2).
//!
//! Computes:
//!   - realized_pnl: closed today's net P&L (from manual_pnl)
//!   - floating_pnl: open trades, current_price vs entry
//!   - fees: already in realized/floating (via manual_pnl / cost_per_unit)
//!   - remaining_downside: open positions, current_price → active_stop
//!
//! DailyNetPnL = current_equity − day_start_equity − external_cash_flows
//! DailyLossUsed = max(0, −DailyNetPnL)
//! DailyRemaining = max(0, DailyLossLimit − DailyLossUsed − OpenRemainingDownside − ExecutionBuffer)
//!
//! DATA-QUALITY POLICY:
//!   - Missing current_price   → FloatingPnlUnknown (block)
//!   - Stale current_price     → MarketDataStale (block)
//!   - Missing instrument      → InstrumentMissing (block)
//!   - Estimated instrument    → InstrumentUnsafe (block)
//!   - Currency mismatch       → CurrencyConversionMissing (block)
//!
//! NEVER silently treat unknown as zero.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use super::error::DailyPnLError;
use crate::domain::instrument::InstrumentSpec;
use crate::domain::trade::{Side, Trade, TradeStatus};

#[derive(Debug, Clone)]
pub struct DailyPnLInput<'a> {
    pub trades: &'a [Trade],
    pub day_start_equity: Decimal,
    pub current_equity: Decimal,
    /// Net external cash flows since reset (deposits − withdrawals, in account currency).
    pub external_cash_flows: Decimal,
    /// Reset boundary timestamp (UTC).
    pub day_start_time: DateTime<Utc>,
    /// Instruments indexed by symbol.
    pub instruments: &'a HashMap<String, InstrumentSpec>,
    /// Max age for current_price before considered stale.
    pub max_price_age_minutes: i64,
    /// Reference "now" (for tests).
    pub now: DateTime<Utc>,
    /// Daily loss limit (money). 0 = disabled.
    pub daily_loss_limit: Decimal,
    /// Execution buffer for remaining calculation.
    pub execution_buffer: Decimal,
}

#[derive(Debug, Clone)]
pub struct DailyPnLResult {
    /// Realized net P&L of trades closed today (from manual_pnl).
    pub realized_pnl: Decimal,
    /// Floating P&L of open trades (current_price vs entry, converted to account ccy).
    pub floating_pnl: Decimal,
    /// Fees already accounted inside realized/floating (informational).
    pub fees_included: Decimal,
    /// Additional remaining downside of open positions (current_price → active_stop).
    pub remaining_downside: Decimal,
    /// DailyNetPnL = current − day_start − external_cash_flows.
    pub daily_net_pnl: Decimal,
    /// DailyLossUsed = max(0, −daily_net_pnl).
    pub daily_loss_used: Decimal,
    /// DailyRemaining = max(0, limit − used − downside − buffer). 0 if limit disabled.
    pub daily_remaining: Decimal,
    /// Count of open trades included.
    pub open_trades_count: usize,
    /// Count of closed-today trades included.
    pub closed_today_count: usize,
}

/// True if the trade was closed on or after `day_start` (reset boundary).
fn closed_after_reset(t: &Trade, day_start: DateTime<Utc>) -> bool {
    if t.status != TradeStatus::Closed {
        return false;
    }
    let Some(c) = t.closed_at.as_deref() else { return false; };
    let Ok(when) = DateTime::parse_from_rfc3339(c) else { return false; };
    when.with_timezone(&Utc) >= day_start
}

fn age_minutes(ts: Option<&str>, now: DateTime<Utc>) -> Option<i64> {
    let ts = ts?;
    let when = DateTime::parse_from_rfc3339(ts).ok()?;
    Some(now.signed_duration_since(when.with_timezone(&Utc)).num_minutes())
}

fn lookup_instrument<'a>(
    instruments: &'a HashMap<String, InstrumentSpec>,
    symbol: &str,
) -> Result<&'a InstrumentSpec, DailyPnLError> {
    let spec = instruments.get(symbol).ok_or_else(|| {
        DailyPnLError::InstrumentMissing { symbol: symbol.into() }
    })?;
    if !spec.spec_source.is_safe_for_sizing() {
        return Err(DailyPnLError::InstrumentUnsafe {
            symbol: symbol.into(),
            source: spec.spec_source.as_str().into(),
        });
    }
    // Currency must match account currency (no conversion service yet).
    if spec.currency != "USD" {
        return Err(DailyPnLError::CurrencyConversionMissing {
            from: spec.currency.clone(),
            to: "USD".into(),
        });
    }
    Ok(spec)
}

/// Compute the floating P&L of one open trade in account currency.
fn floating_one(t: &Trade, spec: &InstrumentSpec) -> Result<Decimal, DailyPnLError> {
    let cur = t.current_price.ok_or_else(|| {
        DailyPnLError::FloatingPnlUnknown {
            trade_id: t.id,
            symbol: t.symbol.clone(),
        }
    })?;
    let diff = match t.side {
        Side::Long => cur - t.entry_price,
        Side::Short => t.entry_price - cur,
    };
    // Convert price diff to money using tick_size / tick_value.
    let ticks = if spec.tick_size.is_zero() {
        return Err(DailyPnLError::ComputeError("tick_size is zero".into()));
    } else {
        diff / spec.tick_size
    };
    Ok(ticks * spec.tick_value * t.total_quantity)
}

/// Remaining downside of one open trade: from current_price to active_stop.
fn downside_one(t: &Trade, spec: &InstrumentSpec) -> Result<Decimal, DailyPnLError> {
    let cur = t.current_price.ok_or_else(|| {
        DailyPnLError::FloatingPnlUnknown {
            trade_id: t.id,
            symbol: t.symbol.clone(),
        }
    })?;
    let stop = t.active_stop();
    let dist = match t.side {
        Side::Long => (cur - stop).max(Decimal::ZERO),
        Side::Short => (stop - cur).max(Decimal::ZERO),
    };
    let ticks = if spec.tick_size.is_zero() {
        return Err(DailyPnLError::ComputeError("tick_size is zero".into()));
    } else {
        dist / spec.tick_size
    };
    Ok(ticks * spec.tick_value * t.total_quantity)
}

pub fn compute(input: &DailyPnLInput<'_>) -> Result<DailyPnLResult, DailyPnLError> {
    let mut realized_pnl = Decimal::ZERO;
    let mut floating_pnl = Decimal::ZERO;
    let mut remaining_downside = Decimal::ZERO;
    let mut fees = Decimal::ZERO;
    let mut open_count = 0usize;
    let mut closed_count = 0usize;

    for t in input.trades {
        match t.status {
            TradeStatus::Closed => {
                if closed_after_reset(t, input.day_start_time) {
                    closed_count += 1;
                    if let Some(p) = t.manual_pnl {
                        realized_pnl += p;
                    } else {
                        // Compute from legs.
                        let mut gross = Decimal::ZERO;
                        let mut cost = Decimal::ZERO;
                        for l in t.legs.iter().filter(|l| {
                            matches!(l.kind, crate::domain::trade::LegKind::Exit)
                        }) {
                            let d = match t.side {
                                Side::Long => l.price - t.entry_price,
                                Side::Short => t.entry_price - l.price,
                            };
                            gross += d * l.quantity;
                            cost += l.cost_per_unit * l.quantity;
                        }
                        realized_pnl += gross - cost;
                        fees += cost;
                    }
                }
            }
            TradeStatus::Open => {
                let spec = lookup_instrument(input.instruments, &t.symbol)?;
                // Stale check
                let age = age_minutes(t.current_price_updated_at.as_deref(), input.now);
                match age {
                    Some(a) if a > input.max_price_age_minutes => {
                        return Err(DailyPnLError::MarketDataStale {
                            trade_id: t.id,
                            symbol: t.symbol.clone(),
                            age_minutes: a,
                        });
                    }
                    None => {
                        return Err(DailyPnLError::FloatingPnlUnknown {
                            trade_id: t.id,
                            symbol: t.symbol.clone(),
                        });
                    }
                    _ => {}
                }
                open_count += 1;
                floating_pnl += floating_one(t, spec)?;
                remaining_downside += downside_one(t, spec)?;
            }
        }
    }

    // DailyNetPnL formula
    let daily_net_pnl = input.current_equity
        - input.day_start_equity
        - input.external_cash_flows;

    let daily_loss_used = if daily_net_pnl < Decimal::ZERO {
        -daily_net_pnl
    } else {
        Decimal::ZERO
    };

    let daily_remaining = if input.daily_loss_limit <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        (input.daily_loss_limit
            - daily_loss_used
            - remaining_downside
            - input.execution_buffer)
            .max(Decimal::ZERO)
    };

    Ok(DailyPnLResult {
        realized_pnl,
        floating_pnl,
        fees_included: fees,
        remaining_downside,
        daily_net_pnl,
        daily_loss_used,
        daily_remaining,
        open_trades_count: open_count,
        closed_today_count: closed_count,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::instrument::{InstrumentSpec, SpecSource};
    use crate::domain::trade::{
        empty_trade, LegKind, Side, TradeLeg, TradeStatus,
    };
    use chrono::Duration;
    use rust_decimal_macros::dec;
    use std::collections::HashMap;

    fn eurusd() -> InstrumentSpec {
        InstrumentSpec {
            symbol: "EURUSD".into(),
            currency: "USD".into(),
            tick_size: dec!(0.00001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            min_qty: dec!(0.01),
            max_qty: dec!(100),
            lot_step: dec!(0.01),
            commission_per_lot: dec!(0),
            slippage_per_unit: dec!(0),
            spec_source: SpecSource::UserEntered,
        }
    }

    fn instruments() -> HashMap<String, InstrumentSpec> {
        let mut m = HashMap::new();
        m.insert("EURUSD".into(), eurusd());
        m
    }

    fn open_trade(id: i64, entry: Decimal, price: Decimal, stop: Decimal, side: Side) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.symbol = "EURUSD".into();
        t.side = side;
        t.entry_price = entry;
        t.initial_stop = stop;
        t.total_quantity = dec!(1);
        t.status = TradeStatus::Open;
        t.current_price = Some(price);
        t.current_price_updated_at = Some(Utc::now().to_rfc3339());
        t.current_price_source = Some("manual".into());
        t
    }

    fn closed_trade(id: i64, pnl: Decimal, closed_at: String) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.symbol = "EURUSD".into();
        t.side = Side::Long;
        t.entry_price = dec!(1.1000);
        t.initial_stop = dec!(1.0950);
        t.total_quantity = dec!(1);
        t.status = TradeStatus::Closed;
        t.manual_pnl = Some(pnl);
        t.closed_at = Some(closed_at);
        t
    }

    fn base_input<'a>(
        trades: &'a [Trade],
        instruments: &'a HashMap<String, InstrumentSpec>,
    ) -> DailyPnLInput<'a> {
        DailyPnLInput {
            trades,
            day_start_equity: dec!(10000),
            current_equity: dec!(10000),
            external_cash_flows: Decimal::ZERO,
            day_start_time: Utc::now() - chrono::Duration::hours(1),
            instruments,
            max_price_age_minutes: 60,
            now: Utc::now(),
            daily_loss_limit: dec!(300),
            execution_buffer: Decimal::ZERO,
        }
    }

    #[test]
    fn empty_day_zero() {
        let trades: Vec<Trade> = vec![];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let r = compute(&inp).unwrap();
        assert_eq!(r.realized_pnl, Decimal::ZERO);
        assert_eq!(r.floating_pnl, Decimal::ZERO);
        assert_eq!(r.daily_loss_used, Decimal::ZERO);
        assert_eq!(r.daily_remaining, dec!(300));
    }

    #[test]
    fn long_floating_profit() {
        let t = open_trade(1, dec!(1.1000), dec!(1.1020), dec!(1.0950), Side::Long);
        let trades = vec![t];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let r = compute(&inp).unwrap();
        // 0.0020 / 0.00001 = 200 ticks × $1 × 1 lot = $200
        assert_eq!(r.floating_pnl, dec!(200));
    }

    #[test]
    fn short_floating_profit() {
        let t = open_trade(1, dec!(1.1000), dec!(1.0980), dec!(1.1050), Side::Short);
        let trades = vec![t];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let r = compute(&inp).unwrap();
        assert_eq!(r.floating_pnl, dec!(200));
    }

    #[test]
    fn floating_loss_adds_to_daily() {
        // Equity dropped $150 from open loss
        let t = open_trade(1, dec!(1.1000), dec!(1.0985), dec!(1.0950), Side::Long);
        let trades = vec![t];
        let ins = instruments();
        let mut inp = base_input(&trades, &ins);
        inp.current_equity = dec!(9850);
        let r = compute(&inp).unwrap();
        assert_eq!(r.floating_pnl, dec!(-150));
        assert_eq!(r.daily_net_pnl, dec!(-150));
        assert_eq!(r.daily_loss_used, dec!(150));
        // Remaining downside = from 1.0985 to 1.0950 = 350 ticks = $350
        assert_eq!(r.remaining_downside, dec!(350));
        // DailyRemaining = 300 - 150 - 350 - 0 = -200 → 0
        assert_eq!(r.daily_remaining, Decimal::ZERO);
    }

    #[test]
    fn missing_current_price_blocks() {
        let mut t = open_trade(1, dec!(1.1000), dec!(1.1020), dec!(1.0950), Side::Long);
        t.current_price = None;
        let trades = vec![t];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let e = compute(&inp).unwrap_err();
        matches!(e, DailyPnLError::FloatingPnlUnknown { .. });
    }

    #[test]
    fn stale_price_blocks() {
        let mut t = open_trade(1, dec!(1.1000), dec!(1.1020), dec!(1.0950), Side::Long);
        t.current_price_updated_at = Some((Utc::now() - Duration::minutes(120)).to_rfc3339());
        let trades = vec![t];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let e = compute(&inp).unwrap_err();
        matches!(e, DailyPnLError::MarketDataStale { .. });
    }

    #[test]
    fn missing_instrument_blocks() {
        let mut t = open_trade(1, dec!(1.1000), dec!(1.1020), dec!(1.0950), Side::Long);
        t.symbol = "UNKNOWN".into();
        let trades = vec![t];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let e = compute(&inp).unwrap_err();
        matches!(e, DailyPnLError::InstrumentMissing { .. });
    }

    #[test]
    fn realized_win_and_loss() {
        let now = Utc::now();
        let t1 = closed_trade(1, dec!(100), (now - Duration::minutes(30)).to_rfc3339());
        let t2 = closed_trade(2, dec!(-50), (now - Duration::minutes(10)).to_rfc3339());
        let trades = vec![t1, t2];
        let ins = instruments();
        let inp = base_input(&trades, &ins);
        let r = compute(&inp).unwrap();
        assert_eq!(r.realized_pnl, dec!(50));
    }

    #[test]
    fn deposits_reduce_daily_loss() {
        // Equity up $500, but $500 deposit happened
        let trades: Vec<Trade> = vec![];
        let ins = instruments();
        let mut inp = base_input(&trades, &ins);
        inp.current_equity = dec!(10500);
        inp.external_cash_flows = dec!(500);
        let r = compute(&inp).unwrap();
        assert_eq!(r.daily_net_pnl, Decimal::ZERO);
        assert_eq!(r.daily_loss_used, Decimal::ZERO);
    }

    #[test]
    fn daily_loss_limit_disabled() {
        let t = open_trade(1, dec!(1.1000), dec!(1.0980), dec!(1.0950), Side::Long);
        let trades = vec![t];
        let ins = instruments();
        let mut inp = base_input(&trades, &ins);
        inp.daily_loss_limit = Decimal::ZERO;
        inp.current_equity = dec!(9800);
        let r = compute(&inp).unwrap();
        assert_eq!(r.daily_remaining, Decimal::ZERO); // disabled
        assert_eq!(r.daily_loss_used, dec!(200));
    }
}
