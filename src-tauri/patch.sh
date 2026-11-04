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
BK="$BD/pre_phase5c_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

echo "==> [1/4] Fix from_snapshot_with_recovery — override m_preserve"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src-tauri/src/equity/state.rs")
s = p.read_text()

# Find from_snapshot_with_recovery method
idx = s.find("pub fn from_snapshot_with_recovery(")
if idx == -1:
    raise SystemExit("method not found")

# Find the end of the method (balanced braces)
brace = s.find("{", idx)
depth = 0
j = brace
while j < len(s):
    if s[j] == "{": depth += 1
    elif s[j] == "}":
        depth -= 1
        if depth == 0: break
    j += 1
end = j + 1

# Replace the method with a fixed version
new_method = '''pub fn from_snapshot_with_recovery(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
        profile: Option<&crate::db::profiles::AccountProfileRow>,
        recovery_authorized: bool,
    ) -> Result<Self, String> {
        use crate::equity::state_machine::{next, State, TransitionInput};

        // Call base constructor
        let mut state = Self::from_snapshot_with_profile(snapshot, config, profile)?;

        // Compute buffer_r for state machine
        let r0 = config.r0.parse::<Decimal>().unwrap_or(Decimal::new(1, 2));
        let r_base = state.snapshot.equity * r0;
        let raw_b = state.snapshot.equity - state.f_internal - state.snapshot.open_portfolio_risk;
        let b = if raw_b > Decimal::ZERO { raw_b } else { Decimal::ZERO };
        let buffer_r = if r_base.is_zero() { Decimal::ZERO } else { b / r_base };

        // Run state machine with recovery_authorized flag
        let previous = state.state_enum;
        let prop_breached = state.snapshot.equity <= state.f_prop && state.f_prop > Decimal::ZERO;
        let ti = TransitionInput {
            previous,
            prop_breached,
            daily_locked: false,
            buffer_r,
            recovery_authorized,
        };
        state.state_enum = next(&ti);

        // PHASE 5-C: In RecoveryMode, override m_preserve with probe_risk_pct (0.25)
        // to allow small probe trades
        if state.state_enum == State::RecoveryMode {
            state.m_preserve = Decimal::new(25, 2);  // 0.25
        }

        // Legacy phase map
        state.phase = match state.state_enum {
            State::PropHardLock => AccountPhase::Disabled,
            State::PreservationPause => AccountPhase::Paused,
            State::RecoveryMode => AccountPhase::Recovery,
            State::Protected => AccountPhase::Caution,
            State::DailyLock => AccountPhase::Paused,
            State::NormalGrowth => AccountPhase::Normal,
        };

        Ok(state)
    }'''

s = s[:idx] + new_method + s[end:]
p.write_text(s)
print("  ✓ from_snapshot_with_recovery: m_preserve = 0.25 in RecoveryMode")
PY

echo ""
echo "==> [2/4] Also check engine.rs — RecoveryMode handling"
grep -n "RecoveryMode\|PreservationPause" src-tauri/src/equity/engine.rs | head -10

echo ""
echo "==> [3/4] Add test for RecoveryMode sizing"
python3 - <<'PY'
import pathlib, re, sys
p = pathlib.Path("src-tauri/src/equity/state.rs")
s = p.read_text()

# Check if test module exists
if "#[cfg(test)]" not in s:
    print("  (no test module — skip)")
    sys.exit(0)

# Add a test for recovery mode
add = '''

    #[cfg(test)]
    mod recovery_tests {
        use super::*;
        use crate::equity::config::EquityConfig;
        use crate::equity::state_machine::State;
        use chrono::Utc;
        use rust_decimal::Decimal;
        use rust_decimal::prelude::FromStr;

        fn snap(equity: &str, initial: &str, hwm: &str, open_risk: &str) -> ParsedSnapshot {
            ParsedSnapshot {
                id: 0,
                equity: Decimal::from_str(equity).unwrap(),
                initial_balance: Decimal::from_str(initial).unwrap(),
                high_watermark: Decimal::from_str(hwm).unwrap(),
                low_watermark: Decimal::from_str(equity).unwrap(),
                day_start_equity: Decimal::from_str(equity).unwrap(),
                open_portfolio_risk: Decimal::from_str(open_risk).unwrap(),
                daily_loss: Decimal::ZERO,
                consecutive_losses: 0,
                taken_at: Utc::now(),
            }
        }

        #[test]
        fn recovery_mode_has_probe_risk() {
            // Equity near floor → PreservationPause
            // With recovery → RecoveryMode + m_preserve = 0.25
            let cfg = EquityConfig::default_v2();
            // Equity 5000, initial 5000, hwm 5000
            // F_trail = 5000 * 1.0 * 0.9 = 4500
            // B = 5000 - 4500 = 500
            // r_base = 5000 * 0.01 = 50
            // buffer_r = 10 → NormalGrowth
            // To force pause: open_risk = 480 → B = 20 → buffer_r = 0.4
            let parsed = snap("5000", "5000", "5000", "480");
            
            // Without recovery → PreservationPause
            let st_normal = AccountState::from_snapshot_with_recovery(
                parsed.clone(), &cfg, None, false
            ).unwrap();
            assert_eq!(st_normal.state_enum, State::PreservationPause);
            assert_eq!(st_normal.m_preserve, Decimal::ZERO);

            // With recovery → RecoveryMode
            let parsed2 = snap("5000", "5000", "5000", "480");
            let st_recovery = AccountState::from_snapshot_with_recovery(
                parsed2, &cfg, None, true
            ).unwrap();
            assert_eq!(st_recovery.state_enum, State::RecoveryMode);
            assert_eq!(st_recovery.m_preserve, Decimal::from_str("0.25").unwrap());
        }
    }
'''

# Find the end of the file and insert
s += add
p.write_text(s)
print("  ✓ test added")
PY

echo ""
echo "==> [4/4] Build + test"
( cd src-tauri && cargo check 2>&1 | grep -E "^error" -A4 | head -15 )
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL: cargo check"
    exit 4
fi

( cd src-tauri && cargo test 2>&1 | grep -E "test result|FAILED" | tail -5 )

echo ""
echo "============================================================"
echo "PHASE 5-C DONE"
echo "  ✓ RecoveryMode: m_preserve = 0.25 (probe risk)
  ✓ Test: recovery_mode_has_probe_risk
  Backup: $BK"
echo "============================================================"
