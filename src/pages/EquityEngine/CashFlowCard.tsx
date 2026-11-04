import { useEffect, useState } from "react";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import {
  CashFlowRow,
  cashflowDeposit,
  cashflowWithdraw,
  cashflowFee,
  listCashFlows,
  deleteCashFlow,
  fmtMoney,
} from "../../lib/equityApi";

type Kind = "deposit" | "withdrawal" | "fee";

interface Props {
  accountId: number;
}

export function CashFlowCard({ accountId }: Props) {
  const [flows, setFlows] = useState<CashFlowRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [kind, setKind] = useState<Kind>("deposit");
  const [amount, setAmount] = useState<string>("");
  const [note, setNote] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [ok, setOk] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<CashFlowRow | null>(null);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      setFlows(await listCashFlows(accountId, 50));
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, [accountId]);
  // Auto-dismiss success message after 4 seconds
  useEffect(() => {
    if (!ok) return;
    const t = setTimeout(() => setOk(null), 4000);
    return () => clearTimeout(t);
  }, [ok]);

  async function submit() {
    if (!amount.trim()) {
      setErr("Amount required");
      return;
    }
    setBusy(true);
    setErr(null);
    setOk(null);
    try {
      const input = {
        account_id: accountId,
        amount: amount.trim(),
        note: note.trim() || null,
      };
      let r;
      if (kind === "deposit") r = await cashflowDeposit(input);
      else if (kind === "withdrawal") r = await cashflowWithdraw(input);
      else r = await cashflowFee(input);
      setOk(`✓ ${kind} ${r.amount} → units ${r.units_before} → ${r.units_after}`);
      setAmount("");
      setNote("");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    setErr(null);
    try {
      await deleteCashFlow(pendingDelete.id);
      setPendingDelete(null);
      await refresh();
    } catch (e) {
      setErr(String(e));
      setPendingDelete(null);
    }
  }


  return (
    <section className="ee-card ee-cashflow">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Cash Flow</h3>
          <p className="ee-card__subtitle">
            Deposits and withdrawals adjust units — NAV stays invariant
          </p>
        </div>
      </div>

      <div className="ee-cf-form ee-cf-form--separated">
        {(err || ok) && (
          <div className="ee-cf-alerts">
            {err && <div className="ee-alert ee-alert--error">{err}</div>}
            {ok && <div className="ee-alert ee-alert--success">{ok}</div>}
          </div>
        )}
        <div className="ee-cf-tabs">
          <button
            className={`ee-cf-tab ${kind === "deposit" ? "is-active" : ""}`}
            onClick={() => setKind("deposit")}
          >
            Deposit
          </button>
          <button
            className={`ee-cf-tab ${kind === "withdrawal" ? "is-active" : ""}`}
            onClick={() => setKind("withdrawal")}
          >
            Withdraw
          </button>
          <button
            className={`ee-cf-tab ${kind === "fee" ? "is-active" : ""}`}
            onClick={() => setKind("fee")}
          >
            Fee
          </button>
        </div>

        <div className="ee-cf-row">
          <label className="ee-cf-label">Amount</label>
          <input
            className="mono ee-cf-input"
            type="number"
            step="0.01"
            min="0"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
          />
          <label className="ee-cf-label">Note</label>
          <input
            className="ee-cf-input"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="optional"
          />
          <button
            className="ee-btn ee-btn--primary"
            onClick={submit}
            disabled={busy}
          >
            {busy ? "…" : kind === "deposit" ? "Deposit" : kind === "withdrawal" ? "Withdraw" : "Record fee"}
          </button>
        </div>


      </div>

      <div className="ee-cf-list-header">
        <span className="ee-cf-list-title">Recent cash flows</span>
        <span className="ee-cf-list-count">{flows.length} entries</span>
      </div>
      <div className="ee-cf-list-wrap">
        {loading && <div className="ee-muted">Loading…</div>}
        {!loading && flows.length === 0 && (
          <div className="ee-muted" style={{ padding: 12, textAlign: "center" }}>
            No cash flows yet.
          </div>
        )}
        {!loading && flows.length > 0 && (
          <table className="ee-cf-table">
            <thead>
              <tr>
                <th>When</th>
                <th>Kind</th>
                <th>Amount</th>
                <th>NAV</th>
                <th>Units before → after</th>
                <th>Note</th>
                <th style={{ width: 60 }}>Delete</th>
              </tr>
            </thead>
            <tbody>
              {flows.map((f) => (
                <tr key={f.id}>
                  <td className="mono">{f.occurred_at.slice(0, 19).replace("T", " ")}</td>
                  <td>
                    <span className={`ee-cf-kind ee-cf-kind--${f.kind}`}>{f.kind}</span>
                  </td>
                  <td className="mono">{fmtMoney(f.amount)} {f.currency}</td>
                  <td className="mono">{f.nav_before}</td>
                  <td className="mono">{f.units_before} → {f.units_after}</td>
                  <td>{f.note || "—"}</td>
                  <td>
                    <button
                      className="ee-cf-delete"
                      onClick={() => setPendingDelete(f)}
                      title="Delete this cash flow"
                      type="button"
                    >
                      ✕
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    
      {pendingDelete && (
        <ConfirmDialog
          title="Delete cash flow?"
          body={`This will permanently remove the ${pendingDelete.kind} of ${pendingDelete.amount} ${pendingDelete.currency} on ${pendingDelete.occurred_at.slice(0, 10)}. Units will be reverted. This cannot be undone.`}
          confirmLabel="Delete"
          cancelLabel="Cancel"
          tone="danger"
          onConfirm={confirmDelete}
          onCancel={() => setPendingDelete(null)}
        />
      )}

    </section>
  );
}
