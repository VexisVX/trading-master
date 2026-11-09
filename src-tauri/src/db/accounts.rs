use super::DbError;
use crate::domain::account::{Account, AccountKind, NewAccount};
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;

fn dec(s: &str) -> Result<Decimal, DbError> {
    Decimal::from_str_exact(s).map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))
}

pub fn create_account(conn: &Connection, a: &NewAccount) -> Result<i64, DbError> {
    if a.name.trim().is_empty() {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if a.initial_balance <= Decimal::ZERO {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    let tx = conn.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();

    tx.execute(
        "INSERT INTO accounts (name, currency, initial_balance, is_active, kind, created_at)
         VALUES (?1,?2,?3,1,?4,?5)",
        params![
            a.name.trim(),
            a.currency,
            a.initial_balance.to_string(),
            a.kind.as_str(),
            now
        ],
    )?;
    let id = tx.last_insert_rowid();

    // Seed risk settings for every account.
    tx.execute(
        "INSERT INTO risk_settings (account_id, updated_at) VALUES (?1, ?2)",
        params![id, now],
    )?;

    // Seed prop-firm rules only for prop-firm accounts.
    if a.kind.is_prop_firm() {
        tx.execute(
            "INSERT INTO prop_firm_rules (account_id, enabled, updated_at) VALUES (?1, 1, ?2)",
            params![id, now],
        )?;
    }

    tx.commit()?;
    Ok(id)
}

pub fn get_account(conn: &Connection, id: i64) -> Result<Option<Account>, DbError> {
    let row = conn
        .query_row(
            "SELECT id, name, currency, initial_balance, is_active, kind, created_at
         FROM accounts WHERE id=?1",
            [id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                ))
            },
        )
        .optional()?;

    let Some((id, name, currency, bal, active, kind_s, created)) = row else {
        return Ok(None);
    };
    Ok(Some(Account {
        id,
        name,
        currency,
        initial_balance: dec(&bal)?,
        is_active: active != 0,
        kind: AccountKind::from_str(&kind_s).unwrap_or(AccountKind::Personal),
        created_at: created,
    }))
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<Account>, DbError> {
    let mut stmt = conn.prepare("SELECT id FROM accounts ORDER BY id ASC")?;
    let ids: Vec<i64> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(a) = get_account(conn, id)? {
            out.push(a);
        }
    }
    Ok(out)
}

