import { useEffect, useState } from "react";
import { LineChart } from "../../components/charts/LineChart";
import { BarChart } from "../../components/charts/BarChart";
import { Donut } from "../../components/charts/Donut";
import { getSummary, getPeriodPnl, Summary, TradeRDetail, PeriodPnl } from "../../lib/analyticsApi";
import { getGoals, setGoals, AccountGoals } from "../../lib/goalsApi";

import { getRiskStatus, RiskStatus } from "../../lib/riskSettingsApi";
import { getPropFirmStatus, PropFirmStatus } from "../../lib/propFirmApi";
import { getActiveAccountId } from "../../lib/accountsApi";
import "./Dashboard.css";

// ============================================================
// Formatters
// ============================================================
const fmtMoney = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n)
    ? `$${n.toLocaleString(undefined, { maximumFractionDigits: 2 })}`
    : v;
};
const fmtPct = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n) ? `${n.toFixed(1)}%` : v;
};
const fmtR = (v: string) => {
  const n = Number(v);
  return Number.isFinite(n) ? `${n >= 0 ? "+" : ""}${n.toFixed(2)}R` : v;
};

// ============================================================
// Motivation messages
// ============================================================
const MOTIVATION_MESSAGES = [
  { min: 1000, msg: "Extraordinary. Discipline is your greatest asset." },
  { min: 500,  msg: "Strong performance. Consistency compounds quietly." },
  { min: 200,  msg: "Well earned. Keep following your plan, not your emotions." },
  { min: 50,   msg: "You are green. Stay patient — the market rewards restraint." },
  { min: 1,    msg: "Small steps forward. Trust your process." },
  { min: -50,  msg: "A pause is not a failure. Review, then continue." },
  { min: -200, msg: "Protect your capital. The next trade is a fresh start." },
  { min: -500, msg: "Slow down. Reduce size, rebuild clarity." },
  { min: -1000,msg: "A losing streak is not a losing trader. Breathe." },
  { min: -999999, msg: "Stop. Rest. Tomorrow is a new opportunity." },
];

