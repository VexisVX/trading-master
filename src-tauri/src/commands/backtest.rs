//! Backtest Tauri commands (schema v2 — flat trades).
//! All price fields cross the boundary as `String` (Decimal-safe rule).
//! R-multiple and all stats are computed HERE in Rust with `Decimal`.

use crate::commands::AppState;
use crate::db::backtest as repo;
use crate::db::backtest::TradeFilter;
use rust_decimal::prelude::{FromPrimitive, FromStr, ToPrimitive};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tauri::State;

// ---------- Input payloads ----------

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FilterPayload {
    pub symbol: Option<String>,
    pub setup_tag: Option<String>,
    pub outcome: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

impl From<FilterPayload> for TradeFilter {
    fn from(f: FilterPayload) -> Self {
        TradeFilter {
            symbol: f.symbol,
            setup_tag: f.setup_tag,
            outcome: f.outcome,
            date_from: f.date_from,
            date_to: f.date_to,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupTagCount {
    pub tag: String,
    pub count: i64,
}

// ---------- Helpers ----------

fn parse_dec(s: &str, field: &str) -> Result<Decimal, String> {
    Decimal::from_str(s).map_err(|e| format!("{field}: {e}"))
}

/// R = (exit - entry) / |entry - stop|, negated for short. Decimal only.
pub fn compute_r_decimal(
    entry: &str,
    stop: &str,
    exit: &str,
    side: &str,
) -> Result<Decimal, String> {
    let e = parse_dec(entry, "entry")?;
    let s = parse_dec(stop, "stop")?;
    let x = parse_dec(exit, "exit")?;
    let risk = (e - s).abs();
    if risk.is_zero() {
        return Err("stop equals entry — zero risk".into());
    }
    let raw = (x - e) / risk;
    Ok(if side == "short" { -raw } else { raw })
}

/// Auto-classify outcome from exit vs entry vs stop when exit_price is set
/// and outcome is not explicitly given.
fn classify_outcome(entry: &str, stop: &str, exit: &str, side: &str) -> Option<String> {
    let e = Decimal::from_str(entry).ok()?;
    let s = Decimal::from_str(stop).ok()?;
    let x = Decimal::from_str(exit).ok()?;
    let eps = Decimal::new(1, 6); // 0.000001 tolerance
    if (x - e).abs() < eps {
        return Some("be".into());
    }
    let is_win = if side == "short" { x < e } else { x > e };
    if is_win {
        Some("tp".into())
    } else if (x - s).abs() < eps {
        Some("sl".into())
    } else {
        Some("sl".into()) // exited past stop — treat as loss
    }
}

// ---------- Commands ----------

#[tauri::command]
pub fn backtest_trades_list(
    state: State<'_, AppState>,
    filter: Option<FilterPayload>,
) -> Result<Vec<repo::BacktestTrade>, String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    let f: TradeFilter = filter.unwrap_or_default().into();
    repo::list_trades(&conn, &f).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn backtest_trade_create(
    state: State<'_, AppState>,
    symbol: String,
    side: String,
    entry_price: String,
    stop_price: String,
    take_profit: Option<String>,
    exit_price: Option<String>,
    manual_r: Option<String>,
    setup_tag: Option<String>,
    opened_at: Option<String>,
    closed_at: Option<String>,
    notes: Option<String>,
) -> Result<i64, String> {
    if side != "long" && side != "short" {
        return Err("side must be 'long' or 'short'".into());
    }
    if symbol.trim().is_empty() {
        return Err("symbol is required".into());
    }
    // Determine source first.
    //   - exit_price given  → Rust computes R (authoritative) — entry+stop required
    //   - manual_r given    → user-provided R, no computation — entry+stop NOT required
    //   - neither given     → open trade, R = NULL — entry+stop required (for the trade)
    let exit_clean = exit_price.as_deref().filter(|s| !s.is_empty());
    let manual_clean = manual_r.as_deref().filter(|s| !s.is_empty());

    if exit_clean.is_some() && manual_clean.is_some() {
        return Err("give either exit_price or manual_r, not both".into());
    }

    let is_manual = manual_clean.is_some();

    // entry/stop are required except in manual R mode.
    // In manual mode, blank entry/stop is accepted and stored as "0".
    if !is_manual {
        let _ = parse_dec(&entry_price, "entry")?;
        let _ = parse_dec(&stop_price, "stop")?;
    } else {
        // if provided anyway, still must be numeric
        if !entry_price.trim().is_empty() {
            let _ = parse_dec(&entry_price, "entry")?;
        }
        if !stop_price.trim().is_empty() {
            let _ = parse_dec(&stop_price, "stop")?;
        }
    }
    if let Some(tp) = take_profit.as_deref().filter(|s| !s.is_empty()) {
        let _ = parse_dec(tp, "take_profit")?;
    }

    let mut outcome: Option<String> = None;
    let mut r_multiple: Option<String> = None;

    if let Some(x) = exit_clean {
        let r = compute_r_decimal(&entry_price, &stop_price, x, &side)?;
        r_multiple = Some(r.round_dp(2).to_string());
        outcome = classify_outcome(&entry_price, &stop_price, x, &side);
    } else if let Some(r_str) = manual_clean {
        // Validate it parses as Decimal; keep the user's value (rounded to 2dp).
        let r = parse_dec(r_str, "manual_r")?;
        let r2 = r.round_dp(2);
        outcome = if r2 > Decimal::ZERO {
            Some("tp".into())
        } else if r2 < Decimal::ZERO {
            Some("sl".into())
        } else {
            Some("be".into())
        };
        r_multiple = Some(r2.to_string());
    }

    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    let opened = opened_at
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| now.clone());
    // In manual R mode, blank entry/stop → stored as "0" (schema stays NOT NULL).
    let entry_val: String = if is_manual && entry_price.trim().is_empty() {
        "0".to_string()
    } else {
        entry_price.trim().to_string()
    };
    let stop_val: String = if is_manual && stop_price.trim().is_empty() {
        "0".to_string()
    } else {
        stop_price.trim().to_string()
    };

    let payload = repo::NewBacktestTrade {
        symbol: symbol.trim(),
        side: &side,
        entry_price: &entry_val,
        stop_price: &stop_val,
        take_profit: take_profit.as_deref().filter(|s| !s.is_empty()),
        exit_price: exit_price.as_deref().filter(|s| !s.is_empty()),
        outcome: outcome.as_deref(),
        r_multiple: r_multiple.as_deref(),
        setup_tag: setup_tag
            .as_deref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty()),
        opened_at: &opened,
        closed_at: closed_at.as_deref().filter(|s| !s.is_empty()),
        notes: notes.as_deref().filter(|s| !s.is_empty()),
    };
    repo::create_trade(&conn, &payload).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn backtest_trade_delete(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    repo::delete_trade(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn backtest_reset(state: State<'_, AppState>) -> Result<(), String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    repo::delete_all(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn backtest_setup_tags(state: State<'_, AppState>) -> Result<Vec<SetupTagCount>, String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    let rows = repo::list_setup_tags(&conn).map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(tag, count)| SetupTagCount { tag, count })
        .collect())
}

#[tauri::command]
pub fn backtest_symbols(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    repo::list_symbols(&conn).map_err(|e| e.to_string())
}

// ---------- Stats (computed here, Decimal) ----------

#[derive(Debug, Clone, Serialize)]
pub struct StatsPayload {
    pub count: i64,
    pub wins: i64,
    pub losses: i64,
    pub breakeven: i64,
    /// "0.52" — Decimal string
    pub win_rate: String,
    /// "0.18" — Decimal string (R)
    pub expectancy: String,
    /// "1.42" — Decimal string, "inf" if no losses
    pub profit_factor: String,
    /// "23.5" — Decimal string (R)
    pub total_r: String,
    /// Equity curve: cumulative R over trades in chronological order
    pub equity: Vec<EquityPoint>,
    /// Histogram buckets by R
    pub histogram: Vec<HistBucket>,
    /// By setup tag: only tags with >= 5 trades
    pub by_setup: Vec<SetupStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EquityPoint {
    pub i: i64,
    pub r: String, // cumulative, Decimal string
}

#[derive(Debug, Clone, Serialize)]
pub struct HistBucket {
    pub label: String, // "-2", "-1", "0", "1", ...
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStats {
    pub tag: String,
    pub count: i64,
    pub avg_r: String,
    pub win_rate: String,
}

#[tauri::command]
pub fn backtest_stats(
    state: State<'_, AppState>,
    filter: Option<FilterPayload>,
) -> Result<StatsPayload, String> {
    let conn = state.backtest_db.lock().map_err(|e| e.to_string())?;
    let f: TradeFilter = filter.unwrap_or_default().into();
    let trades = repo::list_trades(&conn, &f).map_err(|e| e.to_string())?;

    // Chronological order for equity (list_trades returns DESC; reverse)
    let mut chrono: Vec<_> = trades.iter().filter(|t| t.r_multiple.is_some()).collect();
    chrono.reverse();

    let mut rs: Vec<Decimal> = Vec::new();
    let mut wins = 0i64;
    let mut losses = 0i64;
    let mut breakeven = 0i64;
    let mut gross_win = Decimal::ZERO;
    let mut gross_loss = Decimal::ZERO;

    for t in &trades {
        let Some(r_str) = t.r_multiple.as_deref() else {
            continue;
        };
        let Ok(r) = Decimal::from_str(r_str) else {
            continue;
        };
        rs.push(r);
        if r > Decimal::ZERO {
            wins += 1;
            gross_win += r;
        } else if r < Decimal::ZERO {
            losses += 1;
            gross_loss += r.abs();
        } else {
            breakeven += 1;
        }
    }

    let count = rs.len() as i64;
    let win_rate = if count > 0 {
        Decimal::from_i64(wins).unwrap() / Decimal::from_i64(count).unwrap()
    } else {
        Decimal::ZERO
    };
    let total_r = rs.iter().copied().sum::<Decimal>();
    let expectancy = if count > 0 {
        total_r / Decimal::from_i64(count).unwrap()
    } else {
        Decimal::ZERO
    };
    let profit_factor = if gross_loss > Decimal::ZERO {
        (gross_win / gross_loss).round_dp(2).to_string()
    } else if gross_win > Decimal::ZERO {
        "inf".into()
    } else {
        "0".into()
    };

    // Equity curve
    let mut equity = Vec::with_capacity(chrono.len());
    let mut cum = Decimal::ZERO;
    for (i, t) in chrono.iter().enumerate() {
        if let Some(r_str) = t.r_multiple.as_deref() {
            if let Ok(r) = Decimal::from_str(r_str) {
                cum += r;
                equity.push(EquityPoint {
                    i: i as i64,
                    r: cum.round_dp(2).to_string(),
                });
            }
        }
    }

    // Histogram (integer R buckets, -5..=+5 and beyond collapse)
    let buckets: &[(&str, i64)] = &[
        ("≤-3", i64::MIN),
        ("-2", -2),
        ("-1", -1),
        ("0", 0),
        ("+1", 1),
        ("+2", 2),
        ("+3", 3),
        ("≥+4", i64::MAX),
    ];
    let mut histogram = Vec::new();
    for (label, bound) in buckets {
        let c = rs
            .iter()
            .filter(|r| {
                let n = r.round().to_i64().unwrap_or(0);
                match *label {
                    "≤-3" => n <= -3,
                    "≥+4" => n >= 4,
                    _ => n == *bound,
                }
            })
            .count() as i64;
        histogram.push(HistBucket {
            label: (*label).to_string(),
            count: c,
        });
    }

    // By setup (only tags with >= 5 trades)
    use std::collections::HashMap;
    let mut by_tag: HashMap<String, Vec<Decimal>> = HashMap::new();
    for t in &trades {
        let Some(tag) = t.setup_tag.as_deref().filter(|s| !s.is_empty()) else {
            continue;
        };
        let Some(r_str) = t.r_multiple.as_deref() else {
            continue;
        };
        if let Ok(r) = Decimal::from_str(r_str) {
            by_tag.entry(tag.to_string()).or_default().push(r);
        }
    }
    let mut by_setup: Vec<SetupStats> = by_tag
        .into_iter()
        .filter(|(_, v)| v.len() >= 5)
        .map(|(tag, v)| {
            let n = v.len() as i64;
            let sum: Decimal = v.iter().copied().sum();
            let avg = (sum / Decimal::from_i64(n).unwrap()).round_dp(2);
            let w = v.iter().filter(|r| **r > Decimal::ZERO).count() as i64;
            let wr = Decimal::from_i64(w).unwrap() / Decimal::from_i64(n).unwrap();
            SetupStats {
                tag,
                count: n,
                avg_r: avg.to_string(),
                win_rate: wr.round_dp(2).to_string(),
            }
        })
        .collect();
    by_setup.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(StatsPayload {
        count,
        wins,
        losses,
        breakeven,
        win_rate: win_rate.round_dp(3).to_string(),
        expectancy: expectancy.round_dp(2).to_string(),
        profit_factor,
        total_r: total_r.round_dp(2).to_string(),
        equity,
        histogram,
        by_setup,
    })
}
