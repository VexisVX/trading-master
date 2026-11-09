//! Risk budget computation.
//! Given open trades, closed trades today, and current equity,
//! produce a `RiskStatus` snapshot for the dashboard.

use crate::domain::risk_settings::{RiskSettings, RiskStatus};
use crate::domain::trade::{Side, Trade, TradeStatus};
use rust_decimal::Decimal;

/// Compute the open risk of a single trade.
/// open_risk = |entry - initial_stop| * total_quantity
pub fn trade_open_risk(t: &Trade) -> Decimal {
    let per_unit = (t.entry_price - t.initial_stop).abs();
    per_unit * t.total_quantity
}

/// True if the trade was closed today (UTC).
fn closed_today(t: &Trade, today_prefix: &str) -> bool {
    t.status == TradeStatus::Closed
        && t.closed_at
            .as_deref()
            .map(|c| c.starts_with(today_prefix))
            .unwrap_or(false)
}

/// Realized P&L of a closed trade, given entry/exit legs.
/// Positive = profit, negative = loss. Costs are subtracted.
pub fn trade_realized_pnl(t: &Trade) -> Decimal {
    if t.status != TradeStatus::Closed {
        return Decimal::ZERO;
    }

    // Prefer user-entered P&L when present — it is what the trader actually realized.
    if let Some(v) = t.manual_pnl {
        return v;
    }

    let entries: Vec<_> = t
        .legs
        .iter()
        .filter(|l| matches!(l.kind, crate::domain::trade::LegKind::Entry))
        .collect();
    let exits: Vec<_> = t
        .legs
        .iter()
        .filter(|l| matches!(l.kind, crate::domain::trade::LegKind::Exit))
        .collect();
    if entries.is_empty() || exits.is_empty() {
        return Decimal::ZERO;
    }

    let mut gross = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    for ex in &exits {
        let diff = if t.side == Side::Long {
            ex.price - t.entry_price
        } else {
            t.entry_price - ex.price
        };
        gross += diff * ex.quantity;
        cost += ex.cost_per_unit * ex.quantity;
    }
    gross - cost
}

/// Compute the full risk status for an account.
pub fn compute_risk_status(
    settings: &RiskSettings,
    equity: Decimal,
    trades: &[Trade],
    today_prefix: &str, // "YYYY-MM-DD"
) -> RiskStatus {
    let open: Vec<&Trade> = trades
        .iter()
        .filter(|t| t.status == TradeStatus::Open)
        .collect();

    let open_positions = open.len() as i64;

    let mut open_risk = Decimal::ZERO;
    for t in &open {
        open_risk += trade_open_risk(t);
    }

    let open_risk_pct = if equity.is_zero() {
        Decimal::ZERO
    } else {
        (open_risk / equity) * Decimal::from(100)
    };

    // Daily realized P&L (net). Positive = profit, negative = loss.
    let mut daily_pnl = Decimal::ZERO;
    for t in trades {
        if closed_today(t, today_prefix) {
            daily_pnl += trade_realized_pnl(t);
        }
    }
    let daily_loss_today = if daily_pnl < Decimal::ZERO {
        -daily_pnl
    } else {
        Decimal::ZERO
    };
    let daily_loss_pct = if equity.is_zero() {
        Decimal::ZERO
    } else {
        (daily_loss_today / equity) * Decimal::from(100)
    };

    let risk_budget_used_pct = if settings.max_portfolio_risk_pct.is_zero() {
        Decimal::ZERO
    } else {
        (open_risk_pct / settings.max_portfolio_risk_pct) * Decimal::from(100)
    };

    let daily_loss_budget_used_pct = if settings.daily_loss_limit_pct.is_zero() {
        Decimal::ZERO
    } else {
        (daily_loss_pct / settings.daily_loss_limit_pct) * Decimal::from(100)
    };

    let max_positions_hit = open_positions >= settings.max_open_positions;
    let daily_loss_limit_hit = daily_loss_pct >= settings.daily_loss_limit_pct;
    let portfolio_risk_exceeded = open_risk_pct > settings.max_portfolio_risk_pct;

    let mut warnings = Vec::new();
    if max_positions_hit {
        warnings.push(format!(
            "Max open positions reached ({}/{}).",
            open_positions, settings.max_open_positions
        ));
    }
    if daily_loss_limit_hit {
        warnings.push(format!(
            "Daily loss limit hit: {}% of {}%.",
            round2(daily_loss_pct),
            round2(settings.daily_loss_limit_pct)
        ));
    }
    if portfolio_risk_exceeded {
        warnings.push(format!(
            "Portfolio risk exceeded: {}% of {}%.",
            round2(open_risk_pct),
            round2(settings.max_portfolio_risk_pct)
        ));
    }

    RiskStatus {
        settings: settings.clone(),
        current_equity: equity,
        open_positions,
        open_risk,
        open_risk_pct: round2(open_risk_pct),
        daily_loss_today,
        daily_loss_pct: round2(daily_loss_pct),
        risk_budget_used_pct: round2(risk_budget_used_pct),
        daily_loss_budget_used_pct: round2(daily_loss_budget_used_pct),
        max_positions_hit,
        daily_loss_limit_hit,
        portfolio_risk_exceeded,
        warnings,
    }
}

