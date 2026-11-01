use super::DbError;
use crate::domain::trade::*;
use rusqlite::{params, Connection, OptionalExtension};
use rust_decimal::Decimal;

fn dec(s: &str) -> Result<Decimal, DbError> {
    Decimal::from_str_exact(s)
        .map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))
}

pub fn create_trade(conn: &Connection, t: &NewTrade) -> Result<i64, DbError> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = OFF;")?;
    let tx = conn.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();

    tx.execute(
        "INSERT INTO trades
         (account_id, symbol, side, entry_price, initial_stop, take_profit, total_quantity,
          original_quantity, size_type, size_value, size_unit, outcome,
          status, strategy_id, opened_at, closed_at, notes, chart_url, screenshot_path,
          manual_pnl, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?7,?8,?9,?10,'open','open',?11,?12,NULL,?13,?14,?15,NULL,?16,?16)",
        params![
            t.account_id, t.symbol, t.side.as_str(),
            t.entry_price.to_string(), t.initial_stop.to_string(),
            t.take_profit.map(|d| d.to_string()),
            t.total_quantity.to_string(),
            t.size_type.as_str(),
            t.size_value.map(|d| d.to_string()).unwrap_or_else(|| t.total_quantity.to_string()),
            t.size_unit,
            t.strategy_id, t.opened_at, t.notes, t.chart_url, t.screenshot_path, now
        ],
    )?;
    let trade_id = tx.last_insert_rowid();

    for leg in &t.entry_legs {
        tx.execute(
            "INSERT INTO trade_legs (trade_id, kind, price, quantity, executed_at, cost_per_unit)
             VALUES (?1,'entry',?2,?3,?4,?5)",
            params![
                trade_id, leg.price.to_string(), leg.quantity.to_string(),
                t.opened_at, leg.cost_per_unit.to_string()
            ],
        )?;
    }

    for tag in &t.tags {
        if !tag.trim().is_empty() {
            tx.execute(
                "INSERT INTO trade_tags (trade_id, tag) VALUES (?1, ?2)",
                params![trade_id, tag.trim()],
            )?;
        }
    }

    tx.commit()?;
    Ok(trade_id)
}

pub fn get_trade(conn: &Connection, id: i64) -> Result<Option<Trade>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, symbol, side, entry_price, initial_stop, take_profit,
                total_quantity, original_quantity, outcome, size_type, size_value, size_unit,
                manual_pnl, status, strategy_id, opened_at, closed_at, notes,
                chart_url, screenshot_path, created_at, updated_at
         FROM trades WHERE id = ?1",
    )?;

    let row = stmt.query_row([id], |r| {
        Ok((
            r.get::<_, i64>(0)?,                   // id
            r.get::<_, i64>(1)?,                   // account_id
            r.get::<_, String>(2)?,                // symbol
            r.get::<_, String>(3)?,                // side
            r.get::<_, String>(4)?,                // entry_price
            r.get::<_, String>(5)?,                // initial_stop
            r.get::<_, Option<String>>(6)?,        // take_profit
            r.get::<_, String>(7)?,                // total_quantity
            r.get::<_, Option<String>>(8)?,        // original_quantity
            r.get::<_, String>(9)?,                // outcome
            r.get::<_, String>(10)?,               // size_type
            r.get::<_, Option<String>>(11)?,       // size_value
            r.get::<_, Option<String>>(12)?,       // size_unit
            r.get::<_, Option<String>>(13)?,       // manual_pnl
            r.get::<_, String>(14)?,               // status
            r.get::<_, Option<i64>>(15)?,          // strategy_id
            r.get::<_, String>(16)?,               // opened_at
            r.get::<_, Option<String>>(17)?,       // closed_at
            r.get::<_, Option<String>>(18)?,       // notes
            r.get::<_, Option<String>>(19)?,       // chart_url
            r.get::<_, Option<String>>(20)?,       // screenshot_path
            r.get::<_, String>(21)?,               // created_at
            r.get::<_, String>(22)?,               // updated_at
        ))
    }).optional()?;

    let Some((
        tid, acc, sym, side_s, entry_s, stop_s, tp_s,
        qty_s, orig_qty_s, outcome_s,
        size_type_s, size_value_s, size_unit,
        manual_pnl_s, status_s, sid,
        opened, closed, notes, chart_url, screenshot_path, created, updated,
    )) = row else {
        return Ok(None);
    };

    let legs = load_legs(conn, tid)?;

    Ok(Some(Trade {
        id: tid,
        account_id: acc,
        symbol: sym,
        side: Side::from_str(&side_s).unwrap_or(Side::Long),
        entry_price: dec(&entry_s)?,
        initial_stop: dec(&stop_s)?,
        take_profit: tp_s.as_deref().map(dec).transpose()?,
        total_quantity: dec(&qty_s)?,
        original_quantity: match orig_qty_s.as_deref() {
            Some(v) => dec(v)?,
            None => dec(&qty_s)?,
        },
        outcome: TradeOutcome::from_str(&outcome_s).unwrap_or(TradeOutcome::Open),
        size_type: SizeType::from_str(&size_type_s).unwrap_or(SizeType::Lots),
        size_value: size_value_s.as_deref().map(dec).transpose()?,
        size_unit,
        manual_pnl: manual_pnl_s.as_deref().map(dec).transpose()?,
        status: TradeStatus::from_str(&status_s).unwrap_or(TradeStatus::Open),
        strategy_id: sid,
        opened_at: opened,
        closed_at: closed,
        notes,
        chart_url,
        screenshot_path,
        created_at: created,
        updated_at: updated,
        legs,
        tags: load_tags(conn, tid)?,
        attachments: load_attachments(conn, tid)?,
    }))
}

