#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

echo "==> [1/3] Rewrite create_trade INSERT"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/db/trades.rs")
s = p.read_text()

# Replace the entire INSERT INTO trades block
old_insert = '''        "INSERT INTO trades
         (account_id, symbol, side, entry_price, initial_stop, take_profit, total_quantity,
          original_quantity, size_type, size_value, size_unit, outcome,
          status, strategy_id, opened_at, closed_at, notes, chart_url, screenshot_path,
          manual_pnl, current_price, current_price_updated_at, current_price_source, current_stop, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8,?9,?10,'open','open',?11,?12,NULL,?13,?14,?15,NULL,?16,?16)",'''

new_insert = '''        "INSERT INTO trades
         (account_id, symbol, side, entry_price, initial_stop, take_profit, total_quantity,
          original_quantity, outcome, size_type, size_value, size_unit,
          manual_pnl, current_price, current_price_updated_at, current_price_source, current_stop,
          status, strategy_id, opened_at, closed_at, notes, chart_url, screenshot_path,
          created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,'open',?8,?9,?10,NULL,NULL,NULL,NULL,NULL,'open',?11,?12,NULL,?13,?14,?15,?16,?16)",'''

if old_insert in s:
    s = s.replace(old_insert, new_insert)
    p.write_text(s)
    print("  ✓ INSERT rewritten (26 columns, 26 values)")
else:
    print("  WARN: exact INSERT anchor not found — trying regex")
    # Try to match with flexible whitespace
    pattern = re.compile(
        r'"INSERT INTO trades\s*\n[^"]*"',
        re.DOTALL
    )
    match = pattern.search(s)
    if match:
        s = s[:match.start()] + new_insert.rstrip(',"') + s[match.end():]
        p.write_text(s)
        print("  ✓ INSERT replaced via regex")
    else:
        print("  FATAL: INSERT not found")
PY

echo ""
echo "==> [2/3] Show new INSERT"
grep -n "INSERT INTO trades" -A6 src-tauri/src/db/trades.rs | head -10

echo ""
echo "==> [3/3] cargo test db::trades"
( cd src-tauri && cargo test db::trades 2>&1 | grep -E "test result|FAILED" | head -6 )

echo ""
if ( cd src-tauri && cargo test db::trades 2>&1 | grep -qE "FAILED" ); then
    echo "=== errors ==="
    ( cd src-tauri && cargo test db::trades 2>&1 | grep -B2 -A5 "panicked" | head -20 )
    exit 4
fi

echo ""
echo "============================================================"
echo "FIX13 DONE"
echo "============================================================"
