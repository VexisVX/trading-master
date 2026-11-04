//! Account profile persistence (per-account, atomic).

use super::DbError;
use rusqlite::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountProfileRow {
    pub account_id: i64,
    pub account_type: String,
    pub profit_target_pct: String,
    pub max_dd_pct: String,
    pub profit_protect_activation: String,
    pub profit_protect_full: String,
    pub max_profit_retention: String,
    pub trailing_dd_pct: String,
}

pub fn upsert(conn: &Connection, r: &AccountProfileRow) -> Result<(), DbError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM accounts WHERE id = ?1", [r.account_id], |row| row.get(0),
    )?;
    if count == 0 {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO account_profiles
           (account_id, account_type, profit_target_pct, max_dd_pct,
            profit_protect_activation, profit_protect_full,
            max_profit_retention, trailing_dd_pct, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(account_id) DO UPDATE SET
           account_type=excluded.account_type,
           profit_target_pct=excluded.profit_target_pct,
           max_dd_pct=excluded.max_dd_pct,
           profit_protect_activation=excluded.profit_protect_activation,
           profit_protect_full=excluded.profit_protect_full,
           max_profit_retention=excluded.max_profit_retention,
           trailing_dd_pct=excluded.trailing_dd_pct,
           updated_at=excluded.updated_at",
        rusqlite::params![
            r.account_id, r.account_type, r.profit_target_pct, r.max_dd_pct,
            r.profit_protect_activation, r.profit_protect_full,
            r.max_profit_retention, r.trailing_dd_pct, now,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, account_id: i64) -> Result<Option<AccountProfileRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT account_id, account_type, profit_target_pct, max_dd_pct,
                profit_protect_activation, profit_protect_full,
                max_profit_retention, trailing_dd_pct
         FROM account_profiles WHERE account_id = ?1",
    )?;
    let mut rows = stmt.query([account_id])?;
    if let Some(r) = rows.next()? {
        Ok(Some(AccountProfileRow {
            account_id: r.get(0)?, account_type: r.get(1)?,
            profit_target_pct: r.get(2)?, max_dd_pct: r.get(3)?,
            profit_protect_activation: r.get(4)?, profit_protect_full: r.get(5)?,
            max_profit_retention: r.get(6)?, trailing_dd_pct: r.get(7)?,
        }))
    } else { Ok(None) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT);
             INSERT INTO accounts VALUES (1, 'A');
             INSERT INTO accounts VALUES (2, 'B');
             CREATE TABLE account_profiles (
               account_id INTEGER PRIMARY KEY,
               account_type TEXT, profit_target_pct TEXT, max_dd_pct TEXT,
               profit_protect_activation TEXT, profit_protect_full TEXT,
               max_profit_retention TEXT, trailing_dd_pct TEXT, updated_at TEXT
             );",
        ).unwrap();
        c
    }

    fn row(aid: i64, target: &str, dd: &str) -> AccountProfileRow {
        AccountProfileRow {
            account_id: aid, account_type: "prop_firm".into(),
            profit_target_pct: target.into(), max_dd_pct: dd.into(),
            profit_protect_activation: "0.02".into(), profit_protect_full: "0.06".into(),
            max_profit_retention: "0.40".into(), trailing_dd_pct: "0.03".into(),
        }
    }

    #[test]
    fn upsert_and_get() {
        let c = fresh();
        upsert(&c, &row(1, "0.10", "0.05")).unwrap();
        assert_eq!(get(&c, 1).unwrap().unwrap().profit_target_pct, "0.10");
    }

    #[test]
    fn account_not_found_rejected() {
        let c = fresh();
        assert!(upsert(&c, &row(999, "0.10", "0.05")).is_err());
    }

    #[test]
    fn multi_account_isolation() {
        let c = fresh();
        upsert(&c, &row(1, "0.10", "0.05")).unwrap();
        upsert(&c, &row(2, "0.06", "0.03")).unwrap();
        assert_eq!(get(&c, 1).unwrap().unwrap().profit_target_pct, "0.10");
        assert_eq!(get(&c, 2).unwrap().unwrap().profit_target_pct, "0.06");
        upsert(&c, &row(1, "0.08", "0.04")).unwrap();
        assert_eq!(get(&c, 1).unwrap().unwrap().profit_target_pct, "0.08");
        assert_eq!(get(&c, 2).unwrap().unwrap().profit_target_pct, "0.06");
    }
}
