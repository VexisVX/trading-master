import { invoke } from "@tauri-apps/api/core";

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

export async function updateGoals(accountId: number, update: UpdateGoals): Promise<void> {
  return invoke<void>("goals_update", { accountId, update });
}
