// Export HTML builder — separated from TradingPlan.tsx to avoid
// template-literal conflicts.

export interface ChecklistItem {
  id: string;
  text: string;
  category: "htf" | "drt" | "dfr" | "time_model" | "entry" | "other";
}

export type ExportStyle = "journal" | "ui" | "hybrid";
export type ExportTheme = "light" | "dark";

interface Options {
  planName: string;
  strategyName: string;
  items: ChecklistItem[];
  checked: Set<string>;
  style: ExportStyle;
  theme: ExportTheme;
}

function escapeHtml(s: string): string {
  return s.replace(/[&<>"']/g, (c) => {
    const map: Record<string, string> = {
      "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
    };
    return map[c] || c;
  });
}

const GROUP_TITLES: Record<string, string> = {
  htf: "HTF / Long Term",
  drt: "DRT",
  dfr: "DFR",
  time_model: "Time Model",
  entry: "Entry / LTF",
  other: "Other",
};

function buildItems(items: ChecklistItem[], checked: Set<string>): string {
  const groups: Array<ChecklistItem["category"]> = ["htf", "drt", "dfr", "time_model", "entry", "other"];
  return groups.map((cat) => {
    const catItems = items.filter((i) => i.category === cat);
    if (catItems.length === 0) return "";
    const itemsHtml = catItems.map((it) => {
      const isChecked = checked.has(it.id);
      return (
        '<div class="item ' + (isChecked ? "is-checked" : "") + '">' +
          '<span class="check">' + (isChecked ? "✓" : "") + "</span>" +
          '<span class="text">' + escapeHtml(it.text || "") + "</span>" +
        "</div>"
      );
    }).join("");
    return (
      '<div class="group">' +
        '<div class="group-title">' + GROUP_TITLES[cat] + "</div>" +
        itemsHtml +
      "</div>"
    );
  }).join("");
}

