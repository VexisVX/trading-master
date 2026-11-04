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
    EquityBelowFloor { equity: String, floor: String },
    DailyLossExceeded { used: String, limit: String },
    PortfolioRiskFull { used: String, max: String },
    FloorRiskCapacityZero,
    AdaptiveRiskZero,
    RAllowedZeroOrNegative,
    SnapshotTooOld { age_hours: i64, max_hours: i64 },
    InvalidConfig(String),
    InstrumentNotFound { symbol: String },
    BelowMinimumQuantity { min_qty: String, allowed_risk: String, actual_risk: String },
}
