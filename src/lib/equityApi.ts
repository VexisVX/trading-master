import { invoke } from "@tauri-apps/api/core";

/* ============================================================
 *  TYPES — mirror Rust DTOs
 * ============================================================ */

export interface EquitySnapshotRow {
  id: number;
  taken_at: string;
  equity: string;
  initial_balance: string;
  high_watermark: string;
  low_watermark: string;
  day_start_equity: string;
  open_portfolio_risk: string;
  daily_loss: string;
  consecutive_losses: number;
  created_at?: string;
  phase?: string | null;
  protected_profit?: string | null;
  equity_floor?: string | null;
  loss_capacity?: string | null;
  // S10: new columns (nullable for legacy rows)
  state?: string | null;
  buffer_r?: string | null;
  m_preserve?: string | null;
  f_internal?: string | null;
  f_prop_val?: string | null;
}

export interface SnapshotCreateInput {
  equity: string;
  initial_balance: string;
  low_watermark: string;
  day_start_equity: string;
  open_portfolio_risk: string;
  daily_loss: string;
  consecutive_losses: number;
}

export interface TradeProposal {
  // New (PHASE 2.1): symbol drives instrument lookup
  symbol?: string | null;
  entry: string;
  stop: string;
  take_profit: string | null;
  // legacy (only used if allow_legacy=true)
  allow_legacy?: boolean;
  contract_multiplier?: string;
  commission_per_unit?: string;
  slippage_buffer?: string;
}

/** Rust serializes DecisionReason as { kind, detail? } (serde tagged enum). */
export interface DecisionReasonObject {
  kind: string;
  detail?: Record<string, any> | null;
}

/** Full breakdown behind a decision — powers the "Why this size?" modal. */
export interface RiskDetail {
  equity: string;
  r_base: string;
  r_portfolio_cap: string;
  r_daily_cap: string;
  r_prop_cap: string;
  r_allowed: string;
  entry_minus_stop: string;
  contract_multiplier: string;
  per_unit_cost: string;
  buffer_r: string;
  m_preserve: string;
  f_internal: string;
  f_prop: string;
  binding_constraint: string;
  r_target: string;
  r_buffer: string;
}

export interface DecisionOut {
  allowed: boolean;
  reason: DecisionReasonObject;
  r_allowed: string;
  size: string | null;
  rr: string | null;
  // legacy
  phase: string;
  dd_pct: string;
  f_hybrid: string;
  loss_capacity: string;
  // new (S8)
  state: string;
  state_enum: string;
  buffer_r: string;
  m_preserve: string;
  f_internal: string;
  f_prop: string;
  snapshot_id: number | null;
  risk_detail: RiskDetail | null;
  sizing_source?: string;
  actual_risk?: string | null;
  instrument_symbol?: string | null;
}

export interface PreEvalResult {
  /** "ok" | "warn" | "block" | "reject" */
  verdict: string;
  allowed_size: string | null;
  user_size: string;
  /** user_size / allowed_size, decimal-string, empty if allowed is 0 */
  ratio: string;
  reason: string;
  allowed: boolean;
}



export type DecisionVerdict = "allowed" | "reduced" | "rejected" | "paused";

export interface DecisionRowOut {
  id: number;
  snapshot_id: number;
  allowed: boolean;
  reason_kind: string;
  reason_detail: string | null;
  r_allowed: string | null;
  size: string | null;
  created_at: string;
  risk_reduced: boolean;
  phase: string;
}

/** Classify a decision into one of 4 visual states. */
export function decisionVerdict(d: DecisionRowOut): DecisionVerdict {
  if (d.phase === "paused" || d.phase === "disabled") return "paused";
  if (!d.allowed) return "rejected";
  if (d.risk_reduced) return "reduced";
  return "allowed";
}

/** Human-readable label for a verdict. */
export function verdictLabel(v: DecisionVerdict): string {
  switch (v) {
    case "allowed":  return "Allowed";
    case "reduced":  return "Reduced risk";
    case "rejected": return "Rejected";
    case "paused":   return "Paused";
  }
}

/** Icon for a verdict. */
export function verdictIcon(v: DecisionVerdict): string {
  switch (v) {
    case "allowed":  return "✅";
    case "reduced":  return "⚠️";
    case "rejected": return "❌";
    case "paused":   return "⏸";
  }
}

