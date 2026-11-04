#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

DB="$HOME/.local/share/com.trading.manager/app.db"
BD="$HOME/.local/share/com.trading.manager/backups"
mkdir -p "$BD"
STAMP="$(date +%Y%m%d_%H%M%S)"
BK="$BD/pre_p22c_fix_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

# ============================================================
# 1) error.rs — add MarketDataIncomplete (verify + fix)
# ============================================================
echo "==> [1/5] error.rs — add MarketDataIncomplete (verify)"
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/equity/error.rs")
s = p.read_text()

if "MarketDataIncomplete" in s:
    print("  SKIP: already present")
else:
    old = '''    InstrumentNotFound { symbol: String },
    BelowMinimumQuantity { min_qty: String, allowed_risk: String, actual_risk: String },
}'''
    new = '''    InstrumentNotFound { symbol: String },
    BelowMinimumQuantity { min_qty: String, allowed_risk: String, actual_risk: String },
    MarketDataIncomplete { reason: String },
}'''
    if old in s:
        s = s.replace(old, new)
        p.write_text(s)
        print("  ✓ added")
    else:
        print("  FATAL: anchor not found")
PY

# ============================================================
# 2) commands/equity.rs — DecisionOut fields
# ============================================================
echo "==> [2/5] DecisionOut fields"
python3 - <<'PY'
import pathlib, re, sys
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

# Find DecisionOut struct
m = re.search(r'pub struct DecisionOut \{([^}]*)\}', s, re.DOTALL)
if not m:
    raise SystemExit("DecisionOut not found")
body = m.group(1)

if "sizing_source" not in body:
    new_body = body.rstrip() + '''
    pub sizing_source: String,
    pub actual_risk: Option<String>,
    pub instrument_symbol: Option<String>,
'''
    s = s[:m.start(1)] + new_body + s[m.end(1):]
    p.write_text(s)
    print("  ✓ DecisionOut extended")
else:
    print("  SKIP: already has fields")
PY

# ============================================================
# 3) commands/equity.rs — fix auto_snapshot (day_start, today)
# ============================================================
echo "==> [3/5] Fix auto_snapshot day_start/today"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

# In auto_snapshot_create, we need to define `day_start` and `today` BEFORE our daily::compute block.

# Find the daily::compute block
anchor = '''    // ============================================================
    // 5. Daily loss (PHASE 2.2-C: full daily::compute)
    // ============================================================'''
if anchor not in s:
    print("  FATAL: daily::compute block not found")
else:
    # Before this block, insert:
    prelude = '''    // Day-start equity: if there's a previous snapshot from earlier today
    // (in the reset boundary), use its day_start_equity; else use current equity.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let day_start: Decimal = match &prev {
        Some(p) => {
            let prev_day = p.taken_at.chars().take(10).collect::<String>();
            if prev_day == today {
                Decimal::from_str(&p.day_start_equity).unwrap_or(equity)
            } else {
                equity
            }
        }
        None => equity,
    };

'''
    # Need `prev` — it's computed earlier (in HWM/LWM section). If not, define it.
    # Let's check if `prev` exists earlier
    if "let prev = repo::latest_snapshot" not in s:
        print("  WARN: prev not found earlier — will define in prelude")
        prelude += '''    let prev = repo::latest_snapshot(&conn, Some(acc_id)).map_err(|e| e.to_string())?;
'''

    s = s.replace(anchor, prelude + anchor)
    p.write_text(s)
    print("  ✓ day_start/today prelude added")

# Now remove the LATER `let today = ...` (which is after our daily block) to avoid duplicate
# Actually there's `let today = ...` in the OLD block we removed, and also in the day_start computation later.
# The later one is in the day_start block AFTER our new block. Let's find and see.

# Show remaining `let today` occurrences
print("\n=== remaining `let today` ===")
for i, line in enumerate(s.split("\n"), 1):
    if "let today" in line or "let day_start" in line:
        print(f"  line {i}: {line.strip()}")
PY

# ============================================================
# 4) commands/equity.rs — remove duplicate `today` / `day_start`
# ============================================================
echo "==> [4/5] Remove duplicates"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

# Find any LATER `let today = ...` (after our daily::compute block)
daily_idx = s.find("// 5. Daily loss (PHASE 2.2-C: full daily::compute)")

if daily_idx != -1:
    # Find next `let today = ...` AFTER daily_idx
    after = s[daily_idx:]
    # The old `let today` block is probably right after daily::compute (in the day_start section)
    # We need to find it and remove — it was `let today = chrono::Utc::now().format(...)` after block

    # Actually, we just replaced the daily_loss block, but the `day_start` computation comes AFTER.
    # Let's search for `// 7. Day start` or similar
    day_start_anchor = "// 7. Day start"
    idx = s.find(day_start_anchor, daily_idx)
    if idx != -1:
        # Show 20 lines after
        lines = s[idx:].split("\n")[:25]
        print("  Found day_start section:")
        for line in lines:
            print(f"    {line}")
    else:
        print("  WARN: day_start anchor not found")

p.write_text(s)
PY

# ============================================================
# 5) cargo check
# ============================================================
echo "==> [5/5] cargo check"
( cd src-tauri && cargo check 2>&1 | tee /tmp/p22c_fix.log | tail -15 )
RC=${PIPESTATUS[0]:-$?}
echo "check exit: $RC"
if [ "$RC" -ne 0 ]; then
    echo ""
    echo "=== Errors ==="
    grep -E "^error" -A5 /tmp/p22c_fix.log | head -40
    exit 4
fi
