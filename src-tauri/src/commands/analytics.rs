use crate::analytics::{summarize, Summary, TradeRDetail};
use crate::commands::AppState;
use crate::db::trades as trades_db;
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn analytics_summary(state: State<AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Load all trades (200 is plenty for early use)
    let trades = trades_db::list_trades(&conn, 500, 0).map_err(|e| e.to_string())?;

    // Read initial balance from the default account
    let balance_str: String = conn
        .query_row(
            "SELECT initial_balance FROM accounts WHERE is_active=1 ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "0".to_string());

    let initial = Decimal::from_str(&balance_str).unwrap_or_default();
    let summary: Summary = summarize(&trades, initial);

    serde_json::to_string(&summary).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn analytics_recent_r(state: State<AppState>, limit: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let trades = trades_db::list_trades(&conn, 500, 0).map_err(|e| e.to_string())?;
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
    state: State<AppState>, account_id: i64,
    from_date: Option<String>, to_date: Option<String>,
) -> Result<String, String> {
    use crate::analytics::breakdown::compute_breakdown;
    use crate::db::{strategies as strat_db, trades as trades_db};
    use std::collections::HashMap;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let all_trades = trades_db::list_trades(&conn, 500, 0).map_err(|e| e.to_string())?;

    let mut trades: Vec<_> = all_trades.into_iter().filter(|t| t.account_id == account_id).collect();

    // Apply date range filter on closed_at
    if let Some(from) = &from_date {
        trades = trades.into_iter().filter(|t| {
            t.closed_at.as_deref().map(|c| c.get(0..10).map(|d| d >= from.as_str()).unwrap_or(false)).unwrap_or(false)
        }).collect();
    }
    if let Some(to) = &to_date {
        trades = trades.into_iter().filter(|t| {
            t.closed_at.as_deref().map(|c| c.get(0..10).map(|d| d <= to.as_str()).unwrap_or(false)).unwrap_or(false)
        }).collect();
    }

    let strategies = strat_db::list_strategies(&conn).map_err(|e| e.to_string())?;
    let mut names: HashMap<i64, String> = HashMap::new();
    for s in strategies { names.insert(s.id, s.name); }

    let b = compute_breakdown(&trades, &names);
    serde_json::to_string(&b).map_err(|e| e.to_string())
}