fn load_legs(conn: &Connection, trade_id: i64) -> Result<Vec<TradeLeg>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, price, quantity, executed_at, cost_per_unit
         FROM trade_legs WHERE trade_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map([trade_id], |r| {
        Ok((
            r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?,
            r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, String>(5)?,
        ))
    })?;
    let mut out = Vec::new();
    for row in rows {
        let (id, kind_s, price_s, qty_s, exec, cost_s) = row?;
        out.push(TradeLeg {
            id,
            kind: LegKind::from_str(&kind_s).unwrap_or(LegKind::Entry),
            price: dec(&price_s)?,
            quantity: dec(&qty_s)?,
            executed_at: exec,
            cost_per_unit: dec(&cost_s)?,
        });
    }
    Ok(out)
}

fn load_tags(conn: &Connection, trade_id: i64) -> Result<Vec<String>, DbError> {
    let mut stmt = conn.prepare("SELECT tag FROM trade_tags WHERE trade_id=?1 ORDER BY id")?;
    let rows = stmt.query_map([trade_id], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for row in rows { out.push(row?); }
    Ok(out)
}

fn load_attachments(conn: &Connection, trade_id: i64) -> Result<Vec<Attachment>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, kind, created_at FROM trade_attachments WHERE trade_id=?1 ORDER BY id"
    )?;
    let rows = stmt.query_map([trade_id], |r| Ok(Attachment {
        id: r.get(0)?, filename: r.get(1)?, kind: r.get(2)?, created_at: r.get(3)?,
    }))?;
    let mut out = Vec::new();
    for row in rows { out.push(row?); }
    Ok(out)
}

pub fn list_trades(conn: &Connection, limit: i64, offset: i64) -> Result<Vec<Trade>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id FROM trades ORDER BY opened_at DESC, id DESC LIMIT ?1 OFFSET ?2",
    )?;
    let ids: Vec<i64> = stmt.query_map(params![limit, offset], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(t) = get_trade(conn, id)? { out.push(t); }
    }
    Ok(out)
}

