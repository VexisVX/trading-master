//! Database layer: connection, migrations, meta helpers.
//!
//! Rules (see ARCHITECTURE.md):
//! - All SQL access from Rust only.
//! - Migrations are versioned constants; never edit a shipped one.
//! - Decimal values are stored as TEXT, never as REAL.

use rusqlite::{Connection, OptionalExtension};
use std::path::Path;
use thiserror::Error;

pub mod accounts;
pub mod goals;
pub mod instruments;
pub mod equity;
pub mod backtest;
pub mod app_settings;
pub mod backup;
pub mod csv_export;

pub mod live_presets;
pub mod prop_firm;
pub mod risk_settings;
pub mod strategies;
pub mod trading_plans;
pub mod trades;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Cannot delete the last account — create another account first.")]
    CannotDeleteLastAccount,
}

pub fn init_db(db_path: &Path) -> Result<Connection, DbError> {
    let conn = Connection::open(db_path)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;
    conn.execute_batch(MIGRATION_001)?;
    run_migrations(&conn, db_path)?;
    // Initialize Equity Engine schema (idempotent).
    equity::init_equity_schema(&conn).map_err(|e| DbError::from(e))?;

    // Ensure equity snapshot detail columns exist (idempotent)
    let _ = equity::ensure_snapshot_detail_columns(&conn);
    ensure_goals_table(&conn)?;
    let _ = equity::ensure_decision_detail_columns(&conn);
    let _ = equity::ensure_snapshot_account_id(&conn);
    Ok(conn)
}

fn current_version(conn: &Connection) -> Result<i64, DbError> {
    let v: Option<String> = conn
        .query_row(
            "SELECT value FROM app_meta WHERE key='schema_version'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(1))
}

fn set_version(conn: &Connection, v: i64) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [v.to_string()],
    )?;
    Ok(())
}

const CURRENT_VERSION: i64 = 22;

fn run_migrations(conn: &Connection, db_path: &Path) -> Result<(), DbError> {
    let mut v = current_version(conn)?;

    if v >= 1 && v < CURRENT_VERSION {
        let backups = backup::backups_dir(db_path);
        let _ = std::fs::create_dir_all(&backups);
        let name = format!("pre-v{}_to_v{}.db", v, CURRENT_VERSION);
        let _ = backup::create_backup(conn, db_path, &backups.join(name));
    }

    if v < 2 { conn.execute_batch(MIGRATION_002)?; set_version(conn, 2)?; v = 2; }
    if v < 3 { conn.execute_batch(MIGRATION_003)?; set_version(conn, 3)?; v = 3; }
    if v < 4 { conn.execute_batch(MIGRATION_004)?; set_version(conn, 4)?; v = 4; }
    if v < 5 { conn.execute_batch(MIGRATION_005)?; set_version(conn, 5)?; v = 5; }
    if v < 6 { conn.execute_batch(MIGRATION_006)?; set_version(conn, 6)?; v = 6; }
    if v < 7 { conn.execute_batch(MIGRATION_007)?; set_version(conn, 7)?; v = 7; }
    if v < 8 { conn.execute_batch(MIGRATION_008)?; set_version(conn, 8)?; v = 8; }
    if v < 9 { conn.execute_batch(MIGRATION_009)?; set_version(conn, 9)?; v = 9; }
    if v < 10 { conn.execute_batch(MIGRATION_010)?; set_version(conn, 10)?; v = 10; }
    if v < 11 { conn.execute_batch(MIGRATION_011)?; set_version(conn, 11)?; v = 11; }
    if v < 12 { conn.execute_batch(MIGRATION_012)?; set_version(conn, 12)?; v = 12; }
    if v < 13 { conn.execute_batch(MIGRATION_013)?; set_version(conn, 13)?; v = 13; }
    if v < 14 { conn.execute_batch(MIGRATION_014)?; set_version(conn, 14)?; v = 14; }
    if v < 15 { conn.execute_batch(MIGRATION_015)?; set_version(conn, 15)?; v = 15; }
    if v < 16 { conn.execute_batch(MIGRATION_016)?; set_version(conn, 16)?; v = 16; }
    if v < 17 { conn.execute_batch(MIGRATION_017)?; set_version(conn, 17)?; v = 17; }
    if v < 18 { conn.execute_batch(MIGRATION_018)?; set_version(conn, 18)?; v = 18; }
    if v < 19 { conn.execute_batch(MIGRATION_019)?; set_version(conn, 19)?; v = 19; }
    if v < 20 { conn.execute_batch(MIGRATION_020)?; set_version(conn, 20)?; v = 20; }
    if v < 21 { conn.execute_batch(MIGRATION_021)?; set_version(conn, 21)?; v = 21; }
    if v < 22 { conn.execute_batch(MIGRATION_022)?; set_version(conn, 22)?; v = 22; }
    let _ = v;

    Ok(())
}

