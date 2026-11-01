import { useEffect, useState } from "react";
import {
  getStats,
  formatR,
  formatPct,
  formatPF,
  BacktestFilter,
  BacktestStats,
  parseR,
} from "../../lib/backtestApi";
import { EquityChart } from "./EquityChart";
import { RHistogram } from "./RHistogram";
import { SetupBreakdown } from "./SetupBreakdown";

interface Props {
  filter: BacktestFilter;
  refreshKey: number;
}

export function Stats({ filter, refreshKey }: Props) {
  const [stats, setStats] = useState<BacktestStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getStats(filter)
      .then((s) => {
        setStats(s);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  }, [filter, refreshKey]);

  if (error) return <div className="bt-error">{error}</div>;
  if (!stats) return <div className="bt-empty">Loading stats…</div>;

  const expR = parseR(stats.expectancy);
  const totR = parseR(stats.total_r);

  return (
    <section className="bt-stats">
      <div className="bt-stats__grid">
        <div className="bt-stat card">
          <div className="bt-stat__label">Trades</div>
          <div className="bt-stat__value mono">{stats.count}</div>
          <div className="bt-stat__sub">
            {stats.wins}W · {stats.losses}L
            {stats.breakeven > 0 ? ` · ${stats.breakeven}BE` : ""}
          </div>
        </div>
        <div className="bt-stat card">
          <div className="bt-stat__label">Win rate</div>
          <div className="bt-stat__value mono">{formatPct(stats.win_rate)}</div>
        </div>
        <div className="bt-stat card">
          <div className="bt-stat__label">Expectancy</div>
          <div
            className={`bt-stat__value mono ${
              expR === null ? "" : expR > 0 ? "pos" : expR < 0 ? "neg" : ""
            }`}
          >
            {formatR(stats.expectancy)}
            <span className="bt-stat__unit">R</span>
          </div>
        </div>
        <div className="bt-stat card">
          <div className="bt-stat__label">Profit factor</div>
          <div className="bt-stat__value mono">{formatPF(stats.profit_factor)}</div>
        </div>
        <div className="bt-stat card">
          <div className="bt-stat__label">Total R</div>
          <div
            className={`bt-stat__value mono ${
              totR === null ? "" : totR > 0 ? "pos" : totR < 0 ? "neg" : ""
            }`}
          >
            {formatR(stats.total_r)}
          </div>
        </div>
      </div>

      <div className="bt-charts">
        <div className="bt-chart card">
          <div className="bt-chart__title">Equity curve (cumulative R)</div>
          <EquityChart points={stats.equity} />
        </div>
        <div className="bt-chart card">
          <div className="bt-chart__title">R distribution</div>
          <RHistogram buckets={stats.histogram} />
        </div>
        <div className="bt-chart card bt-chart--wide">
          <div className="bt-chart__title">
            By setup <span className="bt-chart__hint">(min 5 trades)</span>
          </div>
          <SetupBreakdown rows={stats.by_setup} />
        </div>
      </div>
    </section>
  );
}
