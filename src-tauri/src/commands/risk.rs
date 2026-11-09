//! Tauri commands for the risk engine.
//! All Decimal values cross the boundary as strings (serde-with-str).

use crate::risk::{compute_position_size, compute_r, PositionSizeInput, TradeR};
use rust_decimal::Decimal;

#[tauri::command]
pub fn risk_position_size(input: PositionSizeInput) -> Result<String, String> {
    compute_position_size(&input)
        .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "{}".into()))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn risk_r_multiple(trade: TradeR) -> Result<String, String> {
    compute_r(&trade)
        .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "{}".into()))
        .map_err(|e| e.to_string())
}

/// Helper: parse a decimal string in the frontend payload.
/// Used for smoke testing that strings survive the round trip.
#[tauri::command]
pub fn risk_echo_decimal(s: String) -> Result<String, String> {
    super::super::risk::parse_decimal(&s)
        .map(|d: Decimal| d.to_string())
        .map_err(|e| e.to_string())
}
