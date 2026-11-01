import { DecisionRowOut, fmtMoney } from "../../lib/equityApi";

interface Props {
  decisions: DecisionRowOut[];
}

export function AuditTab({ decisions }: Props) {
  if (decisions.length === 0) {
    return (
      <div className="ee-empty card">
        No decisions yet. Go to Decision tab and evaluate a trade.
      </div>
    );
  }

  return (
    <div className="ee-audit card">
      <table className="ee-table">
        <thead>
          <tr>
            <th>Time</th>
            <th>Status</th>
            <th>Reason</th>
            <th>R allowed</th>
            <th>Size</th>
            <th>Snapshot</th>
          </tr>
        </thead>
        <tbody>
          {decisions.map((d) => (
            <tr key={d.id}>
              <td className="mono ee-muted">
                {new Date(d.created_at).toLocaleString()}
              </td>
              <td>
                <span
                  className={`ee-chip ${
                    d.allowed ? "ee-chip--ok" : "ee-chip--bad"
                  }`}
                >
                  {d.allowed ? "Allowed" : "Rejected"}
                </span>
              </td>
              <td className="ee-muted">{d.reason_kind}</td>
              <td className="mono">{fmtMoney(d.r_allowed)}</td>
              <td className="mono">
                {d.size !== null ? parseFloat(d.size).toFixed(4) : "—"}
              </td>
              <td className="mono ee-muted">#{d.snapshot_id}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
