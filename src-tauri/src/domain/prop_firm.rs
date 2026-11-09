use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Prop-firm rule configuration, per account.
/// All percentages are decimals (e.g. "10" = 10%).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PropFirmRules {
    pub account_id: i64,
    pub enabled: bool,
    pub max_dd_pct: Decimal,
    pub trailing_dd: bool,
    pub daily_loss_limit_pct: Decimal,
    pub max_total_loss_pct: Decimal,
    pub profit_target_pct: Decimal,
    pub min_trading_days: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdatePropFirmRules {
    pub enabled: bool,
    pub max_dd_pct: Decimal,
    pub trailing_dd: bool,
    pub daily_loss_limit_pct: Decimal,
    pub max_total_loss_pct: Decimal,
    pub profit_target_pct: Decimal,
    pub min_trading_days: i64,
}

/// One rule's evaluation state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleCheck {
    pub name: String,
    pub passed: bool,     // true = rule is met, false = breached
    pub current: Decimal, // current value (e.g. 4.2% drawdown)
    pub limit: Decimal,   // the limit (e.g. 10%)
    pub message: String,
}

/// Full evaluation against prop-firm rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PropFirmStatus {
    pub rules: PropFirmRules,
    pub initial_balance: Decimal,
    pub peak_equity: Decimal,
    pub current_equity: Decimal,
    pub total_pnl: Decimal,
    pub trading_days: i64,
    pub checks: Vec<RuleCheck>,
    pub all_passed: bool,
    pub warnings: Vec<String>,
}
