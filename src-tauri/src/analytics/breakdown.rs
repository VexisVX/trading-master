//! Breakdown analytics: group trades by strategy, month, weekday, session.
//! All math uses Decimal. Never f64.

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::domain::trade::{Side, Trade, TradeStatus};
use crate::risk::r_multiple::{compute_r, Exit, TradeR};

fn round2(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointNearestEven)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMetrics {
    pub key: String,
    pub label: String,
    pub trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub win_rate_pct: Decimal,
    pub net_pnl: Decimal,
    pub gross_profit: Decimal,
    pub gross_loss: Decimal,
    pub profit_factor: Decimal,
    pub expectancy_r: Decimal,
    pub avg_win_r: Decimal,
    pub avg_loss_r: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Breakdown {
    pub by_strategy: Vec<GroupMetrics>,
    pub by_month: Vec<GroupMetrics>,
    pub by_weekday: Vec<GroupMetrics>,
    pub by_session: Vec<GroupMetrics>,
    pub plan_adherence_pct: Decimal,
    pub total_closed: u32,
    pub planned_trades: u32,
    pub tp_hit_count: u32,
    pub sl_hit_count: u32,
    pub breakeven_count: u32,
    pub manual_count: u32,
}

fn trade_r_and_pnl(t: &Trade) -> Option<(Decimal, Decimal)> {
    if t.status != TradeStatus::Closed {
        return None;
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
        return None;
    }

    let cost_per_unit: Decimal =
        exits.iter().map(|l| l.cost_per_unit).sum::<Decimal>() / Decimal::from(exits.len() as u32);

    let tr = TradeR {
        total_quantity: t.total_quantity,
        entry_price: t.entry_price,
        initial_stop: t.initial_stop,
        is_long: t.side == Side::Long,
        cost_per_unit,
        exits: exits
            .iter()
            .map(|l| Exit {
                price: l.price,
                quantity: l.quantity,
            })
            .collect(),
    };
    let res = compute_r(&tr).ok()?;

    if let Some(manual) = t.manual_pnl {
        let initial_risk = (t.entry_price - t.initial_stop).abs() * t.total_quantity;
        if initial_risk.is_zero() {
            return Some((Decimal::ZERO, manual));
        }
        return Some((manual / initial_risk, manual));
    }
    Some((res.r_multiple, res.net_pnl))
}

fn session_of(iso: &str) -> &'static str {
    if iso.len() < 13 {
        return "?";
    }
    let hour: u32 = iso[11..13].parse().unwrap_or(0);
    if hour < 8 {
        "Asia"
    } else if hour < 16 {
        "London"
    } else {
        "NY"
    }
}

/// Zeller's congruence — no chrono feature dependencies.
/// Returns "Mon".."Sun" or "?" if invalid.
fn weekday_of(iso: &str) -> &'static str {
    if iso.len() < 10 {
        return "?";
    }
    let y: i32 = match iso[0..4].parse() {
        Ok(v) => v,
        Err(_) => return "?",
    };
    let m: i32 = match iso[5..7].parse() {
        Ok(v) => v,
        Err(_) => return "?",
    };
    let d: i32 = match iso[8..10].parse() {
        Ok(v) => v,
        Err(_) => return "?",
    };
    let (mut y, mut m) = (y, m);
    if m < 3 {
        m += 12;
        y -= 1;
    }
    let k = y % 100;
    let j = y / 100;
    let h = (d + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    match h {
        0 => "Sat",
        1 => "Sun",
        2 => "Mon",
        3 => "Tue",
        4 => "Wed",
        5 => "Thu",
        _ => "Fri",
    }
}

fn month_of(iso: &str) -> String {
    if iso.len() < 7 {
        return "?".into();
    }
    iso[0..7].to_string()
}

