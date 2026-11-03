use super::DbError;
use crate::domain::strategy::*;
use rusqlite::{params, Connection, OptionalExtension};

// ============================================================
// CREATE
// ============================================================
pub fn create_strategy(conn: &Connection, s: &NewStrategy) -> Result<i64, DbError> {
    if s.name.trim().is_empty() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let now = chrono::Utc::now().to_rfc3339();

    // Phase 1: strategy + timeframes (must succeed)
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO strategies (name, description, version, is_active, created_at, updated_at)
         VALUES (?1, ?2, 1, 1, ?3, ?3)",
        params![s.name, s.description, now],
    )?;
    let strategy_id = tx.last_insert_rowid();

    for (i, tf) in s.timeframes.iter().enumerate() {
        let fields_json = serde_json::to_string(&tf.fields)
            .map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))?;
        tx.execute(
            "INSERT INTO strategy_timeframes (strategy_id, timeframe, fields_json, order_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![strategy_id, tf.timeframe, fields_json, i as i64],
        )?;
    }
    tx.commit()?;

    // Phase 2: version snapshot (best effort, do not fail create)
    let _ = save_snapshot(conn, strategy_id, 1, &s.name, s.description.as_deref(), &s.timeframes);

    Ok(strategy_id)
}

// ============================================================
// READ
// ============================================================
pub fn get_strategy(conn: &Connection, id: i64) -> Result<Option<Strategy>, DbError> {
    let row = conn.query_row(
        "SELECT id, name, description, version, is_active, created_at, updated_at
         FROM strategies WHERE id=?1",
        [id],
        |r| Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, i64>(3)?,
            r.get::<_, i64>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, String>(6)?,
        )),
    ).optional()?;

    let Some((sid, name, description, version, active, created, updated)) = row else {
        return Ok(None);
    };

    let mut stmt = conn.prepare(
        "SELECT timeframe, fields_json FROM strategy_timeframes
         WHERE strategy_id=?1 ORDER BY order_index",
    )?;
    let rows = stmt.query_map([sid], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;

    let mut timeframes = Vec::new();
    for row in rows {
        let (tf_name, fields_s) = row?;
        let fields: Vec<StrategyField> = serde_json::from_str(&fields_s).unwrap_or_default();
        timeframes.push(StrategyTimeframe { timeframe: tf_name, fields });
    }

    Ok(Some(Strategy {
        id: sid, name, description, version,
        is_active: active != 0,
        created_at: created, updated_at: updated,
        timeframes,
    }))
}

pub fn list_strategies(conn: &Connection) -> Result<Vec<Strategy>, DbError> {
    let mut stmt = conn.prepare("SELECT id FROM strategies ORDER BY id DESC")?;
    let ids: Vec<i64> = stmt.query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(s) = get_strategy(conn, id)? { out.push(s); }
    }
    Ok(out)
}

// ============================================================
// UPDATE
// ============================================================
pub fn update_strategy(conn: &Connection, id: i64, s: &NewStrategy) -> Result<(), DbError> {
    if s.name.trim().is_empty() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let now = chrono::Utc::now().to_rfc3339();

    let tx = conn.unchecked_transaction()?;

    let affected = tx.execute(
        "UPDATE strategies
         SET name=?1, description=?2, version=version+1, updated_at=?3
         WHERE id=?4",
        params![s.name, s.description, now, id],
    )?;
    if affected == 0 {
        return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
    }

    tx.execute("DELETE FROM strategy_timeframes WHERE strategy_id=?1", [id])?;
    for (i, tf) in s.timeframes.iter().enumerate() {
        let fields_json = serde_json::to_string(&tf.fields)
            .map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))?;
        tx.execute(
            "INSERT INTO strategy_timeframes (strategy_id, timeframe, fields_json, order_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, tf.timeframe, fields_json, i as i64],
        )?;
    }

    let new_version: i64 = tx.query_row(
        "SELECT version FROM strategies WHERE id=?1", [id], |r| r.get(0),
    )?;

    tx.commit()?;

    // Snapshot (best effort)
    let _ = save_snapshot(conn, id, new_version, &s.name, s.description.as_deref(), &s.timeframes);

    Ok(())
}

// ============================================================
// DELETE
// ============================================================
pub fn delete_strategy(conn: &Connection, id: i64) -> Result<(), DbError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM strategy_timeframes WHERE strategy_id=?1", [id])?;
    tx.execute("DELETE FROM strategy_versions WHERE strategy_id=?1", [id])?;
    tx.execute("DELETE FROM strategies WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(())
}

// ============================================================
// VERSION SNAPSHOTS
// ============================================================
fn save_snapshot(
    conn: &Connection, strategy_id: i64, version: i64,
    name: &str, description: Option<&str>, timeframes: &[StrategyTimeframe],
) -> Result<(), DbError> {
    let json = serde_json::to_string(timeframes)
        .map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))?;
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO strategy_versions (strategy_id, version, name, description, timeframes_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![strategy_id, version, name, description, json, now],
    )?;
    Ok(())
}

