import { useState } from "react";
import { quickCreateTrade, TradeOutcome } from "../../lib/tradesApi";
import "./Journal.css";

interface Props {
  accountId: number;
  onCreated: () => void;
}

const SESSIONS = ["Asia", "London", "New York"];
type Mode = "r" | "prices";

export function QuickEntryForm({ accountId, onCreated }: Props) {
  const [symbol, setSymbol] = useState("EURUSD");
  const [outcome, setOutcome] = useState<TradeOutcome>("tp_hit");
  const [mode, setMode] = useState<Mode>("r");

  const [rValue, setRValue] = useState("");

  const [entry, setEntry] = useState("");
  const [stop, setStop] = useState("");
  const [tp, setTp] = useState("");
  const [qty, setQty] = useState("1");

  const [session, setSession] = useState<string>("London");
  const [timeOfDay, setTimeOfDay] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const rAuto = (() => {
    if (mode !== "prices") return null;
    const e = parseFloat(entry), s = parseFloat(stop);
    if (!isFinite(e) || !isFinite(s) || e === s) return null;
    const tpN = parseFloat(tp);
    const exit = outcome === "tp_hit" && isFinite(tpN) ? tpN
               : outcome === "sl_hit" ? s
               : e;
    const denom = Math.abs(e - s);
    const r = (exit - e) / denom;
    return isFinite(r) ? r : null;
  })();

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (!symbol.trim()) { setError("Symbol is required."); return; }
    if (mode === "prices") {
      if (!entry || !stop) { setError("Entry and Stop are required in prices mode."); return; }
      if (parseFloat(entry) === parseFloat(stop)) {
        setError("Entry and Stop must differ."); return;
      }
    }

    const payload = {
      account_id: accountId,
      symbol: symbol.trim().toUpperCase(),
      outcome,
      r_value: mode === "r" ? (rValue ? rValue.replace(/^\+/, "").trim() || null : null) : null,
      session: session || null,
      time_of_day: timeOfDay ? timeOfDay.slice(0, 5) : null,
      entry_price: mode === "prices" ? entry : null,
      stop_price: mode === "prices" ? stop : null,
      take_profit: mode === "prices" && tp ? tp : null,
      quantity: mode === "prices" && qty ? qty : null,
    };

    setBusy(true);
    try {
      await quickCreateTrade(payload);
      setRValue(""); setTimeOfDay("");
      if (mode === "prices") { setEntry(""); setStop(""); setTp(""); }
      onCreated();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="quick-entry" onSubmit={submit}>
      <div className="quick-entry__mode">
        <button type="button"
          className={`quick-mode-btn ${mode === "r" ? "is-active" : ""}`}
          onClick={() => setMode("r")}>⚡ R mode</button>
        <button type="button"
          className={`quick-mode-btn ${mode === "prices" ? "is-active" : ""}`}
          onClick={() => setMode("prices")}>📈 Prices mode</button>
      </div>

      <div className="quick-entry__row">
        <label className="quick-entry__field quick-entry__field--symbol">
          <span>Symbol</span>
          <input value={symbol} onChange={(e) => setSymbol(e.target.value)}
                 placeholder="EURUSD" className="mono" />
        </label>
        <div className="quick-entry__field quick-entry__field--wide">
          <span>Outcome</span>
          <div className="quick-entry__outcomes">
            <button type="button"
              className={`quick-outcome ${outcome === "tp_hit" ? "is-tp" : ""}`}
              onClick={() => setOutcome("tp_hit")}>✓ TP</button>
            <button type="button"
              className={`quick-outcome ${outcome === "sl_hit" ? "is-sl" : ""}`}
              onClick={() => setOutcome("sl_hit")}>✕ SL</button>
            <button type="button"
              className={`quick-outcome ${outcome === "manual_close" ? "is-manual" : ""}`}
              onClick={() => setOutcome("manual_close")}>Manual</button>
            <button type="button"
              className={`quick-outcome ${outcome === "breakeven" ? "is-be" : ""}`}
              onClick={() => setOutcome("breakeven")}>BE</button>
          </div>
        </div>
      </div>

      {mode === "r" ? (
        <div className="quick-entry__row quick-entry__row--3">
          <label className="quick-entry__field">
            <span>R value</span>
            <input value={rValue} onChange={(e) => setRValue(e.target.value)}
                   placeholder="+2.5" className="mono" />
          </label>
          <label className="quick-entry__field">
            <span>Session</span>
            <select value={session} onChange={(e) => setSession(e.target.value)}>
              {SESSIONS.map((s) => <option key={s} value={s}>{s}</option>)}
            </select>
          </label>
          <label className="quick-entry__field">
            <span>Time</span>
            <input value={timeOfDay} onChange={(e) => setTimeOfDay(e.target.value)}
                   placeholder="14:30" className="mono" maxLength={5} />
          </label>
        </div>
      ) : (
        <>
          <div className="quick-entry__row quick-entry__row--4">
            <label className="quick-entry__field">
              <span>Entry</span>
              <input value={entry} onChange={(e) => setEntry(e.target.value)}
                     placeholder="1.1000" className="mono" />
            </label>
            <label className="quick-entry__field">
              <span>Stop</span>
              <input value={stop} onChange={(e) => setStop(e.target.value)}
                     placeholder="1.0950" className="mono" />
            </label>
            <label className="quick-entry__field">
              <span>Target</span>
              <input value={tp} onChange={(e) => setTp(e.target.value)}
                     placeholder="1.1150" className="mono" />
            </label>
            <label className="quick-entry__field">
              <span>Qty</span>
              <input value={qty} onChange={(e) => setQty(e.target.value)}
                     placeholder="1" className="mono" />
            </label>
          </div>
          <div className="quick-entry__row quick-entry__row--3">
            <div className="quick-entry__field">
              <span>R (auto)</span>
              <div className="quick-r-auto mono">
                {rAuto == null ? "—" : `${rAuto >= 0 ? "+" : ""}${rAuto.toFixed(2)}`}
              </div>
            </div>
            <label className="quick-entry__field">
              <span>Session</span>
              <select value={session} onChange={(e) => setSession(e.target.value)}>
                {SESSIONS.map((s) => <option key={s} value={s}>{s}</option>)}
              </select>
            </label>
            <label className="quick-entry__field">
              <span>Time</span>
              <input value={timeOfDay} onChange={(e) => setTimeOfDay(e.target.value)}
                     placeholder="14:30" className="mono" maxLength={5} />
            </label>
          </div>
        </>
      )}

      {error && <div className="quick-entry__error">{error}</div>}

      <div className="quick-entry__actions">
        <button type="submit" className="btn btn--primary" disabled={busy}>
          {busy ? "Adding…" : "Add Trade"}
        </button>
      </div>
    </form>
  );
}
