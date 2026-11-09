import { useCallback, useEffect, useState } from "react";
import {
  createStrategy, deleteStrategy, listStrategies, updateStrategy,
  Strategy as StrategyT, NewStrategy, StrategyTimeframe,
  StrategyField, FieldType, TEMPLATES, listVersions, getVersionJson,
} from "../../lib/strategiesApi";
import { useSettle } from "../../lib/useSettle";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import "./Strategy.css";

const TIMEFRAMES = ["1W", "1D", "4H", "1H", "15M", "5M", "1M", "15S"];

export function Strategy() {
  const [list, setList] = useState<StrategyT[]>([]);
  const [mode, setMode] = useState<"idle" | "create" | "edit">("idle");
  const [editing, setEditing] = useState<StrategyT | null>(null);
  const [template, setTemplate] = useState<typeof TEMPLATES[0] | null>(null);
  const [showTemplates, setShowTemplates] = useState(false);
  const [versionFor, setVersionFor] = useState<StrategyT | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<StrategyT | null>(null);
  const [busy, setBusy] = useState(false);
  const formRef = useSettle<HTMLDivElement>();

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setList(await listStrategies());
    } catch (e) { setError(String(e)); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  function openCreate() {
    setEditing(null);
    setTemplate(null);
    setMode("create");
  }

  function openEdit(s: StrategyT) {
    setEditing(s);
    setTemplate(null);
    setMode("edit");
  }

  function closeForm() {
    setMode("idle");
    setEditing(null);
    setTemplate(null);
  }

  function pickTemplate(tpl: typeof TEMPLATES[0]) {
    setShowTemplates(false);
    setEditing(null);
    setTemplate(tpl);
    setMode("create");
  }

  async function onDelete(s: StrategyT) {
    setConfirmDelete(s);
  }

  async function performDelete() {
    if (!confirmDelete) return;
    const id = confirmDelete.id;
    setConfirmDelete(null);
    try {
      await deleteStrategy(id);
      refresh();
    } catch (e) { setError(String(e)); }
  }

  return (
    <div className="strategy">
      <div className="strategy__head">
        <h2>Strategies</h2>
        <div style={{ display: "flex", gap: 8 }}>
          <button className="btn btn--ghost" onClick={() => setShowTemplates(true)} disabled={mode !== "idle"}>
            Templates
          </button>
          <button className="btn btn--primary" onClick={openCreate} disabled={mode !== "idle"}>
            New strategy
          </button>
        </div>
      </div>

      {error && <div className="card card--error">{error}</div>}

      {mode !== "idle" && (
        <div className="card pop-in" ref={formRef}>
          <StrategyForm
            key={editing?.id ?? template?.id ?? "new"}
            initial={editing}
            template={template}
            onDone={() => { closeForm(); refresh(); }}
            onCancel={closeForm}
            busy={busy}
            setBusy={setBusy}
            setError={setError}
          />
        </div>
      )}

      {list.length === 0 && mode === "idle" ? (
        <div className="card">
          <div className="empty-state">
            <span className="empty-state__icon">◈</span>
            <span className="empty-state__title">No strategies yet</span>
            <span className="empty-state__hint">
              Create a strategy manually or start from a template.
            </span>
          </div>
        </div>
      ) : (
        <div className="strategy__grid stagger">
          {list.map((s) => (
            <div key={s.id} className="card strategy-card card--interactive lift">
              <div className="strategy-card__head">
                <h3>{s.name}</h3>
                <span className="card__badge">v{s.version}</span>
              </div>
              {s.description && <p className="strategy-card__desc">{s.description}</p>}
              <div className="strategy-card__tfs">
                {s.timeframes.map((tf) => (
                  <div key={tf.timeframe} className="tf-chip">
                    <span className="tf-chip__name">{tf.timeframe}</span>
                    <span className="tf-chip__count">{tf.fields.length} fields</span>
                  </div>
                ))}
              </div>
              <div className="strategy-card__actions">
                <button className="btn btn--ghost" onClick={() => setVersionFor(s)}>Versions</button>
                <button className="btn btn--ghost" onClick={() => openEdit(s)}>Edit</button>
                <button className="btn btn--danger" onClick={() => onDelete(s)}>Delete</button>
              </div>
            </div>
          ))}
        </div>
      )}

      {confirmDelete && (
        <ConfirmDialog
          title={`Delete strategy "${confirmDelete.name}"?`}
          body="All versions of this strategy will be permanently deleted. Trades referencing it will keep their ids but lose the link."
          confirmLabel="Delete strategy"
          cancelLabel="Keep"
          tone="danger"
          onConfirm={performDelete}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {showTemplates && (
        <TemplatesModal onSelect={pickTemplate} onClose={() => setShowTemplates(false)} />
      )}

      {versionFor && (
        <VersionHistoryModal strategy={versionFor} onClose={() => setVersionFor(null)} />
      )}
    </div>
  );
}

// ============================================================
// Templates modal
// ============================================================
function TemplatesModal({ onSelect, onClose }: {
  onSelect: (tpl: typeof TEMPLATES[0]) => void;
  onClose: () => void;
}) {
  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal--wide" onClick={(e) => e.stopPropagation()}>
        <button className="screenshot-close" onClick={onClose}>×</button>
        <h3>Start from a template</h3>
        <p className="dim">Pick a base structure. You can edit everything after.</p>
        <div className="tpl-grid">
          {TEMPLATES.map((tpl) => (
            <button key={tpl.id} className="tpl-card" onClick={() => onSelect(tpl)}>
              <div className="tpl-card__label">{tpl.label}</div>
              <div className="tpl-card__desc">{tpl.description}</div>
              <div className="tpl-card__tfs">
                {tpl.timeframes.map((tf) => (
                  <span key={tf.timeframe} className="tpl-tf">{tf.timeframe}</span>
                ))}
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

// ============================================================
// Version history modal
// ============================================================
function VersionHistoryModal({ strategy, onClose }: {
  strategy: StrategyT;
  onClose: () => void;
}) {
  const [versions, setVersions] = useState<[number, number, string, string | null, string][] | null>(null);
  const [viewing, setViewing] = useState<{ version: number; json: string } | null>(null);

  useEffect(() => {
    (async () => {
      try { setVersions(await listVersions(strategy.id)); }
      catch { setVersions([]); }
    })();
  }, [strategy.id]);

  async function viewVersion(versionId: number, version: number) {
    try {
      const json = await getVersionJson(versionId);
      if (json) setViewing({ version, json });
    } catch { /* ignore */ }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal--wide" onClick={(e) => e.stopPropagation()}>
        <button className="screenshot-close" onClick={onClose}>×</button>
        <h3>Version history · {strategy.name}</h3>
        {versions === null && <p className="dim">Loading…</p>}
        {versions && versions.length === 0 && <p className="dim">No version history yet.</p>}
        {versions && versions.length > 0 && (
          <table className="trades" style={{ marginTop: 12 }}>
            <thead>
              <tr><th>Version</th><th>Name</th><th>Created</th><th></th></tr>
            </thead>
            <tbody>
              {versions.map(([ver, vid, name, _desc, created]) => (
                <tr key={vid}>
                  <td className="mono">v{ver}</td>
                  <td>{name}</td>
                  <td className="mono dim">{created.slice(0, 16).replace("T", " ")}</td>
                  <td>
                    <button className="btn btn--ghost btn--small" onClick={() => viewVersion(vid, ver)}>
                      View
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}

        {viewing && (
          <div className="version-json">
            <h4>v{viewing.version} timeframes</h4>
            <pre>{JSON.stringify(JSON.parse(viewing.json), null, 2)}</pre>
          </div>
        )}

        <div className="modal__actions">
          <button className="btn btn--ghost" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

// ============================================================
// Strategy form
// ============================================================
interface FormProps {
  initial: StrategyT | null;
  template: typeof TEMPLATES[0] | null;
  onDone: () => void;
  onCancel: () => void;
  busy: boolean;
  setBusy: (v: boolean) => void;
  setError: (s: string | null) => void;
}

function StrategyForm({ initial, template, onDone, onCancel, busy, setBusy, setError }: FormProps) {
  const [name, setName] = useState(initial?.name ?? template?.label ?? "");
  const [description, setDescription] = useState(initial?.description ?? template?.description ?? "");
  const [selectedTFs, setSelectedTFs] = useState<string[]>(
    initial ? initial.timeframes.map((t) => t.timeframe)
    : template ? template.timeframes.map((t) => t.timeframe)
    : ["4H", "5M"]
  );
  const [perTF, setPerTF] = useState<Record<string, StrategyField[]>>(() => {
    const map: Record<string, StrategyField[]> = {};
    if (initial) for (const tf of initial.timeframes) map[tf.timeframe] = tf.fields;
    else if (template) for (const tf of template.timeframes) map[tf.timeframe] = tf.fields;
    return map;
  });

  function toggleTF(tf: string) {
    setSelectedTFs((prev) => {
      const has = prev.includes(tf);
      if (has) {
        setPerTF((p) => { const c = { ...p }; delete c[tf]; return c; });
        return prev.filter((x) => x !== tf);
      }
      setPerTF((p) => ({ ...p, [tf]: p[tf] ?? [] }));
      return [...prev, tf];
    });
  }

  function addField(tf: string) {
    setPerTF((p) => ({
      ...p,
      [tf]: [...(p[tf] ?? []), { key: "", label: "", type: "text" as FieldType, options: [], required: false }],
    }));
  }

  function updateField(tf: string, idx: number, patch: Partial<StrategyField>) {
    setPerTF((p) => {
      const arr = [...(p[tf] ?? [])];
      arr[idx] = { ...arr[idx], ...patch };
      return { ...p, [tf]: arr };
    });
  }

  function removeField(tf: string, idx: number) {
    setPerTF((p) => ({ ...p, [tf]: (p[tf] ?? []).filter((_, i) => i !== idx) }));
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (!name.trim()) { setError("Name is required."); return; }
    if (selectedTFs.length === 0) { setError("Select at least one timeframe."); return; }

    const timeframes: StrategyTimeframe[] = selectedTFs.map((tf) => ({
      timeframe: tf,
      fields: (perTF[tf] ?? []).map((f) => ({
        key: f.key.trim(),
        label: f.label.trim(),
        type: f.type,
        options: f.type === "select" ? (f.options ?? []).map((o) => o.trim()).filter(Boolean) : [],
        required: !!f.required,
      })),
    }));

    for (const tf of timeframes) {
      for (const f of tf.fields) {
        if (!f.key || !f.label) { setError(`Field in ${tf.timeframe}: key and label required.`); return; }
        if (f.type === "select" && (!f.options || f.options.length === 0)) {
          setError(`Field "${f.label}" in ${tf.timeframe}: select needs options.`); return;
        }
      }
    }

    const payload: NewStrategy = {
      name: name.trim(),
      description: description.trim() || null,
      timeframes,
    };

    setBusy(true);
    try {
      if (initial) {
        await updateStrategy(initial.id, payload);
      } else {
        await createStrategy(payload);
      }
      onDone();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="strategy-form" onSubmit={submit}>
      <div className="strategy-form__row">
        <label>
          <span>Name</span>
          <input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. ICT Silver Bullet" />
        </label>
        <label className="strategy-form__desc">
          <span>Description</span>
          <input value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Optional" />
        </label>
      </div>

      <div className="strategy-form__tfs">
        <span className="strategy-form__label">Timeframes</span>
        <div className="tf-checkboxes">
          {TIMEFRAMES.map((tf) => (
            <label key={tf} className={`tf-checkbox ${selectedTFs.includes(tf) ? "is-on" : ""}`}>
              <input type="checkbox" checked={selectedTFs.includes(tf)} onChange={() => toggleTF(tf)} />
              <span>{tf}</span>
            </label>
          ))}
        </div>
      </div>

      {selectedTFs.map((tf) => (
        <div key={tf} className="tf-block fade-in">
          <div className="tf-block__head">
            <h4>{tf}</h4>
            <button type="button" className="btn btn--small" onClick={() => addField(tf)}>+ Field</button>
          </div>
          {(perTF[tf] ?? []).length === 0 ? (
            <div className="tf-block__empty">No fields. Add one to define what to record at this timeframe.</div>
          ) : (
            (perTF[tf] ?? []).map((f, i) => (
              <div key={i} className="field-row">
                <input placeholder="key" value={f.key} onChange={(e) => updateField(tf, i, { key: e.target.value })} />
                <input placeholder="Label" value={f.label} onChange={(e) => updateField(tf, i, { label: e.target.value })} />
                <select value={f.type} onChange={(e) => updateField(tf, i, { type: e.target.value as FieldType })}>
                  <option value="text">Text</option>
                  <option value="textarea">Textarea</option>
                  <option value="select">Select</option>
                </select>
                {f.type === "select" && (
                  <input
                    placeholder="opt1, opt2, ..."
                    value={(f.options ?? []).join(", ")}
                    onChange={(e) => updateField(tf, i, { options: e.target.value.split(",").map((s) => s.trim()) })}
                  />
                )}
                <label className="field-row__req">
                  <input type="checkbox" checked={!!f.required} onChange={(e) => updateField(tf, i, { required: e.target.checked })} />
                  <span>req</span>
                </label>
                <button type="button" className="btn btn--ghost" onClick={() => removeField(tf, i)}>×</button>
              </div>
            ))
          )}
        </div>
      ))}

      <div className="strategy-form__actions">
        <button type="button" className="btn btn--ghost" onClick={onCancel}>Cancel</button>
        <button type="submit" className="btn btn--primary" disabled={busy}>
          {busy ? "Saving…" : initial ? "Save changes" : "Create"}
        </button>
      </div>
    </form>
  );
}
