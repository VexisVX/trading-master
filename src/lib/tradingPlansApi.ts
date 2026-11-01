import { invoke } from "@tauri-apps/api/core";

export interface TradingPlan {
  id: number;
  strategy_id: number;
  name: string;
  checklist_json: string;
  notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface PlanInput {
  strategy_id: number;
  name: string;
  checklist_json: string;
  notes: string | null;
}

export async function listPlans(strategyId: number): Promise<TradingPlan[]> {
  return invoke<TradingPlan[]>("plans_list", { strategyId });
}

export async function createPlan(input: PlanInput): Promise<number> {
  return invoke<number>("plans_create", { input });
}

export async function updatePlan(
  id: number, name: string, checklistJson: string, notes: string | null,
): Promise<void> {
  return invoke<void>("plans_update", { id, name, checklistJson, notes });
}

export async function deletePlan(id: number): Promise<void> {
  return invoke<void>("plans_delete", { id });
}
