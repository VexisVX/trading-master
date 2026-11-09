import { useState } from "react";
import { RiskPreview, fmtMoney, parseDec, stateLabel, stateIcon, stateTone } from "../../lib/equityApi";

interface Props {
  preview: RiskPreview | null;
  loading: boolean;
}

const CONSTRAINT_LABELS: Record<string, string> = {
  Target: "Target (R_target)",
  Buffer: "Buffer (R_buffer)",
  Portfolio: "Portfolio cap",
  Daily: "Daily loss cap",
  Prop: "Prop firm cap",
};

export function RiskIntelligenceCard({ preview, loading }: Props) {
  const [helpOpen, setHelpOpen] = useState(false);

  if (loading) {
    return (
      <section className="ee-card ee-risk-intel">
        <div className="ee-card__header">
          <div>
            <h3 className="ee-card__title">Risk Intelligence</h3>
            <p className="ee-card__subtitle">Computing…</p>
          </div>
        </div>
      </section>
    );
  }

  if (!preview) {
    return (
      <section className="ee-card ee-risk-intel">
        <div className="ee-card__header">
          <div>
            <h3 className="ee-card__title">Risk Intelligence</h3>
            <p className="ee-card__subtitle">No snapshot — click Refresh</p>
          </div>
        </div>
      </section>
    );
  }

  const binding = preview.binding_constraint || "Target";
  const bindingLabel = CONSTRAINT_LABELS[binding] || binding;
  const fProfit = parseDec(preview.f_profit);
  const fProfitActive = fProfit !== null && fProfit > 0;
  const bufferR = parseDec(preview.buffer_r) ?? 0;
  const mPreserve = parseDec(preview.m_preserve) ?? 0;
  const stateEnum = preview.state_enum || preview.state;
  const stateT = stateTone(stateEnum);

  return (
    <section className="ee-card ee-risk-intel">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Risk Intelligence</h3>
          <p className="ee-card__subtitle">State · buffer · floors</p>
        </div>
        <div className={`ee-state-badge ee-state-badge--${stateT}`}>
          <span className="ee-state-badge__icon">{stateIcon(stateEnum)}</span>
          <span className="ee-state-badge__label">{stateLabel(stateEnum)}</span>
        </div>
      </div>

      {/* Hero */}
      <div className="ee-risk-hero">
        <div className="ee-risk-hero__label">MAX RISK RIGHT NOW</div>
        <div className="ee-risk-hero__value">{fmtMoney(preview.r_allowed)}</div>
        <div className="ee-risk-hero__caption">
          of {fmtMoney(preview.equity)} equity · <strong>limited by {bindingLabel}</strong>
        </div>
      </div>

      {/* Simple summary rows — no formulas */}
      <div className="ee-risk-simple">
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">Base risk (R₀)</span>
          <span className="ee-risk-simple__value">{fmtMoney(preview.r_base)}</span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">Buffer (R)</span>
          <span className="ee-risk-simple__value">{fmtMoney(preview.r_buffer)}</span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">Buffer / R_base</span>
          <span className="ee-risk-simple__value">{bufferR.toFixed(2)}</span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">Preservation multiplier</span>
          <span className="ee-risk-simple__value">{mPreserve.toFixed(2)}×</span>
        </div>
      </div>

      {/* Floors — compact */}
      <div className="ee-risk-simple">
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">F_profit</span>
          <span className="ee-risk-simple__value">
            {fProfitActive ? fmtMoney(preview.f_profit) : "— (not active)"}
          </span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">F_trail</span>
          <span className="ee-risk-simple__value">{fmtMoney(preview.f_trail)}</span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">F_internal</span>
          <span className="ee-risk-simple__value">{fmtMoney(preview.f_internal)}</span>
        </div>
        <div className="ee-risk-simple__row">
          <span className="ee-risk-simple__label">F_prop (hard rule)</span>
          <span className="ee-risk-simple__value">{fmtMoney(preview.f_prop)}</span>
        </div>
      </div>

      {/* Help button */}
      <div style={{ display: "flex", justifyContent: "flex-end", marginTop: 12 }}>
        <button
          className="ee-risk-help-btn"
          onClick={() => setHelpOpen(true)}
          title="How is this calculated?"
        >
          ?
        </button>
      </div>

      {helpOpen && (
        <RiskHelpModal preview={preview} onClose={() => setHelpOpen(false)} />
      )}
    </section>
  );
}

