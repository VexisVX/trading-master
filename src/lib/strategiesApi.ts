import { invoke } from "@tauri-apps/api/core";

export type FieldType = "text" | "textarea" | "select";

export interface StrategyField {
  key: string;
  label: string;
  type: FieldType;
  options?: string[];
  required?: boolean;
}

export interface StrategyTimeframe {
  timeframe: string;
  fields: StrategyField[];
}

export interface Strategy {
  id: number;
  name: string;
  description: string | null;
  version: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  timeframes: StrategyTimeframe[];
}

export interface NewStrategy {
  name: string;
  description: string | null;
  timeframes: StrategyTimeframe[];
}

export async function createStrategy(s: NewStrategy): Promise<number> {
  return invoke<number>("strategies_create", { strategy: s });
}

export async function listStrategies(): Promise<Strategy[]> {
  return invoke<Strategy[]>("strategies_list");
}

export async function getStrategy(id: number): Promise<Strategy | null> {
  return invoke<Strategy | null>("strategies_get", { id });
}

export async function deleteStrategy(id: number): Promise<void> {
  return invoke<void>("strategies_delete", { id });
}

export async function updateStrategy(id: number, s: NewStrategy): Promise<void> {
  return invoke<void>("strategies_update", { id, strategy: s });
}

export async function listVersions(strategyId: number): Promise<[number, number, string, string | null, string][]> {
  return invoke<[number, number, string, string | null, string][]>("strategies_list_versions", { strategyId });
}

export async function getVersionJson(versionId: number): Promise<string | null> {
  return invoke<string | null>("strategies_get_version", { versionId });
}

// ============================================================
// Timeframe categories
// ============================================================
export const TIMEFRAME_GROUPS = {
  htf: { label: "HTF · Long Term", timeframes: ["1W", "1D", "4H", "1H", "15M", "5M"] },
  ltf: { label: "LTF · Short Term", timeframes: ["1H", "15M", "5M", "1M"] },
};

export const ALL_TIMEFRAMES = [
  "1W", "1D", "4H", "1H", "15M", "5M", "1M", "15S",
];

// ============================================================
// Strategy templates — C.I.T and vexis are NOT strategies,
// they are FRAMEWORKS you use inside any strategy.
// ============================================================
export interface StrategyTemplate {
  id: string;
  label: string;
  description: string;
  timeframes: StrategyTimeframe[];
}

export const TEMPLATES: StrategyTemplate[] = [
  {
    id: "ict",
    label: "ICT / SMC",
    description: "Market structure, POIs, liquidity sweeps with DRT/DFR/Time Model",
    timeframes: [
      {
        timeframe: "4H",
        fields: [
          { key: "bias", label: "HTF Bias", type: "select", options: ["Bullish", "Bearish", "Neutral"], required: true },
          { key: "structure", label: "Market Structure", type: "select", options: ["BOS", "CHoCH", "Ranging"], required: true },
          { key: "poi", label: "POI / Order Block", type: "textarea", options: [], required: false },
          { key: "liquidity", label: "Liquidity", type: "textarea", options: [], required: false },
        ],
      },
      {
        timeframe: "1H",
        fields: [
          { key: "structure_1h", label: "Structure", type: "select", options: ["BOS", "CHoCH", "Ranging"], required: false },
          { key: "drt", label: "DRT (Direction / Refinement / Trigger)", type: "select", options: ["DRT Confirmed", "DRT Pending", "No DRT"], required: false },
        ],
      },
      {
        timeframe: "15M",
        fields: [
          { key: "dfr", label: "DFR (Discount / Fair / Premium)", type: "select", options: ["Discount", "Fair", "Premium"], required: false },
          { key: "poi_confirm", label: "POI confirmation", type: "textarea", options: [], required: false },
        ],
      },
      {
        timeframe: "5M",
        fields: [
          { key: "entry_confirm", label: "Entry confirmation", type: "textarea", options: [], required: true },
          { key: "invalidation", label: "Invalidation", type: "textarea", options: [], required: true },
          { key: "time_model", label: "Time Model", type: "select", options: ["London Open", "NY Open", "London Close", "Asia", "Other"], required: false },
        ],
      },
      {
        timeframe: "1M",
        fields: [
          { key: "execution", label: "Execution trigger", type: "textarea", options: [], required: false },
        ],
      },
    ],
  },
  {
    id: "price_action",
    label: "Price Action",
    description: "Candles, patterns, S/R levels",
    timeframes: [
      {
        timeframe: "4H",
        fields: [
          { key: "trend", label: "Trend", type: "select", options: ["Up", "Down", "Range"], required: true },
          { key: "sr_zones", label: "S/R Zones", type: "textarea", options: [], required: false },
        ],
      },
      {
        timeframe: "1H",
        fields: [
          { key: "drt", label: "DRT", type: "select", options: ["DRT Confirmed", "DRT Pending", "No DRT"], required: false },
        ],
      },
      {
        timeframe: "15M",
        fields: [
          { key: "pattern", label: "Candle pattern", type: "select", options: ["Pin bar", "Engulfing", "Inside bar", "Other"], required: false },
          { key: "entry", label: "Entry trigger", type: "textarea", options: [], required: true },
          { key: "dfr", label: "DFR", type: "select", options: ["Discount", "Fair", "Premium"], required: false },
        ],
      },
      {
        timeframe: "5M",
        fields: [
          { key: "time_model", label: "Time Model", type: "select", options: ["London Open", "NY Open", "London Close", "Asia", "Other"], required: false },
        ],
      },
    ],
  },
  {
    id: "supply_demand",
    label: "Supply & Demand",
    description: "Zones, imbalance, order flow",
    timeframes: [
      {
        timeframe: "4H",
        fields: [
          { key: "zone", label: "Fresh zone", type: "select", options: ["Demand", "Supply", "None"], required: true },
          { key: "imbalance", label: "Imbalance / FVG", type: "textarea", options: [], required: false },
        ],
      },
      {
        timeframe: "1H",
        fields: [
          { key: "drt", label: "DRT", type: "select", options: ["DRT Confirmed", "DRT Pending", "No DRT"], required: false },
        ],
      },
      {
        timeframe: "15M",
        fields: [
          { key: "entry", label: "Entry at zone", type: "textarea", options: [], required: true },
          { key: "invalidation", label: "Invalidation", type: "textarea", options: [], required: false },
          { key: "dfr", label: "DFR", type: "select", options: ["Discount", "Fair", "Premium"], required: false },
        ],
      },
      {
        timeframe: "5M",
        fields: [
          { key: "time_model", label: "Time Model", type: "select", options: ["London Open", "NY Open", "London Close", "Asia", "Other"], required: false },
        ],
      },
    ],
  },
  {
    id: "indicator",
    label: "Indicator-Based",
    description: "Moving averages, RSI, MACD",
    timeframes: [
      {
        timeframe: "1H",
        fields: [
          { key: "ma_cross", label: "MA cross", type: "select", options: ["Above", "Below", "None"], required: false },
          { key: "rsi", label: "RSI level", type: "text", options: [], required: false },
        ],
      },
      {
        timeframe: "15M",
        fields: [
          { key: "macd", label: "MACD signal", type: "select", options: ["Bullish", "Bearish", "Flat"], required: false },
          { key: "entry", label: "Entry confirmation", type: "textarea", options: [], required: true },
        ],
      },
    ],
  },
];
