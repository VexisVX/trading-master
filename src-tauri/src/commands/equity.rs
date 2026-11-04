//! Tauri commands for the Equity Engine.
//! All numeric fields cross the boundary as strings (Decimal-safe).

use crate::commands::AppState;
use crate::db::equity as repo;
use crate::equity::{
    config::EquityConfig,
    engine::{evaluate, TradeProposal},
    error::{DecisionReason, EquityError},
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
    pub symbol: Option<String>,
    pub entry: String,
    pub stop: String,
    pub take_profit: Option<String>,
    pub allow_legacy: Option<bool>,
    pub contract_multiplier: Option<String>,
    pub commission_per_unit: Option<String>,
    pub slippage_buffer: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DecisionOut {
    pub allowed: bool,
    pub reason: DecisionReason,
    pub r_allowed: String,
    pub size: Option<String>,
    pub rr: Option<String>,
    // ---- legacy (S8; UI still reads these) ----
    pub phase: String,
    pub dd_pct: String,
    pub f_hybrid: String,
    pub loss_capacity: String,
    // ---- new (S8) ----
    pub state: String,
    pub state_enum: String,
    pub buffer_r: String,
    pub m_preserve: String,
    pub f_internal: String,
    pub f_prop: String,
    pub snapshot_id: Option<i64>,
    pub risk_detail: Option<RiskDetail>,
    pub sizing_source: String,
    pub actual_risk: Option<String>,
    pub instrument_symbol: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RiskDetail {
    pub equity: String,
    // legacy (S8)
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
    // new (S8)
    pub buffer_r: String,
    pub m_preserve: String,
    pub f_internal: String,
    pub f_prop: String,
    pub binding_constraint: String,
    pub r_target: String,
    pub r_buffer: String,
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
pub fn equity_config_set(state: State<'_, AppState>, config: EquityConfig) -> Result<(), String> {
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
    let prev = repo::latest_snapshot(&conn, None).map_err(|e| e.to_string())?;
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
    let parsed = snap_domain
        .parse()
        .map_err(|e: String| format!("snapshot parse: {e}"))?;
    let acct_state = AccountState::from_snapshot_with_profile(parsed, &cfg, None)
        .map_err(|e| format!("AccountState: {e}"))?;

    let phase_s = phase_str(acct_state.phase).to_string();
    let prot_s = acct_state.profit_retention.to_string();
    let floor_s = acct_state.f_hybrid.to_string();
    let lc_s = acct_state.loss_capacity.to_string();

    let row = repo::SnapshotRow {
        account_id: None,
        equity: &equity.to_string(),
        initial_balance: &initial.to_string(),
        high_watermark: &hwm,
        low_watermark: &low.to_string(),
        day_start_equity: &day_start.to_string(),
        open_portfolio_risk: &input.open_portfolio_risk,
        daily_loss: &input.daily_loss,
        consecutive_losses: input.consecutive_losses,
        taken_at: &taken_at,
        state: None,
        buffer_r: None,
        m_preserve: None,
        f_internal: None,
        f_prop_val: None,
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
    account_id: Option<i64>,
) -> Result<Option<repo::EquitySnapshotRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    repo::latest_snapshot(&conn, account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn equity_evaluate(
    state: State<'_, AppState>,
    account_id: Option<i64>,
    proposal: Option<ProposalInput>,
) -> Result<DecisionOut, String> {
    // 1. Load config + latest snapshot
    let cfg = load_or_default_config(&state)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let snap_row = repo::latest_snapshot(&conn, account_id)
        .map_err(|e| e.to_string())?
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
    let profile_opt = {
        let aid = account_id.unwrap_or_else(|| {
            // fallback: use active account
            0
        });
        if aid > 0 {
            crate::db::profiles::get(&conn, aid).map_err(|e| e.to_string())?
        } else {
            None
        }
    };
    // PHASE 5: load recovery authorization
    let recovery_active = if let Some(aid) = account_id {
        let conn2 = state.db.lock().map_err(|e| e.to_string())?;
        crate::db::recovery::get_active(&conn2, aid)
            .ok()
            .flatten()
            .is_some()
    } else {
        false
    };

    let state_acct = AccountState::from_snapshot_with_recovery(
        parsed, &cfg, profile_opt.as_ref(), recovery_active
    ).map_err(|e| format!("state: {e}"))?;

    // PHASE 2.2-C: reject if daily data is incomplete/stale
    // This is a data-quality block, NOT a PreservationPause.
    if let Err(e) = check_daily_data_quality(&conn, account_id, &state_acct, &cfg) {
        let reason = crate::equity::error::DecisionReason::MarketDataIncomplete {
            reason: e.to_string(),
        };
        return Ok(DecisionOut {
            allowed: false,
            reason,
            r_allowed: "".into(),
            size: None,
            rr: None,
            phase: phase_str(state_acct.phase).to_string(),
            dd_pct: state_acct.dd_pct.to_string(),
            f_hybrid: state_acct.f_hybrid.to_string(),
            loss_capacity: state_acct.loss_capacity.to_string(),
            state: state_acct.state_enum.as_str().to_string(),
            state_enum: state_acct.state_enum.as_str().to_string(),
            buffer_r: state_acct.buffer_r.to_string(),
            m_preserve: state_acct.m_preserve.to_string(),
            f_internal: state_acct.f_internal.to_string(),
            f_prop: state_acct.f_prop.to_string(),
            snapshot_id: Some(snap_row.id),
            risk_detail: None,
            sizing_source: "none".into(),
            actual_risk: None,
            instrument_symbol: None,
        });
    }

    // 2. Build proposal (clone original for later risk_detail)
    let proposal_for_detail = proposal.clone();
    // 2. Build proposal (S2.1: symbol optional, legacy fallback)
    let prop = match proposal {
        Some(p) => Some(TradeProposal {
            symbol: p.symbol,
            entry: p.entry,
            stop: p.stop,
            take_profit: p.take_profit,
            allow_legacy: p.allow_legacy.unwrap_or(false),
            contract_multiplier: p.contract_multiplier,
            commission_per_unit: p.commission_per_unit,
            slippage_buffer: p.slippage_buffer,
            instrument_override: None,
        }),
        None => None,
    };

    // 3. Evaluate
    let decision: TradeDecision =
        crate::equity::engine::evaluate_with_conn(&state_acct, &cfg, prop.as_ref(), Some(&conn))
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
        Some(r) => r.r_target > r.r_allowed,
        None => false,
    };
    let phase_for_db = phase_str(state_acct.phase).to_string();

    // PHASE 4: instrument audit
    let (inst_sym_audit, inst_spec_audit) = match &decision.instrument_symbol {
        Some(sym) => {
            let spec_json = {
                let conn2 = state.db.lock().map_err(|e| e.to_string())?;
                crate::db::instruments::get_typed(&conn2, sym)
                    .ok()
                    .flatten()
                    .and_then(|s| serde_json::to_string(&s).ok())
            };
            (Some(sym.as_str()), spec_json)
        }
        None => (None, None),
    };

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
        instrument_symbol: inst_sym_audit,
        instrument_spec_json: inst_spec_audit.as_deref(),
    };
    let _ = repo::insert_decision(&conn, &drow);

    // 5. Build output

    let state_s = state_acct.state_enum.as_str().to_string();
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
        state: state_s.clone(),
        state_enum: state_s,
        buffer_r: state_acct.buffer_r.to_string(),
        m_preserve: state_acct.m_preserve.to_string(),
        f_internal: state_acct.f_internal.to_string(),
        f_prop: state_acct.f_prop.to_string(),
        snapshot_id: Some(snap_row.id),
        risk_detail: decision.risk_detail.as_ref().map(|r| {
            // Compute entry_minus_stop and per_unit_cost from proposal
            let (ems, mult, per_unit) = match &proposal_for_detail {
                Some(p) => {
                    let entry = parse_dec(&p.entry, "entry").unwrap_or(Decimal::ZERO);
                    let stop = parse_dec(&p.stop, "stop").unwrap_or(Decimal::ZERO);
                    let mult = parse_dec(p.contract_multiplier.as_deref().unwrap_or("1"), "mult")
                        .unwrap_or(Decimal::ONE);
                    let comm = parse_dec(p.commission_per_unit.as_deref().unwrap_or("0"), "comm")
                        .unwrap_or(Decimal::ZERO);
                    let slip = parse_dec(p.slippage_buffer.as_deref().unwrap_or("0"), "slip")
                        .unwrap_or(Decimal::ZERO);
                    let ems = (entry - stop).abs();
                    let per_unit = ems * mult + comm + slip;
                    (ems, mult, per_unit)
                }
                None => (Decimal::ZERO, Decimal::ONE, Decimal::ZERO),
            };
            RiskDetail {
                equity: state_acct.snapshot.equity.to_string(),
                r_base: r.r_base.to_string(),
                m_dd: "1".into(),
                m_loss: "1".into(),
                r_adaptive: r.r_target.to_string(),
                r_portfolio_cap: r.r_portfolio_cap.to_string(),
                r_daily_cap: r.r_daily_cap.to_string(),
                r_floor_cap: r.r_buffer.to_string(),
                r_prop_cap: r.r_prop_cap.to_string(),
                r_allowed: r.r_allowed.to_string(),
                entry_minus_stop: ems.to_string(),
                contract_multiplier: mult.to_string(),
                per_unit_cost: per_unit.to_string(),
                buffer_r: state_acct.buffer_r.to_string(),
                m_preserve: state_acct.m_preserve.to_string(),
                f_internal: state_acct.f_internal.to_string(),
                f_prop: state_acct.f_prop.to_string(),
                binding_constraint: r.binding.to_string(),
                r_target: r.r_target.to_string(),
                r_buffer: r.r_buffer.to_string(),
            }
        }),
        sizing_source: decision.sizing_source.clone(),
        actual_risk: decision.actual_risk.clone(),
        instrument_symbol: decision.instrument_symbol.clone(),
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
        instrument_symbol: None,
        instrument_spec_json: None,
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
    // Guard: skip if last snapshot for this account is < 2 seconds old
    if let Some(aid) = account_id {
        if let Ok(conn0) = state.db.try_lock() {
            let last_taken: Option<String> = conn0
                .query_row(
                    "SELECT taken_at FROM equity_snapshots WHERE account_id = ?1 ORDER BY id DESC LIMIT 1",
                    [aid],
                    |r| r.get(0),
                )
                .ok();
            if let Some(ts) = last_taken {
                if let Ok(when) = chrono::DateTime::parse_from_rfc3339(&ts) {
                    let age_secs = chrono::Utc::now()
                        .signed_duration_since(when.with_timezone(&chrono::Utc))
                        .num_seconds();
                    if age_secs < 2 {
                        eprintln!("[AUTO_SNAPSHOT] skipped: last snapshot {}s ago", age_secs);
                        return Ok(0);
                    }
                }
            }
        }
    }

    use rust_decimal::prelude::FromStr;
    use rust_decimal::Decimal;

    eprintln!("[AUTO_SNAPSHOT] START account_id={:?}", account_id);

    // ★ CRITICAL: Load config BEFORE acquiring the db lock.
    // load_or_default_config itself takes state.db.lock() — if we hold
    // the lock and then call it, we deadlock (and panic).
    let cfg = load_or_default_config(&state)
        .unwrap_or_else(|_| crate::equity::config::EquityConfig::default_v1());
    eprintln!("[AUTO_SNAPSHOT] config preloaded");

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    eprintln!("[AUTO_SNAPSHOT] db lock acquired");

    // ============================================================
    // 1. Get account
    // ============================================================
    let (acc_id, initial): (i64, Decimal) = match account_id {
        Some(aid) => conn
            .query_row(
                "SELECT id, initial_balance FROM accounts WHERE id = ?1",
                [aid],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|e| format!("account {} not found: {}", aid, e))
            .and_then(|(id, ib)| {
                Decimal::from_str(&ib)
                    .map(|d| (id, d))
                    .map_err(|e| format!("initial_balance: {}", e))
            })?,
        None => conn
            .query_row(
                "SELECT id, initial_balance FROM accounts WHERE is_active = 1 ORDER BY id LIMIT 1",
                [],
                |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|e| format!("no active account: {}", e))
            .and_then(|(id, ib)| {
                Decimal::from_str(&ib)
                    .map(|d| (id, d))
                    .map_err(|e| format!("initial_balance: {}", e))
            })?,
    };
    eprintln!("[AUTO_SNAPSHOT] account_id={} initial={}", acc_id, initial);

    // ============================================================
    // 2. Equity from closed trades
    // ============================================================
    let mut total_pnl = Decimal::ZERO;
    {
        let mut stmt = conn
            .prepare("SELECT manual_pnl FROM trades WHERE account_id = ?1 AND status = 'closed'")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([acc_id], |r| r.get::<_, Option<String>>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok(Some(s)) = row {
                if let Ok(d) = Decimal::from_str(&s) {
                    total_pnl += d;
                }
            }
        }
    }
    let equity = initial + total_pnl;
    eprintln!("[AUTO_SNAPSHOT] equity = {}", equity);

    // ============================================================
    // 3. HWM / LWM
    // ============================================================
    let prev = repo::latest_snapshot(&conn, Some(acc_id)).map_err(|e| e.to_string())?;
    let (hwm, lwm) = match &prev {
        Some(p) => {
            let prev_hwm = Decimal::from_str(&p.high_watermark).unwrap_or(initial);
            let prev_lwm = Decimal::from_str(&p.low_watermark).unwrap_or(initial);
            (
                prev_hwm.max(equity).max(initial),
                prev_lwm.min(equity).min(initial),
            )
        }
        None => (equity.max(initial), equity.min(initial)),
    };
    eprintln!("[AUTO_SNAPSHOT] hwm={} lwm={}", hwm, lwm);

    // ============================================================
    // 4. Streak
    // ============================================================
    let mut streak: i64 = 0;
    {
        let mut stmt = conn
            .prepare(
                "SELECT manual_pnl FROM trades WHERE account_id = ?1 AND status = 'closed'
                 ORDER BY COALESCE(closed_at, updated_at) DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([acc_id], |r| r.get::<_, Option<String>>(0))
            .map_err(|e| e.to_string())?;
        for row in rows {
            match row {
                Ok(Some(s)) => match Decimal::from_str(&s) {
                    Ok(d) if d < Decimal::ZERO => streak += 1,
                    _ => break,
                },
                _ => break,
            }
        }
    }
    eprintln!("[AUTO_SNAPSHOT] streak = {}", streak);

    // Day-start equity: if there's a previous snapshot from earlier today
    // (in the reset boundary), use its day_start_equity; else use current equity.
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let day_start: Decimal = match &prev {
        Some(p) => {
            let prev_day = p.taken_at.chars().take(10).collect::<String>();
            if prev_day == today {
                Decimal::from_str(&p.day_start_equity).unwrap_or(equity)
            } else {
                equity
            }
        }
        None => equity,
    };

    // ============================================================
    // 5. Daily loss (PHASE 2.2-C: full daily::compute)
    // ============================================================
    // Load all trades for this account
    let all_trades: Vec<crate::domain::trade::Trade> = {
        let mut stmt = conn
            .prepare("SELECT id FROM trades WHERE account_id = ?1")
            .map_err(|e| e.to_string())?;
        let ids: Vec<i64> = stmt
            .query_map([acc_id], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let mut out = Vec::with_capacity(ids.len());
        for id in ids {
            if let Ok(Some(t)) = crate::db::trades::get_trade(&conn, id) {
                out.push(t);
            }
        }
        out
    };

    // Load instruments
    let mut instruments: std::collections::HashMap<
        String,
        crate::domain::instrument::InstrumentSpec,
    > = std::collections::HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT symbol FROM instruments")
            .map_err(|e| e.to_string())?;
        let syms: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        for sym in syms {
            if let Ok(Some(spec)) = crate::db::instruments::get_typed(&conn, &sym) {
                instruments.insert(sym, spec);
            }
        }
    }

    // Reset boundary from config (PHASE 2.2-B)
    let reset_b = crate::equity::reset::from_config(&cfg);
    let now = chrono::Utc::now();
    let day_start_time = crate::equity::reset::last_boundary(&reset_b, now);

    // External cash flows since reset
    let external_cash_flows = {
        let mut stmt = conn
            .prepare(
                "SELECT amount, kind FROM account_cash_flows
                 WHERE account_id = ?1 AND occurred_at >= ?2",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(
                rusqlite::params![acc_id, day_start_time.to_rfc3339()],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .map_err(|e| e.to_string())?;
        let mut net = Decimal::ZERO;
        for row in rows {
            if let Ok((amt_s, kind)) = row {
                if let Ok(amt) = Decimal::from_str(&amt_s) {
                    match kind.as_str() {
                        "deposit" | "funding" => net += amt,
                        "withdrawal" | "fee" => net -= amt,
                        _ => {}
                    }
                }
            }
        }
        net
    };

    let daily_result = crate::equity::daily::compute(&crate::equity::daily::DailyPnLInput {
        trades: &all_trades,
        day_start_equity: day_start, // from earlier in the function
        current_equity: equity,
        external_cash_flows,
        day_start_time,
        instruments: &instruments,
        max_price_age_minutes: cfg.max_price_age_minutes,
        now,
        daily_loss_limit: Decimal::from_str(&cfg.daily_loss_limit).unwrap_or(Decimal::ZERO)
            * equity,
        execution_buffer: Decimal::ZERO,
    });

    let (daily_loss, daily_error) = match daily_result {
        Ok(d) => (d.daily_loss_used, None::<String>),
        Err(e) => {
            eprintln!("[AUTO_SNAPSHOT] daily::compute error: {}", e);
            (Decimal::ZERO, Some(e.to_string()))
        }
    };
    eprintln!(
        "[AUTO_SNAPSHOT] daily_loss = {} (error: {:?})",
        daily_loss, daily_error
    );

    // ============================================================
    // 6. Open risk
    // ============================================================
    let mut open_risk = Decimal::ZERO;
    {
        let mut stmt = conn
            .prepare(
                "SELECT entry_price, initial_stop, total_quantity FROM trades
                 WHERE account_id = ?1 AND status = 'open'",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([acc_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            if let Ok((ep, sl, qty)) = row {
                let e = Decimal::from_str(&ep).unwrap_or(Decimal::ZERO);
                let s = Decimal::from_str(&sl).unwrap_or(Decimal::ZERO);
                let q = Decimal::from_str(&qty).unwrap_or(Decimal::ZERO);
                open_risk += (e - s).abs() * q;
            }
        }
    }
    eprintln!("[AUTO_SNAPSHOT] open_risk = {}", open_risk);

    // ============================================================
    // 7. Day start
    // ============================================================
    let day_start = match &prev {
        Some(p) => {
            let prev_day = p.taken_at.chars().take(10).collect::<String>();
            if prev_day == today {
                Decimal::from_str(&p.day_start_equity).unwrap_or(equity)
            } else {
                equity
            }
        }
        None => equity,
    };
    eprintln!("[AUTO_SNAPSHOT] day_start = {}", day_start);

    // ============================================================
    // 8. Compute floor / protected / LC / phase — manual
    // ============================================================
    // S10: Use AccountState::from_snapshot instead of manual formulas.
    let parsed_for_state = crate::equity::state::ParsedSnapshot {
        id: 0,
        equity,
        initial_balance: initial,
        high_watermark: hwm,
        low_watermark: lwm,
        day_start_equity: day_start,
        open_portfolio_risk: open_risk,
        daily_loss,
        consecutive_losses: streak,
        taken_at: chrono::Utc::now(),
    };
    // S16-deadlock-fix: use the already-held conn (do NOT re-lock).
    let profile_opt = crate::db::profiles::get(&conn, acc_id).map_err(|e| e.to_string())?;
    let acct_state = crate::equity::state::AccountState::from_snapshot_with_profile(
        parsed_for_state,
        &cfg,
        profile_opt.as_ref(),
    )
    .map_err(|e| format!("AccountState: {}", e))?;

    let protected_profit = acct_state.profit_retention;
    let f_hybrid = acct_state.f_hybrid;
    let loss_capacity = acct_state.loss_capacity;
    let f_internal = acct_state.f_internal;
    let f_prop_val = acct_state.f_prop;
    let buffer_r = acct_state.buffer_r;
    let m_preserve = acct_state.m_preserve;
    let state_enum_s = acct_state.state_enum.as_str().to_string();
    let phase_str_manual = phase_str(acct_state.phase);
    let dd_pct = acct_state.dd_pct;

    eprintln!(
        "[AUTO_SNAPSHOT] protected={} floor={} lc={} dd={} phase={} state={} buffer_r={} m_preserve={}",
        protected_profit, f_hybrid, loss_capacity, dd_pct, phase_str_manual, state_enum_s, buffer_r, m_preserve
    );

    let taken_at = chrono::Utc::now().to_rfc3339();
    let row = repo::SnapshotRow {
        account_id: Some(acc_id),
        equity: &equity.to_string(),
        initial_balance: &initial.to_string(),
        high_watermark: &hwm.to_string(),
        low_watermark: &lwm.to_string(),
        day_start_equity: &day_start.to_string(),
        open_portfolio_risk: &open_risk.to_string(),
        daily_loss: &daily_loss.to_string(),
        consecutive_losses: streak,
        taken_at: &taken_at,
        phase: Some(phase_str_manual),
        protected_profit: Some(&protected_profit.to_string()),
        equity_floor: Some(&f_hybrid.to_string()),
        loss_capacity: Some(&loss_capacity.to_string()),
        state: Some(&state_enum_s),
        buffer_r: Some(&buffer_r.to_string()),
        m_preserve: Some(&m_preserve.to_string()),
        f_internal: Some(&f_internal.to_string()),
        f_prop_val: Some(&f_prop_val.to_string()),
    };

    let id = repo::insert_snapshot(&conn, &row).map_err(|e| e.to_string())?;
    eprintln!("[AUTO_SNAPSHOT] inserted id={}", id);
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
    account_id: Option<i64>,
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
        symbol: None,
        entry,
        stop,
        take_profit,
        allow_legacy: Some(true),
        contract_multiplier: Some("100000".into()),
        commission_per_unit: Some("0".into()),
        slippage_buffer: Some("0".into()),
    };

    let decision = match equity_evaluate(state, account_id, Some(proposal)) {
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
pub fn equity_list_accounts(state: State<'_, AppState>) -> Result<Vec<AccountInfo>, String> {
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
    account_id: Option<i64>,
    limit: i64,
) -> Result<Vec<repo::EquitySnapshotRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let _ = repo::ensure_snapshot_detail_columns(&conn);
    let _ = repo::ensure_snapshot_account_id(&conn);

    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<repo::EquitySnapshotRow> {
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
            state: None,
            buffer_r: None,
            m_preserve: None,
            f_internal: None,
            f_prop_val: None,
        })
    };

    let mut rows: Vec<repo::EquitySnapshotRow> = Vec::new();

    if let Some(aid) = account_id {
        let mut stmt = conn
            .prepare(
                "SELECT id, equity, initial_balance, high_watermark, low_watermark,
                        day_start_equity, open_portfolio_risk, daily_loss,
                        consecutive_losses, taken_at,
                        phase, protected_profit, equity_floor, loss_capacity
                 FROM equity_snapshots
                 WHERE account_id = ?1
                 ORDER BY taken_at DESC
                 LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        let mut query = stmt
            .query(rusqlite::params![aid, limit])
            .map_err(|e| e.to_string())?;
        while let Some(r) = query.next().map_err(|e| e.to_string())? {
            rows.push(map_row(r).map_err(|e| e.to_string())?);
        }
    } else {
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
        let mut query = stmt
            .query(rusqlite::params![limit])
            .map_err(|e| e.to_string())?;
        while let Some(r) = query.next().map_err(|e| e.to_string())? {
            rows.push(map_row(r).map_err(|e| e.to_string())?);
        }
    }

    Ok(rows)
}

#[tauri::command]
pub fn equity_delete_snapshot(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    // PRAGMA must be OUTSIDE transaction
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| format!("PRAGMA OFF failed: {e}"))?;

    let result = (|| -> Result<(), String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        // 1. Delete decisions first
        tx.execute("DELETE FROM equity_decisions WHERE snapshot_id = ?1", [id])
            .map_err(|e| format!("delete decisions: {e}"))?;
        // 2. Then snapshot
        let n = tx
            .execute("DELETE FROM equity_snapshots WHERE id = ?1", [id])
            .map_err(|e| format!("delete snapshot: {e}"))?;
        if n == 0 {
            return Err(format!("snapshot {} not found", id));
        }
        tx.commit().map_err(|e| format!("commit: {e}"))?;
        Ok(())
    })();

    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");

    result
}

#[tauri::command]
pub fn equity_clear_snapshots(state: State<'_, AppState>) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| format!("PRAGMA OFF failed: {e}"))?;

    let result = (|| -> Result<i64, String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM equity_decisions", [])
            .map_err(|e| format!("delete decisions: {e}"))?;
        let n = tx
            .execute("DELETE FROM equity_snapshots", [])
            .map_err(|e| format!("delete snapshots: {e}"))?;
        tx.commit().map_err(|e| format!("commit: {e}"))?;
        Ok(n as i64)
    })();

    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");

    result
}

