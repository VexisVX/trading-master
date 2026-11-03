//! Cash-flow ledger invariants.
//!
//! The ledger is append-only. Each row records the NAV/Units before the
//! flow and the units added/removed. Replaying the ledger must reproduce
//! the current `equity_units` row exactly.

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CashFlowKind {
    Deposit,
    Withdrawal,
    Fee,
    Funding,
    Adjustment,
}

impl CashFlowKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CashFlowKind::Deposit => "deposit",
            CashFlowKind::Withdrawal => "withdrawal",
            CashFlowKind::Fee => "fee",
            CashFlowKind::Funding => "funding",
            CashFlowKind::Adjustment => "adjustment",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "deposit" => CashFlowKind::Deposit,
            "withdrawal" => CashFlowKind::Withdrawal,
            "fee" => CashFlowKind::Fee,
            "funding" => CashFlowKind::Funding,
            "adjustment" => CashFlowKind::Adjustment,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CashFlowRow {
    pub id: i64,
    pub account_id: i64,
    pub kind: CashFlowKind,
    pub amount: Decimal,
    pub currency: String,
    pub occurred_at: String,
    pub nav_before: Decimal,
    pub units_before: Decimal,
    pub units_after: Decimal,
    pub note: Option<String>,
}

/// Replay the ledger and check that each row's units_before matches the
/// previous row's units_after (or the seed units).
pub fn verify_chain(rows: &[CashFlowRow], seed_units: Decimal) -> Result<(), String> {
    let mut expected = seed_units;
    for r in rows {
        if r.units_before != expected {
            return Err(format!(
                "cash-flow {}: units_before {} != expected {}",
                r.id, r.units_before, expected
            ));
        }
        if r.amount.is_zero() {
            return Err(format!("cash-flow {}: amount is zero", r.id));
        }
        if r.nav_before.is_zero() {
            return Err(format!("cash-flow {}: nav_before is zero", r.id));
        }
        expected = r.units_after;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn flow(id: i64, ub: Decimal, ua: Decimal) -> CashFlowRow {
        CashFlowRow {
            id, account_id: 1, kind: CashFlowKind::Deposit,
            amount: dec!(100), currency: "USD".into(),
            occurred_at: "2026-01-01".into(),
            nav_before: Decimal::ONE,
            units_before: ub, units_after: ua,
            note: None,
        }
    }

    #[test]
    fn empty_chain_ok() {
        assert!(verify_chain(&[], dec!(10000)).is_ok());
    }

    #[test]
    fn valid_chain() {
        let rows = vec![
            flow(1, dec!(10000), dec!(10100)),
            flow(2, dec!(10100), dec!(10200)),
        ];
        assert!(verify_chain(&rows, dec!(10000)).is_ok());
    }

    #[test]
    fn broken_chain() {
        let rows = vec![
            flow(1, dec!(10000), dec!(10100)),
            flow(2, dec!(99999), dec!(10200)), // gap
        ];
        assert!(verify_chain(&rows, dec!(10000)).is_err());
    }
}
