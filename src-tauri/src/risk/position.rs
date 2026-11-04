//! Position sizing.
//! All inputs come from the user or verified instrument specs.
//! No defaults are assumed — every field must be supplied.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::money::{round_money, safe_div, MoneyError};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PositionError {
    #[error("risk amount must be > 0")]
    NonPositiveRisk,
    #[error("stop distance must be > 0")]
    NonPositiveStop,
    #[error("tick size must be > 0")]
    NonPositiveTickSize,
    #[error("tick value must be > 0")]
    NonPositiveTickValue,
    #[error("contract size must be > 0")]
    NonPositiveContractSize,
    #[error("money error: {0}")]
    Money(#[from] MoneyError),
}

/// Inputs required to compute position size.
/// All decimals come from strings at the Tauri boundary (serde-with-str).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSizeInput {
    /// Account balance in account currency.
    pub account_balance: Decimal,
    /// Risk per trade as a fraction (e.g. 0.01 = 1%).
    pub risk_fraction: Decimal,
    /// Entry price.
    pub entry_price: Decimal,
    /// Stop-loss price.
    pub stop_price: Decimal,
    /// Smallest price increment (user-verified).
    pub tick_size: Decimal,
    /// Monetary value of one tick, in account currency (user-verified).
    pub tick_value: Decimal,
    /// Units per 1.0 lot (user-verified; e.g. 100000 for EURUSD).
    pub contract_size: Decimal,
    /// Round-trip commission per lot in account currency (0 if unknown).
    pub commission_per_lot: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PositionSizeResult {
    /// Amount at risk in account currency (balance * risk_fraction).
    pub risk_amount: Decimal,
    /// Distance from entry to stop in price units.
    pub stop_distance: Decimal,
    /// Distance expressed in ticks.
    pub stop_ticks: Decimal,
    /// Loss per 1.0 lot if stopped, including commission.
    pub loss_per_lot: Decimal,
    /// Suggested position size in lots (rounded to 2 decimals).
    pub lots: Decimal,
    /// Actual risk at suggested lots (≤ risk_amount).
    pub actual_risk: Decimal,
}

pub fn compute_position_size(
    input: &PositionSizeInput,
) -> Result<PositionSizeResult, PositionError> {
    if input.risk_fraction <= Decimal::ZERO {
        return Err(PositionError::NonPositiveRisk);
    }
    if input.tick_size <= Decimal::ZERO {
        return Err(PositionError::NonPositiveTickSize);
    }
    if input.tick_value <= Decimal::ZERO {
        return Err(PositionError::NonPositiveTickValue);
    }
    if input.contract_size <= Decimal::ZERO {
        return Err(PositionError::NonPositiveContractSize);
    }

    let stop_distance = (input.entry_price - input.stop_price).abs();
    if stop_distance <= Decimal::ZERO {
        return Err(PositionError::NonPositiveStop);
    }

    let stop_ticks = safe_div(stop_distance, input.tick_size)?;
    let risk_amount = round_money(input.account_balance * input.risk_fraction, 2);

    // Loss per 1.0 lot = stop_ticks * tick_value + commission
    let loss_per_lot = round_money(stop_ticks * input.tick_value + input.commission_per_lot, 2);
    if loss_per_lot <= Decimal::ZERO {
        return Err(PositionError::NonPositiveStop);
    }

    let raw_lots = safe_div(risk_amount, loss_per_lot)?;
    // Round DOWN to 2 decimals so we never exceed risk budget.
    let lots = raw_lots.round_dp_with_strategy(2, rust_decimal::RoundingStrategy::ToZero);
    let actual_risk = round_money(lots * loss_per_lot, 2);

    Ok(PositionSizeResult {
        risk_amount,
        stop_distance,
        stop_ticks: stop_ticks.round_dp(4),
        loss_per_lot,
        lots,
        actual_risk,
    })
}

// ============================================================
// S7: size from R_allowed directly + instrument step/min/max
// ============================================================

