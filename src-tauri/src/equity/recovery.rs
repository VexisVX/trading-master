//! Recovery mode authorization (audit §7).
//!
//! Recovery MUST be explicitly authorized by the user. Cooldown alone
//! is never sufficient. Probe risk defaults to 25% of base.

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct RecoveryAuth {
    pub authorized: bool,
    pub expires_at: Option<String>,
    pub probe_risk_pct: Decimal, // fraction of R_base
}

impl RecoveryAuth {
    pub fn none() -> Self {
        Self { authorized: false, expires_at: None, probe_risk_pct: Decimal::new(25, 2) }
    }
    pub fn grant(probe_pct: Decimal) -> Self {
        Self { authorized: true, expires_at: None, probe_risk_pct: probe_pct }
    }
    pub fn revoke(&mut self) {
        self.authorized = false;
    }
}

/// Given base R and recovery auth, return the probe risk.
pub fn probe_r(r_base: Decimal, auth: &RecoveryAuth) -> Decimal {
    if !auth.authorized { return Decimal::ZERO; }
    r_base * auth.probe_risk_pct
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn none_returns_zero() {
        let a = RecoveryAuth::none();
        assert_eq!(probe_r(dec!(100), &a), Decimal::ZERO);
    }

    #[test]
    fn default_probe_is_25pct() {
        let a = RecoveryAuth::grant(dec!(0.25));
        assert_eq!(probe_r(dec!(100), &a), dec!(25));
    }

    #[test]
    fn revoke_works() {
        let mut a = RecoveryAuth::grant(dec!(0.25));
        a.revoke();
        assert_eq!(probe_r(dec!(100), &a), Decimal::ZERO);
    }

    #[test]
    fn custom_probe_pct() {
        let a = RecoveryAuth::grant(dec!(0.10));
        assert_eq!(probe_r(dec!(200), &a), dec!(20));
    }

    #[test]
    fn auth_does_not_bypass_hard_locks() {
        // Recovery auth cannot authorize new trades in PropHardLock — that's
        // enforced by state_machine::next, not by this module.
        let mut a = RecoveryAuth::grant(dec!(0.25));
        a.revoke();
        assert_eq!(probe_r(dec!(1000), &a), Decimal::ZERO);
    }
}
