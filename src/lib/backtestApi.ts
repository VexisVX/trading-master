// Backtest API — schema v2 (flat trades, no sessions).
// Rules (ARCHITECTURE.md):
//   - All price fields are STRINGS on both sides (Decimal-safe).
//   - r_multiple and ALL stats are computed server-side in Rust (Decimal).
//   - No `any`. Side/Outcome are typed unions; setup_tag is free text.

import { invoke } from "@tauri-apps/api/core";

// ---------- Core types ----------

export type Side = "long" | "short";
export type Outcome = "tp" | "sl" | "be";

export interface BacktestTrade {
  id: number;
  symbol: string;
  side: Side;
  entry_price: string;
  stop_price: string;
  take_profit: string | null;
  exit_price: string | null;
  outcome: Outcome | null;
  /** Computed in Rust (Decimal). Read-only from frontend. */
  r_multiple: string | null;
  /** Free-text. Optional. */
  setup_tag: string | null;
  opened_at: string;
  closed_at: string | null;
  notes: string | null;
}

// ---------- Filters ----------

export interface BacktestFilter {
  symbol?: string | null;
  setup_tag?: string | null;
  outcome?: Outcome | null;
  date_from?: string | null; // "YYYY-MM-DD" inclusive
  date_to?: string | null;   // "YYYY-MM-DD" inclusive
}

/** Strip nullish/empty fields so Rust filter stays clean. */
function cleanFilter(f: BacktestFilter | undefined): BacktestFilter | null {
  if (!f) return null;
  const out: BacktestFilter = {};
  if (f.symbol) out.symbol = f.symbol;
  if (f.setup_tag) out.setup_tag = f.setup_tag;
  if (f.outcome) out.outcome = f.outcome;
  if (f.date_from) out.date_from = f.date_from;
  if (f.date_to) out.date_to = f.date_to;
  return Object.keys(out).length > 0 ? out : null;
}

// ---------- Stats types (all Decimal fields as strings) ----------

export interface EquityPoint {
  i: number;
  r: string; // cumulative, Decimal string
}

export interface HistBucket {
  label: string;
  count: number;
}

export interface SetupStats {
  tag: string;
  count: number;
  avg_r: string;
  win_rate: string;
}

export interface BacktestStats {
  count: number;
  wins: number;
  losses: number;
  breakeven: number;
  win_rate: string;       // "0.52"
  expectancy: string;     // "0.18" (R)
  profit_factor: string;  // "1.42" | "inf"
  total_r: string;        // "23.5" (R)
  equity: EquityPoint[];
  histogram: HistBucket[];
  by_setup: SetupStats[];
}

export interface SetupTagCount {
  tag: string;
  count: number;
}

// ---------- Trade input ----------

export interface NewTradeInput {
  symbol: string;
  side: Side;
  entry_price: string;
  stop_price: string;
  take_profit?: string | null;
  exit_price?: string | null;
  /** User-provided R. XOR with exit_price (Rust rejects both). */
  manual_r?: string | null;
  setup_tag?: string | null;
  opened_at?: string | null;
  closed_at?: string | null;
  notes?: string | null;
  // NOTE: no r_multiple, no outcome — Rust derives both from exit_price.
}

// ---------- Commands ----------

export function listTrades(filter?: BacktestFilter): Promise<BacktestTrade[]> {
  return invoke<BacktestTrade[]>("backtest_trades_list", {
    filter: cleanFilter(filter),
  });
}

export function createTrade(t: NewTradeInput): Promise<number> {
  return invoke<number>("backtest_trade_create", {
    symbol: t.symbol,
    side: t.side,
    entryPrice: t.entry_price,
    stopPrice: t.stop_price,
    takeProfit: t.take_profit ?? null,
    exitPrice: t.exit_price ?? null,
    manualR: t.manual_r ?? null,
    setupTag: t.setup_tag ?? null,
    openedAt: t.opened_at ?? null,
    closedAt: t.closed_at ?? null,
    notes: t.notes ?? null,
  });
}

export function deleteTrade(id: number): Promise<void> {
  return invoke<void>("backtest_trade_delete", { id });
}

export function resetBacktest(): Promise<void> {
  return invoke<void>("backtest_reset");
}

export function listSetupTags(): Promise<SetupTagCount[]> {
  return invoke<SetupTagCount[]>("backtest_setup_tags");
}

export function listSymbols(): Promise<string[]> {
  return invoke<string[]>("backtest_symbols");
}

export function getStats(filter?: BacktestFilter): Promise<BacktestStats> {
  return invoke<BacktestStats>("backtest_stats", {
    filter: cleanFilter(filter),
  });
}

// ---------- Display helpers (no financial math — display only) ----------

/** Parse a Decimal-string R safely. For display only; never feed back to Rust. */
export function parseR(r: string | null | undefined): number | null {
  if (r === null || r === undefined || r === "") return null;
  const n = Number(r);
  return Number.isFinite(n) ? n : null;
}

/** "+1.42" / "-0.55" / "—" */
export function formatR(r: string | null | undefined): string {
  const n = parseR(r);
  if (n === null) return "—";
  return (n >= 0 ? "+" : "") + n.toFixed(2);
}

/** "52.3%" from a "0.523" Decimal string. */
export function formatPct(s: string | null | undefined): string {
  const n = parseR(s ?? null);
  if (n === null) return "—";
  return (n * 100).toFixed(1) + "%";
}

/** "inf" stays "∞". */
export function formatPF(s: string | null | undefined): string {
  if (!s) return "—";
  if (s === "inf") return "∞";
  const n = Number(s);
  return Number.isFinite(n) ? n.toFixed(2) : "—";
}

/** ISO date (YYYY-MM-DD) → local "MM/DD HH:mm" (short). */
export function formatDateShort(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `${mm}/${dd} ${hh}:${mi}`;
}

/** Today as YYYY-MM-DD (local). */
export function todayISO(): string {
  const d = new Date();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${mm}-${dd}`;
}
