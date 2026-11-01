//! Tauri commands for the Equity Engine.
//! All numeric fields cross the boundary as strings (Decimal-safe).

use crate::commands::AppState;
use crate::db::equity as repo;
use crate::equity::{
    config::EquityConfig,
    engine::{evaluate, TradeProposal},
    error::{DecisionReason, EquityError},
    risk_engine::{m_dd, m_loss},
    state::{AccountPhase, AccountState, EquitySnapshot},
    TradeDecision,
};
use rust_decimal::prelude::FromStr;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tauri::State;

// ---------- DTOs (all numeric as String) ----------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SnapshotCreateInput {
    pub equity: String,
    pub initial_balance: String,
    pub low_watermark: String,
    pub day_start_equity: String,
    pub open_portfolio_risk: String,
    pub daily_loss: String,
    pub consecutive_losses: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProposalInput {
    pub entry: String,
    pub stop: String,
    pub take_profit: Option<String>,
    pub contract_multiplier: String,
    pub commission_per_unit: String,
    pub slippage_buffer: String,
}

#[derive(Debug, Serialize)]
pub struct DecisionOut {
    pub allowed: bool,
    pub reason: DecisionReason,
    pub r_allowed: String,
    pub size: Option<String>,
    pub rr: Option<String>,
    pub phase: String,
    pub dd_pct: String,
    pub f_hybrid: String,
    pub loss_capacity: String,
    pub snapshot_id: Option<i64>,
    /// Full risk breakdown for "Why this size?" — optional so existing
    /// clients that don't care about it can ignore it.
    pub risk_detail: Option<RiskDetail>,
}

#[derive(Debug, Serialize)]
pub struct RiskDetail {
    // All money values as Decimal-string; multipliers as decimal-string.
    pub equity: String,
    pub r_base: String,
    pub m_dd: String,
    pub m_loss: String,
    pub r_adaptive: String,
    pub r_portfolio_cap: String,
    pub r_daily_cap: String,
    pub r_floor_cap: String,
    pub r_prop_cap: String,
    pub r_allowed: String,
    pub entry_minus_stop: String,
    pub contract_multiplier: String,
    pub per_unit_cost: String,
}

#[derive(Debug, Serialize)]
pub struct DecisionRowOut {
    pub id: i64,
    pub snapshot_id: i64,
    pub allowed: bool,
    pub reason_kind: String,
    pub reason_detail: Option<String>,
    pub r_allowed: Option<String>,
    pub size: Option<String>,
    pub created_at: String,
    pub risk_reduced: bool,
    pub phase: String,
}

// ---------- Helpers ----------

fn parse_dec(s: &str, name: &str) -> Result<Decimal, String> {
    Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
}

fn load_or_default_config(state: &State<'_, AppState>) -> Result<EquityConfig, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    match repo::load_config(&conn).map_err(|e| e.to_string())? {
        Some(json) => serde_json::from_str(&json).map_err(|e| format!("config parse: {e}")),
        None => Ok(EquityConfig::default_v1()),
    }
}

fn phase_str(p: AccountPhase) -> &'static str {
    match p {
        AccountPhase::Normal => "normal",
        AccountPhase::Caution => "caution",
        AccountPhase::Recovery => "recovery",
        AccountPhase::Paused => "paused",
        AccountPhase::Disabled => "disabled",
    }
}

fn decision_row_to_out(row: &repo::DecisionRowOut) -> DecisionRowOut {
    DecisionRowOut {
        id: row.id,
        snapshot_id: row.snapshot_id,
        allowed: row.allowed,
        reason_kind: row.reason_kind.clone(),
        reason_detail: row.reason_detail.clone(),
        r_allowed: row.r_allowed.clone(),
        size: row.size.clone(),
        created_at: row.created_at.clone(),
        risk_reduced: row.risk_reduced,
        phase: row.phase.clone(),
    }
}

// ---------- Commands ----------

#[tauri::command]
pub fn equity_config_get(state: State<'_, AppState>) -> Result<EquityConfig, String> {
    load_or_default_config(&state)
}

