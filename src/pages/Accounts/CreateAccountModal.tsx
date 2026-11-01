import { useMemo, useState } from "react";
import { fmtMoney } from "../../lib/equityApi";

type Kind = "personal" | "prop_firm";
type Currency = "USD" | "EUR" | "GBP" | "JPY" | "CHF" | "AUD" | "CAD";

interface Props {
  onClose: () => void;
  onCreated: () => void;
  onCreate: (data: {
    name: string;
    currency: Currency;
    initial_balance: string;
    kind: Kind;
    prop_firm?: {
      name: string;
      max_daily_loss_pct: string;
      max_total_drawdown_pct: string;
      profit_target_pct: string;
    };
  }) => Promise<void>;
}

export function CreateAccountModal({ onClose, onCreated, onCreate }: Props) {
  const [kind, setKind] = useState<Kind>("personal");
  const [name, setName] = useState("");
  const [currency, setCurrency] = useState<Currency>("USD");
  const [initialBalance, setInitialBalance] = useState("10000");

  // Prop firm fields
  const [firmName, setFirmName] = useState("FTMO");
  const [maxDailyLoss, setMaxDailyLoss] = useState("5");
  const [maxDrawdown, setMaxDrawdown] = useState("10");
  const [profitTarget, setProfitTarget] = useState("10");

  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const initialNum = parseFloat(initialBalance) || 0;

  // Derived $ amounts
  const dailyLossUsd = useMemo(
    () => initialNum * (parseFloat(maxDailyLoss) / 100 || 0),
    [initialNum, maxDailyLoss]
  );
  const maxDdUsd = useMemo(
    () => initialNum * (parseFloat(maxDrawdown) / 100 || 0),
    [initialNum, maxDrawdown]
  );
  const profitTargetUsd = useMemo(
    () => initialNum * (parseFloat(profitTarget) / 100 || 0),
    [initialNum, profitTarget]
  );

  const canCreate =
    name.trim().length > 0 &&
    initialNum > 0 &&
    (kind === "personal" ||
      (firmName.trim().length > 0 &&
        parseFloat(maxDailyLoss) > 0 &&
        parseFloat(maxDrawdown) > 0));

  async function handleCreate() {
    setError(null);
    setBusy(true);
    try {
      await onCreate({
        name: name.trim(),
        currency,
        initial_balance: initialBalance,
        kind,
        prop_firm:
          kind === "prop_firm"
            ? {
                name: firmName.trim(),
                max_daily_loss_pct: (parseFloat(maxDailyLoss) / 100).toFixed(4),
                max_total_drawdown_pct: (parseFloat(maxDrawdown) / 100).toFixed(4),
                profit_target_pct: (parseFloat(profitTarget) / 100).toFixed(4),
              }
            : undefined,
      });
      onCreated();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      className="ee-modal-backdrop"
      onClick={onClose}
      style={{ zIndex: 1200 }}
    >
      <div
        className="ee-modal"
        onClick={(e) => e.stopPropagation()}
        style={{ maxWidth: 620 }}
      >
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Create New Account</h3>
            <p className="ee-modal__subtitle">
              Set up a personal or prop firm account
            </p>
          </div>
          <button className="ee-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ee-modal__body">
          {/* Account type selector */}
          <div className="ca-type-label">Account type</div>
          <div className="ca-type-grid">
            <button
              className={`ca-type-card ${kind === "personal" ? "is-active" : ""}`}
              onClick={() => setKind("personal")}
              type="button"
            >
              <div className="ca-type-card__icon">👤</div>
              <div className="ca-type-card__title">Personal</div>
              <div className="ca-type-card__desc">
                Your own money. No external rules.
              </div>
            </button>
            <button
              className={`ca-type-card ${kind === "prop_firm" ? "is-active" : ""}`}
              onClick={() => setKind("prop_firm")}
              type="button"
            >
              <div className="ca-type-card__icon">🏛</div>
              <div className="ca-type-card__title">Prop Firm</div>
              <div className="ca-type-card__desc">
                Firm's money. Strict rules apply.
              </div>
            </button>
          </div>

          {/* Basic info */}
          <div className="ca-section-title">Account details</div>
          <div className="ee-form-row">
            <label className="ee-field" style={{ gridColumn: "1 / -1" }}>
              <span>Account name *</span>
              <input
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="e.g. My Personal, FTMO 100k"
              />
            </label>
          </div>

          <div className="ee-form-row">
            <label className="ee-field">
              <span>Initial balance *</span>
              <input
                className="mono"
                value={initialBalance}
                onChange={(e) => setInitialBalance(e.target.value)}
                placeholder="10000"
              />
            </label>
            <label className="ee-field">
              <span>Currency</span>
              <select
                className="ee-select"
                value={currency}
                onChange={(e) => setCurrency(e.target.value as Currency)}
                style={{ width: "100%", padding: "10px 12px", fontSize: 13 }}
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
          </div>

          {/* Prop firm fields */}
          {kind === "prop_firm" && (
            <>
              <div className="ca-section-title">Prop firm rules</div>
              <div className="ee-form-row">
                <label className="ee-field" style={{ gridColumn: "1 / -1" }}>
                  <span>Firm name</span>
                  <input
                    value={firmName}
                    onChange={(e) => setFirmName(e.target.value)}
                    placeholder="FTMO, MyForexFunds, etc."
                  />
                </label>
              </div>
              <div className="ee-form-row">
                <label className="ee-field">
                  <span>Max daily loss</span>
                  <input
                    className="mono"
                    value={maxDailyLoss}
                    onChange={(e) => setMaxDailyLoss(e.target.value)}
                    placeholder="5"
                  />
                  <em className="ee-hint">
                    % of balance → {fmtMoney(dailyLossUsd)}
                  </em>
                </label>
                <label className="ee-field">
                  <span>Max total drawdown</span>
                  <input
                    className="mono"
                    value={maxDrawdown}
                    onChange={(e) => setMaxDrawdown(e.target.value)}
                    placeholder="10"
                  />
                  <em className="ee-hint">
                    % of balance → {fmtMoney(maxDdUsd)}
                  </em>
                </label>
                <label className="ee-field">
                  <span>Profit target</span>
                  <input
                    className="mono"
                    value={profitTarget}
                    onChange={(e) => setProfitTarget(e.target.value)}
                    placeholder="10"
                  />
                  <em className="ee-hint">
                    % of balance → {fmtMoney(profitTargetUsd)}
                  </em>
                </label>
              </div>

              {/* Explanation */}
              <div className="ca-explainer">
                <div className="ca-explainer__title">
                  💡 What this means for your trading
                </div>
                <ul className="ca-explainer__list">
                  <li>
                    You can lose up to{" "}
                    <strong>{fmtMoney(dailyLossUsd)}</strong> in one day. After
                    that, no more trades today.
                  </li>
                  <li>
                    Total drawdown cannot exceed{" "}
                    <strong>{fmtMoney(maxDdUsd)}</strong>. If reached, the
                    account is closed.
                  </li>
                  <li>
                    To pass the challenge, reach{" "}
                    <strong>{fmtMoney(profitTargetUsd)}</strong> profit.
                  </li>
                </ul>
              </div>
            </>
          )}

          {kind === "personal" && (
            <div className="ca-explainer ca-explainer--info">
              <div className="ca-explainer__title">
                💡 Personal account
              </div>
              <ul className="ca-explainer__list">
                <li>
                  Your own capital — no external rules or challenges.
                </li>
                <li>
                  The Equity Engine still protects your <strong>profit</strong>{" "}
                  (via profit retention) but not your initial capital.
                </li>
                <li>
                  Configure advanced protection in Settings (trailing drawdown,
                  daily loss limit).
                </li>
              </ul>
            </div>
          )}

          {error && (
            <div className="ee-alert ee-alert--error" style={{ marginTop: 12 }}>
              {error}
            </div>
          )}
        </div>

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose} disabled={busy}>
            Cancel
          </button>
          <button
            className="ee-btn ee-btn--primary"
            onClick={handleCreate}
            disabled={!canCreate || busy}
          >
            {busy ? "Creating…" : "Create Account"}
          </button>
        </div>
      </div>
    </div>
  );
}