function RiskHelpModal({ preview, onClose }: { preview: RiskPreview; onClose: () => void }) {
  const r0Pct = (parseDec(preview.r0_pct) ?? 0) * 100;
  const bufferR = parseDec(preview.buffer_r) ?? 0;
  const mPreserve = parseDec(preview.m_preserve) ?? 0;
  const binding = preview.binding_constraint || "Target";
  const bindingLabel = CONSTRAINT_LABELS[binding] || binding;

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div className="ee-modal ee-modal--wide" onClick={(e) => e.stopPropagation()}>
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">How risk is calculated</h3>
            <p className="ee-modal__subtitle">Step-by-step: from equity to final R_allowed</p>
          </div>
          <button className="ee-modal__close" onClick={onClose}>✕</button>
        </div>
        <div className="ee-modal__body">
          <div className="ee-help-section">
            <h4>1. Base risk (R_base)</h4>
            <p>
              You risk a fixed fraction of your equity per trade. With R₀ = {r0Pct.toFixed(2)}% on{" "}
              <strong>{fmtMoney(preview.equity)}</strong>, the base risk is:
            </p>
            <p>
              <code>R_base = Equity × R₀ = {fmtMoney(preview.equity)} × {r0Pct.toFixed(2)}% = {fmtMoney(preview.r_base)}</code>
            </p>
          </div>

          <div className="ee-help-section">
            <h4>2. Preservation buffer</h4>
            <p>
              Before risking, we reserve the floors that protect accumulated profit and trailing downside:
            </p>
            <p>
              <code>B = Equity − F_internal − R_open = {fmtMoney(preview.buffer)}</code>
            </p>
            <p>
              Then we normalize by R_base to get how many "base risks" of room we have:
            </p>
            <p>
              <code>Buffer_R = B / R_base = {bufferR.toFixed(2)}</code>
            </p>
          </div>

          <div className="ee-help-section">
            <h4>3. Preservation multiplier (M_preserve)</h4>
            <p>
              The further you are from the floor, the more you can risk. M_preserve scales from 0 (paused)
              to 1.0 (full risk). At Buffer_R = {bufferR.toFixed(2)}:
            </p>
            <p>
              <code>M_preserve = {mPreserve.toFixed(2)}×</code>
            </p>
            <p style={{ fontSize: 12, color: "var(--text-1)" }}>
              Piecewise: Buffer_R ≥ 8 → 1.0 · 2 &lt; Buffer_R &lt; 8 → linear 0.25–1.0 · ≤ 2 → pause
            </p>
          </div>

          <div className="ee-help-section">
            <h4>4. Target risk</h4>
            <p>
              <code>R_target = R_base × M_preserve = {fmtMoney(preview.r_base)} × {mPreserve.toFixed(2)} = {fmtMoney(preview.r_target)}</code>
            </p>
          </div>

          <div className="ee-help-section">
            <h4>5. Caps</h4>
            <p>Final risk is limited by the smallest of these constraints:</p>
            <p style={{ fontFamily: "var(--mono)", fontSize: 12, lineHeight: 1.8 }}>
              R_buffer = {fmtMoney(preview.r_buffer)}<br />
              R_portfolio = {fmtMoney(preview.r_portfolio_cap)}<br />
              R_daily = {fmtMoney(preview.r_daily_cap)}<br />
              R_prop = {fmtMoney(preview.r_prop_cap)}
            </p>
            <p>
              <strong>R_allowed = {fmtMoney(preview.r_allowed)}</strong> — binding: <strong>{bindingLabel}</strong>
            </p>
          </div>

          <div className="ee-help-section">
            <h4>6. Profit floors (what protects you)</h4>
            <p>
              <strong>F_profit</strong> = protected profit (activates above {((parseDec(preview.profit_activation) ?? 0) * 100).toFixed(1)}% NAV growth).
              Currently: <strong>{parseDec(preview.f_profit) ? fmtMoney(preview.f_profit) : "not active"}</strong>
            </p>
            <p>
              <strong>F_trail</strong> = trailing floor from NAV HWM at {((parseDec(preview.trailing_dd) ?? 0) * 100).toFixed(1)}% below peak:{" "}
              <strong>{fmtMoney(preview.f_trail)}</strong>
            </p>
            <p>
              <strong>F_internal</strong> = max(F_profit, F_trail) — the internal protection floor:{" "}
              <strong>{fmtMoney(preview.f_internal)}</strong>
            </p>
            <p>
              <strong>F_prop</strong> = actual prop-firm breach floor (hard rule):{" "}
              <strong>{fmtMoney(preview.f_prop)}</strong>
            </p>
          </div>
        </div>
        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--primary" onClick={onClose}>Got it</button>
        </div>
      </div>
    </div>
  );
}
