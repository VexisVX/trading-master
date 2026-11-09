import { invoke } from "@tauri-apps/api/core";

export interface StorageStats {
  db_size_bytes: number;
  trades_count: number;
  strategies_count: number;
  accounts_count: number;
  attachments_count: number;
  attachments_bytes: number;
}

export async function getStorageStats(): Promise<StorageStats> {
  return invoke<StorageStats>("storage_stats");
}
