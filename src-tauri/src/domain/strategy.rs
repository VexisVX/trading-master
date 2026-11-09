use serde::{Deserialize, Serialize};

/// Allowed field types in MVP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    Text,
    Textarea,
    Select,
}

/// A single configurable field within a timeframe.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub kind: FieldType,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

/// A timeframe block within a strategy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategyTimeframe {
    pub timeframe: String, // "4H", "1H", "15M", ...
    pub fields: Vec<StrategyField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Strategy {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub version: i64,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
    pub timeframes: Vec<StrategyTimeframe>,
}

/// Input payload from the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NewStrategy {
    pub name: String,
    pub description: Option<String>,
    pub timeframes: Vec<StrategyTimeframe>,
}
