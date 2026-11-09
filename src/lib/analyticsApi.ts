import { invoke } from "@tauri-apps/api/core";

// All Decimal values are STRINGS.
// Display-only — never compute in JS.

export interface EquityPoint { t: number; v: string; }
export interface RBucket { label: string; count: number; }
export interface TradeRDetail {
  id: number;
  symbol: string;
  side: "long" | "short";
  r_multiple: string;
  net_pnl: string;
  closed_at: string;
}

export interface Summary {
  initial_balance: string;
  current_equity: string;
  net_pnl: string;
  total_trades: number;
  open_trades: number;
  closed_trades: number;
  wins: number;
  losses: number;
  win_rate_pct: string;
  gross_profit: string;
  gross_loss: string;
  profit_factor: string;
  avg_win_r: string;
  avg_loss_r: string;
  expectancy_r: string;
  max_drawdown: string;
  max_drawdown_pct: string;
  equity_curve: EquityPoint[];
  drawdown_curve: EquityPoint[];
  r_distribution: RBucket[];
  r_details: TradeRDetail[];
  avg_planned_rr: string;
  avg_realized_r: string;
  plan_adherence_pct: string;
  tp_hit_count: number;
  sl_hit_count: number;
  breakeven_count: number;
  manual_count: number;
  early_exit_count: number;
}

export async function getSummary(accountId?: number): Promise<Summary> {
  const raw = await invoke<string>("analytics_summary", {
    accountId: accountId ?? null,
  });
  return JSON.parse(raw);
}

export async function getRecentR(limit = 5): Promise<TradeRDetail[]> {
  const raw = await invoke<string>("analytics_recent_r", { limit });
  return JSON.parse(raw);
}

export interface GroupMetrics {
  key: string;
  label: string;
  trades: number;
  wins: number;
  losses: number;
  win_rate_pct: string;
  net_pnl: string;
  gross_profit: string;
  gross_loss: string;
  profit_factor: string;
  expectancy_r: string;
  avg_win_r: string;
  avg_loss_r: string;
}

export interface Breakdown {
  by_strategy: GroupMetrics[];
  by_month: GroupMetrics[];
  by_weekday: GroupMetrics[];
  by_session: GroupMetrics[];
  plan_adherence_pct: string;
  total_closed: number;
  planned_trades: number;
  tp_hit_count: number;
  sl_hit_count: number;
  breakeven_count: number;
  manual_count: number;
}

export async function getBreakdown(accountId: number, fromDate?: string, toDate?: string): Promise<Breakdown> {
  const raw = await invoke<string>("analytics_breakdown", {
    accountId,
    fromDate: fromDate ?? null,
    toDate: toDate ?? null,
  });
  return JSON.parse(raw);
}

/* ============================================================
 *  Period PnL — daily, weekly, monthly
 * ============================================================ */

export interface PeriodPnl {
  daily_pct: string;
  weekly_pct: string;
  monthly_pct: string;
  daily_pnl: string;
  weekly_pnl: string;
  monthly_pnl: string;
}

export async function getPeriodPnl(accountId: number): Promise<PeriodPnl> {
  return invoke<PeriodPnl>("analytics_period_pnl", { accountId });
}
