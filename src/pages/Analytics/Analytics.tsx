import { useEffect, useState } from "react";
import { getBreakdown, Breakdown, GroupMetrics, getSummary, Summary } from "../../lib/analyticsApi";
import { getActiveAccountId } from "../../lib/accountsApi";
import "./Analytics.css";

type DateRange = "all" | "month" | "3m" | "year" | "custom";

const money = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n)
    ? `${n >= 0 ? "+" : ""}${n.toLocaleString(undefined, { maximumFractionDigits: 2 })}`
    : v;
};
const pct = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n) ? `${n.toFixed(1)}%` : v;
};
const rr = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n) ? `${n >= 0 ? "+" : ""}${n.toFixed(2)}R` : v;
};

function rangeToDates(range: DateRange, customFrom: string, customTo: string): { from?: string; to?: string } {
  const now = new Date();
  const today = now.toISOString().slice(0, 10);
  if (range === "all") return {};
  if (range === "custom") {
    return { from: customFrom || undefined, to: customTo || undefined };
  }
  const days = range === "month" ? 30 : range === "3m" ? 90 : 365;
  const from = new Date(now.getTime() - days * 86400000).toISOString().slice(0, 10);
  return { from, to: today };
}

export function Analytics() {
  const [b, setB] = useState<Breakdown | null>(null);
  const [sum, setSum] = useState<Summary | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [range, setRange] = useState<DateRange>("all");
  const [customFrom, setCustomFrom] = useState("");
  const [customTo, setCustomTo] = useState("");

  async function refresh() {
    setLoading(true); setError(null);
    try {
      const id = await getActiveAccountId();
      const { from, to } = rangeToDates(range, customFrom, customTo);
      const [data, summary] = await Promise.all([
        getBreakdown(id, from, to),
        getSummary(),
      ]);
      setB(data);
      setSum(summary);
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }

  useEffect(() => { refresh(); }, [range, customFrom, customTo]);

  if (loading) return <div className="card">Loading…</div>;
  if (error) return <div className="card card--error">{error}</div>;
  if (!b || b.total_closed === 0) {
    return (
      <div className="card">
        <div className="empty-state">
          <span className="empty-state__icon">▥</span>
          <span className="empty-state__title">No closed trades in this range</span>
          <span className="empty-state__hint">
            Close some trades in the Journal, or adjust the date range.
          </span>
        </div>
      </div>
    );
  }

  // Outcome donut
  const outcomeTotal = b.tp_hit_count + b.sl_hit_count + b.breakeven_count + b.manual_count;
  const tpPct = outcomeTotal > 0 ? (b.tp_hit_count / outcomeTotal) * 100 : 0;
  const slPct = outcomeTotal > 0 ? (b.sl_hit_count / outcomeTotal) * 100 : 0;
  const bePct = outcomeTotal > 0 ? (b.breakeven_count / outcomeTotal) * 100 : 0;
  const mPct = outcomeTotal > 0 ? (b.manual_count / outcomeTotal) * 100 : 0;

  return (
    <div className="analytics">
      {/* Date range */}
      <div className="ana-toolbar">
        <div className="ana-seg">
          {(["all", "month", "3m", "year", "custom"] as const).map((r) => (
            <button key={r} className={`ana-btn ${range === r ? "is-active" : ""}`} onClick={() => setRange(r)}>
              {r === "all" ? "All" : r === "month" ? "1M" : r === "3m" ? "3M" : r === "year" ? "1Y" : "Custom"}
            </button>
          ))}
        </div>
        {range === "custom" && (
          <div className="ana-dates">
            <input type="date" value={customFrom} onChange={(e) => setCustomFrom(e.target.value)} />
            <span className="dim">→</span>
            <input type="date" value={customTo} onChange={(e) => setCustomTo(e.target.value)} />
          </div>
        )}
        <button className="btn btn--ghost btn--small" onClick={refresh}>↻</button>
      </div>

      {/* Top KPI row */}
      <div className="ana-kpi-row">
        <Kpi label="Closed trades" value={b.total_closed.toString()} />
        <Kpi label="Planned" value={b.planned_trades.toString()} />
        <Kpi label="Adherence" value={pct(b.plan_adherence_pct)} accent />
        {sum && <Kpi label="Net P&L" value={money(sum.net_pnl)} />}
      </div>

      {/* Outcome breakdown */}
      <div className="card">
        <div className="card__head">
          <h3>Outcome breakdown</h3>
          <span className="card__badge">{outcomeTotal} closed</span>
        </div>
        <div className="ana-outcome-bar">
          {tpPct > 0 && <div className="ana-outcome-seg ana-outcome-seg--tp" style={{ width: `${tpPct}%` }} title={`TP: ${b.tp_hit_count}`} />}
          {slPct > 0 && <div className="ana-outcome-seg ana-outcome-seg--sl" style={{ width: `${slPct}%` }} title={`SL: ${b.sl_hit_count}`} />}
          {bePct > 0 && <div className="ana-outcome-seg ana-outcome-seg--be" style={{ width: `${bePct}%` }} title={`BE: ${b.breakeven_count}`} />}
          {mPct > 0 && <div className="ana-outcome-seg ana-outcome-seg--manual" style={{ width: `${mPct}%` }} title={`Manual: ${b.manual_count}`} />}
        </div>
        <div className="ana-outcome-legend">
          <Legend label="TP hit" count={b.tp_hit_count} color="var(--success)" />
          <Legend label="SL hit" count={b.sl_hit_count} color="var(--danger)" />
          <Legend label="Breakeven" count={b.breakeven_count} color="var(--warning)" />
          <Legend label="Manual" count={b.manual_count} color="var(--text-2)" />
        </div>
      </div>

      <GroupSection title="By Strategy" rows={b.by_strategy} emptyText="No strategy-linked trades." />
      <GroupSection title="By Month" rows={b.by_month} emptyText="No monthly data." />
      <div className="ana-row">
        <GroupSection title="By Weekday" rows={b.by_weekday} emptyText="No weekday data." />
        <GroupSection title="By Session" rows={b.by_session} emptyText="No session data." />
      </div>
    </div>
  );
}

function Kpi({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="card ana-kpi">
      <span className="ana-kpi__label">{label}</span>
      <span className={`ana-kpi__value mono ${accent ? "is-accent" : ""}`}>{value}</span>
    </div>
  );
}

function Legend({ label, count, color }: { label: string; count: number; color: string }) {
  return (
    <div className="ana-legend">
      <span className="ana-legend__dot" style={{ background: color }} />
      <span>{label}</span>
      <span className="mono dim">{count}</span>
    </div>
  );
}

function GroupSection({ title, rows, emptyText }: {
  title: string;
  rows: GroupMetrics[];
  emptyText: string;
}) {
  return (
    <section className="card">
      <div className="card__head">
        <h3>{title}</h3>
        <span className="card__badge">{rows.length} groups</span>
      </div>
      {rows.length === 0 ? (
        <div className="dash__placeholder">{emptyText}</div>
      ) : (
        <table className="trades breakdown-table">
          <thead>
            <tr>
              <th>Group</th>
              <th>Trades</th>
              <th>Win %</th>
              <th>Net</th>
              <th>PF</th>
              <th>Expectancy</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((g) => (
              <tr key={g.key}>
                <td>{g.label}</td>
                <td className="mono">{g.trades}</td>
                <td className="mono">{pct(g.win_rate_pct)}</td>
                <td className={`mono ${Number(g.net_pnl) >= 0 ? "pos" : "neg"}`}>{money(g.net_pnl)}</td>
                <td className="mono">{Number(g.profit_factor).toFixed(2)}</td>
                <td className={`mono ${Number(g.expectancy_r) >= 0 ? "pos" : "neg"}`}>{rr(g.expectancy_r)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
