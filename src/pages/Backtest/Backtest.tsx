import { useCallback, useEffect, useState } from "react";
import {
  listTrades,
  listSetupTags,
  listSymbols,
  resetBacktest,
  todayISO,
  BacktestFilter,
  BacktestTrade,
  Outcome,
  SetupTagCount,
} from "../../lib/backtestApi";
import { QuickAdd } from "./QuickAdd";
import { TradeTable } from "./TradeTable";
import { DeleteModal } from "./DeleteModal";
import { Stats } from "./Stats";
import { Select } from "../../components/ui/Select";
import "./Backtest.css";

type Tab = "trades" | "analytics";

export function Backtest() {
  const [tab, setTab] = useState<Tab>("trades");
  const [trades, setTrades] = useState<BacktestTrade[]>([]);
  const [filter, setFilter] = useState<BacktestFilter>({});
  const [symbolOptions, setSymbolOptions] = useState<string[]>([]);
  const [setupOptions, setSetupOptions] = useState<SetupTagCount[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [confirmReset, setConfirmReset] = useState(false);
  const [tick, setTick] = useState(0);

  const reload = useCallback(async () => {
    try {
      const [rows, syms, tags] = await Promise.all([
        listTrades(filter),
        listSymbols(),
        listSetupTags(),
      ]);
      setTrades(rows);
      setSymbolOptions(syms);
      setSetupOptions(tags);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, [filter]);

  useEffect(() => {
    reload();
  }, [reload, tick]);

  function bump() {
    setTick((t) => t + 1);
  }

  async function doReset() {
    try {
      await resetBacktest();
      setConfirmReset(false);
      bump();
    } catch (e) {
      setError(String(e));
    }
  }

  const activeFilterCount =
    (filter.symbol ? 1 : 0) +
    (filter.setup_tag ? 1 : 0) +
    (filter.outcome ? 1 : 0) +
    (filter.date_from ? 1 : 0) +
    (filter.date_to ? 1 : 0);

  return (
    <div className="backtest">
      <header className="bt-header">
        <div>
          <h2 className="bt-title">Backtest</h2>
          <p className="bt-subtitle">
            Practice trades with no real money. All R and stats computed in Rust.
          </p>
        </div>
        <button
          className="btn btn--ghost bt-reset"
          onClick={() => setConfirmReset(true)}
          title="Delete all backtest trades"
        >
          Reset all
        </button>
      </header>

      <nav className="bt-tabs" role="tablist">
        <button
          role="tab"
          aria-selected={tab === "trades"}
          className={`bt-tab ${tab === "trades" ? "is-active" : ""}`}
          onClick={() => setTab("trades")}
        >
          Trades
          <span className="bt-tab__count">{trades.length}</span>
        </button>
        <button
          role="tab"
          aria-selected={tab === "analytics"}
          className={`bt-tab ${tab === "analytics" ? "is-active" : ""}`}
          onClick={() => setTab("analytics")}
        >
          Analytics
        </button>
      </nav>

      <section className="bt-filters card">
        <div className="bt-filter">
          <label>Symbol</label>
          <Select
            value={(filter.symbol ?? "") as string}
            onChange={(v) => setFilter((f) => ({ ...f, symbol: v || null }))}
            options={[
              { value: "", label: "All" },
              ...symbolOptions.map((s) => ({ value: s, label: s })),
            ]}
          />
        </div>

        <div className="bt-filter">
          <label>Setup</label>
          <Select
            value={(filter.setup_tag ?? "") as string}
            onChange={(v) => setFilter((f) => ({ ...f, setup_tag: v || null }))}
            options={[
              { value: "", label: "All" },
              ...setupOptions.map((s) => ({ value: s.tag, label: `${s.tag} (${s.count})` })),
            ]}
          />
        </div>

        <div className="bt-filter">
          <label>Outcome</label>
          <Select
            value={(filter.outcome ?? "") as string}
            onChange={(v) =>
              setFilter((f) => ({ ...f, outcome: (v || null) as Outcome | null }))
            }
            options={[
              { value: "", label: "All" },
              { value: "tp", label: "TP" },
              { value: "sl", label: "SL" },
              { value: "be", label: "BE" },
            ]}
          />
        </div>

        <div className="bt-filter">
          <label>From</label>
          <input
            type="date"
            value={filter.date_from ?? ""}
            max={filter.date_to ?? todayISO()}
            onChange={(e) =>
              setFilter((f) => ({ ...f, date_from: e.target.value || null }))
            }
          />
        </div>

        <div className="bt-filter">
          <label>To</label>
          <input
            type="date"
            value={filter.date_to ?? ""}
            min={filter.date_from ?? undefined}
            onChange={(e) =>
              setFilter((f) => ({ ...f, date_to: e.target.value || null }))
            }
          />
        </div>

        {activeFilterCount > 0 && (
          <button
            className="btn btn--ghost bt-filter__clear"
            onClick={() => setFilter({})}
          >
            Clear ({activeFilterCount})
          </button>
        )}
      </section>

      {error && <div className="bt-error">{error}</div>}

      {tab === "trades" && (
        <>
          <QuickAdd onCreated={bump} />
          <TradeTable trades={trades} onChanged={bump} />
        </>
      )}

      {tab === "analytics" && <Stats filter={filter} refreshKey={tick} />}

      <DeleteModal
        open={confirmReset}
        title="Reset all backtest trades?"
        message="This deletes every backtest trade. Live trades are NOT touched. This cannot be undone."
        confirmLabel="Reset all"
        onCancel={() => setConfirmReset(false)}
        onConfirm={doReset}
      />
    </div>
  );
}
