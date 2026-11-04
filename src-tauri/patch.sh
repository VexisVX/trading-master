#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

DB="$HOME/.local/share/com.trading.manager/app.db"
BD="$HOME/.local/share/com.trading.manager/backups"
mkdir -p "$BD"
STAMP="$(date +%Y%m%d_%H%M%S)"
BK="$BD/pre_ui_cleanup_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

# ============================================================
# 1) WhyThisSizeModal — separate sections visually
# ============================================================
echo "==> [1/4] WhyThisSizeModal — visual separation"
python3 - <<'PY'
import pathlib, re
p = pathlib.Path("src/pages/EquityEngine/EquityEngine.css")
s = p.read_text()

# Remove old trace CSS
old_trace = re.compile(
    r'/\* =+\s*\n\s*\*\s*WhyThisSizeModal — trace sections \(v2\).*?/\* =+ \*/',
    re.DOTALL
)
s = old_trace.sub("", s)

# Also remove the trace CSS added at the end of run_fix_fk_ui
old_trace2 = re.compile(
    r'/\* =+\s*\n\s*\*\s*WhyThisSizeModal — improve trace layout.*?(?=/\* =+\s*\n\s*\*\s*BUTTONS)',
    re.DOTALL
)
s = old_trace2.sub("", s)

# Append fresh trace CSS with strong visual separation
s += '''

/* ============================================================
 * WhyThisSizeModal — trace sections (clean, spacious)
 * ============================================================ */

.ee-modal--wide .ee-modal__body {
  max-height: 72vh;
  overflow-y: auto;
  padding: 22px 26px;
}

.ee-trace-section {
  margin-bottom: 18px;
  padding: 16px 18px;
  background: var(--bg-1, #11161d);
  border: 1px solid var(--border, #232c38);
  border-radius: 10px;
}

.ee-trace-section:last-child {
  margin-bottom: 0;
}

.ee-trace-section--final {
  background: var(--accent-soft, rgba(45, 212, 191, 0.08));
  border: 1px solid var(--accent, #2dd4bf);
}

.ee-trace-section__title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--accent, #2dd4bf);
  font-weight: 700;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border, #232c38);
}

.ee-trace-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  padding: 7px 0;
  font-size: 13px;
  border-bottom: 1px dashed rgba(154, 167, 180, 0.12);
}

.ee-trace-row:last-child {
  border-bottom: none;
  padding-bottom: 0;
}

.ee-trace-row:first-child {
  padding-top: 0;
}

.ee-trace__label {
  color: var(--text-1, #9aa7b4);
  flex: 1;
}

.ee-trace__value {
  font-family: var(--mono, ui-monospace, monospace);
  color: var(--text-0, #e6edf3);
  text-align: right;
  font-size: 13px;
}

.ee-trace__value.is-bold {
  font-weight: 700;
  color: var(--accent, #2dd4bf);
}

.ee-trace__value.is-highlight {
  color: var(--accent, #2dd4bf);
  font-size: 14px;
}

.ee-trace__result {
  margin-top: 20px;
  padding: 14px 18px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  text-align: center;
}

.ee-trace__result.is-allowed {
  background: rgba(52, 211, 153, 0.1);
  color: var(--success, #34d399);
  border: 1px solid var(--success, #34d399);
}

.ee-trace__result.is-rejected {
  background: rgba(248, 113, 113, 0.1);
  color: var(--danger, #f87171);
  border: 1px solid var(--danger, #f87171);
}

/* Light theme */
[data-theme="light"] .ee-trace-section {
  background: #f8fafc;
  border-color: #e2e8f0;
}
[data-theme="light"] .ee-trace-section__title {
  border-bottom-color: #e2e8f0;
}
[data-theme="light"] .ee-trace-row {
  border-bottom-color: rgba(100, 116, 139, 0.15);
}
[data-theme="light"] .ee-trace__label {
  color: #475569;
}
[data-theme="light"] .ee-trace__value {
  color: #0f172a;
}

/* Cream theme */
[data-theme="cream"] .ee-trace-section {
  background: #efe7d8;
  border-color: #e0d6c2;
}
[data-theme="cream"] .ee-trace-section__title {
  border-bottom-color: #e0d6c2;
}
[data-theme="cream"] .ee-trace-row {
  border-bottom-color: rgba(107, 93, 71, 0.15);
}
[data-theme="cream"] .ee-trace__label {
  color: #4a3f2e;
}
[data-theme="cream"] .ee-trace__value {
  color: #2e2418;
}

/* ============================================================
 * Risk Intelligence — help button
 * ============================================================ */

.ee-risk-help-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 50%;
  background: var(--bg-2, #161d26);
  color: var(--text-1, #9aa7b4);
  border: 1px solid var(--border, #232c38);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.12s ease;
  margin-left: auto;
}

.ee-risk-help-btn:hover {
  background: var(--accent-soft, rgba(45, 212, 191, 0.12));
  color: var(--accent, #2dd4bf);
  border-color: var(--accent, #2dd4bf);
  transform: scale(1.05);
}

[data-theme="light"] .ee-risk-help-btn {
  background: #f1f5f9;
  color: #475569;
  border-color: #cbd5e1;
}
[data-theme="light"] .ee-risk-help-btn:hover {
  background: #cffafe;
  color: #0891b2;
  border-color: #0891b2;
}

[data-theme="cream"] .ee-risk-help-btn {
  background: #efe7d8;
  color: #4a3f2e;
  border-color: #c8bca1;
}
[data-theme="cream"] .ee-risk-help-btn:hover {
  background: #d4ecec;
  color: #0e7490;
  border-color: #0e7490;
}

/* Help modal body */
.ee-help-section {
  margin-bottom: 18px;
  padding-bottom: 14px;
  border-bottom: 1px solid var(--border, #232c38);
}
.ee-help-section:last-child {
  border-bottom: none;
}
.ee-help-section h4 {
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--accent, #2dd4bf);
  font-weight: 700;
  margin: 0 0 8px;
}
.ee-help-section p {
  font-size: 13px;
  line-height: 1.65;
  color: var(--text-0, #e6edf3);
  margin: 0 0 8px;
}
.ee-help-section code {
  background: var(--bg-2, #161d26);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--mono, ui-monospace, monospace);
  font-size: 12px;
  color: var(--accent, #2dd4bf);
}

[data-theme="light"] .ee-help-section {
  border-bottom-color: #e2e8f0;
}
[data-theme="light"] .ee-help-section p {
  color: #0f172a;
}
[data-theme="light"] .ee-help-section code {
  background: #f1f5f9;
  color: #0891b2;
}

[data-theme="cream"] .ee-help-section {
  border-bottom-color: #e0d6c2;
}
[data-theme="cream"] .ee-help-section p {
  color: #2e2418;
}
[data-theme="cream"] .ee-help-section code {
  background: #efe7d8;
  color: #0e7490;
}
'''

