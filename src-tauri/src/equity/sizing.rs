//! Position sizing (formulas 21–22).
//! size = r_allowed / ((|entry − stop| × multiplier) + commission_per_unit + slippage_buffer)

use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct SizingInput {
    pub entry: String,               // price
    pub stop: String,                // price
    pub contract_multiplier: String, // e.g. 100000 for 1 lot of EURUSD
    pub commission_per_unit: String, // money per unit (0 if unknown)
    pub slippage_buffer: String,     // money per unit (conservative)
}

#[derive(Debug, Clone)]
pub struct SizingResult {
    pub risk_amount: Decimal,   // = r_allowed (money)
    pub size: Decimal,          // units of the contract
    pub per_unit_cost: Decimal, // (|e−s|×mult + comm + slip)
    pub rr: Option<Decimal>,    // optional, if take_profit known
}

pub fn compute_size(
    r_allowed: Decimal,
    input: &SizingInput,
    take_profit: Option<&str>,
) -> Result<SizingResult, String> {
    let entry = Decimal::from_str(&input.entry).map_err(|e| format!("entry: {e}"))?;
    let stop = Decimal::from_str(&input.stop).map_err(|e| format!("stop: {e}"))?;
    let mult = Decimal::from_str(&input.contract_multiplier).map_err(|e| format!("mult: {e}"))?;
    let comm = Decimal::from_str(&input.commission_per_unit).map_err(|e| format!("comm: {e}"))?;
    let slip = Decimal::from_str(&input.slippage_buffer).map_err(|e| format!("slip: {e}"))?;

    let price_dist = (entry - stop).abs();
    if price_dist.is_zero() {
        return Err("entry equals stop — cannot size".into());
    }

    let per_unit = price_dist * mult + comm + slip;
    if per_unit <= Decimal::ZERO {
        return Err("per-unit cost is non-positive".into());
    }

    if r_allowed <= Decimal::ZERO {
        return Ok(SizingResult {
            risk_amount: r_allowed,
            size: Decimal::ZERO,
            per_unit_cost: per_unit,
            rr: None,
        });
    }

    let size = r_allowed / per_unit;

    // RR if TP provided
    let rr = match take_profit {
        Some(tp_str) if !tp_str.is_empty() => {
            let tp = Decimal::from_str(tp_str).map_err(|e| format!("tp: {e}"))?;
            let reward = (tp - entry).abs();
            if price_dist.is_zero() {
                None
            } else {
                Some(reward / price_dist)
            }
        }
        _ => None,
    };

    Ok(SizingResult {
        risk_amount: r_allowed,
        size,
        per_unit_cost: per_unit,
        rr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inp(entry: &str, stop: &str, mult: &str) -> SizingInput {
        SizingInput {
            entry: entry.into(),
            stop: stop.into(),
            contract_multiplier: mult.into(),
            commission_per_unit: "0".into(),
            slippage_buffer: "0".into(),
        }
    }

    #[test]
    fn basic_sizing() {
        // RA = 100, entry 1.1000, stop 1.0950, mult 100000
        // per_unit = 0.005 * 100000 = 500 → size = 100/500 = 0.2
        let r = compute_size(
            Decimal::from_str("100").unwrap(),
            &inp("1.1000", "1.0950", "100000"),
            None,
        )
        .unwrap();
        assert_eq!(r.size, Decimal::from_str("0.2").unwrap());
    }

    #[test]
    fn sizing_with_commission_and_slip() {
        // per_unit = 500 + 5 + 2 = 507 → size = 100/507 ≈ 0.197
        let mut i = inp("1.1000", "1.0950", "100000");
        i.commission_per_unit = "5".into();
        i.slippage_buffer = "2".into();
        let r = compute_size(Decimal::from_str("100").unwrap(), &i, None).unwrap();
        assert!(r.size < Decimal::from_str("0.2").unwrap());
        assert!(r.size > Decimal::from_str("0.19").unwrap());
    }

    #[test]
    fn zero_risk_zero_size() {
        let r = compute_size(Decimal::ZERO, &inp("1.1000", "1.0950", "100000"), None).unwrap();
        assert_eq!(r.size, Decimal::ZERO);
    }

    #[test]
    fn identical_prices_error() {
        let e = compute_size(
            Decimal::from_str("100").unwrap(),
            &inp("1.1000", "1.1000", "100000"),
            None,
        );
        assert!(e.is_err());
    }

    #[test]
    fn rr_computed() {
        // entry 1.1000, stop 1.0950 → risk 0.005; tp 1.1150 → reward 0.015 → RR = 3
        let r = compute_size(
            Decimal::from_str("100").unwrap(),
            &inp("1.1000", "1.0950", "100000"),
            Some("1.1150"),
        )
        .unwrap();
        assert_eq!(r.rr.unwrap(), Decimal::from_str("3").unwrap());
    }
}