fn build_group(key: String, label: String, values: &[(Decimal, Decimal)]) -> GroupMetrics {
    let trades = values.len() as u32;
    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut net = Decimal::ZERO;
    let mut gp = Decimal::ZERO;
    let mut gl = Decimal::ZERO;
    let mut win_rs: Vec<Decimal> = Vec::new();
    let mut loss_rs: Vec<Decimal> = Vec::new();

    for (r, pnl) in values {
        net += pnl;
        if *r > Decimal::ZERO {
            wins += 1;
            gp += pnl;
            win_rs.push(*r);
        } else if *r < Decimal::ZERO {
            losses += 1;
            gl += pnl.abs();
            loss_rs.push(*r);
        }
    }

    let decided = wins + losses;
    let win_rate = if decided == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(wins) / Decimal::from(decided) * Decimal::from(100)
    };

    let pf = if gl.is_zero() {
        if gp > Decimal::ZERO {
            Decimal::from(999)
        } else {
            Decimal::ZERO
        }
    } else {
        gp / gl
    };

    let avg_win_r = if win_rs.is_empty() {
        Decimal::ZERO
    } else {
        win_rs.iter().copied().sum::<Decimal>() / Decimal::from(win_rs.len() as u32)
    };
    let avg_loss_r = if loss_rs.is_empty() {
        Decimal::ZERO
    } else {
        loss_rs.iter().copied().sum::<Decimal>() / Decimal::from(loss_rs.len() as u32)
    };

    let expectancy = if trades == 0 {
        Decimal::ZERO
    } else {
        values.iter().map(|(r, _)| *r).sum::<Decimal>() / Decimal::from(trades)
    };

    GroupMetrics {
        key,
        label,
        trades,
        wins,
        losses,
        win_rate_pct: round2(win_rate),
        net_pnl: round2(net),
        gross_profit: round2(gp),
        gross_loss: round2(gl),
        profit_factor: round2(pf),
        expectancy_r: round2(expectancy),
        avg_win_r: round2(avg_win_r),
        avg_loss_r: round2(avg_loss_r),
    }
}

struct Row {
    r: Decimal,
    pnl: Decimal,
    strategy_id: Option<i64>,
    month: String,
    weekday: &'static str,
    session: &'static str,
}