/// Delete snapshots older than N days. Returns how many were deleted.
#[tauri::command]
pub fn equity_cleanup_old_snapshots(state: State<'_, AppState>, days: i64) -> Result<i64, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let cutoff = (chrono::Utc::now() - chrono::Duration::days(days)).to_rfc3339();

    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .map_err(|e| format!("PRAGMA OFF failed: {e}"))?;

    let result = (|| -> Result<i64, String> {
        let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM equity_decisions
             WHERE snapshot_id IN (
               SELECT id FROM equity_snapshots WHERE taken_at < ?1
             )",
            [&cutoff],
        )
        .map_err(|e| format!("delete decisions: {e}"))?;
        let n = tx
            .execute(
                "DELETE FROM equity_snapshots WHERE taken_at < ?1",
                [&cutoff],
            )
            .map_err(|e| format!("delete snapshots: {e}"))?;
        tx.commit().map_err(|e| format!("commit: {e}"))?;
        Ok(n as i64)
    })();

    let _ = conn.execute_batch("PRAGMA foreign_keys = ON;");

    result
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
    pub state: String,
    pub state_enum: String,

    // Floors
    pub f_profit: String, // "" if not active
    pub f_trail: String,
    pub f_internal: String,
    pub f_prop: String,
    pub profit_retention: String,
    pub loss_capacity: String,

    // Buffer
    pub buffer: String,
    pub buffer_r: String,
    pub m_preserve: String,
    pub execution_buffer: String,

    // Risk
    pub r_base: String,
    pub r_target: String,
    pub r_buffer: String,
    pub r_portfolio_cap: String,
    pub r_daily_cap: String,
    pub r_prop_cap: String,
    pub r_allowed: String,
    pub binding_constraint: String,

    // Config (for display)
    pub r0_pct: String,
    pub profit_activation: String,
    pub profit_full: String,
    pub max_retention: String,
    pub trailing_dd: String,
    pub alpha_pct: String,
    pub daily_limit: String,
    pub portfolio_max: String,
    pub prop_max: String,

    // Snapshot context
    pub open_portfolio_risk: String,
    pub daily_loss: String,
    pub consecutive_losses: i64,
}

