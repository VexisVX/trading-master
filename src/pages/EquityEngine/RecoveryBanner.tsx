import { RecoveryStatus, revokeRecovery } from "../../lib/equityApi";
import { useState } from "react";

interface Props {
  accountId: number;
  status: RecoveryStatus;
  onRevoked: () => void;
}

export function RecoveryBanner({ accountId, status, onRevoked }: Props) {
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  async function doRevoke() {
    setBusy(true);
    setErr(null);
    try {
      await revokeRecovery(accountId);
      onRevoked();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  const expiresLabel = status.expires_at
    ? new Date(status.expires_at).toLocaleString()
    : "no expiry";

  return (
    <div className="ee-recovery-banner">
      <div className="ee-recovery-banner__icon">🔬</div>
      <div className="ee-recovery-banner__body">
        <div className="ee-recovery-banner__title">Recovery Mode is ACTIVE</div>
        <div className="ee-recovery-banner__detail">
          Probe risk: {(parseFloat(status.probe_risk_pct ?? "0.25") * 100).toFixed(0)}% of R_base
          {" · "}
          Expires: {expiresLabel}
          {status.reason && <> · Reason: {status.reason}</>}
        </div>
        {err && <div className="ee-recovery-banner__err">{err}</div>}
      </div>
      <button
        className="ee-btn ee-btn--ghost"
        onClick={doRevoke}
        disabled={busy}
        style={{ padding: "4px 10px", fontSize: 12 }}
      >
        {busy ? "Revoking…" : "Revoke"}
      </button>
    </div>
  );
}
