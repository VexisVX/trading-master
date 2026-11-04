import { useEffect, useState } from "react";
import { Instrument, listInstruments, deleteInstrument, restoreDefaultInstruments, exportInstrumentsJSON, importInstrumentsJSON } from "../../lib/equityApi";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeTextFile, readTextFile } from "@tauri-apps/plugin-fs";
import { InstrumentModal } from "./InstrumentModal";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";

export function InstrumentsCard() {
  const [instruments, setInstruments] = useState<Instrument[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [editing, setEditing] = useState<Instrument | null>(null);
  const [creating, setCreating] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<Instrument | null>(null);
  const [restoring, setRestoring] = useState(false);
  const [ok, setOk] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      setInstruments(await listInstruments());
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  // Auto-dismiss success message after 4s
  useEffect(() => {
    if (!ok) return;
    const t = setTimeout(() => setOk(null), 4000);
    return () => clearTimeout(t);
  }, [ok]);

  async function doRestoreDefaults() {
    setRestoring(true);
    setErr(null);
    setOk(null);
    try {
      const n = await restoreDefaultInstruments();
      await refresh();
      if (n > 0) {
        setOk(`Restored ${n} instrument${n === 1 ? "" : "s"}.`);
      } else {
        setOk("All default instruments already exist.");
      }
    } catch (e) {
      setErr(String(e));
    } finally {
      setRestoring(false);
    }
  }

  async function doDelete() {
    if (!pendingDelete) return;
    const sym = pendingDelete.symbol;
    setPendingDelete(null);
    setErr(null);
    try {
      await deleteInstrument(sym);
      await refresh();
    } catch (e) {
      setErr(String(e));
    }
  }

  async function handleExport() {
    setErr(null);
    setOk(null);
    try {
      const json = await exportInstrumentsJSON();
      const dest = await save({
        title: "Export instruments",
        defaultPath: `instruments-${new Date().toISOString().slice(0, 10)}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!dest) return;
      await writeTextFile(dest, json);
      setOk(`Exported to ${dest}`);
    } catch (e) {
      setErr(String(e));
    }
  }

  async function handleImport() {
    setErr(null);
    setOk(null);
    try {
      const src = await open({
        title: "Import instruments",
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!src || Array.isArray(src)) return;
      const json = await readTextFile(src);
      const replace = window.confirm(
        "Replace ALL existing instruments with the file contents?\n\nCancel = merge (add/update only)."
      );
      const count = await importInstrumentsJSON(json, replace);
      await refresh();
      setOk(`Imported ${count} instrument${count === 1 ? "" : "s"}.`);
    } catch (e) {
      setErr(String(e));
    }
  }

  return (
    <section className="ee-card ee-instruments">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Instruments</h3>
          <p className="ee-card__subtitle">
            Specs used for position sizing — no instrument, no trade
          </p>
        </div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <button
            className="ee-btn ee-btn--ghost"
            onClick={handleExport}
            title="Export to JSON file"
          >
            ⤓ Export
          </button>
          <button
            className="ee-btn ee-btn--ghost"
            onClick={handleImport}
            title="Import from JSON file"
          >
            ⤒ Import
          </button>
          <button
            className="ee-btn ee-btn--ghost ee-inst-restore"
            onClick={doRestoreDefaults}
            disabled={restoring}
            title="Restore 18 default instruments"
          >
            {restoring ? "⟳ Restoring…" : "⟳ Restore defaults"}
          </button>
          <button
            className="ee-btn ee-btn--primary"
            onClick={() => setCreating(true)}
          >
            + Add
          </button>
        </div>
      </div>

      {loading && <div className="ee-empty-inline">Loading…</div>}

      {(ok || err) && (
        <div className="ee-inst-toast-wrap">
          {ok && <div className="ee-inst-toast ee-inst-toast--ok">{ok}</div>}
          {err && <div className="ee-inst-toast ee-inst-toast--err">{err}</div>}
        </div>
      )}

      {!loading && !err && (
        <div className="ee-inst-table-wrap">
          <table className="ee-inst-table">
            <thead>
              <tr>
                <th>Symbol</th>
                <th>Currency</th>
                <th>Tick size</th>
                <th>Tick value</th>
                <th>Contract</th>
                <th>Lot step</th>
                <th>Source</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {instruments.length === 0 && (
                <tr>
                  <td colSpan={8} className="ee-muted" style={{ textAlign: "center" }}>
                    No instruments yet. Click + Add.
                  </td>
                </tr>
              )}
              {instruments.map((inst) => (
                <tr key={inst.symbol}>
                  <td className="mono">
                    <strong>{inst.symbol}</strong>
                  </td>
                  <td>{inst.currency}</td>
                  <td className="mono">{inst.tick_size}</td>
                  <td className="mono">{inst.tick_value}</td>
                  <td className="mono">{inst.contract_size}</td>
                  <td className="mono">{inst.lot_step}</td>
                  <td>
                    <span
                      className={`ee-inst-source ee-inst-source--${inst.spec_source}`}
                    >
                      {inst.spec_source}
                    </span>
                  </td>
                  <td>
                    <div style={{ display: "flex", gap: 6 }}>
                      <button
                        className="ee-btn ee-btn--ghost"
                        onClick={() => setEditing(inst)}
                        style={{ padding: "2px 8px", fontSize: 11 }}
                      >
                        Edit
                      </button>
                      <button
                        className="ee-cf-delete"
                        onClick={() => setPendingDelete(inst)}
                        title="Delete this instrument"
                      >
                        ✕
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {(creating || editing) && (
        <InstrumentModal
          initial={editing}
          onClose={() => {
            setCreating(false);
            setEditing(null);
          }}
          onSaved={refresh}
        />
      )}
    
      {pendingDelete && (
        <ConfirmDialog
          title="Delete instrument?"
          body={`This will permanently remove ${pendingDelete.symbol}. Open trades on this symbol will fail to size until you re-add it.`}
          confirmLabel="Delete"
          cancelLabel="Cancel"
          tone="danger"
          onConfirm={doDelete}
          onCancel={() => setPendingDelete(null)}
        />
      )}

    </section>
  );
}
