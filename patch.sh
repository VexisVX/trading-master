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
BK="$BD/pre_fix_select_syntax_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

echo "==> [1/3] Show broken block"
sed -n '172,200p' src/pages/EquityEngine/NewTradeCard.tsx

echo ""
echo "==> [2/3] Fix JSX structure"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src/pages/EquityEngine/NewTradeCard.tsx")
s = p.read_text()

# Find the broken block: from `<div className="ee-symbol-row">` to matching `</div>`
start = s.find('<div className="ee-symbol-row">')
if start == -1:
    raise SystemExit("ee-symbol-row not found")

# Find matching </div> (balance)
depth = 0
i = start
while i < len(s):
    if s[i:i+4] == '<div':
        depth += 1
        i += 4
    elif s[i:i+6] == '</div>':
        depth -= 1
        if depth == 0:
            end = i + 6
            break
        i += 6
    else:
        i += 1
else:
    raise SystemExit("matching </div> not found")

old_block = s[start:end]
print(f"  Old block ({len(old_block)} chars):")
print(f"  {old_block[:300]}")

# Build clean replacement — inline options, not from variable
new_block = '''<div className="ee-symbol-row">
            <select
              value={symbol}
              onChange={(e) => setSymbol(e.target.value)}
              disabled={instruments.length === 0}
              className="ee-symbol-select"
            >
              <option value="">— select symbol —</option>
              {instruments.map((i) => (
                <option key={i.symbol} value={i.symbol}>
                  {i.symbol} ({i.spec_source})
                </option>
              ))}
            </select>
            <button
              type="button"
              className="ee-symbol-refresh"
              onClick={handleRefreshInstruments}
              disabled={refreshingInstruments}
              title="Reload instruments from database"
            >
              <span className={refreshingInstruments ? "ee-spin" : ""}>⟳</span>
            </button>
          </div>'''

s = s[:start] + new_block + s[end:]
p.write_text(s)
print("  ✓ block replaced")
PY

echo ""
echo "==> [3/3] Build"
npm run build 2>&1 | tail -10
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL"
    exit 4
fi

echo ""
echo "============================================================"
echo "SELECT JSX FIXED"
echo "  Backup: $BK"
echo "============================================================"
