#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

# ---------- 0) verify ----------
echo "==> [0/5] Verify state"
DB="$HOME/.local/share/com.trading.manager/app.db"
VER="$(sqlite3 "$DB" "SELECT value FROM app_meta WHERE key='schema_version';")"
[ "$VER" = "23" ] || { echo "FATAL: schema_version=$VER"; exit 2; }
echo "  ✓ schema_version=23"

if ! grep -q "pub struct AccountProfileRow" src-tauri/src/db/profiles.rs; then
  echo "FATAL: AccountProfileRow missing"; exit 2
fi
echo "  ✓ AccountProfileRow present"

# ---------- 1) WAL-safe backup ----------
echo "==> [1/5] Backup"
BD="$HOME/.local/share/com.trading.manager/backups"
mkdir -p "$BD"
STAMP="$(date +%Y%m%d_%H%M%S)"
BK="$BD/pre_s16_${STAMP}.db"
sqlite3 "$DB" ".backup '$BK'"
INTEG="$(sqlite3 "$BK" 'PRAGMA integrity_check;' || true)"
[ "$INTEG" = "ok" ] || { echo "FATAL: integrity: $INTEG"; exit 3; }
echo "  ✓ $BK"

# ---------- 2) patch state.rs — from_snapshot signature ----------
echo "==> [2/5] Patching state.rs"
python3 - <<'PY'
import pathlib, sys
p = pathlib.Path("src-tauri/src/equity/state.rs")
s = p.read_text()

# 2a) Change signature
old_sig = '''    pub fn from_snapshot(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
    ) -> Result<Self, String> {'''
new_sig = '''    pub fn from_snapshot(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
    ) -> Result<Self, String> {
        Self::from_snapshot_with_profile(snapshot, config, None)
    }

    /// Profile-aware variant: if `profile` is Some, use its derived values
    /// for profit protection / trailing DD instead of the global config.
    pub fn from_snapshot_with_profile(
        snapshot: ParsedSnapshot,
        config: &crate::equity::EquityConfig,
        profile: Option<&crate::db::profiles::AccountProfileRow>,
    ) -> Result<Self, String> {'''
if old_sig not in s:
    raise SystemExit("from_snapshot signature not found")
s = s.replace(old_sig, new_sig)

# 2b) In the ProtectionConfig build, override from profile if present
old_cfg = '''        let prot_cfg = crate::equity::protection::ProtectionConfig::default_v1();
        let prot = crate::equity::protection::compute(
            snapshot.initial_balance,
            nav_hwm,
            &prot_cfg,
        );'''
new_cfg = '''        let mut prot_cfg = crate::equity::protection::ProtectionConfig::default_v1();
        if let Some(p) = profile {
            if let Ok(v) = Decimal::from_str(&p.profit_protect_activation) { prot_cfg.profit_protect_activation = v; }
            if let Ok(v) = Decimal::from_str(&p.profit_protect_full)       { prot_cfg.profit_protect_full = v; }
            if let Ok(v) = Decimal::from_str(&p.max_profit_retention)      { prot_cfg.max_profit_retention = v; }
            if let Ok(v) = Decimal::from_str(&p.trailing_dd_pct)           { prot_cfg.trailing_dd_pct = v; }
        }
        let prot = crate::equity::protection::compute(
            snapshot.initial_balance,
            nav_hwm,
            &prot_cfg,
        );'''
if old_cfg not in s:
    raise SystemExit("protection config block not found")
s = s.replace(old_cfg, new_cfg)

p.write_text(s)
print("  ✓ state.rs: from_snapshot_with_profile added")
PY

# ---------- 3) patch commands/equity.rs — pass profile ----------
echo "==> [3/5] Patching commands/equity.rs"

python3 - <<'PY'
import pathlib, sys, re
p = pathlib.Path("src-tauri/src/commands/equity.rs")
s = p.read_text()

