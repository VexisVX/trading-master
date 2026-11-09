import { useEffect, useState } from "react";
import { listStrategies, Strategy } from "../../lib/strategiesApi";
import { listPlans, createPlan, updatePlan, deletePlan, TradingPlan as PlanT } from "../../lib/tradingPlansApi";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { buildExportHtml, ExportStyle, ExportTheme } from "./exportHtml";
import "./TradingPlan.css";

interface ChecklistItem {
  id: string;
  text: string;
  category: "htf" | "drt" | "dfr" | "time_model" | "entry" | "other";
}

const LS_KEY = "trading-plan-current";

interface PersistedState {
  strategyId: number | null;
  planName: string;
  items: ChecklistItem[];
  checked: string[];
}

function loadPersisted(): PersistedState | null {
  try {
    const raw = localStorage.getItem(LS_KEY);
    if (!raw) return null;
    return JSON.parse(raw) as PersistedState;
  } catch { return null; }
}

function savePersisted(state: PersistedState) {
  try { localStorage.setItem(LS_KEY, JSON.stringify(state)); } catch { /* ignore */ }
}

export function TradingPlan() {
  const [strategies, setStrategies] = useState<Strategy[]>([]);
  const [selectedStrategy, setSelectedStrategy] = useState<number | null>(null);
  const [plans, setPlans] = useState<PlanT[]>([]);
  const [activePlan, setActivePlan] = useState<PlanT | null>(null);
  const [items, setItems] = useState<ChecklistItem[]>([]);
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [planName, setPlanName] = useState("");
  const [confirmDelete, setConfirmDelete] = useState<PlanT | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savedMessage, setSavedMessage] = useState<string | null>(null);
  const [showExport, setShowExport] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const list = await listStrategies();
        setStrategies(list);
        const persisted = loadPersisted();
        if (persisted && persisted.strategyId && list.some((s) => s.id === persisted.strategyId)) {
          setSelectedStrategy(persisted.strategyId);
          setPlanName(persisted.planName);
          setItems(persisted.items);
          setChecked(new Set(persisted.checked));
        } else if (list.length > 0) {
          setSelectedStrategy(list[0].id);
        }
      } catch (e) { setError(String(e)); }
    })();
  }, []);

  useEffect(() => {
    if (!selectedStrategy) return;
    (async () => {
      try {
        const list = await listPlans(selectedStrategy);
        setPlans(list);
        const persisted = loadPersisted();
        if (persisted && persisted.strategyId === selectedStrategy && persisted.items.length > 0) {
          return;
        }
        if (list.length > 0) loadPlan(list[0]);
        else { setActivePlan(null); setPlanName(""); setItems([]); setChecked(new Set()); }
      } catch (e) { setError(String(e)); }
    })();
  }, [selectedStrategy]);

  useEffect(() => {
    savePersisted({
      strategyId: selectedStrategy,
      planName,
      items,
      checked: Array.from(checked),
    });
  }, [selectedStrategy, planName, items, checked]);

  function loadPlan(p: PlanT) {
    setActivePlan(p);
    setPlanName(p.name);
    try {
      setItems(JSON.parse(p.checklist_json) as ChecklistItem[]);
      setChecked(new Set());
    } catch {
      setItems([]);
      setChecked(new Set());
    }
  }

  function startNewPlan() {
    setActivePlan(null);
    setPlanName("");
    const strategy = strategies.find((st) => st.id === selectedStrategy);
    const generated: ChecklistItem[] = [];
    if (strategy) {
      for (const tf of strategy.timeframes) {
        for (const field of tf.fields) {
          let category: ChecklistItem["category"] = "other";
          const k = field.key.toLowerCase();
          if (k.includes("bias") || k.includes("trend") || k.includes("structure") || k.includes("poi") || k.includes("liquidity") || k.includes("zone") || k.includes("ma_") || k.includes("rsi")) {
            category = "htf";
          } else if (k.includes("drt")) {
            category = "drt";
          } else if (k.includes("dfr")) {
            category = "dfr";
          } else if (k.includes("time_model") || k.includes("session")) {
            category = "time_model";
          } else if (k.includes("entry") || k.includes("execution") || k.includes("trigger") || k.includes("confirm") || k.includes("macd")) {
            category = "entry";
          }
          generated.push({
            id: `${tf.timeframe}-${field.key}-${Date.now()}-${generated.length}`,
            text: `${tf.timeframe}: ${field.label}`,
            category,
          });
        }
      }
    }
    if (generated.length === 0) {
      generated.push(
        { id: `htf-${Date.now()}`, text: "HTF bias identified", category: "htf" },
        { id: `entry-${Date.now()}`, text: "Entry confirmation on LTF", category: "entry" },
      );
    }
    setItems(generated);
    setChecked(new Set());
  }

  async function savePlan() {
    if (!selectedStrategy || !planName.trim()) {
      setError("Plan name required.");
      return;
    }
    setError(null);
    try {
      const json = JSON.stringify(items);
      if (activePlan) {
        await updatePlan(activePlan.id, planName, json, null);
      } else {
        await createPlan({
          strategy_id: selectedStrategy,
          name: planName,
          checklist_json: json,
          notes: null,
        });
      }
      const list = await listPlans(selectedStrategy);
      setPlans(list);
      const found = list.find((p) => p.name === planName);
      if (found) setActivePlan(found);
      setSavedMessage("Saved ✓");
      setTimeout(() => setSavedMessage(null), 1500);
    } catch (e) { setError(String(e)); }
  }

  async function removePlan() {
    if (!confirmDelete) return;
    try {
      await deletePlan(confirmDelete.id);
      try { localStorage.removeItem(LS_KEY); } catch { /* ignore */ }
      setConfirmDelete(null);
      if (selectedStrategy) {
        const list = await listPlans(selectedStrategy);
        setPlans(list);
        if (list.length > 0) loadPlan(list[0]);
        else { setActivePlan(null); setPlanName(""); setItems([]); }
      }
    } catch (e) { setError(String(e)); }
  }

  function clearDraft() {
    try { localStorage.removeItem(LS_KEY); } catch { /* ignore */ }
    setPlanName("");
    setItems([]);
    setChecked(new Set());
    setActivePlan(null);
  }

  function toggleCheck(id: string) {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function resetChecks() { setChecked(new Set()); }

  function addItem() {
    setItems((i) => [...i, { id: `item-${Date.now()}`, text: "", category: "other" }]);
  }

  function updateItem(id: string, patch: Partial<ChecklistItem>) {
    setItems((i) => i.map((it) => it.id === id ? { ...it, ...patch } : it));
  }

  function removeItem(id: string) {
    setItems((i) => i.filter((it) => it.id !== id));
    setChecked((c) => { const next = new Set(c); next.delete(id); return next; });
  }

  const checkedCount = items.filter((i) => checked.has(i.id)).length;
  const allDone = items.length > 0 && checkedCount === items.length;

  if (strategies.length === 0) {
    return (
      <div className="card">
        <div className="empty-state">
          <span className="empty-state__icon">◈</span>
          <span className="empty-state__title">No strategies yet</span>
          <span className="empty-state__hint">Create a strategy first (Templates → ICT or similar).</span>
        </div>
      </div>
    );
  }

  return (
    <div className="tp">
      <aside className="tp-sidebar">
        <div className="tp-sidebar__section">
          <label className="tp-label">Strategy</label>
          <select value={selectedStrategy ?? ""} onChange={(e) => setSelectedStrategy(Number(e.target.value))}>
            {strategies.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
          </select>
        </div>

        <div className="tp-sidebar__section">
          <div className="tp-sidebar__head">
            <span className="tp-label">Plans ({plans.length})</span>
            <button className="btn btn--ghost btn--small" onClick={startNewPlan}>+ New</button>
          </div>
          <div className="tp-plan-list">
            {plans.length === 0 && <div className="tp-plan-list__empty">No plans yet.</div>}
            {plans.map((p) => (
              <button
                key={p.id}
                className={`tp-plan-item ${activePlan?.id === p.id ? "is-active" : ""}`}
                onClick={() => loadPlan(p)}
              >
                {p.name}
              </button>
            ))}
          </div>
        </div>
      </aside>

      <main className="tp-main">
        <div className="tp-header">
          <input
            className="tp-name-input"
            value={planName}
            onChange={(e) => setPlanName(e.target.value)}
            placeholder="Plan name (e.g. C.I.T London Open)"
          />
          <div className="tp-header__actions">
            {savedMessage && <span className="tp-saved">{savedMessage}</span>}
            <button className="btn btn--primary btn--small" onClick={savePlan}>
              {activePlan ? "Save" : "Create"}
            </button>
            {activePlan && (
              <button className="btn btn--danger btn--small" onClick={() => setConfirmDelete(activePlan)}>
                Delete
              </button>
            )}
            {!activePlan && items.length > 0 && (
              <button className="btn btn--ghost btn--small" onClick={clearDraft}>
                Clear
              </button>
            )}
            <button className="btn btn--ghost btn--small" onClick={() => setShowExport(true)}>
              Export
            </button>
          </div>
        </div>

        {error && <div className="card card--error">{error}</div>}

        <div className="tp-progress">
          <div className="tp-progress__bar">
            <div
              className={`tp-progress__fill ${allDone ? "is-done" : ""}`}
              style={{ width: `${items.length > 0 ? (checkedCount / items.length) * 100 : 0}%` }}
            />
          </div>
          <span className="tp-progress__text mono">
            {checkedCount} / {items.length} {allDone && "✓ ready"}
          </span>
          <button className="btn btn--ghost btn--small" onClick={resetChecks} disabled={items.length === 0}>
            Reset
          </button>
        </div>

        {(["htf", "drt", "dfr", "time_model", "entry", "other"] as const).map((cat) => {
          const catItems = items.filter((i) => i.category === cat);
          if (catItems.length === 0) return null;
          const titles: Record<string, string> = {
            htf: "HTF / Long Term",
            drt: "DRT",
            dfr: "DFR",
            time_model: "Time Model",
            entry: "Entry / LTF",
            other: "Other",
          };
          return (
            <div key={cat} className="tp-category">
              <div className="tp-category__title">{titles[cat]}</div>
              {catItems.map((item) => (
                <div key={item.id} className={`tp-item ${checked.has(item.id) ? "is-checked" : ""}`}>
                  <button
                    type="button"
                    className={`tp-item__check ${checked.has(item.id) ? "is-checked" : ""}`}
                    onClick={() => toggleCheck(item.id)}
                  >
                    {checked.has(item.id) ? "✓" : ""}
                  </button>
                  <input
                    className="tp-item__text"
                    value={item.text}
                    onChange={(e) => updateItem(item.id, { text: e.target.value })}
                    placeholder="What to check…"
                  />
                  <select
                    className="tp-item__cat"
                    value={item.category}
                    onChange={(e) => updateItem(item.id, { category: e.target.value as any })}
                  >
                    <option value="htf">HTF</option>
                    <option value="drt">DRT</option>
                    <option value="dfr">DFR</option>
                    <option value="time_model">Time Model</option>
                    <option value="entry">Entry</option>
                    <option value="other">Other</option>
                  </select>
                  <button className="icon-btn icon-btn--danger" onClick={() => removeItem(item.id)}>×</button>
                </div>
              ))}
            </div>
          );
        })}

        {items.length === 0 && (
          <div className="tp-empty">
            No items yet. Click "+ Add item" or start from a new plan.
          </div>
        )}

        <button className="btn btn--ghost" onClick={addItem} style={{ alignSelf: "flex-start" }}>
          + Add item
        </button>
      </main>

      {confirmDelete && (
        <ConfirmDialog
          title={`Delete plan "${confirmDelete.name}"?`}
          body="This plan will be permanently removed."
          confirmLabel="Delete"
          cancelLabel="Keep"
          tone="danger"
          onConfirm={removePlan}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {showExport && (
        <ExportDialog
          planName={planName}
          strategyName={strategies.find((s) => s.id === selectedStrategy)?.name ?? "—"}
          items={items}
          checked={checked}
          onClose={() => setShowExport(false)}
        />
      )}
    </div>
  );
}

// ============================================================
// Export Dialog
// ============================================================
function ExportDialog({ planName, strategyName, items, checked, onClose }: {
  planName: string;
  strategyName: string;
  items: ChecklistItem[];
  checked: Set<string>;
  onClose: () => void;
}) {
  const [style, setStyle] = useState<ExportStyle>("journal");
  const [theme, setTheme] = useState<ExportTheme>("light");

  async function exportHtml() {
    const html = buildExportHtml({ planName, strategyName, items, checked, style, theme });
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const { invoke } = await import("@tauri-apps/api/core");

      const dest = await save({
        title: "Save trading plan",
        defaultPath: `${planName || "trading-plan"}.html`,
        filters: [{ name: "HTML", extensions: ["html"] }],
      });

      if (!dest) return;
      await invoke("export_text_file", { dest, content: html });
      onClose();
      alert(`Saved to ${dest}\n\nOpen it and press Ctrl+P to save as PDF.`);
    } catch (e) {
      alert(`Export failed: ${e}`);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal--export" onClick={(e) => e.stopPropagation()}>
        <button className="screenshot-close" onClick={onClose}>×</button>
        <h3>Export Trading Plan</h3>
        <p className="dim">Pick a style and theme. Then save as HTML and print to PDF.</p>

        <div className="export-section">
          <div className="export-label">Style</div>
          <div className="export-options">
            <button className={`export-option ${style === "journal" ? "is-active" : ""}`} onClick={() => setStyle("journal")}>
              <div className="export-option__title">Journal</div>
              <div className="export-option__desc">Paper-like layout for printing</div>
            </button>
            <button className={`export-option ${style === "ui" ? "is-active" : ""}`} onClick={() => setStyle("ui")}>
              <div className="export-option__title">UI</div>
              <div className="export-option__desc">Matches the app design</div>
            </button>
            <button className={`export-option ${style === "hybrid" ? "is-active" : ""}`} onClick={() => setStyle("hybrid")}>
              <div className="export-option__title">Hybrid</div>
              <div className="export-option__desc">Modern card with gradient</div>
            </button>
          </div>
        </div>

        <div className="export-section">
          <div className="export-label">Theme</div>
          <div className="export-options">
            <button className={`export-theme ${theme === "light" ? "is-active" : ""}`} onClick={() => setTheme("light")}>
              <span className="export-theme__swatch" style={{ background: "#fdfcfb" }} />
              Light
            </button>
            <button className={`export-theme ${theme === "dark" ? "is-active" : ""}`} onClick={() => setTheme("dark")}>
              <span className="export-theme__swatch" style={{ background: "#11161d" }} />
              Dark
            </button>
          </div>
        </div>

        <div className="modal__actions">
          <button className="btn btn--ghost" onClick={onClose}>Cancel</button>
          <button className="btn btn--primary" onClick={exportHtml}>Export HTML</button>
        </div>
      </div>
    </div>
  );
}
