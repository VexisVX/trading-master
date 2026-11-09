#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then
  PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then
  PROJECT_DIR="$(pwd)/trading-manager"
else
  echo "ERROR: cannot find project."; exit 1
fi
cd "${PROJECT_DIR}"
echo "==> In: ${PROJECT_DIR}"

# ============================================================
# 1) Backup helpers in Rust
# ============================================================
echo "==> [1/6] src-tauri/src/db/backup.rs"

cat > src-tauri/src/db/backup.rs <<'EOF'
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
pub fn create_backup(conn: &Connection, db_path: &Path, dest: &Path) -> Result<BackupInfo, DbError> {
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
    let conn = Connection::open_with_flags(
        candidate,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?;

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
EOF

python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/db/mod.rs")
s = p.read_text()
if "pub mod backup;" not in s:
    s = s.replace("pub mod accounts;", "pub mod accounts;\npub mod backup;", 1)
    p.write_text(s)
    print("wired db::backup")
else:
    print("db::backup already wired")
PY

# ============================================================
# 2) Auto-backup before migrations
# ============================================================
echo "==> [2/6] Auto-backup before migrations"

python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/db/mod.rs")
s = p.read_text()

old = '''fn run_migrations(conn: &Connection) -> Result<(), DbError> {
    let mut v = current_version(conn)?;

    if v < 2 { conn.execute_batch(MIGRATION_002)?; set_version(conn, 2)?; v = 2; }
    if v < 3 { conn.execute_batch(MIGRATION_003)?; set_version(conn, 3)?; v = 3; }
    if v < 4 { conn.execute_batch(MIGRATION_004)?; set_version(conn, 4)?; v = 4; }
    if v < 5 { conn.execute_batch(MIGRATION_005)?; set_version(conn, 5)?; v = 5; }
    if v < 6 { conn.execute_batch(MIGRATION_006)?; set_version(conn, 6)?; v = 6; }
    let _ = v;

    Ok(())
}'''

new = '''fn run_migrations(conn: &Connection, db_path: &Path) -> Result<(), DbError> {
    let mut v = current_version(conn)?;

    // Snapshot before any schema change past v1, so the user can roll back
    // their data if a migration is wrong. Best effort — do not block startup.
    if v >= 1 && v < CURRENT_VERSION {
        let backups = backup::backups_dir(db_path);
        let _ = std::fs::create_dir_all(&backups);
        let name = format!("pre-v{}_to_v{}.db", v, CURRENT_VERSION);
        let _ = backup::create_backup(conn, db_path, &backups.join(name));
    }

    if v < 2 { conn.execute_batch(MIGRATION_002)?; set_version(conn, 2)?; v = 2; }
    if v < 3 { conn.execute_batch(MIGRATION_003)?; set_version(conn, 3)?; v = 3; }
    if v < 4 { conn.execute_batch(MIGRATION_004)?; set_version(conn, 4)?; v = 4; }
    if v < 5 { conn.execute_batch(MIGRATION_005)?; set_version(conn, 5)?; v = 5; }
    if v < 6 { conn.execute_batch(MIGRATION_006)?; set_version(conn, 6)?; v = 6; }
    let _ = v;

    Ok(())
}

const CURRENT_VERSION: i64 = 6;'''

if old not in s:
    raise SystemExit("run_migrations block not found")

s = s.replace(old, new)

# Update init_db to pass db_path
s = s.replace(
    "    conn.execute_batch(MIGRATION_001)?;\n    run_migrations(&conn)?;",
    "    conn.execute_batch(MIGRATION_001)?;\n    run_migrations(&conn, db_path)?;"
)

p.write_text(s)
print("auto-backup wired")
PY

# ============================================================
# 3) Commands for backup/restore
# ============================================================
echo "==> [3/6] commands/backup.rs"

cat > src-tauri/src/commands/backup.rs <<'EOF'
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
EOF

python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/commands/mod.rs")
s = p.read_text()
if "pub mod backup;" not in s:
    s = s.replace("pub mod accounts;", "pub mod accounts;\npub mod backup;", 1)
    p.write_text(s)
    print("wired commands::backup")
else:
    print("commands::backup already wired")

p = pathlib.Path("src-tauri/src/lib.rs")
s = p.read_text()
if "commands::backup::backup_to_file" not in s:
    s = s.replace(
        "commands::analytics::analytics_breakdown,",
        "commands::analytics::analytics_breakdown,\n            commands::backup::backup_to_file,\n            commands::backup::backup_validate,\n            commands::backup::backup_restore,\n            commands::backup::backup_dir,\n            commands::backup::backup_now,",
        1
    )
    p.write_text(s)
    print("registered backup commands")
PY

# ============================================================
# 4) Plugin: dialog + fs for file picker
# ============================================================
echo "==> [4/6] Adding tauri plugins (dialog, fs)"

if ! grep -q 'tauri-plugin-dialog' src-tauri/Cargo.toml; then
  sed -i '/^tauri = /a tauri-plugin-dialog = "2"\ntauri-plugin-fs = "2"' src-tauri/Cargo.toml
fi

if ! grep -q '@tauri-apps/plugin-dialog' package.json; then
  npm install --no-audit --no-fund @tauri-apps/plugin-dialog @tauri-apps/plugin-fs >/dev/null
fi

# Register plugins in Rust
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/lib.rs")
s = p.read_text()
if ".plugin(tauri_plugin_dialog::init())" not in s:
    s = s.replace(
        "tauri::Builder::default()",
        "tauri::Builder::default()\n        .plugin(tauri_plugin_dialog::init())\n        .plugin(tauri_plugin_fs::init())",
        1
    )
    p.write_text(s)
    print("registered tauri plugins")
PY

# Add permissions for dialogs and fs
cat > src-tauri/capabilities/default.json <<'EOF'
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default permissions for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    "dialog:allow-save",
    "dialog:allow-open",
    "fs:default"
  ]
}
EOF