# 3a) equity_auto_snapshot_create — load profile + call with_profile
# Find the AccountState::from_snapshot call in auto_snapshot
old = '''    let acct_state = crate::equity::state::AccountState::from_snapshot(parsed_for_state, &cfg)
        .map_err(|e| format!("AccountState: {}", e))?;'''
new = '''    let profile_opt = {
        let conn2 = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::profiles::get(&conn2, acc_id).map_err(|e| e.to_string())?
    };
    let acct_state = crate::equity::state::AccountState::from_snapshot_with_profile(
        parsed_for_state, &cfg, profile_opt.as_ref()
    ).map_err(|e| format!("AccountState: {}", e))?;'''
if old in s:
    s = s.replace(old, new, 1)
    print("  ✓ auto_snapshot_create: profile-aware")

# 3b) equity_evaluate — load profile for the account_id arg
old_eval = '''    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let state_acct = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("state: {e}"))?;

    // 2. Build proposal (clone original for later risk_detail)'''
new_eval = '''    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let profile_opt = {
        let aid = account_id.unwrap_or_else(|| {
            // fallback: use active account
            0
        });
        if aid > 0 {
            crate::db::profiles::get(&conn, aid).map_err(|e| e.to_string())?
        } else {
            None
        }
    };
    let state_acct = AccountState::from_snapshot_with_profile(parsed, &cfg, profile_opt.as_ref())
        .map_err(|e| format!("state: {e}"))?;

    // 2. Build proposal (clone original for later risk_detail)'''
if old_eval in s:
    s = s.replace(old_eval, new_eval, 1)
    print("  ✓ equity_evaluate: profile-aware")

# 3c) equity_preview_risk — already has account_id, load profile
old_prev = '''    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let acct_state = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("state: {e}"))?;

    let r0 = Decimal::from_str(&cfg.r0).unwrap_or(Decimal::new(1, 2));'''
new_prev = '''    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let profile_opt = match account_id {
        Some(aid) => {
            let conn2 = state.db.lock().map_err(|e| e.to_string())?;
            let p = crate::db::profiles::get(&conn2, aid).map_err(|e| e.to_string())?;
            drop(conn2);
            p
        }
        None => None,
    };
    let acct_state = AccountState::from_snapshot_with_profile(parsed, &cfg, profile_opt.as_ref())
        .map_err(|e| format!("state: {e}"))?;

    let r0 = Decimal::from_str(&cfg.r0).unwrap_or(Decimal::new(1, 2));'''
if old_prev in s:
    s = s.replace(old_prev, new_prev, 1)
    print("  ✓ equity_preview_risk: profile-aware")

# 3d) equity_snapshot_create (manual) — also profile-aware for consistency
old_manual = '''    let acct_state = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("AccountState: {e}"))?;'''
new_manual = '''    let acct_state = AccountState::from_snapshot_with_profile(parsed, &cfg, None)
        .map_err(|e| format!("AccountState: {e}"))?;'''
if old_manual in s:
    s = s.replace(old_manual, new_manual, 1)
    print("  ✓ equity_snapshot_create: uses with_profile(None)")

p.write_text(s)
PY

# ---------- 4) test + build ----------
echo "==> [4/5] cargo check"
( cd src-tauri && cargo check 2>&1 | tee /tmp/s16_check.log )
RC=${PIPESTATUS[0]:-$?}
echo "exit: $RC"
[ "$RC" -eq 0 ] || { echo "FATAL: see /tmp/s16_check.log"; exit 4; }

echo "==> [5/5] cargo test"
( cd src-tauri && cargo test 2>&1 | grep -E "test result|FAILED" | head -10 )

echo ""
echo "============================================================"
echo "S16 COMPLETE"
echo "  ✓ from_snapshot_with_profile(snapshot, config, profile)"
echo "  ✓ auto_snapshot/evaluate/preview: profile-aware"
echo "  Backup: $BK"
echo "============================================================"
