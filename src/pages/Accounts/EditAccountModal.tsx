import { useEffect, useState } from "react";
import {
  Account, AccountKind,
  getTradesCount,
  updateAccount,
  resetAccount,
} from "../../lib/accountsApi";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";

interface Props {
  account: Account;
  onClose: () => void;
  onSaved: () => void;
}

export function EditAccountModal({ account, onClose, onSaved }: Props) {
  const [tradesCount, setTradesCount] = useState<number | null>(null);

  // form state
  const [name, setName] = useState(account.name);
  const [currency, setCurrency] = useState(account.currency);
  const [initialBalance, setInitialBalance] = useState(account.initial_balance);
  const [kind, setKind] = useState<AccountKind>(account.kind);

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [resetConfirm, setResetConfirm] = useState(false);

  // Load trades count
  useEffect(() => {
    (async () => {
      try {
        const n = await getTradesCount(account.id);
        setTradesCount(n);
      } catch {
        setTradesCount(0);
      }
    })();
  }, [account.id]);

  const locked = tradesCount !== null && tradesCount > 0;

  async function save() {
    setError(null);
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    setBusy(true);
    try {
      await updateAccount(account.id, {
        name: name.trim(),
        currency: locked ? undefined : currency,
        initial_balance: locked ? undefined : initialBalance,
        kind: locked ? undefined : kind,
      });
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function doReset() {
    setResetConfirm(false);
    setBusy(true);
    try {
      await resetAccount(account.id);
      setTradesCount(0);
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="ca-backdrop" onClick={onClose}>
      <div
        className="ca-modal"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 480 }}
      >
        <div className="ca-modal__header">
          <div>
            <h3 className="ca-modal__title">Edit Account</h3>
            <p className="ca-modal__subtitle">
              {locked
                ? `${tradesCount} trade${tradesCount === 1 ? "" : "s"} — only name can be changed`
                : "All fields are editable"}
            </p>
          </div>
          <button className="ca-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ca-modal__body">
          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Name</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="Account name"
              />
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field">
              <span>
                Currency {locked && <em className="ee-locked">🔒</em>}
              </span>
              <select
                value={currency}
                onChange={(e) => setCurrency(e.target.value)}
                disabled={locked}
              >
                <option>USD</option>
                <option>EUR</option>
                <option>GBP</option>
                <option>JPY</option>
                <option>CHF</option>
                <option>AUD</option>
                <option>CAD</option>
              </select>
            </label>
            <label className="ca-field">
              <span>
                Initial balance {locked && <em className="ee-locked">🔒</em>}
              </span>
              <input
                className="mono"
                value={initialBalance}
                onChange={(e) => setInitialBalance(e.target.value)}
                disabled={locked}
              />
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>
                Type {locked && <em className="ee-locked">🔒</em>}
              </span>
              <select
                value={kind}
                onChange={(e) => setKind(e.target.value as AccountKind)}
                disabled={locked}
              >
                <option value="personal">Personal</option>
                <option value="prop_firm">Prop firm</option>
              </select>
            </label>
          </div>

          {locked && (
            <div className="ee-alert ee-alert--warn" style={{ marginTop: 12 }}>
              ℹ This account has {tradesCount} trade
              {tradesCount === 1 ? "" : "s"}. To change currency, initial
              balance, or type, you must first reset the account.
            </div>
          )}

          {locked && (
            <div
              className="ee-reset-block"
              style={{
                marginTop: 16,
                padding: 14,
                border: "1px solid #ff6b6b",
                borderRadius: 8,
                background: "rgba(255, 107, 107, 0.04)",
              }}
            >
              <div
                style={{
                  fontSize: 12,
                  fontWeight: 600,
                  color: "#ff6b6b",
                  marginBottom: 6,
                }}
              >
                ⚠ Reset Account
              </div>
              <div
                style={{
                  fontSize: 12,
                  color: "var(--muted, #888)",
                  lineHeight: 1.6,
                  marginBottom: 10,
                }}
              >
                Deletes all {tradesCount} trade
                {tradesCount === 1 ? "" : "s"}, all snapshots, and all
                decisions for this account. The account itself is kept.
                This cannot be undone.
              </div>
              <button
                type="button"
                className="ca-btn"
                style={{
                  background: "#ff6b6b",
                  color: "#0b0f14",
                  border: "1px solid #ff6b6b",
                  fontWeight: 600,
                }}
                onClick={() => setResetConfirm(true)}
                disabled={busy}
              >
                🗑 Reset Account
              </button>
            </div>
          )}

          {error && (
            <div className="ee-alert ee-alert--error" style={{ marginTop: 12 }}>
              {error}
            </div>
          )}
        </div>

        <div className="ca-modal__footer">
          <button className="ca-btn ca-btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="ca-btn ca-btn--primary"
            onClick={save}
            disabled={busy || !name.trim()}
          >
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </div>

      {resetConfirm && (
        <ConfirmDialog
          title={`Reset account "${account.name}"?`}
          body={`This will permanently delete all ${tradesCount ?? 0} trade${
            tradesCount === 1 ? "" : "s"
          }, all equity snapshots, and all decisions for this account. The account itself (name, currency, type) stays. Cannot be undone.`}
          confirmLabel="Yes, reset everything"
          cancelLabel="Cancel"
          tone="danger"
          onConfirm={doReset}
          onCancel={() => setResetConfirm(false)}
        />
      )}
    </div>
  );
}
