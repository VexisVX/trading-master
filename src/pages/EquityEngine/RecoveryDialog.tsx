import { useState } from "react";
import { grantRecovery } from "../../lib/equityApi";

interface Props {
  accountId: number;
  currentState: string;
  onClose: () => void;
  onGranted: () => void;
}

export function RecoveryDialog({ accountId, currentState, onClose, onGranted }: Props) {
  const [reason, setReason] = useState("");
  const [expiresInHours, setExpiresInHours] = useState<string>("24");
  const [probeRiskPct, setProbeRiskPct] = useState<string>("0.25");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [confirmed, setConfirmed] = useState(false);

  async function submit() {
    if (!confirmed) {
      setErr("You must acknowledge the risks before continuing.");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await grantRecovery({
        account_id: accountId,
        reason: reason.trim() || null,
        expires_in_hours: expiresInHours ? parseInt(expiresInHours, 10) : null,
        probe_risk_pct: probeRiskPct,
      });
      onGranted();
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div
        className="ee-modal ee-modal--wide"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">🔬 Enable Recovery Mode</h3>
            <p className="ee-modal__subtitle">
              Current state: <strong>{currentState}</strong>
            </p>
          </div>
          <button className="ee-modal__close" onClick={onClose}>✕</button>
        </div>

        <div className="ee-modal__body">
          <div className="ee-alert ee-alert--warn" style={{ marginBottom: 16 }}>
            <strong>⚠ Read before continuing</strong>
            <p style={{ margin: "8px 0 0" }}>
              Recovery Mode allows <strong>small probe trades</strong> while you are
              in or near the preservation floor. It does NOT bypass:
            </p>
            <ul style={{ margin: "8px 0 0 20px", padding: 0 }}>
              <li>Prop firm breach rules</li>
              <li>Daily loss limits</li>
              <li>Portfolio risk limits</li>
              <li>Instrument / margin constraints</li>
            </ul>
            <p style={{ margin: "8px 0 0" }}>
              Recovery mode will be automatically revoked when state returns to
              Protected or Normal Growth, or when the time limit expires.
            </p>
          </div>

          <div className="ee-rec-field">
            <label>Reason (optional)</label>
            <input
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder="e.g. Small probe to test setup"
            />
          </div>

          <div className="ee-rec-field">
            <label>Expires in (hours)</label>
            <input
              type="number"
              min="1"
              max="720"
              value={expiresInHours}
              onChange={(e) => setExpiresInHours(e.target.value)}
            />
          </div>

          <div className="ee-rec-field">
            <label>Probe risk fraction of R_base</label>
            <input
              type="number"
              step="0.05"
              min="0.05"
              max="1"
              value={probeRiskPct}
              onChange={(e) => setProbeRiskPct(e.target.value)}
            />
            <span className="ee-hint">0.25 = 25% of base risk</span>
          </div>

          <label className="ee-rec-confirm">
            <input
              type="checkbox"
              checked={confirmed}
              onChange={(e) => setConfirmed(e.target.checked)}
            />
            <span>
              I understand that Recovery Mode does not bypass prop or daily limits,
              and I authorize these probe trades for this account.
            </span>
          </label>

          {err && <div className="ee-alert ee-alert--error">{err}</div>}
        </div>

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            className="ee-btn ee-btn--primary"
            onClick={submit}
            disabled={busy || !confirmed}
          >
            {busy ? "Enabling…" : "Enable Recovery Mode"}
          </button>
        </div>
      </div>
    </div>
  );
}
