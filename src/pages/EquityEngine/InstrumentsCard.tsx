import { useEffect, useState } from "react";
import { Instrument, listInstruments } from "../../lib/equityApi";
import { InstrumentModal } from "./InstrumentModal";

export function InstrumentsCard() {
  const [instruments, setInstruments] = useState<Instrument[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [editing, setEditing] = useState<Instrument | null>(null);
  const [creating, setCreating] = useState(false);

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

  return (
    <section className="ee-card ee-instruments">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Instruments</h3>
          <p className="ee-card__subtitle">
            Specs used for position sizing — no instrument, no trade
          </p>
        </div>
        <button
          className="ee-btn ee-btn--primary"
          onClick={() => setCreating(true)}
        >
          + Add instrument
        </button>
      </div>

      {loading && <div className="ee-empty-inline">Loading…</div>}
      {err && <div className="ee-alert ee-alert--error">{err}</div>}

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
                    <button
                      className="ee-btn ee-btn--ghost"
                      onClick={() => setEditing(inst)}
                    >
                      Edit
                    </button>
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
    </section>
  );
}
