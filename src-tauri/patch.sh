#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

echo "==> [1/3] Fix RiskDetail construction with Option<String>"
python3 - <<'PY'
import pathlib, sys
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

old = '''                    let mult = parse_dec(&p.contract_multiplier, "mult").unwrap_or(Decimal::ONE);
                    let comm = parse_dec(&p.commission_per_unit, "comm").unwrap_or(Decimal::ZERO);
                    let slip = parse_dec(&p.slippage_buffer, "slip").unwrap_or(Decimal::ZERO);'''

new = '''                    let mult = parse_dec(p.contract_multiplier.as_deref().unwrap_or("1"), "mult").unwrap_or(Decimal::ONE);
                    let comm = parse_dec(p.commission_per_unit.as_deref().unwrap_or("0"), "comm").unwrap_or(Decimal::ZERO);
                    let slip = parse_dec(p.slippage_buffer.as_deref().unwrap_or("0"), "slip").unwrap_or(Decimal::ZERO);'''

if old in s:
    s = s.replace(old, new)
    p.write_text(s)
    print("  ✓ RiskDetail parse fixed")
else:
    print("  WARN: exact anchor not found — trying flexible")
    import re
    # Try to fix any `&p.contract_multiplier` pattern
    s = re.sub(
        r'parse_dec\(&p\.contract_multiplier,\s*"mult"\)',
        'parse_dec(p.contract_multiplier.as_deref().unwrap_or("1"), "mult")',
        s
    )
    s = re.sub(
        r'parse_dec\(&p\.commission_per_unit,\s*"comm"\)',
        'parse_dec(p.commission_per_unit.as_deref().unwrap_or("0"), "comm")',
        s
    )
    s = re.sub(
        r'parse_dec\(&p\.slippage_buffer,\s*"slip"\)',
        'parse_dec(p.slippage_buffer.as_deref().unwrap_or("0"), "slip")',
        s
    )
    p.write_text(s)
    print("  ✓ regex-based fix applied")
PY

echo ""
echo "==> [2/3] cargo check"
( cd src-tauri && cargo check 2>&1 | tee /tmp/fix6.log | tail -10 )
RC=${PIPESTATUS[0]:-$?}
echo "check exit: $RC"
if [ "$RC" -ne 0 ]; then
    echo ""
    echo "=== remaining errors ==="
    grep -E "^error" -A3 /tmp/fix6.log | head -20
    exit 4
fi

echo ""
echo "==> [3/3] cargo test (quick)"
( cd src-tauri && cargo test --lib 2>&1 | grep -E "test result" | tail -3 )

echo ""
echo "============================================================"
echo "FIX6 DONE"
echo "============================================================"