// ============================================================
// Profile DTOs (PHASE 2.2, restored)
// ============================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProfileInputDto {
    pub account_type: String,
    pub profit_target_pct: String,
    pub max_dd_pct: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ProfileOut {
    pub account_id: i64,
    pub account_type: String,
    pub profit_target_pct: String,
    pub max_dd_pct: String,
    pub profit_protect_activation: String,
    pub profit_protect_full: String,
    pub max_profit_retention: String,
    pub trailing_dd_pct: String,
}

#[tauri::command]
pub fn equity_preview_risk(
    state: State<'_, AppState>,
    account_id: Option<i64>,
) -> Result<RiskPreview, String> {
    use crate::equity::buffer::{self, BufferInput};
    use crate::equity::risk_engine::{self, RiskInput};
    use rust_decimal::prelude::FromStr;

    let cfg = load_or_default_config(&state)?;
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let snap_row = repo::latest_snapshot(&conn, account_id)
        .map_err(|e| e.to_string())?
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
    let profile_opt = match account_id {
        Some(aid) => {
            let conn2 = state.db.lock().map_err(|e| e.to_string())?;
            let p = crate::db::profiles::get(&conn2, aid).map_err(|e| e.to_string())?;
            p
        }
        None => None,
    };
    let acct_state = AccountState::from_snapshot_with_profile(parsed, &cfg, profile_opt.as_ref())
        .map_err(|e| format!("state: {e}"))?;

    let r0 = Decimal::from_str(&cfg.r0).unwrap_or(Decimal::new(1, 2));
    let r_base = acct_state.snapshot.equity * r0;

    let buf = buffer::compute(&BufferInput {
        equity: acct_state.snapshot.equity,
        f_internal: acct_state.f_internal,
        r_open: acct_state.snapshot.open_portfolio_risk,
        execution_buffer: Decimal::ZERO,
        r_base,
    });

    let portfolio_max = Decimal::from_str(&cfg.r_portfolio_max).unwrap_or(Decimal::ZERO);
    let daily_limit = Decimal::from_str(&cfg.daily_loss_limit).unwrap_or(Decimal::ZERO);
    let prop_max = Decimal::from_str(&cfg.r_prop_max).unwrap_or(Decimal::ZERO);

    let r_portfolio_remaining = if portfolio_max.is_zero() {
        Decimal::MAX
    } else {
        (acct_state.snapshot.equity * portfolio_max - acct_state.snapshot.open_portfolio_risk)
            .max(Decimal::ZERO)
    };
    let r_daily_remaining = if daily_limit.is_zero() {
        Decimal::MAX
    } else {
        (acct_state.snapshot.equity * daily_limit - acct_state.snapshot.daily_loss)
            .max(Decimal::ZERO)
    };
    let r_prop_remaining = if prop_max.is_zero() {
        Decimal::MAX
    } else {
        acct_state.snapshot.equity * prop_max
    };

    let risk = risk_engine::compute(&RiskInput {
        equity: acct_state.snapshot.equity,
        r_base_pct: r0,
        m_preserve: buf.m_preserve,
        r_buffer: buf.r_buffer,
        r_portfolio_remaining,
        r_daily_remaining,
        r_prop_remaining,
    });

    let rp_state = acct_state.state_enum.as_str().to_string();
    let disp = |v: Decimal| -> String {
        if v == Decimal::MAX {
            risk.r_target.to_string()
        } else {
            v.to_string()
        }
    };
    let f_profit_str = if acct_state.f_profit.is_zero() {
        String::new()
    } else {
        acct_state.f_profit.to_string()
    };

    Ok(RiskPreview {
        equity: acct_state.snapshot.equity.to_string(),
        initial_balance: acct_state.snapshot.initial_balance.to_string(),
        high_watermark: acct_state.snapshot.high_watermark.to_string(),
        net_profit: acct_state.net_profit.to_string(),
        dd_pct: acct_state.dd_pct.to_string(),
        state: rp_state.clone(),
        state_enum: rp_state,

        f_profit: f_profit_str,
        f_trail: acct_state.f_trail.to_string(),
        f_internal: acct_state.f_internal.to_string(),
        f_prop: acct_state.f_prop.to_string(),
        profit_retention: acct_state.profit_retention.to_string(),
        loss_capacity: acct_state.loss_capacity.to_string(),

        buffer: buf.b.to_string(),
        buffer_r: buf.buffer_r.to_string(),
        m_preserve: buf.m_preserve.to_string(),
        execution_buffer: "0".into(),

        r_base: risk.r_base.to_string(),
        r_target: risk.r_target.to_string(),
        r_buffer: risk.r_buffer.to_string(),
        r_portfolio_cap: disp(risk.r_portfolio_cap),
        r_daily_cap: disp(risk.r_daily_cap),
        r_prop_cap: disp(risk.r_prop_cap),
        r_allowed: risk.r_allowed.to_string(),
        binding_constraint: risk.binding.to_string(),

        r0_pct: cfg.r0.clone(),
        profit_activation: cfg.profit_protect_activation.clone(),
        profit_full: cfg.profit_protect_full.clone(),
        max_retention: cfg.max_profit_retention.clone(),
        trailing_dd: cfg.trailing_dd_pct.clone(),
        alpha_pct: cfg.alpha.clone(),
        daily_limit: cfg.daily_loss_limit.clone(),
        portfolio_max: cfg.r_portfolio_max.clone(),
        prop_max: cfg.r_prop_max.clone(),

        open_portfolio_risk: acct_state.snapshot.open_portfolio_risk.to_string(),
        daily_loss: acct_state.snapshot.daily_loss.to_string(),
        consecutive_losses: acct_state.snapshot.consecutive_losses,
    })
}

