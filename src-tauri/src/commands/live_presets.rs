use crate::commands::AppState;
use crate::db::live_presets as repo;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetPayload {
    pub name: String,
    pub instrument: Option<String>,
    pub payload_json: String,
}

#[tauri::command]
pub fn presets_list(
    state: State<AppState>,
    account_id: i64,
) -> Result<Vec<repo::LivePreset>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_presets(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn presets_create(
    state: State<AppState>,
    account_id: i64,
    preset: PresetPayload,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_preset(
        &conn,
        account_id,
        &preset.name,
        preset.instrument.as_deref(),
        &preset.payload_json,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn presets_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_preset(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn drafts_create(
    state: State<AppState>,
    account_id: i64,
    payload_json: String,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_draft(&conn, account_id, &payload_json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn drafts_get(state: State<AppState>, id: i64) -> Result<Option<repo::TradeDraft>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_draft(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn drafts_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_draft(&conn, id).map_err(|e| e.to_string())
}
