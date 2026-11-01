import { invoke } from "@tauri-apps/api/core";

export type CalcMode = "simple" | "advanced";

export type RiskMode = "pct" | "fixed";

export interface LiveCalcInput {
  mode: CalcMode;
  size: string;
  entry_price: string;
  stop_price: string;
  take_profit: string | null;
  balance: string;

  contract_size: string | null;
  value_per_point: string | null;
  point_size: string | null;
  leverage: string | null;

  margin_usd: string | null;
  value_per_price_unit: string | null;

  commission_per_lot: string | null;
  spread_points: string | null;

  min_quantity: string | null;
  max_quantity: string | null;
  quantity_step: string | null;

  max_risk_per_trade_pct: string | null;
  risk_mode: RiskMode | null;
  fixed_risk_amount: string | null;
}

export interface LiveCalcResult {
  stop_distance_price: string;
  risk_per_lot: string;
  total_risk: string;
  risk_pct: string;
  commission_total: string;
  spread_cost: string;
  total_costs: string;
  net_risk: string;
  net_risk_pct: string;
  reward_per_lot: string | null;
  total_reward: string | null;
  net_reward: string | null;
  planned_rr: string | null;
  notional: string;
  required_margin: string;
  free_margin_after: string;
  margin_used_pct: string;
  warnings: string[];
  verdict: "ok" | "warn" | "danger";
}

export async function calculateLiveRisk(input: LiveCalcInput): Promise<LiveCalcResult> {
  const raw = await invoke<string>("live_risk_calculate", { input });
  return JSON.parse(raw);
}

// ---------- Lab ----------

export interface PartialCloseInput {
  size: string; entry_price: string; current_price: string;
  value_per_price_unit: string; close_fraction: string;
}
export interface PartialCloseResult {
  locked_profit: string; remaining_size: string;
  remaining_potential_per_1unit: string; be_price_after: string;
}
export async function calcPartialClose(i: PartialCloseInput): Promise<PartialCloseResult> {
  const raw = await invoke<string>("live_partial_close", { input: i });
  return JSON.parse(raw);
}

export interface MartingaleCheckInput {
  current_size: string;
  last_losing_size: string | null;
  last_losing_was_recent: boolean;
}
export interface MartingaleCheckResult {
  multiplier: string | null;
  is_risky: boolean;
  verdict: "ok" | "warn" | "danger";
  message: string;
}
export async function calcMartingaleCheck(i: MartingaleCheckInput): Promise<MartingaleCheckResult> {
  const raw = await invoke<string>("live_martingale_check", { input: i });
  return JSON.parse(raw);
}

export interface BreakEvenInput {
  entry_price: string; side: string;
  locked_profit: string; remaining_size: string;
  value_per_price_unit: string;
}
export interface BreakEvenResult {
  be_price: string; move_from_entry: string;
}
export async function calcBreakEven(i: BreakEvenInput): Promise<BreakEvenResult> {
  const raw = await invoke<string>("live_break_even", { input: i });
  return JSON.parse(raw);
}

export interface SizeFromRiskInput {
  balance: string; risk_pct: string;
  entry_price: string; stop_price: string;
  value_per_price_unit: string;
}
export interface SizeFromRiskResult {
  risk_amount: string; size_lots: string; stop_distance: string;
}
export async function calcSizeFromRisk(i: SizeFromRiskInput): Promise<SizeFromRiskResult> {
  const raw = await invoke<string>("live_size_from_risk", { input: i });
  return JSON.parse(raw);
}

// ---------- Assistant ----------

export interface Advice {
  kind: string;
  severity: "info" | "warn" | "danger";
  message: string;
  suggested_size: string | null;
}

export interface AssistantInput {
  size: string;
  entry_price: string;
  stop_price: string;
  take_profit: string | null;
  balance: string;
  value_per_price_unit: string;
  max_risk_pct: string;
}

export interface AssistantResult {
  verdict: "ok" | "warn" | "danger";
  headline: string;
  current_risk_pct: string;
  suggested_size: string | null;
  advices: Advice[];
}

export async function getAssistant(input: AssistantInput): Promise<AssistantResult> {
  const raw = await invoke<string>("live_assistant", { input });
  return JSON.parse(raw);
}

// ---------- Portfolio ----------

export interface PortfolioTradeInput {
  symbol: string;
  side: "long" | "short";
  size: string;
  entry_price: string;
  stop_price: string;
  value_per_price_unit: string;
}

export interface PortfolioRiskInput {
  trades: PortfolioTradeInput[];
  balance: string;
  max_portfolio_risk_pct: string;
  max_open_positions: number;
}

export interface PortfolioTradeResult {
  symbol: string;
  side: string;
  size: string;
  risk: string;
}

export interface PortfolioRiskResult {
  positions: number;
  total_risk: string;
  total_risk_pct: string;
  per_trade: PortfolioTradeResult[];
  positions_limit_hit: boolean;
  portfolio_limit_hit: boolean;
  warnings: string[];
  verdict: "ok" | "warn" | "danger";
  advices: string[];
  suggested_trim_pct: string | null;
}

export async function getPortfolioRisk(input: PortfolioRiskInput): Promise<PortfolioRiskResult> {
  const raw = await invoke<string>("live_portfolio_risk", { input });
  return JSON.parse(raw);
}
