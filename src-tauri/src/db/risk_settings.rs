use super::DbError;
use crate::domain::risk_settings::{RiskSettings, UpdateRiskSettings};
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;

fn dec(s: &str) -> Result<Decimal, DbError> {
    Decimal::from_str_exact(s)
        .map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))
}

pub fn get_settings(conn: &Connection, account_id: i64) -> Result<Option<RiskSettings>, DbError> {
    let row = conn.query_row(
        "SELECT account_id, max_risk_per_trade_pct, daily_loss_limit_pct,
                max_open_positions, max_portfolio_risk_pct, updated_at
         FROM risk_settings WHERE account_id=?1",
        [account_id],
        |r| Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, i64>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
        )),
    ).optional()?;

    let Some((aid, mrt, dll, mop, mpr, upd)) = row else { return Ok(None) };
    Ok(Some(RiskSettings {
        account_id: aid,
        max_risk_per_trade_pct: dec(&mrt)?,
        daily_loss_limit_pct: dec(&dll)?,
        max_open_positions: mop,
        max_portfolio_risk_pct: dec(&mpr)?,
        updated_at: upd,
    }))
}

pub fn update_settings(
    conn: &Connection, account_id: i64, u: &UpdateRiskSettings,
) -> Result<(), DbError> {
    // Basic validation — reject obviously wrong values
    if u.max_risk_per_trade_pct <= Decimal::ZERO || u.max_risk_per_trade_pct > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if u.daily_loss_limit_pct <= Decimal::ZERO || u.daily_loss_limit_pct > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if u.max_open_positions < 1 || u.max_open_positions > 100 {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if u.max_portfolio_risk_pct <= Decimal::ZERO || u.max_portfolio_risk_pct > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }

    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE risk_settings
         SET max_risk_per_trade_pct=?1,
             daily_loss_limit_pct=?2,
             max_open_positions=?3,
             max_portfolio_risk_pct=?4,
             updated_at=?5
         WHERE account_id=?6",
        params![
            u.max_risk_per_trade_pct.to_string(),
            u.daily_loss_limit_pct.to_string(),
            u.max_open_positions,
            u.max_portfolio_risk_pct.to_string(),
            now,
            account_id,
        ],
    )?;
    if affected == 0 {
        // No row: create with defaults + given values
        conn.execute(
            "INSERT INTO risk_settings
             (account_id, max_risk_per_trade_pct, daily_loss_limit_pct,
              max_open_positions, max_portfolio_risk_pct, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                account_id,
                u.max_risk_per_trade_pct.to_string(),
                u.daily_loss_limit_pct.to_string(),
                u.max_open_positions,
                u.max_portfolio_risk_pct.to_string(),
                now,
            ],
        )?;
    }
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
               id INTEGER PRIMARY KEY, name TEXT, currency TEXT,
               initial_balance TEXT, is_active INTEGER, created_at TEXT
             );
             INSERT INTO accounts VALUES (1,'T','USD','10000',1,'2026-01-01');
             CREATE TABLE risk_settings (
               account_id INTEGER PRIMARY KEY,
               max_risk_per_trade_pct TEXT, daily_loss_limit_pct TEXT,
               max_open_positions INTEGER,
               max_portfolio_risk_pct TEXT, updated_at TEXT
             );
             INSERT INTO risk_settings VALUES (1,'1','3',3,'5','2026-10-20');",
        ).unwrap();
        conn
    }

    #[test]
    fn get_defaults() {
        let conn = fresh_db();
        let s = get_settings(&conn, 1).unwrap().unwrap();
        assert_eq!(s.max_risk_per_trade_pct, dec!(1));
        assert_eq!(s.daily_loss_limit_pct, dec!(3));
        assert_eq!(s.max_open_positions, 3);
        assert_eq!(s.max_portfolio_risk_pct, dec!(5));
    }

    #[test]
    fn update_and_read() {
        let conn = fresh_db();
        update_settings(&conn, 1, &UpdateRiskSettings {
            max_risk_per_trade_pct: dec!(0.5),
            daily_loss_limit_pct: dec!(2),
            max_open_positions: 5,
            max_portfolio_risk_pct: dec!(4),
        }).unwrap();
        let s = get_settings(&conn, 1).unwrap().unwrap();
        assert_eq!(s.max_risk_per_trade_pct, dec!(0.5));
        assert_eq!(s.daily_loss_limit_pct, dec!(2));
        assert_eq!(s.max_open_positions, 5);
        assert_eq!(s.max_portfolio_risk_pct, dec!(4));
    }

    #[test]
    fn rejects_invalid_values() {
        let conn = fresh_db();
        let bad = UpdateRiskSettings {
            max_risk_per_trade_pct: dec!(0),
            daily_loss_limit_pct: dec!(3),
            max_open_positions: 3,
            max_portfolio_risk_pct: dec!(5),
        };
        assert!(update_settings(&conn, 1, &bad).is_err());

        let bad2 = UpdateRiskSettings {
            max_risk_per_trade_pct: dec!(1),
            daily_loss_limit_pct: dec!(1),
            max_open_positions: 0,
            max_portfolio_risk_pct: dec!(5),
        };
        assert!(update_settings(&conn, 1, &bad2).is_err());
    }

    #[test]
    fn unknown_account_creates_row() {
        let conn = fresh_db();
        conn.execute("INSERT INTO accounts VALUES (2,'B','USD','5000',1,'2026-01-01')", []).unwrap();
        update_settings(&conn, 2, &UpdateRiskSettings {
            max_risk_per_trade_pct: dec!(2),
            daily_loss_limit_pct: dec!(4),
            max_open_positions: 2,
            max_portfolio_risk_pct: dec!(8),
        }).unwrap();
        let s = get_settings(&conn, 2).unwrap().unwrap();
        assert_eq!(s.max_risk_per_trade_pct, dec!(2));
    }
}
