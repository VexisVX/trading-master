//! Drawdown computation on an equity series.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::money::{round_stat, safe_div, MoneyError};

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DrawdownError {
    #[error("equity series is empty")]
    Empty,
    #[error("non-positive initial equity")]
    NonPositiveInitial,
    #[error("money error: {0}")]
    Money(#[from] MoneyError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawdownPoint {
    pub index: usize,
    pub equity: Decimal,
    pub peak: Decimal,
    /// Absolute drawdown from peak (equity - peak, negative or zero).
    pub drawdown: Decimal,
    /// Drawdown as a fraction of peak (0..1).
    pub drawdown_pct: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DrawdownResult {
    pub series: Vec<DrawdownPoint>,
    pub max_drawdown: Decimal,
    pub max_drawdown_pct: Decimal,
    pub max_drawdown_index: usize,
}

pub fn compute_drawdown(equity: &[Decimal]) -> Result<DrawdownResult, DrawdownError> {
    if equity.is_empty() {
        return Err(DrawdownError::Empty);
    }
    if equity[0] <= Decimal::ZERO {
        return Err(DrawdownError::NonPositiveInitial);
    }

    let mut series = Vec::with_capacity(equity.len());
    let mut peak = equity[0];
    let mut max_dd = Decimal::ZERO;
    let mut max_dd_pct = Decimal::ZERO;
    let mut max_dd_idx = 0;

    for (i, &eq) in equity.iter().enumerate() {
        if eq > peak {
            peak = eq;
        }
        let dd = eq - peak;
        let dd_pct = if peak.is_zero() {
            Decimal::ZERO
        } else {
            safe_div(dd, peak)?
        };
        if dd < max_dd {
            max_dd = dd;
            max_dd_pct = dd_pct;
            max_dd_idx = i;
        }
        series.push(DrawdownPoint {
            index: i,
            equity: eq,
            peak,
            drawdown: dd,
            drawdown_pct: round_stat(dd_pct, 6),
        });
    }

    Ok(DrawdownResult {
        series,
        max_drawdown: max_dd,
        max_drawdown_pct: round_stat(max_dd_pct, 6),
        max_drawdown_index: max_dd_idx,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn no_drawdown_when_monotonic_increasing() {
        let eq = vec![dec!(100), dec!(110), dec!(120)];
        let r = compute_drawdown(&eq).unwrap();
        assert_eq!(r.max_drawdown, dec!(0));
        assert_eq!(r.max_drawdown_pct, dec!(0));
    }

    #[test]
    fn simple_drawdown() {
        let eq = vec![dec!(100), dec!(120), dec!(90), dec!(110)];
        let r = compute_drawdown(&eq).unwrap();
        // peak 120, low 90 => dd = -30, pct = -0.25
        assert_eq!(r.max_drawdown, dec!(-30));
        assert_eq!(r.max_drawdown_pct, dec!(-0.25));
        assert_eq!(r.max_drawdown_index, 2);
    }

    #[test]
    fn empty_rejected() {
        assert_eq!(compute_drawdown(&[]), Err(DrawdownError::Empty));
    }

    #[test]
    fn non_positive_initial_rejected() {
        assert_eq!(
            compute_drawdown(&[dec!(0), dec!(10)]),
            Err(DrawdownError::NonPositiveInitial)
        );
    }
}
