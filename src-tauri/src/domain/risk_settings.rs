use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// User-editable risk configuration, per account.
/// All percentages are stored as decimals (e.g. "1" = 1%, "0.5" = 0.5%).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskSettings {
    pub account_id: i64,
    pub max_risk_per_trade_pct: Decimal,
    pub daily_loss_limit_pct: Decimal,
    pub max_open_positions: i64,
    pub max_portfolio_risk_pct: Decimal,
    pub updated_at: String,
}

/// Input from the frontend to update risk settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateRiskSettings {
    pub max_risk_per_trade_pct: Decimal,
    pub daily_loss_limit_pct: Decimal,
    pub max_open_positions: i64,
    pub max_portfolio_risk_pct: Decimal,
}

/// Computed snapshot of risk status for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RiskStatus {
    pub settings: RiskSettings,

    // Account state
    pub current_equity: Decimal,
    pub open_positions: i64,
    pub open_risk: Decimal,
    pub open_risk_pct: Decimal,
    pub daily_loss_today: Decimal,
    pub daily_loss_pct: Decimal,

    // Budget utilization
    pub risk_budget_used_pct: Decimal,
    pub daily_loss_budget_used_pct: Decimal,

    // Boolean flags for UI coloring
    pub max_positions_hit: bool,
    pub daily_loss_limit_hit: bool,
    pub portfolio_risk_exceeded: bool,

    // Human-readable warnings
    pub warnings: Vec<String>,
}