#[tauri::command]
pub fn equity_profile_get(
    state: State<'_, AppState>,
    account_id: i64,
) -> Result<Option<ProfileOut>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let row = crate::db::profiles::get(&conn, account_id).map_err(|e| e.to_string())?;
    Ok(row.map(|r| ProfileOut {
        account_id: r.account_id,
        account_type: r.account_type,
        profit_target_pct: r.profit_target_pct,
        max_dd_pct: r.max_dd_pct,
        profit_protect_activation: r.profit_protect_activation,
        profit_protect_full: r.profit_protect_full,
        max_profit_retention: r.max_profit_retention,
        trailing_dd_pct: r.trailing_dd_pct,
    }))
}

#[tauri::command]
pub fn equity_profile_set(
    state: State<'_, AppState>,
    account_id: i64,
    input: ProfileInputDto,
) -> Result<ProfileOut, String> {
    use crate::equity::profile::{derive, AccountType, ProfileInput};
    use rust_decimal::prelude::FromStr;

    let at = AccountType::parse(&input.account_type)?;
    let target = Decimal::from_str(&input.profit_target_pct)
        .map_err(|e| format!("profit_target_pct: {e}"))?;
    let dd = Decimal::from_str(&input.max_dd_pct).map_err(|e| format!("max_dd_pct: {e}"))?;

    let derived = derive(&ProfileInput {
        account_type: at,
        profit_target_pct: target,
        max_dd_pct: dd,
    })?;

    let row = crate::db::profiles::AccountProfileRow {
        account_id,
        account_type: at.as_str().to_string(),
        profit_target_pct: target.to_string(),
        max_dd_pct: dd.to_string(),
        profit_protect_activation: derived.profit_protect_activation.to_string(),
        profit_protect_full: derived.profit_protect_full.to_string(),
        max_profit_retention: derived.max_profit_retention.to_string(),
        trailing_dd_pct: derived.trailing_dd_pct.to_string(),
    };

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    crate::db::profiles::upsert(&tx, &row).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(ProfileOut {
        account_id,
        account_type: row.account_type,
        profit_target_pct: row.profit_target_pct,
        max_dd_pct: row.max_dd_pct,
        profit_protect_activation: row.profit_protect_activation,
        profit_protect_full: row.profit_protect_full,
        max_profit_retention: row.max_profit_retention,
        trailing_dd_pct: row.trailing_dd_pct,
    })
}

