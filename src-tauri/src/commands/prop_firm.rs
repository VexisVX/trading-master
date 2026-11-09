use crate::commands::AppState;
use crate::db::{accounts as accounts_db, prop_firm as repo, trades as trades_db};
use crate::domain::prop_firm::{PropFirmRules, UpdatePropFirmRules};
use crate::risk::evaluate_prop_firm;
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn prop_firm_get(
    state: State<AppState>,
    account_id: i64,
) -> Result<Option<PropFirmRules>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let acct = accounts_db::get_account(&conn, account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "account not found".to_string())?;
    if !acct.kind.is_prop_firm() {
        return Ok(None);
    }
    repo::get_rules(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn prop_firm_update(
    state: State<AppState>,
    account_id: i64,
    update: UpdatePropFirmRules,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let acct = accounts_db::get_account(&conn, account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "account not found".to_string())?;
    if !acct.kind.is_prop_firm() {
        return Err("prop-firm rules only apply to prop-firm accounts".into());
    }
    repo::update_rules(&conn, account_id, &update).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn prop_firm_status(state: State<AppState>, account_id: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let acct = accounts_db::get_account(&conn, account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "account not found".to_string())?;
    if !acct.kind.is_prop_firm() {
        return Err("not a prop-firm account".into());
    }

    let rules = repo::get_rules(&conn, account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no prop-firm rules for account".to_string())?;

    let initial_str: String = conn
        .query_row(
            "SELECT initial_balance FROM accounts WHERE id=?1",
            [account_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let initial = Decimal::from_str(&initial_str).map_err(|e| e.to_string())?;

    let trades = trades_db::list_trades(&conn, None, 500, 0).map_err(|e| e.to_string())?;

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let status = evaluate_prop_firm(&rules, initial, &trades, &today);

    serde_json::to_string(&status).map_err(|e| e.to_string())
}