#[derive(Debug, Clone)]
pub struct SizeFromAllowedInput {
    pub r_allowed: Decimal,
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub tick_size: Decimal,
    pub tick_value: Decimal,
    pub contract_size: Decimal,
    pub commission_per_lot: Decimal,
    pub lot_step: Decimal,
    pub min_qty: Decimal,
    pub max_qty: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeFromAllowedResult {
    pub r_allowed: Decimal,
    pub lots: Decimal,
    pub actual_risk: Decimal,
    pub clamped_to_min: bool,
    pub clamped_to_max: bool,
    pub rejected_below_min: bool,
}

pub fn size_from_allowed(i: &SizeFromAllowedInput) -> Result<SizeFromAllowedResult, PositionError> {
    eprintln!(
        "[SIZE] r_allowed={} entry={} stop={} tick_size={} tick_value={} contract_size={} lot_step={} min_qty={} max_qty={}",
        i.r_allowed, i.entry_price, i.stop_price, i.tick_size, i.tick_value,
        i.contract_size, i.lot_step, i.min_qty, i.max_qty
    );

    if i.r_allowed <= Decimal::ZERO {
        return Err(PositionError::NonPositiveRisk);
    }
    if i.tick_size <= Decimal::ZERO {
        return Err(PositionError::NonPositiveTickSize);
    }
    if i.tick_value <= Decimal::ZERO {
        return Err(PositionError::NonPositiveTickValue);
    }
    if i.contract_size <= Decimal::ZERO {
        return Err(PositionError::NonPositiveContractSize);
    }
    let stop_distance = (i.entry_price - i.stop_price).abs();
    if stop_distance <= Decimal::ZERO {
        return Err(PositionError::NonPositiveStop);
    }

    let stop_ticks = safe_div(stop_distance, i.tick_size)?;
    let loss_per_lot = round_money(stop_ticks * i.tick_value + i.commission_per_lot, 2);
    if loss_per_lot <= Decimal::ZERO {
        return Err(PositionError::NonPositiveStop);
    }

    let raw = safe_div(i.r_allowed, loss_per_lot)?;
    let step = if i.lot_step > Decimal::ZERO {
        i.lot_step
    } else {
        Decimal::new(1, 2)
    };
    // Round DOWN to step.
    let steps = (raw / step).round_dp_with_strategy(0, rust_decimal::RoundingStrategy::ToZero);
    let mut lots = steps * step;

    // Apply min/max bounds.
    let mut clamped_min = false;
    let mut clamped_max = false;
    if i.min_qty > Decimal::ZERO && lots < i.min_qty {
        lots = i.min_qty;
        clamped_min = true;
    }
    if i.max_qty > Decimal::ZERO && lots > i.max_qty {
        lots = i.max_qty;
        clamped_max = true;
    }

    let actual_risk = round_money(lots * loss_per_lot, 2);
    let rejected_below_min = actual_risk > i.r_allowed && clamped_min;

    Ok(SizeFromAllowedResult {
        r_allowed: i.r_allowed,
        lots,
        actual_risk,
        clamped_to_min: clamped_min,
        clamped_to_max: clamped_max,
        rejected_below_min,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base() -> PositionSizeInput {
        PositionSizeInput {
            account_balance: dec!(10000),
            risk_fraction: dec!(0.01), // 1% = $100
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950), // 50 pip stop
            tick_size: dec!(0.0001), // 1 pip = 10 ticks on 5-digit broker? No: tick = 0.00001 typically
            tick_value: dec!(1),     // $1 per tick per 1.0 lot
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
        }
    }

    #[test]
    fn basic_position_size() {
        // stop_distance = 0.0050, tick_size = 0.0001 -> 50 ticks
        // loss_per_lot = 50 * $1 = $50
        // risk_amount = $100
        // lots = 100 / 50 = 2.00
        let r = compute_position_size(&base()).unwrap();
        assert_eq!(r.risk_amount, dec!(100.00));
        assert_eq!(r.stop_ticks, dec!(50.0000));
        assert_eq!(r.loss_per_lot, dec!(50.00));
        assert_eq!(r.lots, dec!(2.00));
        assert_eq!(r.actual_risk, dec!(100.00));
    }

    #[test]
    fn never_exceeds_risk_budget() {
        // 0.7 risk per lot scenario: risk_amount = 100, loss_per_lot = 60 -> 1.66 lots (not 1.67)
        let mut i = base();
        i.stop_price = dec!(1.0940); // 60 ticks -> $60 loss per lot
        let r = compute_position_size(&i).unwrap();
        assert_eq!(r.lots, dec!(1.66));
        assert!(r.actual_risk <= r.risk_amount);
    }

    #[test]
    fn commission_included() {
        let mut i = base();
        i.commission_per_lot = dec!(7);
        let r = compute_position_size(&i).unwrap();
        // loss_per_lot = 50 + 7 = 57; lots = 100/57 = 1.75
        assert_eq!(r.loss_per_lot, dec!(57.00));
        assert_eq!(r.lots, dec!(1.75));
    }

    #[test]
    fn rejects_zero_stop() {
        let mut i = base();
        i.stop_price = i.entry_price;
        assert_eq!(
            compute_position_size(&i),
            Err(PositionError::NonPositiveStop)
        );
    }

    #[test]
    fn rejects_zero_tick() {
        let mut i = base();
        i.tick_size = Decimal::ZERO;
        assert_eq!(
            compute_position_size(&i),
            Err(PositionError::NonPositiveTickSize)
        );
    }

    #[test]
    fn long_and_short_symmetric() {
        let mut long = base();
        long.entry_price = dec!(1.1050);
        long.stop_price = dec!(1.1000);
        let mut short = base();
        short.entry_price = dec!(1.1000);
        short.stop_price = dec!(1.1050);
        let rl = compute_position_size(&long).unwrap();
        let rs = compute_position_size(&short).unwrap();
        assert_eq!(rl.lots, rs.lots);
        assert_eq!(rl.stop_distance, rs.stop_distance);
    }

    #[test]
    fn size_from_allowed_basic() {
        // R = 100, stop 50 ticks, tick 1 → loss_per_lot = 50
        // lots = 100/50 = 2.00
        let r = size_from_allowed(&SizeFromAllowedInput {
            r_allowed: dec!(100),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            tick_size: dec!(0.0001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
            lot_step: dec!(0.01),
            min_qty: dec!(0.01),
            max_qty: dec!(100),
        })
        .unwrap();
        assert_eq!(r.lots, dec!(2.00));
        assert_eq!(r.actual_risk, dec!(100.00));
    }

    #[test]
    fn size_from_allowed_lot_step_rounds_down() {
        // R = 100, loss_per_lot = 60 → raw = 1.6667 → step 0.05 → 1.65
        let r = size_from_allowed(&SizeFromAllowedInput {
            r_allowed: dec!(100),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0940),
            tick_size: dec!(0.0001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
            lot_step: dec!(0.05),
            min_qty: dec!(0),
            max_qty: dec!(0),
        })
        .unwrap();
        assert_eq!(r.lots, dec!(1.65));
    }

    #[test]
    fn size_from_allowed_min_qty() {
        let r = size_from_allowed(&SizeFromAllowedInput {
            r_allowed: dec!(1),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            tick_size: dec!(0.0001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
            lot_step: dec!(0.01),
            min_qty: dec!(0.10),
            max_qty: dec!(0),
        })
        .unwrap();
        assert_eq!(r.lots, dec!(0.10));
        assert!(r.clamped_to_min);
        assert!(r.rejected_below_min);
    }

    #[test]
    fn size_from_allowed_max_qty() {
        let r = size_from_allowed(&SizeFromAllowedInput {
            r_allowed: dec!(10000),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            tick_size: dec!(0.0001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
            lot_step: dec!(0.01),
            min_qty: dec!(0),
            max_qty: dec!(1.00),
        })
        .unwrap();
        assert_eq!(r.lots, dec!(1.00));
        assert!(r.clamped_to_max);
    }

    #[test]
    fn size_from_allowed_rejects_zero_r() {
        assert!(size_from_allowed(&SizeFromAllowedInput {
            r_allowed: Decimal::ZERO,
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            tick_size: dec!(0.0001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            commission_per_lot: dec!(0),
            lot_step: dec!(0.01),
            min_qty: dec!(0),
            max_qty: dec!(0),
        })
        .is_err());
    }
}
