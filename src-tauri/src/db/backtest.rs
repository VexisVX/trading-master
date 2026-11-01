//! Backtest database — isolated SQLite file, flat trades (no sessions).
//! Schema v2 (sessions removed). Migration from v1 (with sessions) is automatic.
//! Wiping this file is safe.

use super::DbError;
use rusqlite::Connection;
use std::path::Path;

pub const BACKTEST_SCHEMA_VERSION: i64 = 2;

const BT_V2_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS app_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS backtest_trades (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    symbol       TEXT NOT NULL,
    side         TEXT NOT NULL CHECK (side IN ('long','short')),
    entry_price  TEXT NOT NULL,
    stop_price   TEXT NOT NULL,
    take_profit  TEXT,
    exit_price   TEXT,
    outcome      TEXT,
    r_multiple   TEXT,
    setup_tag    TEXT,
    opened_at    TEXT NOT NULL,
    closed_at    TEXT,
    notes        TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_bt_symbol   ON backtest_trades(symbol);
CREATE INDEX IF NOT EXISTS idx_bt_setup    ON backtest_trades(setup_tag);
CREATE INDEX IF NOT EXISTS idx_bt_opened   ON backtest_trades(opened_at);
"#;

/// Open the backtest DB. Creates schema if missing, migrates v1→v2 if needed.
/// Backs up the file once on the first call that finds existing data.
pub fn init_backtest_db(db_path: &Path) -> Result<Connection, DbError> {
    if db_path.exists() {
        let meta = db_path.metadata()?;
        if meta.len() > 0 {
            let bak = db_path.with_extension("db.bak");
            if !bak.exists() {
                let _ = std::fs::copy(db_path, &bak);
            }
        }
    }

    let conn = Connection::open(db_path)?;
    conn.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Detect v1 (with sessions) → migrate.
    let has_sessions: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='backtest_sessions'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);

    if has_sessions {
        migrate_v1_to_v2(&conn)?;
    }

    conn.execute_batch(BT_V2_SCHEMA)?;

    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [BACKTEST_SCHEMA_VERSION.to_string()],
    )?;

    Ok(conn)
}

/// v1 had `backtest_sessions` + `backtest_trades(session_id, ...)`.
/// v2 drops sessions and the `session_id` column. We copy trades into a fresh
/// table and drop the old one. `.bak` was created above.
fn migrate_v1_to_v2(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(
        r#"
        BEGIN;
        CREATE TABLE backtest_trades_v2 (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol       TEXT NOT NULL,
            side         TEXT NOT NULL CHECK (side IN ('long','short')),
            entry_price  TEXT NOT NULL,
            stop_price   TEXT NOT NULL,
            take_profit  TEXT,
            exit_price   TEXT,
            outcome      TEXT,
            r_multiple   TEXT,
            setup_tag    TEXT,
            opened_at    TEXT NOT NULL,
            closed_at    TEXT,
            notes        TEXT,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        );
        INSERT INTO backtest_trades_v2
            (symbol, side, entry_price, stop_price, take_profit, exit_price,
             outcome, r_multiple, setup_tag, opened_at, closed_at, notes,
             created_at, updated_at)
        SELECT symbol, side, entry_price, stop_price, take_profit, exit_price,
               outcome, r_multiple, setup_tag, opened_at, closed_at, notes,
               created_at, updated_at
        FROM backtest_trades;
        DROP TABLE backtest_trades;
        ALTER TABLE backtest_trades_v2 RENAME TO backtest_trades;
        DROP TABLE IF EXISTS backtest_sessions;
        COMMIT;
        "#,
    )?;
    Ok(())
}

// ---------- Row structs ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct BacktestTrade {
    pub id: i64,
    pub symbol: String,
    pub side: String,
    pub entry_price: String,
    pub stop_price: String,
    pub take_profit: Option<String>,
    pub exit_price: Option<String>,
    pub outcome: Option<String>,
    pub r_multiple: Option<String>,
    pub setup_tag: Option<String>,
    pub opened_at: String,
    pub closed_at: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TradeFilter {
    pub symbol: Option<String>,
    pub setup_tag: Option<String>,
    pub outcome: Option<String>,
    pub date_from: Option<String>, // ISO date (inclusive)
    pub date_to: Option<String>,   // ISO date (inclusive)
}