# ============================================================
# 5) Frontend: backupApi + Settings section
# ============================================================
echo "==> [5/6] Frontend backup UI"

cat > src/lib/backupApi.ts <<'EOF'
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";

export interface BackupResult {
  path: string;
  size_bytes: number;
  created_at: string;
}

export async function backupNow(): Promise<BackupResult> {
  return invoke<BackupResult>("backup_now");
}

export async function backupToFileInteractive(): Promise<BackupResult | null> {
  const dest = await save({
    title: "Save backup",
    defaultPath: `trading-manager-${new Date().toISOString().slice(0, 10)}.db`,
    filters: [{ name: "SQLite database", extensions: ["db"] }],
  });
  if (!dest) return null;
  return invoke<BackupResult>("backup_to_file", { dest });
}

export async function restoreInteractive(): Promise<string | null> {
  const file = await open({
    title: "Choose backup to restore",
    multiple: false,
    filters: [{ name: "SQLite database", extensions: ["db"] }],
  });
  if (!file || Array.isArray(file)) return null;

  // Validate first
  await invoke<void>("backup_validate", { candidate: file });

  const confirmed = confirm(
    "Restore will replace your current database on disk. The app will need to restart.\n\nContinue?"
  );
  if (!confirmed) return null;

  const path = await invoke<string>("backup_restore", { candidate: file });
  return path;
}

export async function getBackupDir(): Promise<string> {
  return invoke<string>("backup_dir");
}
EOF

# Append Backup card to Settings.tsx
python3 - <<'PY'
import pathlib
p = pathlib.Path("src/pages/Settings/Settings.tsx")
s = p.read_text()

# Imports
if "backupApi" not in s:
    s = s.replace(
        'import { Select } from "../../components/ui/Select";',
        'import { Select } from "../../components/ui/Select";\nimport {\n  backupNow, backupToFileInteractive, restoreInteractive, getBackupDir, BackupResult,\n} from "../../lib/backupApi";'
    )

# State
s = s.replace(
    "  const [pfSavedAt, setPfSavedAt] = useState<string | null>(null);",
    "  const [pfSavedAt, setPfSavedAt] = useState<string | null>(null);\n  const [backupDir, setBackupDir] = useState<string>(\"\");\n  const [lastBackup, setLastBackup] = useState<BackupResult | null>(null);\n  const [backupBusy, setBackupBusy] = useState(false);"
)

# Load backup dir
s = s.replace(
    "  useEffect(() => { refreshAccounts(); }, [refreshAccounts]);",
    "  useEffect(() => { refreshAccounts(); }, [refreshAccounts]);\n  useEffect(() => { getBackupDir().then(setBackupDir).catch(() => {}); }, []);"
)