fn round2(v: Decimal) -> Decimal {
    super::money::round_money(v, 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::trade::*;
    use rust_decimal_macros::dec;

    fn settings() -> RiskSettings {
        RiskSettings {
            account_id: 1,
            max_risk_per_trade_pct: dec!(1),
            daily_loss_limit_pct: dec!(3),
            max_open_positions: 3,
            max_portfolio_risk_pct: dec!(5),
            updated_at: "2026-10-20".into(),
        }
    }

    fn open_trade(id: i64, entry: Decimal, stop: Decimal, qty: Decimal) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.entry_price = entry;
        t.initial_stop = stop;
        t.total_quantity = qty;
        t.status = TradeStatus::Open;
        t
    }

    fn closed_trade(id: i64, entry: Decimal, exit: Decimal, qty: Decimal, closed: &str) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.entry_price = entry;
        t.initial_stop = entry - dec!(0.01);
        t.total_quantity = qty;
        t.status = TradeStatus::Closed;
        t.closed_at = Some(closed.to_string());
        t.opened_at = closed.to_string();
        t.created_at = closed.to_string();
        t.updated_at = closed.to_string();
        t.legs = vec![
            TradeLeg {
                id: 1,
                kind: LegKind::Entry,
                price: entry,
                quantity: qty,
                executed_at: closed.into(),
                cost_per_unit: Decimal::ZERO,
            },
            TradeLeg {
                id: 2,
                kind: LegKind::Exit,
                price: exit,
                quantity: qty,
                executed_at: closed.into(),
                cost_per_unit: Decimal::ZERO,
            },
        ];
        t
    }

    #[test]
    fn empty_state_is_zero() {
        let st = compute_risk_status(&settings(), dec!(10000), &[], "2026-10-20");
        assert_eq!(st.open_positions, 0);
        assert_eq!(st.open_risk, Decimal::ZERO);
        assert!(!st.max_positions_hit);
        assert!(!st.daily_loss_limit_hit);
        assert!(!st.portfolio_risk_exceeded);
        assert!(st.warnings.is_empty());
    }

    #[test]
    fn open_risk_pct_correct() {
        let t = open_trade(1, dec!(1.1000), dec!(1.0900), dec!(1));
        let st = compute_risk_status(&settings(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.open_positions, 1);
        assert_eq!(st.open_risk, dec!(0.01));
    }

    #[test]
    fn max_positions_hit() {
        let trades = vec![
            open_trade(1, dec!(1.1), dec!(1.09), dec!(1)),
            open_trade(2, dec!(1.1), dec!(1.09), dec!(1)),
            open_trade(3, dec!(1.1), dec!(1.09), dec!(1)),
        ];
        let st = compute_risk_status(&settings(), dec!(10000), &trades, "2026-10-20");
        assert_eq!(st.open_positions, 3);
        assert!(st.max_positions_hit);
    }

    #[test]
    fn daily_loss_limit_hit() {
        let t = closed_trade(
            1,
            dec!(1.10),
            dec!(1.06),
            dec!(10000),
            "2026-10-20T15:00:00Z",
        );
        let st = compute_risk_status(&settings(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.daily_loss_today, dec!(400));
        assert_eq!(st.daily_loss_pct, dec!(4.00));
        assert!(st.daily_loss_limit_hit);
    }

    #[test]
    fn trade_from_yesterday_is_ignored() {
        let t = closed_trade(1, dec!(1.10), dec!(1.06), dec!(100), "2026-10-19T15:00:00Z");
        let st = compute_risk_status(&settings(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.daily_loss_today, Decimal::ZERO);
    }

    #[test]
    fn profit_does_not_trigger_daily_loss_limit() {
        let t = closed_trade(
            1,
            dec!(1.10),
            dec!(1.14),
            dec!(10000),
            "2026-10-20T15:00:00Z",
        );
        let st = compute_risk_status(&settings(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.daily_loss_today, Decimal::ZERO);
        assert!(!st.daily_loss_limit_hit);
    }

    #[test]
    fn manual_pnl_preferred_over_price() {
        let mut t = closed_trade(1, dec!(1.10), dec!(1.06), dec!(100), "2026-10-20T15:00:00Z");
        // manual_pnl says -50, even though price-derived would be -4
        t.manual_pnl = Some(dec!(-50));
        let st = compute_risk_status(&settings(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.daily_loss_today, dec!(50));
    }
}
