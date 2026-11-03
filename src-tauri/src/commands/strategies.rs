use crate::commands::AppState;
use crate::db::strategies as repo;
use crate::domain::strategy::{NewStrategy, Strategy};
use tauri::State;

#[tauri::command]
pub fn strategies_create(state: State<AppState>, strategy: NewStrategy) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_strategy(&conn, &strategy).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_list(state: State<AppState>) -> Result<Vec<Strategy>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_strategies(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_get(state: State<AppState>, id: i64) -> Result<Option<Strategy>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_strategy(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_strategy(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_update(state: State<AppState>, id: i64, strategy: NewStrategy) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::update_strategy(&conn, id, &strategy).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_list_versions(
    state: State<AppState>, strategy_id: i64,
) -> Result<Vec<(i64, i64, String, Option<String>, String)>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_versions(&conn, strategy_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn strategies_get_version(
    state: State<AppState>, version_id: i64,
) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_version_json(&conn, version_id).map_err(|e| e.to_string())
}
