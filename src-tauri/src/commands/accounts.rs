use crate::commands::AppState;
use crate::db::accounts as repo;
use crate::db::get_meta;
use crate::domain::account::{Account, NewAccount};
use rusqlite::params;
use tauri::State;

#[tauri::command]
pub fn accounts_list(state: State<AppState>) -> Result<Vec<Account>, String> {
    use rust_decimal::prelude::FromStr;
    use rust_decimal::Decimal;

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Read active account from app_meta (source of truth)
    let active_id: Option<i64> = get_meta(&conn, "active_account_id")
        .map_err(|e| e.to_string())?
        .and_then(|v| v.parse::<i64>().ok());

    let mut stmt = conn
        .prepare(
            "SELECT id, name, currency, initial_balance, is_active, created_at,
                    COALESCE(kind, 'personal') AS kind
             FROM accounts
             ORDER BY id",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |r| {
            let id: i64 = r.get(0)?;
            let name: String = r.get(1)?;
            let currency: String = r.get(2)?;
            let initial_balance_str: String = r.get(3)?;
            let stored_active: i64 = r.get(4)?;
            let created_at: String = r.get(5)?;
            let kind_str: String = r.get(6)?;

            // Parse Decimal from string
            let initial_balance = Decimal::from_str(&initial_balance_str)
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        3,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            // Parse AccountKind from string
            let kind = crate::domain::account::AccountKind::from_str(&kind_str)
                .unwrap_or(crate::domain::account::AccountKind::Personal);

            // is_active = true if matches app_meta active_account_id,
            // else fall back to stored is_active
            let is_active = match active_id {
                Some(aid) => id == aid,
                None => stored_active != 0,
            };

            Ok(Account {
                id,
                name,
                currency,
                initial_balance,
                is_active,
                kind,
                created_at,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rows)
}

#[tauri::command]
pub fn accounts_create(state: State<AppState>, account: NewAccount) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::create_account(&conn, &account).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_account(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accounts_active_get(state: State<AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    if let Some(v) = get_meta(&conn, "active_account_id").map_err(|e| e.to_string())? {
        if let Ok(id) = v.parse::<i64>() {
            return Ok(id);
        }
    }
    let id: i64 = conn
        .query_row("SELECT id FROM accounts ORDER BY id LIMIT 1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn accounts_active_set(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM accounts WHERE id=?1", [id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        return Err("account not found".into());
    }

    // 1) Update app_meta (source of truth for getActiveAccountId)
    conn.execute(
        "INSERT INTO app_meta (key, value) VALUES ('active_account_id', ?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![id.to_string()],
    )
    .map_err(|e| e.to_string())?;

    // 2) Update is_active in accounts (for accounts_list / firstActive fallback)
    conn.execute("UPDATE accounts SET is_active = 0 WHERE id != ?1", [id])
        .map_err(|e| e.to_string())?;
    conn.execute("UPDATE accounts SET is_active = 1 WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================
// accounts_trades_count — برای Edit Modal
// ============================================================
#[tauri::command]
pub fn accounts_trades_count(state: State<AppState>, id: i64) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let n: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM trades WHERE account_id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(n)
}

// ============================================================
// accounts_update — نام همیشه، بقیه فقط اگر معامله ندارد
// ============================================================
#[derive(Debug, serde::Deserialize)]
pub struct UpdateAccountInput {
    pub name: String,
    pub currency: Option<String>,
    pub initial_balance: Option<String>,
    pub kind: Option<String>,
}

#[tauri::command]
pub fn accounts_update(
    state: State<AppState>,
    id: i64,
    update: UpdateAccountInput,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let trades_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM trades WHERE account_id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    // name همیشه
    conn.execute(
        "UPDATE accounts SET name = ?1 WHERE id = ?2",
        rusqlite::params![update.name, id],
    )
    .map_err(|e| e.to_string())?;

    if trades_count > 0 {
        return Ok(());
    }

    if let Some(currency) = update.currency {
        conn.execute(
            "UPDATE accounts SET currency = ?1 WHERE id = ?2",
            rusqlite::params![currency, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(initial) = update.initial_balance {
        use rust_decimal::Decimal;
        use rust_decimal::prelude::FromStr;
        let d = Decimal::from_str(&initial).map_err(|e| format!("initial: {}", e))?;
        if d <= Decimal::ZERO {
            return Err("initial_balance must be > 0".into());
        }
        conn.execute(
            "UPDATE accounts SET initial_balance = ?1 WHERE id = ?2",
            rusqlite::params![initial, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(kind) = update.kind {
        conn.execute(
            "UPDATE accounts SET kind = ?1 WHERE id = ?2",
            rusqlite::params![kind, id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ============================================================
// accounts_reset — حذف trades + snapshotها + decisions
// ============================================================
#[tauri::command]
pub fn accounts_reset(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // 1) snapshot_ids این حساب
    let snapshot_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM equity_snapshots WHERE account_id = ?1")
            .map_err(|e| e.to_string())?;
        let result: Result<Vec<i64>, _> = stmt
            .query_map([id], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect();
        result.map_err(|e| e.to_string())?
    };

    // 2) equity_decisions وابسته به این snapshotها
    for sid in &snapshot_ids {
        conn.execute(
            "DELETE FROM equity_decisions WHERE snapshot_id = ?1",
            [sid],
        )
        .map_err(|e| e.to_string())?;
    }

    // 3) equity_snapshots
    conn.execute(
        "DELETE FROM equity_snapshots WHERE account_id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;

    // 4) trade_ids این حساب
    let trade_ids: Vec<i64> = {
        let mut stmt = conn
            .prepare("SELECT id FROM trades WHERE account_id = ?1")
            .map_err(|e| e.to_string())?;
        let result: Result<Vec<i64>, _> = stmt
            .query_map([id], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect();
        result.map_err(|e| e.to_string())?
    };

    for tid in &trade_ids {
        let _ = conn.execute("DELETE FROM trade_legs WHERE trade_id = ?1", [tid]);
        let _ = conn.execute("DELETE FROM trade_tags WHERE trade_id = ?1", [tid]);
        let _ = conn.execute("DELETE FROM trade_attachments WHERE trade_id = ?1", [tid]);
        let _ = conn.execute("DELETE FROM trade_sl_history WHERE trade_id = ?1", [tid]);
    }

    // 5) trades
    conn.execute("DELETE FROM trades WHERE account_id = ?1", [id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================
// accounts_backup_json — export full account to JSON file
// ============================================================
#[tauri::command]
pub fn accounts_backup_json(state: State<AppState>, id: i64) -> Result<String, String> {
    use std::fs;
    use std::path::PathBuf;

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // 1. Read account
    let (name, currency, initial_balance, kind, created_at): (
        String, String, String, String, String,
    ) = conn
        .query_row(
            "SELECT name, currency, initial_balance, COALESCE(kind, 'personal'), created_at
             FROM accounts WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .map_err(|e| format!("account {}: {}", id, e))?;

    // 2. Risk settings (optional)
    let risk_settings: Option<serde_json::Value> = conn
        .query_row(
            "SELECT max_risk_per_trade_pct, daily_loss_limit_pct, max_open_positions, max_portfolio_risk_pct
             FROM risk_settings WHERE account_id = ?1",
            [id],
            |r| {
                Ok(serde_json::json!({
                    "max_risk_per_trade_pct": r.get::<_, String>(0)?,
                    "daily_loss_limit_pct": r.get::<_, String>(1)?,
                    "max_open_positions": r.get::<_, i64>(2)?,
                    "max_portfolio_risk_pct": r.get::<_, String>(3)?,
                }))
            },
        )
        .ok();

    // 3. Prop firm rules (optional)
    let prop_firm_rules: Option<serde_json::Value> = conn
        .query_row(
            "SELECT enabled, max_dd_pct, trailing_dd, daily_loss_limit_pct,
                    max_total_loss_pct, profit_target_pct, min_trading_days
             FROM prop_firm_rules WHERE account_id = ?1",
            [id],
            |r| {
                Ok(serde_json::json!({
                    "enabled": r.get::<_, i64>(0)? != 0,
                    "max_dd_pct": r.get::<_, String>(1)?,
                    "trailing_dd": r.get::<_, i64>(2)? != 0,
                    "daily_loss_limit_pct": r.get::<_, String>(3)?,
                    "max_total_loss_pct": r.get::<_, String>(4)?,
                    "profit_target_pct": r.get::<_, String>(5)?,
                    "min_trading_days": r.get::<_, i64>(6)?,
                }))
            },
        )
        .ok();

    // 4. Goals (optional)
    let goals: Option<serde_json::Value> = conn
        .query_row(
            "SELECT daily_target_pct, weekly_target_pct, monthly_target_pct
             FROM account_goals WHERE account_id = ?1",
            [id],
            |r| {
                Ok(serde_json::json!({
                    "daily_target_pct": r.get::<_, String>(0)?,
                    "weekly_target_pct": r.get::<_, String>(1)?,
                    "monthly_target_pct": r.get::<_, String>(2)?,
                }))
            },
        )
        .ok();

    // 5. Trades
    let mut trades: Vec<serde_json::Value> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, symbol, side, entry_price, initial_stop, take_profit,
                        total_quantity, status, opened_at, closed_at, outcome, manual_pnl
                 FROM trades WHERE account_id = ?1 ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "symbol": r.get::<_, String>(1)?,
                    "side": r.get::<_, String>(2)?,
                    "entry_price": r.get::<_, String>(3)?,
                    "initial_stop": r.get::<_, String>(4)?,
                    "take_profit": r.get::<_, Option<String>>(5)?,
                    "total_quantity": r.get::<_, String>(6)?,
                    "status": r.get::<_, String>(7)?,
                    "opened_at": r.get::<_, String>(8)?,
                    "closed_at": r.get::<_, Option<String>>(9)?,
                    "outcome": r.get::<_, String>(10)?,
                    "manual_pnl": r.get::<_, Option<String>>(11)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok(v) = row {
                trades.push(v);
            }
        }
    }

    // 6. Snapshots
    let mut snapshots: Vec<serde_json::Value> = Vec::new();
    {
        let mut stmt = conn
            .prepare(
                "SELECT id, equity, initial_balance, high_watermark, low_watermark,
                        day_start_equity, open_portfolio_risk, daily_loss, consecutive_losses,
                        taken_at, phase, protected_profit, equity_floor, loss_capacity
                 FROM equity_snapshots WHERE account_id = ?1 ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([id], |r| {
                Ok(serde_json::json!({
                    "id": r.get::<_, i64>(0)?,
                    "equity": r.get::<_, String>(1)?,
                    "initial_balance": r.get::<_, String>(2)?,
                    "high_watermark": r.get::<_, String>(3)?,
                    "low_watermark": r.get::<_, String>(4)?,
                    "day_start_equity": r.get::<_, String>(5)?,
                    "open_portfolio_risk": r.get::<_, String>(6)?,
                    "daily_loss": r.get::<_, String>(7)?,
                    "consecutive_losses": r.get::<_, i64>(8)?,
                    "taken_at": r.get::<_, String>(9)?,
                    "phase": r.get::<_, Option<String>>(10)?,
                    "protected_profit": r.get::<_, Option<String>>(11)?,
                    "equity_floor": r.get::<_, Option<String>>(12)?,
                    "loss_capacity": r.get::<_, Option<String>>(13)?,
                }))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok(v) = row {
                snapshots.push(v);
            }
        }
    }

    // 7. Build full backup object
    let now = chrono::Utc::now();
    let exported_at = now.to_rfc3339();
    let timestamp = now.format("%Y%m%d_%H%M%S").to_string();

    let backup = serde_json::json!({
        "version": 1,
        "exported_at": exported_at,
        "account": {
            "id": id,
            "name": name,
            "currency": currency,
            "initial_balance": initial_balance,
            "kind": kind,
            "created_at": created_at,
        },
        "risk_settings": risk_settings,
        "prop_firm_rules": prop_firm_rules,
        "goals": goals,
        "trades": trades,
        "snapshots": snapshots,
    });

    let json_str = serde_json::to_string_pretty(&backup)
        .map_err(|e| format!("json: {}", e))?;

    // 8. Write to backups/ folder in app data dir
    // App data dir: ~/.local/share/com.trading.manager/
    let app_data_dir: PathBuf = if let Ok(v) = std::env::var("HOME") {
        PathBuf::from(v).join(".local/share/com.trading.manager")
    } else {
        return Err("cannot determine app data dir".into());
    };
    let backups_dir = app_data_dir.join("backups");
    fs::create_dir_all(&backups_dir)
        .map_err(|e| format!("create backups dir: {}", e))?;

    let safe_name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect();
    let filename = format!("account_{}_{}.json", safe_name, timestamp);
    let filepath = backups_dir.join(&filename);

    fs::write(&filepath, json_str)
        .map_err(|e| format!("write backup: {}", e))?;

    Ok(filepath.to_string_lossy().to_string())
}