/** CSS class suffix for a verdict (used with .ee-chip--*). */
export function verdictTone(v: DecisionVerdict): string {
  switch (v) {
    case "allowed":  return "ok";
    case "reduced":  return "warn";
    case "rejected": return "bad";
    case "paused":   return "muted";
  }
}

export interface EquityConfig {
  r0: string;

  // v2 protection (smoothstep)
  profit_protect_activation: string;
  profit_protect_full: string;
  max_profit_retention: string;
  trailing_dd_pct: string;

  // prop budget (legacy D_prop)
  d_prop: string;

  // risk capacity
  alpha: string;
  daily_loss_limit: string;
  r_portfolio_max: string;
  r_prop_max: string;

  // execution buffer
  execution_buffer_pct: string;

  // recovery
  recovery_probe_pct: string;

  // snapshot policy
  max_snapshot_age_hours: number;

  // legacy fields kept (accepted but ignored)
  r_min?: string;
  delta_r?: string;
  dd_1?: string;
  dd_2?: string;
  dd_3?: string;
  m_dd_1?: string;
  m_dd_2?: string;
  m_dd_3?: string;
  m_dd_4?: string;
  m_loss_table?: string;
  n_max?: number;
  q_retention?: string;
  d_trailing?: string;
  mu_max?: string;
  recovery_target?: string;
}

/** Derived account status — computed from snapshot for display. */

export interface RiskPreview {
  equity: string;
  initial_balance: string;
  high_watermark: string;
  net_profit: string;
  dd_pct: string;
  state: string;
  state_enum: string;

  f_profit: string;
  f_trail: string;
  f_internal: string;
  f_prop: string;
  profit_retention: string;
  loss_capacity: string;

  buffer: string;
  buffer_r: string;
  m_preserve: string;
  execution_buffer: string;

  r_base: string;
  r_target: string;
  r_buffer: string;
  r_portfolio_cap: string;
  r_daily_cap: string;
  r_prop_cap: string;
  r_allowed: string;
  binding_constraint: string;

  r0_pct: string;
  profit_activation: string;
  profit_full: string;
  max_retention: string;
  trailing_dd: string;
  alpha_pct: string;
  daily_limit: string;
  portfolio_max: string;
  prop_max: string;

  open_portfolio_risk: string;
  daily_loss: string;
  consecutive_losses: number;
}

export interface AccountInfo {
  id: number;
  name: string;
  currency: string;
  initial_balance: string;
  is_active: boolean;
  kind: string;
}

export interface AccountStatus {
  label: "Healthy" | "Caution" | "Warning" | "Paused" | "Disabled";
  tone: "pos" | "warn" | "neg" | "";
  snapshot: EquitySnapshotRow | null;
  equity: number;
  hwm: number;
  lwm: number;
  ddPct: number;
  netProfit: number;
  openRisk: number;
  dailyLoss: number;
  streak: number;
  snapshotAgeHours: number;
}

/* ============================================================
 *  API FUNCTIONS — match Rust command names exactly
 * ============================================================ */

export async function getLatestSnapshot(
  accountId?: number
): Promise<EquitySnapshotRow | null> {
  return invoke<EquitySnapshotRow | null>("equity_snapshot_latest", {
    accountId: accountId ?? null,
  });
}

export async function createSnapshot(
  input: SnapshotCreateInput
): Promise<number> {
  return invoke<number>("equity_snapshot_create", { input });
}

export async function autoSnapshotFromTrades(
  accountId?: number
): Promise<number> {
  return invoke<number>("equity_auto_snapshot_create", {
    accountId: accountId ?? null,
  });
}

export async function listDecisions(limit: number): Promise<DecisionRowOut[]> {
  return invoke<DecisionRowOut[]>("equity_decisions_list", { limit });
}

export async function evaluateTrade(
  accountId: number | undefined,
  proposal: TradeProposal | null
): Promise<DecisionOut> {
  return invoke<DecisionOut>("equity_evaluate", {
    accountId: accountId ?? null,
    proposal,
  });
}


/**
 * Pre-check a proposed trade WITHOUT saving it.
 * Returns verdict: ok | warn | block | reject.
 */
export async function preEvaluateTrade(
  accountId: number | undefined,
  input: {
    entry: string;
    stop: string;
    take_profit: string | null;
    contract_multiplier: string;
    size: string;
  }
): Promise<PreEvalResult> {
  return invoke<PreEvalResult>("equity_pre_evaluate", {
    accountId: accountId ?? null,
    entry: input.entry,
    stop: input.stop,
    takeProfit: input.take_profit,
    contractMultiplier: input.contract_multiplier,
    size: input.size,
  });
}


