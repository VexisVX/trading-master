import { invoke } from "@tauri-apps/api/core";

// All Decimal values are STRINGS (serde-with-str on Rust side).
// Never do arithmetic on these in JS.

export type Side = "long" | "short";
export type TradeStatus = "open" | "closed";
export type LegKind = "entry" | "exit";
export type TradeOutcome = "open" | "tp_hit" | "sl_hit" | "manual_close" | "breakeven";
export type SizeType = "lots" | "units" | "money";

export interface TradeLeg {
  id: number;
  kind: LegKind;
  price: string;
  quantity: string;
  executed_at: string;
  cost_per_unit: string;
}

export interface Attachment {
  id: number;
  filename: string;
  kind: string;
  created_at: string;
}

export interface Trade {
  id: number;
  account_id: number;
  symbol: string;
  side: Side;
  entry_price: string;
  initial_stop: string;
  take_profit: string | null;
  total_quantity: string;
  original_quantity: string;
  outcome: TradeOutcome;
  size_type: SizeType;
  size_value: string | null;
  size_unit: string | null;
  status: TradeStatus;
  strategy_id: number | null;
  opened_at: string;
  closed_at: string | null;
  notes: string | null;
  chart_url: string | null;
  screenshot_path: string | null;
  manual_pnl: string | null;
  created_at: string;
  updated_at: string;
  legs: TradeLeg[];
  tags: string[];
  attachments: Attachment[];
}

export interface NewLeg {
  price: string;
  quantity: string;
  cost_per_unit: string;
}

export interface NewTrade {
  account_id: number;
  symbol: string;
  side: Side;
  entry_price: string;
  initial_stop: string;
  take_profit: string | null;
  total_quantity: string;
  size_type: SizeType;
  size_value: string | null;
  size_unit: string | null;
  opened_at: string;
  notes: string | null;
  chart_url: string | null;
  screenshot_path: string | null;
  strategy_id: number | null;
  entry_legs: NewLeg[];
  tags: string[];
}

export interface UpdateTrade {
  symbol: string;
  entry_price: string;
  initial_stop: string;
  take_profit: string | null;
  total_quantity: string;
  outcome: TradeOutcome;
  size_type: SizeType;
  size_value: string | null;
  size_unit: string | null;
  strategy_id: number | null;
  notes: string | null;
  chart_url: string | null;
  screenshot_path: string | null;
  manual_pnl: string | null;
  tags: string[];
}

export async function createTrade(trade: NewTrade): Promise<number> {
  return invoke<number>("trades_create", { trade });
}

export async function listTrades(limit = 100, offset = 0): Promise<Trade[]> {
  return invoke<Trade[]>("trades_list", { limit, offset });
}

export async function getTrade(id: number): Promise<Trade | null> {
  return invoke<Trade | null>("trades_get", { id });
}

export async function addExit(
  tradeId: number, price: string, quantity: string,
  costPerUnit: string, executedAt: string,
  outcome?: TradeOutcome,
  manualPnl?: string | null,
): Promise<void> {
  return invoke<void>("trades_add_exit", {
    tradeId, price, quantity, costPerUnit, executedAt,
    outcome: outcome ?? null,
    manualPnl: manualPnl ?? null,
  });
}

export async function getDefaultAccountId(): Promise<number> {
  return invoke<number>("trades_default_account_id");
}

export async function updateTrade(id: number, update: UpdateTrade): Promise<void> {
  return invoke<void>("trades_update", { id, update });
}

export async function deleteTrade(id: number): Promise<void> {
  return invoke<void>("trades_delete", { id });
}

export async function exportTradesCsv(dest: string): Promise<number> {
  return invoke<number>("trades_export_csv", { dest });
}

export async function addAttachment(tradeId: number, filename: string, kind: string): Promise<number> {
  return invoke<number>("trades_add_attachment", { tradeId, filename, kind });
}

export async function removeAttachment(id: number): Promise<string> {
  return invoke<string>("trades_remove_attachment", { id });
}

