//! Prop-firm rule evaluation.
//!
//! All math is Decimal. No floats. Rules are evaluated left-to-right and
//! produce a flat list of `RuleCheck` — each with pass/fail, current value,
//! limit, and a human message.
//!
//! Rules:
//!  - Max drawdown from peak (trailing or static)
//!  - Daily loss limit (per UTC day)
//!  - Max total loss from initial balance
//!  - Profit target
//!  - Minimum trading days
//!
//! A rule with limit == 0 is treated as disabled and always passes.

use rust_decimal::Decimal;
use crate::domain::prop_firm::{PropFirmRules, PropFirmStatus, RuleCheck};
use crate::domain::trade::{Trade, TradeStatus};

fn round2(v: Decimal) -> Decimal {
    super::money::round_money(v, 2)
}

fn pct(num: Decimal, den: Decimal) -> Decimal {
    if den.is_zero() { Decimal::ZERO } else { round2(num / den * Decimal::from(100)) }
}

/// Realized P&L of a closed trade.
fn trade_pnl(t: &Trade) -> Decimal {
    if t.status != TradeStatus::Closed { return Decimal::ZERO; }
    // Prefer the user-entered P&L when present.
    if let Some(v) = t.manual_pnl { return v; }
    let mut gross = Decimal::ZERO;
    let mut cost = Decimal::ZERO;
    for ex in t.legs.iter().filter(|l| matches!(l.kind, crate::domain::trade::LegKind::Exit)) {
        let diff = match t.side {
            crate::domain::trade::Side::Long => ex.price - t.entry_price,
            crate::domain::trade::Side::Short => t.entry_price - ex.price,
        };
        gross += diff * ex.quantity;
        cost += ex.cost_per_unit * ex.quantity;
    }
    gross - cost
}

/// Distinct UTC dates on which at least one trade was closed.
fn trading_day_set(trades: &[Trade]) -> std::collections::BTreeSet<String> {
    trades.iter()
        .filter_map(|t| {
            if t.status != TradeStatus::Closed { return None; }
            t.closed_at.as_deref()
                .and_then(|c| c.get(0..10))
                .map(|s| s.to_string())
        })
        .collect()
}