pub fn list_versions(conn: &Connection, strategy_id: i64) -> Result<Vec<(i64, i64, String, Option<String>, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT version, id, name, description, created_at FROM strategy_versions
         WHERE strategy_id=?1 ORDER BY version DESC"
    )?;
    let rows = stmt.query_map([strategy_id], |r| Ok((
        r.get::<_, i64>(0)?,
        r.get::<_, i64>(1)?,
        r.get::<_, String>(2)?,
        r.get::<_, Option<String>>(3)?,
        r.get::<_, String>(4)?,
    )))?;
    let mut out = Vec::new();
    for row in rows { out.push(row?); }
    Ok(out)
}

pub fn get_version_json(conn: &Connection, version_id: i64) -> Result<Option<String>, DbError> {
    conn.query_row(
        "SELECT timeframes_json FROM strategy_versions WHERE id=?1",
        [version_id],
        |r| r.get(0),
    ).optional().map_err(Into::into)
}

// ============================================================
// TESTS
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use crate::db::trades as trades_db;
    use crate::domain::trade::*;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT, currency TEXT, initial_balance TEXT, is_active INTEGER, kind TEXT DEFAULT 'personal', created_at TEXT);
             INSERT INTO accounts VALUES (1,'T','USD','10000',1,'personal','2026-01-01');
             CREATE TABLE trades (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id INTEGER, symbol TEXT, side TEXT,
               entry_price TEXT, initial_stop TEXT, take_profit TEXT,
               total_quantity TEXT, original_quantity TEXT, outcome TEXT DEFAULT 'open',
               size_type TEXT DEFAULT 'lots', size_value TEXT, size_unit TEXT,
               manual_pnl TEXT,
               status TEXT, strategy_id INTEGER,
               opened_at TEXT, closed_at TEXT, notes TEXT,
               chart_url TEXT, screenshot_path TEXT,
               created_at TEXT, updated_at TEXT
             );
             CREATE TABLE trade_legs (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, kind TEXT, price TEXT, quantity TEXT, executed_at TEXT, cost_per_unit TEXT DEFAULT '0');
             CREATE TABLE trade_tags (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, tag TEXT);
             CREATE TABLE trade_attachments (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, filename TEXT, kind TEXT, created_at TEXT);
             CREATE TABLE strategies (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT, description TEXT, version INTEGER DEFAULT 1, is_active INTEGER DEFAULT 1, created_at TEXT, updated_at TEXT);
             CREATE TABLE strategy_timeframes (id INTEGER PRIMARY KEY AUTOINCREMENT, strategy_id INTEGER, timeframe TEXT, fields_json TEXT, order_index INTEGER);
             CREATE TABLE strategy_versions (id INTEGER PRIMARY KEY AUTOINCREMENT, strategy_id INTEGER, version INTEGER, name TEXT, description TEXT, timeframes_json TEXT, created_at TEXT);",
        ).unwrap();
        conn
    }

    fn sample() -> NewStrategy {
        NewStrategy {
            name: "ICT Silver Bullet".into(),
            description: Some("Liquidity + POI".into()),
            timeframes: vec![
                StrategyTimeframe {
                    timeframe: "4H".into(),
                    fields: vec![
                        StrategyField {
                            key: "bias".into(), label: "Bias".into(),
                            kind: FieldType::Select,
                            options: vec!["Bullish".into(), "Bearish".into(), "Neutral".into()],
                            required: true,
                        },
                    ],
                },
                StrategyTimeframe {
                    timeframe: "5M".into(),
                    fields: vec![StrategyField {
                        key: "entry".into(), label: "Entry confirm".into(),
                        kind: FieldType::Text, options: vec![], required: true,
                    }],
                },
            ],
        }
    }

    #[test]
    fn create_and_read() {
        let conn = fresh_db();
        let id = create_strategy(&conn, &sample()).unwrap();
        let s = get_strategy(&conn, id).unwrap().unwrap();
        assert_eq!(s.name, "ICT Silver Bullet");
        assert_eq!(s.timeframes.len(), 2);
    }

    #[test]
    fn create_saves_versions() {
        let conn = fresh_db();
        let id = create_strategy(&conn, &sample()).unwrap();
        let versions = list_versions(&conn, id).unwrap();
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].0, 1);
    }

    #[test]
    fn update_bumps_version() {
        let conn = fresh_db();
        let id = create_strategy(&conn, &sample()).unwrap();
        let before = get_strategy(&conn, id).unwrap().unwrap();
        assert_eq!(before.version, 1);

        let mut updated = sample();
        updated.name = "Renamed".into();
        update_strategy(&conn, id, &updated).unwrap();

        let after = get_strategy(&conn, id).unwrap().unwrap();
        assert_eq!(after.name, "Renamed");
        assert_eq!(after.version, 2);

        let versions = list_versions(&conn, id).unwrap();
        assert_eq!(versions.len(), 2);
    }

    #[test]
    fn update_unknown_fails() {
        let conn = fresh_db();
        assert!(update_strategy(&conn, 999, &sample()).is_err());
    }

    #[test]
    fn delete_cascades() {
        let conn = fresh_db();
        let id = create_strategy(&conn, &sample()).unwrap();
        delete_strategy(&conn, id).unwrap();
        let after = get_strategy(&conn, id).unwrap();
        assert!(after.is_none());
    }
}