fn sum_leg_quantities(conn: &Connection, trade_id: i64, kind: LegKind) -> Result<Decimal, DbError> {
    let mut stmt = conn.prepare(
        "SELECT quantity FROM trade_legs WHERE trade_id = ?1 AND kind = ?2",
    )?;
    let rows = stmt.query_map(params![trade_id, kind.as_str()], |r| r.get::<_, String>(0))?;
    let mut total = Decimal::ZERO;
    for row in rows { total += dec(&row?)?; }
    Ok(total)
}

pub fn add_exit_leg(
    conn: &Connection, trade_id: i64, price: Decimal, quantity: Decimal,
    cost_per_unit: Decimal, executed_at: String,
    outcome: Option<TradeOutcome>, manual_pnl: Option<Decimal>,
) -> Result<(), DbError> {
    if quantity <= Decimal::ZERO {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = OFF;")?;
    let tx = conn.unchecked_transaction()?;

    let total_qty_str: String = tx.query_row(
        "SELECT total_quantity FROM trades WHERE id=?1", [trade_id], |r| r.get(0),
    )?;
    let total_qty = dec(&total_qty_str)?;

    tx.execute(
        "INSERT INTO trade_legs (trade_id, kind, price, quantity, executed_at, cost_per_unit)
         VALUES (?1,'exit',?2,?3,?4,?5)",
        params![
            trade_id, price.to_string(), quantity.to_string(),
            executed_at, cost_per_unit.to_string()
        ],
    )?;

    let exited = sum_leg_quantities(&tx, trade_id, LegKind::Exit)?;
    let now = chrono::Utc::now().to_rfc3339();

    if exited >= total_qty {
        let outcome_s = outcome.map(|o| o.as_str().to_string()).unwrap_or_else(|| "manual_close".to_string());
        tx.execute(
            "UPDATE trades SET status='closed', outcome=?1, manual_pnl=?2, closed_at=?3, updated_at=?3 WHERE id=?4",
            params![outcome_s, manual_pnl.map(|d| d.to_string()), now, trade_id],
        )?;
    } else {
        tx.execute("UPDATE trades SET updated_at=?1 WHERE id=?2", params![now, trade_id])?;
    }

    tx.commit()?;
    Ok(())
}

pub fn update_trade(conn: &Connection, id: i64, u: &UpdateTrade) -> Result<(), DbError> {
    conn.execute_batch("PRAGMA query_only = OFF;")?;
    let tx = conn.unchecked_transaction()?;
    let now = chrono::Utc::now().to_rfc3339();

    let affected = tx.execute(
        "UPDATE trades SET
            symbol=?1, entry_price=?2, initial_stop=?3, take_profit=?4,
            total_quantity=?5,
            outcome=?6, size_type=?7, size_value=?8, size_unit=?9,
            manual_pnl=?10,
            strategy_id=?11, notes=?12,
            chart_url=?13, screenshot_path=?14, updated_at=?15
         WHERE id=?16",
        params![
            u.symbol, u.entry_price.to_string(), u.initial_stop.to_string(),
            u.take_profit.map(|d| d.to_string()),
            u.total_quantity.to_string(),
            u.outcome.as_str(), u.size_type.as_str(),
            u.size_value.map(|d| d.to_string()).unwrap_or_else(|| u.total_quantity.to_string()),
            u.size_unit,
            u.manual_pnl.map(|d| d.to_string()),
            u.strategy_id, u.notes, u.chart_url, u.screenshot_path, now, id
        ],
    )?;
    if affected == 0 { return Err(DbError::Sqlite(rusqlite::Error::QueryReturnedNoRows)); }

    tx.execute("DELETE FROM trade_tags WHERE trade_id=?1", [id])?;
    for tag in &u.tags {
        if !tag.trim().is_empty() {
            tx.execute("INSERT INTO trade_tags (trade_id, tag) VALUES (?1, ?2)", params![id, tag.trim()])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn delete_trade(conn: &Connection, id: i64) -> Result<(), DbError> {
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = OFF;")?;
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM trade_attachments WHERE trade_id=?1", [id])?;
    tx.execute("DELETE FROM trade_tags WHERE trade_id=?1", [id])?;
    tx.execute("DELETE FROM trade_legs WHERE trade_id=?1", [id])?;
    tx.execute("DELETE FROM trades WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(())
}

pub fn add_attachment(
    conn: &Connection, trade_id: i64, filename: &str, kind: &str,
) -> Result<i64, DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO trade_attachments (trade_id, filename, kind, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![trade_id, filename, kind, now],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn remove_attachment(conn: &Connection, id: i64) -> Result<String, DbError> {
    let filename: String = conn.query_row(
        "SELECT filename FROM trade_attachments WHERE id=?1", [id], |r| r.get(0),
    )?;
    conn.execute("DELETE FROM trade_attachments WHERE id=?1", [id])?;
    Ok(filename)
}


#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn fresh_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch(
            "CREATE TABLE accounts (id INTEGER PRIMARY KEY, name TEXT, currency TEXT, initial_balance TEXT, is_active INTEGER, kind TEXT DEFAULT 'personal', created_at TEXT);
             INSERT INTO accounts VALUES (1,'T','USD','10000',1,'personal','2026-01-01');
             CREATE TABLE trades (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               account_id INTEGER, symbol TEXT, side TEXT,
               entry_price TEXT, initial_stop TEXT, take_profit TEXT,
               total_quantity TEXT, outcome TEXT DEFAULT 'open',
               size_type TEXT DEFAULT 'lots', size_value TEXT, size_unit TEXT,
               manual_pnl TEXT,
               status TEXT, strategy_id INTEGER,
               opened_at TEXT, closed_at TEXT, notes TEXT,
               chart_url TEXT, screenshot_path TEXT,
               created_at TEXT, updated_at TEXT
             );
             CREATE TABLE trade_legs (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, kind TEXT, price TEXT, quantity TEXT, executed_at TEXT, cost_per_unit TEXT DEFAULT '0');
             CREATE TABLE trade_tags (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, tag TEXT);
             CREATE TABLE trade_attachments (id INTEGER PRIMARY KEY AUTOINCREMENT, trade_id INTEGER, filename TEXT, kind TEXT, created_at TEXT);",
        ).unwrap();
        conn
    }

    fn sample_trade(total: Decimal) -> NewTrade {
        NewTrade {
            account_id: 1, symbol: "EURUSD".into(), side: Side::Long,
            entry_price: dec!(1.1000), initial_stop: dec!(1.0950),
            take_profit: Some(dec!(1.1100)),
            total_quantity: total,
            size_type: SizeType::Lots, size_value: Some(total), size_unit: None,
            opened_at: "2026-10-20T10:00:00Z".into(),
            notes: None, chart_url: None, screenshot_path: None,
            strategy_id: None,
            entry_legs: vec![NewLeg { price: dec!(1.1000), quantity: total, cost_per_unit: Decimal::ZERO }],
            tags: vec![],
        }
    }

    #[test]
    fn round_trip() {
        let conn = fresh_db();
        let id = create_trade(&conn, &sample_trade(dec!(1))).unwrap();
        let t = get_trade(&conn, id).unwrap().unwrap();
        assert_eq!(t.symbol, "EURUSD");
        assert_eq!(t.take_profit, Some(dec!(1.1100)));
        assert_eq!(t.outcome, TradeOutcome::Open);
    }

    #[test]
    fn manual_pnl_persisted_on_close() {
        let conn = fresh_db();
        let id = create_trade(&conn, &sample_trade(dec!(1))).unwrap();
        add_exit_leg(&conn, id, dec!(1.1100), dec!(1), Decimal::ZERO, "t".into(),
            Some(TradeOutcome::TpHit), Some(dec!(420))).unwrap();
        let t = get_trade(&conn, id).unwrap().unwrap();
        assert_eq!(t.status, TradeStatus::Closed);
        assert_eq!(t.outcome, TradeOutcome::TpHit);
        assert_eq!(t.manual_pnl, Some(dec!(420)));
    }

    #[test]
    fn sl_hit_can_have_negative_pnl() {
        let conn = fresh_db();
        let id = create_trade(&conn, &sample_trade(dec!(1))).unwrap();
        add_exit_leg(&conn, id, dec!(1.0950), dec!(1), Decimal::ZERO, "t".into(),
            Some(TradeOutcome::SlHit), Some(dec!(-200))).unwrap();
        let t = get_trade(&conn, id).unwrap().unwrap();
        assert_eq!(t.manual_pnl, Some(dec!(-200)));
    }
}

/// Partial close: exit fewer than total_quantity. Trade remains open if
/// remaining quantity > 0. Adds an exit leg and updates `total_quantity`
/// to the remaining amount. Keeps `original_quantity` unchanged.
pub fn partial_close(
    conn: &Connection, trade_id: i64, exit_price: Decimal,
    exit_qty: Decimal, cost_per_unit: Decimal, executed_at: String,
    manual_pnl: Option<Decimal>,
) -> Result<(), DbError> {
    if exit_qty <= Decimal::ZERO {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }

    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA query_only = OFF;")?;
    let tx = conn.unchecked_transaction()?;

    let total_qty_s: String = tx.query_row(
        "SELECT total_quantity FROM trades WHERE id=?1", [trade_id], |r| r.get(0),
    )?;
    let total_qty = dec(&total_qty_s)?;

    if exit_qty >= total_qty {
        return Err(DbError::Sqlite(rusqlite::Error::InvalidQuery));
    }

    // Add exit leg
    tx.execute(
        "INSERT INTO trade_legs (trade_id, kind, price, quantity, executed_at, cost_per_unit)
         VALUES (?1, 'exit', ?2, ?3, ?4, ?5)",
        params![
            trade_id, exit_price.to_string(), exit_qty.to_string(),
            executed_at, cost_per_unit.to_string()
        ],
    )?;

    // Reduce total_quantity
    let new_qty = total_qty - exit_qty;
    let now = chrono::Utc::now().to_rfc3339();

    // If manual_pnl provided, accumulate it
    let existing_pnl_s: Option<String> = tx.query_row(
        "SELECT manual_pnl FROM trades WHERE id=?1", [trade_id], |r| r.get(0),
    ).ok();
    let existing_pnl = existing_pnl_s.as_deref().and_then(|s| dec(s).ok()).unwrap_or_default();
    let new_pnl = match manual_pnl {
        Some(p) => Some(existing_pnl + p),
        None => if existing_pnl.is_zero() { None } else { Some(existing_pnl) },
    };

    tx.execute(
        "UPDATE trades SET total_quantity=?1, manual_pnl=?2, updated_at=?3 WHERE id=?4",
        params![
            new_qty.to_string(),
            new_pnl.map(|p| p.to_string()),
            now, trade_id
        ],
    )?;

    tx.commit()?;
    Ok(())
}

/// Log a stop-loss move.
pub fn log_sl_move(
    conn: &Connection, trade_id: i64,
    old_stop: Decimal, new_stop: Decimal, reason: Option<&str>,
) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO trade_sl_history (trade_id, old_stop, new_stop, reason, changed_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![trade_id, old_stop.to_string(), new_stop.to_string(), reason, now],
    )?;
    Ok(())
}

pub fn list_sl_history(conn: &Connection, trade_id: i64) -> Result<Vec<(String, String, Option<String>, String)>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT old_stop, new_stop, reason, changed_at FROM trade_sl_history
         WHERE trade_id=?1 ORDER BY changed_at DESC"
    )?;
    let rows = stmt.query_map([trade_id], |r| Ok((
        r.get::<_, String>(0)?,
        r.get::<_, String>(1)?,
        r.get::<_, Option<String>>(2)?,
        r.get::<_, String>(3)?,
    )))?;
    let mut out = Vec::new();
    for row in rows { out.push(row?); }
    Ok(out)
}
