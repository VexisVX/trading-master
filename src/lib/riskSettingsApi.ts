import { invoke } from "@tauri-apps/api/core";

// Decimal values are strings.

export interface RiskSettings {
  account_id: number;
  max_risk_per_trade_pct: string;
  daily_loss_limit_pct: string;
  max_open_positions: number;
  max_portfolio_risk_pct: string;
  updated_at: string;
}

export interface UpdateRiskSettings {
  max_risk_per_trade_pct: string;
  daily_loss_limit_pct: string;
  max_open_positions: number;
  max_portfolio_risk_pct: string;
}

export interface RiskStatus {
  settings: RiskSettings;
  current_equity: string;
  open_positions: number;
  open_risk: string;
  open_risk_pct: string;
  daily_loss_today: string;
  daily_loss_pct: string;
  risk_budget_used_pct: string;
  daily_loss_budget_used_pct: string;
  max_positions_hit: boolean;
  daily_loss_limit_hit: boolean;
  portfolio_risk_exceeded: boolean;
  warnings: string[];
}

export async function getRiskSettings(accountId: number): Promise<RiskSettings | null> {
  return invoke<RiskSettings | null>("risk_settings_get", { accountId });
}

export async function updateRiskSettings(accountId: number, update: UpdateRiskSettings): Promise<void> {
  return invoke<void>("risk_settings_update", { accountId, update });
}

export async function getRiskStatus(accountId: number): Promise<RiskStatus> {
  const raw = await invoke<string>("risk_status", { accountId });
  return JSON.parse(raw);
}