#[tauri::command]
pub fn equity_config_set(
    state: State<'_, AppState>,
    config: EquityConfig,
) -> Result<(), String> {
    config.validate()?;
    let json = serde_json::to_string(&config).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::save_config(&conn, &json).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn equity_snapshot_create(
    state: State<'_, AppState>,
    input: SnapshotCreateInput,
) -> Result<i64, String> {
    let equity = parse_dec(&input.equity, "equity")?;
    let initial = parse_dec(&input.initial_balance, "initial_balance")?;
    let low = parse_dec(&input.low_watermark, "low_watermark")?;
    let day_start = parse_dec(&input.day_start_equity, "day_start_equity")?;
    let _ = parse_dec(&input.open_portfolio_risk, "open_portfolio_risk")?;
    let _ = parse_dec(&input.daily_loss, "daily_loss")?;

    if equity < Decimal::ZERO {
        return Err("equity cannot be negative".into());
    }
    if initial <= Decimal::ZERO {
        return Err("initial_balance must be positive".into());
    }
    if input.consecutive_losses < 0 {
        return Err("consecutive_losses cannot be negative".into());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // High watermark = max(previous HWM, current equity, initial_balance).
    // If no previous snapshot exists, use max(equity, initial_balance).
    let prev = repo::latest_snapshot(&conn).map_err(|e| e.to_string())?;
    let hwm = match &prev {
        Some(p) => {
            let prev_hwm = parse_dec(&p.high_watermark, "prev_hwm")?;
            prev_hwm.max(equity).max(initial).to_string()
        }
        None => equity.max(initial).to_string(),
    };

    let taken_at = chrono::Utc::now().to_rfc3339();
    // Compute AccountState for new fields (phase/protected/floor/LC)
    let cfg = load_or_default_config(&state)?;
    let snap_domain = EquitySnapshot {
        id: 0,
        equity: equity.to_string(),
        initial_balance: initial.to_string(),
        high_watermark: hwm.clone(),
        low_watermark: low.to_string(),
        day_start_equity: day_start.to_string(),
        open_portfolio_risk: input.open_portfolio_risk.clone(),
        daily_loss: input.daily_loss.clone(),
        consecutive_losses: input.consecutive_losses,
        taken_at: taken_at.clone(),
    };
    let parsed = snap_domain.parse().map_err(|e: String| format!("snapshot parse: {e}"))?;
    let acct_state = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("AccountState: {e}"))?;

    let phase_s = phase_str(acct_state.phase).to_string();
    let prot_s = acct_state.profit_retention.to_string();
    let floor_s = acct_state.f_hybrid.to_string();
    let lc_s = acct_state.loss_capacity.to_string();

    let row = repo::SnapshotRow {
        equity: &equity.to_string(),
        initial_balance: &initial.to_string(),
        high_watermark: &hwm,
        low_watermark: &low.to_string(),
        day_start_equity: &day_start.to_string(),
        open_portfolio_risk: &input.open_portfolio_risk,
        daily_loss: &input.daily_loss,
        consecutive_losses: input.consecutive_losses,
        taken_at: &taken_at,
        phase: Some(&phase_s),
        protected_profit: Some(&prot_s),
        equity_floor: Some(&floor_s),
        loss_capacity: Some(&lc_s),
    };

    let id = repo::insert_snapshot(&conn, &row).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn equity_snapshot_latest(
    state: State<'_, AppState>,
) -> Result<Option<repo::EquitySnapshotRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::latest_snapshot(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn equity_evaluate(
    state: State<'_, AppState>,
    proposal: Option<ProposalInput>,
) -> Result<DecisionOut, String> {
    // 1. Load config + latest snapshot
    let cfg = load_or_default_config(&state)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let snap_row = repo::latest_snapshot(&conn).map_err(|e| e.to_string())?
        .ok_or_else(|| "no snapshot yet — create one first".to_string())?;

    let snap_domain = EquitySnapshot {
        id: snap_row.id,
        equity: snap_row.equity.clone(),
        initial_balance: snap_row.initial_balance.clone(),
        high_watermark: snap_row.high_watermark.clone(),
        low_watermark: snap_row.low_watermark.clone(),
        day_start_equity: snap_row.day_start_equity.clone(),
        open_portfolio_risk: snap_row.open_portfolio_risk.clone(),
        daily_loss: snap_row.daily_loss.clone(),
        consecutive_losses: snap_row.consecutive_losses,
        taken_at: snap_row.taken_at.clone(),
    };
    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let state_acct = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("state: {e}"))?;

    // 2. Build proposal (clone original for later risk_detail)
    let proposal_for_detail = proposal.clone();
    // 2. Build proposal
    let prop = match proposal {
        Some(p) => Some(TradeProposal {
            entry: p.entry,
            stop: p.stop,
            take_profit: p.take_profit,
            contract_multiplier: p.contract_multiplier,
            commission_per_unit: p.commission_per_unit,
            slippage_buffer: p.slippage_buffer,
        }),
        None => None,
    };

    // 3. Evaluate
    let decision: TradeDecision = evaluate(&state_acct, &cfg, prop.as_ref())
        .map_err(|e: EquityError| e.to_string())?;

    // 4. Persist decision
    let cfg_json = serde_json::to_string(&cfg).map_err(|e| e.to_string())?;
    let reason_kind = format!("{:?}", decision.reason);
    let reason_detail = match &decision.reason {
        DecisionReason::EquityBelowFloor { .. }
        | DecisionReason::DailyLossExceeded { .. }
        | DecisionReason::PortfolioRiskFull { .. }
        | DecisionReason::SnapshotTooOld { .. } => Some(reason_kind.clone()),
        _ => None,
    };
    // Determine risk_reduced: adaptive > allowed means we cut risk
    let risk_reduced = match &decision.risk_detail {
        Some(r) => {
            let adaptive = r.r_adaptive;
            let allowed = r.r_allowed;
            adaptive > allowed
        }
        None => false,
    };
    let phase_for_db = phase_str(state_acct.phase).to_string();

    let drow = repo::DecisionRow {
        snapshot_id: snap_row.id,
        allowed: decision.allowed,
        reason_kind: &reason_kind,
        reason_detail: reason_detail.as_deref(),
        r_allowed: if decision.r_allowed.is_empty() {
            None
        } else {
            Some(decision.r_allowed.as_str())
        },
        size: decision.size.as_deref(),
        config_json: &cfg_json,
        risk_reduced,
        phase: &phase_for_db,
    };
    let _ = repo::insert_decision(&conn, &drow);

    // 5. Build output
    let _ = m_dd(&state_acct, &cfg);
    let _ = m_loss(0, &cfg);

    Ok(DecisionOut {
        allowed: decision.allowed,
        reason: decision.reason,
        r_allowed: decision.r_allowed,
        size: decision.size,
        rr: decision.rr,
        phase: phase_str(state_acct.phase).to_string(),
        dd_pct: state_acct.dd_pct.to_string(),
        f_hybrid: state_acct.f_hybrid.to_string(),
        loss_capacity: state_acct.loss_capacity.to_string(),
        snapshot_id: Some(snap_row.id),
        risk_detail: decision.risk_detail.as_ref().map(|r| {
            // Compute entry_minus_stop and per_unit_cost from proposal
            let (ems, mult, per_unit) = match &proposal_for_detail {
                Some(p) => {
                    let entry = parse_dec(&p.entry, "entry").unwrap_or(Decimal::ZERO);
                    let stop = parse_dec(&p.stop, "stop").unwrap_or(Decimal::ZERO);
                    let mult = parse_dec(&p.contract_multiplier, "mult").unwrap_or(Decimal::ONE);
                    let comm = parse_dec(&p.commission_per_unit, "comm").unwrap_or(Decimal::ZERO);
                    let slip = parse_dec(&p.slippage_buffer, "slip").unwrap_or(Decimal::ZERO);
                    let ems = (entry - stop).abs();
                    let per_unit = ems * mult + comm + slip;
                    (ems, mult, per_unit)
                }
                None => (Decimal::ZERO, Decimal::ONE, Decimal::ZERO),
            };
            RiskDetail {
                equity: state_acct.snapshot.equity.to_string(),
                r_base: r.r_base.to_string(),
                m_dd: r.m_dd.to_string(),
                m_loss: r.m_loss.to_string(),
                r_adaptive: r.r_adaptive.to_string(),
                r_portfolio_cap: r.r_portfolio_cap.to_string(),
                r_daily_cap: r.r_daily_cap.to_string(),
                r_floor_cap: r.r_floor_cap.to_string(),
                r_prop_cap: r.r_prop_cap.to_string(),
                r_allowed: r.r_allowed.to_string(),
                entry_minus_stop: ems.to_string(),
                contract_multiplier: mult.to_string(),
                per_unit_cost: per_unit.to_string(),
            }
        }),
    })
}

#[tauri::command]
pub fn equity_decisions_list(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<DecisionRowOut>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(50).clamp(1, 500);
    let rows = repo::list_decisions(&conn, n).map_err(|e| e.to_string())?;
    Ok(rows.iter().map(decision_row_to_out).collect())
}

#[tauri::command]
pub fn equity_decision_record(
    state: State<'_, AppState>,
    snapshot_id: i64,
    allowed: bool,
    reason_kind: String,
    reason_detail: Option<String>,
    r_allowed: Option<String>,
    size: Option<String>,
) -> Result<i64, String> {
    let cfg = load_or_default_config(&state)?;
    let cfg_json = serde_json::to_string(&cfg).map_err(|e| e.to_string())?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let drow = repo::DecisionRow {
        snapshot_id,
        allowed,
        reason_kind: &reason_kind,
        reason_detail: reason_detail.as_deref(),
        r_allowed: r_allowed.as_deref(),
        size: size.as_deref(),
        config_json: &cfg_json,
        risk_reduced: false,
        phase: "normal",
    };
    repo::insert_decision(&conn, &drow).map_err(|e| e.to_string())
}

// ============================================================
// AUTO SNAPSHOT — derived from real trades (no manual input)
// ============================================================

#[tauri::command]
pub fn equity_auto_snapshot_create(
    state: State<'_, AppState>,
    account_id: Option<i64>,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // 1) account: use given account_id, or first active
    let (account_id, initial): (i64, Decimal) = match account_id {
        Some(aid) => conn
            .query_row(
                "SELECT id, initial_balance FROM accounts WHERE id = ?1",
                [aid],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|e| format!("account {} not found: {}", aid, e))
            .and_then(|(id, ib)| Ok((id, parse_dec(&ib, "initial_balance")?)))?,
        None => conn
            .query_row(
                "SELECT id, initial_balance FROM accounts WHERE is_active = 1 ORDER BY id LIMIT 1",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|e| format!("no active account: {}", e))
            .and_then(|(id, ib)| Ok((id, parse_dec(&ib, "initial_balance")?)))?,
    };

    // 2) sum of closed-trade PnL (manual_pnl takes precedence; otherwise null)
    let mut stmt = conn
        .prepare(
            "SELECT manual_pnl FROM trades
             WHERE account_id = ?1 AND status = 'closed'",
        )
        .map_err(|e| e.to_string())?;
    let pnls = stmt
        .query_map([account_id], |r| r.get::<_, Option<String>>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut total_pnl = Decimal::ZERO;
    for p in pnls.iter().flatten() {
        if let Ok(d) = Decimal::from_str(p) {
            total_pnl += d;
        }
    }
    let equity = initial + total_pnl;

    // 3) HWM: max(prev HWM, equity, initial)
    let prev = repo::latest_snapshot(&conn).map_err(|e| e.to_string())?;
    let hwm = match &prev {
        Some(p) => {
            let prev_hwm = parse_dec(&p.high_watermark, "prev_hwm")?;
            prev_hwm.max(equity).max(initial)
        }
        None => equity.max(initial),
    };

    // 4) LWM: min(prev LWM, equity). If first snapshot, = min(equity, initial)
    let lwm = match &prev {
        Some(p) => {
            let prev_lwm = parse_dec(&p.low_watermark, "prev_lwm")?;
            prev_lwm.min(equity)
        }
        None => equity.min(initial),
    };

    // 5) consecutive_losses: walk closed trades newest-first,
    //    count leading negatives in pnl until first non-negative.
    let mut stmt2 = conn
        .prepare(
            "SELECT manual_pnl FROM trades
             WHERE account_id = ?1 AND status = 'closed'
             ORDER BY COALESCE(closed_at, updated_at) DESC",
        )
        .map_err(|e| e.to_string())?;
    let ordered = stmt2
        .query_map([account_id], |r| r.get::<_, Option<String>>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut streak: i64 = 0;
    for p in ordered.iter().flatten() {
        match Decimal::from_str(p) {
            Ok(d) if d < Decimal::ZERO => streak += 1,
            Ok(_) => break,
            Err(_) => break,
        }
    }

    // 6) daily_loss: sum of negative manual_pnl on closed trades
    //    whose closed_at date (UTC) == today.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let mut stmt3 = conn
        .prepare(
            "SELECT manual_pnl FROM trades
             WHERE account_id = ?1
               AND status = 'closed'
               AND substr(COALESCE(closed_at, updated_at), 1, 10) = ?2",
        )
        .map_err(|e| e.to_string())?;
    let today_pnls = stmt3
        .query_map(rusqlite::params![account_id, today], |r| {
            r.get::<_, Option<String>>(0)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut daily_loss = Decimal::ZERO;
    for p in today_pnls.iter().flatten() {
        if let Ok(d) = Decimal::from_str(p) {
            if d < Decimal::ZERO {
                daily_loss += d.abs();
            }
        }
    }

    // 7) open_portfolio_risk: sum over open trades of
    //    |entry - initial_stop| * total_quantity
    let mut stmt4 = conn
        .prepare(
            "SELECT entry_price, initial_stop, total_quantity FROM trades
             WHERE account_id = ?1 AND status = 'open'",
        )
        .map_err(|e| e.to_string())?;
    let opens = stmt4
        .query_map([account_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut open_risk = Decimal::ZERO;
    for (ep, sl, qty) in opens {
        let e = parse_dec(&ep, "entry")?;
        let s = parse_dec(&sl, "stop")?;
        let q = parse_dec(&qty, "qty")?;
        let per_unit = (e - s).abs();
        open_risk += per_unit * q;
    }

    // 8) day_start_equity: if we have a snapshot from a previous UTC day
    //    use its equity as the day-start; otherwise use current equity.
    let day_start = match &prev {
        Some(p) => {
            let prev_taken = p.taken_at.get(..10).unwrap_or("");
            if prev_taken == today {
                // same day: reuse previous day_start
                parse_dec(&p.day_start_equity, "prev_day_start")?
            } else {
                // new day: today's start = current equity
                equity
            }
        }
        None => equity,
    };

    // 9) insert snapshot
    let taken_at = chrono::Utc::now().to_rfc3339();
        // 9b) Compute AccountState for phase/protected/floor/LC
    let cfg = load_or_default_config(&state)?;
    let snap_domain = EquitySnapshot {
        id: 0,
        equity: equity.to_string(),
        initial_balance: initial.to_string(),
        high_watermark: hwm.to_string(),
        low_watermark: lwm.to_string(),
        day_start_equity: day_start.to_string(),
        open_portfolio_risk: open_risk.to_string(),
        daily_loss: daily_loss.to_string(),
        consecutive_losses: streak,
        taken_at: taken_at.clone(),
    };
    let parsed = snap_domain.parse().map_err(|e: String| format!("snapshot parse: {e}"))?;
    let acct_state = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("AccountState: {e}"))?;

    let phase_s = phase_str(acct_state.phase).to_string();
    let prot_s = acct_state.profit_retention.to_string();
    let floor_s = acct_state.f_hybrid.to_string();
    let lc_s = acct_state.loss_capacity.to_string();

    let row = repo::SnapshotRow {
        equity: &equity.to_string(),
        initial_balance: &initial.to_string(),
        high_watermark: &hwm.to_string(),
        low_watermark: &lwm.to_string(),
        day_start_equity: &day_start.to_string(),
        open_portfolio_risk: &open_risk.to_string(),
        daily_loss: &daily_loss.to_string(),
        consecutive_losses: streak,
        taken_at: &taken_at,
        phase: Some(&phase_s),
        protected_profit: Some(&prot_s),
        equity_floor: Some(&floor_s),
        loss_capacity: Some(&lc_s),
    };
    let id = repo::insert_snapshot(&conn, &row).map_err(|e| e.to_string())?;
    Ok(id)
}

// ============================================================
// PRE-EVALUATE — check a proposed trade without saving it
// ============================================================

#[derive(Debug, Serialize)]
pub struct PreEvalResult {
    /// "ok" | "warn" | "block" | "reject"
    pub verdict: String,
    pub allowed_size: Option<String>,
    pub user_size: String,
    /// user_size / allowed_size as decimal-string (empty if allowed is 0)
    pub ratio: String,
    pub reason: String,
    pub allowed: bool,
}

#[tauri::command]
pub fn equity_pre_evaluate(
    state: State<'_, AppState>,
    entry: String,
    stop: String,
    take_profit: Option<String>,
    contract_multiplier: String,
    size: String,
) -> Result<PreEvalResult, String> {
    // 1) parse the proposed size (return ok if it's empty/zero — nothing to compare)
    let user_size = parse_dec(&size, "size")?;

    // 2) run the normal evaluator with the same proposal, WITHOUT a size
    //    (we only want to know allowed risk; size is computed by us)
    let proposal = ProposalInput {
        entry,
        stop,
        take_profit,
        contract_multiplier,
        commission_per_unit: "0".into(),
        slippage_buffer: "0".into(),
    };

    let decision = match equity_evaluate(state, Some(proposal)) {
        Ok(d) => d,
        Err(e) => {
            return Ok(PreEvalResult {
                verdict: "reject".into(),
                allowed_size: None,
                user_size: user_size.to_string(),
                ratio: String::new(),
                reason: e,
                allowed: false,
            });
        }
    };

    // 3) if decision was rejected — return reject verdict
    if !decision.allowed || decision.size.is_none() {
        return Ok(PreEvalResult {
            verdict: "reject".into(),
            allowed_size: decision.size.clone(),
            user_size: user_size.to_string(),
            ratio: String::new(),
            reason: format!("{:?}", decision.reason),
            allowed: false,
        });
    }

    let allowed_size_str = decision.size.clone().unwrap();
    let allowed_size = parse_dec(&allowed_size_str, "allowed_size")?;

    // 4) compare
    if allowed_size <= Decimal::ZERO {
        return Ok(PreEvalResult {
            verdict: "reject".into(),
            allowed_size: Some(allowed_size_str),
            user_size: user_size.to_string(),
            ratio: String::new(),
            reason: "Allowed size is zero — nothing to compare.".into(),
            allowed: false,
        });
    }

    let ratio = if user_size > Decimal::ZERO {
        user_size / allowed_size
    } else {
        Decimal::ZERO
    };

    let verdict: &str = if ratio <= Decimal::ONE {
        "ok"
    } else if ratio <= Decimal::from(3) {
        "warn"
    } else {
        "block"
    };

    let reason = match verdict {
        "ok" => format!(
            "Size {} lots is within the allowed {} lots.",
            user_size, allowed_size_str
        ),
        "warn" => format!(
            "⚠ Size {} lots is {:.1}× the allowed {} lots.",
            user_size,
            ratio.to_f64().unwrap_or(0.0),
            allowed_size_str
        ),
        "block" => format!(
            "🚫 Size {} lots is {:.1}× the allowed {} lots — heavily above limit.",
            user_size,
            ratio.to_f64().unwrap_or(0.0),
            allowed_size_str
        ),
        _ => unreachable!(),
    };

    Ok(PreEvalResult {
        verdict: verdict.into(),
        allowed_size: Some(allowed_size_str),
        user_size: user_size.to_string(),
        ratio: ratio.to_string(),
        reason,
        allowed: decision.allowed,
    })
}

// ============================================================
// MANAGEMENT COMMANDS — accounts, snapshots, cleanup
// ============================================================

#[derive(Debug, Serialize)]
pub struct AccountInfo {
    pub id: i64,
    pub name: String,
    pub currency: String,
    pub initial_balance: String,
    pub is_active: bool,
    pub kind: String,
}

#[tauri::command]
pub fn equity_list_accounts(
    state: State<'_, AppState>,
) -> Result<Vec<AccountInfo>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, currency, initial_balance, is_active, kind
             FROM accounts ORDER BY is_active DESC, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(AccountInfo {
                id: r.get(0)?,
                name: r.get(1)?,
                currency: r.get(2)?,
                initial_balance: r.get(3)?,
                is_active: r.get::<_, i64>(4)? != 0,
                kind: r.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn equity_list_snapshots(
    state: State<'_, AppState>,
    limit: i64,
) -> Result<Vec<repo::EquitySnapshotRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, equity, initial_balance, high_watermark, low_watermark,
                    day_start_equity, open_portfolio_risk, daily_loss,
                    consecutive_losses, taken_at,
                    phase, protected_profit, equity_floor, loss_capacity
             FROM equity_snapshots
             ORDER BY taken_at DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([limit], |r| {
            Ok(repo::EquitySnapshotRow {
                id: r.get(0)?,
                equity: r.get(1)?,
                initial_balance: r.get(2)?,
                high_watermark: r.get(3)?,
                low_watermark: r.get(4)?,
                day_start_equity: r.get(5)?,
                open_portfolio_risk: r.get(6)?,
                daily_loss: r.get(7)?,
                consecutive_losses: r.get(8)?,
                taken_at: r.get(9)?,
                phase: r.get(10).ok(),
                protected_profit: r.get(11).ok(),
                equity_floor: r.get(12).ok(),
                loss_capacity: r.get(13).ok(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn equity_delete_snapshot(
    state: State<'_, AppState>,
    id: i64,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // 1) decisions that reference this snapshot
    conn.execute("DELETE FROM equity_decisions WHERE snapshot_id = ?1", [id])
        .map_err(|e| e.to_string())?;
    // 2) the snapshot itself
    conn.execute("DELETE FROM equity_snapshots WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn equity_clear_snapshots(state: State<'_, AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    // 1) all decisions
    conn.execute("DELETE FROM equity_decisions", [])
        .map_err(|e| e.to_string())?;
    // 2) all snapshots
    let n = conn
        .execute("DELETE FROM equity_snapshots", [])
        .map_err(|e| e.to_string())?;
    Ok(n as i64)
}

/// Delete snapshots older than N days. Returns how many were deleted.
#[tauri::command]
pub fn equity_cleanup_old_snapshots(
    state: State<'_, AppState>,
    days: i64,
) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();
    // 1) decisions whose snapshot is older than cutoff
    conn.execute(
        "DELETE FROM equity_decisions
         WHERE snapshot_id IN (
           SELECT id FROM equity_snapshots WHERE taken_at < ?1
         )",
        [&cutoff],
    )
    .map_err(|e| e.to_string())?;
    // 2) old snapshots
    let n = conn
        .execute(
            "DELETE FROM equity_snapshots WHERE taken_at < ?1",
            [&cutoff],
        )
        .map_err(|e| e.to_string())?;
    Ok(n as i64)
}

// ============================================================
// RISK PREVIEW — full risk intelligence without a proposal
// ============================================================

#[derive(Debug, Serialize)]
pub struct RiskPreview {
    // Account state
    pub equity: String,
    pub initial_balance: String,
    pub high_watermark: String,
    pub net_profit: String,
    pub dd_pct: String,
    pub phase: String,
    pub loss_capacity: String,
    pub profit_retention: String,
    pub f_profit: String,
    pub f_trail: String,
    pub f_prop: String,
    pub f_hybrid: String,

    // Risk breakdown
    pub r_base: String,
    pub m_dd: String,
    pub m_loss: String,
    pub r_adaptive: String,

    // Caps
    pub r_portfolio_cap: String,
    pub r_daily_cap: String,
    pub r_floor_cap: String,
    pub r_prop_cap: String,

    // Final
    pub r_allowed: String,
    pub binding_constraint: String, // "BaseRate" | "Portfolio" | "Daily" | "Floor" | "Prop"

    // Config (for display)
    pub r0_pct: String,
    pub dd_1_pct: String,
    pub dd_2_pct: String,
    pub dd_3_pct: String,
    pub m_dd_1: String,
    pub m_dd_2: String,
    pub m_dd_3: String,
    pub m_dd_4: String,
    pub m_loss_table: String,
    pub n_max: i64,
    pub alpha_pct: String,
    pub daily_limit: String,
    pub portfolio_max: String,
    pub prop_max: String,

    // Snapshot context
    pub open_portfolio_risk: String,
    pub daily_loss: String,
    pub consecutive_losses: i64,
}

#[tauri::command]
pub fn equity_preview_risk(
    state: State<'_, AppState>,
) -> Result<RiskPreview, String> {
    use crate::equity::risk_engine::compute_risk;

    let cfg = load_or_default_config(&state)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let snap_row = repo::latest_snapshot(&conn).map_err(|e| e.to_string())?
        .ok_or_else(|| "no snapshot yet — click Refresh".to_string())?;
    drop(conn);

    let snap_domain = EquitySnapshot {
        id: snap_row.id,
        equity: snap_row.equity.clone(),
        initial_balance: snap_row.initial_balance.clone(),
        high_watermark: snap_row.high_watermark.clone(),
        low_watermark: snap_row.low_watermark.clone(),
        day_start_equity: snap_row.day_start_equity.clone(),
        open_portfolio_risk: snap_row.open_portfolio_risk.clone(),
        daily_loss: snap_row.daily_loss.clone(),
        consecutive_losses: snap_row.consecutive_losses,
        taken_at: snap_row.taken_at.clone(),
    };
    let parsed = snap_domain.parse().map_err(|e| format!("snapshot: {e}"))?;
    let acct_state = AccountState::from_snapshot(parsed, &cfg)
        .map_err(|e| format!("state: {e}"))?;

    let risk = compute_risk(&acct_state, &cfg).map_err(|e| e.to_string())?;

    // Determine binding constraint
    let mut binding = "BaseRate";
    let mut best = risk.r_adaptive;
    if risk.r_portfolio_cap < best {
        best = risk.r_portfolio_cap;
        binding = "Portfolio";
    }
    if risk.r_daily_cap < best {
        best = risk.r_daily_cap;
        binding = "Daily";
    }
    if risk.r_floor_cap < best {
        best = risk.r_floor_cap;
        binding = "Floor";
    }
    if risk.r_prop_cap < best {
        best = risk.r_prop_cap;
        binding = "Prop";
    }

    Ok(RiskPreview {
        equity: acct_state.snapshot.equity.to_string(),
        initial_balance: acct_state.snapshot.initial_balance.to_string(),
        high_watermark: acct_state.snapshot.high_watermark.to_string(),
        net_profit: acct_state.net_profit.to_string(),
        dd_pct: acct_state.dd_pct.to_string(),
        phase: phase_str(acct_state.phase).to_string(),
        loss_capacity: acct_state.loss_capacity.to_string(),
        profit_retention: acct_state.profit_retention.to_string(),
        f_profit: acct_state.f_profit.to_string(),
        f_trail: acct_state.f_trail.to_string(),
        f_prop: acct_state.f_prop.to_string(),
        f_hybrid: acct_state.f_hybrid.to_string(),

        r_base: risk.r_base.to_string(),
        m_dd: risk.m_dd.to_string(),
        m_loss: risk.m_loss.to_string(),
        r_adaptive: risk.r_adaptive.to_string(),

        r_portfolio_cap: risk.r_portfolio_cap.to_string(),
        r_daily_cap: risk.r_daily_cap.to_string(),
        r_floor_cap: risk.r_floor_cap.to_string(),
        r_prop_cap: risk.r_prop_cap.to_string(),

        r_allowed: risk.r_allowed.to_string(),
        binding_constraint: binding.into(),

        r0_pct: cfg.r0.clone(),
        dd_1_pct: cfg.dd_1.clone(),
        dd_2_pct: cfg.dd_2.clone(),
        dd_3_pct: cfg.dd_3.clone(),
        m_dd_1: cfg.m_dd_1.clone(),
        m_dd_2: cfg.m_dd_2.clone(),
        m_dd_3: cfg.m_dd_3.clone(),
        m_dd_4: cfg.m_dd_4.clone(),
        m_loss_table: cfg.m_loss_table.clone(),
        n_max: cfg.n_max,
        alpha_pct: cfg.alpha.clone(),
        daily_limit: cfg.daily_loss_limit.clone(),
        portfolio_max: cfg.r_portfolio_max.clone(),
        prop_max: cfg.r_prop_max.clone(),

        open_portfolio_risk: acct_state.snapshot.open_portfolio_risk.to_string(),
        daily_loss: acct_state.snapshot.daily_loss.to_string(),
        consecutive_losses: acct_state.snapshot.consecutive_losses,
    })
}
