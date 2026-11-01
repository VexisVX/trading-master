import { SetupStats, parseR, formatPct } from "../../lib/backtestApi";

interface Props {
  rows: SetupStats[];
}

export function SetupBreakdown({ rows }: Props) {
  if (rows.length === 0) {
    return (
      <div className="bt-empty bt-empty--chart">
        Need at least 5 trades per setup to show breakdown.
      </div>
    );
  }

  const maxAbs = Math.max(
    0.01,
    ...rows.map((r) => Math.abs(parseR(r.avg_r) ?? 0))
  );

  return (
    <div className="bt-setup-bars">
      {rows.map((r) => {
        const avg = parseR(r.avg_r) ?? 0;
        const pct = (Math.abs(avg) / maxAbs) * 100;
        const isPos = avg >= 0;
        return (
          <div className="bt-setup-bar" key={r.tag}>
            <div className="bt-setup-bar__label">
              <span className="bt-chip">{r.tag}</span>
              <span className="bt-muted bt-setup-bar__meta">
                {r.count} trades · {formatPct(r.win_rate)}
              </span>
            </div>
            <div className="bt-setup-bar__track">
              <div
                className={`bt-setup-bar__fill ${isPos ? "pos" : "neg"}`}
                style={{ width: `${pct}%` }}
              />
            </div>
            <div className={`bt-setup-bar__val mono ${isPos ? "pos" : "neg"}`}>
              {avg >= 0 ? "+" : ""}
              {avg.toFixed(2)}R
            </div>
          </div>
        );
      })}
    </div>
  );
}
