#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "ERROR"; exit 1; fi
cd "${PROJECT_DIR}"

echo "==> 1) DB state before snapshot"
DB="$HOME/.local/share/com.trading.manager/app.db"
sqlite3 "$DB" "SELECT id, state, buffer_r, m_preserve, f_internal, f_prop FROM equity_snapshots ORDER BY id DESC LIMIT 3;" 2>&1 || echo "(no rows)"

echo ""
echo "==> 2) call auto_snapshot via test (bypasses GUI)"
cat > /tmp/test_auto_snap.rs <<'EOF'
// temporary probe (not committed)
EOF
# We can't easily call the Tauri command. Instead: query current latest snapshot after user runs refresh.
echo "Run: npm run tauri dev → EquityEngine → Refresh"
echo "Then re-run this script."

echo ""
echo "==> 3) current snapshot (id, state, buffer_r, m_preserve, f_internal, f_prop)"
sqlite3 "$DB" "SELECT id, state, buffer_r, m_preserve, f_internal, f_prop FROM equity_snapshots ORDER BY id DESC LIMIT 1;" 2>&1 || echo "(no rows)"
