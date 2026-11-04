//! CSV export for trades. Strings only — no float coercion.

use super::DbError;
use crate::domain::trade::{LegKind, Trade};
use std::io::Write;
use std::path::Path;

pub fn export_trades_csv(trades: &[Trade], path: &Path) -> Result<u64, DbError> {
    let mut out = std::fs::File::create(path)?;
    writeln!(
        out,
        "id,account_id,symbol,side,status,entry_price,initial_stop,total_quantity,\
         strategy_id,opened_at,closed_at,realized_pnl,entry_tags,notes"
    )?;

    for t in trades {
        let pnl = realized_pnl(t);
        let tags = t.tags.join(";");
        let notes = t.notes.clone().unwrap_or_default().replace('\n', " ");
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            t.id,
            t.account_id,
            csv_escape(&t.symbol),
            t.side.as_str(),
            t.status.as_str(),
            t.entry_price,
            t.initial_stop,
            t.total_quantity,
            t.strategy_id.map(|s| s.to_string()).unwrap_or_default(),
            csv_escape(&t.opened_at),
            csv_escape(t.closed_at.as_deref().unwrap_or("")),
            pnl,
            csv_escape(&tags),
            csv_escape(&notes),
        )?;
    }
    out.flush()?;
    Ok(std::fs::metadata(path)?.len())
}

fn realized_pnl(t: &Trade) -> String {
    if t.status != crate::domain::trade::TradeStatus::Closed {
        return "0".into();
    }
    // Prefer user-entered P&L when present.
    if let Some(v) = t.manual_pnl {
        return v.to_string();
    }
    let mut gross = rust_decimal::Decimal::ZERO;
    let mut cost = rust_decimal::Decimal::ZERO;
    for leg in t.legs.iter().filter(|l| matches!(l.kind, LegKind::Exit)) {
        let diff = if t.side.is_long() {
            leg.price - t.entry_price
        } else {
            t.entry_price - leg.price
        };
        gross += diff * leg.quantity;
        cost += leg.cost_per_unit * leg.quantity;
    }
    (gross - cost).to_string()
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::trade::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn sample_closed_trade() -> Trade {
        let mut t = empty_trade();
        t.id = 1;
        t.account_id = 1;
        t.symbol = "EURUSD".into();
        t.side = Side::Long;
        t.entry_price = dec!(1.10);
        t.initial_stop = dec!(1.09);
        t.take_profit = Some(dec!(1.13));
        t.total_quantity = dec!(1);
        t.status = TradeStatus::Closed;
        t.outcome = TradeOutcome::TpHit;
        t.strategy_id = Some(2);
        t.opened_at = "2026-10-20T10:00:00Z".into();
        t.closed_at = Some("2026-10-20T15:00:00Z".into());
        t.notes = Some("note, with comma".into());
        t.created_at = "2026-10-20T10:00:00Z".into();
        t.updated_at = "2026-10-20T15:00:00Z".into();
        t.legs = vec![
            TradeLeg {
                id: 1,
                kind: LegKind::Entry,
                price: dec!(1.10),
                quantity: dec!(1),
                executed_at: "t".into(),
                cost_per_unit: Decimal::ZERO,
            },
            TradeLeg {
                id: 2,
                kind: LegKind::Exit,
                price: dec!(1.12),
                quantity: dec!(1),
                executed_at: "t".into(),
                cost_per_unit: Decimal::ZERO,
            },
        ];
        t.tags = vec!["A".into(), "B".into()];
        t
    }

    #[test]
    fn exports_header_and_row() {
        let trade = sample_closed_trade();
        let dir = std::env::temp_dir();
        let path = dir.join("test_export.csv");
        let size = export_trades_csv(&[trade], &path).unwrap();
        assert!(size > 0);
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("EURUSD"));
        assert!(content.contains("\"note, with comma\""));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn manual_pnl_overrides_price_derived() {
        let mut trade = sample_closed_trade();
        trade.manual_pnl = Some(dec!(420));
        let dir = std::env::temp_dir();
        let path = dir.join("test_export_manual.csv");
        export_trades_csv(&[trade], &path).unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("420"));
        let _ = std::fs::remove_file(&path);
    }
}
