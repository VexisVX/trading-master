import { invoke } from "@tauri-apps/api/core";

export async function getSchemaVersion(): Promise<string> {
  return invoke<string>("get_schema_version");
}

export async function getBalancePlaceholder(): Promise<string> {
  return invoke<string>("get_balance_placeholder");
}