p.write_text(s)
print("  ✓ CSS updated")
PY

# ============================================================
# 2) RiskIntelligenceCard — cleaner + help modal
# ============================================================
echo "==> [2/4] RiskIntelligenceCard — simplify + help modal"
cat > src/pages/EquityEngine/RiskIntelligenceCard.tsx <<'TSX_EOF'
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
  const r0Pct = (parseDec(preview.r0_pct) ?? 0) * 100;

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
TSX_EOF
echo "  ✓ RiskIntelligenceCard simplified + help modal"

# ============================================================
# 3) CSS for simple rows
# ============================================================
echo "==> [3/4] CSS for simple risk rows"
cat >> src/pages/EquityEngine/EquityEngine.css <<'CSS_EOF'

/* ============================================================
 * Risk Intelligence — simple rows (no formulas)
 * ============================================================ */

.ee-risk-simple {
  margin-top: 16px;
  padding: 12px 14px;
  background: var(--bg-1, #11161d);
  border: 1px solid var(--border, #232c38);
  border-radius: 8px;
}

.ee-risk-simple + .ee-risk-simple {
  margin-top: 10px;
}

.ee-risk-simple__row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  padding: 6px 0;
  font-size: 12px;
  border-bottom: 1px dashed rgba(154, 167, 180, 0.12);
}

.ee-risk-simple__row:last-child {
  border-bottom: none;
}

.ee-risk-simple__label {
  color: var(--text-1, #9aa7b4);
}

.ee-risk-simple__value {
  font-family: var(--mono, ui-monospace, monospace);
  color: var(--text-0, #e6edf3);
  font-weight: 500;
}

[data-theme="light"] .ee-risk-simple {
  background: #f8fafc;
  border-color: #e2e8f0;
}
[data-theme="light"] .ee-risk-simple__row {
  border-bottom-color: rgba(100, 116, 139, 0.15);
}
[data-theme="light"] .ee-risk-simple__label {
  color: #475569;
}
[data-theme="light"] .ee-risk-simple__value {
  color: #0f172a;
}

[data-theme="cream"] .ee-risk-simple {
  background: #efe7d8;
  border-color: #e0d6c2;
}
[data-theme="cream"] .ee-risk-simple__row {
  border-bottom-color: rgba(107, 93, 71, 0.15);
}
[data-theme="cream"] .ee-risk-simple__label {
  color: #4a3f2e;
}
[data-theme="cream"] .ee-risk-simple__value {
  color: #2e2418;
}
CSS_EOF
echo "  ✓ simple rows CSS added"

# ============================================================
# 4) Build
# ============================================================
echo "==> [4/4] Build"
( cd src-tauri && cargo check 2>&1 | grep -E "^error" | head -5 )
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL: cargo check"
    exit 4
fi

npm run build 2>&1 | tail -10
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL: npm build"
    exit 4
fi

echo ""
echo "============================================================"
echo "UI CLEANUP DONE"
echo "  ✓ WhyThisSizeModal: sections separated visually
  ✓ RiskIntelligenceCard: simplified + help popup
  Backup: $BK"
echo "============================================================"