// ============================================================
// JOURNAL STYLE
// ============================================================
function buildJournal(opts: Options): string {
  const { planName, strategyName, items, checked, theme } = opts;
  const dark = theme === "dark";

  const bg = dark ? "#1a1f26" : "#fdfcfb";
  const bgOuter = dark ? "#0b0f14" : "#e8e3d8";
  const fg = dark ? "#e6edf3" : "#1a1a1a";
  const fgMuted = dark ? "#9aa7b4" : "#555";
  const fgDim = dark ? "#6b7886" : "#999";
  const border = dark ? "#2f3a48" : "#d8d0c0";
  const accent = dark ? "#2dd4bf" : "#0d9488";
  const accentBg = dark ? "rgba(45,212,191,0.08)" : "rgba(13,148,136,0.06)";
  const notesBg = dark ? "#161d26" : "#fafaf7";

  const itemsHtml = buildItems(items, checked);

  return "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n" +
    "<title>" + escapeHtml(planName || "Trading Plan") + "</title>\n<style>\n" +
    "* { box-sizing: border-box; margin: 0; padding: 0; }\n" +
    "body { font-family: Georgia, 'Times New Roman', serif; background: " + bgOuter + "; color: " + fg + "; padding: 32px 16px; line-height: 1.6; }\n" +
    ".page { max-width: 820px; margin: 0 auto; background: " + bg + "; padding: 56px 64px; box-shadow: 0 2px 24px rgba(0,0,0,0.15); }\n" +
    ".header { text-align: center; margin-bottom: 28px; padding-bottom: 20px; border-bottom: 2px solid " + accent + "; }\n" +
    ".title { font-size: 32px; font-weight: 400; letter-spacing: 4px; color: " + fg + "; text-transform: uppercase; }\n" +
    ".subtitle { font-size: 12px; font-style: italic; color: " + fgMuted + "; margin-top: 6px; letter-spacing: 1px; }\n" +
    ".fields { display: grid; grid-template-columns: 1fr 1fr; gap: 18px 32px; margin-bottom: 28px; }\n" +
    ".field { display: flex; flex-direction: column; gap: 4px; }\n" +
    ".field-label { font-size: 10px; text-transform: uppercase; letter-spacing: 1.5px; color: " + fgDim + "; }\n" +
    ".field-line { border-bottom: 1px solid " + border + "; height: 24px; display: flex; align-items: flex-end; padding-bottom: 2px; font-size: 13px; color: " + fg + "; }\n" +
    ".section { margin-bottom: 24px; }\n" +
    ".section-title { font-size: 11px; text-transform: uppercase; letter-spacing: 2px; color: " + accent + "; margin-bottom: 12px; padding-bottom: 6px; border-bottom: 1px solid " + border + "; }\n" +
    ".group { margin-bottom: 20px; }\n" +
    ".group-title { font-size: 11px; text-transform: uppercase; letter-spacing: 1.2px; color: " + fgMuted + "; margin-bottom: 8px; font-style: italic; }\n" +
    ".item { display: flex; align-items: flex-start; gap: 10px; padding: 5px 0; font-size: 14px; border-bottom: 1px dotted " + border + "; }\n" +
    ".item:last-child { border-bottom: none; }\n" +
    ".item.is-checked .text { text-decoration: line-through; color: " + fgDim + "; }\n" +
    ".check { display: inline-flex; align-items: center; justify-content: center; width: 16px; height: 16px; border: 1.5px solid " + fgDim + "; border-radius: 2px; font-size: 11px; color: " + accent + "; flex-shrink: 0; margin-top: 2px; }\n" +
    ".item.is-checked .check { background: " + accentBg + "; border-color: " + accent + "; }\n" +
    ".notes-area { min-height: 180px; border: 1px solid " + border + "; border-radius: 3px; padding: 12px; background: " + notesBg + "; background-image: repeating-linear-gradient(" + notesBg + " 0px, " + notesBg + " 27px, " + border + " 27px, " + border + " 28px); }\n" +
    ".notes-area.small { min-height: 100px; }\n" +
    ".footer { margin-top: 32px; padding-top: 16px; border-top: 1px solid " + border + "; text-align: center; font-size: 10px; font-style: italic; color: " + fgDim + "; }\n" +
    "@media print { body { background: white; padding: 0; } .page { box-shadow: none; padding: 40px 50px; } }\n" +
    "</style>\n</head>\n<body>\n<div class=\"page\">\n" +
    "<div class=\"header\"><div class=\"title\">Trading Plan</div><div class=\"subtitle\">Pre-Trade Checklist &amp; Journal</div></div>\n" +
    "<div class=\"fields\">\n" +
    "<div class=\"field\"><span class=\"field-label\">Plan</span><div class=\"field-line\">" + escapeHtml(planName || "") + "</div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Strategy</span><div class=\"field-line\">" + escapeHtml(strategyName) + "</div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Date</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Time</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Asset / Symbol</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Timeframe</span><div class=\"field-line\"></div></div>\n" +
    "</div>\n" +
    "<div class=\"section\"><div class=\"section-title\">Pre-Trade Checklist</div>" + (itemsHtml || "<div class=\"item\">No items</div>") + "</div>\n" +
    "<div class=\"section\"><div class=\"section-title\">Result</div>\n" +
    "<div class=\"fields\" style=\"grid-template-columns: 1fr 1fr 1fr;\">\n" +
    "<div class=\"field\"><span class=\"field-label\">Entry</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Stop</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Target</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">R:R Planned</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">R Realized</span><div class=\"field-line\"></div></div>\n" +
    "<div class=\"field\"><span class=\"field-label\">Outcome</span><div class=\"field-line\"></div></div>\n" +
    "</div></div>\n" +
    "<div class=\"section\"><div class=\"section-title\">Notes &amp; Observations</div><div class=\"notes-area\"></div></div>\n" +
    "<div class=\"section\"><div class=\"section-title\">Review (after trade)</div><div class=\"notes-area small\"></div></div>\n" +
    "<div class=\"footer\">Generated " + escapeHtml(new Date().toLocaleDateString()) + " · Trading Manager v0.1.0</div>\n" +
    "</div>\n</body>\n</html>";
}

