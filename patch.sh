#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

python3 - <<'PY'
import pathlib
p = pathlib.Path("src/pages/EquityEngine/NewTradeCard.tsx")
s = p.read_text()

# Remove unused imports
s = s.replace("  TradeProposal,\n", "")
s = s.replace("  parseDec,\n", "")

p.write_text(s)
print("  ✓ removed TradeProposal + parseDec")
PY

echo ""
echo "==> Build"
npm run build 2>&1 | tail -10
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL"
    exit 4
fi

echo ""
echo "============================================================"
echo "UNUSED IMPORTS REMOVED"
echo "============================================================"
