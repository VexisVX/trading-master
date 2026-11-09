//! Persistence for Live Risk presets and trade drafts.
//! Payload is stored as JSON string — the shape is defined by the frontend.

use super::DbError;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivePreset {
    pub id: i64,
    pub account_id: i64,
    pub name: String,
    pub instrument: Option<String>,
    pub payload_json: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_presets(conn: &Connection, account_id: i64) -> Result<Vec<LivePreset>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, name, instrument, payload_json, created_at, updated_at
         FROM live_presets WHERE account_id=?1 ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map([account_id], |r| {
        Ok(LivePreset {
            id: r.get(0)?,
            account_id: r.get(1)?,
            name: r.get(2)?,
            instrument: r.get(3)?,
            payload_json: r.get(4)?,
            created_at: r.get(5)?,
            updated_at: r.get(6)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn create_preset(
    conn: &Connection,
    account_id: i64,
    name: &str,
    instrument: Option<&str>,
    payload_json: &str,
) -> Result<i64, DbError> {
    if name.trim().is_empty() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO live_presets (account_id, name, instrument, payload_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![account_id, name.trim(), instrument, payload_json, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn delete_preset(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM live_presets WHERE id=?1", [id])?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDraft {
    pub id: i64,
    pub account_id: i64,
    pub payload_json: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn create_draft(
    conn: &Connection,
    account_id: i64,
    payload_json: &str,
) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO trade_drafts (account_id, payload_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)",
        params![account_id, payload_json, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_draft(conn: &Connection, id: i64) -> Result<Option<TradeDraft>, DbError> {
    conn.query_row(
        "SELECT id, account_id, payload_json, created_at, updated_at FROM trade_drafts WHERE id=?1",
        [id],
        |r| {
            Ok(TradeDraft {
                id: r.get(0)?,
                account_id: r.get(1)?,
                payload_json: r.get(2)?,
                created_at: r.get(3)?,
                updated_at: r.get(4)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn delete_draft(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM trade_drafts WHERE id=?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT, currency TEXT, initial_balance TEXT, is_active INTEGER, kind TEXT DEFAULT 'personal', created_at TEXT);
             INSERT INTO accounts VALUES (1,'T','USD','10000',1,'personal','2026-01-01');
             CREATE TABLE live_presets (id INTEGER PRIMARY KEY AUTOINCREMENT, account_id INTEGER, name TEXT, instrument TEXT, payload_json TEXT, created_at TEXT, updated_at TEXT);
             CREATE TABLE trade_drafts (id INTEGER PRIMARY KEY AUTOINCREMENT, account_id INTEGER, payload_json TEXT, created_at TEXT, updated_at TEXT);",
        ).unwrap();
        conn
    }

    #[test]
    fn preset_round_trip() {
        let conn = fresh_db();
        let id = create_preset(&conn, 1, "My EURUSD", Some("EURUSD"), r#"{"size":"0.5"}"#).unwrap();
        let list = list_presets(&conn, 1).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "My EURUSD");
        assert_eq!(list[0].id, id);
    }

    #[test]
    fn draft_round_trip() {
        let conn = fresh_db();
        let id = create_draft(&conn, 1, r#"{"symbol":"EURUSD"}"#).unwrap();
        let d = get_draft(&conn, id).unwrap().unwrap();
        assert_eq!(d.payload_json, r#"{"symbol":"EURUSD"}"#);
    }

    #[test]
    fn draft_delete() {
        let conn = fresh_db();
        let id = create_draft(&conn, 1, r#"{}"#).unwrap();
        delete_draft(&conn, id).unwrap();
        assert!(get_draft(&conn, id).unwrap().is_none());
    }
}
