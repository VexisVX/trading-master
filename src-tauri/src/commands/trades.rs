use crate::commands::AppState;
use crate::db::trades as repo;
use crate::domain::trade::{NewTrade, Trade, UpdateTrade};
use rust_decimal::Decimal;
use std::str::FromStr;
use tauri::State;

#[tauri::command]
pub fn trades_create(state: State<AppState>, trade: NewTrade) -> Result<i64, String> {
    // Phase 1: create the trade with an explicit scoped lock
    let (id, account_id) = {
        let conn = state.db.lock().map_err(|e| e.to_string())?;
        let id = repo::create_trade(&conn, &trade).map_err(|e| e.to_string())?;
        let acc = trade.account_id;
        (id, acc)
        // conn is dropped HERE, before returning
    };
    // Now the lock is GUARANTEED to be released (Rust drops conn at end of block)
    // before the next lock acquisition inside auto_snapshot.

    // Phase 2: auto-snapshot DISABLED (investigating crash)
    // if let Err(e) = super::equity::equity_auto_snapshot_create(state, Some(account_id)) {
    //     eprintln!("[trades_create] auto-snapshot non-fatal: {}", e);
    // }

    Ok(id)
}

#[tauri::command]
pub fn trades_list(
    state: State<AppState>,
    account_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> Result<Vec<Trade>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_trades(&conn, account_id, limit, offset).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_get(state: State<AppState>, id: i64) -> Result<Option<Trade>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::get_trade(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_add_exit(
    state: State<AppState>,
    trade_id: i64,
    price: String,
    quantity: String,
    cost_per_unit: String,
    executed_at: String,
    outcome: Option<String>,
    manual_pnl: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let p = Decimal::from_str(&price).map_err(|e| format!("price: {e}"))?;
    let q = Decimal::from_str(&quantity).map_err(|e| format!("quantity: {e}"))?;
    let c = Decimal::from_str(&cost_per_unit).map_err(|e| format!("cost: {e}"))?;
    let parsed_outcome = outcome
        .as_deref()
        .and_then(crate::domain::trade::TradeOutcome::from_str);
    let parsed_pnl = match manual_pnl {
        Some(ref s) if !s.trim().is_empty() => {
            Some(Decimal::from_str(s.trim()).map_err(|e| format!("manual_pnl: {e}"))?)
        }
        _ => None,
    };
    repo::add_exit_leg(
        &conn,
        trade_id,
        p,
        q,
        c,
        executed_at,
        parsed_outcome,
        parsed_pnl,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_update(state: State<AppState>, id: i64, update: UpdateTrade) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::update_trade(&conn, id, &update).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_delete(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::delete_trade(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_default_account_id(state: State<AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let id: i64 = conn
        .query_row(
            "SELECT id FROM accounts WHERE is_active=1 ORDER BY id LIMIT 1",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn trades_export_csv(state: State<AppState>, dest: String) -> Result<u64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let trades = repo::list_trades(&conn, None, 10000, 0).map_err(|e| e.to_string())?;
    crate::db::csv_export::export_trades_csv(&trades, std::path::Path::new(&dest))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_add_attachment(
    state: State<AppState>,
    trade_id: i64,
    filename: String,
    kind: String,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::add_attachment(&conn, trade_id, &filename, &kind).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_remove_attachment(state: State<AppState>, id: i64) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::remove_attachment(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_partial_close(
    state: State<AppState>,
    trade_id: i64,
    exit_price: String,
    exit_quantity: String,
    cost_per_unit: String,
    executed_at: String,
    manual_pnl: Option<String>,
) -> Result<(), String> {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let price = Decimal::from_str(&exit_price).map_err(|e| format!("price: {e}"))?;
    let qty = Decimal::from_str(&exit_quantity).map_err(|e| format!("qty: {e}"))?;
    let cost = Decimal::from_str(&cost_per_unit).map_err(|e| format!("cost: {e}"))?;
    let pnl = match manual_pnl {
        Some(s) if !s.trim().is_empty() => {
            Some(Decimal::from_str(s.trim()).map_err(|e| format!("pnl: {e}"))?)
        }
        _ => None,
    };

    repo::partial_close(&conn, trade_id, price, qty, cost, executed_at, pnl)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_log_sl_move(
    state: State<AppState>,
    trade_id: i64,
    old_stop: String,
    new_stop: String,
    reason: Option<String>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let old = Decimal::from_str(&old_stop).map_err(|e| e.to_string())?;
    let new = Decimal::from_str(&new_stop).map_err(|e| e.to_string())?;
    repo::log_sl_move(&conn, trade_id, old, new, reason.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_sl_history(
    state: State<AppState>,
    trade_id: i64,
) -> Result<Vec<(String, String, Option<String>, String)>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::list_sl_history(&conn, trade_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn trades_copy_screenshot(
    app: tauri::AppHandle,
    state: State<AppState>,
    trade_id: i64,
    source_path: String,
) -> Result<String, String> {
    use tauri::Manager;
    let _conn = state.db.lock().map_err(|e| e.to_string())?;

    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir: {e}"))?;

    let source = std::path::Path::new(&source_path);
    let dest = crate::utils::files::copy_attachment(source, &app_dir, trade_id, "screenshot")
        .map_err(|e| e.to_string())?;

    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_text_file(dest: String, content: String) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::File::create(&dest).map_err(|e| format!("create file: {e}"))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn trades_quick_create(
    state: State<AppState>,
    account_id: i64,
    symbol: String,
    outcome: String,
    r_value: Option<String>,
    session: Option<String>,
    time_of_day: Option<String>,
    entry_price: Option<String>,
    stop_price: Option<String>,
    take_profit: Option<String>,
    quantity: Option<String>,
) -> Result<i64, String> {
    use crate::domain::trade::{NewLeg, NewTrade, Side, SizeType, TradeOutcome};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let parse_opt = |s: &Option<String>| -> Result<Option<Decimal>, String> {
        match s {
            Some(v) if !v.trim().is_empty() => Decimal::from_str(v.trim())
                .map(Some)
                .map_err(|e| format!("decimal: {e}")),
            _ => Ok(None),
        }
    };

    let outcome_enum =
        TradeOutcome::from_str(&outcome).ok_or_else(|| format!("invalid outcome: {outcome}"))?;

    let r_input = parse_opt(&r_value)?;
    let entry_input = parse_opt(&entry_price)?;
    let stop_input = parse_opt(&stop_price)?;
    let tp_input = parse_opt(&take_profit)?;

    // ── Mode selection ─────────────────────────────────────────
    // If entry AND stop provided → price mode: R is computed, never read.
    // Otherwise → R mode: prices ignored, R read as manual_pnl.
    let price_mode = entry_input.is_some() && stop_input.is_some();

    let (entry, stop, tp, qty, manual_pnl) = if price_mode {
        let entry = entry_input.unwrap();
        let stop = stop_input.unwrap();
        let tp = tp_input;
        let qty = parse_opt(&quantity)?.unwrap_or(Decimal::ONE);

        if entry == stop {
            return Err("entry and stop must differ for price mode".into());
        }

        let exit_for_r = match outcome_enum {
            TradeOutcome::TpHit => tp.unwrap_or(entry),
            TradeOutcome::SlHit => stop,
            TradeOutcome::Breakeven => entry,
            _ => entry,
        };

        let denom = (entry - stop).abs();
        let r = if denom.is_zero() {
            Decimal::ZERO
        } else {
            (exit_for_r - entry) / denom
        };

        (entry, stop, tp, qty, Some(r))
    } else {
        (Decimal::ONE, Decimal::ONE, None, Decimal::ONE, r_input)
    };

    let now = chrono::Utc::now().to_rfc3339();

    let opened_at = match time_of_day {
        Some(t) if !t.trim().is_empty() => {
            let today = chrono::Local::now().format("%Y-%m-%d").to_string();
            let hhmm: String = t.trim().split(':').take(2).collect::<Vec<_>>().join(":");
            format!("{}T{}:00", today, hhmm)
        }
        _ => now.clone(),
    };

    let side = Side::Long;

    let new_trade = NewTrade {
        account_id,
        symbol: symbol.trim().to_uppercase(),
        side,
        entry_price: entry,
        initial_stop: stop,
        take_profit: tp,
        total_quantity: qty,
        size_type: SizeType::Lots,
        size_value: Some(qty),
        size_unit: None,
        opened_at: opened_at.clone(),
        notes: session.as_ref().map(|s| format!("Session: {}", s)),
        chart_url: None,
        screenshot_path: None,
        strategy_id: None,
        entry_legs: vec![NewLeg {
            price: entry,
            quantity: qty,
            cost_per_unit: Decimal::ZERO,
        }],
        tags: session.into_iter().collect(),
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let trade_id = repo::create_trade(&conn, &new_trade).map_err(|e| e.to_string())?;

    if outcome_enum != TradeOutcome::Open {
        let exit_price = match outcome_enum {
            TradeOutcome::TpHit => tp.unwrap_or(entry),
            TradeOutcome::SlHit => stop,
            TradeOutcome::Breakeven => entry,
            _ => entry,
        };
        repo::add_exit_leg(
            &conn,
            trade_id,
            exit_price,
            qty,
            Decimal::ZERO,
            now,
            Some(outcome_enum),
            manual_pnl,
        )
        .map_err(|e| e.to_string())?;
    }

    let _id = trade_id;
    drop(conn); // release lock before auto-snapshot
                //     let _ = super::equity::equity_auto_snapshot_create(...); // DISABLED
    Ok(_id)
}

// ============================================================
// PHASE 2.2-A — current price + current stop commands
// ============================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdatePriceInput {
    pub trade_id: i64,
    pub price: String,
    pub source: String, // "manual" only in this phase
}

#[derive(Debug, serde::Serialize)]
pub struct UpdatePriceOut {
    pub trade_id: i64,
    pub price: String,
    pub updated_at: String,
    pub source: String,
}

#[tauri::command]
pub fn trades_update_current_price(
    state: tauri::State<'_, crate::commands::AppState>,
    input: UpdatePriceInput,
) -> Result<UpdatePriceOut, String> {
    use rust_decimal::prelude::FromStr;

    // 1. Validate price
    let price = Decimal::from_str(&input.price).map_err(|e| format!("price: {e}"))?;
    if price <= Decimal::ZERO {
        return Err("price must be > 0".into());
    }

    // 2. Validate source
    let source = input.source.as_str();
    if source == "estimated" {
        return Err("estimated price source is prohibited".into());
    }
    if source != "manual" && source != "broker" {
        return Err(format!("invalid source: {source}"));
    }

    // 3. Check trade exists and is open
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let (status, account_id): (String, i64) = conn
        .query_row(
            "SELECT status, account_id FROM trades WHERE id = ?1",
            [input.trade_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| format!("trade {} not found: {e}", input.trade_id))?;

    if status != "open" {
        return Err(format!("trade {} is {}, not open", input.trade_id, status));
    }
    let _ = account_id; // reserved for future account-scoped checks

    // 4. Update
    let now = chrono::Utc::now().to_rfc3339();
    let updated = conn
        .execute(
            "UPDATE trades
            SET current_price = ?1,
                current_price_updated_at = ?2,
                current_price_source = ?3,
                updated_at = ?2
          WHERE id = ?4 AND status = 'open'",
            rusqlite::params![price.to_string(), now, source, input.trade_id],
        )
        .map_err(|e| e.to_string())?;

    if updated == 0 {
        return Err("update affected 0 rows (trade may have closed)".into());
    }

    Ok(UpdatePriceOut {
        trade_id: input.trade_id,
        price: price.to_string(),
        updated_at: now,
        source: source.to_string(),
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct UpdateStopInput {
    pub trade_id: i64,
    pub new_stop: String,
    pub reason: Option<String>,
}

#[tauri::command]
pub fn trades_update_current_stop(
    state: tauri::State<'_, crate::commands::AppState>,
    input: UpdateStopInput,
) -> Result<(), String> {
    use rust_decimal::prelude::FromStr;

    let new_stop = Decimal::from_str(&input.new_stop).map_err(|e| format!("new_stop: {e}"))?;
    if new_stop <= Decimal::ZERO {
        return Err("new_stop must be > 0".into());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // Load trade
    let (status, side, entry_price, old_stop): (String, String, String, String) = conn
        .query_row(
            "SELECT status, side, entry_price, initial_stop FROM trades WHERE id = ?1",
            [input.trade_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| format!("trade {} not found: {e}", input.trade_id))?;

    if status != "open" {
        return Err(format!("trade {} is not open", input.trade_id));
    }

    // Validate stop direction
    let entry_d = Decimal::from_str(&entry_price).map_err(|e| e.to_string())?;
    if side == "long" && new_stop > entry_d {
        return Err(format!("long stop {new_stop} > entry {entry_d}"));
    }
    if side == "short" && new_stop < entry_d {
        return Err(format!("short stop {new_stop} < entry {entry_d}"));
    }

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    // 1. Update current_stop
    tx.execute(
        "UPDATE trades SET current_stop = ?1, updated_at = ?2 WHERE id = ?3",
        rusqlite::params![new_stop.to_string(), now, input.trade_id],
    )
    .map_err(|e| e.to_string())?;

    // 2. Audit into trade_sl_history
    tx.execute(
        "INSERT INTO trade_sl_history (trade_id, old_stop, new_stop, reason, changed_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            input.trade_id,
            old_stop,
            new_stop.to_string(),
            input.reason.as_deref().unwrap_or("manual_update"),
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}
