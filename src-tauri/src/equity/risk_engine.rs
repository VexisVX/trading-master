//! Risk engine (audit §6, §8).
//!
//! R_base = Equity * BaseRiskPercent
//! R_target = R_base * M_preserve
//! R_allowed = min(R_target, R_buffer, PortfolioRemaining, DailyRemaining, PropRemaining)
//!
//! All money values are Decimal.

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct RiskInput {
    pub equity: Decimal,
    pub r_base_pct: Decimal,
    pub m_preserve: Decimal,
    pub r_buffer: Decimal,
    pub r_portfolio_remaining: Decimal,
    pub r_daily_remaining: Decimal,
    pub r_prop_remaining: Decimal,
}

#[derive(Debug, Clone)]
pub struct RiskResult {
    pub r_base: Decimal,
    pub r_target: Decimal,
    pub r_buffer: Decimal,
    pub r_portfolio_cap: Decimal,
    pub r_daily_cap: Decimal,
    pub r_prop_cap: Decimal,
    pub r_allowed: Decimal,
    pub binding: &'static str,
}

pub fn compute(i: &RiskInput) -> RiskResult {
    let r_base = i.equity * i.r_base_pct;
    let r_target = r_base * i.m_preserve;

    let mut r_allowed = r_target;
    let mut binding: &'static str = "Target";

    if i.r_buffer < r_allowed {
        r_allowed = i.r_buffer;
        binding = "Buffer";
    }
    if i.r_portfolio_remaining < r_allowed {
        r_allowed = i.r_portfolio_remaining;
        binding = "Portfolio";
    }
    if i.r_daily_remaining < r_allowed {
        r_allowed = i.r_daily_remaining;
        binding = "Daily";
    }
    if i.r_prop_remaining < r_allowed {
        r_allowed = i.r_prop_remaining;
        binding = "Prop";
    }

    if r_allowed < Decimal::ZERO {
        r_allowed = Decimal::ZERO;
    }

    // Display-friendly caps: when a cap is Decimal::MAX (disabled),
    // show r_target instead of the sentinel value.
    let display_cap = |v: Decimal| -> Decimal {
        if v == Decimal::MAX {
            r_target
        } else {
            v
        }
    };

    RiskResult {
        r_base,
        r_target,
        r_buffer: i.r_buffer,
        r_portfolio_cap: display_cap(i.r_portfolio_remaining),
        r_daily_cap: display_cap(i.r_daily_remaining),
        r_prop_cap: display_cap(i.r_prop_remaining),
        r_allowed,
        binding,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base() -> RiskInput {
        RiskInput {
            equity: dec!(10000),
            r_base_pct: dec!(0.01),
            m_preserve: Decimal::ONE,
            r_buffer: dec!(1000),
            r_portfolio_remaining: dec!(1000),
            r_daily_remaining: dec!(1000),
            r_prop_remaining: dec!(1000),
        }
    }

    #[test]
    fn happy_path() {
        let r = compute(&base());
        assert_eq!(r.r_base, dec!(100));
        assert_eq!(r.r_target, dec!(100));
        assert_eq!(r.r_allowed, dec!(100));
        assert_eq!(r.binding, "Target");
    }

    #[test]
    fn m_preserve_halves() {
        let mut i = base();
        i.m_preserve = dec!(0.5);
        let r = compute(&i);
        assert_eq!(r.r_target, dec!(50));
        assert_eq!(r.r_allowed, dec!(50));
    }

    #[test]
    fn buffer_binds() {
        let mut i = base();
        i.r_buffer = dec!(30);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(30));
        assert_eq!(r.binding, "Buffer");
    }

    #[test]
    fn portfolio_binds() {
        let mut i = base();
        i.r_portfolio_remaining = dec!(20);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(20));
        assert_eq!(r.binding, "Portfolio");
    }

    #[test]
    fn daily_binds() {
        let mut i = base();
        i.r_daily_remaining = dec!(10);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(10));
        assert_eq!(r.binding, "Daily");
    }

    #[test]
    fn prop_binds() {
        let mut i = base();
        i.r_prop_remaining = dec!(5);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(5));
        assert_eq!(r.binding, "Prop");
    }
}