// ============================================================
// Migrations — append only, never modify shipped content.
// ============================================================

const MIGRATION_001: &str = r#"
CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT OR IGNORE INTO app_meta (key, value) VALUES ('schema_version', '1');
"#;

const MIGRATION_002: &str = r#"
CREATE TABLE accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  currency TEXT NOT NULL,
  initial_balance TEXT NOT NULL,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL
);

CREATE TABLE trades (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES accounts(id),
  symbol TEXT NOT NULL,
  side TEXT NOT NULL CHECK (side IN ('long','short')),
  entry_price TEXT NOT NULL,
  initial_stop TEXT NOT NULL,
  total_quantity TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('open','closed')),
  strategy_id INTEGER,
  opened_at TEXT NOT NULL,
  closed_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_trades_account ON trades(account_id);
CREATE INDEX idx_trades_status ON trades(status);
CREATE INDEX idx_trades_opened ON trades(opened_at);

CREATE TABLE trade_legs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trade_id INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('entry','exit')),
  price TEXT NOT NULL,
  quantity TEXT NOT NULL,
  executed_at TEXT NOT NULL,
  cost_per_unit TEXT NOT NULL DEFAULT '0'
);
CREATE INDEX idx_legs_trade ON trade_legs(trade_id);

INSERT INTO accounts (name, currency, initial_balance, is_active, created_at)
VALUES ('Default', 'USD', '10000', 1, datetime('now'));
"#;

const MIGRATION_003: &str = r#"
CREATE TABLE strategies (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  version INTEGER NOT NULL DEFAULT 1,
  is_active INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE strategy_timeframes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
  timeframe TEXT NOT NULL,
  fields_json TEXT NOT NULL,
  order_index INTEGER NOT NULL
);
CREATE INDEX idx_timeframes_strategy ON strategy_timeframes(strategy_id);
"#;

const MIGRATION_004: &str = r#"
CREATE TABLE risk_settings (
  account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  max_risk_per_trade_pct TEXT NOT NULL DEFAULT '1',
  daily_loss_limit_pct   TEXT NOT NULL DEFAULT '3',
  max_open_positions     INTEGER NOT NULL DEFAULT 3,
  max_portfolio_risk_pct TEXT NOT NULL DEFAULT '5',
  updated_at TEXT NOT NULL
);

INSERT INTO risk_settings (account_id, updated_at)
SELECT id, datetime('now') FROM accounts;
"#;

const MIGRATION_005: &str = r#"
CREATE TABLE prop_firm_rules (
  account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  enabled INTEGER NOT NULL DEFAULT 0,
  max_dd_pct TEXT NOT NULL DEFAULT '10',
  trailing_dd INTEGER NOT NULL DEFAULT 1,
  daily_loss_limit_pct TEXT NOT NULL DEFAULT '5',
  max_total_loss_pct TEXT NOT NULL DEFAULT '10',
  profit_target_pct TEXT NOT NULL DEFAULT '8',
  min_trading_days INTEGER NOT NULL DEFAULT 5,
  updated_at TEXT NOT NULL
);

INSERT INTO prop_firm_rules (account_id, updated_at)
SELECT id, datetime('now') FROM accounts;
"#;

const MIGRATION_006: &str = r#"
ALTER TABLE accounts ADD COLUMN kind TEXT NOT NULL DEFAULT 'personal';

INSERT OR IGNORE INTO app_meta (key, value)
SELECT 'active_account_id', CAST(id AS TEXT) FROM accounts ORDER BY id LIMIT 1;
"#;

const MIGRATION_007: &str = r#"
CREATE TABLE IF NOT EXISTS trade_tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trade_id INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
  tag TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tags_trade ON trade_tags(trade_id);
CREATE INDEX IF NOT EXISTS idx_tags_tag ON trade_tags(tag);

