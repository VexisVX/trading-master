//! Recovery authorizations persistence (PHASE 5).

use super::DbError;
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct RecoveryRow {
    pub id: i64,
    pub account_id: i64,
    pub authorized_by: String,
    pub authorized_at: String,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
    pub reason: Option<String>,
    pub probe_risk_pct: String,
}

pub fn get_active(conn: &Connection, account_id: i64) -> Result<Option<RecoveryRow>, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT id, account_id, authorized_by, authorized_at, expires_at,
                revoked_at, reason, probe_risk_pct
         FROM recovery_authorizations
         WHERE account_id = ?1
           AND revoked_at IS NULL
           AND (expires_at IS NULL OR expires_at > ?2)
         ORDER BY id DESC LIMIT 1",
    )?;
    let mut rows = stmt.query(rusqlite::params![account_id, now])?;
    if let Some(r) = rows.next()? {
        Ok(Some(RecoveryRow {
            id: r.get(0)?,
            account_id: r.get(1)?,
            authorized_by: r.get(2)?,
            authorized_at: r.get(3)?,
            expires_at: r.get(4).ok(),
            revoked_at: r.get(5).ok(),
            reason: r.get(6).ok(),
            probe_risk_pct: r.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn insert(
    conn: &Connection,
    account_id: i64,
    authorized_by: &str,
    expires_at: Option<&str>,
    reason: Option<&str>,
    probe_risk_pct: &str,
) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO recovery_authorizations
           (account_id, authorized_by, authorized_at, expires_at, reason, probe_risk_pct, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?3)",
        rusqlite::params![account_id, authorized_by, now, expires_at, reason, probe_risk_pct],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn revoke(conn: &Connection, account_id: i64) -> Result<usize, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    let n = conn.execute(
        "UPDATE recovery_authorizations
            SET revoked_at = ?1
          WHERE account_id = ?2 AND revoked_at IS NULL",
        rusqlite::params![now, account_id],
    )?;
    Ok(n)
}
