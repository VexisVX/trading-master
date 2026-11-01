import { DecisionOut, fmtMoney, fmtPct, parseDec } from "../../lib/equityApi";

interface Props {
  result: DecisionOut | null;
  onClose: () => void;
}

export function WhyThisSizeModal({ result, onClose }: Props) {
  if (!result) return null;

  const d = result.risk_detail;

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div
        className="ee-modal ee-modal--wide"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Why this size?</h3>
            <p className="ee-modal__subtitle">
              Full decision trace — every number behind the result
            </p>
          </div>
          <button
            className="ee-modal__close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        {!d ? (
          <div className="ee-modal__body">
            <div className="ee-alert ee-alert--warn">
              Risk details are not available for this decision. This usually
              means the backend didn't return <code>risk_detail</code>.
            </div>
          </div>
        ) : (
          <div className="ee-modal__body">
            <div className="ee-trace">
              <TraceSection title="1 · Base Risk">
                <TraceRow label="Equity" value={fmtMoney(d.equity)} />
                <TraceRow
                  label="× Base rate (R₀)"
                  value={`${fmtPct(safeRatio(d.r_base, d.equity))} → ${fmtMoney(
                    d.r_base
                  )}`}
                />
                <TraceRow
                  label="× Drawdown factor"
                  value={`× ${d.m_dd}`}
                  highlight={parseDec(d.m_dd) !== 1}
                />
                <TraceRow
                  label="× Loss-streak factor"
                  value={`× ${d.m_loss}`}
                  highlight={parseDec(d.m_loss) !== 1}
                />
                <TraceRow
                  label="= Adaptive risk"
                  value={fmtMoney(d.r_adaptive)}
                  bold
                />
              </TraceSection>

              <TraceSection title="2 · Risk Caps">
                <TraceRow
                  label="Portfolio cap"
                  value={fmtMoney(d.r_portfolio_cap)}
                />
                <TraceRow
                  label="Daily-loss cap"
                  value={fmtMoney(d.r_daily_cap)}
                />
                <TraceRow
                  label="Floor-risk cap"
                  value={fmtMoney(d.r_floor_cap)}
                />
                <TraceRow
                  label="Prop-firm cap"
                  value={fmtMoney(d.r_prop_cap)}
                />
                <TraceRow
                  label="= Allowed risk (min of all)"
                  value={fmtMoney(d.r_allowed)}
                  bold
                  highlight
                />
              </TraceSection>

              <TraceSection title="3 · Position Sizing">
                <TraceRow label="|Entry − Stop|" value={d.entry_minus_stop} />
                <TraceRow
                  label="× Contract multiplier"
                  value={`× ${d.contract_multiplier}`}
                />
                <TraceRow
                  label="Per-unit cost"
                  value={fmtMoney(d.per_unit_cost)}
                  bold
                />
                <TraceRow
                  label="Size = Allowed risk / per-unit"
                  value={
                    result.size !== null
                      ? `${parseFloat(result.size).toFixed(4)} lots`
                      : "—"
                  }
                  bold
                  highlight
                />
              </TraceSection>

              <div
                className={`ee-trace__result ${
                  result.allowed ? "is-allowed" : "is-rejected"
                }`}
              >
                <div className="ee-trace__result-badge">
                  {result.allowed ? "ALLOWED" : "REJECTED"}
                </div>
                <div className="ee-trace__result-body">
                  <div>
                    <strong>Allowed risk:</strong>{" "}
                    {fmtMoney(result.r_allowed)}
                  </div>
                  {result.size !== null && (
                    <div>
                      <strong>Size:</strong>{" "}
                      {parseFloat(result.size).toFixed(2)} lots
                    </div>
                  )}
                  {result.rr !== null && (
                    <div>
                      <strong>R : R:</strong> 1 :{" "}
                      {parseFloat(result.rr).toFixed(2)}
                    </div>
                  )}
                  <div>
                    <strong>Phase:</strong> {result.phase} ·{" "}
                    <strong>DD:</strong> {fmtPct(result.dd_pct)}
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}

function TraceSection({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="ee-trace__section">
      <div className="ee-trace__section-title">{title}</div>
      <div className="ee-trace__rows">{children}</div>
    </div>
  );
}

function TraceRow({
  label,
  value,
  bold,
  highlight,
}: {
  label: string;
  value: string;
  bold?: boolean;
  highlight?: boolean;
}) {
  return (
    <div
      className={`ee-trace__row ${bold ? "is-bold" : ""} ${
        highlight ? "is-highlight" : ""
      }`}
    >
      <span className="ee-trace__label">{label}</span>
      <span className="ee-trace__value mono">{value}</span>
    </div>
  );
}

function safeRatio(a: string, b: string): number {
  const na = parseDec(a) ?? 0;
  const nb = parseDec(b) ?? 0;
  return nb > 0 ? na / nb : 0;
}
