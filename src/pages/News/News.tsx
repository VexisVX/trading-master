import { useEffect, useMemo, useState } from "react";
import { fetchFinnhubNews, FinnhubArticle, fetchCalendar, CalendarEvent } from "../../lib/newsApi";
import { HelpTip } from "../../components/ui/HelpTip";
import { NEWS_TIMEZONES, formatTimeInTz, formatDayInTz, getTzOffset } from "../../lib/timezone";
import { getSetting, setSetting } from "../../lib/settingsApi";
import "./News.css";

type Tab = "news" | "calendar";

// ============================================================
// Root component
// ============================================================
export function News() {
  const [tab, setTab] = useState<Tab>("news");
  const [newsTimezone, setNewsTimezone] = useState<string>("Asia/Tehran");

  useEffect(() => {
    (async () => {
      try {
        const saved = await getSetting("news.timezone");
        if (saved) setNewsTimezone(saved);
      } catch { /* ignore */ }
    })();
  }, []);

  function changeTimezone(tz: string) {
    setNewsTimezone(tz);
    setSetting("news.timezone", tz).catch(() => {});
  }

  return (
    <div className="news">
      <div className="news-tabs">
        <button className={`news-tab ${tab === "news" ? "is-active" : ""}`} onClick={() => setTab("news")}>News</button>
        <button className={`news-tab ${tab === "calendar" ? "is-active" : ""}`} onClick={() => setTab("calendar")}>Calendar</button>
      </div>

      <div className="news-tz-bar">
        <span className="news-tz-bar__label">Timezone</span>
        <select
          className="news-tz-bar__select"
          value={newsTimezone}
          onChange={(e) => changeTimezone(e.target.value)}
        >
          {NEWS_TIMEZONES.map((tz) => (
            <option key={tz.id} value={tz.id}>
              {tz.label} · {getTzOffset(tz.id)}
            </option>
          ))}
        </select>
        <HelpTip title="Timezone">
          All news and calendar times are shown in this timezone.
        </HelpTip>
      </div>

      {tab === "news" && <NewsFeed timezone={newsTimezone} />}
      {tab === "calendar" && <Calendar timezone={newsTimezone} />}
    </div>
  );
}

