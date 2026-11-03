import { useEffect, useState } from "react";
import { Account } from "../../lib/accountsApi";
import { getGoals, setGoals, UpdateGoals } from "../../lib/goalsApi";

interface Props {
  account: Account;
  onClose: () => void;
  onSaved: () => void;
}

const DEFAULT_GOALS: UpdateGoals = {
  daily_target_pct: "6.0",
  weekly_target_pct: "18.0",
  monthly_target_pct: "24.0",
};

export function GoalsModal({ account, onClose, onSaved }: Props) {
  const [form, setForm] = useState<UpdateGoals | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const g = await getGoals(account.id);
        if (g) {
          setForm({
            daily_target_pct: g.daily_target_pct,
            weekly_target_pct: g.weekly_target_pct,
            monthly_target_pct: g.monthly_target_pct,
          });
        } else {
          setForm(DEFAULT_GOALS);
        }
      } catch (e) {
        setError(String(e));
      }
    })();
  }, [account.id]);

  async function save() {
    if (!form) return;
    setError(null);
    setSaved(false);
    setBusy(true);
    try {
      await setGoals(account.id, form);
      setSaved(true);
      onSaved();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  if (!form) {
    return (
      <div className="ca-backdrop" onClick={onClose}>
        <div className="ca-modal" onClick={(e) => e.stopPropagation()}>
          <div className="ca-modal__body">Loading…</div>
        </div>
      </div>
    );
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
            <h3 className="ca-modal__title">Goals · {account.name}</h3>
            <p className="ca-modal__subtitle">
              Profit targets for daily, weekly, and monthly performance
            </p>
          </div>
          <button className="ca-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ca-modal__body">
          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Daily target (%)</span>
              <input
                className="mono"
                value={form.daily_target_pct}
                onChange={(e) =>
                  setForm({ ...form, daily_target_pct: e.target.value })
                }
              />
              <em className="ca-hint">
                Example: 6% of $10,000 = $600 profit per day
              </em>
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Weekly target (%)</span>
              <input
                className="mono"
                value={form.weekly_target_pct}
                onChange={(e) =>
                  setForm({ ...form, weekly_target_pct: e.target.value })
                }
              />
              <em className="ca-hint">
                Example: 18% of $10,000 = $1,800 per week
              </em>
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Monthly target (%)</span>
              <input
                className="mono"
                value={form.monthly_target_pct}
                onChange={(e) =>
                  setForm({ ...form, monthly_target_pct: e.target.value })
                }
              />
              <em className="ca-hint">
                Example: 24% of $10,000 = $2,400 per month
              </em>
            </label>
          </div>

          {error && (
            <div className="ee-alert ee-alert--error" style={{ marginTop: 12 }}>
              {error}
            </div>
          )}
          {saved && (
            <div
              className="ee-alert ee-alert--success"
              style={{ marginTop: 12 }}
            >
              ✓ Saved
            </div>
          )}
        </div>

        <div className="ca-modal__footer">
          <button
            className="ca-btn ca-btn--ghost"
            onClick={() => {
              setForm(DEFAULT_GOALS);
              setSaved(false);
            }}
            disabled={busy}
            style={{ marginRight: "auto" }}
          >
            Reset defaults
          </button>
          <button className="ca-btn ca-btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="ca-btn ca-btn--primary"
            onClick={save}
            disabled={busy}
          >
            {busy ? "Saving…" : "Save changes"}
          </button>
        </div>
      </div>
    </div>
  );
}
