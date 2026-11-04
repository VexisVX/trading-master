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
BK="$BD/pre_phase3a_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"
echo "Backup: $BK"

# ============================================================
# 1) MIGRATION_028 — equity_units baseline for existing accounts
# ============================================================
echo "==> [1/6] MIGRATION_028 — equity_units baseline"
python3 - <<'PY'
import pathlib, sys
p = pathlib.Path("src-tauri/src/db/mod.rs")
s = p.read_text()

if "MIGRATION_028" in s:
    print("  SKIP: MIGRATION_028 already present")
    sys.exit(0)

anchor = 'const MIGRATION_027: &str = r#"'
idx = s.find(anchor)
if idx == -1: raise SystemExit("MIGRATION_027 anchor not found")
end = s.find('"#;', idx + len(anchor))
if end == -1: raise SystemExit("MIGRATION_027 closing not found")
end += 3

new_mig = '''

const MIGRATION_028: &str = r#"
-- PHASE 3: seed equity_units for accounts that don't have one yet.
-- Units = initial_balance (NAV starts at 1.0).
INSERT INTO equity_units (account_id, units, nav, equity, source, taken_at, created_at)
SELECT a.id, a.initial_balance, '1', a.initial_balance, 'init',
       datetime('now'), datetime('now')
FROM accounts a
LEFT JOIN equity_units u ON u.account_id = a.id
WHERE u.id IS NULL;
"#;
'''

s = s[:end] + new_mig + s[end:]
s = s.replace("const CURRENT_VERSION: i64 = 27;", "const CURRENT_VERSION: i64 = 28;")
needle = 'if v < 27 { conn.execute_batch(MIGRATION_027)?; set_version(conn, 27)?; v = 27; }'
if needle in s:
    s = s.replace(needle, needle + '\n    if v < 28 { conn.execute_batch(MIGRATION_028)?; set_version(conn, 28)?; v = 28; }')
p.write_text(s)
print("  ✓ MIGRATION_028 added; CURRENT_VERSION=28")
PY

# ============================================================
# 2) db/equity.rs — helpers for units + cash flows
# ============================================================
echo "==> [2/6] db/equity.rs — units + cashflow helpers"
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/db/equity.rs")
s = p.read_text()

if "pub fn latest_units" in s:
    print("  SKIP: helpers already present")
else:
    add = '''

// ============================================================
// PHASE 3: units + cash-flow helpers
// ============================================================

#[derive(Debug, Clone)]
pub struct EquityUnitsRow {
    pub id: i64,
    pub account_id: i64,
    pub units: String,
    pub nav: String,
    pub equity: String,
    pub source: String,
    pub taken_at: String,
}

pub fn latest_units(conn: &Connection, account_id: i64) -> Result<Option<EquityUnitsRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, units, nav, equity, source, taken_at
         FROM equity_units
         WHERE account_id = ?1
         ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query([account_id])?;
    if let Some(r) = rows.next()? {
        Ok(Some(EquityUnitsRow {
            id: r.get(0)?, account_id: r.get(1)?,
            units: r.get(2)?, nav: r.get(3)?, equity: r.get(4)?,
            source: r.get(5)?, taken_at: r.get(6)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert_units(
    conn: &Connection,
    account_id: i64,
    units: &str,
    nav: &str,
    equity: &str,
    source: &str,
    cash_flow_id: Option<i64>,
) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO equity_units
           (account_id, units, nav, equity, source, cash_flow_id, taken_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        rusqlite::params![account_id, units, nav, equity, source, cash_flow_id, now],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CashFlowRow {
    pub id: i64,
    pub account_id: i64,
    pub kind: String,
    pub amount: String,
    pub currency: String,
    pub occurred_at: String,
    pub nav_before: String,
    pub units_before: String,
    pub units_after: String,
    pub note: Option<String>,
}

pub fn insert_cash_flow(
    conn: &Connection,
    account_id: i64,
    kind: &str,
    amount: &str,
    currency: &str,
    nav_before: &str,
    units_before: &str,
    units_after: &str,
    note: Option<&str>,
) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO account_cash_flows
           (account_id, kind, amount, currency, occurred_at,
            nav_before, units_before, units_after, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        rusqlite::params![
            account_id, kind, amount, currency, now,
            nav_before, units_before, units_after, note, now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_cash_flows(
    conn: &Connection,
    account_id: i64,
    limit: i64,
) -> Result<Vec<CashFlowRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, kind, amount, currency, occurred_at,
                nav_before, units_before, units_after, note
         FROM account_cash_flows
         WHERE account_id = ?1
         ORDER BY id DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map([account_id, limit], |r| Ok(CashFlowRow {
        id: r.get(0)?, account_id: r.get(1)?,
        kind: r.get(2)?, amount: r.get(3)?, currency: r.get(4)?,
        occurred_at: r.get(5)?,
        nav_before: r.get(6)?, units_before: r.get(7)?, units_after: r.get(8)?,
        note: r.get(9)?,
    }))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
'''
    s += add
    p.write_text(s)
    print("  ✓ units + cashflow helpers added")
