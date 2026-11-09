import { useEffect, useState } from "react";
import { getPropFirmRules, updatePropFirmRules, UpdatePropFirmRules } from "../../lib/propFirmApi";
import { Account } from "../../lib/accountsApi";

interface Props {
  account: Account;
  onClose: () => void;
  onSaved: () => void;
}

const DEFAULT_PF: UpdatePropFirmRules = {
  enabled: false,
  max_dd_pct: "10.0",
  trailing_dd: true,
  daily_loss_limit_pct: "5.0",
  max_total_loss_pct: "10.0",
  profit_target_pct: "8.0",
  min_trading_days: 5,
};

export function PropFirmModal({ account, onClose, onSaved }: Props) {
  const [form, setForm] = useState<UpdatePropFirmRules | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const pfr = await getPropFirmRules(account.id);
        if (pfr) {
          setForm({
            enabled: pfr.enabled,
            max_dd_pct: pfr.max_dd_pct,
            trailing_dd: pfr.trailing_dd,
            daily_loss_limit_pct: pfr.daily_loss_limit_pct,
            max_total_loss_pct: pfr.max_total_loss_pct,
            profit_target_pct: pfr.profit_target_pct,
            min_trading_days: pfr.min_trading_days,
          });
        } else {
          setForm(DEFAULT_PF);
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
      await updatePropFirmRules(account.id, form);
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
            <h3 className="ca-modal__title">
              Prop-firm Rules · {account.name}
            </h3>
            <p className="ca-modal__subtitle">
              Evaluation rules — warnings only
            </p>
          </div>
          <button className="ca-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ca-modal__body">
          <label
            style={{
              display: "flex",
              flexDirection: "row",
              alignItems: "center",
              gap: 8,
              marginBottom: 14,
            }}
          >
            <input
              type="checkbox"
              checked={form.enabled}
              onChange={(e) => setForm({ ...form, enabled: e.target.checked })}
            />
            <span style={{ fontSize: 13 }}>Enable prop-firm evaluation</span>
          </label>

          <div className="ca-form-row">
            <label className="ca-field">
              <span>Max drawdown (%)</span>
              <input
                className="mono"
                value={form.max_dd_pct}
                onChange={(e) =>
                  setForm({ ...form, max_dd_pct: e.target.value })
                }
              />
            </label>
            <label
              className="ca-field"
              style={{
                flexDirection: "row",
                alignItems: "center",
                gap: 8,
                justifyContent: "flex-end",
              }}
            >
              <input
                type="checkbox"
                checked={form.trailing_dd}
                onChange={(e) =>
                  setForm({ ...form, trailing_dd: e.target.checked })
                }
              />
              <span style={{ fontSize: 12 }}>Trailing</span>
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field">
              <span>Daily loss limit (%)</span>
              <input
                className="mono"
                value={form.daily_loss_limit_pct}
                onChange={(e) =>
                  setForm({ ...form, daily_loss_limit_pct: e.target.value })
                }
              />
            </label>
            <label className="ca-field">
              <span>Max total loss (%)</span>
              <input
                className="mono"
                value={form.max_total_loss_pct}
                onChange={(e) =>
                  setForm({ ...form, max_total_loss_pct: e.target.value })
                }
              />
            </label>
          </div>

          <div className="ca-form-row">
            <label className="ca-field">
              <span>Profit target (%)</span>
              <input
                className="mono"
                value={form.profit_target_pct}
                onChange={(e) =>
                  setForm({ ...form, profit_target_pct: e.target.value })
                }
              />
            </label>
            <label className="ca-field">
              <span>Min trading days</span>
              <input
                type="number"
                min={0}
                max={365}
                value={form.min_trading_days}
                onChange={(e) =>
                  setForm({
                    ...form,
                    min_trading_days: Number(e.target.value),
                  })
                }
              />
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
              setForm(DEFAULT_PF);
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
