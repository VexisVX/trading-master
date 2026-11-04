use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum EquityError {
    InvalidInput(String),
    DecimalParse(String),
    ConfigMissing(String),
    StateInvalid(String),
}

impl std::fmt::Display for EquityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EquityError::InvalidInput(s) => write!(f, "invalid input: {s}"),
            EquityError::DecimalParse(s) => write!(f, "decimal parse: {s}"),
            EquityError::ConfigMissing(s) => write!(f, "config missing: {s}"),
            EquityError::StateInvalid(s) => write!(f, "state invalid: {s}"),
        }
    }
}
impl std::error::Error for EquityError {}

/// Structured reason for a rejection. Stored in audit log.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum DecisionReason {
    Allowed,
    AccountDisabled,
    AccountPaused,
    EquityBelowFloor {
        equity: String,
        floor: String,
    },
    DailyLossExceeded {
        used: String,
        limit: String,
    },
    PortfolioRiskFull {
        used: String,
        max: String,
    },
    FloorRiskCapacityZero,
    AdaptiveRiskZero,
    RAllowedZeroOrNegative,
    SnapshotTooOld {
        age_hours: i64,
        max_hours: i64,
    },
    InvalidConfig(String),
    InstrumentNotFound {
        symbol: String,
    },
    BelowMinimumQuantity {
        min_qty: String,
        allowed_risk: String,
        actual_risk: String,
    },
    MarketDataIncomplete {
        reason: String,
    },
}

/// PHASE 2.2 — daily P&L computation errors.
///
/// These are DATA-QUALITY blocking conditions, not state-machine transitions.
/// The caller (auto_snapshot / engine) should reject the decision with
/// `DecisionReason::MarketDataIncomplete` and NOT change account state.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum DailyPnLError {
    /// current_price is missing for an open trade — floating P&L unknown.
    FloatingPnlUnknown { trade_id: i64, symbol: String },
    /// current_price is older than max_price_age_minutes.
    MarketDataStale {
        trade_id: i64,
        symbol: String,
        age_minutes: i64,
    },
    /// Instrument spec missing from DB — cannot convert to account currency.
    InstrumentMissing { symbol: String },
    /// Instrument spec has estimated source — prohibited for risk.
    InstrumentUnsafe { symbol: String, source: String },
    /// account_currency vs instrument currency mismatch, no rate available.
    CurrencyConversionMissing { from: String, to: String },
    /// Generic computation error.
    ComputeError(String),
}

impl std::fmt::Display for DailyPnLError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DailyPnLError::FloatingPnlUnknown { trade_id, symbol } => write!(
                f,
                "trade {} {}: current_price missing (floating P&L unknown)",
                trade_id, symbol
            ),
            DailyPnLError::MarketDataStale {
                trade_id,
                symbol,
                age_minutes,
            } => write!(
                f,
                "trade {} {}: price stale ({} min old)",
                trade_id, symbol, age_minutes
            ),
            DailyPnLError::InstrumentMissing { symbol } => write!(
                f,
                "instrument {} not found — cannot compute floating P&L",
                symbol
            ),
            DailyPnLError::InstrumentUnsafe { symbol, source } => write!(
                f,
                "instrument {} has spec_source={} — prohibited",
                symbol, source
            ),
            DailyPnLError::CurrencyConversionMissing { from, to } => {
                write!(f, "currency conversion {}→{} unavailable", from, to)
            }
            DailyPnLError::ComputeError(s) => write!(f, "compute error: {}", s),
        }
    }
}

impl std::error::Error for DailyPnLError {}
