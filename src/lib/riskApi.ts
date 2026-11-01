import { invoke } from "@tauri-apps/api/core";

// All Decimal values are strings (serde-with-str on Rust side).
// Never convert these to JS number for arithmetic — display only.

export interface PositionSizeInput {
  account_balance: string;
  risk_fraction: string;
  entry_price: string;
  stop_price: string;
  tick_size: string;
  tick_value: string;
  contract_size: string;
  commission_per_lot: string;
}

export interface PositionSizeResult {
  risk_amount: string;
  stop_distance: string;
  stop_ticks: string;
  loss_per_lot: string;
  lots: string;
  actual_risk: string;
}

export async function computePositionSize(input: PositionSizeInput): Promise<PositionSizeResult> {
  const raw = await invoke<string>("risk_position_size", { input });
  return JSON.parse(raw);
}

export interface ExitInput { price: string; quantity: string; }
export interface TradeRInput {
  total_quantity: string;
  entry_price: string;
  initial_stop: string;
  is_long: boolean;
  cost_per_unit: string;
  exits: ExitInput[];
}

export interface RResult {
  gross_pnl: string;
  total_costs: string;
  net_pnl: string;
  initial_risk: string;
  r_multiple: string;
}

export async function computeR(trade: TradeRInput): Promise<RResult> {
  const raw = await invoke<string>("risk_r_multiple", { trade });
  return JSON.parse(raw);
}

export async function echoDecimal(s: string): Promise<string> {
  return invoke<string>("risk_echo_decimal", { s });
}