export async function previewRisk(
  accountId?: number
): Promise<RiskPreview> {
  return invoke<RiskPreview>("equity_preview_risk", {
    accountId: accountId ?? null,
  });
}

export async function getEquityConfig(): Promise<EquityConfig> {
  return invoke<EquityConfig>("equity_config_get");
}

export async function setEquityConfig(
  config: EquityConfig
): Promise<EquityConfig> {
  return invoke<EquityConfig>("equity_config_set", { config });
}

/* ============================================================
 *  MANAGEMENT — accounts & snapshots
 * ============================================================ */

export async function listAccounts(): Promise<AccountInfo[]> {
  return invoke<AccountInfo[]>("equity_list_accounts");
}

export async function listSnapshots(
  accountId?: number,
  limit: number = 50
): Promise<EquitySnapshotRow[]> {
  return invoke<EquitySnapshotRow[]>("equity_list_snapshots", {
    accountId: accountId ?? null,
    limit,
  });
}

export async function deleteSnapshot(id: number): Promise<void> {
  return invoke<void>("equity_delete_snapshot", { id });
}

export async function clearSnapshots(): Promise<number> {
  return invoke<number>("equity_clear_snapshots");
}

export async function cleanupOldSnapshots(days: number): Promise<number> {
  return invoke<number>("equity_cleanup_old_snapshots", { days });
}


/* ============================================================
 *  NUMBER HELPERS
 * ============================================================ */

