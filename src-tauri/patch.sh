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
BK="$BD/pre_p22b_fix10_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

# ============================================================
# 1) db/trades.rs — fresh_db() add current_price columns
# ============================================================
echo "==> [1/3] db/trades.rs fresh_db()"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/db/trades.rs")
s = p.read_text()

# Find fresh_db's CREATE TABLE trades
old = '''CREATE TABLE trades (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id INTEGER, symbol TEXT, side TEXT,
               entry_price TEXT, initial_stop TEXT, take_profit TEXT,
               total_quantity TEXT, outcome TEXT DEFAULT 'open',
               size_type TEXT DEFAULT 'lots', size_value TEXT, size_unit TEXT,
               manual_pnl TEXT,
               original_quantity TEXT,
               status TEXT, strategy_id INTEGER,
               opened_at TEXT, closed_at TEXT, notes TEXT,
               chart_url TEXT, screenshot_path TEXT,
               created_at TEXT, updated_at TEXT
             );'''

new = '''CREATE TABLE trades (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id INTEGER, symbol TEXT, side TEXT,
               entry_price TEXT, initial_stop TEXT, take_profit TEXT,
               total_quantity TEXT, outcome TEXT DEFAULT 'open',
               size_type TEXT DEFAULT 'lots', size_value TEXT, size_unit TEXT,
               manual_pnl TEXT,
               original_quantity TEXT,
               current_price TEXT,
               current_price_updated_at TEXT,
               current_price_source TEXT,
               current_stop TEXT,
               status TEXT, strategy_id INTEGER,
               opened_at TEXT, closed_at TEXT, notes TEXT,
               chart_url TEXT, screenshot_path TEXT,
               created_at TEXT, updated_at TEXT
             );'''

if old in s:
    s = s.replace(old, new)
    p.write_text(s)
    print("  ✓ fresh_db schema updated")
else:
    print("  WARN: fresh_db CREATE TABLE trades anchor not found")
PY

# ============================================================
# 2) equity/engine.rs — fix tests
# ============================================================
echo "==> [2/3] equity/engine.rs tests"
python3 - <<'PY'
import pathlib, re, sys
p = pathlib.Path("src-tauri/src/equity/engine.rs")
s = p.read_text()

# --- Test 1: t_eurusd_sizing ---
# Problem: uses instrument_override + eval_with_inst → should work
# But got 0.2 instead of ~1.0. Because eval_with_inst calls evaluate() (no conn)
# And allow_legacy: false + instrument_override → uses override path
# BUT the issue: snap_fresh() returns equity=10000, R_allowed = 10000 * 0.01 = 100
# 50 pips = 50 ticks × $1 = $50 → size = 100/50 = 2.0
# Why 0.2?
# → Actually check: test says 1.1000 entry, 1.0950 stop = 0.0050 diff
#   tick_size = 0.00001 → 500 ticks (not 50!)
#   Wait: 0.0050 / 0.00001 = 500 ticks
#   × $1 × 1 = $500/lot
#   size = 100/500 = 0.2 ✅ (my assertion was wrong!)
# Fix: assert ~0.2, not ~1.0

old1 = '''    #[test]
    fn t_eurusd_sizing() {
        // EURUSD: 50 pips = 50 ticks (tick_size 0.00001, tick_value 1)
        // loss_per_lot = 50 * 1 = 50
        // R_allowed = 50.03 → size = 50.03/50 ≈ 1.00
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed, "should allow: {:?}", d.reason);
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(size > 0.9 && size < 1.1, "expected ~1.0 lots, got {}", size);
    }'''

new1 = '''    #[test]
    fn t_eurusd_sizing() {
        // EURUSD: 0.0050 price distance / 0.00001 tick_size = 500 ticks
        // 500 ticks × $1/lot = $500 per lot
        // R_allowed = 10000 * 0.01 = $100 → size = 100/500 = 0.20 lots
        let inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(d.allowed, "should allow: {:?}", d.reason);
        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!((size - 0.20).abs() < 0.05, "expected ~0.20 lots, got {}", size);
    }'''

if old1 in s:
    s = s.replace(old1, new1)
    print("  ✓ t_eurusd_sizing fixed (0.20 not 1.0)")

