import { useMemo, useState } from "react";
import {
  DecisionOut,
  evaluateTrade,
  fmtMoney,
  fmtPct,
  humanizeReason,
  reasonDetail,
} from "../../lib/equityApi";

interface Props {
  hasSnapshot: boolean;
  allowedRisk: number;      // ← from parent (equity × r0)
  onOpenWhyThisSize: (result: DecisionOut) => void;
}

export function NewTradeCard({
  hasSnapshot,
  allowedRisk,
  onOpenWhyThisSize,
}: Props) {
  const [entry, setEntry] = useState("1.1000");
  const [stop, setStop] = useState("1.0950");
  const [tp, setTp] = useState("1.1150");
  const [perLotLoss, setPerLotLoss] = useState("500");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<DecisionOut | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);

  // Compute implied multiplier from per-lot loss (UI only)
  const implied = useMemo(() => {
    const e = parseFloat(entry);
    const s = parseFloat(stop);
    const p = parseFloat(perLotLoss);
    if (!isFinite(e) || !isFinite(s) || !isFinite(p) || e === s || p <= 0) {
      return null;
    }
    return {
      diff: Math.abs(e - s),
      multiplier: p / Math.abs(e - s),
    };
  }, [entry, stop, perLotLoss]);

  // Max size we could take with current allowed risk
  const maxSize = useMemo(() => {
    const p = parseFloat(perLotLoss);
    if (!isFinite(p) || p <= 0) return null;
    return allowedRisk / p;
  }, [perLotLoss, allowedRisk]);

  async function evaluate(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setResult(null);
    if (!implied) {
      setError("Enter valid entry, stop, and per-lot loss.");
      return;
    }
    setBusy(true);
    try {
      const r = await evaluateTrade({
        entry,
        stop,
        take_profit: tp || null,
        contract_multiplier: implied.multiplier.toFixed(10),
        commission_per_unit: "0",
        slippage_buffer: "0",
      });
      setResult(r);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="ee-card ee-new-trade">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Evaluate a Trade</h3>
          <p className="ee-card__subtitle">
            You risk up to <strong>{fmtMoney(allowedRisk)}</strong> right now —
            enter the trade details
          </p>
        </div>
      </div>

      <form onSubmit={evaluate}>
        {/* Row 1: entry / stop / target */}
        <div className="ee-form-row">
          <label className="ee-field">
            <span>Entry</span>
            <input
              className="mono"
              value={entry}
              onChange={(e) => setEntry(e.target.value)}
              placeholder="0.00"
            />
          </label>
          <label className="ee-field">
            <span>Stop</span>
            <input
              className="mono"
              value={stop}
              onChange={(e) => setStop(e.target.value)}
              placeholder="0.00"
            />
          </label>
          <label className="ee-field">
            <span>Target (optional)</span>
            <input
              className="mono"
              value={tp}
              onChange={(e) => setTp(e.target.value)}
              placeholder="0.00"
            />
          </label>
        </div>

        {/* Row 2: per-lot loss — the only "new" question */}
        <div className="ee-form-row">
          <label className="ee-field" style={{ gridColumn: "1 / -1" }}>
            <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
              💰 If 1 lot hits the stop, I lose (USD)
              <button
                type="button"
                className="ee-btn ee-btn--ghost"
                style={{ padding: "2px 8px", fontSize: 11 }}
                onClick={() => setShowHelp((v) => !v)}
              >
                {showHelp ? "Hide" : "How?"}
              </button>
            </span>
            <input
              className="mono"
              value={perLotLoss}
              onChange={(e) => setPerLotLoss(e.target.value)}
              placeholder="e.g. 500"
              style={{ fontSize: 15, padding: "10px 12px" }}
            />
          </label>
        </div>

        {showHelp && (
          <div
            style={{
              padding: "12px 14px",
              background: "var(--bg-0)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              fontSize: 12,
              lineHeight: 1.7,
              marginBottom: 14,
            }}
          >
            <div style={{ marginBottom: 6 }}>
              <strong>From your broker:</strong> open a paper trade with the
              same entry and stop, size <strong>1.00 lot</strong>. The
              "Potential loss" or "Risk" value shown is this number.
            </div>
            <div className="ee-muted" style={{ marginTop: 8 }}>
              <strong>Or approximate:</strong> loss ≈ |entry − stop| × contract
              size. For EURUSD: × 100,000. For BTCUSD: × 1. For indices: × 1.
            </div>
          </div>
        )}

        {/* Preview: max size if allowed risk is used */}
        {maxSize !== null && (
          <div
            style={{
              padding: "10px 14px",
              background: "var(--bg-0)",
              border: "1px solid var(--border)",
              borderRadius: 6,
              fontSize: 12,
              marginBottom: 14,
            }}
          >
            <div className="ee-muted" style={{ fontSize: 11, marginBottom: 4 }}>
              PREVIEW (UI)
            </div>
            <div>
              Max size at allowed risk{" "}
              <strong>{fmtMoney(allowedRisk)}</strong> ÷ per-lot{" "}
              <strong>{fmtMoney(parseFloat(perLotLoss) || 0)}</strong> ={" "}
              <strong className="mono">
                {maxSize.toFixed(2)} lots
              </strong>
            </div>
          </div>
        )}

        {!hasSnapshot && (
          <div className="ee-alert ee-alert--warn">
            No snapshot yet — click <strong>⟳ Refresh</strong> on Account Status
            first.
          </div>
        )}
        {error && <div className="ee-alert ee-alert--error">{error}</div>}

        <div className="ee-form-actions">
          <button
            className="ee-btn ee-btn--primary"
            disabled={busy || !hasSnapshot || !implied}
          >
            {busy ? "…" : "Evaluate Trade"}
          </button>
        </div>
      </form>

      {result && <ResultPanel result={result} onOpenWhy={onOpenWhyThisSize} />}
    </section>
  );
}

