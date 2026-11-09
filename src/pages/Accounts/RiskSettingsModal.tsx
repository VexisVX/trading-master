import { useEffect, useState } from "react";
import { getRiskSettings, updateRiskSettings, UpdateRiskSettings } from "../../lib/riskSettingsApi";
import { Account } from "../../lib/accountsApi";

interface Props {
  account: Account;
  onClose: () => void;
  onSaved: () => void;
}

const DEFAULT_RISK: UpdateRiskSettings = {
  max_risk_per_trade_pct: "1.0",
  daily_loss_limit_pct: "3.0",
  max_open_positions: 3,
  max_portfolio_risk_pct: "6.0",
};

export function RiskSettingsModal({ account, onClose, onSaved }: Props) {
  const [form, setForm] = useState<UpdateRiskSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    (async () => {
      try {
        const rs = await getRiskSettings(account.id);
        if (rs) {
          setForm({
            max_risk_per_trade_pct: rs.max_risk_per_trade_pct,
            daily_loss_limit_pct: rs.daily_loss_limit_pct,
            max_open_positions: rs.max_open_positions,
            max_portfolio_risk_pct: rs.max_portfolio_risk_pct,
          });
        } else {
          setForm(DEFAULT_RISK);
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
      await updateRiskSettings(account.id, form);
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
            <h3 className="ca-modal__title">Risk Settings · {account.name}</h3>
            <p className="ca-modal__subtitle">
              Warnings only — does not block trades
            </p>
          </div>
          <button className="ca-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ca-modal__body">
          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Max risk per trade (%)</span>
              <input
                className="mono"
                value={form.max_risk_per_trade_pct}
                onChange={(e) =>
                  setForm({ ...form, max_risk_per_trade_pct: e.target.value })
                }
              />
            </label>
          </div>
          <div className="ca-form-row">
            <label className="ca-field" style={{ gridColumn: "1 / -1" }}>
              <span>Daily loss limit (%)</span>
              <input
                className="mono"
                value={form.daily_loss_limit_pct}
                onChange={(e) =>
                  setForm({ ...form, daily_loss_limit_pct: e.target.value })
                }
              />
            </label>
          </div>
          <div className="ca-form-row">
            <label className="ca-field">
              <span>Max open positions</span>
              <input
                type="number"
                min={1}
                max={100}
                value={form.max_open_positions}
                onChange={(e) =>
                  setForm({
                    ...form,
                    max_open_positions: Number(e.target.value),
                  })
                }
              />
            </label>
            <label className="ca-field">
              <span>Max portfolio risk (%)</span>
              <input
                className="mono"
                value={form.max_portfolio_risk_pct}
                onChange={(e) =>
                  setForm({
                    ...form,
                    max_portfolio_risk_pct: e.target.value,
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
              setForm(DEFAULT_RISK);
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
