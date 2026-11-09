//! Money primitives: Decimal-only, explicit rounding.
//! Never use f64/f32 here.

#![allow(dead_code)] // TODO: used by Journal/Strategy in later phases

use rust_decimal::{Decimal, RoundingStrategy};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum MoneyError {
    #[error("division by zero")]
    DivByZero,
    #[error("negative amount not allowed: {0}")]
    NegativeAmount(String),
    #[error("invalid decimal string: {0}")]
    InvalidDecimal(String),
}

/// Round money to `scale` decimals using banker-ish default (MidpointAwayFromZero).
/// Money should never silently round; this function is the only allowed path.
/// Round money to an instrument-specific scale.
/// Unlike `round_money`, the scale is supplied by the caller (from
/// `InstrumentSpec`). Use this for sizing where 2dp is too coarse.
pub fn round_money_scale(v: Decimal, scale: u32) -> Decimal {
    v.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

pub fn round_money(v: Decimal, scale: u32) -> Decimal {
    v.round_dp_with_strategy(scale, RoundingStrategy::MidpointAwayFromZero)
}

/// Round statistics (win rate, R multiples) using MidpointEven.
pub fn round_stat(v: Decimal, scale: u32) -> Decimal {
    v.round_dp_with_strategy(scale, RoundingStrategy::MidpointNearestEven)
}

/// Parse a Decimal from string, rejecting empty/NaN-like input.
pub fn parse_decimal(s: &str) -> Result<Decimal, MoneyError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(MoneyError::InvalidDecimal(s.to_string()));
    }
    Decimal::from_str_exact(trimmed).map_err(|_| MoneyError::InvalidDecimal(s.to_string()))
}

/// Safely divide, returning error on zero denominator.
pub fn safe_div(a: Decimal, b: Decimal) -> Result<Decimal, MoneyError> {
    if b.is_zero() {
        Err(MoneyError::DivByZero)
    } else {
        Ok(a / b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_money_midpoint_away_from_zero() {
        // 1.005 at 2dp -> 1.01
        let v = parse_decimal("1.005").unwrap();
        assert_eq!(round_money(v, 2), parse_decimal("1.01").unwrap());
    }

    #[test]
    fn round_stat_midpoint_even() {
        // 2.5 at 0dp -> 2 (even), 3.5 -> 4
        assert_eq!(
            round_stat(parse_decimal("2.5").unwrap(), 0),
            Decimal::from(2)
        );
        assert_eq!(
            round_stat(parse_decimal("3.5").unwrap(), 0),
            Decimal::from(4)
        );
    }

    #[test]
    fn parse_rejects_empty() {
        assert!(parse_decimal("").is_err());
        assert!(parse_decimal("   ").is_err());
    }

    #[test]
    fn safe_div_zero() {
        assert_eq!(
            safe_div(Decimal::ONE, Decimal::ZERO),
            Err(MoneyError::DivByZero)
        );
    }
}