CREATE TABLE IF NOT EXISTS trade_attachments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trade_id INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
  filename TEXT NOT NULL,
  kind TEXT NOT NULL DEFAULT 'image',
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_attachments_trade ON trade_attachments(trade_id);
"#;

const MIGRATION_008: &str = r#"
ALTER TABLE trades ADD COLUMN take_profit TEXT;
ALTER TABLE trades ADD COLUMN chart_url TEXT;
ALTER TABLE trades ADD COLUMN screenshot_path TEXT;

DELETE FROM prop_firm_rules
WHERE account_id IN (SELECT id FROM accounts WHERE kind = 'personal');
"#;

const MIGRATION_009: &str = r#"
ALTER TABLE trades ADD COLUMN outcome TEXT NOT NULL DEFAULT 'open';
ALTER TABLE trades ADD COLUMN size_type TEXT NOT NULL DEFAULT 'lots';
ALTER TABLE trades ADD COLUMN size_value TEXT;
ALTER TABLE trades ADD COLUMN size_unit TEXT;

UPDATE trades
SET size_type = 'lots', size_value = total_quantity
WHERE size_value IS NULL;
"#;


const MIGRATION_010: &str = r#"
ALTER TABLE trades ADD COLUMN manual_pnl TEXT;
"#;
const MIGRATION_011: &str = r#"
CREATE TABLE live_presets (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  instrument TEXT,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_live_presets_account ON live_presets(account_id);

CREATE TABLE trade_drafts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX idx_drafts_account ON trade_drafts(account_id);
"#;


const MIGRATION_012: &str = r#"
CREATE TABLE IF NOT EXISTS app_settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"#;

const MIGRATION_013: &str = r#"
ALTER TABLE trades ADD COLUMN original_quantity TEXT;
UPDATE trades SET original_quantity = total_quantity WHERE original_quantity IS NULL;
"#;

const MIGRATION_014: &str = r#"
CREATE TABLE IF NOT EXISTS trade_sl_history (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trade_id INTEGER NOT NULL REFERENCES trades(id) ON DELETE CASCADE,
  old_stop TEXT NOT NULL,
  new_stop TEXT NOT NULL,
  reason TEXT,
  changed_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_sl_trade ON trade_sl_history(trade_id);
"#;

const MIGRATION_015: &str = r#"
CREATE TABLE IF NOT EXISTS strategy_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
  version INTEGER NOT NULL,
  name TEXT NOT NULL,
  description TEXT,
  timeframes_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_versions_strategy ON strategy_versions(strategy_id);
"#;

const MIGRATION_016: &str = r#"
CREATE TABLE IF NOT EXISTS trading_plans (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  strategy_id INTEGER NOT NULL REFERENCES strategies(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  checklist_json TEXT NOT NULL DEFAULT '[]',
  notes TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_plans_strategy ON trading_plans(strategy_id);
"#;

const MIGRATION_017: &str = r#"
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES
  ('ui.theme', 'dark', datetime('now')),
  ('ui.accent', 'turquoise', datetime('now')),
  ('ui.font_size', 'medium', datetime('now')),
  ('ui.density', 'comfortable', datetime('now')),
  ('ui.motion', 'full', datetime('now')),
  ('ui.language', 'en', datetime('now'));
"#;

const MIGRATION_018: &str = r#"
INSERT OR IGNORE INTO app_settings (key, value, updated_at) VALUES
  ('ui.radius', 'soft', datetime('now')),
  ('ui.date_format', 'iso', datetime('now'));
"#;

const MIGRATION_019: &str = r#"
CREATE TABLE IF NOT EXISTS account_goals (
  account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
  daily_target_pct   TEXT NOT NULL DEFAULT '1',
  weekly_target_pct  TEXT NOT NULL DEFAULT '3',
  monthly_target_pct TEXT NOT NULL DEFAULT '8',
  updated_at TEXT NOT NULL
);

INSERT INTO account_goals (account_id, updated_at)
SELECT id, datetime('now') FROM accounts;
"#;

const MIGRATION_020: &str = r#"
-- Immutable cash-flow ledger (audit bند ۵).
-- No UPDATE/DELETE via app logic. Corrections are new rows.
CREATE TABLE IF NOT EXISTS account_cash_flows (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id      INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  kind            TEXT NOT NULL CHECK (kind IN ('deposit','withdrawal','fee','funding','adjustment')),
  amount          TEXT NOT NULL,
  currency        TEXT NOT NULL,
  occurred_at     TEXT NOT NULL,
  nav_before      TEXT NOT NULL,
  units_before    TEXT NOT NULL,
  units_after     TEXT NOT NULL,
  note            TEXT,
  created_at      TEXT NOT NULL,
  CONSTRAINT amount_nonzero CHECK (amount <> '0')
);
CREATE INDEX IF NOT EXISTS idx_cash_flows_account ON account_cash_flows(account_id);
CREATE INDEX IF NOT EXISTS idx_cash_flows_occurred ON account_cash_flows(occurred_at);
"#;

const MIGRATION_021: &str = r#"
CREATE TABLE IF NOT EXISTS prop_firm_rule_sets (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  account_id           INTEGER NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
  firm_name            TEXT NOT NULL,
  enabled              INTEGER NOT NULL DEFAULT 0,
  dd_type              TEXT NOT NULL CHECK (dd_type IN ('static_balance','static_equity','trailing_balance','trailing_equity')),
  dd_amount            TEXT NOT NULL,
  dd_reference         TEXT NOT NULL CHECK (dd_reference IN ('initial_balance','high_watermark','balance','equity')),
  daily_loss_pct       TEXT NOT NULL DEFAULT '0',
  max_total_loss_pct   TEXT NOT NULL DEFAULT '0',
  profit_target_pct    TEXT NOT NULL DEFAULT '0',
  min_trading_days     INTEGER NOT NULL DEFAULT 0,
  reset_tz             TEXT NOT NULL DEFAULT 'UTC',
  reset_time           TEXT NOT NULL DEFAULT '00:00',
  include_cash_flows   INTEGER NOT NULL DEFAULT 1,
  notes                TEXT,
  created_at           TEXT NOT NULL,
  updated_at           TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_prop_rule_sets_account ON prop_firm_rule_sets(account_id);

INSERT INTO prop_firm_rule_sets
  (account_id, firm_name, enabled, dd_type, dd_amount, dd_reference,
   daily_loss_pct, max_total_loss_pct, profit_target_pct, min_trading_days,
   reset_tz, reset_time, include_cash_flows, notes, created_at, updated_at)
SELECT account_id, 'legacy_migration', enabled,
  CASE WHEN trailing_dd=1 THEN 'trailing_equity' ELSE 'static_equity' END,
  max_dd_pct, 'initial_balance',
  daily_loss_limit_pct, max_total_loss_pct, profit_target_pct, min_trading_days,
  'UTC', '00:00', 1, 'migrated from v20', datetime('now'), datetime('now')
FROM prop_firm_rules;
"#;

const MIGRATION_022: &str = r#"
CREATE TABLE IF NOT EXISTS instruments (
  id                    INTEGER PRIMARY KEY AUTOINCREMENT,
  symbol                TEXT NOT NULL UNIQUE,
  currency              TEXT NOT NULL,
  tick_size             TEXT NOT NULL,
  tick_value            TEXT NOT NULL,
  contract_size         TEXT NOT NULL,
  min_qty               TEXT NOT NULL DEFAULT '0',
  max_qty               TEXT NOT NULL DEFAULT '0',
  lot_step              TEXT NOT NULL DEFAULT '0.01',
  commission_per_lot    TEXT NOT NULL DEFAULT '0',
  slippage_per_unit     TEXT NOT NULL DEFAULT '0',
  spec_source           TEXT NOT NULL DEFAULT 'user_entered' CHECK (spec_source IN ('verified','user_entered','estimated')),
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);
"#;




// ============================================================
// Meta helpers
// ============================================================

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>, DbError> {
    let mut stmt = conn.prepare("SELECT value FROM app_meta WHERE key = ?1")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        Ok(Some(row.get(0)?))
    } else {
        Ok(None)
    }
}


// ============================================================
// Ensure account_goals table exists (idempotent)
// ============================================================
pub fn ensure_goals_table(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS account_goals (
            account_id INTEGER PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
            daily_target_pct   TEXT NOT NULL DEFAULT '2',
            weekly_target_pct  TEXT NOT NULL DEFAULT '6',
            monthly_target_pct TEXT NOT NULL DEFAULT '8',
            updated_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}
