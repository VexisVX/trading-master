//! R-multiple computation.
//! Supports partial exits and stop-loss modifications.
//! "R" = initial risk per 1.0 unit of size at entry.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::money::{round_money, round_stat, safe_div, MoneyError};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RError {
    #[error("initial risk must be > 0")]
    NonPositiveInitialRisk,
    #[error("exit quantity must be > 0")]
    NonPositiveQty,
    #[error("exit prices required")]
    NoExits,
    #[error("money error: {0}")]
    Money(#[from] MoneyError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exit {
    pub price: Decimal,
    pub quantity: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeR {
    /// Total position size (sum of entries).
    pub total_quantity: Decimal,
    /// Entry price (single entry for MVP; averaging handled later).
    pub entry_price: Decimal,
    /// Original stop price at the time of entry.
    pub initial_stop: Decimal,
    /// Direction: true = long.
    pub is_long: bool,
    /// Slippage/commission per unit of size (subtracted from gross).
    pub cost_per_unit: Decimal,
    /// Partial or final exits. Sum of quantities must equal total_quantity.
    pub exits: Vec<Exit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RResult {
    /// Total gross P&L before costs.
    pub gross_pnl: Decimal,
    /// Total costs.
    pub total_costs: Decimal,
    /// Net P&L.
    pub net_pnl: Decimal,
    /// Initial risk in account currency (|entry - stop| * qty).
    pub initial_risk: Decimal,
    /// Net P&L / initial risk.
    pub r_multiple: Decimal,
}

pub fn compute_r(t: &TradeR) -> Result<RResult, RError> {
    if t.exits.is_empty() {
        return Err(RError::NoExits);
    }

    let initial_risk_per_unit = (t.entry_price - t.initial_stop).abs();
    // Use high precision for the intermediate risk value.
    // Rounding to 2 dp here destroys R for small-priced instruments.
    let initial_risk_raw = initial_risk_per_unit * t.total_quantity;
    if initial_risk_raw <= Decimal::ZERO {
        return Err(RError::NonPositiveInitialRisk);
    }
    let initial_risk = round_stat(initial_risk_raw, 8);

    let mut gross = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    let mut exited_qty = Decimal::ZERO;

    for ex in &t.exits {
        if ex.quantity <= Decimal::ZERO {
            return Err(RError::NonPositiveQty);
        }
        let diff = if t.is_long {
            ex.price - t.entry_price
        } else {
            t.entry_price - ex.price
        };
        gross += diff * ex.quantity;
        cost += t.cost_per_unit * ex.quantity;
        exited_qty += ex.quantity;
    }

    if exited_qty != t.total_quantity {
        return Err(RError::NoExits);
    }

    let net_raw = gross - cost;
    let gross = round_money(gross, 2);
    let cost = round_money(cost, 2);
    let net = round_money(net_raw, 2);
    // R uses the full-precision numerator.
    let r = round_stat(safe_div(net_raw, initial_risk)?, 4);

    Ok(RResult {
        gross_pnl: gross,
        total_costs: cost,
        net_pnl: net,
        initial_risk,
        r_multiple: r,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn long_full() -> TradeR {
        TradeR {
            total_quantity: dec!(1.0),
            entry_price: dec!(1.1000),
            initial_stop: dec!(1.0950),        // 50 pips
            is_long: true,
            cost_per_unit: dec!(0),
            exits: vec![Exit { price: dec!(1.1100), quantity: dec!(1.0) }],
        }
    }

    #[test]
    fn simple_2r() {
        // risk = 0.0050 * 1 = 0.0050; profit = 0.0100 * 1 = 0.0100
        // R = 0.0100 / 0.0050 = 2.0
        let r = compute_r(&long_full()).unwrap();
        assert_eq!(r.r_multiple, dec!(2.0000));
        assert_eq!(r.net_pnl, dec!(0.01));
    }

    #[test]
    fn partial_exits() {
        let t = TradeR {
            total_quantity: dec!(2.0),
            entry_price: dec!(1.1000),
            initial_stop: dec!(1.0950),
            is_long: true,
            cost_per_unit: dec!(0),
            exits: vec![
                Exit { price: dec!(1.1100), quantity: dec!(1.0) }, // +0.0100 * 1
                Exit { price: dec!(1.1200), quantity: dec!(1.0) }, // +0.0200 * 1
            ],
        };
        let r = compute_r(&t).unwrap();
        // risk = 0.0050 * 2 = 0.0100; gross = 0.0300; R = 3.0
        assert_eq!(r.r_multiple, dec!(3.0000));
    }

    #[test]
    fn short_trade() {
        let t = TradeR {
            total_quantity: dec!(1.0),
            entry_price: dec!(1.1050),
            initial_stop: dec!(1.1100),
            is_long: false,
            cost_per_unit: dec!(0),
            exits: vec![Exit { price: dec!(1.0950), quantity: dec!(1.0) }],
        };
        let r = compute_r(&t).unwrap();
        // risk = 0.0050; profit = 0.0100; R = 2.0
        assert_eq!(r.r_multiple, dec!(2.0000));
    }

    #[test]
    fn costs_reduce_r() {
        let mut t = long_full();
        t.cost_per_unit = dec!(0.0005); // 0.5 pip per unit
        let r = compute_r(&t).unwrap();
        // net = 0.0100 - 0.0005 = 0.0095; R = 0.0095 / 0.0050 = 1.9
        assert_eq!(r.r_multiple, dec!(1.9000));
    }

    #[test]
    fn rejects_zero_risk() {
        let mut t = long_full();
        t.initial_stop = t.entry_price;
        assert_eq!(compute_r(&t), Err(RError::NonPositiveInitialRisk));
    }

    #[test]
    fn rejects_open_position() {
        let mut t = long_full();
        t.total_quantity = dec!(2.0);
        assert_eq!(compute_r(&t), Err(RError::NoExits));
    }
}
