import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";

export interface BackupResult {
  path: string;
  size_bytes: number;
  created_at: string;
}

export async function backupNow(): Promise<BackupResult> {
  return invoke<BackupResult>("backup_now");
}

export async function backupToFileInteractive(): Promise<BackupResult | null> {
  const dest = await save({
    title: "Save backup",
    defaultPath: `trading-manager-${new Date().toISOString().slice(0, 10)}.db`,
    filters: [{ name: "SQLite database", extensions: ["db"] }],
  });
  if (!dest) return null;
  return invoke<BackupResult>("backup_to_file", { dest });
}

export async function restoreInteractive(): Promise<string | null> {
  const file = await open({
    title: "Choose backup to restore",
    multiple: false,
    filters: [{ name: "SQLite database", extensions: ["db"] }],
  });
  if (!file || Array.isArray(file)) return null;

  // Validate first
  await invoke<void>("backup_validate", { candidate: file });

  const confirmed = confirm(
    "Restore will replace your current database on disk. The app will need to restart.\n\nContinue?"
  );
  if (!confirmed) return null;

  const path = await invoke<string>("backup_restore", { candidate: file });
  return path;
}

export async function getBackupDir(): Promise<string> {
  return invoke<string>("backup_dir");
}
