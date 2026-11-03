//! Internal profit protection (audit §4).
//!
//! F_internal = max(F_profit, F_trail). Independent of prop rules.
//! Math is Decimal. No floats.

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct ProtectionConfig {
    pub profit_protect_activation: Decimal, // default 0.05
    pub profit_protect_full: Decimal,       // default 0.10
    pub max_profit_retention: Decimal,      // default 0.40
    pub trailing_dd_pct: Decimal,           // default 0.10
}

impl ProtectionConfig {
    pub fn default_v1() -> Self {
        Self {
            profit_protect_activation: Decimal::new(5, 2),
            profit_protect_full: Decimal::new(10, 2),
            max_profit_retention: Decimal::new(40, 2),
            trailing_dd_pct: Decimal::new(10, 2),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProtectionResult {
    pub f_profit: Option<Decimal>,
    pub f_trail: Decimal,
    pub f_internal: Decimal,
    pub q_retention: Decimal,
}

fn smoothstep(x: Decimal) -> Decimal {
    let x2 = x * x;
    x2 * Decimal::from(3) - x2 * x * Decimal::from(2)
}

pub fn compute(units: Decimal, nav_hwm: Decimal, cfg: &ProtectionConfig) -> ProtectionResult {
    let f_trail = units * nav_hwm * (Decimal::ONE - cfg.trailing_dd_pct);
    let g = nav_hwm - Decimal::ONE;

    let (f_profit, q_retention) = if g < cfg.profit_protect_activation {
        (None, Decimal::ZERO)
    } else {
        let span = cfg.profit_protect_full - cfg.profit_protect_activation;
        if span <= Decimal::ZERO {
            (None, Decimal::ZERO)
        } else {
            let mut x = (g - cfg.profit_protect_activation) / span;
            if x < Decimal::ZERO { x = Decimal::ZERO; }
            if x > Decimal::ONE { x = Decimal::ONE; }
            let q = cfg.max_profit_retention * smoothstep(x);
            let fp = units * (Decimal::ONE + q * g);
            (Some(fp), q)
        }
    };

    let f_internal = match f_profit { Some(fp) => fp.max(f_trail), None => f_trail };

    ProtectionResult { f_profit, f_trail, f_internal, q_retention }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn cfg() -> ProtectionConfig { ProtectionConfig::default_v1() }

    #[test]
    fn below_activation_no_f_profit() {
        let r = compute(dec!(10000), dec!(1.03), &cfg());
        assert!(r.f_profit.is_none());
        assert_eq!(r.q_retention, Decimal::ZERO);
        assert_eq!(r.f_trail, dec!(9270.00));
        assert_eq!(r.f_internal, dec!(9270.00));
    }

    #[test]
    fn at_activation_zero_q() {
        let r = compute(dec!(10000), dec!(1.05), &cfg());
        assert_eq!(r.f_profit.unwrap(), dec!(10000));
    }

    #[test]
    fn at_full_protection_max_q() {
        let r = compute(dec!(10000), dec!(1.10), &cfg());
        assert_eq!(r.f_profit.unwrap(), dec!(10400.00));
        assert_eq!(r.q_retention, dec!(0.40));
    }

    #[test]
    fn f_internal_is_max() {
        let r = compute(dec!(10000), dec!(1.10), &cfg());
        assert_eq!(r.f_internal, dec!(10400.00));
    }

    #[test]
    fn smoothstep_monotonic() {
        let c = cfg();
        let mut prev = Decimal::ZERO;
        for i in 0..=10 {
            let g = Decimal::new(5 + i * 5, 2);
            let nav = Decimal::ONE + g;
            let r = compute(dec!(10000), nav, &c);
            let fp = r.f_profit.unwrap_or(Decimal::ZERO);
            assert!(fp >= prev);
            prev = fp;
        }
    }
}
