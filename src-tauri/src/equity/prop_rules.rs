//! Independent prop-firm rules (audit §5, §10).
//!
//! A breach of F_prop is a HARD prop-rule breach. Must never auto-clear.
//! Math is Decimal.

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdType {
    StaticBalance,
    StaticEquity,
    TrailingBalance,
    TrailingEquity,
}

impl DdType {
    pub fn as_str(self) -> &'static str {
        match self {
            DdType::StaticBalance => "static_balance",
            DdType::StaticEquity => "static_equity",
            DdType::TrailingBalance => "trailing_balance",
            DdType::TrailingEquity => "trailing_equity",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "static_balance" => DdType::StaticBalance,
            "static_equity" => DdType::StaticEquity,
            "trailing_balance" => DdType::TrailingBalance,
            "trailing_equity" => DdType::TrailingEquity,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PropRuleSet {
    pub enabled: bool,
    pub dd_type: DdType,
    pub dd_amount: Decimal,
}

#[derive(Debug, Clone)]
pub struct PropContext {
    pub initial_balance: Decimal,
    pub current_equity: Decimal,
    pub peak_equity: Decimal,
    pub peak_balance: Decimal,
    pub current_balance: Decimal,
}

#[derive(Debug, Clone)]
pub struct PropResult {
    pub f_prop: Decimal,
    pub breached: bool,
    pub reference: Decimal,
}

pub fn evaluate(rules: &PropRuleSet, ctx: &PropContext) -> PropResult {
    if !rules.enabled || rules.dd_amount <= Decimal::ZERO {
        return PropResult {
            f_prop: Decimal::ZERO,
            breached: false,
            reference: Decimal::ZERO,
        };
    }
    let reference = match rules.dd_type {
        DdType::StaticBalance => ctx.current_balance,
        DdType::StaticEquity => ctx.initial_balance,
        DdType::TrailingBalance => ctx.peak_balance,
        DdType::TrailingEquity => ctx.peak_equity,
    };
    let f_prop = reference - rules.dd_amount;
    let breached = ctx.current_equity <= f_prop;
    PropResult {
        f_prop,
        breached,
        reference,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ctx(eq: Decimal) -> PropContext {
        PropContext {
            initial_balance: dec!(10000),
            current_equity: eq,
            peak_equity: dec!(12000),
            peak_balance: dec!(11000),
            current_balance: dec!(10000),
        }
    }

    fn rules(t: DdType, amt: Decimal) -> PropRuleSet {
        PropRuleSet {
            enabled: true,
            dd_type: t,
            dd_amount: amt,
        }
    }

    #[test]
    fn disabled_always_pass() {
        let mut r = rules(DdType::StaticEquity, dec!(1000));
        r.enabled = false;
        assert!(!evaluate(&r, &ctx(dec!(100))).breached);
    }

    #[test]
    fn static_equity_uses_initial() {
        let r = rules(DdType::StaticEquity, dec!(1000));
        let res = evaluate(&r, &ctx(dec!(9500)));
        assert_eq!(res.f_prop, dec!(9000));
        assert!(!res.breached);
        assert!(evaluate(&r, &ctx(dec!(8900))).breached);
    }

    #[test]
    fn trailing_equity_uses_peak() {
        let r = rules(DdType::TrailingEquity, dec!(1000));
        let res = evaluate(&r, &ctx(dec!(11500)));
        assert_eq!(res.f_prop, dec!(11000));
        assert!(!res.breached);
        assert!(evaluate(&r, &ctx(dec!(10900))).breached);
    }

    #[test]
    fn static_balance_uses_balance() {
        let r = rules(DdType::StaticBalance, dec!(1000));
        assert_eq!(evaluate(&r, &ctx(dec!(9500))).f_prop, dec!(9000));
    }

    #[test]
    fn trailing_balance_uses_peak_balance() {
        let r = rules(DdType::TrailingBalance, dec!(1000));
        assert_eq!(evaluate(&r, &ctx(dec!(11500))).f_prop, dec!(10000));
    }
}
