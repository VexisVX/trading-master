import {
  AccountInfo,
  AccountStatus,
  fmtMoney,
  fmtPct,
  timeAgo,
  stateLabel,
  stateIcon,
  stateTone,
} from "../../lib/equityApi";

interface Props {
  status: AccountStatus;
  accounts: AccountInfo[];
  selectedAccountId: number | null;
  onSelectAccount: (id: number) => void;
  refreshing: boolean;
  onRefresh: () => void;
  onOpenSnapshots: () => void;
}

export function AccountStatusCard({
  status,
  accounts,
  selectedAccountId,
  onSelectAccount,
  refreshing,
  onRefresh,
  onOpenSnapshots,
}: Props) {
  const noSnapshot = status.snapshot === null;
  const ageWarn = status.snapshotAgeHours > 24;
  const ageStale = status.snapshotAgeHours > 48;

  const selected = accounts.find((a) => a.id === selectedAccountId);
  const initialFromSnapshot = status.snapshot?.initial_balance;
  const currency = selected?.currency ?? "USD";

  return (
    <section className="ee-card ee-status">
      <div className="ee-card__header">
        <div style={{ flex: 1 }}>
          <h3 className="ee-card__title">Account Status</h3>
          <p className="ee-card__subtitle">
            {selected
              ? `${selected.name} · ${currency} · Initial: ${fmtMoney(
                  initialFromSnapshot ?? selected.initial_balance
                )}`
              : "Derived automatically from your Journal trades"}
          </p>
        </div>

        <div className="ee-card__actions">
          {accounts.length > 1 && (
            <select
              className="ee-select"
              value={selectedAccountId ?? ""}
              onChange={(e) => onSelectAccount(Number(e.target.value))}
              title="Select account"
            >
              {accounts.map((a) => (
                <option key={a.id} value={a.id}>
                  {a.name} ({a.currency})
                </option>
              ))}
            </select>
          )}

          <span
            className={`ee-status__badge ee-status__badge--${
              status.tone || "muted"
            }`}
          >
            {status.label}
          </span>

          <button
            className="ee-btn ee-btn--ghost"
            onClick={onRefresh}
            disabled={refreshing}
            title="Recompute from trades"
          >
            {refreshing ? "⟳ Refreshing…" : "⟳ Refresh"}
          </button>

          <button
            className="ee-btn ee-btn--ghost"
            onClick={onOpenSnapshots}
            title="Manage snapshots"
          >
            🗑 Snapshots
          </button>
        </div>
      </div>

      {noSnapshot ? (
        <div className="ee-empty-inline">
          No snapshot yet. Click <strong>⟳ Refresh</strong> to compute from your
          trades.
        </div>
      ) : (
        <>
          <div className="ee-status__grid">
            <div className="ee-status__item">
              <div className="ee-status__label">Equity</div>
              <div className="ee-status__value">
                {fmtMoney(status.equity)}
              </div>
              <div className="ee-status__sub">
                Initial: {fmtMoney(initialFromSnapshot)}
              </div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Peak</div>
              <div className="ee-status__value">{fmtMoney(status.hwm)}</div>
              <div className="ee-status__sub">High-watermark</div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Drawdown</div>
              <div
                className={`ee-status__value ee-status__value--${
                  status.tone || "muted"
                }`}
              >
                {fmtPct(status.ddPct)}
              </div>
              <div className="ee-status__sub">
                {status.ddPct > 0
                  ? `${fmtMoney(status.hwm - status.equity)} below peak`
                  : "At peak"}
              </div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Net Profit</div>
              <div
                className={`ee-status__value ${
                  status.netProfit > 0
                    ? "pos"
                    : status.netProfit < 0
                    ? "neg"
                    : ""
                }`}
              >
                {status.netProfit >= 0 ? "+" : ""}
                {fmtMoney(status.netProfit)}
              </div>
              <div className="ee-status__sub">Since inception</div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Streak</div>
              <div
                className={`ee-status__value ${
                  status.streak >= 3 ? "neg" : status.streak >= 2 ? "warn" : ""
                }`}
              >
                {status.streak}{" "}
                {status.streak === 1 ? "loss" : "losses"}
              </div>
              <div className="ee-status__sub">Consecutive</div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Daily Loss</div>
              <div className="ee-status__value">
                {fmtMoney(status.dailyLoss)}
              </div>
              <div className="ee-status__sub">Today</div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Open Risk</div>
              <div className="ee-status__value">
                {fmtMoney(status.openRisk)}
              </div>
              <div className="ee-status__sub">Across open trades</div>
            </div>

            <div className="ee-status__item">
              <div className="ee-status__label">Snapshot</div>
              <div
                className={`ee-status__value ${
                  ageStale ? "neg" : ageWarn ? "warn" : ""
                }`}
              >
                {timeAgo(status.snapshot?.taken_at ?? "")}
              </div>
              <div className="ee-status__sub">
                {status.snapshot?.taken_at
                  ? new Date(status.snapshot.taken_at).toLocaleString()
                  : "—"}
              </div>
            </div>
          </div>

          {ageWarn && (
            <div className="ee-status__warning">
              ⚠ Snapshot is {Math.floor(status.snapshotAgeHours)}h old — click{" "}
              <strong>⟳ Refresh</strong> to update from current trades.
            </div>
          )}
        </>
      )}
    
      <div className={`ee-state-badge ee-state-badge--${stateTone((status.snapshot as any)?.state_enum || status.label.toLowerCase())}`}>
        <span className="ee-state-badge__icon">{stateIcon((status.snapshot as any)?.state_enum || "")}</span>
        <span className="ee-state-badge__label">{stateLabel((status.snapshot as any)?.state_enum || "")}</span>
      </div>
</section>
  );
}
