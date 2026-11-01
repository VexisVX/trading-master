import { useState } from "react";
import {
  evaluateTrade,
  fmtMoney,
  fmtPct,
  humanizeReason,
  DecisionOut,
} from "../../lib/equityApi";

interface Props {
  hasSnapshot: boolean;
}

export function DecisionTab({ hasSnapshot }: Props) {
  const [entry, setEntry] = useState("1.1000");
  const [stop, setStop] = useState("1.0950");
  const [tp, setTp] = useState("1.1150");
  const [mult, setMult] = useState("100000");
  const [comm, setComm] = useState("0");
  const [slip, setSlip] = useState("0");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<DecisionOut | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function evaluate(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setResult(null);
    setBusy(true);
    try {
      const r = await evaluateTrade({
        entry,
        stop,
        take_profit: tp || null,
        contract_multiplier: mult,
        commission_per_unit: comm || "0",
        slippage_buffer: slip || "0",
      });
      setResult(r);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function queryOnly() {
    setError(null);
    setResult(null);
    setBusy(true);
    try {
      const r = await evaluateTrade(null);
      setResult(r);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="ee-decision">
      <form className="ee-decision__form card" onSubmit={evaluate}>
        <div className="ee-form-grid">
          <label>
            <span>Entry</span>
            <input className="mono" value={entry} onChange={(e) => setEntry(e.target.value)} />
          </label>
          <label>
            <span>Stop</span>
            <input className="mono" value={stop} onChange={(e) => setStop(e.target.value)} />
          </label>
          <label>
            <span>Take profit (opt)</span>
            <input className="mono" value={tp} onChange={(e) => setTp(e.target.value)} />
          </label>
          <label>
            <span>Contract multiplier</span>
            <input className="mono" value={mult} onChange={(e) => setMult(e.target.value)} />
          </label>
          <label>
            <span>Commission / unit</span>
            <input className="mono" value={comm} onChange={(e) => setComm(e.target.value)} />
          </label>
          <label>
            <span>Slippage / unit</span>
            <input className="mono" value={slip} onChange={(e) => setSlip(e.target.value)} />
          </label>
        </div>

        {!hasSnapshot && (
          <div className="ee-error">No snapshot yet — go to Snapshot tab first.</div>
        )}
        {error && <div className="ee-error">{error}</div>}

        <div className="ee-decision__actions">
          <button className="btn btn--primary" disabled={busy || !hasSnapshot}>
            {busy ? "…" : "Evaluate trade"}
          </button>
          <button
            type="button"
            className="btn btn--ghost"
            onClick={queryOnly}
            disabled={busy || !hasSnapshot}
            title="See max allowed risk without proposing a specific size"
          >
            Query allowed risk
          </button>
        </div>
      </form>

      {result && (
        <div
          className={`ee-decision__result card ${
            result.allowed ? "is-allowed" : "is-rejected"
          }`}
        >
          <div className="ee-decision__badge">
            {result.allowed ? "ALLOWED" : "REJECTED"}
          </div>
          <div className="ee-decision__reason">
            {humanizeReason(result.reason)}
          </div>

          <div className="ee-decision__grid">
            <div>
              <div className="ee-decision__label">Phase</div>
              <div className="ee-decision__value mono">{result.phase}</div>
            </div>
            <div>
              <div className="ee-decision__label">Drawdown</div>
              <div className="ee-decision__value mono">
                {fmtPct(result.dd_pct)}
              </div>
            </div>
            {result.f_hybrid != null && (
              <div>
                <div className="ee-decision__label">Hybrid floor</div>
                <div className="ee-decision__value mono">
                  {fmtMoney(result.f_hybrid)}
                </div>
              </div>
            )}
            {result.loss_capacity != null && (
              <div>
                <div className="ee-decision__label">Loss capacity</div>
                <div className="ee-decision__value mono">
                  {fmtMoney(result.loss_capacity)}
                </div>
              </div>
            )}
            <div>
              <div className="ee-decision__label">R allowed</div>
              <div className="ee-decision__value mono">{fmtMoney(result.r_allowed)}</div>
            </div>
            {result.size !== null && (
              <div>
                <div className="ee-decision__label">Size</div>
                <div className="ee-decision__value mono">
                  {parseFloat(result.size).toFixed(4)}
                </div>
              </div>
            )}
            {result.rr !== null && (
              <div>
                <div className="ee-decision__label">R:R</div>
                <div className="ee-decision__value mono">
                  {parseFloat(result.rr).toFixed(2)}
                </div>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
