import { useMemo, useState } from "react";
import {
  BacktestTrade,
  deleteTrade,
  formatDateShort,
  formatR,
  parseR,
} from "../../lib/backtestApi";
import { DeleteModal } from "./DeleteModal";

interface Props {
  trades: BacktestTrade[];
  onChanged: () => void;
}

type SortKey = "opened_at" | "symbol" | "r_multiple";

export function TradeTable({ trades, onChanged }: Props) {
  const [sortKey, setSortKey] = useState<SortKey>("opened_at");
  const [sortAsc, setSortAsc] = useState(false);
  const [pendingDelete, setPendingDelete] = useState<BacktestTrade | null>(null);

  const sorted = useMemo(() => {
    const copy = [...trades];
    copy.sort((a, b) => {
      let cmp = 0;
      if (sortKey === "opened_at") cmp = a.opened_at.localeCompare(b.opened_at);
      else if (sortKey === "symbol") cmp = a.symbol.localeCompare(b.symbol);
      else {
        const ra = parseR(a.r_multiple);
        const rb = parseR(b.r_multiple);
        cmp = (ra ?? -Infinity) - (rb ?? -Infinity);
      }
      return sortAsc ? cmp : -cmp;
    });
    return copy;
  }, [trades, sortKey, sortAsc]);

  function toggleSort(k: SortKey) {
    if (k === sortKey) setSortAsc((x) => !x);
    else {
      setSortKey(k);
      setSortAsc(false);
    }
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    try {
      await deleteTrade(pendingDelete.id);
      setPendingDelete(null);
      onChanged();
    } catch (e) {
      alert(String(e));
    }
  }

  if (trades.length === 0) {
    return <div className="bt-empty">No trades yet. Add one above.</div>;
  }

  return (
    <>
      <div className="bt-table-wrap card">
        <table className="bt-table">
          <thead>
            <tr>
              <th onClick={() => toggleSort("opened_at")} className="is-sortable">
                Date{sortKey === "opened_at" ? (sortAsc ? " ↑" : " ↓") : ""}
              </th>
              <th onClick={() => toggleSort("symbol")} className="is-sortable">
                Symbol{sortKey === "symbol" ? (sortAsc ? " ↑" : " ↓") : ""}
              </th>
              <th>Side</th>
              <th>Entry</th>
              <th>Stop</th>
              <th>Exit</th>
              <th onClick={() => toggleSort("r_multiple")} className="is-sortable">
                R{sortKey === "r_multiple" ? (sortAsc ? " ↑" : " ↓") : ""}
              </th>
              <th>Setup</th>
              <th className="bt-table__act"></th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((t) => {
              const r = parseR(t.r_multiple);
              const rClass =
                r === null ? "" : r > 0 ? "pos" : r < 0 ? "neg" : "";
              return (
                <tr key={t.id}>
                  <td className="mono bt-muted">{formatDateShort(t.opened_at)}</td>
                  <td className="mono">{t.symbol}</td>
                  <td>
                    <span className={`bt-side bt-side--${t.side}`}>
                      {t.side === "long" ? "L" : "S"}
                    </span>
                  </td>
                  <td className="mono">{t.entry_price === "0" ? "—" : t.entry_price}</td>
                  <td className="mono">{t.stop_price === "0" ? "—" : t.stop_price}</td>
                  <td className="mono">{t.exit_price ?? "—"}</td>
                  <td className={`mono ${rClass}`}>{formatR(t.r_multiple)}</td>
                  <td>
                    {t.setup_tag ? (
                      <span className="bt-chip">{t.setup_tag}</span>
                    ) : (
                      <span className="bt-muted">—</span>
                    )}
                  </td>
                  <td className="bt-table__act">
                    <button
                      className="icon-btn bt-row-del"
                      title="Delete"
                      onClick={() => setPendingDelete(t)}
                    >
                      ✕
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <DeleteModal
        open={pendingDelete !== null}
        title="Delete trade?"
        message={
          pendingDelete
            ? `${pendingDelete.symbol} ${pendingDelete.side} @ ${pendingDelete.entry_price} — this cannot be undone.`
            : ""
        }
        onCancel={() => setPendingDelete(null)}
        onConfirm={confirmDelete}
      />
    </>
  );
}
