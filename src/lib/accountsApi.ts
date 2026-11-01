import { invoke } from "@tauri-apps/api/core";

export type AccountKind = "personal" | "prop_firm";

export interface Account {
  id: number;
  name: string;
  currency: string;
  initial_balance: string;
  is_active: boolean;
  kind: AccountKind;
  created_at: string;
}

export interface NewAccount {
  name: string;
  currency: string;
  initial_balance: string;
  kind: AccountKind;
}

export async function listAccounts(): Promise<Account[]> {
  return invoke<Account[]>("accounts_list");
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
  return invoke<void>("accounts_active_set", { id });
}
