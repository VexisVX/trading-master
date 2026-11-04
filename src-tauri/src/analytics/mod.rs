//! Aggregate analytics over closed trades.
//! Uses the risk engine (Decimal-only) — never f64.

pub mod breakdown;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::domain::trade::{Side, Trade, TradeStatus};
use crate::risk::drawdown::{compute_drawdown, DrawdownResult};
use crate::risk::r_multiple::{compute_r, Exit, TradeR};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquityPoint {
    pub t: usize,
    pub v: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RBucket {
    pub label: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRDetail {
    pub id: i64,
    pub symbol: String,
    pub side: Side,
    pub r_multiple: Decimal,
    pub net_pnl: Decimal,
    pub closed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub initial_balance: Decimal,
    pub current_equity: Decimal,
    pub net_pnl: Decimal,
    pub total_trades: u32,
    pub open_trades: u32,
    pub closed_trades: u32,
    pub wins: u32,
    pub losses: u32,
    pub win_rate_pct: Decimal,
    pub gross_profit: Decimal,
    pub gross_loss: Decimal,
    pub profit_factor: Decimal,
    pub avg_win_r: Decimal,
    pub avg_loss_r: Decimal,
    pub expectancy_r: Decimal,
    pub max_drawdown: Decimal,
    pub max_drawdown_pct: Decimal,
    pub equity_curve: Vec<EquityPoint>,
    pub drawdown_curve: Vec<EquityPoint>,
    pub r_distribution: Vec<RBucket>,
    pub r_details: Vec<TradeRDetail>,

    // --- R:R + outcome analysis ---
    pub avg_planned_rr: Decimal,
    pub avg_realized_r: Decimal,
    pub plan_adherence_pct: Decimal,
    pub tp_hit_count: u32,
    pub sl_hit_count: u32,
    pub breakeven_count: u32,
    pub manual_count: u32,
    pub early_exit_count: u32,
}

/// Compute the aggregate summary for a list of trades.
/// `initial_balance` is the account starting capital.
/// Only closed trades affect P&L; open trades count toward `open_trades`.
pub fn summarize(trades: &[Trade], initial_balance: Decimal) -> Summary {
    let mut closed_count = 0u32;
    let mut open_count = 0u32;

    let mut wins = 0u32;
    let mut losses = 0u32;
    let mut gross_profit = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;
    let mut net_pnl = Decimal::ZERO;

    let mut r_values: Vec<Decimal> = Vec::new();
    let mut r_details: Vec<TradeRDetail> = Vec::new();
    let mut equity = initial_balance;
    let mut equity_curve: Vec<EquityPoint> = vec![EquityPoint { t: 0, v: equity }];

    let mut closed: Vec<&Trade> = trades
        .iter()
        .filter(|t| t.status == TradeStatus::Closed)
        .collect();
    closed.sort_by(|a, b| {
        a.closed_at
            .as_deref()
            .unwrap_or("")
            .cmp(b.closed_at.as_deref().unwrap_or(""))
    });

    for t in trades.iter() {
        match t.status {
            TradeStatus::Open => open_count += 1,
            TradeStatus::Closed => closed_count += 1,
        }
    }

    for t in &closed {
        let r_result = compute_trade_r(t);
        let (r_val, trade_net) = match r_result {
            Ok((r, n)) => (r, n),
            Err(_) => continue,
        };

        r_values.push(r_val);
        if r_val > Decimal::ZERO {
            wins += 1;
            gross_profit += trade_net;
        } else if r_val < Decimal::ZERO {
            losses += 1;
            gross_loss += trade_net.abs();
        }
        net_pnl += trade_net;
        equity += trade_net;
        equity_curve.push(EquityPoint {
            t: equity_curve.len(),
            v: equity,
        });

        r_details.push(TradeRDetail {
            id: t.id,
            symbol: t.symbol.clone(),
            side: t.side,
            r_multiple: r_val,
            net_pnl: trade_net,
            closed_at: t.closed_at.clone().unwrap_or_default(),
        });
    }

    let total_decided = wins + losses;
    let win_rate = if total_decided == 0 {
        Decimal::ZERO
    } else {
        (Decimal::from(wins) / Decimal::from(total_decided)) * Decimal::from(100)
    };

    let profit_factor = if gross_loss.is_zero() {
        if gross_profit > Decimal::ZERO {
            Decimal::from(999)
        } else {
            Decimal::ZERO
        }
    } else {
        gross_profit / gross_loss
    };

    let r_count = Decimal::from(r_values.len() as u32);
    let avg_r = if r_values.is_empty() {
        Decimal::ZERO
    } else {
        r_values.iter().copied().sum::<Decimal>() / r_count
    };

    let (wins_r, losses_r): (Vec<Decimal>, Vec<Decimal>) =
        r_values.iter().copied().partition(|r| *r > Decimal::ZERO);
    let avg_win_r = if wins_r.is_empty() {
        Decimal::ZERO
    } else {
        wins_r.iter().copied().sum::<Decimal>() / Decimal::from(wins_r.len() as u32)
    };
    let avg_loss_r = if losses_r.is_empty() {
        Decimal::ZERO
    } else {
        losses_r.iter().copied().sum::<Decimal>() / Decimal::from(losses_r.len() as u32)
    };

    let r_distribution = build_r_buckets(&r_values);

    let equity_series: Vec<Decimal> = equity_curve.iter().map(|p| p.v).collect();
    let dd_result: DrawdownResult = compute_drawdown(&equity_series).unwrap_or(DrawdownResult {
        series: vec![],
        max_drawdown: Decimal::ZERO,
        max_drawdown_pct: Decimal::ZERO,
        max_drawdown_index: 0,
    });
    let drawdown_curve: Vec<EquityPoint> = dd_result
        .series
        .iter()
        .map(|p| EquityPoint {
            t: p.index,
            v: p.drawdown_pct * Decimal::from(100),
        })
        .collect();

    // --- R:R and outcome analysis ---
    let mut planned_rr_sum = Decimal::ZERO;
    let mut planned_rr_count = Decimal::ZERO;
    let mut realized_r_sum = Decimal::ZERO;
    let mut realized_r_count = Decimal::ZERO;
    let mut adherent = 0u32;
    let mut early = 0u32;
    let mut tp_hit = 0u32;
    let mut sl_hit = 0u32;
    let mut breakeven = 0u32;
    let mut manual = 0u32;

    for t in &closed {
        // Outcome counts
        match t.outcome {
            crate::domain::trade::TradeOutcome::TpHit => tp_hit += 1,
            crate::domain::trade::TradeOutcome::SlHit => sl_hit += 1,
            crate::domain::trade::TradeOutcome::Breakeven => breakeven += 1,
            crate::domain::trade::TradeOutcome::ManualClose => manual += 1,
            _ => {}
        }

        // Planned R:R (only when TP exists)
        if let Some(p) = crate::risk::trade_planned_rr(t) {
            planned_rr_sum += p;
            planned_rr_count += Decimal::ONE;
        }

        // Realized R
        if let Some(r) = crate::risk::realized_r(t) {
            realized_r_sum += r;
            realized_r_count += Decimal::ONE;

            // Adherent = outcome is TP hit or manual_close with r >= 0.8 * planned
            if let Some(p) = crate::risk::trade_planned_rr(t) {
                let threshold = p * Decimal::new(8, 1); // 0.8
                if r >= threshold {
                    adherent += 1;
                } else if r < p * Decimal::new(5, 1) {
                    // less than 0.5 of plan → early exit
                    early += 1;
                }
            } else if t.outcome == crate::domain::trade::TradeOutcome::TpHit {
                adherent += 1;
            }
        }
    }

    let avg_planned = if planned_rr_count.is_zero() {
        Decimal::ZERO
    } else {
        planned_rr_sum / planned_rr_count
    };
    let avg_realized = if realized_r_count.is_zero() {
        Decimal::ZERO
    } else {
        realized_r_sum / realized_r_count
    };
    let denom = (tp_hit + sl_hit + manual + breakeven).max(1);
    let adherence_pct = Decimal::from(adherent) / Decimal::from(denom) * Decimal::from(100);

    Summary {
        initial_balance,
        current_equity: equity,
        net_pnl,
        total_trades: closed_count + open_count,
        open_trades: open_count,
        closed_trades: closed_count,
        wins,
        losses,
        win_rate_pct: win_rate,
        gross_profit,
        gross_loss,
        profit_factor,
        avg_win_r,
        avg_loss_r,
        expectancy_r: avg_r,
        max_drawdown: dd_result.max_drawdown,
        max_drawdown_pct: dd_result.max_drawdown_pct * Decimal::from(100),
        equity_curve,
        drawdown_curve,
        r_distribution,
        r_details,
        avg_planned_rr: avg_planned,
        avg_realized_r: avg_realized,
        plan_adherence_pct: adherence_pct,
        tp_hit_count: tp_hit,
        sl_hit_count: sl_hit,
        breakeven_count: breakeven,
        manual_count: manual,
        early_exit_count: early,
    }
}

fn compute_trade_r(t: &Trade) -> Result<(Decimal, Decimal), &'static str> {
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
        return Err("no legs");
    }

    let total_qty: Decimal = entries.iter().map(|l| l.quantity).sum();
    if total_qty != t.total_quantity {
        return Err("qty mismatch");
    }

    let cost_per_unit: Decimal =
        exits.iter().map(|l| l.cost_per_unit).sum::<Decimal>() / Decimal::from(exits.len() as u32);

    let trade_r = TradeR {
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

    let result = compute_r(&trade_r).map_err(|_| "compute_r failed")?;
    // Prefer user-entered P&L over price-derived P&L for the net figure.
    // Recompute R if manual_pnl is set: R = manual_pnl / initial_risk.
    if let Some(manual) = t.manual_pnl {
        let initial_risk = (t.entry_price - t.initial_stop).abs() * t.total_quantity;
        if !initial_risk.is_zero() {
            let r = manual / initial_risk;
            return Ok((r, manual));
        }
        return Ok((Decimal::ZERO, manual));
    }
    Ok((result.r_multiple, result.net_pnl))
}

fn build_r_buckets(r_values: &[Decimal]) -> Vec<RBucket> {
    let labels = [
        "≤ -2R", "-2R..-1R", "-1R..0R", "0R..1R", "1R..2R", "2R..3R", "> 3R",
    ];
    let mut counts = [0u32; 7];

    for &r in r_values {
        let idx = if r <= Decimal::from(-2) {
            0
        } else if r <= Decimal::from(-1) {
            1
        } else if r < Decimal::ZERO {
            2
        } else if r < Decimal::ONE {
            3
        } else if r < Decimal::from(2) {
            4
        } else if r < Decimal::from(3) {
            5
        } else {
            6
        };
        counts[idx] += 1;
    }

    labels
        .iter()
        .zip(counts.iter())
        .map(|(l, c)| RBucket {
            label: l.to_string(),
            count: *c,
        })
        .collect()
}

// Silence: FromStr kept for future use (String -> Decimal)
#[allow(dead_code)]
fn _keep_from_str() {
    let _ = Decimal::from_str("0");
}
