import { DecisionOut, fmtMoney, parseDec, stateLabel } from "../../lib/equityApi";

interface Props {
  result: DecisionOut | null;
  onClose: () => void;
}

export function WhyThisSizeModal({ result, onClose }: Props) {
  if (!result) return null;

  const d = result.risk_detail;

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div className="ee-modal ee-modal--wide" onClick={(e) => e.stopPropagation()}>
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Why this size?</h3>
            <p className="ee-modal__subtitle">
              {result.allowed ? "Decision trace" : "Rejection trace"} — every number behind the result
            </p>
          </div>
          <button className="ee-modal__close" onClick={onClose} aria-label="Close">✕</button>
        </div>

        <div className="ee-modal__body">
          {!d ? (
            <div className="ee-alert ee-alert--warn">
              Risk details unavailable — backend didn't return <code>risk_detail</code>.
            </div>
          ) : (
            <>
              {/* Section 1: Base */}
              <div className="ee-trace-section">
                <div className="ee-trace-section__title">1 · Base risk</div>
                <TraceRow label="Equity" value={fmtMoney(d.equity)} />
                <TraceRow label="× R₀" value={fmtMoney(d.r_base) + ` (${((parseDec(d.r_base) ?? 0) / (parseDec(d.equity) ?? 1) * 100).toFixed(2)}%)`} />
                <TraceRow label="= Base risk" value={fmtMoney(d.r_base)} bold />
              </div>

              {/* Section 2: Buffer */}
              <div className="ee-trace-section">
                <div className="ee-trace-section__title">2 · Preservation buffer</div>
                <TraceRow label="Equity" value={fmtMoney(d.equity)} />
                <TraceRow label="− F_internal" value={fmtMoney(d.f_internal)} />
                <TraceRow label="− R_open" value="—" />
                <TraceRow label="= Buffer B" value={fmtMoney((parseDec(d.equity) ?? 0) - (parseDec(d.f_internal) ?? 0))} bold />
                <TraceRow label="Buffer_R = B / R_base" value={(parseDec(d.buffer_r) ?? 0).toFixed(2)} />
                <TraceRow label="M_preserve" value={(parseDec(d.m_preserve) ?? 0).toFixed(2)} bold />
              </div>

              {/* Section 3: Target */}
              <div className="ee-trace-section">
                <div className="ee-trace-section__title">3 · Target risk</div>
                <TraceRow label="R_base" value={fmtMoney(d.r_base)} />
                <TraceRow label="× M_preserve" value={(parseDec(d.m_preserve) ?? 0).toFixed(2)} />
                <TraceRow label="= R_target" value={fmtMoney(d.r_target)} bold />
              </div>

              {/* Section 4: Caps */}
              <div className="ee-trace-section">
                <div className="ee-trace-section__title">4 · Caps</div>
                <TraceRow label="R_buffer" value={fmtMoney(d.r_buffer)} />
                <TraceRow label="R_portfolio_cap" value={fmtMoney(d.r_portfolio_cap)} />
                <TraceRow label="R_daily_cap" value={fmtMoney(d.r_daily_cap)} />
                <TraceRow label="R_prop_cap" value={fmtMoney(d.r_prop_cap)} />
              </div>

              {/* Final */}
              <div className="ee-trace-section ee-trace-section--final">
                <div className="ee-trace-section__title">= Allowed risk</div>
                <TraceRow label="Binding constraint" value={d.binding_constraint || "Target"} />
                <TraceRow label="R_allowed" value={fmtMoney(d.r_allowed)} bold highlight />
              </div>

              {/* Sizing */}
              <div className="ee-trace-section">
                <div className="ee-trace-section__title">5 · Position sizing</div>
                <TraceRow label="|Entry − Stop|" value={d.entry_minus_stop} />
                <TraceRow label="× Contract multiplier" value={`× ${d.contract_multiplier}`} />
                <TraceRow label="Per-unit cost" value={fmtMoney(d.per_unit_cost)} />
                <TraceRow
                  label="Size = R_allowed / per-unit"
                  value={
                    result.size !== null
                      ? `${parseFloat(result.size).toFixed(4)} lots`
                      : "—"
                  }
                  bold
                  highlight
                />
                {result.actual_risk && (
                  <TraceRow label="Actual risk (after rounding)" value={fmtMoney(result.actual_risk)} />
                )}
              </div>

              {/* State summary */}
              <div className={`ee-trace__result ${result.allowed ? "is-allowed" : "is-rejected"}`}>
                {result.allowed ? "✓ ALLOWED" : "✗ REJECTED"}
                {" · "}
                State: {stateLabel(result.state_enum || result.state)}
                {" · "}
                Phase: {result.phase}
                {" · "}
                DD: {(parseDec(result.dd_pct) ?? 0) * 100}%
              </div>
            </>
          )}
        </div>

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>Close</button>
        </div>
      </div>
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
    <div className="ee-trace-row">
      <span className="ee-trace__label">{label}</span>
      <span className={`ee-trace__value ${bold ? "is-bold" : ""} ${highlight ? "is-highlight" : ""}`}>
        {value}
      </span>
    </div>
  );
}