// ============================================================
// UI STYLE
// ============================================================
function buildUI(opts: Options): string {
  const { planName, strategyName, items, checked, theme } = opts;
  const dark = theme === "dark";

  const bg = dark ? "#11161d" : "#ffffff";
  const bgOuter = dark ? "#0b0f14" : "#f1f5f9";
  const fg = dark ? "#e6edf3" : "#0f172a";
  const fgMuted = dark ? "#9aa7b4" : "#475569";
  const fgDim = dark ? "#6b7886" : "#94a3b8";
  const border = dark ? "#232c38" : "#e2e8f0";
  const accent = dark ? "#2dd4bf" : "#0d9488";
  const accentBg = dark ? "rgba(45,212,191,0.08)" : "rgba(13,148,136,0.06)";
  const cardBg = dark ? "#161d26" : "#f8fafc";
  const checkedCount = items.filter((i) => checked.has(i.id)).length;

  const itemsHtml = buildItems(items, checked);

  return "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n" +
    "<title>" + escapeHtml(planName || "Trading Plan") + "</title>\n<style>\n" +
    "* { box-sizing: border-box; margin: 0; padding: 0; }\n" +
    "body { font-family: -apple-system, 'Segoe UI', system-ui, sans-serif; background: " + bgOuter + "; color: " + fg + "; padding: 32px 16px; line-height: 1.5; }\n" +
    ".page { max-width: 860px; margin: 0 auto; background: " + bg + "; border: 1px solid " + border + "; border-radius: 12px; padding: 32px 36px; box-shadow: 0 4px 20px rgba(0,0,0,0.08); }\n" +
    ".header { display: flex; align-items: center; gap: 12px; padding-bottom: 16px; border-bottom: 1px solid " + border + "; margin-bottom: 24px; }\n" +
    ".title { font-size: 20px; font-weight: 600; color: " + fg + "; }\n" +
    ".sub { font-size: 12px; color: " + fgMuted + "; margin-top: 3px; }\n" +
    ".badge { margin-left: auto; font-size: 11px; padding: 3px 10px; border-radius: 999px; background: " + accentBg + "; color: " + accent + "; font-weight: 600; }\n" +
    ".grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 24px; }\n" +
    ".stat { padding: 12px 14px; background: " + cardBg + "; border-radius: 8px; }\n" +
    ".stat-label { font-size: 10px; text-transform: uppercase; letter-spacing: 0.4px; color: " + fgDim + "; margin-bottom: 4px; }\n" +
    ".stat-value { font-size: 13px; font-weight: 600; color: " + fg + "; min-height: 18px; border-bottom: 1px dashed " + border + "; }\n" +
    ".section-title { font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; color: " + fgMuted + "; margin: 20px 0 10px; font-weight: 600; }\n" +
    ".group { margin-bottom: 16px; }\n" +
    ".group-title { font-size: 11px; font-weight: 600; color: " + accent + "; text-transform: uppercase; letter-spacing: 0.4px; margin-bottom: 8px; }\n" +
    ".item { display: flex; align-items: center; gap: 10px; padding: 8px 12px; margin-bottom: 4px; background: " + cardBg + "; border-radius: 6px; font-size: 13px; }\n" +
    ".item.is-checked { opacity: 0.6; }\n" +
    ".item.is-checked .text { text-decoration: line-through; }\n" +
    ".check { width: 18px; height: 18px; border: 1.5px solid " + border + "; border-radius: 4px; display: inline-flex; align-items: center; justify-content: center; font-size: 11px; color: " + accent + "; flex-shrink: 0; }\n" +
    ".item.is-checked .check { background: " + accentBg + "; border-color: " + accent + "; }\n" +
    ".notes { min-height: 120px; padding: 12px 14px; background: " + cardBg + "; border: 1px solid " + border + "; border-radius: 8px; }\n" +
    ".footer { margin-top: 24px; padding-top: 14px; border-top: 1px solid " + border + "; font-size: 10.5px; color: " + fgDim + "; text-align: center; }\n" +
    "@media print { body { background: white; padding: 0; } .page { box-shadow: none; border: none; } }\n" +
    "</style>\n</head>\n<body>\n<div class=\"page\">\n" +
    "<div class=\"header\"><div><div class=\"title\">Trading Plan</div><div class=\"sub\">" + escapeHtml(strategyName) + " · " + checkedCount + "/" + items.length + " checked</div></div><div class=\"badge\">" + escapeHtml(planName || "Untitled") + "</div></div>\n" +
    "<div class=\"grid\">\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Date</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Time</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Asset</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Entry</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Stop</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Target</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">R:R Planned</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">R Realized</div><div class=\"stat-value\"></div></div>\n" +
    "<div class=\"stat\"><div class=\"stat-label\">Outcome</div><div class=\"stat-value\"></div></div>\n" +
    "</div>\n" +
    "<div class=\"section-title\">Pre-Trade Checklist</div>\n" +
    (itemsHtml || "<div class=\"item\">No items</div>") +
    "\n<div class=\"section-title\">Notes</div>\n<div class=\"notes\"></div>\n" +
    "<div class=\"footer\">Generated " + escapeHtml(new Date().toLocaleDateString()) + " · Trading Manager v0.1.0</div>\n" +
    "</div>\n</body>\n</html>";
}

