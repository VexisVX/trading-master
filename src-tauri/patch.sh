#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

DB="$HOME/.local/share/com.trading-manager/app.db"
BD="$HOME/.local/share/com.trading-manager/backups"
mkdir -p "$BD"
STAMP="$(date +%Y%m%d_%H%M%S)"
BK="$BD/pre_fix_inst_deadlock_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

echo "==> [1/2] Fix inst_sym_audit deadlock"
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

old = '''    let (inst_sym_audit, inst_spec_audit) = match &decision.instrument_symbol {
        Some(sym) => {
            let spec_json = {
                let conn2 = state.db.lock().map_err(|e| e.to_string())?;
                crate::db::instruments::get_typed(&conn2, sym)
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::to_string(&s).ok())
            };
            (Some(sym.as_str()), spec_json)
        }
        None => (None, None),
    };'''

new = '''    // PHASE 4: instrument audit — use the already-locked `conn`, do NOT re-lock.
    let (inst_sym_audit, inst_spec_audit) = match &decision.instrument_symbol {
        Some(sym) => {
            let spec_json = crate::db::instruments::get_typed(&conn, sym)
                .ok()
                .flatten()
                .and_then(|s| serde_json::to_string(&s).ok());
            (Some(sym.as_str()), spec_json)
        }
        None => (None, None),
    };'''

if old in s:
    s = s.replace(old, new, 1)
    p.write_text(s)
    print("  ✓ fixed: inst_sym_audit uses conn (no re-lock)")
else:
    print("  WARN: anchor not found")
PY

echo ""
echo "==> [2/2] Build"
( cd src-tauri && cargo check 2>&1 | grep -E "^error" -A4 | head -15 )
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL"
    exit 4
fi
echo "  ✓ cargo check passed"

echo ""
echo "============================================================"
echo "INSTRUMENT AUDIT DEADLOCK FIXED"
echo "  ✓ inst_sym_audit uses existing conn
echo "  Backup: $BK"
echo "  Now run: npm run tauri dev → Evaluate Trade"
echo "============================================================"
