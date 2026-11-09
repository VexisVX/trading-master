import { useEffect, useState } from "react";
import { createTrade, NewTrade, Side, SizeType } from "../../lib/tradesApi";
import { Select } from "../../components/ui/Select";
import { HelpTip } from "../../components/ui/HelpTip";
import { listStrategies, Strategy } from "../../lib/strategiesApi";
import { open } from "@tauri-apps/plugin-dialog";

interface Props {
  accountId: number;
  onCreated: () => void;
  onCancel?: () => void;
}
export function TradeForm({ accountId, onCreated, onCancel }: Props) {
  const [symbol, setSymbol] = useState("EURUSD");
  const [openedDay, setOpenedDay] = useState<string>(() => {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  });
  const [openedTime, setOpenedTime] = useState<string>(() => {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  });
  const [side, setSide] = useState<Side>("long");
  const [entry, setEntry] = useState("");
  const [stop, setStop] = useState("");
  const [takeProfit, setTakeProfit] = useState("");
  const [qty, setQty] = useState("1");
  const [sizeType, setSizeType] = useState<SizeType>("lots");
  const [sizeValue, setSizeValue] = useState("1");
  const [sizeUnit, setSizeUnit] = useState("USD");
  const [cost, setCost] = useState("0");
  const [notes, setNotes] = useState("");
  const [chartUrl, setChartUrl] = useState("");
  const [screenshotPath, setScreenshotPath] = useState("");
  const [tagsInput, setTagsInput] = useState("");
  const [strategies, setStrategies] = useState<Strategy[]>([]);
  const [strategyId, setStrategyId] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listStrategies().then(setStrategies).catch(() => setStrategies([]));
  }, []);

  async function pickScreenshot() {
    try {
      const file = await open({
        title: "Select screenshot",
        multiple: false,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (file && !Array.isArray(file)) setScreenshotPath(file);
    } catch (e) {
      setError(String(e));
    }
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);

    if (!entry || !stop || !qty) {
      setError("Entry, stop, and quantity are required.");
      return;
    }
    if (Number(qty) <= 0) {
      setError("Quantity must be > 0.");
      return;
    }
    if (Number(entry) === Number(stop)) {
      setError("Entry and stop cannot be equal.");
      return;
    }
    if (side === "long" && Number(stop) >= Number(entry)) {
      setError("For a long trade, stop must be below entry.");
      return;
    }
    if (side === "short" && Number(stop) <= Number(entry)) {
      setError("For a short trade, stop must be above entry.");
      return;
    }
    if (takeProfit) {
      if (side === "long" && Number(takeProfit) <= Number(entry)) {
        setError("For a long trade, take profit must be above entry.");
        return;
      }
      if (side === "short" && Number(takeProfit) >= Number(entry)) {
        setError("For a short trade, take profit must be below entry.");
        return;
      }
    }

    const payload: NewTrade = {
      account_id: accountId,
      symbol: symbol.trim().toUpperCase(),
      side,
      entry_price: entry,
      opened_at: (() => {
        const parts = (openedTime || "00:00").split(":");
        const hh = Math.min(23, Math.max(0, parseInt(parts[0] || "0", 10)));
        const mm = Math.min(59, Math.max(0, parseInt(parts[1] || "0", 10)));
        const dayStr = openedDay || (() => {
          const d = new Date();
          const pad = (n: number) => String(n).padStart(2, "0");
          return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
        })();
        const [y, mo, da] = dayStr.split("-").map((v) => parseInt(v, 10));
        const d = new Date(y, (mo || 1) - 1, da || 1, hh, mm, 0, 0);
        return d.toISOString();
      })(),
      initial_stop: stop,
      take_profit: takeProfit || null,
      total_quantity: qty,
      size_type: sizeType,
      size_value: sizeValue || null,
      size_unit: sizeType === "money" ? sizeUnit : null,
      notes: notes.trim() || null,
      chart_url: chartUrl.trim() || null,
      screenshot_path: screenshotPath || null,
      strategy_id: strategyId ? Number(strategyId) : null,
      tags: tagsInput.split(",").map((t) => t.trim()).filter(Boolean),
      entry_legs: [{ price: entry, quantity: qty, cost_per_unit: cost || "0" }],
    };

    setBusy(true);
    try {
      await createTrade(payload);
      setEntry(""); setStop(""); setTakeProfit(""); setNotes("");
      setChartUrl(""); setScreenshotPath(""); setTagsInput("");
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="trade-form" onSubmit={submit}>
      {/* Row 1: Symbol, Side, Time, Entry, Stop, TP, Qty, Cost */}
      <div className="trade-form__row">
        <label>
          <span>Symbol</span>
          <input value={symbol} onChange={(e) => setSymbol(e.target.value)} placeholder="EURUSD" />
        </label>
        <label>
          <span>Side</span>
          <Select
            value={side}
            onChange={(v) => setSide(v as Side)}
            options={[
              { value: "long", label: "Long" },
              { value: "short", label: "Short" },
            ]}
          />
        </label>
        <label className="trade-form__time">
          <span>Opened at</span>
          <div className="trade-form__time-inner">
            <input
              type="date"
              value={openedDay}
              onChange={(e) => setOpenedDay(e.target.value)}
              className="trade-form__date"
            />
            <input
              type="number"
              min={0}
              max={23}
              step={1}
              value={openedTime.split(":")[0] ?? "00"}
              onChange={(e) => {
                const raw = e.target.value.replace(/[^0-9]/g, "").slice(0, 2);
                const hh = Math.min(23, Math.max(0, parseInt(raw || "0", 10)));
                const mm = openedTime.split(":")[1] ?? "00";
                setOpenedTime(String(hh).padStart(2, "0") + ":" + mm);
              }}
              className="trade-form__hh"
              placeholder="HH"
            />
            <span className="trade-form__colon">:</span>
            <input
              type="number"
              min={0}
              max={59}
              step={1}
              value={openedTime.split(":")[1] ?? "00"}
              onChange={(e) => {
                const raw = e.target.value.replace(/[^0-9]/g, "").slice(0, 2);
                const mm = Math.min(59, Math.max(0, parseInt(raw || "0", 10)));
                const hh = openedTime.split(":")[0] ?? "00";
                setOpenedTime(hh + ":" + String(mm).padStart(2, "0"));
              }}
              className="trade-form__mm"
              placeholder="MM"
            />
            <button
              type="button"
              className="trade-form__now"
              onClick={() => {
                const d = new Date();
                const pad = (n: number) => String(n).padStart(2, "0");
                setOpenedDay(`${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`);
                setOpenedTime(pad(d.getHours()) + ":" + pad(d.getMinutes()));
              }}
              title="Set to now (date + time)"
            >Now</button>
          </div>
        </label>
                <label>
          <span>Entry</span>
          <input value={entry} onChange={(e) => setEntry(e.target.value)} placeholder="1.1000" />
        </label>
        <label>
          <span>Stop</span>
          <input value={stop} onChange={(e) => setStop(e.target.value)} placeholder="1.0950" />
        </label>
        <label>
          <span>Take Profit<HelpTip title="Take Profit">Optional. The price where you plan to exit in profit. Used to compute the R:R preview and, when closing, to prefill the exit price if you pick "TP hit".</HelpTip></span>
          <input value={takeProfit} onChange={(e) => setTakeProfit(e.target.value)} placeholder="1.1150 (optional)" />
        </label>
        <label>
          <span>Quantity</span>
          <input value={qty} onChange={(e) => setQty(e.target.value)} placeholder="1.0" />
        </label>
        <label>
          <span>Cost / unit</span>
          <input value={cost} onChange={(e) => setCost(e.target.value)} placeholder="0" />
        </label>
              </div>

      {/* Row 2: Size type, size value, currency, strategy, tags */}
      <div className="trade-form__row trade-form__row--size">
        <label>
          <span>Size type<HelpTip title="Size type">How you express your position: lots (FX/CFD), units (crypto/stocks), or a direct money amount. This is for journaling — the app does not place trades.</HelpTip></span>
          <Select
            value={sizeType}
            onChange={(v) => setSizeType(v as SizeType)}
            options={[
              { value: "lots", label: "Lots" },
              { value: "units", label: "Units" },
              { value: "money", label: "Money" },
            ]}
          />
        </label>
        <label>
          <span>{sizeType === "lots" ? "Lot size" : sizeType === "units" ? "Units" : "Amount"}</span>
          <input
            value={sizeValue}
            onChange={(e) => setSizeValue(e.target.value)}
            placeholder={sizeType === "money" ? "5000" : "1.0"}
          />
        </label>
        {sizeType === "money" && (
          <label>
            <span>Currency</span>
            <input value={sizeUnit} onChange={(e) => setSizeUnit(e.target.value.toUpperCase())} maxLength={4} />
          </label>
        )}
        <label>
          <span>Strategy</span>
          <Select
            value={strategyId}
            onChange={(v) => setStrategyId(v)}
            options={[
              { value: "", label: "— none —" },
              ...strategies.map((s) => ({ value: String(s.id), label: s.name })),
            ]}
          />
        </label>
        <label>
          <span>Tags (comma separated)</span>
          <input value={tagsInput} onChange={(e) => setTagsInput(e.target.value)} placeholder="breakout, london, A+" />
        </label>
        <label>
          <span>Chart URL<HelpTip title="Chart URL">Paste a link to the TradingView or broker chart for this setup. It appears as a clickable icon in the trade list.</HelpTip></span>
          <input value={chartUrl} onChange={(e) => setChartUrl(e.target.value)} placeholder="https://tradingview.com/..." />
        </label>
        <label className="trade-form__screenshot">
          <span>Screenshot<HelpTip title="Screenshot">Save a snapshot of the chart before or after the trade. The path is stored in the database; the image itself stays on disk.</HelpTip></span>
          <div className="screenshot-picker">
            <input value={screenshotPath} readOnly placeholder="No file selected" />
            <button type="button" className="btn btn--ghost" onClick={pickScreenshot}>Browse…</button>
            {screenshotPath && (
              <button type="button" className="btn btn--ghost" onClick={() => setScreenshotPath("")}>Clear</button>
            )}
          </div>
        </label>
      </div>

      <label className="trade-form__notes">
        <span>Notes</span>
        <textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          rows={2}
          placeholder="Setup notes, invalidation, execution mistakes…"
        />
      </label>

      <RiskPreview
        side={side}
        entry={entry}
        stop={stop}
        takeProfit={takeProfit}
        qty={qty}
      />
      <PlannedRR entry={entry} stop={stop} takeProfit={takeProfit} />

      {error && <div className="trade-form__error">{error}</div>}

      <div className="trade-form__actions">
        {onCancel && <button type="button" className="btn btn--ghost" onClick={onCancel}>Cancel</button>}
        <button type="submit" className="btn btn--primary" disabled={busy}>
          {busy ? "Saving…" : "Add Trade"}
        </button>
      </div>
    </form>
  );
}

function computeRiskReward(
  side: Side, entry: string, stop: string, takeProfit: string, qty: string,
): { rr: string | null; risk: string | null; reward: string | null } {
  const e = Number(entry);
  const s = Number(stop);
  const tp = Number(takeProfit);
  const q = Number(qty);
  if (!Number.isFinite(e) || !Number.isFinite(s) || !Number.isFinite(q) || q <= 0) {
    return { rr: null, risk: null, reward: null };
  }
  const riskPerUnit = Math.abs(e - s);
  if (riskPerUnit === 0) return { rr: null, risk: null, reward: null };
  const risk = riskPerUnit * q;
  const riskStr = risk.toFixed(4);

  if (!takeProfit || !Number.isFinite(tp)) {
    return { rr: null, risk: riskStr, reward: null };
  }
  const direction = side === "long" ? 1 : -1;
  const rewardPerUnit = (tp - e) * direction;
  if (rewardPerUnit <= 0) {
    return { rr: "—", risk: riskStr, reward: null };
  }
  const reward = rewardPerUnit * q;
  const rr = reward / risk;
  return { rr: rr.toFixed(2), risk: riskStr, reward: reward.toFixed(4) };
}

function RiskPreview({ side, entry, stop, takeProfit, qty }: {
  side: Side; entry: string; stop: string; takeProfit: string; qty: string;
}) {
  const { rr, risk, reward } = computeRiskReward(side, entry, stop, takeProfit, qty);
  if (!risk) return null;

  return (
    <div className="risk-preview">
      <div className="risk-preview__item">
        <span className="risk-preview__label">Risk</span>
        <span className="risk-preview__value mono">{risk}</span>
      </div>
      {reward && (
        <div className="risk-preview__item">
          <span className="risk-preview__label">Reward</span>
          <span className="risk-preview__value mono">{reward}</span>
        </div>
      )}
      {rr && (
        <div className="risk-preview__item risk-preview__item--accent">
          <span className="risk-preview__label">R:R</span>
          <span className="risk-preview__value mono">1 : {rr}</span>
        </div>
      )}
    </div>
  );
}

function PlannedRR({ entry, stop, takeProfit }: {
  entry: string; stop: string; takeProfit: string;
}) {
  if (!entry || !stop || !takeProfit) return null;
  const e = Number(entry);
  const s = Number(stop);
  const tp = Number(takeProfit);
  if (!Number.isFinite(e) || !Number.isFinite(s) || !Number.isFinite(tp)) return null;
  const risk = Math.abs(e - s);
  if (risk === 0) return null;
  const reward = Math.abs(tp - e);
  const rr = reward / risk;
  return (
    <div className="plan-rr">
      <span className="plan-rr__label">Planned R:R</span>
      <span className="plan-rr__value mono">1 : {rr.toFixed(2)}</span>
    </div>
  );
}
