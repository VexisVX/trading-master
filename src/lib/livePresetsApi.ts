import { invoke } from "@tauri-apps/api/core";

export interface LivePreset {
  id: number;
  account_id: number;
  name: string;
  instrument: string | null;
  payload_json: string;
  created_at: string;
  updated_at: string;
}

export async function listPresets(accountId: number): Promise<LivePreset[]> {
  return invoke<LivePreset[]>("presets_list", { accountId });
}

export interface PresetPayload {
  name: string;
  instrument: string | null;
  payload_json: string;
}

export async function createPreset(accountId: number, preset: PresetPayload): Promise<number> {
  return invoke<number>("presets_create", { accountId, preset });
}

export async function deletePreset(id: number): Promise<void> {
  return invoke<void>("presets_delete", { id });
}

export interface TradeDraft {
  id: number;
  account_id: number;
  payload_json: string;
  created_at: string;
  updated_at: string;
}

export async function createDraft(accountId: number, payloadJson: string): Promise<number> {
  return invoke<number>("drafts_create", { accountId, payloadJson });
}

export async function getDraft(id: number): Promise<TradeDraft | null> {
  return invoke<TradeDraft | null>("drafts_get", { id });
}

export async function deleteDraft(id: number): Promise<void> {
  return invoke<void>("drafts_delete", { id });
}
