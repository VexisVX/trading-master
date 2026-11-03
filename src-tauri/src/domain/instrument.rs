//! Instrument specification.
//!
//! All fields come from verified exchange/broker specs, user input, or
//! estimation. `spec_source` records which, so downstream code can decide
//! whether to trust the spec for live sizing.
//!
//! Never invent missing fields. If a field is unknown, the caller must
//! reject the trade (see `risk/position.rs`).

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
    fn default() -> Self { SpecSource::UserEntered }
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
