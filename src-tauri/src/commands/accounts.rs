use crate::commands::AppState;
use crate::db::accounts as repo;
use crate::db::get_meta;
use crate::domain::account::{Account, NewAccount};
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn accounts_list(state: State<AppState>) -> Result<Vec<Account>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_accounts(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_create(state: State<AppState>, account: NewAccount) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_account(&conn, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_account(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_active_get(state: State<AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(v) = get_meta(&conn, "active_account_id").map_err(|e| e.to_string())? {
        if let Ok(id) = v.parse::<i64>() {
            return Ok(id);
        }
    }
    let id: i64 = conn
        .query_row("SELECT id FROM accounts ORDER BY id LIMIT 1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn accounts_active_set(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts WHERE id=?1", [id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if exists == 0 { return Err("account not found".into()); }
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('active_account_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![id.to_string()],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