pub fn evaluate(
    rules: &PropFirmRules,
    initial_balance: Decimal,
    trades: &[Trade],
    today_prefix: &str,
) -> PropFirmStatus {
    // --- Order closed trades chronologically, build equity series ---
    let mut closed: Vec<&Trade> = trades.iter()
        .filter(|t| t.status == TradeStatus::Closed)
        .collect();
    closed.sort_by(|a, b| {
        a.closed_at.as_deref().unwrap_or("")
            .cmp(b.closed_at.as_deref().unwrap_or(""))
    });

    let mut equity = initial_balance;
    let mut peak = initial_balance;
    // For daily loss, track day-start equity and intraday low.
    let mut day_start_equity = initial_balance;
    let mut current_day = String::new();
    let mut worst_daily_loss_pct = Decimal::ZERO;
    let mut worst_daily_loss_day = String::new();

    for t in &closed {
        let closed_at = t.closed_at.as_deref().unwrap_or("");
        let day = closed_at.get(0..10).unwrap_or("").to_string();
        if day != current_day {
            current_day = day.clone();
            day_start_equity = equity;
        }
        equity += trade_pnl(t);
        if equity > peak { peak = equity; }
        let day_loss_pct = pct(day_start_equity - equity, day_start_equity);
        if day_loss_pct > worst_daily_loss_pct {
            worst_daily_loss_pct = day_loss_pct;
            worst_daily_loss_day = day.clone();
        }
    }

    // Today's loss: recompute only for trades closed today
    let mut today_start_equity = initial_balance;
    // Walk again, capturing equity before today's first trade.
    {
        let mut eq = initial_balance;
        for t in &closed {
            let c = t.closed_at.as_deref().unwrap_or("");
            let d = c.get(0..10).unwrap_or("");
            if d < today_prefix {
                eq += trade_pnl(t);
            } else if d == today_prefix {
                today_start_equity = eq;
                break;
            }
        }
    }
    let mut today_equity = today_start_equity;
    for t in &closed {
        let c = t.closed_at.as_deref().unwrap_or("");
        if c.get(0..10) == Some(today_prefix) {
            today_equity += trade_pnl(t);
        }
    }
    let today_loss_pct = pct(today_start_equity - today_equity, today_start_equity);

    let total_pnl = equity - initial_balance;
    let profit_pct = pct(total_pnl, initial_balance);

    // --- Drawdown (trailing or static) ---
    let floor = if rules.trailing_dd { peak } else { initial_balance };
    let dd_amount = floor - equity;
    let dd_pct = if floor.is_zero() { Decimal::ZERO } else { pct(dd_amount.max(Decimal::ZERO), floor) };

    // --- Trading days ---
    let days = trading_day_set(trades);
    let trading_days = days.len() as i64;

    // --- Checks ---
    let mut checks: Vec<RuleCheck> = Vec::new();

    checks.push(RuleCheck {
        name: "Max drawdown".into(),
        passed: dd_pct <= rules.max_dd_pct,
        current: dd_pct,
        limit: rules.max_dd_pct,
        message: format!(
            "{} drawdown {}% of {} (limit {}%)",
            if rules.trailing_dd { "Trailing" } else { "Static" },
            round2(dd_pct), round2(floor), round2(rules.max_dd_pct),
        ),
    });

    if !rules.daily_loss_limit_pct.is_zero() {
        checks.push(RuleCheck {
            name: "Daily loss".into(),
            passed: today_loss_pct <= rules.daily_loss_limit_pct,
            current: today_loss_pct,
            limit: rules.daily_loss_limit_pct,
            message: format!(
                "Today's loss {}% (limit {}%){}",
                round2(today_loss_pct),
                round2(rules.daily_loss_limit_pct),
                if worst_daily_loss_day.is_empty() {
                    String::new()
                } else {
                    format!(" · worst day {} at {}%", worst_daily_loss_day, round2(worst_daily_loss_pct))
                }
            ),
        });
    }

    if !rules.max_total_loss_pct.is_zero() {
        let loss_pct = pct((-total_pnl).max(Decimal::ZERO), initial_balance);
        checks.push(RuleCheck {
            name: "Total loss".into(),
            passed: loss_pct <= rules.max_total_loss_pct,
            current: loss_pct,
            limit: rules.max_total_loss_pct,
            message: format!(
                "Total loss {}% of initial (limit {}%)",
                round2(loss_pct), round2(rules.max_total_loss_pct)
            ),
        });
    }

    if !rules.profit_target_pct.is_zero() {
        checks.push(RuleCheck {
            name: "Profit target".into(),
            passed: profit_pct >= rules.profit_target_pct,
            current: profit_pct,
            limit: rules.profit_target_pct,
            message: format!(
                "Profit {}% (target {}%)",
                round2(profit_pct), round2(rules.profit_target_pct)
            ),
        });
    }

    if rules.min_trading_days > 0 {
        checks.push(RuleCheck {
            name: "Trading days".into(),
            passed: Decimal::from(trading_days) >= Decimal::from(rules.min_trading_days),
            current: Decimal::from(trading_days),
            limit: Decimal::from(rules.min_trading_days),
            message: format!(
                "{} of {} required trading days",
                trading_days, rules.min_trading_days
            ),
        });
    }

    let mut warnings: Vec<String> = Vec::new();
    for c in &checks {
        if !c.passed {
            warnings.push(c.message.clone());
        }
    }

    let all_passed = checks.iter().all(|c| c.passed);

    PropFirmStatus {
        rules: rules.clone(),
        initial_balance,
        peak_equity: round2(peak),
        current_equity: round2(equity),
        total_pnl: round2(total_pnl),
        trading_days,
        checks,
        all_passed,
        warnings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use crate::domain::trade::*;

    fn rules() -> PropFirmRules {
        PropFirmRules {
            account_id: 1,
            enabled: true,
            max_dd_pct: dec!(10),
            trailing_dd: true,
            daily_loss_limit_pct: dec!(5),
            max_total_loss_pct: dec!(10),
            profit_target_pct: dec!(8),
            min_trading_days: 2,
            updated_at: "2026-10-20".into(),
        }
    }

    fn closed(id: i64, entry: Decimal, exit: Decimal, qty: Decimal, at: &str) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.entry_price = entry;
        t.initial_stop = entry - dec!(0.01);
        t.total_quantity = qty;
        t.status = TradeStatus::Closed;
        t.outcome = TradeOutcome::ManualClose;
        t.closed_at = Some(at.into());
        t.opened_at = at.into();
        t.created_at = at.into();
        t.updated_at = at.into();
        t.legs = vec![
            TradeLeg { id: 1, kind: LegKind::Entry, price: entry, quantity: qty, executed_at: at.into(), cost_per_unit: Decimal::ZERO },
            TradeLeg { id: 2, kind: LegKind::Exit,  price: exit,  quantity: qty, executed_at: at.into(), cost_per_unit: Decimal::ZERO },
        ];
        t
    }

    #[test]
    fn empty_account_rules_evaluated() {
        let st = evaluate(&rules(), dec!(10000), &[], "2026-10-20");
        assert!(st.checks.iter().any(|c| c.name == "Max drawdown" && c.passed));
        assert!(st.checks.iter().any(|c| c.name == "Profit target" && !c.passed));
        assert!(st.checks.iter().any(|c| c.name == "Trading days" && !c.passed));
    }

    #[test]
    fn profit_target_met() {
        let t = closed(1, dec!(1.00), dec!(2.00), dec!(1000), "2026-10-20T10:00:00Z");
        let st = evaluate(&rules(), dec!(10000), &[t], "2026-10-20");
        let pt = st.checks.iter().find(|c| c.name == "Profit target").unwrap();
        assert!(pt.passed);
    }

    #[test]
    fn trailing_dd_uses_peak() {
        let win = closed(1, dec!(1.00), dec!(2.00), dec!(1000), "2026-10-19T10:00:00Z");
        let loss = closed(2, dec!(1.00), dec!(0.00), dec!(1100), "2026-10-20T10:00:00Z");
        let mut r = rules();
        r.trailing_dd = true;
        let st = evaluate(&r, dec!(10000), &[win, loss], "2026-10-20");
        assert_eq!(st.peak_equity, dec!(11000));
        assert_eq!(st.current_equity, dec!(9900));
    }

    #[test]
    fn static_dd_uses_initial() {
        let win = closed(1, dec!(1.00), dec!(2.00), dec!(1000), "2026-10-19T10:00:00Z");
        let loss = closed(2, dec!(1.00), dec!(0.00), dec!(500), "2026-10-20T10:00:00Z");
        let mut r = rules();
        r.trailing_dd = false;
        let st = evaluate(&r, dec!(10000), &[win, loss], "2026-10-20");
        assert_eq!(st.current_equity, dec!(10500));
    }

    #[test]
    fn daily_loss_breach() {
        let t = closed(1, dec!(1.00), dec!(0.40), dec!(1000), "2026-10-20T10:00:00Z");
        let st = evaluate(&rules(), dec!(10000), &[t], "2026-10-20");
        let dl = st.checks.iter().find(|c| c.name == "Daily loss").unwrap();
        assert!(!dl.passed);
    }

    #[test]
    fn min_trading_days_counted() {
        let t1 = closed(1, dec!(1.00), dec!(1.10), dec!(1000), "2026-10-18T10:00:00Z");
        let t2 = closed(2, dec!(1.00), dec!(1.10), dec!(1000), "2026-10-19T10:00:00Z");
        let t3 = closed(3, dec!(1.00), dec!(1.10), dec!(1000), "2026-10-19T15:00:00Z");
        let st = evaluate(&rules(), dec!(10000), &[t1, t2, t3], "2026-10-20");
        assert_eq!(st.trading_days, 2);
    }

    #[test]
    fn manual_pnl_used_in_equity() {
        let mut t = closed(1, dec!(1.00), dec!(1.10), dec!(1000), "2026-10-20T10:00:00Z");
        // Price-derived would be +100, but manual says +500.
        t.manual_pnl = Some(dec!(500));
        let st = evaluate(&rules(), dec!(10000), &[t], "2026-10-20");
        assert_eq!(st.current_equity, dec!(10500));
    }
}


