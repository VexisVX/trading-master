use super::DbError;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradingPlan {
    pub id: i64,
    pub strategy_id: i64,
    pub name: String,
    pub checklist_json: String,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn list_plans(conn: &Connection, strategy_id: i64) -> Result<Vec<TradingPlan>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, strategy_id, name, checklist_json, notes, created_at, updated_at
         FROM trading_plans WHERE strategy_id=?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([strategy_id], |r| {
        Ok(TradingPlan {
            id: r.get(0)?,
            strategy_id: r.get(1)?,
            name: r.get(2)?,
            checklist_json: r.get(3)?,
            notes: r.get(4)?,
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

pub fn create_plan(
    conn: &Connection,
    strategy_id: i64,
    name: &str,
    checklist_json: &str,
    notes: Option<&str>,
) -> Result<i64, DbError> {
    if name.trim().is_empty() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO trading_plans (strategy_id, name, checklist_json, notes, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
        params![strategy_id, name.trim(), checklist_json, notes, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_plan(
    conn: &Connection,
    id: i64,
    name: &str,
    checklist_json: &str,
    notes: Option<&str>,
) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE trading_plans SET name=?1, checklist_json=?2, notes=?3, updated_at=?4 WHERE id=?5",
        params![name, checklist_json, notes, now, id],
    )?;
    if affected == 0 {
        return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows));
    }
    Ok(())
}

pub fn delete_plan(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute("DELETE FROM trading_plans WHERE id=?1", [id])?;
    Ok(())
}
