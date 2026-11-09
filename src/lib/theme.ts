// Theme + preferences manager
// All setters apply immediately to the DOM and persist to app_settings.

export type Theme = "dark" | "light" | "cream" | "dim" | "amoled";
export type AccentId = "turquoise" | "teal" | "cyan" | "blue" | "indigo" | "purple" | "pink" | "emerald" | "amber";
export type RadiusId = "square" | "soft" | "round";
export type DateFormat = "iso" | "dmy" | "mdy";

export const ACCENTS: { id: AccentId; color: string; label: string }[] = [
  { id: "turquoise", color: "#2dd4bf", label: "Turquoise" },
  { id: "teal",      color: "#14b8a6", label: "Teal" },
  { id: "cyan",      color: "#22d3ee", label: "Cyan" },
  { id: "blue",      color: "#60a5fa", label: "Blue" },
  { id: "indigo",    color: "#818cf8", label: "Indigo" },
  { id: "purple",    color: "#a78bfa", label: "Purple" },
  { id: "pink",      color: "#f472b6", label: "Pink" },
  { id: "emerald",   color: "#34d399", label: "Emerald" },
  { id: "amber",     color: "#fbbf24", label: "Amber" },
];

export const RADIUS_VALUES: Record<RadiusId, { sm: string; md: string; lg: string }> = {
  square: { sm: "0px", md: "0px", lg: "0px" },
  soft:   { sm: "6px", md: "10px", lg: "14px" },
  round:  { sm: "10px", md: "16px", lg: "22px" },
};

// ============================================================
// Theme color palettes
// ============================================================
const THEMES: Record<Theme, Record<string, string>> = {
  dark: {
    "--bg-0": "#0b0f14", "--bg-1": "#11161d", "--bg-2": "#161d26",
    "--bg-3": "#1d2530", "--bg-4": "#26313e",
    "--border": "#232c38", "--border-strong": "#2f3a48",
    "--text-0": "#e6edf3", "--text-1": "#9aa7b4",
    "--text-2": "#6b7886", "--text-3": "#4b5663",
  },
  light: {
    "--bg-0": "#f8fafc", "--bg-1": "#ffffff", "--bg-2": "#f1f5f9",
    "--bg-3": "#e2e8f0", "--bg-4": "#cbd5e1",
    "--border": "#e2e8f0", "--border-strong": "#cbd5e1",
    "--text-0": "#0f172a", "--text-1": "#475569",
    "--text-2": "#64748b", "--text-3": "#94a3b8",
  },
  cream: {
    "--bg-0": "#f5f0e6", "--bg-1": "#faf6ed", "--bg-2": "#efe7d8",
    "--bg-3": "#e5dbc8", "--bg-4": "#d8cbaf",
    "--border": "#e0d6c2", "--border-strong": "#c8bca1",
    "--text-0": "#3a2f20", "--text-1": "#6b5d47",
    "--text-2": "#8d7f68", "--text-3": "#a89a82",
  },
  dim: {
    "--bg-0": "#1a1f26", "--bg-1": "#222831", "--bg-2": "#2b323d",
    "--bg-3": "#343c49", "--bg-4": "#3f4855",
    "--border": "#343c49", "--border-strong": "#4b5563",
    "--text-0": "#e6edf3", "--text-1": "#b8c0cc",
    "--text-2": "#8792a1", "--text-3": "#6b7785",
  },
  amoled: {
    "--bg-0": "#000000", "--bg-1": "#050505", "--bg-2": "#0a0a0a",
    "--bg-3": "#101010", "--bg-4": "#181818",
    "--border": "#1a1a1a", "--border-strong": "#2a2a2a",
    "--text-0": "#ffffff", "--text-1": "#b0b0b0",
    "--text-2": "#808080", "--text-3": "#505050",
  },
};

