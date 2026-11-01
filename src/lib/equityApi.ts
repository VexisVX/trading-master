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
  created_at: string;
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
  entry: string;
  stop: string;
  take_profit: string | null;
  contract_multiplier: string;
  commission_per_unit: string;
  slippage_buffer: string;
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
  m_dd: string;
  m_loss: string;
  r_adaptive: string;
  r_portfolio_cap: string;
  r_daily_cap: string;
  r_floor_cap: string;
  r_prop_cap: string;
  r_allowed: string;
  entry_minus_stop: string;
  contract_multiplier: string;
  per_unit_cost: string;
}

export interface DecisionOut {
  allowed: boolean;
  reason: DecisionReasonObject;
  r_allowed: string;
  size: string | null;
  rr: string | null;
  phase: string;
  dd_pct: string;
  f_hybrid: string;
  loss_capacity: string;
  snapshot_id: number | null;
  risk_detail: RiskDetail | null;
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
  r_min: string;
  delta_r: string;
  dd_1: string;
  dd_2: string;
  dd_3: string;
  m_dd_1: string;
  m_dd_2: string;
  m_dd_3: string;
  m_dd_4: string;
  m_loss_table: string;
  n_max: number;
  q_retention: string;
  d_trailing: string;
  d_prop: string;
  alpha: string;
  daily_loss_limit: string;
  r_portfolio_max: string;
  r_prop_max: string;
  mu_max: string;
  max_snapshot_age_hours: number;
  recovery_target: string;
}

/** Derived account status — computed from snapshot for display. */

export interface RiskPreview {
  equity: string;
  initial_balance: string;
  high_watermark: string;
  net_profit: string;
  dd_pct: string;
  phase: string;
  loss_capacity: string;
  profit_retention: string;
  f_profit: string;
  f_trail: string;
  f_prop: string;
  f_hybrid: string;

  r_base: string;
  m_dd: string;
  m_loss: string;
  r_adaptive: string;

  r_portfolio_cap: string;
  r_daily_cap: string;
  r_floor_cap: string;
  r_prop_cap: string;

  r_allowed: string;
  binding_constraint: string;

  r0_pct: string;
  dd_1_pct: string;
  dd_2_pct: string;
  dd_3_pct: string;
  m_dd_1: string;
  m_dd_2: string;
  m_dd_3: string;
  m_dd_4: string;
  m_loss_table: string;
  n_max: number;
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

export async function getLatestSnapshot(): Promise<EquitySnapshotRow | null> {
  return invoke<EquitySnapshotRow | null>("equity_snapshot_latest");
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
  proposal: TradeProposal | null
): Promise<DecisionOut> {
  return invoke<DecisionOut>("equity_evaluate", { proposal });
}


/**
 * Pre-check a proposed trade WITHOUT saving it.
 * Returns verdict: ok | warn | block | reject.
 */
export async function preEvaluateTrade(input: {
  entry: string;
  stop: string;
  take_profit: string | null;
  contract_multiplier: string;
  size: string;
}): Promise<PreEvalResult> {
  return invoke<PreEvalResult>("equity_pre_evaluate", {
    entry: input.entry,
    stop: input.stop,
    takeProfit: input.take_profit,
    contractMultiplier: input.contract_multiplier,
    size: input.size,
  });
}


export async function previewRisk(): Promise<RiskPreview> {
  return invoke<RiskPreview>("equity_preview_risk");
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

export async function listSnapshots(limit: number = 50): Promise<EquitySnapshotRow[]> {
  return invoke<EquitySnapshotRow[]>("equity_list_snapshots", { limit });
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

  if (ddPct >= 0.3) {
    label = "Disabled";
    tone = "neg";
  } else if (snap.consecutive_losses >= 5) {
    label = "Paused";
    tone = "neg";
  } else if (ddPct >= 0.1 || snap.consecutive_losses >= 3) {
    label = "Warning";
    tone = "neg";
  } else if (ddPct >= 0.05 || snap.consecutive_losses >= 2) {
    label = "Caution";
    tone = "warn";
  } else {
    label = "Healthy";
    tone = "pos";
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
 *  DEFAULTS
 * ============================================================ */

export function defaultConfig(): EquityConfig {
  return {
    r0: "0.01",
    r_min: "0.0025",
    delta_r: "0.005",
    dd_1: "0.05",
    dd_2: "0.10",
    dd_3: "0.20",
    m_dd_1: "0.5",
    m_dd_2: "0.25",
    m_dd_3: "0.1",
    m_dd_4: "0.05",
    m_loss_table: "1.0,1.0,0.75,0.5,0.25,0,0",
    n_max: 5,
    q_retention: "0.3",
    d_trailing: "0.1",
    d_prop: "0.06",
    alpha: "0.5",
    daily_loss_limit: "0.03",
    r_portfolio_max: "0.06",
    r_prop_max: "0.02",
    mu_max: "1.0",
    max_snapshot_age_hours: 24,
    recovery_target: "0.5",
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
