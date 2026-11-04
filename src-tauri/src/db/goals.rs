use super::DbError;
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountGoals {
    pub account_id: i64,
    pub daily_target_pct: String,
    pub weekly_target_pct: String,
    pub monthly_target_pct: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateGoals {
    pub daily_target_pct: String,
    pub weekly_target_pct: String,
    pub monthly_target_pct: String,
}

pub fn get_goals(conn: &Connection, account_id: i64) -> Result<Option<AccountGoals>, DbError> {
    let result = conn
        .query_row(
            "SELECT account_id, daily_target_pct, weekly_target_pct, monthly_target_pct, updated_at
             FROM account_goals WHERE account_id=?1",
            [account_id],
            |r| {
                Ok(AccountGoals {
                    account_id: r.get(0)?,
                    daily_target_pct: r.get(1)?,
                    weekly_target_pct: r.get(2)?,
                    monthly_target_pct: r.get(3)?,
                    updated_at: r.get(4)?,
                })
            },
        )
        .optional()
        .map_err(DbError::from)?;
    Ok(result)
}

pub fn set_goals(conn: &Connection, account_id: i64, g: &UpdateGoals) -> Result<(), DbError> {
    // Validate: targets must be > 0 and <= 100
    let daily = Decimal::from_str(&g.daily_target_pct).unwrap_or(Decimal::ZERO);
    let weekly = Decimal::from_str(&g.weekly_target_pct).unwrap_or(Decimal::ZERO);
    let monthly = Decimal::from_str(&g.monthly_target_pct).unwrap_or(Decimal::ZERO);

    if daily <= Decimal::ZERO || daily > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if weekly <= Decimal::ZERO || weekly > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    if monthly <= Decimal::ZERO || monthly > Decimal::from(100) {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }

    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO account_goals (account_id, daily_target_pct, weekly_target_pct, monthly_target_pct, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(account_id) DO UPDATE SET
             daily_target_pct = excluded.daily_target_pct,
             weekly_target_pct = excluded.weekly_target_pct,
             monthly_target_pct = excluded.monthly_target_pct,
             updated_at = excluded.updated_at",
        params![
            account_id,
            g.daily_target_pct,
            g.weekly_target_pct,
            g.monthly_target_pct,
            now
        ],
    )
    .map_err(DbError::from)?;

    Ok(())
}
