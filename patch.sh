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
BK="$BD/pre_p22c_fix4_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

# ============================================================
# 1) error.rs — add MarketDataIncomplete variant
# ============================================================
echo "==> [1/3] error.rs — add MarketDataIncomplete variant"
python3 - <<'PY'
import pathlib, re, sys
p = pathlib.Path("src-tauri/src/equity/error.rs")
s = p.read_text()

# Find `pub enum DecisionReason {` and add variant BEFORE closing }
m = re.search(r'(pub enum DecisionReason \{)(.*?)(\n\})', s, re.DOTALL)
if not m:
    raise SystemExit("DecisionReason not found")

body = m.group(2)
if "MarketDataIncomplete" in body and "MarketDataIncomplete {" in body:
    print("  SKIP: variant already present")
    sys.exit(0)

# Add after last variant (before closing })
# Find the last variant line in body
new_body = body.rstrip()
if not new_body.endswith(','):
    new_body += ','
new_body += '\n    MarketDataIncomplete { reason: String },'

s = s[:m.start(2)] + new_body + s[m.end(2):]
p.write_text(s)
print("  ✓ MarketDataIncomplete variant added")

# Verify
print("\n=== DecisionReason variants ===")
for line in s.split("\n"):
    if "MarketDataIncomplete" in line:
        print(f"  {line.strip()}")
PY

# ============================================================
# 2) commands/equity.rs — fix DecisionOut at line 387
# ============================================================
echo "==> [2/3] Fix DecisionOut at 387"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

# The construction at 387 ends with `})` (RiskDetail close) then `})` (DecisionOut close)
# Look for `risk_detail: decision.risk_detail.as_ref().map(|r| { ... })` block
# Need to add fields AFTER the risk_detail block, before final `})`

# Find `Ok(DecisionOut {` at ~387
# Then find the matching `})` 
# Easier: find `risk_detail: decision.risk_detail.as_ref().map(|r| {` and locate the closing `})` before the final `})` of DecisionOut

# Strategy: find `snapshot_id: Some(snap_row.id),` and ensure new fields are after
idx = s.find("snapshot_id: Some(snap_row.id),")
if idx == -1:
    print("  FATAL: snapshot_id anchor not found")
else:
    # Check if sizing_source already present after this
    after = s[idx:idx+2000]
    if "sizing_source:" in after and "sizing_source: decision" in after:
        print("  SKIP: already has fields")
    else:
        # Find `risk_detail: decision.risk_detail.as_ref().map(|r| {` and its closing `}),`
        risk_start = s.find("risk_detail: decision.risk_detail.as_ref().map(|r| {", idx)
        if risk_start == -1:
            print("  FATAL: risk_detail block not found")
        else:
            # Find the closing `}),` for the map
            # Count braces
            depth = 0
            i = s.find("{", risk_start)  # open of map closure
            j = i
            while j < len(s):
                if s[j] == "{":
                    depth += 1
                elif s[j] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            # j is at closing `}` of map closure
            # Then there's `}),` for DecisionOut's risk_detail field
            # We need to add new fields AFTER this `}),`
            
            # Find `}),` after j
            comma_close = s.find("}),", j)
            if comma_close == -1:
                print("  FATAL: closing `}),` not found")
            else:
                after_close = comma_close + 3  # after "}),"
                new_fields = '''
        sizing_source: decision.sizing_source.clone(),
        actual_risk: decision.actual_risk.clone(),
        instrument_symbol: decision.instrument_symbol.clone(),'''
                s = s[:after_close] + new_fields + s[after_close:]
                p.write_text(s)
                print("  ✓ sizing_source/actual_risk/instrument_symbol added")
PY

# ============================================================
# 3) cargo check
# ============================================================
echo "==> [3/3] cargo check"
( cd src-tauri && cargo check 2>&1 | tee /tmp/p22c_fix4.log | tail -15 )
RC=${PIPESTATUS[0]:-$?}
echo "check exit: $RC"

if [ "$RC" -ne 0 ]; then
    echo ""
    echo "=== Errors ==="
    grep -E "^error" -A6 /tmp/p22c_fix4.log | head -30
    exit 4
fi

echo ""
echo "============================================================"
echo "PHASE 2.2-C FIX4 DONE"
echo "  Backup: $BK"
echo "============================================================"
