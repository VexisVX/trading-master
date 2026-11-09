//! Backup and restore helpers.
//!
//! A backup is a copy of the SQLite file, made while a checkpoint is forced.
//! Restore replaces the live file atomically:
//!   1. Validate the candidate file (open, check schema_version exists).
//!   2. Copy it next to the live file as `.incoming`.
//!   3. Close live connection, rename live → `.old`, rename `.incoming` → live.
//!   4. Remove `.old` if everything succeeded.
//!
//! If anything fails between steps 3 and 4, the caller can restore `.old`.

use super::DbError;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BackupInfo {
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

/// Force a WAL checkpoint so the .db file is self-contained, then copy it.
pub fn create_backup(
    conn: &Connection,
    db_path: &Path,
    dest: &Path,
) -> Result<BackupInfo, DbError> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(db_path, dest)?;

    let size = fs::metadata(dest)?.len();
    let created_at = chrono::Utc::now().to_rfc3339();

    Ok(BackupInfo {
        path: dest.to_string_lossy().to_string(),
        size_bytes: size,
        created_at,
    })
}

/// Validate that the given file is a readable SQLite database with our schema.
pub fn validate_backup(candidate: &Path) -> Result<(), DbError> {
    let conn = Connection::open_with_flags(candidate, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    // Must have app_meta with schema_version
    let version: Option<String> = conn
        .query_row(
            "SELECT value FROM app_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .ok();

    if version.is_none() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }

    // Sanity: must have core tables
    for table in &["accounts", "trades", "trade_legs"] {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |r| r.get(0),
        )?;
        if exists == 0 {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
    }
    Ok(())
}

/// Replace `live` with `candidate` (already validated). Returns the path of
/// the previous backup-in-place, which the caller may delete after success.
pub fn restore_from(live: &Path, candidate: &Path) -> Result<PathBuf, DbError> {
    validate_backup(candidate)?;

    let tmp = live.with_extension("incoming");
    fs::copy(candidate, &tmp)?;

    let old = live.with_extension("old");

    // If a previous .old exists from a failed restore, remove it first.
    if old.exists() {
        let _ = fs::remove_file(&old);
    }

    if live.exists() {
        fs::rename(live, &old)?;
    }
    fs::rename(&tmp, live)?;

    Ok(old)
}

/// Ensure `backups/` exists next to the live DB and return its path.
pub fn backups_dir(db_path: &Path) -> PathBuf {
    db_path.parent().unwrap_or(Path::new(".")).join("backups")
}