// ============================================================
// News Feed
// ============================================================
function NewsFeed({ timezone }: { timezone: string }) {
  const [articles, setArticles] = useState<FinnhubArticle[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true); setError(null);
    try {
      setArticles(await fetchFinnhubNews());
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }

  useEffect(() => { refresh(); }, []);

  return (
    <>
      <div className="news-toolbar">
        <button className="news-refresh" onClick={refresh} disabled={loading}>
          {loading ? "…" : "↻"}
        </button>
        <span className="news-count">{articles.length} articles</span>
        <HelpTip title="News Feed">
          Live headlines from Finnhub. Click any article to open the source.
        </HelpTip>
      </div>

      {error && <div className="news-error">{error}</div>}

      {loading && articles.length === 0 && (
        <div className="news-loading">
          {[1, 2, 3].map((i) => <div key={i} className="news-skeleton" />)}
        </div>
      )}

      {!loading && articles.length === 0 && !error && (
        <div className="news-empty">
          <span className="news-empty__icon">📰</span>
          <span className="news-empty__title">No articles</span>
        </div>
      )}

      {articles.length > 0 && (
        <div className="news-feed">
          {articles.map((a) => (
            <div key={a.id} className="news-card">
              {a.image && (
                <div className="news-card__image">
                  <img src={a.image} alt="" loading="lazy" onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }} />
                </div>
              )}
              <div className="news-card__body">
                <div className="news-card__meta">
                  <span className="news-card__source">{a.source}</span>
                  <span className="news-card__dot">·</span>
                  <span className="news-card__time">{formatTimeInTz(new Date(a.datetime * 1000).toISOString(), timezone)}</span>
                  <span className="news-card__category">{a.category}</span>
                </div>
                <a className="news-card__title" href={a.url} target="_blank" rel="noreferrer noopener">
                  {a.headline}
                </a>
                {a.summary && <p className="news-card__summary">{a.summary}</p>}
                {a.related && (
                  <div className="news-card__related">
                    {a.related.split(",").filter(Boolean).slice(0, 6).map((sym, i) => (
                      <span key={i} className="news-card__symbol">{sym.trim()}</span>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}

// ============================================================
// Helpers
// ============================================================
function categorize(title: string): string {
  const t = title.toLowerCase();
  if (t.includes("cpi") || t.includes("inflation") || t.includes("ppi") || t.includes("pce")) return "Inflation";
  if (t.includes("non-farm") || t.includes("nfp") || t.includes("unemployment") || t.includes("employment") || t.includes("jobless") || t.includes("payroll")) return "Employment";
  if (t.includes("interest rate") || t.includes("fomc") || t.includes("fed") || t.includes("ecb") || t.includes("boe") || t.includes("boj") || t.includes("rate decision") || t.includes("central bank")) return "Central Bank";
  if (t.includes("gdp") || t.includes("growth")) return "GDP";
  if (t.includes("retail") || t.includes("consumer")) return "Consumer";
  if (t.includes("pmi") || t.includes("manufacturing") || t.includes("production")) return "Manufacturing";
  if (t.includes("trade balance") || t.includes("current account")) return "Trade";
  if (t.includes("housing") || t.includes("home") || t.includes("building")) return "Housing";
  if (t.includes("speech") || t.includes("speaks") || t.includes("testimony")) return "Speeches";
  return "Other";
}

function explainCategory(cat: string): string {
  switch (cat) {
    case "Inflation": return "Inflation data (CPI, PPI, PCE) measures price growth. High readings typically push central banks to raise rates, strengthening the currency. Low readings suggest easing, weakening the currency.";
    case "Employment": return "Employment reports (NFP, unemployment, payrolls) show labor market health. Strong jobs data usually strengthens the currency and can push rate expectations higher.";
    case "Central Bank": return "Rate decisions and central bank communications move markets most. A hike or hawkish tone strengthens the currency; a cut or dovish tone weakens it.";
    case "GDP": return "GDP measures economic growth. Stronger growth supports the currency and equities; weaker growth often pressures the currency and supports safe-haven assets.";
    case "Consumer": return "Consumer data (retail sales, confidence) shows spending health. Strong data supports the currency; weak data suggests slowing demand.";
    case "Manufacturing": return "Manufacturing data (PMI, industrial production) shows factory activity. Expansion supports the currency; contraction warns of slowdown.";
    case "Trade": return "Trade balance measures exports vs imports. A surplus supports the currency; a deficit weighs on it.";
    case "Housing": return "Housing data reflects real estate health and consumer confidence. Strong data typically supports the currency.";
    case "Speeches": return "Speeches by central bank officials can signal future policy. Markets watch for hints of rate changes or policy shifts.";
    default: return "This event may move markets depending on the reading versus forecast.";
  }
}

// ============================================================
// Calendar
// ============================================================
function Calendar({ timezone }: { timezone: string }) {
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [impactSet, setImpactSet] = useState<Set<string>>(new Set(["High", "Medium", "Low"]));
  const [currency, setCurrency] = useState<string>("All");
  const [categoryFilter, setCategoryFilter] = useState<string>("All");
  const [now, setNow] = useState(Date.now());
  const [showPast, setShowPast] = useState(false);
  const [selected, setSelected] = useState<CalendarEvent | null>(null);

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(id);
  }, []);

  async function refresh() {
    setLoading(true); setError(null);
    try {
      setEvents(await fetchCalendar());
    } catch (e) { setError(String(e)); }
    finally { setLoading(false); }
  }

  useEffect(() => { refresh(); }, []);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") setSelected(null);
    }
    if (selected) {
      document.addEventListener("keydown", onKey);
      return () => document.removeEventListener("keydown", onKey);
    }
  }, [selected]);

  const currencies = useMemo(() => {
    const set = new Set<string>();
    for (const e of events) {
      const c = (e.currency && e.currency.trim()) || (e.country && e.country.trim());
      if (c) set.add(c);
    }
    return ["All", ...Array.from(set).sort()];
  }, [events]);

  const categories = useMemo(() => {
    const set = new Set<string>();
    for (const e of events) set.add(categorize(e.title));
    return ["All", ...Array.from(set).sort()];
  }, [events]);

  const filtered = useMemo(() => {
    let out = events;
    out = out.filter((e) => impactSet.has(e.impact));
    if (currency !== "All") {
      out = out.filter((e) => {
        const c = (e.currency && e.currency.trim()) || (e.country && e.country.trim());
        return c === currency;
      });
    }
    if (categoryFilter !== "All") {
      out = out.filter((e) => categorize(e.title) === categoryFilter);
    }
    return out;
  }, [events, impactSet, currency, categoryFilter]);

  const { future, past, nearest } = useMemo(() => {
    const futureMap = new Map<string, CalendarEvent[]>();
    const pastMap = new Map<string, CalendarEvent[]>();
    const nowMs = Date.now();
    let nearestEvent: CalendarEvent | null = null;
    let nearestDiff = Infinity;

    for (const e of filtered) {
      const day = e.date.slice(0, 10);
      const eventTime = new Date(e.date).getTime();
      const isPast = eventTime < nowMs;
      const target = isPast ? pastMap : futureMap;
      if (!target.has(day)) target.set(day, []);
      target.get(day)!.push(e);

      if (!isPast) {
        const diff = eventTime - nowMs;
        if (diff > 0 && diff < nearestDiff) {
          nearestDiff = diff;
          nearestEvent = e;
        }
      }
    }

    return {
      future: Array.from(futureMap.entries()).sort((a, b) => a[0].localeCompare(b[0])),
      past: Array.from(pastMap.entries()).sort((a, b) => b[0].localeCompare(a[0])),
      nearest: nearestEvent,
    };
  }, [filtered]);

  const hasNoResults = filtered.length === 0 && !loading;

  return (
    <>
      <div className="cal-filters">
        <div className="cal-impact-toggles">
          {(["High", "Medium", "Low"] as const).map((i) => {
            const active = impactSet.has(i);
            const cls = `cal-toggle cal-toggle--${i.toLowerCase()} ${active ? "is-active" : ""}`;
            return (
              <button
                key={i}
                className={cls}
                onClick={() => {
                  setImpactSet((prev) => {
                    const next = new Set(prev);
                    if (next.has(i)) next.delete(i);
                    else next.add(i);
                    return next;
                  });
                }}
              >
                <span className="cal-toggle__dot" />
                {i}
              </button>
            );
          })}
        </div>

        <select className="cal-select" value={currency} onChange={(e) => setCurrency(e.target.value)}>
          {currencies.map((c) => (
            <option key={c} value={c}>{c === "All" ? "All currencies" : c}</option>
          ))}
        </select>

        <select className="cal-select" value={categoryFilter} onChange={(e) => setCategoryFilter(e.target.value)}>
          {categories.map((c) => (
            <option key={c} value={c}>{c === "All" ? "All categories" : c}</option>
          ))}
        </select>

        <button className="cal-refresh" onClick={refresh} disabled={loading}>
          {loading ? "…" : "↻"}
        </button>
      </div>

      {error && <div className="news-error">{error}</div>}

      {loading && events.length === 0 && (
        <div className="news-loading">
          {[1, 2, 3].map((i) => <div key={i} className="news-skeleton" />)}
        </div>
      )}

      {hasNoResults && !error && (
        <div className="cal-empty">
          <div className="cal-empty__glow"></div>
          <span className="cal-empty__icon">🔍</span>
          <h3 className="cal-empty__title">No events match your filters</h3>
          <p className="cal-empty__hint">
            {currency !== "All" && categoryFilter !== "All"
              ? `No ${categoryFilter} events for ${currency} with the selected impact levels.`
              : currency !== "All"
              ? `No ${currency} events match the selected impact levels.`
              : categoryFilter !== "All"
              ? `No ${categoryFilter} events match the selected impact levels.`
              : "Try adjusting impact, currency, or category filters."}
          </p>
          <div className="cal-empty__actions">
            {currency !== "All" && (
              <button className="btn btn--ghost btn--small" onClick={() => setCurrency("All")}>Clear currency</button>
            )}
            {categoryFilter !== "All" && (
              <button className="btn btn--ghost btn--small" onClick={() => setCategoryFilter("All")}>Clear category</button>
            )}
            {impactSet.size < 3 && (
              <button className="btn btn--ghost btn--small" onClick={() => setImpactSet(new Set(["High", "Medium", "Low"]))}>
                Show all impact
              </button>
            )}
          </div>
        </div>
      )}

      {future.map(([day, dayEvents]) => (
        <div key={day} className="cal-day">
          <div className="cal-day__head">
            <span className="cal-day__name">{formatDayInTz(day, timezone)}</span>
            <span className="cal-day__count">{dayEvents.length} events</span>
          </div>
          <div className="cal-cards">
            {dayEvents.map((e, i) => (
              <EventCard
                key={`${day}-${i}`}
                event={e}
                now={now}
                timezone={timezone}
                isNearest={nearest != null && e.date === nearest.date && e.title === nearest.title}
                onClick={() => setSelected(e)}
              />
            ))}
          </div>
        </div>
      ))}

      {past.length > 0 && (
        <div className="cal-past">
          <button className="cal-past__toggle" onClick={() => setShowPast((v) => !v)}>
            {showPast ? "▼" : "▶"} Past events ({past.reduce((sum, [, evs]) => sum + evs.length, 0)})
          </button>

          {showPast && past.map(([day, dayEvents]) => (
            <div key={day} className="cal-day cal-day--past">
              <div className="cal-day__head">
                <span className="cal-day__name">{formatDayInTz(day, timezone)}</span>
                <span className="cal-day__count">{dayEvents.length} events</span>
              </div>
              <div className="cal-cards">
                {dayEvents.map((e, i) => (
                  <EventCard
                    key={`${day}-${i}`}
                    event={e}
                    now={now}
                    timezone={timezone}
                    isNearest={false}
                    onClick={() => setSelected(e)}
                  />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      {selected && <EventDetailPopup event={selected} timezone={timezone} onClose={() => setSelected(null)} />}
    </>
  );
}

// ============================================================
// Event Card
// ============================================================
function EventCard({ event, now, timezone, isNearest, onClick }: {
  event: CalendarEvent;
  now: number;
  timezone: string;
  isNearest: boolean;
  onClick: () => void;
}) {
  const time = new Date(event.date).getTime();
  const diff = time - now;
  const past = diff < 0;
  const currency = event.currency || event.country || "—";
  const impactClass = event.impact.toLowerCase();
  const category = categorize(event.title);

  let countdownText: string;
  if (past) {
    countdownText = "passed";
  } else if (isNearest) {
    const totalSec = Math.floor(diff / 1000);
    const d = Math.floor(totalSec / 86400);
    const h = Math.floor((totalSec % 86400) / 3600);
    const m = Math.floor((totalSec % 3600) / 60);
    const s = totalSec % 60;
    if (d > 0) countdownText = `${d}d ${h}h ${m}m ${s}s`;
    else if (h > 0) countdownText = `${h}h ${m}m ${s}s`;
    else if (m > 0) countdownText = `${m}m ${s}s`;
    else countdownText = `${s}s`;
  } else {
    const days = Math.floor(diff / 86400000);
    const hours = Math.floor((diff % 86400000) / 3600000);
    const mins = Math.floor((diff % 3600000) / 60000);
    if (days > 0) countdownText = `${days}d`;
    else if (hours > 0) countdownText = `${hours}h`;
    else countdownText = `${mins}m`;
  }

  return (
    <button
      type="button"
      className={`cal-card cal-card--${impactClass} ${past ? "cal-card--past" : ""} ${isNearest ? "cal-card--nearest" : ""}`}
      onClick={onClick}
    >
      <div className="cal-card__head">
        <span className="cal-card__icon"><IconForEvent title={event.title} impact={event.impact} /></span>
        <span className="cal-card__time">{formatTimeInTz(event.date, timezone)}</span>
        <span className={`cal-card__impact cal-card__impact--${impactClass}`}>{event.impact}</span>
        <span className="cal-card__currency">{currency}</span>
        <span className={`cal-card__countdown ${past ? "past" : isNearest ? "nearest" : "future"}`}>
          {isNearest && !past && <span className="cal-card__pulse" />}
          {countdownText}
        </span>
      </div>
      <div className="cal-card__title">{event.title}</div>
      <div className="cal-card__meta">
        <span className="cal-card__category">{category}</span>
        {(event.forecast || event.previous) && (
          <div className="cal-card__values">
            {event.forecast && <span className="cal-card__val"><span className="label">F</span>{event.forecast}</span>}
            {event.previous && <span className="cal-card__val"><span className="label">P</span>{event.previous}</span>}
          </div>
        )}
      </div>
    </button>
  );
}

// ============================================================
// Detail Popup
// ============================================================
function EventDetailPopup({ event, timezone, onClose }: {
  event: CalendarEvent;
  timezone: string;
  onClose: () => void;
}) {
  const category = categorize(event.title);
  const impact = event.impact.toLowerCase();
  const past = new Date(event.date).getTime() < Date.now();

  return (
    <div className="cal-modal-backdrop" onClick={onClose}>
      <div className="cal-modal" onClick={(e) => e.stopPropagation()}>
        <button className="cal-modal__close" onClick={onClose}>×</button>

        <div className={`cal-modal__head cal-modal__head--${impact}`}>
          <span className={`cal-modal__impact cal-modal__impact--${impact}`}>{event.impact} Impact</span>
          <span className="cal-modal__currency">{event.currency || event.country}</span>
          <span className="cal-modal__category">{category}</span>
        </div>

        <h2 className="cal-modal__title">{event.title}</h2>

        <div className="cal-modal__meta">
          <div className="cal-modal__meta-item">
            <span className="label">Date</span>
            <span className="value">{formatDayInTz(event.date.slice(0, 10), timezone)}</span>
          </div>
          <div className="cal-modal__meta-item">
            <span className="label">Time</span>
            <span className="value mono">{formatTimeInTz(event.date, timezone)}</span>
          </div>
          <div className="cal-modal__meta-item">
            <span className="label">Status</span>
            <span className={`value ${past ? "dim" : "accent"}`}>
              {past ? "Already passed" : "Upcoming"}
            </span>
          </div>
        </div>

        <div className="cal-modal__values">
          <div className="cal-modal__val">
            <span className="label">Forecast</span>
            <span className="value mono">{event.forecast || "—"}</span>
          </div>
          <div className="cal-modal__val">
            <span className="label">Previous</span>
            <span className="value mono">{event.previous || "—"}</span>
          </div>
        </div>

        {category !== "Other" && (
          <div className="cal-modal__explain">
            <div className="cal-modal__explain-title">{category} — what it means</div>
            <p>{explainCategory(category)}</p>
          </div>
        )}

        <div className="cal-modal__footer">Source: ForexFactory weekly feed</div>
      </div>
    </div>
  );
}

// ============================================================
// Icon picker
// ============================================================
function IconForEvent({ title, impact }: { title: string; impact: string }) {
  const t = title.toLowerCase();
  const color = impact === "High" ? "var(--danger)" : impact === "Medium" ? "var(--warning)" : "var(--text-3)";

  let path = "";
  if (t.includes("cpi") || t.includes("inflation") || t.includes("ppi") || t.includes("pce")) {
    path = "M4 18 L8 14 L12 16 L16 10 L20 6 M16 6 L20 6 L20 10";
  } else if (t.includes("nfp") || t.includes("non-farm") || t.includes("unemployment") || t.includes("payroll")) {
    path = "M12 11 a3 3 0 1 0 0-6 a3 3 0 0 0 0 6 M5 20 c0-4 3-6 7-6 s7 2 7 6";
  } else if (t.includes("fomc") || t.includes("rate decision") || t.includes("interest rate") || t.includes("fed ") || t.includes("ecb") || t.includes("boe")) {
    path = "M3 10 L12 4 L21 10 M5 10 L5 19 M9 10 L9 19 M15 10 L15 19 M19 10 L19 19 M3 19 L21 19";
  } else if (t.includes("gdp") || t.includes("growth")) {
    path = "M4 20 L4 12 M10 20 L10 8 M16 20 L16 14 M22 20 L22 4";
  } else if (t.includes("retail") || t.includes("consumer")) {
    path = "M6 7 L6 20 L18 20 L18 7 M9 7 a3 3 0 0 1 6 0";
  } else if (t.includes("pmi") || t.includes("manufacturing")) {
    path = "M3 20 L3 10 L8 13 L8 10 L13 13 L13 10 L18 13 L18 20 Z M21 20 L21 10";
  } else {
    path = "M12 3 a9 9 0 1 0 0 18 a9 9 0 0 0 0-18 M12 8 L12 13 M12 16 L12 16";
  }

  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d={path} />
    </svg>
  );
}
