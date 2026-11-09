//! Generic key-value settings store.

use super::DbError;
use rusqlite::{params, Connection, OptionalExtension};

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, DbError> {
    conn.query_row("SELECT value FROM app_settings WHERE key=?1", [key], |r| {
        r.get(0)
    })
    .optional()
    .map_err(Into::into)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at=excluded.updated_at",
        params![key, value, now],
    )?;
    Ok(())
}

pub fn delete_setting(conn: &Connection, key: &str) -> Result<(), DbError> {
    conn.execute("DELETE FROM app_settings WHERE key=?1", [key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE app_settings (key TEXT PRIMARY KEY, value TEXT NOT NULL, updated_at TEXT NOT NULL);"
        ).unwrap();
        conn
    }

    #[test]
    fn round_trip() {
        let conn = fresh_db();
        set_setting(&conn, "marketaux_api_key", "test123").unwrap();
        assert_eq!(
            get_setting(&conn, "marketaux_api_key").unwrap(),
            Some("test123".into())
        );
    }

    #[test]
    fn update_overwrites() {
        let conn = fresh_db();
        set_setting(&conn, "k", "v1").unwrap();
        set_setting(&conn, "k", "v2").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), Some("v2".into()));
    }

    #[test]
    fn delete_removes() {
        let conn = fresh_db();
        set_setting(&conn, "k", "v").unwrap();
        delete_setting(&conn, "k").unwrap();
        assert_eq!(get_setting(&conn, "k").unwrap(), None);
    }
}