// ============================================================
// PHASE 2.1 — instrument management commands
// ============================================================

#[derive(Debug, serde::Serialize)]
pub struct InstrumentOut {
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

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct InstrumentInputDto {
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

#[tauri::command]
pub fn equity_instruments_list(state: State<'_, AppState>) -> Result<Vec<InstrumentOut>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT symbol, currency, tick_size, tick_value, contract_size,
                min_qty, max_qty, lot_step, commission_per_lot, slippage_per_unit,
                spec_source
         FROM instruments ORDER BY symbol",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<InstrumentOut> = stmt
        .query_map([], |r| {
            Ok(InstrumentOut {
                symbol: r.get(0)?,
                currency: r.get(1)?,
                tick_size: r.get(2)?,
                tick_value: r.get(3)?,
                contract_size: r.get(4)?,
                min_qty: r.get(5)?,
                max_qty: r.get(6)?,
                lot_step: r.get(7)?,
                commission_per_lot: r.get(8)?,
                slippage_per_unit: r.get(9)?,
                spec_source: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

#[tauri::command]
pub fn equity_instrument_upsert(
    state: State<'_, AppState>,
    input: InstrumentInputDto,
) -> Result<(), String> {
    use crate::domain::instrument::SpecSource;
    use rust_decimal::prelude::FromStr;

    if input.symbol.trim().is_empty() {
        return Err("symbol required".into());
    }
    if input.symbol.len() > 20 {
        return Err("symbol too long".into());
    }
    // Validate numeric fields parse
    let _ = Decimal::from_str(&input.tick_size).map_err(|e| format!("tick_size: {e}"))?;
    let _ = Decimal::from_str(&input.tick_value).map_err(|e| format!("tick_value: {e}"))?;
    let _ = Decimal::from_str(&input.contract_size).map_err(|e| format!("contract_size: {e}"))?;
    let _ = Decimal::from_str(&input.min_qty).map_err(|e| format!("min_qty: {e}"))?;
    let _ = Decimal::from_str(&input.max_qty).map_err(|e| format!("max_qty: {e}"))?;
    let _ = Decimal::from_str(&input.lot_step).map_err(|e| format!("lot_step: {e}"))?;
    let _ = Decimal::from_str(&input.commission_per_lot)
        .map_err(|e| format!("commission_per_lot: {e}"))?;
    let _ = Decimal::from_str(&input.slippage_per_unit)
        .map_err(|e| format!("slippage_per_unit: {e}"))?;

    let _ = SpecSource::parse(&input.spec_source)?;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let row = crate::db::instruments::InstrumentRow {
        symbol: input.symbol.trim().to_uppercase(),
        currency: input.currency,
        tick_size: input.tick_size,
        tick_value: input.tick_value,
        contract_size: input.contract_size,
        min_qty: input.min_qty,
        max_qty: input.max_qty,
        lot_step: input.lot_step,
        commission_per_lot: input.commission_per_lot,
        slippage_per_unit: input.slippage_per_unit,
        spec_source: input.spec_source,
    };
    crate::db::instruments::upsert(&conn, &row).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn equity_instrument_delete(state: State<'_, AppState>, symbol: String) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM instruments WHERE symbol = ?1", [&symbol])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================
// PHASE 2.2-C: daily data quality check
// ============================================================

fn check_daily_data_quality(
    conn: &rusqlite::Connection,
    account_id: Option<i64>,
    state: &crate::equity::state::AccountState,
    cfg: &EquityConfig,
) -> Result<(), crate::equity::error::DailyPnLError> {
    use std::collections::HashMap;

    let aid = match account_id {
        Some(a) => a,
        None => return Ok(()), // no account scope → skip check
    };

    // Load all trades
    let mut stmt = conn
        .prepare("SELECT id FROM trades WHERE account_id = ?1")
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?;
    let ids: Vec<i64> = stmt
        .query_map([aid], |r| r.get(0))
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?;
    let mut trades = Vec::with_capacity(ids.len());
    for id in ids {
        if let Ok(Some(t)) = crate::db::trades::get_trade(conn, id) {
            trades.push(t);
        }
    }

    // Load instruments
    let mut instruments: HashMap<String, crate::domain::instrument::InstrumentSpec> =
        HashMap::new();
    let mut stmt2 = conn
        .prepare("SELECT symbol FROM instruments")
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?;
    let syms: Vec<String> = stmt2
        .query_map([], |r| r.get(0))
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| crate::equity::error::DailyPnLError::ComputeError(e.to_string()))?;
    for sym in syms {
        if let Ok(Some(spec)) = crate::db::instruments::get_typed(conn, &sym) {
            instruments.insert(sym, spec);
        }
    }

    let reset_b = crate::equity::reset::from_config(cfg);
    let now = chrono::Utc::now();
    let day_start_time = crate::equity::reset::last_boundary(&reset_b, now);

    let inp = crate::equity::daily::DailyPnLInput {
        trades: &trades,
        day_start_equity: state.snapshot.equity,
        current_equity: state.snapshot.equity,
        external_cash_flows: Decimal::ZERO,
        day_start_time,
        instruments: &instruments,
        max_price_age_minutes: cfg.max_price_age_minutes,
        now,
        daily_loss_limit: Decimal::ZERO,
        execution_buffer: Decimal::ZERO,
    };

    crate::equity::daily::compute(&inp).map(|_| ())
}

// ============================================================
// PHASE 3 — cash flow (deposit / withdraw / fee / list)
// ============================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CashFlowInput {
    pub account_id: i64,
    pub amount: String,
    pub note: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct CashFlowOut {
    pub cash_flow_id: i64,
    pub units_row_id: i64,
    pub nav_before: String,
    pub nav_after: String,
    pub units_before: String,
    pub units_after: String,
    pub amount: String,
    pub kind: String,
}

fn cashflow_deposit_impl(
    conn: &rusqlite::Connection,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    // Load latest units
    let u = crate::db::equity::latest_units(conn, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row — run migration 028 or refresh".to_string())?;

    let units_old = Decimal::from_str(&u.units).map_err(|e| e.to_string())?;
    let nav_before = Decimal::from_str(&u.nav).map_err(|e| e.to_string())?;
    if nav_before.is_zero() {
        return Err("NAV is zero — cannot compute units".into());
    }

    let added_units = amount / nav_before;
    let units_new = units_old + added_units;
    let equity_new = units_new * nav_before; // NAV invariant

    // Currency from account
    let currency: String = conn
        .query_row(
            "SELECT currency FROM accounts WHERE id = ?1",
            [input.account_id],
            |r| r.get(0),
        )
        .map_err(|e| format!("account not found: {e}"))?;

    // Insert cash flow
    let cf_id = crate::db::equity::insert_cash_flow(
        conn,
        input.account_id,
        "deposit",
        &amount.to_string(),
        &currency,
        &nav_before.to_string(),
        &units_old.to_string(),
        &units_new.to_string(),
        input.note.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    // Insert units
    let u_id = crate::db::equity::insert_units(
        conn,
        input.account_id,
        &units_new.to_string(),
        &nav_before.to_string(),
        &equity_new.to_string(),
        "deposit",
        Some(cf_id),
    )
    .map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u_id,
        nav_before: nav_before.to_string(),
        nav_after: nav_before.to_string(),
        units_before: units_old.to_string(),
        units_after: units_new.to_string(),
        amount: amount.to_string(),
        kind: "deposit".into(),
    })
}

fn cashflow_withdraw_impl(
    conn: &rusqlite::Connection,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    let u = crate::db::equity::latest_units(conn, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row".to_string())?;

    let units_old = Decimal::from_str(&u.units).map_err(|e| e.to_string())?;
    let nav_before = Decimal::from_str(&u.nav).map_err(|e| e.to_string())?;
    if nav_before.is_zero() {
        return Err("NAV is zero".into());
    }

    let removed_units = amount / nav_before;
    let units_new = units_old - removed_units;
    if units_new <= Decimal::ZERO {
        return Err("withdrawal would zero out units".into());
    }
    let equity_new = units_new * nav_before;

    let currency: String = conn
        .query_row(
            "SELECT currency FROM accounts WHERE id = ?1",
            [input.account_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    let cf_id = crate::db::equity::insert_cash_flow(
        conn,
        input.account_id,
        "withdrawal",
        &amount.to_string(),
        &currency,
        &nav_before.to_string(),
        &units_old.to_string(),
        &units_new.to_string(),
        input.note.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    let u_id = crate::db::equity::insert_units(
        conn,
        input.account_id,
        &units_new.to_string(),
        &nav_before.to_string(),
        &equity_new.to_string(),
        "withdraw",
        Some(cf_id),
    )
    .map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u_id,
        nav_before: nav_before.to_string(),
        nav_after: nav_before.to_string(),
        units_before: units_old.to_string(),
        units_after: units_new.to_string(),
        amount: amount.to_string(),
        kind: "withdrawal".into(),
    })
}

#[tauri::command]
pub fn equity_cashflow_deposit(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let r = cashflow_deposit_impl(&tx, input)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(r)
}

#[tauri::command]
pub fn equity_cashflow_withdraw(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let r = cashflow_withdraw_impl(&tx, input)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(r)
}

#[tauri::command]
pub fn equity_cashflow_fee(
    state: State<'_, AppState>,
    input: CashFlowInput,
) -> Result<CashFlowOut, String> {
    use rust_decimal::prelude::FromStr;

    let amount = Decimal::from_str(&input.amount).map_err(|e| format!("amount: {e}"))?;
    if amount <= Decimal::ZERO {
        return Err("amount must be > 0".into());
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    let u = crate::db::equity::latest_units(&tx, input.account_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "no equity_units row".to_string())?;

    let units = u.units.clone();
    let nav = u.nav.clone();

    let currency: String = tx
        .query_row(
            "SELECT currency FROM accounts WHERE id = ?1",
            [input.account_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Fee: no units change (NAV units unaffected)
    let cf_id = crate::db::equity::insert_cash_flow(
        &tx,
        input.account_id,
        "fee",
        &amount.to_string(),
        &currency,
        &nav,
        &units,
        &units, // unchanged
        input.note.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(CashFlowOut {
        cash_flow_id: cf_id,
        units_row_id: u.id,
        nav_before: nav.clone(),
        nav_after: nav,
        units_before: units.clone(),
        units_after: units,
        amount: amount.to_string(),
        kind: "fee".into(),
    })
}

#[tauri::command]
pub fn equity_cashflow_list(
    state: State<'_, AppState>,
    account_id: i64,
    limit: Option<i64>,
) -> Result<Vec<crate::db::equity::CashFlowRow>, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let n = limit.unwrap_or(50).clamp(1, 500);
    crate::db::equity::list_cash_flows(&conn, account_id, n).map_err(|e| e.to_string())
}

// ============================================================
// PHASE 6-B — cash flow delete (revert units + remove ledger row)
// ============================================================

#[tauri::command]
pub fn equity_cashflow_delete(state: State<'_, AppState>, flow_id: i64) -> Result<(), String> {
    use rust_decimal::prelude::FromStr;

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    // Load the flow
    let (account_id, kind, amount_s, nav_before, units_before, units_after): (
        i64,
        String,
        String,
        String,
        String,
        String,
    ) = tx
        .query_row(
            "SELECT account_id, kind, amount, nav_before, units_before, units_after
             FROM account_cash_flows WHERE id = ?1",
            [flow_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .map_err(|e| format!("flow {} not found: {e}", flow_id))?;

    let amount = Decimal::from_str(&amount_s).map_err(|e| e.to_string())?;
    let nav = Decimal::from_str(&nav_before).map_err(|e| e.to_string())?;

    // Check this is the LATEST flow for this account
    let latest_flow_id: Option<i64> = tx
        .query_row(
            "SELECT id FROM account_cash_flows WHERE account_id = ?1 ORDER BY id DESC LIMIT 1",
            [account_id],
            |r| r.get(0),
        )
        .ok();

    if latest_flow_id != Some(flow_id) {
        return Err(format!(
            "cannot delete flow {} — not the latest for account {}. Delete newer flows first.",
            flow_id, account_id
        ));
    }

    // Reverse the units change
    let (new_units, new_equity): (Decimal, Decimal) = match kind.as_str() {
        "deposit" | "withdrawal" => {
            let u_before = Decimal::from_str(&units_before).map_err(|e| e.to_string())?;
            let eq = u_before * nav;
            (u_before, eq)
        }
        "fee" | "funding" => {
            let u = crate::db::equity::latest_units(&tx, account_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "no equity_units row".to_string())?;
            let cur_units = Decimal::from_str(&u.units).map_err(|e| e.to_string())?;
            let cur_equity = Decimal::from_str(&u.equity).map_err(|e| e.to_string())?;
            (cur_units, cur_equity)
        }
        other => return Err(format!("unknown kind: {}", other)),
    };

    // 1. FIRST delete equity_units that reference this flow (breaks FK)
    tx.execute(
        "DELETE FROM equity_units WHERE cash_flow_id = ?1",
        [flow_id],
    )
    .map_err(|e| e.to_string())?;

    // 2. THEN delete the cash_flow row
    tx.execute("DELETE FROM account_cash_flows WHERE id = ?1", [flow_id])
        .map_err(|e| e.to_string())?;

    // 3. Insert corrective units row
    let _ = crate::db::equity::insert_units(
        &tx,
        account_id,
        &new_units.to_string(),
        &nav.to_string(),
        &new_equity.to_string(),
        "adjustment",
        None,
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    let _ = amount;
    let _ = units_after;
    Ok(())
}

// ============================================================
// Instruments — restore defaults
// ============================================================

#[tauri::command]
pub fn equity_instruments_restore_defaults(state: State<'_, AppState>) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    let defaults = [
        // (symbol, tick_size, tick_value, contract_size, min_qty, max_qty, lot_step)
        ("EURUSD", "0.00001", "1", "100000", "0.01", "100", "0.01"),
        ("GBPUSD", "0.00001", "1", "100000", "0.01", "100", "0.01"),
        ("USDJPY", "0.001", "1", "100000", "0.01", "100", "0.01"),
        ("AUDUSD", "0.00001", "1", "100000", "0.01", "100", "0.01"),
        ("USDCAD", "0.00001", "0.73", "100000", "0.01", "100", "0.01"),
        ("USDCHF", "0.00001", "0.79", "100000", "0.01", "100", "0.01"),
        ("NZDUSD", "0.00001", "1", "100000", "0.01", "100", "0.01"),
        ("EURGBP", "0.00001", "1.27", "100000", "0.01", "100", "0.01"),
        ("EURJPY", "0.001", "1", "100000", "0.01", "100", "0.01"),
        ("GBPJPY", "0.001", "1", "100000", "0.01", "100", "0.01"),
        ("CHFJPY", "0.001", "1", "100000", "0.01", "100", "0.01"),
        ("XAUUSD", "0.01", "1", "100", "0.01", "50", "0.01"),
        ("XAGUSD", "0.001", "5", "5000", "0.01", "50", "0.01"),
        ("BTCUSD", "0.01", "0.01", "1", "0.01", "10", "0.01"),
        ("ETHUSD", "0.01", "0.01", "1", "0.01", "100", "0.01"),
        ("NAS100", "0.1", "0.1", "1", "0.1", "50", "0.1"),
        ("SPX500", "0.1", "0.1", "1", "0.1", "50", "0.1"),
        ("US30", "0.1", "0.1", "1", "0.1", "50", "0.1"),
    ];

    let now = chrono::Utc::now().to_rfc3339();
    let mut inserted = 0usize;

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    for (sym, tick_size, tick_value, contract_size, min_qty, max_qty, lot_step) in defaults.iter() {
        let rows = tx
            .execute(
                "INSERT OR IGNORE INTO instruments
               (symbol, currency, tick_size, tick_value, contract_size,
                min_qty, max_qty, lot_step, commission_per_lot, slippage_per_unit,
                spec_source, created_at, updated_at)
             VALUES (?1, 'USD', ?2, ?3, ?4, ?5, ?6, ?7, '0', '0',
                     'user_entered', ?8, ?8)",
                rusqlite::params![
                    sym,
                    tick_size,
                    tick_value,
                    contract_size,
                    min_qty,
                    max_qty,
                    lot_step,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        inserted += rows;
    }
    tx.commit().map_err(|e| e.to_string())?;

    Ok(inserted)
}

// ============================================================
// Instruments — export/import (backup)
// ============================================================

#[tauri::command]
pub fn equity_instruments_export_json(state: State<'_, AppState>) -> Result<String, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;

    #[derive(serde::Serialize)]
    struct InstRow {
        symbol: String,
        currency: String,
        tick_size: String,
        tick_value: String,
        contract_size: String,
        min_qty: String,
        max_qty: String,
        lot_step: String,
        commission_per_lot: String,
        slippage_per_unit: String,
        spec_source: String,
    }

    let mut stmt = conn
        .prepare(
            "SELECT symbol, currency, tick_size, tick_value, contract_size,
                    min_qty, max_qty, lot_step, commission_per_lot, slippage_per_unit,
                    spec_source
             FROM instruments ORDER BY symbol",
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<InstRow> = stmt
        .query_map([], |r| {
            Ok(InstRow {
                symbol: r.get(0)?,
                currency: r.get(1)?,
                tick_size: r.get(2)?,
                tick_value: r.get(3)?,
                contract_size: r.get(4)?,
                min_qty: r.get(5)?,
                max_qty: r.get(6)?,
                lot_step: r.get(7)?,
                commission_per_lot: r.get(8)?,
                slippage_per_unit: r.get(9)?,
                spec_source: r.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    serde_json::to_string_pretty(&rows).map_err(|e| e.to_string())
}

#[derive(Debug, serde::Deserialize)]
pub struct ImportInstrumentRow {
    pub symbol: String,
    pub currency: String,
    pub tick_size: String,
    pub tick_value: String,
    pub contract_size: String,
    #[serde(default = "default_min_qty")]
    pub min_qty: String,
    #[serde(default = "default_max_qty")]
    pub max_qty: String,
    #[serde(default = "default_lot_step")]
    pub lot_step: String,
    #[serde(default)]
    pub commission_per_lot: String,
    #[serde(default)]
    pub slippage_per_unit: String,
    #[serde(default = "default_source")]
    pub spec_source: String,
}

fn default_min_qty() -> String {
    "0.01".into()
}
fn default_max_qty() -> String {
    "100".into()
}
fn default_lot_step() -> String {
    "0.01".into()
}
fn default_source() -> String {
    "user_entered".into()
}

#[tauri::command]
pub fn equity_instruments_import_json(
    state: State<'_, AppState>,
    json: String,
    replace_all: bool,
) -> Result<usize, String> {
    use rust_decimal::prelude::FromStr;

    let rows: Vec<ImportInstrumentRow> =
        serde_json::from_str(&json).map_err(|e| format!("parse JSON: {e}"))?;

    if rows.is_empty() {
        return Ok(0);
    }

    // Validate each row
    for r in &rows {
        if r.symbol.trim().is_empty() {
            return Err("symbol required".into());
        }
        let _ =
            Decimal::from_str(&r.tick_size).map_err(|e| format!("{}: tick_size: {e}", r.symbol))?;
        let _ = Decimal::from_str(&r.tick_value)
            .map_err(|e| format!("{}: tick_value: {e}", r.symbol))?;
        let _ = Decimal::from_str(&r.contract_size)
            .map_err(|e| format!("{}: contract_size: {e}", r.symbol))?;
    }

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;

    if replace_all {
        tx.execute("DELETE FROM instruments", [])
            .map_err(|e| format!("delete all: {e}"))?;
    }

    let mut count = 0usize;
    for r in &rows {
        let n = tx
            .execute(
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
                    r.symbol.trim().to_uppercase(),
                    r.currency,
                    r.tick_size,
                    r.tick_value,
                    r.contract_size,
                    r.min_qty,
                    r.max_qty,
                    r.lot_step,
                    if r.commission_per_lot.is_empty() {
                        "0".into()
                    } else {
                        r.commission_per_lot.clone()
                    },
                    if r.slippage_per_unit.is_empty() {
                        "0".into()
                    } else {
                        r.slippage_per_unit.clone()
                    },
                    if r.spec_source.is_empty() {
                        "user_entered".into()
                    } else {
                        r.spec_source.clone()
                    },
                    now,
                ],
            )
            .map_err(|e| format!("insert {}: {e}", r.symbol))?;
        count += n;
    }

    tx.commit().map_err(|e| format!("commit: {e}"))?;
    Ok(count)
}

// ============================================================
// PHASE 5: Recovery authorization commands
// ============================================================

#[derive(Debug, serde::Serialize)]
pub struct RecoveryStatusOut {
    pub active: bool,
    pub authorized_at: Option<String>,
    pub expires_at: Option<String>,
    pub reason: Option<String>,
    pub probe_risk_pct: Option<String>,
}

#[tauri::command]
pub fn equity_recovery_get(
    state: State<'_, AppState>,
    account_id: i64,
) -> Result<RecoveryStatusOut, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let row = crate::db::recovery::get_active(&conn, account_id).map_err(|e| e.to_string())?;
    Ok(match row {
        Some(r) => RecoveryStatusOut {
            active: true,
            authorized_at: Some(r.authorized_at),
            expires_at: r.expires_at,
            reason: r.reason,
            probe_risk_pct: Some(r.probe_risk_pct),
        },
        None => RecoveryStatusOut {
            active: false,
            authorized_at: None,
            expires_at: None,
            reason: None,
            probe_risk_pct: None,
        },
    })
}

#[derive(Debug, serde::Deserialize)]
pub struct RecoveryGrantInput {
    pub account_id: i64,
    pub reason: Option<String>,
    pub expires_in_hours: Option<i64>,
    pub probe_risk_pct: Option<String>,
}

#[tauri::command]
pub fn equity_recovery_grant(
    state: State<'_, AppState>,
    input: RecoveryGrantInput,
) -> Result<RecoveryStatusOut, String> {
    use rust_decimal::prelude::FromStr;

    let pct = input.probe_risk_pct.clone().unwrap_or_else(|| "0.25".into());
    let pct_d = Decimal::from_str(&pct).map_err(|e| format!("probe_risk_pct: {e}"))?;
    if pct_d <= Decimal::ZERO || pct_d > Decimal::ONE {
        return Err("probe_risk_pct must be in (0, 1]".into());
    }

    let expires_at = input.expires_in_hours.map(|h| {
        (chrono::Utc::now() + chrono::Duration::hours(h)).to_rfc3339()
    });

    let conn = state.db.lock().map_err(|e| e.to_string())?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    crate::db::recovery::insert(
        &tx,
        input.account_id,
        "user",
        expires_at.as_deref(),
        input.reason.as_deref(),
        &pct,
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(RecoveryStatusOut {
        active: true,
        authorized_at: Some(chrono::Utc::now().to_rfc3339()),
        expires_at,
        reason: input.reason,
        probe_risk_pct: Some(pct),
    })
}

#[tauri::command]
pub fn equity_recovery_revoke(
    state: State<'_, AppState>,
    account_id: i64,
) -> Result<usize, String> {
    let conn = state.db.lock().map_err(|e| e.to_string())?;
    crate::db::recovery::revoke(&conn, account_id).map_err(|e| e.to_string())
}
