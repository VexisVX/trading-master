import { useMemo, useState } from "react";
import {
  DecisionRowOut,
  DecisionVerdict,
  decisionVerdict,
  fmtMoney,
  humanizeReason,
  verdictIcon,
  verdictLabel,
  verdictTone,
} from "../../lib/equityApi";

interface Props {
  decisions: DecisionRowOut[];
  onClose: () => void;
}

type Filter = "all" | DecisionVerdict;

export function AllDecisionsModal({ decisions, onClose }: Props) {
  const [filter, setFilter] = useState<Filter>("all");

  const counts = useMemo(() => {
    const c = { all: decisions.length, allowed: 0, reduced: 0, rejected: 0, paused: 0 };
    for (const d of decisions) {
      c[decisionVerdict(d)]++;
    }
    return c;
  }, [decisions]);

  const filtered = useMemo(() => {
    if (filter === "all") return decisions;
    return decisions.filter((d) => decisionVerdict(d) === filter);
  }, [decisions, filter]);

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div
        className="ee-modal ee-modal--wide"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Decision History</h3>
            <p className="ee-modal__subtitle">
              {decisions.length} total decisions
            </p>
          </div>
          <button
            className="ee-modal__close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        <nav className="ee-modal__tabs">
          {(
            [
              ["all", `All (${counts.all})`],
              ["allowed", `Allowed (${counts.allowed})`],
              ["reduced", `Reduced (${counts.reduced})`],
              ["rejected", `Rejected (${counts.rejected})`],
              ["paused", `Paused (${counts.paused})`],
            ] as [Filter, string][]
          ).map(([id, label]) => (
            <button
              key={id}
              className={`ee-modal__tab ${filter === id ? "is-active" : ""}`}
              onClick={() => setFilter(id)}
            >
              {label}
            </button>
          ))}
        </nav>

        <div className="ee-modal__body ee-modal__body--table">
          {filtered.length === 0 ? (
            <div className="ee-empty-inline">
              No decisions match this filter.
            </div>
          ) : (
            <table className="ee-table">
              <thead>
                <tr>
                  <th>Time</th>
                  <th>Result</th>
                  <th>Reason</th>
                  <th>Risk</th>
                  <th>Size</th>
                  <th>Phase</th>
                </tr>
              </thead>
              <tbody>
                {filtered.map((d) => {
                  const v = decisionVerdict(d);
                  const tone = verdictTone(v);
                  return (
                    <tr key={d.id}>
                      <td className="mono ee-muted">
                        {new Date(d.created_at).toLocaleString()}
                      </td>
                      <td>
                        <span className={`ee-chip ee-chip--${tone}`}>
                          {verdictIcon(v)} {verdictLabel(v)}
                        </span>
                      </td>
                      <td className="ee-muted">
                        {humanizeReason({ kind: d.reason_kind })}
                      </td>
                      <td className="mono">{fmtMoney(d.r_allowed)}</td>
                      <td className="mono">
                        {d.size !== null ? parseFloat(d.size).toFixed(2) : "—"}
                      </td>
                      <td className="mono ee-muted">{d.phase || "—"}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}
        </div>

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