export function parseDec(s: string | null | undefined): number | null {
  if (s === null || s === undefined || s === "") return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

export function fmtMoney(s: string | number | null | undefined): string {
  if (s === null || s === undefined || s === "") return "—";
  const n = typeof s === "number" ? s : Number(s);
  if (!Number.isFinite(n)) return "—";
  return n.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function fmtPct(s: string | number | null | undefined): string {
  if (s === null || s === undefined || s === "") return "—";
  const n = typeof s === "number" ? s : Number(s);
  if (!Number.isFinite(n)) return "—";
  return (n * 100).toFixed(2) + "%";
}

/* ============================================================
 *  REASON HELPERS
 * ============================================================ */

export function humanizeReason(reason: DecisionReasonObject | string): string {
  const kind = typeof reason === "string" ? reason : reason?.kind ?? "";

  const map: Record<string, string> = {
    Allowed: "Trade allowed — size computed within risk limits.",
    AccountDisabled: "Account is disabled — trading is blocked.",
    AccountPaused: "Account is paused — trading is blocked.",
    EquityBelowFloor:
      "Equity has fallen below the safety floor — trading blocked.",
    DailyLossExceeded:
      "Daily loss limit reached — trading paused until tomorrow.",
    PortfolioRiskFull:
      "Open portfolio risk is at the limit — close a trade first.",
    FloorRiskCapacityZero: "Floor risk capacity is zero — no room to trade.",
    AdaptiveRiskZero: "Adaptive risk is zero — no room to trade.",
    RAllowedZeroOrNegative: "Allowed risk is zero or negative — cannot size.",
    SnapshotTooOld:
      "Snapshot is older than the allowed age — refresh from trades.",
    InvalidConfig: "Equity configuration is invalid — check settings.",
  };

  return map[kind] ?? kind;
}

export function reasonDetail(
  reason: DecisionReasonObject | string
): Record<string, any> | null {
  if (typeof reason === "string") return null;
  return reason?.detail ?? null;
}

/* ============================================================
 *  STATUS HELPER
 * ============================================================ */

export function deriveStatus(snap: EquitySnapshotRow | null): AccountStatus {
  if (!snap) {
    return {
      label: "Disabled",
      tone: "neg",
      snapshot: null,
      equity: 0,
      hwm: 0,
      lwm: 0,
      ddPct: 0,
      netProfit: 0,
      openRisk: 0,
      dailyLoss: 0,
      streak: 0,
      snapshotAgeHours: Infinity,
    };
  }

  const equity = parseDec(snap.equity) ?? 0;
  const hwm = parseDec(snap.high_watermark) ?? equity;
  const lwm = parseDec(snap.low_watermark) ?? equity;
  const initial = parseDec(snap.initial_balance) ?? equity;
  const openRisk = parseDec(snap.open_portfolio_risk) ?? 0;
  const dailyLoss = Math.abs(parseDec(snap.daily_loss) ?? 0);
  const ddPct = hwm > 0 ? (hwm - equity) / hwm : 0;
  const netProfit = equity - initial;
  const ageMs = Date.now() - new Date(snap.taken_at).getTime();
  const snapshotAgeHours = ageMs / (1000 * 60 * 60);

  let label: AccountStatus["label"];
  let tone: AccountStatus["tone"];

  // S10: prefer persisted state_enum. Fallback to legacy thresholds if absent.
  const stateEnum = snap.state ?? null;
  if (stateEnum) {
    switch (stateEnum) {
      case "normal_growth":
        label = "Healthy"; tone = "pos"; break;
      case "protected":
        label = "Caution"; tone = "warn"; break;
      case "preservation_pause":
        label = "Paused"; tone = "neg"; break;
      case "recovery_mode":
        label = "Warning"; tone = "warn"; break;
      case "daily_lock":
        label = "Paused"; tone = "neg"; break;
      case "prop_hard_lock":
        label = "Disabled"; tone = "neg"; break;
      default:
        label = "Healthy"; tone = "pos";
    }
  } else if (ddPct >= 0.3) {
    label = "Disabled"; tone = "neg";
  } else if (snap.consecutive_losses >= 5) {
    label = "Paused"; tone = "neg";
  } else if (ddPct >= 0.1 || snap.consecutive_losses >= 3) {
    label = "Warning"; tone = "neg";
  } else if (ddPct >= 0.05 || snap.consecutive_losses >= 2) {
    label = "Caution"; tone = "warn";
  } else {
    label = "Healthy"; tone = "pos";
  }

  return {
    label,
    tone,
    snapshot: snap,
    equity,
    hwm,
    lwm,
    ddPct,
    netProfit,
    openRisk,
    dailyLoss,
    streak: snap.consecutive_losses,
    snapshotAgeHours,
  };
}

/* ============================================================
 *  STATE MACHINE HELPERS (S8)
 * ============================================================ */

export type EngineState =
  | "normal_growth"
  | "protected"
  | "preservation_pause"
  | "recovery_mode"
  | "daily_lock"
  | "prop_hard_lock";

export function stateLabel(s: string): string {
  switch (s) {
    case "normal_growth":      return "Normal Growth";
    case "protected":          return "Protected";
    case "preservation_pause": return "Preservation Pause";
    case "recovery_mode":      return "Recovery Mode";
    case "daily_lock":         return "Daily Lock";
    case "prop_hard_lock":     return "Prop Hard Lock";
    default:                   return s || "—";
  }
}

export function stateIcon(s: string): string {
  switch (s) {
    case "normal_growth":      return "🟢";
    case "protected":          return "🟡";
    case "preservation_pause": return "⏸";
    case "recovery_mode":      return "🔬";
    case "daily_lock":         return "🔒";
    case "prop_hard_lock":     return "🚫";
    default:                   return "•";
  }
}

export function stateTone(s: string): "ok" | "warn" | "bad" | "muted" | "" {
  switch (s) {
    case "normal_growth":      return "ok";
    case "protected":          return "warn";
    case "preservation_pause": return "muted";
    case "recovery_mode":      return "warn";
    case "daily_lock":         return "bad";
    case "prop_hard_lock":     return "bad";
    default:                   return "";
  }
}

export function isBlockingState(s: string): boolean {
  return s === "preservation_pause" || s === "daily_lock" || s === "prop_hard_lock";
}

/* ============================================================
 *  ACCOUNT PROFILE (S14)
 * ============================================================ */

export type AccountType = "personal" | "prop_firm";

export interface AccountProfile {
  account_id: number;
  account_type: AccountType;
  profit_target_pct: string;
  max_dd_pct: string;
  profit_protect_activation: string;
  profit_protect_full: string;
  max_profit_retention: string;
  trailing_dd_pct: string;
}

export interface ProfileInput {
  account_type: AccountType;
  profit_target_pct: string;
  max_dd_pct: string;
}

export async function getAccountProfile(accountId: number): Promise<AccountProfile | null> {
  return invoke<AccountProfile | null>("equity_profile_get", { accountId });
}

export async function setAccountProfile(
  accountId: number,
  input: ProfileInput
): Promise<AccountProfile> {
  return invoke<AccountProfile>("equity_profile_set", { accountId, input });
}



/* ============================================================
 * INSTRUMENTS (PHASE 2.1)
 * ============================================================ */

export type SpecSource = "verified" | "user_entered" | "estimated";

export interface Instrument {
  symbol: string;
  currency: string;
  tick_size: string;
  tick_value: string;
  contract_size: string;
  min_qty: string;
  max_qty: string;
  lot_step: string;
  commission_per_lot: string;
  slippage_per_unit: string;
  spec_source: SpecSource;
}

export interface InstrumentInput {
  symbol: string;
  currency: string;
  tick_size: string;
  tick_value: string;
  contract_size: string;
  min_qty: string;
  max_qty: string;
  lot_step: string;
  commission_per_lot: string;
  slippage_per_unit: string;
  spec_source: SpecSource;
}

export async function listInstruments(): Promise<Instrument[]> {
  return invoke<Instrument[]>("equity_instruments_list");
}

export async function upsertInstrument(input: InstrumentInput): Promise<void> {
  return invoke<void>("equity_instrument_upsert", { input });
}

export async function deleteInstrument(symbol: string): Promise<void> {
  return invoke<void>("equity_instrument_delete", { symbol });
}

export async function restoreDefaultInstruments(): Promise<number> {
  return invoke<number>("equity_instruments_restore_defaults");
}

export async function exportInstrumentsJSON(): Promise<string> {
  return invoke<string>("equity_instruments_export_json");
}

export async function importInstrumentsJSON(
  json: string,
  replaceAll: boolean
): Promise<number> {
  return invoke<number>("equity_instruments_import_json", { json, replaceAll });
}




/* ============================================================
 * CASH FLOW (PHASE 3)
 * ============================================================ */

export interface CashFlowInput {
  account_id: number;
  amount: string;
  note?: string | null;
}

export interface CashFlowOut {
  cash_flow_id: number;
  units_row_id: number;
  nav_before: string;
  nav_after: string;
  units_before: string;
  units_after: string;
  amount: string;
  kind: string;
}

export interface CashFlowRow {
  id: number;
  account_id: number;
  kind: string;
  amount: string;
  currency: string;
  occurred_at: string;
  nav_before: string;
  units_before: string;
  units_after: string;
  note: string | null;
}

export async function cashflowDeposit(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_deposit", { input });
}

export async function cashflowWithdraw(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_withdraw", { input });
}

export async function cashflowFee(input: CashFlowInput): Promise<CashFlowOut> {
  return invoke<CashFlowOut>("equity_cashflow_fee", { input });
}

export async function listCashFlows(accountId: number, limit = 50): Promise<CashFlowRow[]> {
  return invoke<CashFlowRow[]>("equity_cashflow_list", { accountId, limit });
}

export async function deleteCashFlow(flowId: number): Promise<void> {
  return invoke<void>("equity_cashflow_delete", { flowId });
}



/* ============================================================
 * RECOVERY MODE (PHASE 5)
 * ============================================================ */

export interface RecoveryStatus {
  active: boolean;
  authorized_at: string | null;
  expires_at: string | null;
  reason: string | null;
  probe_risk_pct: string | null;
}

export interface RecoveryGrantInput {
  account_id: number;
  reason?: string | null;
  expires_in_hours?: number | null;
  probe_risk_pct?: string | null;
}

export async function getRecoveryStatus(accountId: number): Promise<RecoveryStatus> {
  return invoke<RecoveryStatus>("equity_recovery_get", { accountId });
}

export async function grantRecovery(input: RecoveryGrantInput): Promise<RecoveryStatus> {
  return invoke<RecoveryStatus>("equity_recovery_grant", { input });
}

export async function revokeRecovery(accountId: number): Promise<number> {
  return invoke<number>("equity_recovery_revoke", { accountId });
}

/* ============================================================
 *  DEFAULTS
 * ============================================================ */

export function defaultConfig(): EquityConfig {
  return {
    r0: "0.01",
    profit_protect_activation: "0.05",
    profit_protect_full: "0.10",
    max_profit_retention: "0.40",
    trailing_dd_pct: "0.10",
    d_prop: "0",
    alpha: "0.5",
    daily_loss_limit: "0",
    r_portfolio_max: "0",
    r_prop_max: "0",
    execution_buffer_pct: "0.001",
    recovery_probe_pct: "0.25",
    max_snapshot_age_hours: 24,
  };
}

/* ============================================================
 *  TIME HELPERS
 * ============================================================ */

export function timeAgo(iso: string): string {
  const then = new Date(iso).getTime();
  if (!Number.isFinite(then)) return "—";
  const sec = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}