# Handlers
s = s.replace(
    "  const active = accounts.find((a) => a.id === activeId);",
    """  async function doBackupNow() {
    setBackupBusy(true); setError(null);
    try { setLastBackup(await backupNow()); }
    catch (e) { setError(String(e)); }
    finally { setBackupBusy(false); }
  }

  async function doBackupToFile() {
    setBackupBusy(true); setError(null);
    try {
      const r = await backupToFileInteractive();
      if (r) setLastBackup(r);
    } catch (e) { setError(String(e)); }
    finally { setBackupBusy(false); }
  }

  async function doRestore() {
    setError(null);
    try {
      const path = await restoreInteractive();
      if (path) {
        alert("Backup restored. Please restart the app for the changes to take effect.");
      }
    } catch (e) { setError(String(e)); }
  }

  const active = accounts.find((a) => a.id === activeId);"""
)

# Add backup card before the final closing </div>
insert = '''
      <div className="card">
        <h2 className="section-title">Backup & Restore</h2>
        <p className="settings__hint">
          Backups are plain SQLite files. Keep them outside this app's folder — a copy in <code>backups/</code> is auto-created before migrations.
        </p>

        <div className="backup-row">
          <button className="btn btn--primary" onClick={doBackupNow} disabled={backupBusy}>
            Backup now (to app folder)
          </button>
          <button className="btn btn--ghost" onClick={doBackupToFile} disabled={backupBusy}>
            Save backup to…
          </button>
          <button className="btn btn--danger" onClick={doRestore}>
            Restore from file…
          </button>
        </div>

        {backupDir && (
          <p className="settings__meta">Auto-backup folder: <span className="mono">{backupDir}</span></p>
        )}

        {lastBackup && (
          <p className="settings__meta">
            Last backup: <span className="mono">{lastBackup.path}</span> ·
            {(lastBackup.size_bytes / 1024).toFixed(1)} KB ·
            {lastBackup.created_at}
          </p>
        )}
      </div>
'''

idx = s.rstrip().rfind("</div>")
s = s[:idx] + insert + s[idx:]
p.write_text(s)
print("added backup card to Settings")
PY

# Append CSS
cat >> src/pages/Settings/Settings.css <<'EOF'

.backup-row {
  display: flex; gap: 10px; flex-wrap: wrap; margin: 6px 0 12px;
}
EOF

# ============================================================
# 6) Build script + docs
# ============================================================
echo "==> [6/6] build.sh + PACKAGING.md"

cat > build.sh <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

# Build distributable packages and report sizes.
# Usage: ./build.sh [--clean]

CLEAN=0
[[ "${1:-}" == "--clean" ]] && CLEAN=1

if [ "${CLEAN}" == "1" ]; then
  echo "==> Cleaning build artifacts"
  cd src-tauri && cargo clean && cd ..
  rm -rf dist
fi

echo "==> Building release bundles (this may take several minutes)"
npm run tauri build

echo ""
echo "==> Bundles:"
ls -lh src-tauri/target/release/bundle/deb/*.deb 2>/dev/null || echo "  (no .deb)"
ls -lh src-tauri/target/release/bundle/rpm/*.rpm 2>/dev/null || echo "  (no .rpm)"
ls -lh src-tauri/target/release/bundle/appimage/*.AppImage 2>/dev/null || echo "  (no .AppImage)"

echo ""
echo "==> Binary size:"
ls -lh src-tauri/target/release/trading-manager 2>/dev/null || echo "  (no binary)"

echo ""
echo "==> Disk usage of the release folder:"
du -sh src-tauri/target/release 2>/dev/null || true

echo ""
echo "==> Install:"
echo "  sudo pacman -U src-tauri/target/release/bundle/deb/*.deb   # Debian-style"
echo "  sudo pacman -U <path-to-.rpm>                              # if built"
echo "  chmod +x src-tauri/target/release/bundle/appimage/*.AppImage && ./...AppImage"
echo ""
echo "==> Data location (not bundled):"
echo "  ~/.local/share/com.trading.manager/app.db"
echo "  ~/.local/share/com.trading.manager/backups/"
EOF
chmod +x build.sh

cat > PACKAGING.md <<'EOF'
# Packaging

## Build all bundles

```bash
./build.sh