function ResultPanel({
  result,
  onOpenWhy,
}: {
  result: DecisionOut;
  onOpenWhy: (r: DecisionOut) => void;
}) {
  const tone = result.allowed ? "allowed" : "rejected";
  const label = result.allowed ? "ALLOWED" : "REJECTED";
  const detail = reasonDetail(result.reason);

  return (
    <div className={`ee-result ee-result--${tone}`}>
      <div className="ee-result__header">
        <span className="ee-result__badge">{label}</span>
        <span className="ee-result__reason">
          {humanizeReason(result.reason)}
        </span>
      </div>

      {result.allowed && result.size && (
        <div className="ee-result__primary">
          <div className="ee-result__primary-label">Position size</div>
          <div className="ee-result__primary-value">
            {parseFloat(result.size).toFixed(2)} lots
          </div>
        </div>
      )}

      <div className="ee-result__grid">
        <div>
          <div className="ee-result__label">Allowed risk</div>
          <div className="ee-result__value mono">
            {fmtMoney(result.r_allowed)}
          </div>
        </div>
        <div>
          <div className="ee-result__label">Drawdown</div>
          <div className="ee-result__value mono">{fmtPct(result.dd_pct)}</div>
        </div>
        {result.rr !== null && (
          <div>
            <div className="ee-result__label">R : R</div>
            <div className="ee-result__value mono">
              1 : {parseFloat(result.rr).toFixed(2)}
            </div>
          </div>
        )}
        <div>
          <div className="ee-result__label">Phase</div>
          <div className="ee-result__value mono">{result.phase}</div>
        </div>
        {detail && (
          <div style={{ gridColumn: "1 / -1" }}>
            <div className="ee-result__label">Details</div>
            <div className="ee-result__value mono">
              {Object.entries(detail).map(([k, v]) => (
                <span key={k} style={{ marginRight: 12 }}>
                  {k}: <strong>{String(v)}</strong>
                </span>
              ))}
            </div>
          </div>
        )}
      </div>

      <div className="ee-result__footer">
        <button
          className="ee-btn ee-btn--ghost"
          onClick={() => onOpenWhy(result)}
        >
          Why this size? →
        </button>
      </div>
    </div>
  );
}