pub fn delete_account(conn: &Connection, id: i64) -> Result<(), DbError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM accounts", [], |r| r.get(0))?;
    if count <= 1 {
        return Err(DbError::CannotDeleteLastAccount);
    }

    // Ensure write-mode; a previous read-only open can persist a flag.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = OFF;")?;

    let tx = conn.unchecked_transaction()?;

    // Explicit child-first deletes — do not rely on FK cascade.
    tx.execute(
        "DELETE FROM trade_legs WHERE trade_id IN (SELECT id FROM trades WHERE account_id=?1)",
        [id],
    )?;
    tx.execute("DELETE FROM trade_attachments WHERE trade_id IN (SELECT id FROM trades WHERE account_id=?1)", [id])?;
    tx.execute(
        "DELETE FROM trade_tags WHERE trade_id IN (SELECT id FROM trades WHERE account_id=?1)",
        [id],
    )?;
    tx.execute("DELETE FROM trades WHERE account_id=?1", [id])?;
    tx.execute("DELETE FROM prop_firm_rules WHERE account_id=?1", [id])?;
    tx.execute("DELETE FROM risk_settings WHERE account_id=?1", [id])?;
    tx.execute("DELETE FROM accounts WHERE id=?1", [id])?;

    // Repoint active_account_id if needed.
    let active: Option<String> = tx
        .query_row(
            "SELECT value FROM app_meta WHERE key='active_account_id'",
            [],
            |r| r.get(0),
        )
        .ok();
    if active.as_deref() == Some(&id.to_string()) {
        let first: i64 = tx.query_row("SELECT id FROM accounts ORDER BY id LIMIT 1", [], |r| {
            r.get(0)
        })?;
        tx.execute(
            "INSERT INTO app_meta (key, value) VALUES ('active_account_id', ?1)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [first.to_string()],
        )?;
    }

    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE accounts (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               name TEXT, currency TEXT, initial_balance TEXT,
               is_active INTEGER, kind TEXT DEFAULT 'personal', created_at TEXT
             );
             CREATE TABLE risk_settings (
               account_id INTEGER PRIMARY KEY, max_risk_per_trade_pct TEXT DEFAULT '1',
               daily_loss_limit_pct TEXT DEFAULT '3', max_open_positions INTEGER DEFAULT 3,
               max_portfolio_risk_pct TEXT DEFAULT '5', updated_at TEXT
             );
             CREATE TABLE prop_firm_rules (
               account_id INTEGER PRIMARY KEY, enabled INTEGER DEFAULT 1,
               max_dd_pct TEXT DEFAULT '10', trailing_dd INTEGER DEFAULT 1,
               daily_loss_limit_pct TEXT DEFAULT '5', max_total_loss_pct TEXT DEFAULT '10',
               profit_target_pct TEXT DEFAULT '8', min_trading_days INTEGER DEFAULT 5,
               updated_at TEXT
             );
             CREATE TABLE trades (id INTEGER PRIMARY KEY, account_id INTEGER);
             CREATE TABLE trade_legs (id INTEGER PRIMARY KEY, trade_id INTEGER);
             CREATE TABLE app_meta (key TEXT PRIMARY KEY, value TEXT);",
        )
        .unwrap();
        conn
    }

    #[test]
    fn create_personal_seeds_only_risk_settings() {
        let conn = fresh_db();
        let id = create_account(
            &conn,
            &NewAccount {
                name: "My Personal".into(),
                currency: "USD".into(),
                initial_balance: dec!(10000),
                kind: AccountKind::Personal,
            },
        )
        .unwrap();
        let rs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM risk_settings WHERE account_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        let pf: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prop_firm_rules WHERE account_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rs, 1);
        assert_eq!(pf, 0);
    }

    #[test]
    fn create_prop_firm_seeds_both() {
        let conn = fresh_db();
        let id = create_account(
            &conn,
            &NewAccount {
                name: "FTMO".into(),
                currency: "USD".into(),
                initial_balance: dec!(100000),
                kind: AccountKind::PropFirm,
            },
        )
        .unwrap();
        let rs: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM risk_settings WHERE account_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        let pf: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prop_firm_rules WHERE account_id=?1",
                [id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rs, 1);
        assert_eq!(pf, 1);
    }

    #[test]
    fn reject_empty_name_and_zero_balance() {
        let conn = fresh_db();
        assert!(create_account(
            &conn,
            &NewAccount {
                name: " ".into(),
                currency: "USD".into(),
                initial_balance: dec!(10000),
                kind: AccountKind::Personal,
            }
        )
        .is_err());
        assert!(create_account(
            &conn,
            &NewAccount {
                name: "X".into(),
                currency: "USD".into(),
                initial_balance: dec!(0),
                kind: AccountKind::Personal,
            }
        )
        .is_err());
    }

    #[test]
    fn refuse_delete_last_account() {
        let conn = fresh_db();
        let id = create_account(
            &conn,
            &NewAccount {
                name: "Only".into(),
                currency: "USD".into(),
                initial_balance: dec!(10000),
                kind: AccountKind::Personal,
            },
        )
        .unwrap();
        assert!(delete_account(&conn, id).is_err());
    }

    #[test]
    fn list_returns_all() {
        let conn = fresh_db();
        let a = create_account(
            &conn,
            &NewAccount {
                name: "A".into(),
                currency: "USD".into(),
                initial_balance: dec!(10000),
                kind: AccountKind::Personal,
            },
        )
        .unwrap();
        let b = create_account(
            &conn,
            &NewAccount {
                name: "B".into(),
                currency: "USD".into(),
                initial_balance: dec!(50000),
                kind: AccountKind::PropFirm,
            },
        )
        .unwrap();
        let list = list_accounts(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, a);
        assert_eq!(list[1].id, b);
    }
}