function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace("#", "");
  const r = parseInt(h.substring(0, 2), 16);
  const g = parseInt(h.substring(2, 4), 16);
  const b = parseInt(h.substring(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function applyTheme(theme: Theme) {
  const vars = THEMES[theme];
  const root = document.documentElement;
  for (const [k, v] of Object.entries(vars)) {
    root.style.setProperty(k, v);
  }
  root.dataset.theme = theme;
  // color-scheme for native controls
  root.style.colorScheme = theme === "light" ? "light" : "dark";
}

export function applyAccent(accentId: AccentId) {
  const found = ACCENTS.find((a) => a.id === accentId);
  if (!found) return;
  const root = document.documentElement;
  root.style.setProperty("--accent", found.color);
  root.style.setProperty("--accent-dim", found.color);
  root.style.setProperty("--accent-soft", hexToRgba(found.color, 0.12));
  root.style.setProperty("--accent-softer", hexToRgba(found.color, 0.06));
}

export function applyRadius(radius: RadiusId) {
  const v = RADIUS_VALUES[radius];
  const root = document.documentElement;
  root.style.setProperty("--radius-sm", v.sm);
  root.style.setProperty("--radius", v.md);
  root.style.setProperty("--radius-lg", v.lg);
}

export function applyFontSize(size: "small" | "medium" | "large") {
  const root = document.documentElement;
  let scale: string;
  let md: string, sm: string, lg: string, xl: string, xxl: string;

  if (size === "small") {
    scale = "0.875";   // ~12.25px base
    md = "11.5px"; sm = "10.5px"; lg = "13px"; xl = "15.5px"; xxl = "19px";
  } else if (size === "large") {
    scale = "1.15";    // ~16.1px base
    md = "15.5px"; sm = "14.5px"; lg = "17.5px"; xl = "20.5px"; xxl = "25px";
  } else {
    scale = "1";       // 14px base
    md = "13.5px"; sm = "12.5px"; lg = "15px"; xl = "18px"; xxl = "22px";
  }

  root.style.setProperty("--fs-scale", scale);
  root.style.setProperty("--fs-md", md);
  root.style.setProperty("--fs-sm", sm);
  root.style.setProperty("--fs-lg", lg);
  root.style.setProperty("--fs-xl", xl);
  root.style.setProperty("--fs-2xl", xxl);

  // Force reflow — this actually makes the change visible
  void root.offsetWidth;
}

export function applyMotion(m: "full" | "reduced" | "off") {
  const root = document.documentElement;
  // Remove any previous override class
  root.classList.remove("motion-off", "motion-reduced");
  root.style.removeProperty("--dur-fast");
  root.style.removeProperty("--dur-med");
  root.style.removeProperty("--dur-slow");

  if (m === "off") {
    root.classList.add("motion-off");
    root.style.setProperty("--dur-fast", "0ms");
    root.style.setProperty("--dur-med", "0ms");
    root.style.setProperty("--dur-slow", "0ms");
  } else if (m === "reduced") {
    root.classList.add("motion-reduced");
    root.style.setProperty("--dur-fast", "80ms");
    root.style.setProperty("--dur-med", "120ms");
    root.style.setProperty("--dur-slow", "180ms");
  } else {
    root.style.setProperty("--dur-fast", "140ms");
    root.style.setProperty("--dur-med", "220ms");
    root.style.setProperty("--dur-slow", "320ms");
  }
}

// ============================================================
// Date formatting
// ============================================================
export function formatDate(iso: string, format: DateFormat): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;

  const yyyy = d.getFullYear();
  const mm = String(d.getMonth() + 1).padStart(2, "0");
  const dd = String(d.getDate()).padStart(2, "0");

  if (format === "dmy") return `${dd}/${mm}/${yyyy}`;
  if (format === "mdy") return `${mm}/${dd}/${yyyy}`;
  return `${yyyy}-${mm}-${dd}`;
}

export function formatDateTime(iso: string, format: DateFormat): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (isNaN(d.getTime())) return iso;
  const dateStr = formatDate(iso, format);
  const hh = String(d.getHours()).padStart(2, "0");
  const mi = String(d.getMinutes()).padStart(2, "0");
  return `${dateStr} ${hh}:${mi}`;
}