// Display-only helpers — authoritative math lives in Rust.
export function computePlannedRR(t: Pick<Trade, "entry_price" | "initial_stop" | "take_profit">): number | null {
  if (!t.take_profit) return null;
  const entry = Number(t.entry_price);
  const stop = Number(t.initial_stop);
  const tp = Number(t.take_profit);
  if (!Number.isFinite(entry) || !Number.isFinite(stop) || !Number.isFinite(tp)) return null;
  const risk = Math.abs(entry - stop);
  if (risk === 0) return null;
  const reward = Math.abs(tp - entry);
  return reward / risk;
}

export function computeRealizedR(t: Trade): number | null {
  if (t.status !== "closed") return null;
  const entry = Number(t.entry_price);
  const stop = Number(t.initial_stop);
  const qty = Number(t.total_quantity);
  if (!Number.isFinite(entry) || !Number.isFinite(stop) || qty === 0) return null;
  const riskPerUnit = Math.abs(entry - stop);
  if (riskPerUnit === 0) return null;
  const initialRisk = riskPerUnit * qty;

  if (t.manual_pnl != null) {
    const manual = Number(t.manual_pnl);
    if (!Number.isFinite(manual)) return null;
    return manual / initialRisk;
  }

  const exits = t.legs.filter((l) => l.kind === "exit");
  if (exits.length === 0) return null;
  let gross = 0, cost = 0;
  for (const ex of exits) {
    const diff = t.side === "long" ? Number(ex.price) - entry : entry - Number(ex.price);
    gross += diff * Number(ex.quantity);
    cost += Number(ex.cost_per_unit) * Number(ex.quantity);
  }
  return (gross - cost) / initialRisk;
}

export function computePnl(t: Trade): number | null {
  if (t.status !== "closed") return null;
  if (t.manual_pnl != null) {
    const n = Number(t.manual_pnl);
    return Number.isFinite(n) ? n : null;
  }
  const entry = Number(t.entry_price);
  const exits = t.legs.filter((l) => l.kind === "exit");
  if (exits.length === 0 || !Number.isFinite(entry)) return null;
  let gross = 0, cost = 0;
  for (const ex of exits) {
    const diff = t.side === "long" ? Number(ex.price) - entry : entry - Number(ex.price);
    gross += diff * Number(ex.quantity);
    cost += Number(ex.cost_per_unit) * Number(ex.quantity);
  }
  return gross - cost;
}

export async function partialClose(
  tradeId: number,
  exitPrice: string,
  exitQuantity: string,
  costPerUnit: string,
  executedAt: string,
  manualPnl?: string | null,
): Promise<void> {
  return invoke<void>("trades_partial_close", {
    tradeId, exitPrice, exitQuantity, costPerUnit, executedAt,
    manualPnl: manualPnl ?? null,
  });
}

export async function logSlMove(
  tradeId: number, oldStop: string, newStop: string, reason: string | null,
): Promise<void> {
  return invoke<void>("trades_log_sl_move", { tradeId, oldStop, newStop, reason });
}

export interface SlMove {
  old_stop: string;
  new_stop: string;
  reason: string | null;
  changed_at: string;
}

export async function getSlHistory(tradeId: number): Promise<[string, string, string | null, string][]> {
  return invoke<[string, string, string | null, string][]>("trades_sl_history", { tradeId });
}

export async function copyScreenshot(tradeId: number, sourcePath: string): Promise<string> {
  return invoke<string>("trades_copy_screenshot", { tradeId, sourcePath });
}

export interface QuickTradeInput {
  account_id: number;
  symbol: string;
  outcome: TradeOutcome;
  r_value?: string | null;
  session?: string | null;
  time_of_day?: string | null;
  entry_price?: string | null;
  stop_price?: string | null;
  take_profit?: string | null;
  quantity?: string | null;
}

export async function quickCreateTrade(input: QuickTradeInput): Promise<number> {
  return invoke<number>("trades_quick_create", {
    accountId: input.account_id,
    symbol: input.symbol,
    outcome: input.outcome,
    rValue: input.r_value ?? null,
    session: input.session ?? null,
    timeOfDay: input.time_of_day ?? null,
    entryPrice: input.entry_price ?? null,
    stopPrice: input.stop_price ?? null,
    takeProfit: input.take_profit ?? null,
    quantity: input.quantity ?? null,
  });
}