PY

# ============================================================
# 3) commands/equity.rs — Tauri commands
# ============================================================
echo "==> [3/6] cashflow Tauri commands"
cat > /tmp/p3_cmds.rs <<'RUST_EOF'

// ============================================================
// PHASE 3 — cash flow (deposit / withdraw / fee / list)
// ============================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CashFlowInput {
    pub account_id: i64,
    pub amount: String,
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CashFlowOut {
    pub cash_flow_id: i64,
    pub units_row_id: i64,
    pub nav_before: String,
    pub nav_after: String,
    pub units_before: String,
    pub units_after: String,
    pub amount: String,
    pub kind: String,
}

fn cashflow_deposit_impl(
    conn: &rusqlite::Connection,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    // Load latest units
    let u = crate::db::equity::latest_units(conn, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row — run migration 028 or refresh".to_string())?;

    let units_old = Decimal::from_str(&u.units).map_err(|e| e.to_string())?;
    let nav_before = Decimal::from_str(&u.nav).map_err(|e| e.to_string())?;
    if nav_before.is_zero() {
        return Err("NAV is zero — cannot compute units".into());
    }

    let added_units = amount / nav_before;
    let units_new = units_old + added_units;
    let equity_new = units_new * nav_before;  // NAV invariant

    // Currency from account
    let currency: String = conn
        .query_row("SELECT currency FROM accounts WHERE id = ?1", [input.account_id], |r| r.get(0))
        .map_err(|e| format!("account not found: {e}"))?;

    // Insert cash flow
    let cf_id = crate::db::equity::insert_cash_flow(
        conn,
        input.account_id,
        "deposit",
        &amount.to_string(),
        &currency,
        &nav_before.to_string(),
        &units_old.to_string(),
        &units_new.to_string(),
        input.note.as_deref(),
    ).map_err(|e| e.to_string())?;

    // Insert units
    let u_id = crate::db::equity::insert_units(
        conn,
        input.account_id,
        &units_new.to_string(),
        &nav_before.to_string(),
        &equity_new.to_string(),
        "deposit",
        Some(cf_id),
    ).map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u_id,
        nav_before: nav_before.to_string(),
        nav_after: nav_before.to_string(),
        units_before: units_old.to_string(),
        units_after: units_new.to_string(),
        amount: amount.to_string(),
        kind: "deposit".into(),
    })
}

