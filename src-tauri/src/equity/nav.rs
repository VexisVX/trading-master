//! NAV / Units.
//!
//! NAV = Equity / Units. Deposits/withdrawals change Units, not NAV.
//! This prevents cash-flows from being mistaken for performance.
//!
//! All math is Decimal. No floats.

use rust_decimal::Decimal;

/// Initial state: Units = E0, NAV = 1.
#[derive(Debug, Clone)]
pub struct NavState {
    pub units: Decimal,
    pub nav: Decimal,
}

impl NavState {
    pub fn initial(equity: Decimal) -> Result<Self, String> {
        if equity <= Decimal::ZERO {
            return Err("initial equity must be > 0".into());
        }
        Ok(Self {
            units: equity,
            nav: Decimal::ONE,
        })
    }

    /// Recompute NAV = Equity / Units.
    pub fn nav_for(&self, equity: Decimal) -> Result<Decimal, String> {
        if self.units.is_zero() {
            return Err("units is zero — cannot compute NAV".into());
        }
        Ok(equity / self.units)
    }

    /// Deposit: Units_new = Units_old + Deposit / NAV_before.
    pub fn deposit(&self, amount: Decimal) -> Result<(Self, Decimal), String> {
        if amount <= Decimal::ZERO {
            return Err("deposit must be > 0".into());
        }
        let nav_before = self.nav;
        if nav_before.is_zero() {
            return Err("NAV before deposit is zero".into());
        }
        let added_units = amount / nav_before;
        Ok((
            Self {
                units: self.units + added_units,
                nav: nav_before,
            },
            added_units,
        ))
    }

    /// Withdrawal: Units_new = Units_old - Withdrawal / NAV_before.
    /// Reject if it would make units <= 0.
    pub fn withdraw(&self, amount: Decimal) -> Result<(Self, Decimal), String> {
        if amount <= Decimal::ZERO {
            return Err("withdrawal must be > 0".into());
        }
        let nav_before = self.nav;
        if nav_before.is_zero() {
            return Err("NAV before withdrawal is zero".into());
        }
        let removed_units = amount / nav_before;
        let new_units = self.units - removed_units;
        if new_units <= Decimal::ZERO {
            return Err("withdrawal would zero out units".into());
        }
        Ok((
            Self {
                units: new_units,
                nav: nav_before,
            },
            removed_units,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn initial_nav_is_one() {
        let n = NavState::initial(dec!(10000)).unwrap();
        assert_eq!(n.units, dec!(10000));
        assert_eq!(n.nav, Decimal::ONE);
    }

    #[test]
    fn deposit_preserves_nav() {
        let n = NavState::initial(dec!(10000)).unwrap();
        let (n2, added) = n.deposit(dec!(1000)).unwrap();
        assert_eq!(added, dec!(1000));
        assert_eq!(n2.units, dec!(11000));
        // NAV = 11000 / 11000 = 1
        assert_eq!(n2.nav_for(dec!(11000)).unwrap(), Decimal::ONE);
    }

    #[test]
    fn deposit_after_growth() {
        // Start 10000, grow to 12000 (NAV = 1.2), then deposit 1200.
        let n = NavState::initial(dec!(10000)).unwrap();
        let nav_after_growth = n.nav_for(dec!(12000)).unwrap(); // 1.2
        let n2 = NavState {
            units: n.units,
            nav: nav_after_growth,
        };
        let (n3, added) = n2.deposit(dec!(1200)).unwrap();
        // added = 1200 / 1.2 = 1000 units
        assert_eq!(added, dec!(1000));
        assert_eq!(n3.units, dec!(11000));
        // NAV = (12000+1200) / 11000 = 1.2
        assert_eq!(n3.nav_for(dec!(13200)).unwrap(), dec!(1.2));
    }

    #[test]
    fn withdraw_rejects_empty() {
        let n = NavState::initial(dec!(10000)).unwrap();
        assert!(n.withdraw(dec!(10001)).is_err());
    }

    #[test]
    fn deposit_rejects_zero() {
        let n = NavState::initial(dec!(10000)).unwrap();
        assert!(n.deposit(Decimal::ZERO).is_err());
        assert!(n.deposit(dec!(-100)).is_err());
    }
}