// ============================================================
// Main Dashboard
// ============================================================
export function Dashboard() {
  const [s, setS] = useState<Summary | null>(null);
const [risk, setRisk] = useState<RiskStatus | null>(null);
  const [pf, setPf] = useState<PropFirmStatus | null>(null);
  const [goals, setGoalsState] = useState<AccountGoals | null>(null);
  const [periodPnl, setPeriodPnl] = useState<PeriodPnl | null>(null);
  const [showGoalsEdit, setShowGoalsEdit] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function load() {
    setError(null);
    try {
      const accId = await getActiveAccountId();
      const summary = await getSummary(accId);
      setS(summary);
      try { setGoalsState(await getGoals(accId)); } catch { setGoalsState(null); }
      try { setPeriodPnl(await getPeriodPnl(accId)); } catch { setPeriodPnl(null); }
try { setRisk(await getRiskStatus(accId)); } catch { setRisk(null); }
      try { setPf(await getPropFirmStatus(accId)); } catch { setPf(null); }
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    (async () => {
      setLoading(true);
      await load();
      setLoading(false);
    })();
  }, []);

  // ★ Listen to account changes — reload page for fresh data
  useEffect(() => {
    const handler = () => window.location.reload();
    window.addEventListener("activeAccountChanged", handler);
    return () => window.removeEventListener("activeAccountChanged", handler);
  }, []);

  if (loading) return <div className="dash"><div className="card">Loading…</div></div>;
  if (error) return <div className="dash"><div className="card">Error: {error}</div></div>;
  if (!s) return <div className="dash"><div className="card">No data</div></div>;

  const equitySeries = s.equity_curve.map((p) => ({ t: p.t, v: Number(p.v) }));
  const drawdownSeries = s.drawdown_curve.map((p) => ({ t: p.t, v: Number(p.v) }));
  const rBars = s.r_distribution.map((b) => ({ label: b.label.replace("R", ""), v: b.count }));
  const pnlPositive = Number(s.net_pnl) >= 0;

  return (
    <div className="dash">
      <div className="dash__header">
        <div>
          <h1 className="dash__title">Dashboard</h1>
          <p className="dash__subtitle">
            Live overview · {s.total_trades} trade{s.total_trades === 1 ? "" : "s"}
            {s.open_trades > 0 && ` · ${s.open_trades} open`}
          </p>
        </div>
        <button className="dash__refresh" onClick={load}>
          <span className="dash__refresh-icon">↻</span>
          <span>Refresh</span>
        </button>
      </div>

      {/* KPI Cards */}
      <section className="dash__grid">
        <div className="card stat">
          <span className="stat__label">Balance</span>
          <span className="stat__value stat__value--accent mono">{fmtMoney(s.current_equity)}</span>
          <span className="stat__hint">Initial {fmtMoney(s.initial_balance)}</span>
        </div>
        <div className="card stat">
          <span className="stat__label">Net P&L</span>
          <span className={`stat__value ${pnlPositive ? "stat__value--success" : "stat__value--danger"} mono`}>
            {fmtMoney(s.net_pnl)}
          </span>
          <span className="stat__hint">{s.closed_trades} closed trades</span>
        </div>
        <div className="card stat">
          <span className="stat__label">Win Rate</span>
          <span className="stat__value mono">{s.closed_trades > 0 ? fmtPct(s.win_rate_pct) : "—"}</span>
          <span className="stat__hint">{s.wins}W / {s.losses}L</span>
        </div>
        <div className="card stat">
          <span className="stat__label">Open Positions</span>
          <span className="stat__value mono">{s.open_trades}</span>
          <span className="stat__hint">{s.open_trades === 0 ? "No open trades" : "Currently running"}</span>
        </div>
      </section>

      {/* Equity + Drawdown */}
      <section className="dash__row dash__row--2">
        <div className="card">
          <div className="card__head">
            <h3>Equity Curve</h3>
            <span className="card__badge">{s.closed_trades} trades</span>
          </div>
          {equitySeries.length > 1 ? (
            <LineChart data={equitySeries} formatY={(v) => `$${Math.round(v).toLocaleString()}`} height={220} />
          ) : (
            <div className="dash__placeholder">Equity curve appears after your first closed trade.</div>
          )}
        </div>
        <div className="card">
          <div className="card__head">
            <h3>Drawdown</h3>
            <span className="card__badge">max {fmtPct(s.max_drawdown_pct)}</span>
          </div>
          {drawdownSeries.length > 1 ? (
            <LineChart data={drawdownSeries} color="var(--danger)" formatY={(v) => `${v.toFixed(1)}%`} height={220} />
          ) : (
            <div className="dash__placeholder">Drawdown will be shown once you have trades.</div>
          )}
        </div>
      </section>

      {/* R-Dist + Donut + Key Stats */}
      <section className="dash__row dash__row--3">
        <div className="card">
          <div className="card__head"><h3>R-Multiple Distribution</h3></div>
          {rBars.some((b) => b.v > 0) ? <BarChart data={rBars} height={180} /> : <div className="dash__placeholder">No R data yet.</div>}
        </div>
        <div className="card">
          <div className="card__head"><h3>Win / Loss</h3></div>
          {s.wins + s.losses > 0 ? <Donut wins={s.wins} losses={s.losses} /> : <div className="dash__placeholder">No results yet.</div>}
        </div>
        <div className="card">
          <div className="card__head"><h3>Key Stats</h3></div>
          <ul className="risk-list">
            <li><span>Profit factor</span><span className="mono">{Number(s.profit_factor).toFixed(2)}</span></li>
            <li><span>Expectancy</span><span className={`mono ${Number(s.expectancy_r) >= 0 ? "pos" : "neg"}`}>{fmtR(s.expectancy_r)}</span></li>
            <li><span>Avg win</span><span className="mono pos">{fmtR(s.avg_win_r)}</span></li>
            <li><span>Avg loss</span><span className="mono neg">{fmtR(s.avg_loss_r)}</span></li>
          </ul>
        </div>
      </section>

      {/* Motivation + Goals */}
      <section className="dash__row dash__row--2">
        <MotivationCard netPnl={s.net_pnl} closedTrades={s.closed_trades} />
        <GoalsCard
          goals={goals}
          periodPnl={periodPnl}
          risk={risk}
          onEdit={() => setShowGoalsEdit(true)}
        />
      </section>

      {/* Weekday Performance */}
      <WeekdayPerformanceCard trades={s.r_details} />

      {/* Risk Budget */}
      {risk && (
        <section className="card">
          <div className="card__head">
            <h3>Risk Budget</h3>
            <span className={`card__badge ${risk.max_positions_hit || risk.daily_loss_limit_hit || risk.portfolio_risk_exceeded ? "bad" : "ok"}`}>
              {risk.max_positions_hit || risk.daily_loss_limit_hit || risk.portfolio_risk_exceeded ? "⚠ limits hit" : "within limits"}
            </span>
          </div>
          <div className="risk-budget-grid">
            <RiskBar label="Portfolio risk" value={Number(risk.open_risk_pct)} limit={Number(risk.settings.max_portfolio_risk_pct)} unit="%" />
            <RiskBar label="Daily loss" value={Number(risk.daily_loss_pct)} limit={Number(risk.settings.daily_loss_limit_pct)} unit="%" />
            <RiskBar label="Open positions" value={risk.open_positions} limit={risk.settings.max_open_positions} unit="" />
          </div>
          {risk.warnings.length > 0 && (
            <ul className="risk-warnings">{risk.warnings.map((w, i) => <li key={i}>⚠ {w}</li>)}</ul>
          )}
        </section>
      )}

      {/* Prop-firm */}
      {pf && pf.rules.enabled && (
        <section className="card">
          <div className="card__head">
            <h3>Prop-firm Rules</h3>
            <span className={`card__badge ${pf.all_passed ? "ok" : "bad"}`}>
              {pf.all_passed ? "all passed" : "⚠ breach"}
            </span>
          </div>
          <div className="propfirm-grid">
            {pf.checks.map((c) => (
              <div key={c.name} className={`pf-check ${c.passed ? "is-pass" : "is-fail"}`}>
                <div className="pf-check__head">
                  <span className="pf-check__name">{c.name}</span>
                  <span className={`pf-check__status ${c.passed ? "ok" : "bad"}`}>{c.passed ? "✓" : "✕"}</span>
                </div>
                <div className="pf-check__msg">{c.message}</div>
              </div>
            ))}
          </div>
          <div className="pf-summary mono">
            Peak {Number(pf.peak_equity).toLocaleString()} · Current {Number(pf.current_equity).toLocaleString()} ·
            P&L {Number(pf.total_pnl) >= 0 ? "+" : ""}{Number(pf.total_pnl).toLocaleString()} · Trading days {pf.trading_days}
          </div>
        </section>
      )}

      {/* Session Heatmap */}
      <SessionHeatmapCard trades={s.r_details} />

      {/* Recent Trades */}
      <section className="card">
        <div className="card__head">
          <h3>Recent Closed Trades</h3>
          <span className="card__badge">{Math.min(5, s.r_details.length)} shown</span>
        </div>
        {s.r_details.length === 0 ? (
          <div className="dash__placeholder">Your closed trades will appear here.</div>
        ) : (
          <table className="trades">
            <thead>
              <tr><th>ID</th><th>Symbol</th><th>Side</th><th>R</th><th>P&L</th><th>Closed</th></tr>
            </thead>
            <tbody>
              {[...s.r_details].reverse().slice(0, 5).map((t: TradeRDetail) => (
                <tr key={t.id}>
                  <td className="mono dim">{t.id}</td>
                  <td className="mono">{t.symbol}</td>
                  <td className={t.side === "long" ? "side-long" : "side-short"}>{t.side}</td>
                  <td className={`mono ${Number(t.r_multiple) >= 0 ? "pos" : "neg"}`}>{fmtR(t.r_multiple)}</td>
                  <td className={`mono ${Number(t.net_pnl) >= 0 ? "pos" : "neg"}`}>{fmtMoney(t.net_pnl)}</td>
                  <td className="mono dim">{t.closed_at.slice(0, 10)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>

      {showGoalsEdit && (
        <GoalsEditModal
          current={goals}
          defaultDaily={risk ? (Number(risk.settings.daily_loss_limit_pct) * 2).toFixed(1) : "2"}
          onClose={() => setShowGoalsEdit(false)}
          onSaved={async () => {
            setShowGoalsEdit(false);
            await load();
          }}
        />
      )}
    </div>
  );
}

// ============================================================
// Motivation Card
// ============================================================
function MotivationCard({ netPnl, closedTrades }: {
  netPnl: string;
  closedTrades: number;
}) {
  const pnl = Number(netPnl);
  const message = MOTIVATION_MESSAGES.find((m) => pnl >= m.min);

  const [customText, setCustomText] = useState("");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");

  const displayedText = customText.trim() || message?.msg || "Keep going.";

  function save() {
    setCustomText(draft.trim());
    setEditing(false);
  }

  return (
    <>
      <div className="motivation-card">
        <div className="motivation-card__bg" />
        <div className="motivation-card__inner">
          <div className="motivation-card__head">
            <span className="motivation-card__label">Motivation</span>
            <button
              className="motivation-card__btn"
              onClick={() => { setDraft(customText); setEditing(true); }}
              title="Change message"
            >
              ✎
            </button>
          </div>
          <p className="motivation-card__msg">{displayedText}</p>
          {closedTrades === 0 && (
            <p className="motivation-card__sub">Start with your first trade.</p>
          )}
        </div>
      </div>

      {editing && (
        <div className="modal-backdrop" onClick={() => setEditing(false)}>
          <div className="modal motivation-edit" onClick={(e) => e.stopPropagation()}>
            <button className="screenshot-close" onClick={() => setEditing(false)}>×</button>
            <h3>Your motivation</h3>
            <p className="dim">Write something that keeps you grounded. It will appear on the card.</p>
            <textarea
              className="motivation-edit__input"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder="e.g. 'Protect capital. Trust the process.'"
              maxLength={240}
              rows={4}
              autoFocus
            />
            <div className="motivation-edit__counter">{draft.length} / 240</div>
            <div className="modal__actions">
              {customText && (
                <button className="btn btn--ghost" onClick={() => { setCustomText(""); setDraft(""); setEditing(false); }}>
                  Clear
                </button>
              )}
              <button className="btn btn--ghost" onClick={() => setEditing(false)}>Cancel</button>
              <button className="btn btn--primary" onClick={save}>Save</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}

// ============================================================
// Goals Card
// ============================================================

// ============================================================
// Weekday Performance
// ============================================================
const WEEKDAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function WeekdayPerformanceCard({ trades }: { trades: TradeRDetail[] }) {
  const buckets = WEEKDAY_LABELS.map((label) => ({ label, r: 0, count: 0 }));

  for (const t of trades) {
    if (!t.closed_at) continue;
    const d = new Date(t.closed_at);
    if (isNaN(d.getTime())) continue;
    const jsDay = d.getDay();
    const idx = jsDay === 0 ? 6 : jsDay - 1;
    buckets[idx].r += Number(t.r_multiple);
    buckets[idx].count += 1;
  }

  const maxAbs = Math.max(1, ...buckets.map((b) => Math.abs(b.r)));
  let bestIdx = -1;
  let worstIdx = -1;
  buckets.forEach((b, i) => {
    if (b.count === 0) return;
    if (bestIdx === -1 || b.r > buckets[bestIdx].r) bestIdx = i;
    if (worstIdx === -1 || b.r < buckets[worstIdx].r) worstIdx = i;
  });

  return (
    <section className="card">
      <div className="card__head">
        <h3>Performance by Day of Week</h3>
        <span className="card__badge">{trades.length} trades</span>
      </div>
      {trades.length === 0 ? (
        <div className="dash__placeholder">Day analysis appears once you have closed trades.</div>
      ) : (
        <div className="weekday-grid">
          {buckets.map((b, i) => {
            const empty = b.count === 0;
            const positive = b.r >= 0;
            const pct = (Math.abs(b.r) / maxAbs) * 100;
            const isBest = i === bestIdx && b.count > 0 && bestIdx !== worstIdx;
            const isWorst = i === worstIdx && b.count > 0 && bestIdx !== worstIdx;
            return (
              <div key={b.label} className={`wd-col ${empty ? "is-empty" : ""} ${isBest ? "is-best" : ""} ${isWorst ? "is-worst" : ""}`}>
                <div className="wd-col__label">{b.label}</div>
                <div className="wd-col__bar-wrap">
                  <div className={`wd-col__bar ${positive ? "is-positive" : "is-negative"}`} style={{ height: `${empty ? 0 : Math.max(8, pct)}%` }} />
                </div>
                <div className={`wd-col__value mono ${empty ? "dim" : positive ? "pos" : "neg"}`}>
                  {empty ? "—" : `${positive ? "+" : ""}${b.r.toFixed(1)}R`}
                </div>
                <div className="wd-col__count mono dim">{b.count > 0 ? `${b.count}t` : ""}</div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

// ============================================================
// Risk Bar
// ============================================================
function RiskBar({ label, value, limit, unit }: {
  label: string;
  value: number;
  limit: number;
  unit: string;
}) {
  const pct = limit > 0 ? Math.min(100, (value / limit) * 100) : 0;
  const danger = pct >= 100;
  const warn = pct >= 75 && !danger;
  const tone = danger ? "var(--danger)" : warn ? "var(--warning)" : "var(--accent)";
  const fmt = (v: number) => (unit === "%" ? v.toFixed(2) : String(v));
  return (
    <div className="risk-bar">
      <div className="risk-bar__head">
        <span>{label}</span>
        <span className={`mono ${danger ? "neg" : warn ? "" : "pos"}`}>
          {fmt(value)}{unit} / {fmt(limit)}{unit}
        </span>
      </div>
      <div className="risk-bar__track">
        <div className="risk-bar__fill" style={{ width: `${pct}%`, background: tone }} />
      </div>
    </div>
  );
}


// ============================================================
// Session Heatmap
// ============================================================
const SESSIONS = ["Asia", "London", "New York"] as const;
type Session = typeof SESSIONS[number];

function sessionOf(iso: string): Session {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "Asia";
  const hour = d.getUTCHours();
  if (hour < 8) return "Asia";
  if (hour < 16) return "London";
  return "New York";
}

function SessionHeatmapCard({ trades }: { trades: TradeRDetail[] }) {
  // 7 days × 3 sessions
  const grid: number[][] = Array.from({ length: 7 }, () => [0, 0, 0]);
  const counts: number[][] = Array.from({ length: 7 }, () => [0, 0, 0]);

  for (const t of trades) {
    if (!t.closed_at) continue;
    const d = new Date(t.closed_at);
    if (isNaN(d.getTime())) continue;
    const jsDay = d.getDay();
    const dayIdx = jsDay === 0 ? 6 : jsDay - 1;
    const sess = sessionOf(t.closed_at);
    const sessIdx = SESSIONS.indexOf(sess);
    grid[dayIdx][sessIdx] += Number(t.r_multiple);
    counts[dayIdx][sessIdx] += 1;
  }

  // Find min/max R for color scale
  const flat = grid.flat();
  const maxR = Math.max(...flat.map(Math.abs), 1);

  function cellColor(r: number, count: number): string {
    if (count === 0) return "var(--bg-2)";
    const intensity = Math.min(1, Math.abs(r) / maxR);
    if (r > 0) {
      return `rgba(45, 212, 191, ${0.15 + intensity * 0.6})`;
    } else if (r < 0) {
      return `rgba(248, 113, 113, ${0.15 + intensity * 0.6})`;
    }
    return "var(--bg-3)";
  }

  const DAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

  return (
    <section className="card">
      <div className="card__head">
        <h3>Session Heatmap</h3>
        <span className="card__badge">{trades.length} trades</span>
      </div>
      {trades.length === 0 ? (
        <div className="dash__placeholder">Session analysis appears once you have closed trades.</div>
      ) : (
        <div className="heatmap">
          <div className="heatmap__header">
            <div className="heatmap__empty"></div>
            {SESSIONS.map((sess) => (
              <div key={sess} className="heatmap__sess-label">{sess}</div>
            ))}
          </div>
          {DAYS.map((day, dayIdx) => (
            <div key={day} className="heatmap__row">
              <div className="heatmap__day-label">{day}</div>
              {SESSIONS.map((sess, sessIdx) => {
                const r = grid[dayIdx][sessIdx];
                const count = counts[dayIdx][sessIdx];
                return (
                  <div
                    key={sess}
                    className="heatmap__cell"
                    style={{ background: cellColor(r, count) }}
                    title={
                      count > 0
                        ? `${day} · ${sess}: ${count} trade${count === 1 ? "" : "s"}, ${r >= 0 ? "+" : ""}${r.toFixed(2)}R`
                        : `${day} · ${sess}: no trades`
                    }
                  >
                    {count > 0 && (
                      <span className={`heatmap__value mono ${r >= 0 ? "pos" : "neg"}`}>
                        {r >= 0 ? "+" : ""}{r.toFixed(1)}
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
          <div className="heatmap__legend">
            <span className="heatmap__legend-label">Loss</span>
            <span className="heatmap__legend-cell" style={{ background: "rgba(248, 113, 113, 0.6)" }}></span>
            <span className="heatmap__legend-cell" style={{ background: "rgba(248, 113, 113, 0.3)" }}></span>
            <span className="heatmap__legend-cell" style={{ background: "var(--bg-2)" }}></span>
            <span className="heatmap__legend-cell" style={{ background: "rgba(45, 212, 191, 0.3)" }}></span>
            <span className="heatmap__legend-cell" style={{ background: "rgba(45, 212, 191, 0.6)" }}></span>
            <span className="heatmap__legend-label">Win</span>
          </div>
        </div>
      )}
    </section>
  );
}

// ============================================================
// Goals Card — 3 progress bars for daily/weekly/monthly
// ============================================================
function GoalsCard({
  goals,
  periodPnl,
  risk,
  onEdit,
}: {
  goals: AccountGoals | null;
  periodPnl: PeriodPnl | null;
  risk: RiskStatus | null;
  onEdit: () => void;
}) {
  // Auto-default from risk if no goals saved
  const autoDaily = risk
    ? (Number(risk.settings.daily_loss_limit_pct) * 2).toFixed(1)
    : "2";
  const autoWeekly = (Number(autoDaily) * 3).toFixed(1);
  const autoMonthly = (Number(autoWeekly) * 1.33).toFixed(1);

  const dailyTarget = goals ? Number(goals.daily_target_pct) : Number(autoDaily);
  const weeklyTarget = goals ? Number(goals.weekly_target_pct) : Number(autoWeekly);
  const monthlyTarget = goals ? Number(goals.monthly_target_pct) : Number(autoMonthly);

  const dailyActual = periodPnl ? Number(periodPnl.daily_pct) : 0;
  const weeklyActual = periodPnl ? Number(periodPnl.weekly_pct) : 0;
  const monthlyActual = periodPnl ? Number(periodPnl.monthly_pct) : 0;

  const renderBar = (
    label: string,
    actual: number,
    target: number,
    pnlValue: string | undefined,
    period: string,
  ) => {
    const pct = target > 0 ? Math.min(100, Math.max(0, (actual / target) * 100)) : 0;
    const done = pct >= 100;
    const positive = actual >= 0;
    return (
      <div className="goal-bar" key={label}>
        <div className="goal-bar__head">
          <span className="goal-bar__label">{label}</span>
          <span className={`mono ${positive ? "pos" : "neg"}`}>
            {positive ? "+" : ""}
            {actual.toFixed(2)}% / {target.toFixed(1)}%
            {pnlValue && (
              <span className="goal-bar__amount dim">
                {" "}
                ({Number(pnlValue) >= 0 ? "+" : ""}
                {fmtMoney(pnlValue)})
              </span>
            )}
          </span>
        </div>
        <div className="goal-bar__track">
          <div
            className={`goal-bar__fill ${done ? "is-done" : ""} ${
              !positive ? "is-negative" : ""
            }`}
            style={{ width: `${pct}%` }}
          />
        </div>
        <div className="goal-bar__hint">
          {done
            ? `✓ ${period} target reached`
            : `${pct.toFixed(0)}% toward ${period} target`}
        </div>
      </div>
    );
  };

  return (
    <div className="card goals-card">
      <div className="card__head">
        <h3>Goals</h3>
        <button
          className="btn btn--ghost btn--small"
          onClick={onEdit}
          title="Edit target percentages"
        >
          ⚙ Edit
        </button>
      </div>
      {!goals && (
        <div className="goals-card__default-hint">
          <span className="dim">
            💡 Using auto-defaults from your risk settings:
          </span>
        </div>
      )}
      <div className="goals-list">
        {renderBar("Daily", dailyActual, dailyTarget, periodPnl?.daily_pnl, "today")}
        {renderBar("Weekly", weeklyActual, weeklyTarget, periodPnl?.weekly_pnl, "this week")}
        {renderBar("Monthly", monthlyActual, monthlyTarget, periodPnl?.monthly_pnl, "this month")}
      </div>
    </div>
  );
}

// ============================================================
// Goals Edit Modal
// ============================================================
function GoalsEditModal({
  current,
  defaultDaily,
  onClose,
  onSaved,
}: {
  current: AccountGoals | null;
  defaultDaily: string;
  onClose: () => void;
  onSaved: () => void;
}) {
  const [daily, setDaily] = useState(
    current ? current.daily_target_pct : defaultDaily
  );
  const [weekly, setWeekly] = useState(
    current
      ? current.weekly_target_pct
      : (Number(defaultDaily) * 3).toFixed(1)
  );
  const [monthly, setMonthly] = useState(
    current
      ? current.monthly_target_pct
      : (Number(defaultDaily) * 4).toFixed(1)
  );
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setError(null);
    setBusy(true);
    try {
      const accId = await getActiveAccountId();
      await setGoals(accId, {
        daily_target_pct: daily,
        weekly_target_pct: weekly,
        monthly_target_pct: monthly,
      });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div
        className="modal"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 420 }}
      >
        <h3>Edit Goals</h3>
        <p className="dim">
          Target percentages for daily, weekly, and monthly profit.
        </p>

        <div style={{ display: "flex", flexDirection: "column", gap: 12, marginTop: 16 }}>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <span>Daily target (%)</span>
            <input
              className="mono"
              value={daily}
              onChange={(e) => setDaily(e.target.value)}
              placeholder="2.0"
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <span>Weekly target (%)</span>
            <input
              className="mono"
              value={weekly}
              onChange={(e) => setWeekly(e.target.value)}
              placeholder="6.0"
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            <span>Monthly target (%)</span>
            <input
              className="mono"
              value={monthly}
              onChange={(e) => setMonthly(e.target.value)}
              placeholder="8.0"
            />
          </label>
        </div>

        {error && (
          <div className="card card--error" style={{ marginTop: 12 }}>
            {error}
          </div>
        )}

        <div className="modal__actions">
          <button className="btn btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button className="btn btn--primary" onClick={save} disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}
