//! Instrument specification.
//!
//! All fields come from verified broker specs or user-entered values.
//! `Estimated` specs MUST NOT be used for live sizing.

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpecSource {
    Verified,
    UserEntered,
    Estimated,
}

impl Default for SpecSource {
    fn default() -> Self {
        SpecSource::UserEntered
    }
}

impl SpecSource {
    pub fn is_safe_for_sizing(self) -> bool {
        matches!(self, SpecSource::Verified | SpecSource::UserEntered)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            SpecSource::Verified => "verified",
            SpecSource::UserEntered => "user_entered",
            SpecSource::Estimated => "estimated",
        }
    }
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "verified" => Ok(SpecSource::Verified),
            "user_entered" => Ok(SpecSource::UserEntered),
            "estimated" => Ok(SpecSource::Estimated),
            other => Err(format!("unknown spec_source: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentSpec {
    pub symbol: String,
    pub currency: String,

    // --- pricing ---
    pub tick_size: Decimal,
    pub tick_value: Decimal,
    pub contract_size: Decimal,

    // --- quantity ---
    pub min_qty: Decimal,
    pub max_qty: Decimal,
    pub lot_step: Decimal,

    // --- costs ---
    pub commission_per_lot: Decimal,
    pub slippage_per_unit: Decimal,

    // --- provenance ---
    pub spec_source: SpecSource,
}

impl InstrumentSpec {
    /// Returns Err if spec is not safe for live sizing.
    pub fn validate_for_sizing(&self) -> Result<(), String> {
        if !self.spec_source.is_safe_for_sizing() {
            return Err(format!(
                "instrument '{}' has spec_source={:?} — not safe for live sizing",
                self.symbol, self.spec_source
            ));
        }
        if self.tick_size <= Decimal::ZERO {
            return Err(format!("'{}' tick_size must be > 0", self.symbol));
        }
        if self.tick_value <= Decimal::ZERO {
            return Err(format!("'{}' tick_value must be > 0", self.symbol));
        }
        if self.contract_size <= Decimal::ZERO {
            return Err(format!("'{}' contract_size must be > 0", self.symbol));
        }
        if self.lot_step <= Decimal::ZERO {
            return Err(format!("'{}' lot_step must be > 0", self.symbol));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn spec(source: SpecSource) -> InstrumentSpec {
        InstrumentSpec {
            symbol: "EURUSD".into(),
            currency: "USD".into(),
            tick_size: dec!(0.00001),
            tick_value: dec!(1),
            contract_size: dec!(100000),
            min_qty: dec!(0.01),
            max_qty: dec!(100),
            lot_step: dec!(0.01),
            commission_per_lot: dec!(0),
            slippage_per_unit: dec!(0),
            spec_source: source,
        }
    }

    #[test]
    fn verified_ok() {
        assert!(spec(SpecSource::Verified).validate_for_sizing().is_ok());
    }

    #[test]
    fn user_entered_ok() {
        assert!(spec(SpecSource::UserEntered).validate_for_sizing().is_ok());
    }

    #[test]
    fn estimated_rejected() {
        assert!(spec(SpecSource::Estimated).validate_for_sizing().is_err());
    }

    #[test]
    fn zero_tick_rejected() {
        let mut s = spec(SpecSource::Verified);
        s.tick_size = Decimal::ZERO;
        assert!(s.validate_for_sizing().is_err());
    }
}
