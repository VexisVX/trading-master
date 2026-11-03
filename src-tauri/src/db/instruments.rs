//! Instruments CRUD (audit §9).

use super::DbError;
use rusqlite::Connection;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

#[derive(Debug, Clone)]
pub struct InstrumentRow {
    pub symbol: String,
    pub currency: String,
    pub tick_size: String,
    pub tick_value: String,
    pub contract_size: String,
    pub min_qty: String,
    pub max_qty: String,
    pub lot_step: String,
    pub commission_per_lot: String,
    pub slippage_per_unit: String,
    pub spec_source: String,
}

pub fn upsert(conn: &Connection, r: &InstrumentRow) -> Result<(), DbError> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO instruments
           (symbol, currency, tick_size, tick_value, contract_size,
            min_qty, max_qty, lot_step, commission_per_lot, slippage_per_unit,
            spec_source, created_at, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?12)
         ON CONFLICT(symbol) DO UPDATE SET
           currency=excluded.currency,
           tick_size=excluded.tick_size,
           tick_value=excluded.tick_value,
           contract_size=excluded.contract_size,
           min_qty=excluded.min_qty,
           max_qty=excluded.max_qty,
           lot_step=excluded.lot_step,
           commission_per_lot=excluded.commission_per_lot,
           slippage_per_unit=excluded.slippage_per_unit,
           spec_source=excluded.spec_source,
           updated_at=excluded.updated_at",
        rusqlite::params![
            r.symbol, r.currency, r.tick_size, r.tick_value, r.contract_size,
            r.min_qty, r.max_qty, r.lot_step, r.commission_per_lot, r.slippage_per_unit,
            r.spec_source, now,
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, symbol: &str) -> Result<Option<InstrumentRow>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT symbol, currency, tick_size, tick_value, contract_size,
                min_qty, max_qty, lot_step, commission_per_lot, slippage_per_unit,
                spec_source
         FROM instruments WHERE symbol = ?1",
    )?;
    let mut rows = stmt.query([symbol])?;
    if let Some(r) = rows.next()? {
        Ok(Some(InstrumentRow {
            symbol: r.get(0)?, currency: r.get(1)?,
            tick_size: r.get(2)?, tick_value: r.get(3)?, contract_size: r.get(4)?,
            min_qty: r.get(5)?, max_qty: r.get(6)?, lot_step: r.get(7)?,
            commission_per_lot: r.get(8)?, slippage_per_unit: r.get(9)?,
            spec_source: r.get(10)?,
        }))
    } else {
        Ok(None)
    }
}

#[allow(dead_code)]
fn _dec(s: &str) -> Result<Decimal, DbError> {
    Decimal::from_str(s).map_err(|_| DbError::Sqlite(rusqlite::Error::InvalidQuery))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE instruments (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               symbol TEXT NOT NULL UNIQUE,
               currency TEXT NOT NULL,
               tick_size TEXT NOT NULL, tick_value TEXT NOT NULL,
               contract_size TEXT NOT NULL,
               min_qty TEXT NOT NULL DEFAULT '0',
               max_qty TEXT NOT NULL DEFAULT '0',
               lot_step TEXT NOT NULL DEFAULT '0.01',
               commission_per_lot TEXT NOT NULL DEFAULT '0',
               slippage_per_unit TEXT NOT NULL DEFAULT '0',
               spec_source TEXT NOT NULL DEFAULT 'user_entered',
               created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );",
        ).unwrap();
        c
    }

    fn row() -> InstrumentRow {
        InstrumentRow {
            symbol: "EURUSD".into(), currency: "USD".into(),
            tick_size: "0.0001".into(), tick_value: "1".into(),
            contract_size: "100000".into(),
            min_qty: "0.01".into(), max_qty: "100".into(),
            lot_step: "0.01".into(),
            commission_per_lot: "0".into(), slippage_per_unit: "0".into(),
            spec_source: "user_entered".into(),
        }
    }

    #[test]
    fn upsert_and_get() {
        let c = fresh();
        upsert(&c, &row()).unwrap();
        let got = get(&c, "EURUSD").unwrap().unwrap();
        assert_eq!(got.contract_size, "100000");
    }

    #[test]
    fn upsert_updates() {
        let c = fresh();
        upsert(&c, &row()).unwrap();
        let mut r = row();
        r.contract_size = "200000".into();
        upsert(&c, &r).unwrap();
        let got = get(&c, "EURUSD").unwrap().unwrap();
        assert_eq!(got.contract_size, "200000");
    }
}
