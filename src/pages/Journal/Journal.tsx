import { useCallback, useEffect, useRef, useState } from "react";
import { TradeForm } from "./TradeForm";
import { QuickEntryForm } from "./QuickEntryForm";
import { JournalStats } from "./JournalStats";
import { TradeList } from "./TradeList";
import {
  addExit, listTrades, Trade, updateTrade, deleteTrade, exportTradesCsv,
  partialClose, TradeOutcome, SizeType,
} from "../../lib/tradesApi";
import { listStrategies, Strategy } from "../../lib/strategiesApi";
import { getActiveAccountId } from "../../lib/accountsApi";
import { save, open } from "@tauri-apps/plugin-dialog";
import { convertFileSrc } from "@tauri-apps/api/core";
import { SkeletonRows } from "../../components/ui/Skeleton";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { HelpTip } from "../../components/ui/HelpTip";
import "./Journal.css";

export function Journal() {
  const [accountId, setAccountId] = useState<number | null>(null);
  const [trades, setTrades] = useState<Trade[]>([]);
  const [showForm, setShowForm] = useState(true);
  const formRef = useRef<HTMLDivElement>(null);
  const [entryMode, setEntryMode] = useState<"quick" | "full">("quick");
  const [exitFor, setExitFor] = useState<Trade | null>(null);
  const [editFor, setEditFor] = useState<Trade | null>(null);
  const [screenshotFor, setScreenshotFor] = useState<string | null>(null);
  const [slFor, setSlFor] = useState<Trade | null>(null);
  const [confirmState, setConfirmState] = useState<{
    title: string; body: string; onConfirm: () => void;
  } | null>(null);
  const [strategies, setStrategies] = useState<Strategy[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      // ★ Always read fresh accountId from DB
      const activeId = await getActiveAccountId();
      setAccountId(activeId);
      const list = await listTrades(activeId, 200, 0);
      setTrades(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);  // no dependency on accountId

  useEffect(() => {
    console.log("[Journal] useEffect fired");
    (async () => {
      try {
        const id = await getActiveAccountId();
        console.log("[Journal] getActiveAccountId ->", id);
        setAccountId(id);
        console.log("[Journal] setAccountId called with", id);
      } catch (e) { setError(String(e)); }
      try {
        const list = await listStrategies();
        setStrategies(list);
      } catch { /* non-fatal */ }
      await refresh();
    })();
  }, [refresh]);

  function onDeleteTrade(t: Trade) {
    setConfirmState({
      title: `Delete trade #${t.id}?`,
      body: `This will permanently remove ${t.symbol} and all its legs, tags, and attachments. This cannot be undone.`,
      onConfirm: async () => {
        setConfirmState(null);
        try { await deleteTrade(t.id); refresh(); }
        catch (e) { setError(String(e)); }
      },
    });
  }

  async function onExportCsv() {
    try {
      const dest = await save({
        title: "Export trades to CSV",
        defaultPath: `trades-${new Date().toISOString().slice(0, 10)}.csv`,
        filters: [{ name: "CSV", extensions: ["csv"] }],
      });
      if (!dest) return;
      const bytes = await exportTradesCsv(dest);
      alert(`Exported ${bytes} bytes to ${dest}`);
    } catch (e) { setError(String(e)); }
  }

  // ★ Listen to account changes
  useEffect(() => {
    const handler = async (e: Event) => {
      const detail = (e as CustomEvent).detail as { id?: number } | undefined;
      if (detail?.id) {
        setAccountId(detail.id);
        await refresh();
      }
    };
    window.addEventListener("activeAccountChanged", handler);
    return () => window.removeEventListener("activeAccountChanged", handler);
  }, [refresh]);

  return (
    <div className="journal">
      <div className="journal__head">
        <h2>Trade Journal</h2>
        <div className="journal__actions">
          <button className="btn btn--ghost" onClick={() => setShowForm((v) => !v)}>
            {showForm ? "Hide form" : "New trade"}
          </button>
          <button className="btn btn--ghost" onClick={onExportCsv}>Export CSV</button>
          <button className="btn btn--ghost" onClick={refresh}>Refresh</button>
        </div>
      </div>

      {showForm && accountId !== null && (
        <>
          <div className="journal__mode-toggle">
            <button
              className={`mode-btn ${entryMode === "quick" ? "is-active" : ""}`}
              onClick={() => setEntryMode("quick")}
            >
              ⚡ Quick Entry
            </button>
            <button
              className={`mode-btn ${entryMode === "full" ? "is-active" : ""}`}
              onClick={() => setEntryMode("full")}
            >
              Full Form
            </button>
          </div>

          {entryMode === "quick" ? (
            <div className="journal__quick-layout">
              <div className="card pop-in" ref={formRef}>
                <QuickEntryForm accountId={accountId} onCreated={refresh} />
              </div>
              <JournalStats trades={trades} />
            </div>
          ) : (
            <div className="card pop-in" ref={formRef}>
              <TradeForm
                accountId={accountId}
                onCreated={() => { refresh(); }}
                onCancel={() => setShowForm(false)}
              />
            </div>
          )}
        </>
      )}

      {loading && <SkeletonRows rows={6} />}
      {error && <div className="card card--error">{error}</div>}

      {!loading && !error && (
        <div className="card">
          <TradeList
            trades={trades}
            strategies={strategies}
            onAddExit={setExitFor}
            onEdit={setEditFor}
            onDelete={onDeleteTrade}
            onViewScreenshot={setScreenshotFor}
            onViewSl={setSlFor}
          />
        </div>
      )}

      {confirmState && (
        <ConfirmDialog
          title={confirmState.title}
          body={confirmState.body}
          confirmLabel="Delete"
          cancelLabel="Keep"
          tone="danger"
          onConfirm={confirmState.onConfirm}
          onCancel={() => setConfirmState(null)}
        />
      )}

      {editFor && (
        <EditTradeDialog
          trade={editFor}
          strategies={strategies}
          onClose={() => setEditFor(null)}
          onDone={() => { setEditFor(null); refresh(); }}
          setError={setError}
        />
      )}

      {exitFor && (
        <ExitDialog
          trade={exitFor}
          onClose={() => setExitFor(null)}
          onDone={() => { setExitFor(null); refresh(); }}
        />
      )}

      {slFor && <SlHistoryModal trade={slFor} onClose={() => setSlFor(null)} />}

      {screenshotFor && (
        <div className="modal-backdrop" onClick={() => setScreenshotFor(null)}>
          <div className="modal modal--screenshot" onClick={(e) => e.stopPropagation()}>
            <button className="screenshot-close" onClick={() => setScreenshotFor(null)}>×</button>
            <ScreenshotImage path={screenshotFor} />
            <p className="screenshot-path mono">{screenshotFor}</p>
          </div>
        </div>
      )}
    </div>
  );
}

function ScreenshotImage({ path }: { path: string }) {
  const url = /^(https?:|data:|asset:)/.test(path) ? path : convertFileSrc(path);
  return (
    <img
      className="screenshot-img"
      src={url}
      alt="Trade screenshot"
      onError={(e) => { (e.target as HTMLImageElement).alt = "Could not load image"; }}
    />
  );
}

// ============================================================
// ExitDialog — supports partial close
// ============================================================
interface ExitDialogProps {
  trade: Trade;
  onClose: () => void;
  onDone: () => void;
}

function ExitDialog({ trade, onClose, onDone }: ExitDialogProps) {
  const totalQty = Number(trade.total_quantity);
  const [price, setPrice] = useState("");
  const [qty, setQty] = useState(trade.total_quantity);
  const [cost, setCost] = useState("0");
  const [outcome, setOutcome] = useState<TradeOutcome>(
    trade.outcome === "open" ? "manual_close" : trade.outcome
  );
  const [pnlSign, setPnlSign] = useState<"+" | "-">("+");
  const [pnlAmount, setPnlAmount] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const enteredQty = Number(qty) || 0;
  const isPartial = enteredQty > 0 && enteredQty < totalQty;
  const isFull = enteredQty >= totalQty && enteredQty > 0;
  const invalidQty = enteredQty <= 0 || enteredQty > totalQty;

  function pickOutcome(o: TradeOutcome) {
    setOutcome(o);
    if (o === "tp_hit" && trade.take_profit) setPrice(trade.take_profit);
    else if (o === "sl_hit" && trade.initial_stop) setPrice(trade.initial_stop);
    else if (o === "breakeven") { setPnlSign("+"); setPnlAmount("0"); }
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setErr(null);
    if (!price) { setErr("Exit price required."); return; }
    if (invalidQty) { setErr(`Quantity must be between 0.01 and ${totalQty}.`); return; }

    let manualPnl: string | null = null;
    if (pnlAmount.trim() !== "") {
      const n = Number(pnlAmount);
      if (!Number.isFinite(n)) { setErr("P&L amount must be a number."); return; }
      const sign = outcome === "sl_hit" ? "-" : outcome === "tp_hit" ? "+" : pnlSign;
      manualPnl = n === 0 ? "0" : `${sign}${Math.abs(n)}`;
    }

    setBusy(true);
    try {
      if (isPartial) {
        await partialClose(trade.id, price, qty, cost || "0", new Date().toISOString(), manualPnl);
      } else {
        await addExit(trade.id, price, qty, cost || "0", new Date().toISOString(), outcome, manualPnl);
      }
      onDone();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <form className="modal modal--exit" onClick={(e) => e.stopPropagation()} onSubmit={submit}>
        <h3>
          Close trade #{trade.id} · {trade.symbol}
          <HelpTip title="Close modes">
            Full close: exit the entire position. Partial close: exit part of it, keep the rest running.
          </HelpTip>
        </h3>
        <p className="dim">
          {trade.side} · entry {trade.entry_price} · total {trade.total_quantity}
          {trade.take_profit && <> · TP {trade.take_profit}</>}
        </p>

        <div className={`exit-mode ${isPartial ? "exit-mode--partial" : "exit-mode--full"}`}>
          {isPartial && `⚠ Partial close — ${((enteredQty / totalQty) * 100).toFixed(0)}% closed, ${(totalQty - enteredQty).toFixed(2)} stays open`}
          {isFull && `Full close — entire position will be closed`}
          {invalidQty && `Quantity must be between 0.01 and ${totalQty}`}
        </div>

        {isFull && (
          <div className="outcome-grid">
            <button type="button" className={`outcome-btn ${outcome === "tp_hit" ? "is-on outcome-btn--tp" : ""}`} onClick={() => pickOutcome("tp_hit")}>
              ✓ TP hit
            </button>
            <button type="button" className={`outcome-btn ${outcome === "sl_hit" ? "is-on outcome-btn--sl" : ""}`} onClick={() => pickOutcome("sl_hit")}>
              ✕ SL hit
            </button>
            <button type="button" className={`outcome-btn ${outcome === "manual_close" ? "is-on" : ""}`} onClick={() => pickOutcome("manual_close")}>
              Manual close
            </button>
            <button type="button" className={`outcome-btn ${outcome === "breakeven" ? "is-on" : ""}`} onClick={() => pickOutcome("breakeven")}>
              Breakeven
            </button>
          </div>
        )}

        <label><span>Exit price</span><input value={price} onChange={(e) => setPrice(e.target.value)} autoFocus /></label>

        <div className="exit-qty-row">
          <label><span>Quantity to close</span><input value={qty} onChange={(e) => setQty(e.target.value)} /></label>
          <div className="exit-qty-shortcuts">
            <button type="button" className="btn btn--ghost btn--small" onClick={() => setQty((totalQty / 2).toFixed(2))}>50%</button>
            <button type="button" className="btn btn--ghost btn--small" onClick={() => setQty(trade.total_quantity)}>All</button>
          </div>
        </div>

        <label><span>Cost / unit</span><input value={cost} onChange={(e) => setCost(e.target.value)} /></label>

        <div className="pnl-block">
          <span className="pnl-block__label">Manual P&L (optional)</span>
          <div className="pnl-block__row">
            <div className="pnl-sign">
              <button type="button" className={`pnl-sign__btn ${pnlSign === "+" ? "is-on pnl-sign__btn--plus" : ""}`} onClick={() => setPnlSign("+")}>+ Profit</button>
              <button type="button" className={`pnl-sign__btn ${pnlSign === "-" ? "is-on pnl-sign__btn--minus" : ""}`} onClick={() => setPnlSign("-")}>− Loss</button>
            </div>
            <input className="pnl-block__amount mono" value={pnlAmount} onChange={(e) => setPnlAmount(e.target.value)} placeholder="e.g. 420" />
          </div>
        </div>

        {err && <div className="trade-form__error">{err}</div>}

        <div className="modal__actions">
          <button type="button" className="btn btn--ghost" onClick={onClose}>Cancel</button>
          <button type="submit" className="btn btn--primary" disabled={busy || invalidQty}>
            {busy ? "Saving…" : isPartial ? "Partial Close" : "Record exit"}
          </button>
        </div>
      </form>
    </div>
  );
}

// ============================================================
// EditTradeDialog
// ============================================================
interface EditDialogProps {
  trade: Trade;
  strategies: { id: number; name: string }[];
  onClose: () => void;
  onDone: () => void;
  setError: (s: string | null) => void;
}

function EditTradeDialog({ trade, strategies, onClose, onDone, setError }: EditDialogProps) {
  const [symbol, setSymbol] = useState(trade.symbol);
  const [entry, setEntry] = useState(trade.entry_price);
  const [stop, setStop] = useState(trade.initial_stop);
  const [takeProfit, setTakeProfit] = useState(trade.take_profit ?? "");
  const [qty, setQty] = useState(trade.total_quantity);
  const [sizeType, setSizeType] = useState<SizeType>(trade.size_type);
  const [sizeValue, setSizeValue] = useState(trade.size_value ?? trade.total_quantity);
  const [sizeUnit, setSizeUnit] = useState(trade.size_unit ?? "USD");
  const [strategyId, setStrategyId] = useState<string>(trade.strategy_id ? String(trade.strategy_id) : "");
  const [notes, setNotes] = useState(trade.notes ?? "");
  const [chartUrl, setChartUrl] = useState(trade.chart_url ?? "");
  const [screenshotPath, setScreenshotPath] = useState(trade.screenshot_path ?? "");
  const [tagsInput, setTagsInput] = useState(trade.tags.join(", "));
  const [manualPnl, setManualPnl] = useState(trade.manual_pnl ?? "");
  const [busy, setBusy] = useState(false);

  async function pickScreenshot() {
    try {
      const file = await open({
        title: "Select screenshot",
        multiple: false,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (file && !Array.isArray(file)) setScreenshotPath(file);
    } catch (e) { setError(String(e)); }
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!symbol || !entry || !stop || !qty) { setError("All fields required."); return; }
    setBusy(true);
    try {
      // Copy screenshot if a new file was picked
      let finalScreenshotPath = screenshotPath;
      if (screenshotPath && screenshotPath !== trade.screenshot_path) {
        try {
          const { copyScreenshot } = await import("../../lib/tradesApi");
          finalScreenshotPath = await copyScreenshot(trade.id, screenshotPath);
          setScreenshotPath(finalScreenshotPath);
        } catch { /* non-fatal */ }
      }
      await updateTrade(trade.id, {
        symbol: symbol.trim().toUpperCase(),
        entry_price: entry,
        initial_stop: stop,
        take_profit: takeProfit || null,
        total_quantity: qty,
        outcome: trade.outcome,
        size_type: sizeType,
        size_value: sizeValue || null,
        size_unit: sizeType === "money" ? sizeUnit : null,
        manual_pnl: manualPnl || null,
        strategy_id: strategyId ? Number(strategyId) : null,
        notes: notes.trim() || null,
        chart_url: chartUrl.trim() || null,
        screenshot_path: finalScreenshotPath || null,
        tags: tagsInput.split(",").map((t) => t.trim()).filter(Boolean),
      });
      // Log SL move if stop changed
      if (stop !== trade.initial_stop) {
        try {
          const { logSlMove } = await import("../../lib/tradesApi");
          await logSlMove(trade.id, trade.initial_stop, stop, null);
        } catch { /* non-fatal */ }
      }
      onDone();
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <form className="modal modal--wide" onClick={(e) => e.stopPropagation()} onSubmit={submit}>
        <h3>Edit trade #{trade.id}</h3>

        <div className="modal__grid">
          <label><span>Symbol</span><input value={symbol} onChange={(e) => setSymbol(e.target.value)} /></label>
          <label><span>Entry</span><input value={entry} onChange={(e) => setEntry(e.target.value)} /></label>
          <label><span>Stop</span><input value={stop} onChange={(e) => setStop(e.target.value)} /></label>
          <label><span>Take Profit</span><input value={takeProfit} onChange={(e) => setTakeProfit(e.target.value)} /></label>
          <label><span>Quantity</span><input value={qty} onChange={(e) => setQty(e.target.value)} /></label>
          <label>
            <span>Size type</span>
            <select value={sizeType} onChange={(e) => setSizeType(e.target.value as SizeType)}>
              <option value="lots">Lots</option>
              <option value="units">Units</option>
              <option value="money">Money</option>
            </select>
          </label>
          <label>
            <span>{sizeType === "lots" ? "Lot size" : sizeType === "units" ? "Units" : "Amount"}</span>
            <input value={sizeValue} onChange={(e) => setSizeValue(e.target.value)} />
          </label>
          {sizeType === "money" && (
            <label><span>Currency</span><input value={sizeUnit} onChange={(e) => setSizeUnit(e.target.value.toUpperCase())} maxLength={4} /></label>
          )}
          <label>
            <span>Strategy</span>
            <select value={strategyId} onChange={(e) => setStrategyId(e.target.value)}>
              <option value="">— none —</option>
              {strategies.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
            </select>
          </label>
          <label className="modal__span-2"><span>Tags (comma separated)</span><input value={tagsInput} onChange={(e) => setTagsInput(e.target.value)} /></label>
          <label className="modal__span-2"><span>Chart URL</span><input value={chartUrl} onChange={(e) => setChartUrl(e.target.value)} /></label>
          <label className="modal__span-2">
            <span>Screenshot</span>
            <div className="screenshot-picker">
              <input value={screenshotPath} readOnly placeholder="No file selected" />
              <button type="button" className="btn btn--ghost" onClick={pickScreenshot}>Browse…</button>
              {screenshotPath && (
                <button type="button" className="btn btn--ghost" onClick={() => setScreenshotPath("")}>Clear</button>
              )}
            </div>
          </label>
          <label className="modal__span-2"><span>Notes</span><textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} /></label>
          <label className="modal__span-2">
            <span>Manual P&L (optional, overrides computed P&L)</span>
            <input value={manualPnl} onChange={(e) => setManualPnl(e.target.value)} placeholder="e.g. 420 or -200" />
          </label>
        </div>

        <div className="modal__actions">
          <button type="button" className="btn btn--ghost" onClick={onClose}>Cancel</button>
          <button type="submit" className="btn btn--primary" disabled={busy}>
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </form>
    </div>
  );
}


// ============================================================
// SL History Modal
// ============================================================
interface SlHistoryModalProps {
  trade: Trade;
  onClose: () => void;
}

function SlHistoryModal({ trade, onClose }: SlHistoryModalProps) {
  const [history, setHistory] = useState<[string, string, string | null, string][] | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const { getSlHistory } = await import("../../lib/tradesApi");
        setHistory(await getSlHistory(trade.id));
      } catch { setHistory([]); }
    })();
  }, [trade.id]);

  return (
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal modal--wide" onClick={(e) => e.stopPropagation()}>
        <button className="screenshot-close" onClick={onClose}>×</button>
        <h3>Stop-loss history · trade #{trade.id}</h3>
        {history === null && <p className="dim">Loading…</p>}
        {history && history.length === 0 && <p className="dim">No stop-loss moves recorded.</p>}
        {history && history.length > 0 && (
          <table className="trades" style={{ marginTop: 12 }}>
            <thead>
              <tr><th>Old</th><th>New</th><th>Reason</th><th>When</th></tr>
            </thead>
            <tbody>
              {history.map(([oldS, newS, reason, when], i) => (
                <tr key={i}>
                  <td className="mono">{oldS}</td>
                  <td className="mono">{newS}</td>
                  <td>{reason ?? "—"}</td>
                  <td className="mono dim">{when.slice(0, 16).replace("T", " ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <div className="modal__actions">
          <button className="btn btn--ghost" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