# --- Test 2: t_xauusd_sizing ---
# XAUUSD: entry 2000, stop 1995 → 5 pts / 0.01 = 500 ticks × $1 = $500
# R_allowed ≈ 100 → size = 0.2 ✅
# My old assertion was `size < 0.2` which fails at exactly 0.2
old2 = '''        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(size < 0.2, "expected small size, got {}", size);
    }'''
new2 = '''        let size: f64 = d.size.unwrap().parse().unwrap();
        assert!(size <= 0.21, "expected ≤0.21 lots, got {}", size);
    }'''
if old2 in s:
    s = s.replace(old2, new2)
    print("  ✓ t_xauusd_sizing fixed (≤0.21)")

# --- Test 3: t_estimated_spec_rejected & t_zero_tick_rejected ---
# Problem: eval_with_inst calls evaluate() which validates and returns Err
# But test expects Ok(TradeDecision) with allowed: false
# Fix: change to expect Err from evaluate() OR use a snapshot+override
# Simplest: check that evaluate() returns Err with "not safe" message

old3 = '''    #[test]
    fn t_estimated_spec_rejected() {
        use crate::domain::instrument::SpecSource;
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.spec_source = SpecSource::Estimated;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(!d.allowed);
    }'''

new3 = '''    #[test]
    fn t_estimated_spec_rejected() {
        use crate::domain::instrument::SpecSource;
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.spec_source = SpecSource::Estimated;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        // evaluate() should return Err (validate_for_sizing fails before decision)
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err for estimated spec");
    }'''

if old3 in s:
    s = s.replace(old3, new3)
    print("  ✓ t_estimated_spec_rejected fixed (expects Err)")

# --- Test 4: t_zero_tick_rejected --- (same pattern)
old4 = '''    #[test]
    fn t_zero_tick_rejected() {
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.tick_size = Decimal::ZERO;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let d = eval_with_inst(&st, &cfg, &inst);
        assert!(!d.allowed);
    }'''

new4 = '''    #[test]
    fn t_zero_tick_rejected() {
        let mut inst = make_instrument("EURUSD", "0.00001", "1", "100000", "0.01", "0.01");
        inst.tick_size = Decimal::ZERO;
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: None,
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: Some(inst),
        };
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err for zero tick_size");
    }'''

if old4 in s:
    s = s.replace(old4, new4)
    print("  ✓ t_zero_tick_rejected fixed (expects Err)")

# --- Test 5: t_symbol_not_found_rejected ---
# evaluate() (no conn) with symbol → returns Err (InvalidInput)
# Test should expect Err, not Ok with !allowed

old5 = '''    #[test]
    fn t_symbol_not_found_rejected() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: Some("UNKNOWN".into()),
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        // evaluate() without conn → symbol lookup fails → reject
        let d = evaluate(&st, &cfg, Some(&p)).unwrap();
        assert!(!d.allowed);
        assert_eq!(d.sizing_source, "none");
    }'''

new5 = '''    #[test]
    fn t_symbol_not_found_rejected() {
        let cfg = EquityConfig::default_v2();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        let p = TradeProposal {
            symbol: Some("UNKNOWN".into()),
            entry: "1.1000".into(),
            stop: "1.0950".into(),
            take_profit: None,
            allow_legacy: false,
            contract_multiplier: None,
            commission_per_unit: None,
            slippage_buffer: None,
            instrument_override: None,
        };
        // evaluate() without conn → symbol requires DB → returns Err
        let result = evaluate(&st, &cfg, Some(&p));
        assert!(result.is_err(), "expected Err: symbol needs DB connection");
    }'''

if old5 in s:
    s = s.replace(old5, new5)
    print("  ✓ t_symbol_not_found_rejected fixed (expects Err)")

p.write_text(s)
PY

# ============================================================
# 3) cargo test
# ============================================================
echo "==> [3/3] cargo test"
( cd src-tauri && cargo test 2>&1 | tee /tmp/p22b_fix10.log | grep -E "test result|FAILED" | head -10 )

echo ""
echo "============================================================"
echo "FIX10 DONE"
echo "  Backup: $BK"
echo "============================================================"
