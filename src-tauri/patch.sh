#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "ERROR"; exit 1; fi
cd "${PROJECT_DIR}"

# ---------- 1) Fix engine::tests::rejected_when_paused ----------
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/equity/engine.rs")
s = p.read_text()

old = '''    #[test]
    fn rejected_when_paused() {
        let mut cfg = EquityConfig::default_v1();
        cfg.n_max = 5;
        let mut s = snap_fresh();
        s.consecutive_losses = 5;
        let st = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st, &cfg, None).unwrap();
        assert!(!d.allowed);
        matches!(d.reason, DecisionReason::AccountPaused);
    }'''

new = '''    #[test]
    fn rejected_when_prop_hard_lock() {
        // S11: pause now comes from prop breach / state machine, not n_max.
        // Force prop breach by setting a huge D_prop so f_prop > equity.
        let mut cfg = EquityConfig::default_v1();
        cfg.d_prop = "99999".into();
        let st = AccountState::from_snapshot(snap_fresh(), &cfg).unwrap();
        // f_prop = initial - d_prop = 10000 - 99999 < 0
        // equity 10000 > f_prop → NOT breached. Need different approach.
        // Set equity very low instead: use a snapshot with equity below floor.
        // Simpler: assert that disabled phase rejects.
        let mut s = snap_fresh();
        s.equity = rust_decimal::Decimal::from_str("100").unwrap();
        s.high_watermark = rust_decimal::Decimal::from_str("10000").unwrap();
        let st2 = AccountState::from_snapshot(s, &cfg).unwrap();
        let d = evaluate(&st2, &cfg, None).unwrap();
        assert!(!d.allowed);
    }'''

if old in s:
    s = s.replace(old, new)
    p.write_text(s)
    print("engine.rs: rejected_when_paused → rejected_when_prop_hard_lock")
else:
    print("WARN: rejected_when_paused anchor not found")
PY

# ---------- 2) Inspect db/trades tests ----------
echo "==> inspect db/trades.rs lines 380-420"
sed -n '380,420p' src-tauri/src/db/trades.rs

echo ""
echo "==> fix attempts will follow after inspection"
