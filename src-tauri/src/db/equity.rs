//! Equity engine persistence — snapshots, config, decisions.
//! Lives in the MAIN app DB (not backtest.db) since it must see live trades.

use super::DbError;
use rusqlite::Connection;

pub const EQUITY_SCHEMA_VERSION: i64 = 3;

const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS equity_config (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    data         TEXT NOT NULL,         -- JSON-serialized EquityConfig
    updated_at   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS equity_snapshots (
    id                   INTEGER PRIMARY KEY AUTOINCREMENT,
    equity               TEXT NOT NULL,
    initial_balance      TEXT NOT NULL,
    high_watermark       TEXT NOT NULL,
    low_watermark        TEXT NOT NULL,
    day_start_equity     TEXT NOT NULL,
    open_portfolio_risk  TEXT NOT NULL,
    daily_loss           TEXT NOT NULL,
    consecutive_losses   INTEGER NOT NULL,
    taken_at             TEXT NOT NULL,
    created_at           TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_equity_snap_taken ON equity_snapshots(taken_at);

CREATE TABLE IF NOT EXISTS equity_decisions (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id      INTEGER NOT NULL,
    allowed          INTEGER NOT NULL,    -- 0/1
    reason_kind      TEXT NOT NULL,
    reason_detail    TEXT,
    r_allowed        TEXT,                -- money (Decimal)
    size             TEXT,                -- computed size (Decimal, or NULL)
    config_json      TEXT NOT NULL,
    created_at       TEXT NOT NULL,
    FOREIGN KEY (snapshot_id) REFERENCES equity_snapshots(id)
);
CREATE INDEX IF NOT EXISTS idx_equity_dec_created ON equity_decisions(created_at);
"#;

const EQUITY_MIGRATION_002: &str = r#"
-- Units / NAV tracking (audit بند ۳).
CREATE TABLE IF NOT EXISTS equity_units (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  units           TEXT NOT NULL,
  nav             TEXT NOT NULL,
  equity          TEXT NOT NULL,
  source          TEXT NOT NULL CHECK (source IN ('init','deposit','withdraw','trade','adjustment')),
  cash_flow_id    INTEGER REFERENCES account_cash_flows(id),
  taken_at        TEXT NOT NULL,
  created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_units_account_time ON equity_units(account_id, taken_at);
"#;

const EQUITY_MIGRATION_003: &str = r#"
CREATE TABLE IF NOT EXISTS recovery_authorizations (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  authorized_by   TEXT NOT NULL,
  authorized_at   TEXT NOT NULL,
  expires_at      TEXT,
  revoked_at      TEXT,
  reason          TEXT,
  probe_risk_pct  TEXT NOT NULL DEFAULT '0.25',
  created_at      TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_recovery_account ON recovery_authorizations(account_id);
"#;

pub fn init_equity_schema(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(MIGRATION_001)?;
    conn.execute_batch(EQUITY_MIGRATION_002)?;
    conn.execute_batch(EQUITY_MIGRATION_003)?;
    ensure_snapshot_nav_columns(conn)?;
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('equity_schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [EQUITY_SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

// ---------- Config persistence ----------

pub fn load_config(conn: &Connection) -> Result<Option<String>, DbError> {
    let mut stmt = conn.prepare("SELECT data FROM equity_config WHERE id = 1")?;
    let mut rows = stmt.query([])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get::<_, String>(0)?))
    } else {
        Ok(None)
    }
}

pub fn save_config(conn: &Connection, json: &str) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO equity_config (id, data, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET data=excluded.data, updated_at=excluded.updated_at",
        rusqlite::params![json, now],
    )?;
    Ok(())
}

// ---------- Snapshot persistence ----------

pub fn insert_snapshot(conn: &Connection, s: &SnapshotRow) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();

    // Try full insert with the new columns. If they don't exist on older DBs
    // (should not happen after EQUITY_MIGRATION_002 + ensure_snapshot_nav_columns),
    // fall back to the legacy column set.
    let full = conn.execute(
        "INSERT INTO equity_snapshots
            (account_id, equity, initial_balance, high_watermark, low_watermark,
             day_start_equity, open_portfolio_risk, daily_loss,
             consecutive_losses, taken_at, created_at,
             phase, protected_profit, equity_floor, loss_capacity,
             state, buffer_r, m_preserve, f_internal, f_prop)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        rusqlite::params![
            s.account_id,
            s.equity,
            s.initial_balance,
            s.high_watermark,
            s.low_watermark,
            s.day_start_equity,
            s.open_portfolio_risk,
            s.daily_loss,
            s.consecutive_losses,
            s.taken_at,
            now,
            s.phase,
            s.protected_profit,
            s.equity_floor,
            s.loss_capacity,
            s.state,
            s.buffer_r,
            s.m_preserve,
            s.f_internal,
            s.f_prop_val,
        ],
    );

    if full.is_err() {
        // Legacy fallback
        conn.execute(
            "INSERT INTO equity_snapshots
                (account_id, equity, initial_balance, high_watermark, low_watermark,
                 day_start_equity, open_portfolio_risk, daily_loss,
                 consecutive_losses, taken_at, created_at,
                 phase, protected_profit, equity_floor, loss_capacity)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            rusqlite::params![
                s.account_id,
                s.equity,
                s.initial_balance,
                s.high_watermark,
                s.low_watermark,
                s.day_start_equity,
                s.open_portfolio_risk,
                s.daily_loss,
                s.consecutive_losses,
                s.taken_at,
                now,
                s.phase,
                s.protected_profit,
                s.equity_floor,
                s.loss_capacity,
            ],
        )?;
    }
    Ok(conn.last_insert_rowid())
}

pub struct SnapshotRow<'a> {
    pub account_id: Option<i64>,
    pub equity: &'a str,
    pub initial_balance: &'a str,
    pub high_watermark: &'a str,
    pub low_watermark: &'a str,
    pub day_start_equity: &'a str,
    pub open_portfolio_risk: &'a str,
    pub daily_loss: &'a str,
    pub consecutive_losses: i64,
    pub taken_at: &'a str,
    // legacy
    pub phase: Option<&'a str>,
    pub protected_profit: Option<&'a str>,
    pub equity_floor: Option<&'a str>,
    pub loss_capacity: Option<&'a str>,
    // new (S10) — optional so existing callers still compile
    pub state: Option<&'a str>,
    pub buffer_r: Option<&'a str>,
    pub m_preserve: Option<&'a str>,
    pub f_internal: Option<&'a str>,
    pub f_prop_val: Option<&'a str>,
}

