import { invoke } from "@tauri-apps/api/core";

export interface PropFirmRules {
  account_id: number;
  enabled: boolean;
  max_dd_pct: string;
  trailing_dd: boolean;
  daily_loss_limit_pct: string;
  max_total_loss_pct: string;
  profit_target_pct: string;
  min_trading_days: number;
  updated_at: string;
}

export interface UpdatePropFirmRules {
  enabled: boolean;
  max_dd_pct: string;
  trailing_dd: boolean;
  daily_loss_limit_pct: string;
  max_total_loss_pct: string;
  profit_target_pct: string;
  min_trading_days: number;
}

export interface RuleCheck {
  name: string;
  passed: boolean;
  current: string;
  limit: string;
  message: string;
}

export interface PropFirmStatus {
  rules: PropFirmRules;
  initial_balance: string;
  peak_equity: string;
  current_equity: string;
  total_pnl: string;
  trading_days: number;
  checks: RuleCheck[];
  all_passed: boolean;
  warnings: string[];
}

export async function getPropFirmRules(accountId: number): Promise<PropFirmRules | null> {
  return invoke<PropFirmRules | null>("prop_firm_get", { accountId });
}

export async function updatePropFirmRules(accountId: number, update: UpdatePropFirmRules): Promise<void> {
  return invoke<void>("prop_firm_update", { accountId, update });
}

export async function getPropFirmStatus(accountId: number): Promise<PropFirmStatus> {
  const raw = await invoke<string>("prop_firm_status", { accountId });
  return JSON.parse(raw);
}
