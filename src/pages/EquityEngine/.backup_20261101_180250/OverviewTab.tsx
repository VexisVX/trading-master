import { useMemo } from "react";
import {
  EquitySnapshotRow,
  fmtMoney,
  parseDec,
} from "../../lib/equityApi";

interface Props {
  snapshot: EquitySnapshotRow | null;
  onGoSnapshot: () => void;
}

function Metric({
  label,
  value,
  sub,
  tone,
}: {
  label: string;
  value: string;
  sub?: string;
  tone?: "pos" | "neg" | "warn" | "";
}) {
  return (
    <div className="ee-metric card">
      <div className="ee-metric__label">{label}</div>
      <div className={`ee-metric__value mono ${tone ?? ""}`}>{value}</div>
      {sub && <div className="ee-metric__sub">{sub}</div>}
    </div>
  );
}

export function OverviewTab({ snapshot, onGoSnapshot }: Props) {
  const stats = useMemo(() => {
    if (!snapshot) return null;
    const eq = parseDec(snapshot.equity) ?? 0;
    const hwm = parseDec(snapshot.high_watermark) ?? 0;
    const init = parseDec(snapshot.initial_balance) ?? 0;
    const dd = hwm > 0 ? (hwm - eq) / hwm : 0;
    const netProfit = eq - init;
    return { eq, hwm, init, dd, netProfit };
  }, [snapshot]);

  if (!snapshot) {
    return (
      <div className="ee-empty card">
        <div className="ee-empty__title">No snapshot yet</div>
        <div className="ee-empty__msg">
          The Equity Engine needs a real equity snapshot from your broker to
          make any decision. Create the first one to get started.
        </div>
        <button className="btn btn--primary" onClick={onGoSnapshot}>
          Create snapshot
        </button>
      </div>
    );
  }

  return (
    <div className="ee-overview">
      <div className="ee-metrics-grid">
        <Metric
          label="Equity"
          value={fmtMoney(snapshot.equity)}
          sub={`Initial: ${fmtMoney(snapshot.initial_balance)}`}
        />
        <Metric
          label="High Watermark"
          value={fmtMoney(snapshot.high_watermark)}
          sub="All-time peak"
        />
        <Metric
          label="Drawdown"
          value={stats ? (stats.dd * 100).toFixed(2) + "%" : "—"}
          sub={
            stats && stats.dd > 0
              ? `${fmtMoney(String(stats.hwm - stats.eq))} below peak`
              : "At peak"
          }
          tone={stats ? (stats.dd > 0.1 ? "neg" : stats.dd > 0.05 ? "warn" : "pos") : ""}
        />
        <Metric
          label="Net Profit"
          value={
            stats
              ? (stats.netProfit >= 0 ? "+" : "") +
                fmtMoney(String(stats.netProfit))
              : "—"
          }
          tone={
            stats ? (stats.netProfit >= 0 ? "pos" : "neg") : ""
          }
        />
        <Metric
          label="Consecutive Losses"
          value={String(snapshot.consecutive_losses)}
          sub="Streak"
          tone={snapshot.consecutive_losses >= 3 ? "neg" : ""}
        />
        <Metric
          label="Snapshot Age"
          value={timeAgo(snapshot.taken_at)}
          sub={new Date(snapshot.taken_at).toLocaleString()}
        />
      </div>

      <div className="ee-info card">
        <div className="ee-info__title">Notes</div>
        <ul className="ee-info__list">
          <li>
            Equity shown here is the real snapshot from your broker — not
            computed from closed trades.
          </li>
          <li>
            Decisions use this snapshot with its timestamp. If it gets older
            than the configured max age, decisions are blocked.
          </li>
          <li>
            Every decision is logged in the Audit tab with the full config
            used at the time.
          </li>
        </ul>
      </div>
    </div>
  );
}

function timeAgo(iso: string): string {
  const then = new Date(iso).getTime();
  if (!Number.isFinite(then)) return "—";
  const sec = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (sec < 60) return `${sec}s ago`;
  const min = Math.floor(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr}h ago`;
  return `${Math.floor(hr / 24)}d ago`;
}
