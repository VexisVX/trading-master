//! Adaptive profile: derive internal protection presets from prop target & DD.
//!
//! IMPORTANT: The ratios below are POLICY HEURISTICS, not mathematically
//! proven optimal parameters. They are internal protection presets, NOT
//! actual prop-firm rules. Actual prop rules live in `prop_firm_rule_sets`
//! and are enforced independently.
//!
//! Multi-account safe: each account has its own profile in `account_profiles`.

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Personal,
    PropFirm,
}

impl AccountType {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountType::Personal => "personal",
            AccountType::PropFirm => "prop_firm",
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "personal" => Ok(AccountType::Personal),
            "prop_firm" => Ok(AccountType::PropFirm),
            other => Err(format!("unknown account_type: '{other}'")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProfileInput {
    pub account_type: AccountType,
    pub profit_target_pct: Decimal,
    pub max_dd_pct: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileResult {
    pub profit_protect_activation: Decimal,
    pub profit_protect_full: Decimal,
    pub max_profit_retention: Decimal,
    pub trailing_dd_pct: Decimal,
}

const PERSONAL_ACTIVATION: Decimal = Decimal::from_parts(5, 0, 0, false, 2);
const PERSONAL_FULL: Decimal = Decimal::from_parts(10, 0, 0, false, 2);
const PERSONAL_RETENTION: Decimal = Decimal::from_parts(40, 0, 0, false, 2);
const PERSONAL_TRAILING: Decimal = Decimal::from_parts(10, 0, 0, false, 2);

pub fn derive(i: &ProfileInput) -> Result<ProfileResult, String> {
    match i.account_type {
        AccountType::Personal => Ok(ProfileResult {
            profit_protect_activation: PERSONAL_ACTIVATION,
            profit_protect_full: PERSONAL_FULL,
            max_profit_retention: PERSONAL_RETENTION,
            trailing_dd_pct: PERSONAL_TRAILING,
        }),
        AccountType::PropFirm => {
            if i.profit_target_pct <= Decimal::ZERO {
                return Err(format!("profit_target_pct must be > 0 (got {})", i.profit_target_pct));
            }
            if i.max_dd_pct <= Decimal::ZERO {
                return Err(format!("max_dd_pct must be > 0 (got {})", i.max_dd_pct));
            }
            if i.profit_target_pct > Decimal::new(50, 2) {
                return Err(format!("profit_target_pct > 0.50 unrealistic (got {})", i.profit_target_pct));
            }
            if i.max_dd_pct > Decimal::new(20, 2) {
                return Err(format!("max_dd_pct > 0.20 unrealistic (got {})", i.max_dd_pct));
            }
            if i.max_dd_pct >= i.profit_target_pct {
                return Err(format!("max_dd ({}) must be < target ({})", i.max_dd_pct, i.profit_target_pct));
            }

            let activation = i.profit_target_pct * Decimal::new(20, 2);
            let full = i.profit_target_pct * Decimal::new(60, 2);
            let retention_raw = i.max_dd_pct * Decimal::from(8);
            let retention = if retention_raw > Decimal::new(60, 2) { Decimal::new(60, 2) } else { retention_raw };
            let trailing = i.max_dd_pct * Decimal::new(60, 2);

            if activation >= full {
                return Err(format!("activation ({activation}) >= full ({full})"));
            }

            Ok(ProfileResult {
                profit_protect_activation: activation,
                profit_protect_full: full,
                max_profit_retention: retention,
                trailing_dd_pct: trailing,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn personal_defaults() {
        let r = derive(&ProfileInput { account_type: AccountType::Personal, profit_target_pct: dec!(0.08), max_dd_pct: dec!(0.05) }).unwrap();
        assert_eq!(r.profit_protect_activation, dec!(0.05));
        assert_eq!(r.max_profit_retention, dec!(0.40));
    }

    #[test]
    fn prop_10_target_5_dd() {
        let r = derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.10), max_dd_pct: dec!(0.05) }).unwrap();
        assert_eq!(r.profit_protect_activation, dec!(0.02));
        assert_eq!(r.profit_protect_full, dec!(0.06));
        assert_eq!(r.max_profit_retention, dec!(0.40));
        assert_eq!(r.trailing_dd_pct, dec!(0.03));
    }

    #[test]
    fn prop_8_target_4_dd() {
        let r = derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.08), max_dd_pct: dec!(0.04) }).unwrap();
        assert_eq!(r.profit_protect_activation, dec!(0.016));
        assert_eq!(r.max_profit_retention, dec!(0.32));
    }

    #[test]
    fn retention_capped() {
        let r = derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.20), max_dd_pct: dec!(0.10) }).unwrap();
        assert_eq!(r.max_profit_retention, dec!(0.60));
    }

    #[test]
    fn invalid_account_type() {
        assert!(AccountType::parse("unknown").is_err());
        assert!(AccountType::parse("").is_err());
        assert!(AccountType::parse("PROP_FIRM").is_err());
        assert!(AccountType::parse("prop_firm").is_ok());
    }

    #[test]
    fn zero_target() {
        assert!(derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0), max_dd_pct: dec!(0.05) }).is_err());
    }

    #[test]
    fn zero_dd() {
        assert!(derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.10), max_dd_pct: dec!(0) }).is_err());
    }

    #[test]
    fn negative_input() {
        assert!(derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(-0.10), max_dd_pct: dec!(0.05) }).is_err());
    }

    #[test]
    fn out_of_range() {
        assert!(derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.60), max_dd_pct: dec!(0.05) }).is_err());
    }

    #[test]
    fn dd_must_be_less_than_target() {
        assert!(derive(&ProfileInput { account_type: AccountType::PropFirm, profit_target_pct: dec!(0.05), max_dd_pct: dec!(0.10) }).is_err());
    }
}