// ============================================================
// HYBRID STYLE
// ============================================================
function buildHybrid(opts: Options): string {
  const { planName, strategyName, items, checked, theme } = opts;
  const dark = theme === "dark";

  const bg = dark ? "#11161d" : "#ffffff";
  const bgOuter = dark ? "#0b0f14" : "#f8fafc";
  const fg = dark ? "#e6edf3" : "#0f172a";
  const fgMuted = dark ? "#9aa7b4" : "#475569";
  const fgDim = dark ? "#6b7886" : "#94a3b8";
  const border = dark ? "#232c38" : "#e2e8f0";
  const accent = dark ? "#2dd4bf" : "#0d9488";
  const cellBg = dark ? "#161d26" : "#fafbfc";
  const notesBg = dark ? "#0f151c" : "#fafbfc";
  const footerBg = dark ? "#0f151c" : "#f8fafc";

  const itemsHtml = buildItems(items, checked);

  return "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n" +
    "<title>" + escapeHtml(planName || "Trading Plan") + "</title>\n<style>\n" +
    "* { box-sizing: border-box; margin: 0; padding: 0; }\n" +
    "body { font-family: -apple-system, 'Segoe UI', system-ui, sans-serif; background: " + bgOuter + "; color: " + fg + "; padding: 32px 16px; line-height: 1.5; }\n" +
    ".page { max-width: 860px; margin: 0 auto; background: " + bg + "; border-radius: 16px; overflow: hidden; box-shadow: 0 12px 40px rgba(0,0,0,0.15); border: 1px solid " + border + "; }\n" +
    ".banner { background: linear-gradient(135deg, #2dd4bf 0%, #14b8a6 50%, #0d9488 100%); padding: 40px 40px 32px; color: #052e2b; }\n" +
    ".title { font-size: 28px; font-weight: 700; letter-spacing: -0.5px; }\n" +
    ".strategy { font-size: 13px; font-weight: 500; opacity: 0.85; margin-top: 6px; }\n" +
    ".plan-name { display: inline-block; margin-top: 10px; padding: 4px 12px; background: rgba(5,46,43,0.15); border-radius: 999px; font-size: 11px; font-weight: 600; }\n" +
    ".body { padding: 32px 40px; }\n" +
    ".grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; margin-bottom: 28px; }\n" +
    ".cell { padding: 12px 14px; border: 1px solid " + border + "; border-radius: 10px; background: " + cellBg + "; }\n" +
    ".cell-label { font-size: 9.5px; text-transform: uppercase; letter-spacing: 0.6px; color: " + fgDim + "; margin-bottom: 4px; }\n" +
    ".cell-value { font-size: 13px; font-weight: 600; color: " + fg + "; min-height: 18px; border-bottom: 1px dashed " + border + "; padding-bottom: 2px; }\n" +
    ".section-title { font-size: 11px; text-transform: uppercase; letter-spacing: 1px; color: " + accent + "; margin: 24px 0 10px; font-weight: 700; display: flex; align-items: center; gap: 8px; }\n" +
    ".section-title::before { content: ''; width: 3px; height: 14px; background: " + accent + "; border-radius: 2px; }\n" +
    ".group { margin-bottom: 16px; }\n" +
    ".group-title { font-size: 10.5px; color: " + fgMuted + "; text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 8px; font-weight: 600; }\n" +
    ".item { display: flex; align-items: center; gap: 10px; padding: 9px 12px; margin-bottom: 5px; border-radius: 8px; font-size: 13px; background: " + notesBg + "; border-left: 3px solid " + accent + "; }\n" +
    ".item.is-checked { border-left-color: " + fgDim + "; opacity: 0.65; }\n" +
    ".item.is-checked .text { text-decoration: line-through; }\n" +
    ".check { width: 18px; height: 18px; border: 1.5px solid " + border + "; border-radius: 50%; display: inline-flex; align-items: center; justify-content: center; font-size: 10px; color: " + accent + "; flex-shrink: 0; }\n" +
    ".item.is-checked .check { background: " + accent + "; border-color: " + accent + "; color: #052e2b; }\n" +
    ".notes { min-height: 100px; padding: 12px 14px; border: 1px solid " + border + "; border-radius: 10px; background: " + notesBg + "; }\n" +
    ".footer { text-align: center; padding: 16px; font-size: 10px; color: " + fgDim + "; border-top: 1px solid " + border + "; background: " + footerBg + "; }\n" +
    "@media print { body { background: white; padding: 0; } .page { box-shadow: none; } }\n" +
    "</style>\n</head>\n<body>\n<div class=\"page\">\n" +
    "<div class=\"banner\"><div class=\"title\">Trading Plan</div><div class=\"strategy\">" + escapeHtml(strategyName) + "</div><div class=\"plan-name\">" + escapeHtml(planName || "Untitled") + "</div></div>\n" +
    "<div class=\"body\">\n<div class=\"grid\">\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Date</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Time</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Asset</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Entry</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Stop</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Target</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">R:R Planned</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">R Realized</div><div class=\"cell-value\"></div></div>\n" +
    "<div class=\"cell\"><div class=\"cell-label\">Outcome</div><div class=\"cell-value\"></div></div>\n" +
    "</div>\n" +
    "<div class=\"section-title\">Checklist</div>\n" +
    (itemsHtml || "<div class=\"item\">No items</div>") +
    "\n<div class=\"section-title\">Notes</div>\n<div class=\"notes\"></div>\n" +
    "</div>\n<div class=\"footer\">Generated " + escapeHtml(new Date().toLocaleDateString()) + " · Trading Manager v0.1.0</div>\n" +
    "</div>\n</body>\n</html>";
}

// ============================================================
// Public entry point
// ============================================================
export function buildExportHtml(opts: Options): string {
  if (opts.style === "journal") return buildJournal(opts);
  if (opts.style === "ui") return buildUI(opts);
  return buildHybrid(opts);
}
