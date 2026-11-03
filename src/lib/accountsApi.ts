import { invoke } from "@tauri-apps/api/core";

export type AccountKind = "personal" | "prop_firm";

export interface Account {
  id: number;
  name: string;
  currency: string;
  initial_balance: string;
  is_active: boolean;
  created_at: string;
  kind: AccountKind;
}

export interface NewAccount {
  name: string;
  currency: string;
  initial_balance: string;
  kind?: AccountKind;
}

export async function listAccounts(): Promise<Account[]> {
  const [accounts, activeId] = await Promise.all([
    invoke<Account[]>("accounts_list"),
    getActiveAccountId().catch(() => null),
  ]);
  if (activeId === null) return accounts;
  return accounts.map((a) => ({
    ...a,
    is_active: a.id === activeId,
  }));
}

export async function createAccount(account: NewAccount): Promise<number> {
  return invoke<number>("accounts_create", { account });
}

export async function deleteAccount(id: number): Promise<void> {
  return invoke<void>("accounts_delete", { id });
}

export async function getActiveAccountId(): Promise<number> {
  return invoke<number>("accounts_active_get");
}

export async function setActiveAccountId(id: number): Promise<void> {
  await invoke<void>("accounts_active_set", { id });
  if (typeof window !== "undefined") {
    window.dispatchEvent(
      new CustomEvent("activeAccountChanged", { detail: { id } })
    );
  }
}

/* ============================================================
 * Edit / Reset helpers
 * ============================================================ */

export interface UpdateAccountInput {
  name: string;
  currency?: string;
  initial_balance?: string;
  kind?: AccountKind;
}

/** تعداد معاملات این حساب — برای تعیین حالت Edit */
export async function getTradesCount(id: number): Promise<number> {
  return invoke<number>("accounts_trades_count", { id });
}

/** ویرایش — اگر trades_count > 0 فقط name آپدیت می‌شود */
export async function updateAccount(
  id: number,
  update: UpdateAccountInput
): Promise<void> {
  return invoke<void>("accounts_update", { id, update });
}

/** ریست کامل — حذف trades + snapshots + decisions */
export async function resetAccount(id: number): Promise<void> {
  return invoke<void>("accounts_reset", { id });
}

/* ============================================================
 * Backup — export full account to JSON file
 * ============================================================ */

/**
 * Export a full account snapshot (trades, snapshots, settings, goals)
 * to a JSON file in the app's backups folder.
 * Returns the absolute path of the created file.
 */
export async function backupAccountJson(id: number): Promise<string> {
  return invoke<string>("accounts_backup_json", { id });
}
