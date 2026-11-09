use crate::commands::AppState;
use crate::db::app_settings as repo;
use tauri::State;

#[tauri::command]
pub fn settings_get(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_setting(&conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_set(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn settings_delete(state: State<AppState>, key: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_setting(&conn, &key).map_err(|e| e.to_string())
}