pub fn latest_snapshot(
    conn: &Connection,
    account_id: Option<i64>,
) -> Result<Option<EquitySnapshotRow>, DbError> {
    let _ = ensure_snapshot_detail_columns(conn);
    let _ = ensure_snapshot_account_id(conn);

    if let Some(aid) = account_id {
        let mut stmt = conn.prepare(
            "SELECT id, equity, initial_balance, high_watermark, low_watermark,
                    day_start_equity, open_portfolio_risk, daily_loss,
                    consecutive_losses, taken_at,
                    phase, protected_profit, equity_floor, loss_capacity,
                    state, buffer_r, m_preserve, f_internal, f_prop
             FROM equity_snapshots
             WHERE account_id = ?1
             ORDER BY taken_at DESC, id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([aid])?;
        if let Some(r) = rows.next()? {
            return Ok(Some(EquitySnapshotRow {
                id: r.get(0)?,
                equity: r.get(1)?,
                initial_balance: r.get(2)?,
                high_watermark: r.get(3)?,
                low_watermark: r.get(4)?,
                day_start_equity: r.get(5)?,
                open_portfolio_risk: r.get(6)?,
                daily_loss: r.get(7)?,
                consecutive_losses: r.get(8)?,
                taken_at: r.get(9)?,
                phase: r.get(10).ok(),
                protected_profit: r.get(11).ok(),
                equity_floor: r.get(12).ok(),
                loss_capacity: r.get(13).ok(),
                state: r.get(14).ok(),
                buffer_r: r.get(15).ok(),
                m_preserve: r.get(16).ok(),
                f_internal: r.get(17).ok(),
                f_prop_val: r.get(18).ok(),
            }));
        }
        Ok(None)
    } else {
        let mut stmt = conn.prepare(
            "SELECT id, equity, initial_balance, high_watermark, low_watermark,
                    day_start_equity, open_portfolio_risk, daily_loss,
                    consecutive_losses, taken_at,
                    phase, protected_profit, equity_floor, loss_capacity,
                    state, buffer_r, m_preserve, f_internal, f_prop
             FROM equity_snapshots
             ORDER BY taken_at DESC, id DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(r) = rows.next()? {
            return Ok(Some(EquitySnapshotRow {
                id: r.get(0)?,
                equity: r.get(1)?,
                initial_balance: r.get(2)?,
                high_watermark: r.get(3)?,
                low_watermark: r.get(4)?,
                day_start_equity: r.get(5)?,
                open_portfolio_risk: r.get(6)?,
                daily_loss: r.get(7)?,
                consecutive_losses: r.get(8)?,
                taken_at: r.get(9)?,
                phase: r.get(10).ok(),
                protected_profit: r.get(11).ok(),
                equity_floor: r.get(12).ok(),
                loss_capacity: r.get(13).ok(),
                state: r.get(14).ok(),
                buffer_r: r.get(15).ok(),
                m_preserve: r.get(16).ok(),
                f_internal: r.get(17).ok(),
                f_prop_val: r.get(18).ok(),
            }));
        }
        Ok(None)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EquitySnapshotRow {
    pub id: i64,
    pub equity: String,
    pub initial_balance: String,
    pub high_watermark: String,
    pub low_watermark: String,
    pub day_start_equity: String,
    pub open_portfolio_risk: String,
    pub daily_loss: String,
    pub consecutive_losses: i64,
    pub taken_at: String,
    pub phase: Option<String>,
    pub protected_profit: Option<String>,
    pub equity_floor: Option<String>,
    pub loss_capacity: Option<String>,
    // S10 new
    pub state: Option<String>,
    pub buffer_r: Option<String>,
    pub m_preserve: Option<String>,
    pub f_internal: Option<String>,
    pub f_prop_val: Option<String>,
}

// ---------- Decision persistence ----------

pub struct DecisionRow<'a> {
    pub snapshot_id: i64,
    pub allowed: bool,
    pub reason_kind: &'a str,
    pub reason_detail: Option<&'a str>,
    pub r_allowed: Option<&'a str>,
    pub size: Option<&'a str>,
    pub config_json: &'a str,
    pub risk_reduced: bool,
    pub phase: &'a str,
    pub instrument_symbol: Option<&'a str>,
    pub instrument_spec_json: Option<&'a str>,
}

pub fn insert_decision(conn: &Connection, d: &DecisionRow) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO equity_decisions (snapshot_id, allowed, reason_kind, reason_detail, r_allowed, size, config_json, created_at, risk_reduced, phase, instrument_symbol, instrument_spec_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            d.snapshot_id,
            if d.allowed { 1 } else { 0 },
            d.reason_kind,
            d.reason_detail,
            d.r_allowed,
            d.size,
            d.config_json,
            now,
            if d.risk_reduced { 1 } else { 0 },
            d.phase,
            d.instrument_symbol,
            d.instrument_spec_json,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DecisionRowOut {
    pub id: i64,
    pub snapshot_id: i64,
    pub allowed: bool,
    pub reason_kind: String,
    pub reason_detail: Option<String>,
    pub r_allowed: Option<String>,
    pub size: Option<String>,
    pub created_at: String,
    pub risk_reduced: bool,
    pub phase: String,
}

pub fn list_decisions(conn: &Connection, limit: i64) -> Result<Vec<DecisionRowOut>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, snapshot_id, allowed, reason_kind, reason_detail, r_allowed, size, created_at,
                COALESCE(risk_reduced, 0) AS risk_reduced,
                COALESCE(phase, 'normal') AS phase
         FROM equity_decisions ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit], |r| {
        Ok(DecisionRowOut {
            id: r.get(0)?,
            snapshot_id: r.get(1)?,
            allowed: r.get::<_, i64>(2)? != 0,
            reason_kind: r.get(3)?,
            reason_detail: r.get(4)?,
            r_allowed: r.get(5)?,
            size: r.get(6)?,
            created_at: r.get(7)?,
            risk_reduced: r.get::<_, i64>(8)? != 0,
            phase: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ============================================================
// Migration: add snapshot detail columns (idempotent)
// ============================================================
pub fn ensure_snapshot_detail_columns(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    // Get current column names
    let mut existing: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_snapshots)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            existing.push(r?);
        }
    }

    let additions = [
        ("phase", "TEXT"),
        ("protected_profit", "TEXT"),
        ("equity_floor", "TEXT"),
        ("loss_capacity", "TEXT"),
    ];

    for (col, ty) in additions.iter() {
        if !existing.iter().any(|e| e == col) {
            let sql = format!("ALTER TABLE equity_snapshots ADD COLUMN {} {}", col, ty);
            conn.execute(&sql, [])?;
        }
    }
    Ok(())
}
// ============================================================
// Migration: add decision detail columns (idempotent)
// ============================================================
pub fn ensure_decision_detail_columns(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut existing: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_decisions)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            existing.push(r?);
        }
    }

    let additions = [("risk_reduced", "INTEGER"), ("phase", "TEXT")];

    for (col, ty) in additions.iter() {
        if !existing.iter().any(|e| e == col) {
            let sql = format!("ALTER TABLE equity_decisions ADD COLUMN {} {}", col, ty);
            conn.execute(&sql, [])?;
        }
    }
    Ok(())
}
// ============================================================
// Migration: add account_id to equity_snapshots (idempotent)
// ============================================================
pub fn ensure_snapshot_account_id(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut existing: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_snapshots)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            existing.push(r?);
        }
    }
    if !existing.iter().any(|e| e == "account_id") {
        conn.execute(
            "ALTER TABLE equity_snapshots ADD COLUMN account_id INTEGER",
            [],
        )?;
    }
    Ok(())
}

