use crate::analytics::{summarize, Summary, TradeRDetail};
use crate::commands::AppState;
use crate::db::trades as trades_db;
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn analytics_summary(
    state: State<AppState>,
    account_id: Option<i64>,
) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Load trades for the specific account (or all if None)
    let trades = trades_db::list_trades(&conn, account_id, 500, 0).map_err(|e| e.to_string())?;

    // Read initial balance from the specified account (or active default)
    let balance_str: String = match account_id {
        Some(aid) => conn
            .query_row(
                "SELECT initial_balance FROM accounts WHERE id = ?1",
                [aid],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "0".to_string()),
        None => conn
            .query_row(
                "SELECT initial_balance FROM accounts WHERE is_active=1 ORDER BY id LIMIT 1",
                [],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| "0".to_string()),
    };

    let initial = Decimal::from_str(&balance_str).unwrap_or_default();
    let summary: Summary = summarize(&trades, initial);

    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn analytics_recent_r(state: State<AppState>, limit: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let trades = trades_db::list_trades(&conn, None, 500, 0).map_err(|e| e.to_string())?;
    let balance_str: String = conn
        .query_row(
            "SELECT initial_balance FROM accounts WHERE is_active=1 ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "0".to_string());
    let initial = Decimal::from_str(&balance_str).unwrap_or_default();
    let summary = summarize(&trades, initial);
    let take = limit.max(1) as usize;
    let recent: Vec<TradeRDetail> = summary.r_details.into_iter().rev().take(take).collect();
    serde_json::to_string(&recent).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn analytics_breakdown(
    state: State<AppState>,
    account_id: i64,
    from_date: Option<String>,
    to_date: Option<String>,
) -> Result<String, String> {
    use crate::analytics::breakdown::compute_breakdown;
    use crate::db::{strategies as strat_db, trades as trades_db};
    use std::collections::HashMap;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let all_trades = trades_db::list_trades(&conn, None, 500, 0).map_err(|e| e.to_string())?;

    let mut trades: Vec<_> = all_trades
        .into_iter()
        .filter(|t| t.account_id == account_id)
        .collect();

    // Apply date range filter on closed_at
    if let Some(from) = &from_date {
        trades = trades
            .into_iter()
            .filter(|t| {
                t.closed_at
                    .as_deref()
                    .map(|c| c.get(0..10).map(|d| d >= from.as_str()).unwrap_or(false))
                    .unwrap_or(false)
            })
            .collect();
    }
    if let Some(to) = &to_date {
        trades = trades
            .into_iter()
            .filter(|t| {
                t.closed_at
                    .as_deref()
                    .map(|c| c.get(0..10).map(|d| d <= to.as_str()).unwrap_or(false))
                    .unwrap_or(false)
            })
            .collect();
    }

    let strategies = strat_db::list_strategies(&conn).map_err(|e| e.to_string())?;
    let mut names: HashMap<i64, String> = HashMap::new();
    for s in strategies {
        names.insert(s.id, s.name);
    }

    let b = compute_breakdown(&trades, &names);
    serde_json::to_string(&b).map_err(|e| e.to_string())
}

// ============================================================
// Period PnL — daily, weekly, monthly (% of initial balance)
// ============================================================

#[derive(Debug, serde::Serialize)]
pub struct PeriodPnl {
    pub daily_pct: String,
    pub weekly_pct: String,
    pub monthly_pct: String,
    pub daily_pnl: String,
    pub weekly_pnl: String,
    pub monthly_pnl: String,
}

#[tauri::command]
pub fn analytics_period_pnl(state: State<AppState>, account_id: i64) -> Result<PeriodPnl, String> {
    use chrono::{Datelike, Utc, Weekday};
    use rust_decimal::prelude::FromStr;

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Read initial balance
    let balance_str: String = conn
        .query_row(
            "SELECT initial_balance FROM accounts WHERE id = ?1",
            [account_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("account {}: {}", account_id, e))?;
    let balance = rust_decimal::Decimal::from_str(&balance_str).unwrap_or_default();
    if balance <= rust_decimal::Decimal::ZERO {
        return Ok(PeriodPnl {
            daily_pct: "0".into(),
            weekly_pct: "0".into(),
            monthly_pct: "0".into(),
            daily_pnl: "0".into(),
            weekly_pnl: "0".into(),
            monthly_pnl: "0".into(),
        });
    }

    // Compute date boundaries
    let now = Utc::now();
    let today_start = now.date_naive().and_hms_opt(0, 0, 0).unwrap();
    let today_start_iso = today_start.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Week start (Monday)
    let weekday = now.weekday().num_days_from_monday();
    let week_start = today_start - chrono::Duration::days(weekday as i64);
    let week_start_iso = week_start.format("%Y-%m-%dT%H:%M:%S").to_string();

    // Month start
    let month_start = now
        .date_naive()
        .with_day(1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let month_start_iso = month_start.format("%Y-%m-%dT%H:%M:%S").to_string();

    let _ = Weekday::Mon; // suppress unused warning

    // Sum PnL in each period from closed trades (manual_pnl)
    let sum_pnl_since = |since_iso: &str| -> Result<rust_decimal::Decimal, String> {
        let mut stmt = conn
            .prepare(
                "SELECT manual_pnl FROM trades
                 WHERE account_id = ?1
                   AND status = 'closed'
                   AND COALESCE(closed_at, updated_at) >= ?2
                   AND manual_pnl IS NOT NULL",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![account_id, since_iso], |r| {
                r.get::<_, Option<String>>(0)
            })
            .map_err(|e| e.to_string())?;
        let mut total = rust_decimal::Decimal::ZERO;
        for row in rows {
            if let Ok(Some(s)) = row {
                if let Ok(d) = rust_decimal::Decimal::from_str(&s) {
                    total += d;
                }
            }
        }
        Ok(total)
    };

    let daily_pnl = sum_pnl_since(&today_start_iso)?;
    let weekly_pnl = sum_pnl_since(&week_start_iso)?;
    let monthly_pnl = sum_pnl_since(&month_start_iso)?;

    let hundred = rust_decimal::Decimal::from(100);
    let daily_pct = (daily_pnl / balance * hundred).to_string();
    let weekly_pct = (weekly_pnl / balance * hundred).to_string();
    let monthly_pct = (monthly_pnl / balance * hundred).to_string();

    Ok(PeriodPnl {
        daily_pct,
        weekly_pct,
        monthly_pct,
        daily_pnl: daily_pnl.to_string(),
        weekly_pnl: weekly_pnl.to_string(),
        monthly_pnl: monthly_pnl.to_string(),
    })
}
