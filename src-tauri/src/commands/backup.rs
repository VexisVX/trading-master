use crate::commands::AppState;
use crate::db::backup as repo;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Resolve the live DB path from Tauri's app_data_dir.
fn live_db_path<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {e}"))?;
    Ok(dir.join("app.db"))
}

#[tauri::command]
pub fn backup_to_file(
    app: tauri::AppHandle,
    state: State<AppState>,
    dest: String,
) -> Result<BackupResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let db = live_db_path(&app)?;
    let info = repo::create_backup(&conn, &db, &PathBuf::from(&dest))
        .map_err(|e| e.to_string())?;
    Ok(BackupResult {
        path: info.path,
        size_bytes: info.size_bytes,
        created_at: info.created_at,
    })
}

#[tauri::command]
pub fn backup_validate(candidate: String) -> Result<(), String> {
    repo::validate_backup(&PathBuf::from(candidate)).map_err(|e| e.to_string())
}

/// Restores from a candidate file. The live connection stays open until the
/// caller restarts the app — this command only swaps the file on disk.
#[tauri::command]
pub fn backup_restore(
    app: tauri::AppHandle,
    candidate: String,
) -> Result<String, String> {
    let db = live_db_path(&app)?;
    let old = repo::restore_from(&db, &PathBuf::from(&candidate))
        .map_err(|e| e.to_string())?;

    // On success, remove the .old file. If the app fails to start, the user
    // still has `.old` on disk as a recovery option.
    if old.exists() {
        let _ = std::fs::remove_file(&old);
    }

    Ok(db.to_string_lossy().to_string())
}

#[tauri::command]
pub fn backup_dir(app: tauri::AppHandle) -> Result<String, String> {
    let db = live_db_path(&app)?;
    let dir = repo::backups_dir(&db);
    Ok(dir.to_string_lossy().to_string())
}

/// Quick backup into the app's backups/ folder. Used by the "Backup now"
/// button — the user doesn't have to pick a path.
#[tauri::command]
pub fn backup_now(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<BackupResult, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let db = live_db_path(&app)?;
    let dir = repo::backups_dir(&db);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name = format!("manual_{}.db", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    let info = repo::create_backup(&conn, &db, &dir.join(name))
        .map_err(|e| e.to_string())?;
    Ok(BackupResult {
        path: info.path,
        size_bytes: info.size_bytes,
        created_at: info.created_at,
    })
}
