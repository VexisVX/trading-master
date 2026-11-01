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

echo "==> [1/3] commands/live_presets.rs"
cat > src-tauri/src/commands/live_presets.rs <<'EOF'
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
pub fn presets_list(state: State<AppState>, account_id: i64) -> Result<Vec<repo::LivePreset>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_presets(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn presets_create(state: State<AppState>, account_id: i64, preset: PresetPayload) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_preset(&conn, account_id, &preset.name, preset.instrument.as_deref(), &preset.payload_json)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn presets_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_preset(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn drafts_create(state: State<AppState>, account_id: i64, payload_json: String) -> Result<i64, String> {
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
EOF

echo "==> [2/3] Wiring commands"

python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/commands/mod.rs")
s = p.read_text()
if "pub mod live_presets;" not in s:
    s = s.replace("pub mod live_calc;", "pub mod live_calc;\npub mod live_presets;", 1)
    p.write_text(s)

p = pathlib.Path("src-tauri/src/lib.rs")
s = p.read_text()
for cmd in [
    "commands::live_presets::presets_list",
    "commands::live_presets::presets_create",
    "commands::live_presets::presets_delete",
    "commands::live_presets::drafts_create",
    "commands::live_presets::drafts_get",
    "commands::live_presets::drafts_delete",
]:
    if cmd not in s:
        s = s.replace(
            "commands::live_calc::live_portfolio_risk,",
            f"commands::live_calc::live_portfolio_risk,\n            {cmd},",
            1
        )
p.write_text(s)
print("registered commands")
PY

echo "==> [3/3] Creating src/lib/livePresetsApi.ts"

cat > src/lib/livePresetsApi.ts <<'EOF'
import { invoke } from "@tauri-apps/api/core";

export interface LivePreset {
  id: number;
  account_id: number;
  name: string;
  instrument: string | null;
  payload_json: string;
  created_at: string;
  updated_at: string;
}

export async function listPresets(accountId: number): Promise<LivePreset[]> {
  return invoke<LivePreset[]>("presets_list", { accountId });
}

export interface PresetPayload {
  name: string;
  instrument: string | null;
  payload_json: string;
}

export async function createPreset(accountId: number, preset: PresetPayload): Promise<number> {
  return invoke<number>("presets_create", { accountId, preset });
}

export async function deletePreset(id: number): Promise<void> {
  return invoke<void>("presets_delete", { id });
}

export interface TradeDraft {
  id: number;
  account_id: number;
  payload_json: string;
  created_at: string;
  updated_at: string;
}

export async function createDraft(accountId: number, payloadJson: string): Promise<number> {
  return invoke<number>("drafts_create", { accountId, payloadJson });
}

export async function getDraft(id: number): Promise<TradeDraft | null> {
  return invoke<TradeDraft | null>("drafts_get", { id });
}

export async function deleteDraft(id: number): Promise<void> {
  return invoke<void>("drafts_delete", { id });
}
EOF

echo ""
echo "==> cargo check"
cd src-tauri && cargo check 2>&1 | tail -8 && cd ..
echo ""
echo "==> tsc --noEmit"
npx tsc --noEmit 2>&1 | tail -8

echo ""
echo "============================================================"
echo " Step 3.1 done (backend + API, no UI changes yet)."
echo " Run: npm run tauri dev"
echo ""
echo " The 8 warnings about live_presets should now be gone."
echo "============================================================"
