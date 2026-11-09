pub mod accounts;
pub mod analytics;
pub mod backtest;
pub mod backup;
pub mod equity;
pub mod goals;
pub mod live_calc;

pub mod live_presets;
pub mod news;
pub mod prop_firm;
pub mod risk;
pub mod risk_settings;
pub mod settings;
pub mod storage;
pub mod strategies;
pub mod trades;
pub mod trading_plans;

use crate::db;
use rusqlite::Connection;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub db: Mutex<Connection>,
    pub backtest_db: Mutex<Connection>,
}

#[tauri::command]
pub fn get_schema_version(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    db::get_meta(&conn, "schema_version")
        .map_err(|e| e.to_string())
        .map(|v| v.unwrap_or_else(|| "unknown".into()))
}

#[tauri::command]
pub fn get_balance_placeholder() -> String {
    "—".to_string()
}
