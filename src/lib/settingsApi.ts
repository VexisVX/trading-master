import { invoke } from "@tauri-apps/api/core";

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>("settings_get", { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>("settings_set", { key, value });
}

export async function deleteSetting(key: string): Promise<void> {
  return invoke<void>("settings_delete", { key });
}