// ---------- Queries ----------

pub fn list_trades(
    conn: &Connection,
    filter: &TradeFilter,
) -> Result<Vec<BacktestTrade>, DbError> {
    let mut sql = String::from(
        "SELECT id, symbol, side, entry_price, stop_price, take_profit,
                exit_price, outcome, r_multiple, setup_tag, opened_at, closed_at, notes
         FROM backtest_trades WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(s) = filter.symbol.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND symbol = ?");
        params.push(Box::new(s.to_string()));
    }
    if let Some(t) = filter.setup_tag.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND setup_tag = ?");
        params.push(Box::new(t.to_string()));
    }
    if let Some(o) = filter.outcome.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND outcome = ?");
        params.push(Box::new(o.to_string()));
    }
    if let Some(d) = filter.date_from.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND substr(opened_at,1,10) >= ?");
        params.push(Box::new(d.to_string()));
    }
    if let Some(d) = filter.date_to.as_deref().filter(|s| !s.is_empty()) {
        sql.push_str(" AND substr(opened_at,1,10) <= ?");
        params.push(Box::new(d.to_string()));
    }
    sql.push_str(" ORDER BY opened_at DESC, id DESC");

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|b| b.as_ref()).collect();
    let rows = stmt.query_map(param_refs.as_slice(), |r| {
        Ok(BacktestTrade {
            id: r.get(0)?, symbol: r.get(1)?, side: r.get(2)?,
            entry_price: r.get(3)?, stop_price: r.get(4)?,
            take_profit: r.get(5)?, exit_price: r.get(6)?,
            outcome: r.get(7)?, r_multiple: r.get(8)?,
            setup_tag: r.get(9)?, opened_at: r.get(10)?,
            closed_at: r.get(11)?, notes: r.get(12)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub struct NewBacktestTrade<'a> {
    pub symbol: &'a str,
    pub side: &'a str,
    pub entry_price: &'a str,
    pub stop_price: &'a str,
    pub take_profit: Option<&'a str>,
    pub exit_price: Option<&'a str>,
    pub outcome: Option<&'a str>,
    pub r_multiple: Option<&'a str>,
    pub setup_tag: Option<&'a str>,
    pub opened_at: &'a str,
    pub closed_at: Option<&'a str>,
    pub notes: Option<&'a str>,
}

pub fn create_trade(conn: &Connection, t: &NewBacktestTrade) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO backtest_trades
         (symbol, side, entry_price, stop_price, take_profit, exit_price,
          outcome, r_multiple, setup_tag, opened_at, closed_at, notes,
          created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)",
        rusqlite::params![
            t.symbol, t.side, t.entry_price, t.stop_price,
            t.take_profit, t.exit_price, t.outcome, t.r_multiple,
            t.setup_tag, t.opened_at, t.closed_at, t.notes, now,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_trade(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM backtest_trades WHERE id = ?1", [id])?;
    Ok(())
}

pub fn delete_all(conn: &Connection) -> Result<(), DbError> {
    conn.execute("DELETE FROM backtest_trades", [])?;
    Ok(())
}

/// Distinct setup tags ordered by frequency (for autocomplete).
pub fn list_setup_tags(conn: &Connection) -> Result<Vec<(String, i64)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT setup_tag, COUNT(*) as n
         FROM backtest_trades
         WHERE setup_tag IS NOT NULL AND setup_tag != ''
         GROUP BY setup_tag
         ORDER BY n DESC, setup_tag ASC
         LIMIT 50",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_symbols(conn: &Connection) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT symbol FROM backtest_trades
         WHERE symbol IS NOT NULL AND symbol != ''
         ORDER BY symbol ASC",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