fn cashflow_withdraw_impl(
    conn: &rusqlite::Connection,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    let u = crate::db::equity::latest_units(conn, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row".to_string())?;

    let units_old = Decimal::from_str(&u.units).map_err(|e| e.to_string())?;
    let nav_before = Decimal::from_str(&u.nav).map_err(|e| e.to_string())?;
    if nav_before.is_zero() {
        return Err("NAV is zero".into());
    }

    let removed_units = amount / nav_before;
    let units_new = units_old - removed_units;
    if units_new <= Decimal::ZERO {
        return Err("withdrawal would zero out units".into());
    }
    let equity_new = units_new * nav_before;

    let currency: String = conn
        .query_row("SELECT currency FROM accounts WHERE id = ?1", [input.account_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let cf_id = crate::db::equity::insert_cash_flow(
        conn,
        input.account_id,
        "withdrawal",
        &amount.to_string(),
        &currency,
        &nav_before.to_string(),
        &units_old.to_string(),
        &units_new.to_string(),
        input.note.as_deref(),
    ).map_err(|e| e.to_string())?;

    let u_id = crate::db::equity::insert_units(
        conn,
        input.account_id,
        &units_new.to_string(),
        &nav_before.to_string(),
        &equity_new.to_string(),
        "withdraw",
        Some(cf_id),
    ).map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u_id,
        nav_before: nav_before.to_string(),
        nav_after: nav_before.to_string(),
        units_before: units_old.to_string(),
        units_after: units_new.to_string(),
        amount: amount.to_string(),
        kind: "withdrawal".into(),
    })
}

#[tauri::command]
pub fn equity_cashflow_deposit(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let r = cashflow_deposit_impl(&tx, input)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(r)
}

#[tauri::command]
pub fn equity_cashflow_withdraw(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let r = cashflow_withdraw_impl(&tx, input)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(r)
}

#[tauri::command]
pub fn equity_cashflow_fee(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    let u = crate::db::equity::latest_units(&tx, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row".to_string())?;

    let units = u.units.clone();
    let nav = u.nav.clone();

    let currency: String = tx
        .query_row("SELECT currency FROM accounts WHERE id = ?1", [input.account_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    // Fee: no units change (NAV units unaffected)
    let cf_id = crate::db::equity::insert_cash_flow(
        &tx,
        input.account_id,
        "fee",
        &amount.to_string(),
        &currency,
        &nav,
        &units,
        &units,  // unchanged
        input.note.as_deref(),
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u.id,
        nav_before: nav.clone(),
        nav_after: nav,
        units_before: units.clone(),
        units_after: units,
        amount: amount.to_string(),
        kind: "fee".into(),
    })
}

#[tauri::command]
pub fn equity_cashflow_list(
    state: State<'_, AppState>,
    account_id: i64,
    limit: Option<i64>,
) -> Result<Vec<crate::db::equity::CashFlowRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(50).clamp(1, 500);
    crate::db::equity::list_cash_flows(&conn, account_id, n)
        .map_err(|e| e.to_string())
}
RUST_EOF

cat /tmp/p3_cmds.rs >> src-tauri/src/commands/equity.rs
echo "  ✓ cashflow commands appended"

# Register in lib.rs
python3 - <<'PY'
import pathlib
p = pathlib.Path("src-tauri/src/lib.rs")
s = p.read_text()

if "equity_cashflow_deposit" not in s:
    anchor = "commands::equity::equity_instruments_list,"
    if anchor in s:
        s = s.replace(
            anchor,
            anchor + "\n            commands::equity::equity_cashflow_deposit,\n            commands::equity::equity_cashflow_withdraw,\n            commands::equity::equity_cashflow_fee,\n            commands::equity::equity_cashflow_list,"
        )
        p.write_text(s)
        print("  ✓ registered in lib.rs")
    else:
        print("  WARN: lib.rs anchor not found")
else:
    print("  SKIP: already registered")
PY

# ============================================================
# 4) equityApi.ts — types + helpers
# ============================================================
echo "==> [4/6] equityApi.ts — cashflow helpers"
python3 - <<'PY'
import pathlib
p = pathlib.Path("src/lib/equityApi.ts")
s = p.read_text()

if "cashflowDeposit" in s:
    print("  SKIP: helpers already present")
else:
    add = '''

/* ============================================================
 * CASH FLOW (PHASE 3)
 * ============================================================ */

export interface CashFlowInput {
  account_id: number;
  amount: string;
  note?: string | null;
}

export interface CashFlowOut {
  cash_flow_id: number;
  units_row_id: number;
  nav_before: string;
  nav_after: string;
  units_before: string;
  units_after: string;
  amount: string;
  kind: string;
}

export interface CashFlowRow {
  id: number;
  account_id: number;
  kind: string;
  amount: string;
  currency: string;
  occurred_at: string;
  nav_before: string;
  units_before: string;
  units_after: string;
  note: string | null;
}

export async function cashflowDeposit(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_deposit", { input });
}

export async function cashflowWithdraw(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_withdraw", { input });
}

export async function cashflowFee(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_fee", { input });
}

export async function listCashFlows(accountId: number, limit = 50): Promise<CashFlowRow[]> {
  return invoke<CashFlowRow[]>("equity_cashflow_list", { accountId, limit });
}
'''
    anchor = "/* ============================================================\n *  DEFAULTS\n * ============================================================ */"
    if anchor in s:
        s = s.replace(anchor, add + "\n" + anchor)
        p.write_text(s)
        print("  ✓ types + helpers added")
    else:
        print("  WARN: DEFAULTS anchor not found")
PY

# ============================================================
# 5) cargo check
# ============================================================
echo "==> [5/6] cargo check"
( cd src-tauri && cargo check 2>&1 | tee /tmp/p3a_check.log | tail -12 )
RC=${PIPESTATUS[0]:-$?}
echo "check exit: $RC"

if [ "$RC" -ne 0 ]; then
    echo ""
    echo "=== Errors ==="
    grep -E "^error" -A5 /tmp/p3a_check.log | head -30
    exit 4
fi

# ============================================================
# 6) npm run build
# ============================================================
echo "==> [6/6] npm run build"
npm run build 2>&1 | tail -8

echo ""
echo "============================================================"
echo "PHASE 3-A COMPLETE"
echo "  ✓ MIGRATION_028: baseline equity_units for accounts"
echo "  ✓ db/equity.rs: units + cashflow helpers"
echo "  ✓ Tauri: deposit/withdraw/fee/list"
echo "  ✓ equityApi.ts: types + helpers"
echo "  Backup: $BK"
echo "  Next: open app once to run migration 028"
echo "============================================================"
