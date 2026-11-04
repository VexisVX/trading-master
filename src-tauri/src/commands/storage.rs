use crate::commands::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct StorageStats {
    pub db_size_bytes: u64,
    pub trades_count: i64,
    pub strategies_count: i64,
    pub accounts_count: i64,
    pub attachments_count: i64,
    pub attachments_bytes: u64,
}

#[tauri::command]
pub fn storage_stats(
    app: tauri::AppHandle,
    state: State<AppState>,
) -> Result<StorageStats, String> {
    use tauri::Manager;

    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {e}"))?;
    let db_path = app_dir.join("app.db");

    let db_size_bytes = std::fs::metadata(&db_path).map(|m| m.len()).unwrap_or(0);

    // Count attachments folder size
    let attach_dir = app_dir.join("attachments");
    let (attachments_count, attachments_bytes) = count_attachments(&attach_dir);

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let trades_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM trades", [], |r| r.get(0))
        .unwrap_or(0);
    let strategies_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM strategies", [], |r| r.get(0))
        .unwrap_or(0);
    let accounts_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))
        .unwrap_or(0);

    Ok(StorageStats {
        db_size_bytes,
        trades_count,
        strategies_count,
        accounts_count,
        attachments_count,
        attachments_bytes,
    })
}

fn count_attachments(dir: &std::path::Path) -> (i64, u64) {
    if !dir.exists() {
        return (0, 0);
    }
    let mut count = 0i64;
    let mut bytes = 0u64;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_dir() {
                    if let Ok(sub) = std::fs::read_dir(entry.path()) {
                        for subentry in sub.flatten() {
                            if let Ok(m) = subentry.metadata() {
                                count += 1;
                                bytes += m.len();
                            }
                        }
                    }
                } else if meta.is_file() {
                    count += 1;
                    bytes += meta.len();
                }
            }
        }
    }
    (count, bytes)
}
