import {
  DecisionRowOut,
  decisionVerdict,
  fmtMoney,
  humanizeReason,
  verdictIcon,
  verdictLabel,
  verdictTone,
} from "../../lib/equityApi";

interface Props {
  decisions: DecisionRowOut[];
  onViewAll: () => void;
}

export function RecentDecisionsCard({ decisions, onViewAll }: Props) {
  const recent = decisions.slice(0, 5);
  const today = new Date().toISOString().slice(0, 10);
  const todayDecisions = decisions.filter((d) =>
    d.created_at.startsWith(today)
  );

  const counts = {
    allowed: todayDecisions.filter((d) => decisionVerdict(d) === "allowed").length,
    reduced: todayDecisions.filter((d) => decisionVerdict(d) === "reduced").length,
    rejected: todayDecisions.filter((d) => decisionVerdict(d) === "rejected").length,
    paused: todayDecisions.filter((d) => decisionVerdict(d) === "paused").length,
  };

  const summary = todayDecisions.length === 0
    ? "No decisions yet today"
    : [
        counts.allowed > 0 && `${counts.allowed} allowed`,
        counts.reduced > 0 && `${counts.reduced} reduced`,
        counts.rejected > 0 && `${counts.rejected} rejected`,
        counts.paused > 0 && `${counts.paused} paused`,
      ]
        .filter(Boolean)
        .join(" · ");

  return (
    <section className="ee-card ee-recent">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Recent Decisions</h3>
          <p className="ee-card__subtitle">
            {todayDecisions.length > 0 ? `Today · ${summary}` : summary}
          </p>
        </div>
        {decisions.length > 5 && (
          <button className="ee-btn ee-btn--ghost" onClick={onViewAll}>
            View all →
          </button>
        )}
      </div>

      {recent.length === 0 ? (
        <div className="ee-empty-inline">
          No decisions yet. Evaluate a trade above to start.
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
            </tr>
          </thead>
          <tbody>
            {recent.map((d) => {
              const v = decisionVerdict(d);
              const tone = verdictTone(v);
              return (
                <tr key={d.id}>
                  <td className="mono ee-muted">
                    {new Date(d.created_at).toLocaleTimeString([], {
                      hour: "2-digit",
                      minute: "2-digit",
                    })}
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
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </section>
  );
}
