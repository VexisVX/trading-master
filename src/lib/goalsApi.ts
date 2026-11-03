import { invoke } from "@tauri-apps/api/core";

/**
 * Account goals — daily, weekly, monthly target percentages.
 *
 * All values are Decimal strings (never floats).
 * Percentages use 0.0–100.0 range (e.g. "2" = 2%).
 */
export interface AccountGoals {
  account_id: number;
  daily_target_pct: string;
  weekly_target_pct: string;
  monthly_target_pct: string;
  updated_at: string;
}

export interface UpdateGoals {
  daily_target_pct: string;
  weekly_target_pct: string;
  monthly_target_pct: string;
}

export async function getGoals(accountId: number): Promise<AccountGoals | null> {
  return invoke<AccountGoals | null>("goals_get", { accountId });
}

export async function setGoals(
  accountId: number,
  goals: UpdateGoals
): Promise<void> {
  return invoke<void>("goals_set", { accountId, goals });
}