// Updated EquitySnapshotRow with new optional fields
pub struct EquitySnapshotRowExt {
    pub id: i64,
    pub equity: String,
    pub initial_balance: String,
    pub high_watermark: String,
    pub low_watermark: String,
    pub day_start_equity: String,
    pub open_portfolio_risk: String,
    pub daily_loss: String,
    pub consecutive_losses: i64,
    pub taken_at: String,
    pub phase: Option<String>,
    pub protected_profit: Option<String>,
    pub equity_floor: Option<String>,
    pub loss_capacity: Option<String>,
}

// ============================================================
// Migration: add NAV/Units columns to equity_snapshots (idempotent)
// ============================================================
pub fn ensure_snapshot_nav_columns(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut existing: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_snapshots)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            existing.push(r?);
        }
    }
    let additions = [
        ("units", "TEXT"),
        ("nav", "TEXT"),
        ("buffer", "TEXT"),
        ("buffer_r", "TEXT"),
        ("m_preserve", "TEXT"),
        ("f_internal", "TEXT"),
        ("f_prop", "TEXT"),
        ("execution_buffer", "TEXT"),
        ("state", "TEXT"),
    ];
    for (col, ty) in additions.iter() {
        if !existing.iter().any(|e| e == col) {
            conn.execute(
                &format!("ALTER TABLE equity_snapshots ADD COLUMN {} {}", col, ty),
                [],
            )?;
        }
    }
    // decision columns
    let mut dec: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_decisions)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            dec.push(r?);
        }
    }
    for (col, ty) in [
        ("state_from", "TEXT"),
        ("state_to", "TEXT"),
        ("buffer_r_at_decision", "TEXT"),
    ]
    .iter()
    {
        if !dec.iter().any(|e| e == col) {
            conn.execute(
                &format!("ALTER TABLE equity_decisions ADD COLUMN {} {}", col, ty),
                [],
            )?;
        }
    }
    Ok(())
}

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
            id: r.get(0)?,
            account_id: r.get(1)?,
            units: r.get(2)?,
            nav: r.get(3)?,
            equity: r.get(4)?,
            source: r.get(5)?,
            taken_at: r.get(6)?,
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
            account_id,
            kind,
            amount,
            currency,
            now,
            nav_before,
            units_before,
            units_after,
            note,
            now,
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
    let rows = stmt.query_map([account_id, limit], |r| {
        Ok(CashFlowRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            kind: r.get(2)?,
            amount: r.get(3)?,
            currency: r.get(4)?,
            occurred_at: r.get(5)?,
            nav_before: r.get(6)?,
            units_before: r.get(7)?,
            units_after: r.get(8)?,
            note: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn ensure_decision_instrument_columns(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut existing: Vec<String> = Vec::new();
    {
        let mut stmt = conn.prepare("PRAGMA table_info(equity_decisions)")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(1))?;
        for r in rows {
            existing.push(r?);
        }
    }
    for (col, ty) in [
        ("instrument_symbol", "TEXT"),
        ("instrument_spec_json", "TEXT"),
    ]
    .iter()
    {
        if !existing.iter().any(|e| e == col) {
            conn.execute(
                &format!("ALTER TABLE equity_decisions ADD COLUMN {} {}", col, ty),
                [],
            )?;
        }
    }
    Ok(())
}
