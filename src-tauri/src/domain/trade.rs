use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Side { Long, Short }

impl Side {
    pub fn as_str(&self) -> &'static str {
        match self { Side::Long => "long", Side::Short => "short" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "long" => Some(Side::Long), "short" => Some(Side::Short), _ => None }
    }
    pub fn is_long(&self) -> bool { matches!(self, Side::Long) }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TradeStatus { Open, Closed }

impl TradeStatus {
    pub fn as_str(&self) -> &'static str {
        match self { TradeStatus::Open => "open", TradeStatus::Closed => "closed" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "open" => Some(TradeStatus::Open), "closed" => Some(TradeStatus::Closed), _ => None }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TradeOutcome { Open, TpHit, SlHit, ManualClose, Breakeven }

impl TradeOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            TradeOutcome::Open => "open",
            TradeOutcome::TpHit => "tp_hit",
            TradeOutcome::SlHit => "sl_hit",
            TradeOutcome::ManualClose => "manual_close",
            TradeOutcome::Breakeven => "breakeven",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(TradeOutcome::Open),
            "tp_hit" => Some(TradeOutcome::TpHit),
            "sl_hit" => Some(TradeOutcome::SlHit),
            "manual_close" => Some(TradeOutcome::ManualClose),
            "breakeven" => Some(TradeOutcome::Breakeven),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SizeType { Lots, Units, Money }

impl SizeType {
    pub fn as_str(&self) -> &'static str {
        match self { SizeType::Lots => "lots", SizeType::Units => "units", SizeType::Money => "money" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "lots" => Some(SizeType::Lots), "units" => Some(SizeType::Units), "money" => Some(SizeType::Money), _ => None }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LegKind { Entry, Exit }

impl LegKind {
    pub fn as_str(&self) -> &'static str {
        match self { LegKind::Entry => "entry", LegKind::Exit => "exit" }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s { "entry" => Some(LegKind::Entry), "exit" => Some(LegKind::Exit), _ => None }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attachment {
    pub id: i64,
    pub filename: String,
    pub kind: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TradeLeg {
    pub id: i64,
    pub kind: LegKind,
    pub price: Decimal,
    pub quantity: Decimal,
    pub executed_at: String,
    pub cost_per_unit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Trade {
    pub id: i64,
    pub account_id: i64,
    pub symbol: String,
    pub side: Side,
    pub entry_price: Decimal,
    pub initial_stop: Decimal,
    pub take_profit: Option<Decimal>,
    pub total_quantity: Decimal,
    pub original_quantity: Decimal,
    pub outcome: TradeOutcome,
    pub size_type: SizeType,
    pub size_value: Option<Decimal>,
    pub size_unit: Option<String>,
    pub manual_pnl: Option<Decimal>,

    // PHASE 2.2: live market data
    /// Current market price for open trades (manual entry only in this phase).
    pub current_price: Option<Decimal>,
    /// When current_price was last updated (UTC, ISO8601).
    pub current_price_updated_at: Option<String>,
    /// Provenance: "manual" | "broker". Estimated is prohibited.
    pub current_price_source: Option<String>,
    /// Trailing/adjusted stop. Falls back to initial_stop when None.
    pub current_stop: Option<Decimal>,
    pub status: TradeStatus,
    pub strategy_id: Option<i64>,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
    pub screenshot_path: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub legs: Vec<TradeLeg>,
    pub tags: Vec<String>,
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTrade {
    pub account_id: i64,
    pub symbol: String,
    pub side: Side,
    pub entry_price: Decimal,
    pub initial_stop: Decimal,
    pub take_profit: Option<Decimal>,
    pub total_quantity: Decimal,
    pub size_type: SizeType,
    pub size_value: Option<Decimal>,
    pub size_unit: Option<String>,
    pub opened_at: String,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
    pub screenshot_path: Option<String>,
    pub strategy_id: Option<i64>,
    pub entry_legs: Vec<NewLeg>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewLeg {
    pub price: Decimal,
    pub quantity: Decimal,
    pub cost_per_unit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTrade {
    pub symbol: String,
    pub entry_price: Decimal,
    pub initial_stop: Decimal,
    pub take_profit: Option<Decimal>,
    pub total_quantity: Decimal,
    pub outcome: TradeOutcome,
    pub size_type: SizeType,
    pub size_value: Option<Decimal>,
    pub size_unit: Option<String>,
    pub manual_pnl: Option<Decimal>,
    pub strategy_id: Option<i64>,
    pub notes: Option<String>,
    pub chart_url: Option<String>,
    pub screenshot_path: Option<String>,
    pub tags: Vec<String>,
}

/// Test helper: build a minimal Trade with all fields set to safe defaults.
/// Never use this in production code — only in tests.
#[cfg(test)]
pub fn empty_trade() -> Trade {
    use rust_decimal::Decimal;
    Trade {
        id: 0,
        account_id: 1,
        symbol: "TEST".into(),
        side: Side::Long,
        entry_price: Decimal::ZERO,
        initial_stop: Decimal::ZERO,
        take_profit: None,
        total_quantity: Decimal::ZERO,
        original_quantity: Decimal::ZERO,
        outcome: TradeOutcome::Open,
        size_type: SizeType::Lots,
        size_value: None,
        size_unit: None,
        manual_pnl: None,
        status: TradeStatus::Open,
        strategy_id: None,
        opened_at: "2026-01-01T00:00:00Z".into(),
        closed_at: None,
        notes: None,
        chart_url: None,
        screenshot_path: None,
        created_at: "2026-01-01T00:00:00Z".into(),
        updated_at: "2026-01-01T00:00:00Z".into(),
        legs: vec![],
        tags: vec![],
        attachments: vec![],
        current_price: None,
        current_price_updated_at: None,
        current_price_source: None,
        current_stop: None,
    }}


// ============================================================
// PHASE 2.2 — active stop + price source validation
// ============================================================

impl Trade {
    /// Active stop for downside calculation.
    /// Falls back to initial_stop when current_stop is None.
    pub fn active_stop(&self) -> Decimal {
        self.current_stop.unwrap_or(self.initial_stop)
    }

    /// True if a current price has been set.
    pub fn has_current_price(&self) -> bool {
        self.current_price.is_some()
    }

    /// Validate current price for this trade (must be > 0).
    pub fn validate_current_price(&self) -> Result<(), String> {
        if let Some(p) = self.current_price {
            if p <= Decimal::ZERO {
                return Err("current_price must be > 0".into());
            }
        }
        Ok(())
    }

    /// Validate current_stop direction relative to entry.
    /// Long: stop <= entry. Short: stop >= entry.
    pub fn validate_current_stop(&self) -> Result<(), String> {
        let stop = self.active_stop();
        match self.side {
            Side::Long if stop > self.entry_price => {
                Err(format!("long stop {} > entry {}", stop, self.entry_price))
            }
            Side::Short if stop < self.entry_price => {
                Err(format!("short stop {} < entry {}", stop, self.entry_price))
            }
            _ => Ok(()),
        }
    }

    /// True if current_price is stale (older than `max_age_minutes`).
    pub fn is_price_stale(&self, max_age_minutes: i64) -> bool {
        let Some(ts) = &self.current_price_updated_at else {
            return true; // no price = stale
        };
        let Ok(when) = chrono::DateTime::parse_from_rfc3339(ts) else {
            return true;
        };
        let age = chrono::Utc::now().signed_duration_since(when.with_timezone(&chrono::Utc));
        age.num_minutes() > max_age_minutes
    }
}
