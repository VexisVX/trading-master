//! R:R (risk-to-reward) helpers.
//!
//! "Planned R:R" is derived from entry/stop/take_profit at creation time.
//! "Realized R" is the actual R-multiple using manual_pnl if present, or
//! computed from legs. Both are useful for plan-adherence analysis:
//! was the trader faithful to their own plan?

use crate::domain::trade::{Side, Trade};
use rust_decimal::Decimal;

/// Planned reward:risk ratio from entry, stop, and take-profit.
/// Returns None if no TP or invalid prices.
pub fn planned_rr(entry: Decimal, stop: Decimal, take_profit: Option<Decimal>) -> Option<Decimal> {
    let tp = take_profit?;
    let risk = (entry - stop).abs();
    let reward = (tp - entry).abs();
    if risk.is_zero() {
        return None;
    }
    Some(reward / risk)
}

/// R-multiple for a closed trade.
/// - If `manual_pnl` is present: R = manual_pnl / initial_risk
/// - Else: compute from legs (price × quantity)
/// Returns None if the trade is not closed or the risk is zero.
pub fn realized_r(t: &Trade) -> Option<Decimal> {
    if t.status != crate::domain::trade::TradeStatus::Closed {
        return None;
    }
    let risk_per_unit = (t.entry_price - t.initial_stop).abs();
    let initial_risk = risk_per_unit * t.total_quantity;
    if initial_risk.is_zero() {
        return None;
    }

    if let Some(manual) = t.manual_pnl {
        return Some(manual / initial_risk);
    }

    // Price-derived P&L
    let mut gross = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    for leg in t
        .legs
        .iter()
        .filter(|l| matches!(l.kind, crate::domain::trade::LegKind::Exit))
    {
        let diff = if t.side == Side::Long {
            leg.price - t.entry_price
        } else {
            t.entry_price - leg.price
        };
        gross += diff * leg.quantity;
        cost += leg.cost_per_unit * leg.quantity;
    }
    let net = gross - cost;
    Some(net / initial_risk)
}

/// Planned R:R as a convenience accessor on the Trade.
pub fn trade_planned_rr(t: &Trade) -> Option<Decimal> {
    planned_rr(t.entry_price, t.initial_stop, t.take_profit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn planned_rr_basic() {
        // entry 1.10, stop 1.09, tp 1.13 → risk 0.01, reward 0.03 → R:R 3
        assert_eq!(
            planned_rr(dec!(1.10), dec!(1.09), Some(dec!(1.13))),
            Some(dec!(3))
        );
    }

    #[test]
    fn planned_rr_short() {
        // entry 1.10, stop 1.11, tp 1.07 → risk 0.01, reward 0.03 → R:R 3
        assert_eq!(
            planned_rr(dec!(1.10), dec!(1.11), Some(dec!(1.07))),
            Some(dec!(3))
        );
    }

    #[test]
    fn planned_rr_no_tp() {
        assert_eq!(planned_rr(dec!(1.10), dec!(1.09), None), None);
    }

    #[test]
    fn planned_rr_zero_risk() {
        assert_eq!(planned_rr(dec!(1.10), dec!(1.10), Some(dec!(1.12))), None);
    }
}
