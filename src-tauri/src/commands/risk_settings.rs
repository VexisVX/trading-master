use crate::commands::AppState;
use crate::db::{risk_settings as rs, trades as trades_db};
use crate::domain::risk_settings::{RiskSettings, UpdateRiskSettings};
use crate::risk::compute_risk_status;
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn risk_settings_get(
    state: State<AppState>,
    account_id: i64,
) -> Result<Option<RiskSettings>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    rs::get_settings(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn risk_settings_update(
    state: State<AppState>,
    account_id: i64,
    update: UpdateRiskSettings,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    rs::update_settings(&conn, account_id, &update).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn risk_status(state: State<AppState>, account_id: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let settings = rs::get_settings(&conn, account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no risk settings for account".to_string())?;

    let balance_str: String = conn
        .query_row(
            "SELECT initial_balance FROM accounts WHERE id=?1",
            [account_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let initial = Decimal::from_str(&balance_str).map_err(|e| e.to_string())?;

    let trades = trades_db::list_trades(&conn, None, 500, 0).map_err(|e| e.to_string())?;

    // Current equity = initial balance + realized P&L of all closed trades.
    // (Open risk is not subtracted — it's potential, not realized.)
    let mut realized = Decimal::ZERO;
    for t in &trades {
        if t.status == crate::domain::trade::TradeStatus::Closed {
            realized += crate::risk::trade_realized_pnl(t);
        }
    }
    let equity = initial + realized;

    let today_prefix = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let status = compute_risk_status(&settings, equity, &trades, &today_prefix);

    serde_json::to_string(&status).map_err(|e| e.to_string())
}
