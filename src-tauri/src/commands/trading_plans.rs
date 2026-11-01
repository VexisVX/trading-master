use crate::commands::AppState;
use crate::db::trading_plans as repo;
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
pub struct PlanInput {
    pub strategy_id: i64,
    pub name: String,
    pub checklist_json: String,
    pub notes: Option<String>,
}

#[tauri::command]
pub fn plans_list(state: State<AppState>, strategy_id: i64) -> Result<Vec<repo::TradingPlan>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_plans(&conn, strategy_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plans_create(state: State<AppState>, input: PlanInput) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_plan(&conn, input.strategy_id, &input.name, &input.checklist_json, input.notes.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plans_update(
    state: State<AppState>, id: i64, name: String, checklist_json: String, notes: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::update_plan(&conn, id, &name, &checklist_json, notes.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn plans_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_plan(&conn, id).map_err(|e| e.to_string())
}
