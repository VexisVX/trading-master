import { useEffect, useState } from "react";
import {
  createTrade,
  listSymbols,
  Side,
} from "../../lib/backtestApi";
import { SetupInput } from "./SetupInput";
import { Select } from "../../components/ui/Select";

interface Props {
  onCreated: () => void;
}

/**
 * QuickAdd — inline add form.
 *
 * R mode (toggle "R" button):
 *   - off (default): Exit → Rust computes R
 *   - on:            Manual R → Rust stores as-is (outcome derived from sign)
 *
 * Advanced ("+"): adds TP · Setup · Date-time.
 * Rust enforces XOR between exit_price and manual_r.
 */
export function QuickAdd({ onCreated }: Props) {
  // simple fields
  const [symbol, setSymbol] = useState("");
  const [side, setSide] = useState<Side>("long");
  const [entry, setEntry] = useState("");
  const [stop, setStop] = useState("");
  const [exit, setExit] = useState("");

  // R-mode
  const [manualMode, setManualMode] = useState(false);
  const [manualR, setManualR] = useState("");

  // advanced
  const [advanced, setAdvanced] = useState(false);
  const [tp, setTp] = useState("");
  const [setup, setSetup] = useState("");
  const [openedAt, setOpenedAt] = useState<string>("");

  const [symbols, setSymbols] = useState<string[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listSymbols().then(setSymbols).catch(() => setSymbols([]));
  }, []);

  function reset() {
    setEntry("");
    setStop("");
    setExit("");
    setManualR("");
    setTp("");
    setSetup("");
    setOpenedAt("");
  }

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!symbol.trim()) return setError("Symbol is required");

    if (manualMode) {
      if (!manualR.trim()) return setError("R is required in R mode");
    } else {
      if (!entry || !stop) return setError("Entry and stop are required");
    }

    setBusy(true);
    try {
      await createTrade({
        symbol: symbol.trim().toUpperCase(),
        side,
        // In R mode, entry/stop are irrelevant → send "0" (Rust also accepts blank and defaults to "0")
        entry_price: manualMode ? "0" : entry,
        stop_price: manualMode ? "0" : stop,
        take_profit: advanced && tp ? tp : null,
        exit_price: manualMode ? null : exit || null,
        manual_r: manualMode ? manualR : null,
        setup_tag: advanced && setup.trim() ? setup.trim() : null,
        opened_at: advanced && openedAt ? new Date(openedAt).toISOString() : null,
      });
      reset();
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="bt-quickadd card" onSubmit={submit}>
      <div className="bt-quickadd__row">
        <input
          className="mono bt-quickadd__sym"
          value={symbol}
          onChange={(e) => setSymbol(e.target.value.toUpperCase())}
          placeholder="EURUSD"
          list="bt-symbols"
        />
        <datalist id="bt-symbols">
          {symbols.map((s) => (
            <option key={s} value={s} />
          ))}
        </datalist>

        <div className="bt-quickadd__side-wrap">
          <Select
            value={side}
            onChange={(v) => setSide(v as Side)}
            options={[
              { value: "long", label: "Long" },
              { value: "short", label: "Short" },
            ]}
          />
        </div>

        {!manualMode && (
          <>
  <input
            className="mono"
            value={entry}
            onChange={(e) => setEntry(e.target.value)}
            placeholder="Entry"
          />
  <input
            className="mono"
            value={stop}
            onChange={(e) => setStop(e.target.value)}
            placeholder="Stop"
          />
          </>
        )}

        <button
          type="button"
          className={`bt-quickadd__rmode ${manualMode ? "is-active" : ""}`}
          onClick={() => {
            setManualMode((v) => !v);
            // Clear the other field so we never submit both
            if (!manualMode) setExit("");
            else setManualR("");
          }}
          title={
            manualMode
              ? "Compute mode (use Exit) — click to switch back"
              : "Manual R mode — type R directly"
          }
        >
          R
        </button>

        {manualMode ? (
          <input
            className="mono bt-quickadd__r"
            value={manualR}
            onChange={(e) => setManualR(e.target.value)}
            placeholder="R (e.g. 1.5)"
            title="Type R directly, e.g. 1.5 or -0.8"
          />
        ) : (
          <input
            className="mono bt-quickadd__exit"
            value={exit}
            onChange={(e) => setExit(e.target.value)}
            placeholder="Exit (opt)"
            title="Leave empty for an open trade"
          />
        )}

        <button
          type="button"
          className="bt-quickadd__toggle"
          onClick={() => setAdvanced((v) => !v)}
          title={advanced ? "Hide advanced" : "Show advanced"}
        >
          {advanced ? "−" : "+"}
        </button>

        <button className="btn btn--primary bt-quickadd__submit" disabled={busy}>
          {busy ? "…" : "Add"}
        </button>
      </div>

      {advanced && (
        <div className="bt-quickadd__row bt-quickadd__row--advanced">
          <input
            className="mono"
            value={tp}
            onChange={(e) => setTp(e.target.value)}
            placeholder="TP (opt)"
          />
          <div className="bt-quickadd__setup">
            <SetupInput
              value={setup}
              onChange={setSetup}
              placeholder="Setup (opt)"
            />
          </div>
          <input
            type="datetime-local"
            className="bt-quickadd__date"
            value={openedAt}
            onChange={(e) => setOpenedAt(e.target.value)}
            title="Leave empty for now()"
          />
        </div>
      )}

      {error && <div className="bt-error">{error}</div>}
    </form>
  );
}