pub fn compute_breakdown(
    trades: &[Trade],
    strategy_names: &std::collections::HashMap<i64, String>,
) -> Breakdown {
    let mut rows: Vec<Row> = Vec::new();
    for t in trades {
        let Some((r, pnl)) = trade_r_and_pnl(t) else {
            continue;
        };
        let closed = t.closed_at.as_deref().unwrap_or("");
        rows.push(Row {
            r,
            pnl,
            strategy_id: t.strategy_id,
            month: month_of(closed),
            weekday: weekday_of(closed),
            session: session_of(closed),
        });
    }

    let total_closed = rows.len() as u32;
    let planned = rows.iter().filter(|r| r.strategy_id.is_some()).count() as u32;
    let plan_pct = if total_closed == 0 {
        Decimal::ZERO
    } else {
        Decimal::from(planned) / Decimal::from(total_closed) * Decimal::from(100)
    };

    fn bucket<F, K>(rows: &[Row], key_of: F) -> BTreeMap<K, Vec<(Decimal, Decimal)>>
    where
        F: Fn(&Row) -> K,
        K: Ord + Clone,
    {
        let mut m: BTreeMap<K, Vec<(Decimal, Decimal)>> = BTreeMap::new();
        for r in rows {
            m.entry(key_of(r)).or_default().push((r.r, r.pnl));
        }
        m
    }

    // By strategy
    let by_strategy_map = bucket(&rows, |r| r.strategy_id);
    let mut by_strategy: Vec<GroupMetrics> = by_strategy_map
        .into_iter()
        .map(|(sid, vals)| {
            let key = sid
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unplanned".into());
            let label = sid
                .and_then(|id| strategy_names.get(&id).cloned())
                .unwrap_or_else(|| {
                    if let Some(s) = sid {
                        format!("Strategy #{}", s)
                    } else {
                        "Unplanned".into()
                    }
                });
            build_group(key, label, &vals)
        })
        .collect();
    by_strategy.sort_by(|a, b| b.trades.cmp(&a.trades));

    // By month
    let by_month_map = bucket(&rows, |r| r.month.clone());
    let mut by_month: Vec<GroupMetrics> = by_month_map
        .into_iter()
        .map(|(m, vals)| build_group(m.clone(), m, &vals))
        .collect();
    by_month.sort_by(|a, b| b.key.cmp(&a.key));

    // By weekday
    let wd_order = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let by_wd_map = bucket(&rows, |r| r.weekday);
    let mut by_weekday: Vec<GroupMetrics> = by_wd_map
        .into_iter()
        .map(|(w, vals)| build_group(w.into(), w.into(), &vals))
        .collect();
    by_weekday.sort_by_key(|g| wd_order.iter().position(|x| *x == g.key).unwrap_or(99));

    // By session
    let sess_order = ["Asia", "London", "NY"];
    let by_sess_map = bucket(&rows, |r| r.session);
    let mut by_session: Vec<GroupMetrics> = by_sess_map
        .into_iter()
        .map(|(s, vals)| build_group(s.into(), s.into(), &vals))
        .collect();
    by_session.sort_by_key(|g| sess_order.iter().position(|x| *x == g.key).unwrap_or(99));

    // Outcome counts
    let mut tp_hit_count = 0u32;
    let mut sl_hit_count = 0u32;
    let mut breakeven_count = 0u32;
    let mut manual_count = 0u32;

    for t in trades {
        if t.status != TradeStatus::Closed {
            continue;
        }
        match t.outcome {
            crate::domain::trade::TradeOutcome::TpHit => tp_hit_count += 1,
            crate::domain::trade::TradeOutcome::SlHit => sl_hit_count += 1,
            crate::domain::trade::TradeOutcome::Breakeven => breakeven_count += 1,
            crate::domain::trade::TradeOutcome::ManualClose => manual_count += 1,
            _ => {}
        }
    }

    Breakdown {
        by_strategy,
        by_month,
        by_weekday,
        by_session,
        plan_adherence_pct: round2(plan_pct),
        total_closed,
        planned_trades: planned,
        tp_hit_count,
        sl_hit_count,
        breakeven_count,
        manual_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::trade::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn closed(
        id: i64,
        entry: Decimal,
        exit: Decimal,
        qty: Decimal,
        at: &str,
        sid: Option<i64>,
    ) -> Trade {
        let mut t = empty_trade();
        t.id = id;
        t.entry_price = entry;
        t.initial_stop = entry - dec!(0.01);
        t.total_quantity = qty;
        t.status = TradeStatus::Closed;
        t.outcome = TradeOutcome::ManualClose;
        t.strategy_id = sid;
        t.opened_at = at.into();
        t.closed_at = Some(at.into());
        t.created_at = at.into();
        t.updated_at = at.into();
        t.legs = vec![
            TradeLeg {
                id: 1,
                kind: LegKind::Entry,
                price: entry,
                quantity: qty,
                executed_at: at.into(),
                cost_per_unit: Decimal::ZERO,
            },
            TradeLeg {
                id: 2,
                kind: LegKind::Exit,
                price: exit,
                quantity: qty,
                executed_at: at.into(),
                cost_per_unit: Decimal::ZERO,
            },
        ];
        t
    }

    #[test]
    fn weekday_of_works() {
        assert_eq!(weekday_of("2026-10-19T10:00:00Z"), "Mon");
        assert_eq!(weekday_of("2026-10-20T10:00:00Z"), "Tue");
        assert_eq!(weekday_of("2026-10-21T10:00:00Z"), "Wed");
        assert_eq!(weekday_of("2026-10-25T10:00:00Z"), "Sun");
        assert_eq!(weekday_of(""), "?");
    }

    #[test]
    fn empty_returns_zeros() {
        let b = compute_breakdown(&[], &Default::default());
        assert_eq!(b.total_closed, 0);
        assert_eq!(b.plan_adherence_pct, dec!(0));
        assert!(b.by_strategy.is_empty());
    }

    #[test]
    fn plan_adherence_counts_strategy_linked() {
        let t1 = closed(
            1,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T10:00:00Z",
            Some(1),
        );
        let t2 = closed(
            2,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T11:00:00Z",
            None,
        );
        let t3 = closed(
            3,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T12:00:00Z",
            Some(1),
        );
        let b = compute_breakdown(&[t1, t2, t3], &Default::default());
        assert_eq!(b.total_closed, 3);
        assert_eq!(b.planned_trades, 2);
    }

    #[test]
    fn weekday_and_month_grouping() {
        // 2026-10-19 = Mon
        // 2026-10-20 = Tue
        // 2026-11-04 = Wed  (different month AND different weekday)
        let t1 = closed(
            1,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-19T10:00:00Z",
            None,
        );
        let t2 = closed(
            2,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T10:00:00Z",
            None,
        );
        let t3 = closed(
            3,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-11-04T10:00:00Z",
            None,
        );
        let b = compute_breakdown(&[t1, t2, t3], &Default::default());
        assert!(b
            .by_month
            .iter()
            .any(|g| g.key == "2026-10" && g.trades == 2));
        assert!(b
            .by_month
            .iter()
            .any(|g| g.key == "2026-11" && g.trades == 1));
        assert!(
            b.by_weekday.iter().any(|g| g.key == "Mon" && g.trades == 1),
            "by_weekday: {:?}",
            b.by_weekday
                .iter()
                .map(|g| (&g.key, g.trades))
                .collect::<Vec<_>>()
        );
        assert!(b.by_weekday.iter().any(|g| g.key == "Tue" && g.trades == 1));
        assert!(b.by_weekday.iter().any(|g| g.key == "Wed" && g.trades == 1));
    }

    #[test]
    fn session_grouping_by_hour() {
        let t1 = closed(
            1,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T03:00:00Z",
            None,
        );
        let t2 = closed(
            2,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T10:00:00Z",
            None,
        );
        let t3 = closed(
            3,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T18:00:00Z",
            None,
        );
        let b = compute_breakdown(&[t1, t2, t3], &Default::default());
        assert!(b
            .by_session
            .iter()
            .any(|g| g.key == "Asia" && g.trades == 1));
        assert!(b
            .by_session
            .iter()
            .any(|g| g.key == "London" && g.trades == 1));
        assert!(b.by_session.iter().any(|g| g.key == "NY" && g.trades == 1));
    }

    #[test]
    fn strategy_metrics_aggregate() {
        let names: std::collections::HashMap<i64, String> =
            [(1, "ICT".to_string())].into_iter().collect();
        let t1 = closed(
            1,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T10:00:00Z",
            Some(1),
        );
        let t2 = closed(
            2,
            dec!(1.10),
            dec!(1.09),
            dec!(100),
            "2026-10-20T11:00:00Z",
            Some(1),
        );
        let b = compute_breakdown(&[t1, t2], &names);
        let g = b.by_strategy.iter().find(|g| g.key == "1").unwrap();
        assert_eq!(g.label, "ICT");
        assert_eq!(g.trades, 2);
        assert_eq!(g.wins, 1);
        assert_eq!(g.losses, 1);
        assert_eq!(g.win_rate_pct, dec!(50));
    }

    #[test]
    fn manual_pnl_used_in_breakdown() {
        let mut t = closed(
            1,
            dec!(1.10),
            dec!(1.11),
            dec!(100),
            "2026-10-20T10:00:00Z",
            Some(1),
        );
        t.manual_pnl = Some(dec!(5));
        let names: std::collections::HashMap<i64, String> =
            [(1, "ICT".to_string())].into_iter().collect();
        let b = compute_breakdown(&[t], &names);
        let g = b.by_strategy.iter().find(|g| g.key == "1").unwrap();
        assert_eq!(g.net_pnl, dec!(5));
    }
}
