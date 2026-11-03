use crate::commands::AppState;
use crate::db::goals as repo;
use tauri::State;

#[tauri::command]
pub fn goals_get(
    state: State<AppState>,
    account_id: i64,
) -> Result<Option<repo::AccountGoals>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_goals(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn goals_set(
    state: State<AppState>,
    account_id: i64,
    goals: repo::UpdateGoals,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::set_goals(&conn, account_id, &goals).map_err(|e| e.to_string())
}
