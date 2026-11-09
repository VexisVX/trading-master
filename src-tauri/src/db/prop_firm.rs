use super::DbError;
use crate::domain::prop_firm::{PropFirmRules, UpdatePropFirmRules};
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;

fn dec(s: &str) -> Result<Decimal, DbError> {
    Decimal::from_str_exact(s).map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))
}

pub fn get_rules(conn: &Connection, account_id: i64) -> Result<Option<PropFirmRules>, DbError> {
    let row = conn
        .query_row(
            "SELECT account_id, enabled, max_dd_pct, trailing_dd,
                daily_loss_limit_pct, max_total_loss_pct, profit_target_pct,
                min_trading_days, updated_at
         FROM prop_firm_rules WHERE account_id=?1",
            [account_id],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, i64>(7)?,
                    r.get::<_, String>(8)?,
                ))
            },
        )
        .optional()?;

    let Some((aid, en, dd, tr, dl, mtl, pt, md, upd)) = row else {
        return Ok(None);
    };
    Ok(Some(PropFirmRules {
        account_id: aid,
        enabled: en != 0,
        max_dd_pct: dec(&dd)?,
        trailing_dd: tr != 0,
        daily_loss_limit_pct: dec(&dl)?,
        max_total_loss_pct: dec(&mtl)?,
        profit_target_pct: dec(&pt)?,
        min_trading_days: md,
        updated_at: upd,
    }))
}

pub fn update_rules(
    conn: &Connection,
    account_id: i64,
    u: &UpdatePropFirmRules,
) -> Result<(), DbError> {
    // Validate: percentages must be in (0, 100] if enabled
    let valid_pct = |v: Decimal| -> bool { v > Decimal::ZERO && v <= Decimal::from(100) };
    let any_enabled = u.enabled;
    if any_enabled {
        if !valid_pct(u.max_dd_pct) {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
        if u.daily_loss_limit_pct > Decimal::ZERO && !valid_pct(u.daily_loss_limit_pct) {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
        if u.max_total_loss_pct > Decimal::ZERO && !valid_pct(u.max_total_loss_pct) {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
        if u.profit_target_pct > Decimal::ZERO && !valid_pct(u.profit_target_pct) {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
        if u.min_trading_days < 0 || u.min_trading_days > 365 {
            return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();
    let affected = conn.execute(
        "UPDATE prop_firm_rules
         SET enabled=?1, max_dd_pct=?2, trailing_dd=?3,
             daily_loss_limit_pct=?4, max_total_loss_pct=?5,
             profit_target_pct=?6, min_trading_days=?7, updated_at=?8
         WHERE account_id=?9",
        params![
            u.enabled as i64,
            u.max_dd_pct.to_string(),
            u.trailing_dd as i64,
            u.daily_loss_limit_pct.to_string(),
            u.max_total_loss_pct.to_string(),
            u.profit_target_pct.to_string(),
            u.min_trading_days,
            now,
            account_id,
        ],
    )?;
    if affected == 0 {
        conn.execute(
            "INSERT INTO prop_firm_rules
             (account_id, enabled, max_dd_pct, trailing_dd,
              daily_loss_limit_pct, max_total_loss_pct,
              profit_target_pct, min_trading_days, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                account_id,
                u.enabled as i64,
                u.max_dd_pct.to_string(),
                u.trailing_dd as i64,
                u.daily_loss_limit_pct.to_string(),
                u.max_total_loss_pct.to_string(),
                u.profit_target_pct.to_string(),
                u.min_trading_days,
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
             CREATE TABLE prop_firm_rules (
               account_id INTEGER PRIMARY KEY, enabled INTEGER,
               max_dd_pct TEXT, trailing_dd INTEGER,
               daily_loss_limit_pct TEXT, max_total_loss_pct TEXT,
               profit_target_pct TEXT, min_trading_days INTEGER,
               updated_at TEXT
             );
             INSERT INTO prop_firm_rules VALUES (1,0,'10',1,'5','10','8',5,'2026-10-20');",
        )
        .unwrap();
        conn
    }

    #[test]
    fn get_defaults() {
        let conn = fresh_db();
        let r = get_rules(&conn, 1).unwrap().unwrap();
        assert!(!r.enabled);
        assert_eq!(r.max_dd_pct, dec!(10));
        assert!(r.trailing_dd);
        assert_eq!(r.min_trading_days, 5);
    }

    #[test]
    fn update_round_trip() {
        let conn = fresh_db();
        update_rules(
            &conn,
            1,
            &UpdatePropFirmRules {
                enabled: true,
                max_dd_pct: dec!(8),
                trailing_dd: false,
                daily_loss_limit_pct: dec!(4),
                max_total_loss_pct: dec!(6),
                profit_target_pct: dec!(10),
                min_trading_days: 3,
            },
        )
        .unwrap();
        let r = get_rules(&conn, 1).unwrap().unwrap();
        assert!(r.enabled);
        assert_eq!(r.max_dd_pct, dec!(8));
        assert!(!r.trailing_dd);
        assert_eq!(r.min_trading_days, 3);
    }

    #[test]
    fn rejects_invalid_when_enabled() {
        let conn = fresh_db();
        let bad = UpdatePropFirmRules {
            enabled: true,
            max_dd_pct: dec!(0), // invalid
            trailing_dd: true,
            daily_loss_limit_pct: dec!(5),
            max_total_loss_pct: dec!(10),
            profit_target_pct: dec!(8),
            min_trading_days: 5,
        };
        assert!(update_rules(&conn, 1, &bad).is_err());
    }
}
