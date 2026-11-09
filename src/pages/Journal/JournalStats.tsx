import { useMemo } from "react";
import { Trade } from "../../lib/tradesApi";

interface Props {
  trades: Trade[];
}

function sessionOf(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "Asia";
  const hour = d.getUTCHours();
  if (hour < 8) return "Asia";
  if (hour < 16) return "London";
  return "New York";
}

export function JournalStats({ trades }: Props) {
  const stats = useMemo(() => {
    try {
      const closed = (trades ?? []).filter((t) => t && t.status === "closed");
      if (closed.length === 0) return null;

      const symCount: Record<string, number> = {};
      const symR: Record<string, number> = {};
      const sessR: Record<string, number> = {};
      const dayR: Record<string, number> = {};
      const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

      let wins = 0;
      let losses = 0;
      let totalR = 0;
      let grossWin = 0;
      let grossLoss = 0;

      for (const t of closed) {
        const sym = t.symbol || "?";
        symCount[sym] = (symCount[sym] || 0) + 1;
        const r = Number(t.manual_pnl ?? 0) || 0;
        symR[sym] = (symR[sym] || 0) + r;

        totalR += r;
        if (r > 0) { wins++; grossWin += r; }
        else if (r < 0) { losses++; grossLoss += Math.abs(r); }

        if (t.closed_at) {
          const sess = sessionOf(t.closed_at);
          sessR[sess] = (sessR[sess] || 0) + r;
          const d = new Date(t.closed_at);
          if (!isNaN(d.getTime())) {
            const day = dayNames[d.getDay()] || "?";
            dayR[day] = (dayR[day] || 0) + r;
          }
        }
      }

      const pick = (m: Record<string, number>, fb: [string, number]) =>
        Object.entries(m).sort((a, b) => b[1] - a[1])[0] ?? fb;

      const winRate = (wins / closed.length) * 100;
      const expectancy = totalR / closed.length;
      const profitFactor = grossLoss > 0 ? grossWin / grossLoss : (grossWin > 0 ? Infinity : 0);

      return {
        count: closed.length,
        wins,
        losses,
        winRate,
        expectancy,
        profitFactor,
        mostTraded: pick(symCount, ["—", 0]),
        mostProfitable: pick(symR, ["—", 0]),
        bestSession: pick(sessR, ["—", 0]),
        bestDay: pick(dayR, ["—", 0]),
      };
    } catch (e) {
      console.error("JournalStats:", e);
      return null;
    }
  }, [trades]);

  if (!stats) {
    return (
      <div className="card journal-stats">
        <div className="journal-stats__head"><h3>Quick Stats</h3></div>
        <div className="journal-stats__empty">Stats appear after your first closed trade.</div>
      </div>
    );
  }

  const fmtR = (n: number) => `${n >= 0 ? "+" : ""}${n.toFixed(2)}R`;
  const pf = stats.profitFactor === Infinity ? "∞" : stats.profitFactor.toFixed(2);
  const wrClass = stats.winRate >= 50 ? "pos" : "neg";
  const expClass = stats.expectancy >= 0 ? "pos" : "neg";
  const pfClass = stats.profitFactor >= 1 ? "pos" : "neg";

  return (
    <div className="card journal-stats">
      <div className="journal-stats__head">
        <h3>Quick Stats</h3>
        <span className="card__badge">{stats.count} closed</span>
      </div>

      {/* Headline numbers — what the user actually wants to know */}
      <div className="journal-stats__headline">
        <div className="headline-item">
          <span className="headline-label">Win rate</span>
          <span className={`headline-value ${wrClass}`}>{stats.winRate.toFixed(1)}%</span>
          <span className="headline-sub">{stats.wins}W · {stats.losses}L</span>
        </div>
        <div className="headline-item">
          <span className="headline-label">Expectancy</span>
          <span className={`headline-value ${expClass}`}>{fmtR(stats.expectancy)}</span>
          <span className="headline-sub">per trade</span>
        </div>
        <div className="headline-item">
          <span className="headline-label">Profit factor</span>
          <span className={`headline-value ${pfClass}`}>{pf}</span>
          <span className="headline-sub">gross win / loss</span>
        </div>
      </div>

      <div className="journal-stats__grid">
        <div className="stats-item">
          <span className="stats-item__label">Most traded</span>
          <span className="stats-item__value">
            {stats.mostTraded[0]}
            <span className="stats-item__hint">{stats.mostTraded[1]} trades</span>
          </span>
        </div>
        <div className="stats-item">
          <span className="stats-item__label">Most profitable</span>
          <span className={`stats-item__value ${stats.mostProfitable[1] >= 0 ? "pos" : "neg"}`}>
            {stats.mostProfitable[0]}
            <span className="stats-item__hint">{fmtR(stats.mostProfitable[1])}</span>
          </span>
        </div>
        <div className="stats-item">
          <span className="stats-item__label">Best session</span>
          <span className={`stats-item__value ${stats.bestSession[1] >= 0 ? "pos" : "neg"}`}>
            {stats.bestSession[0]}
            <span className="stats-item__hint">{fmtR(stats.bestSession[1])}</span>
          </span>
        </div>
        <div className="stats-item">
          <span className="stats-item__label">Best day</span>
          <span className={`stats-item__value ${stats.bestDay[1] >= 0 ? "pos" : "neg"}`}>
            {stats.bestDay[0]}
            <span className="stats-item__hint">{fmtR(stats.bestDay[1])}</span>
          </span>
        </div>
      </div>
    </div>
  );
}
