import { RiskPreview, fmtMoney, fmtPct, parseDec } from "../../lib/equityApi";

interface Props {
  preview: RiskPreview | null;
  loading: boolean;
}

const CONSTRAINT_LABELS: Record<string, string> = {
  BaseRate: "Base rate (R₀)",
  Portfolio: "Portfolio risk cap",
  Daily: "Daily loss cap",
  Floor: "Floor risk capacity",
  Prop: "Prop firm cap",
};

export function RiskIntelligenceCard({ preview, loading }: Props) {
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
            <p className="ee-card__subtitle">
              No snapshot yet — click ⟳ Refresh on Account Status
            </p>
          </div>
        </div>
      </section>
    );
  }

  const binding = preview.binding_constraint;
  const rAllowed = parseDec(preview.r_allowed) ?? 0;
  const equity = parseDec(preview.equity) ?? 0;
  const r0Pct = (parseDec(preview.r0_pct) ?? 0) * 100;
  const pctOfEquity = equity > 0 ? (rAllowed / equity) * 100 : 0;

  return (
    <section className="ee-card ee-risk-intel">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Risk Intelligence</h3>
          <p className="ee-card__subtitle">
            How the engine computes your allowed risk right now
          </p>
        </div>
        <span
          className={`ee-status__badge ee-status__badge--${statusTone(
            preview.phase
          )}`}
        >
          {preview.phase}
        </span>
      </div>

      {/* Hero: MAX RISK RIGHT NOW */}
      <div className="ee-risk-hero">
        <div className="ee-risk-hero__label">MAX RISK RIGHT NOW</div>
        <div className="ee-risk-hero__value">{fmtMoney(rAllowed)}</div>
        <div className="ee-risk-hero__meta">
          {pctOfEquity.toFixed(2)}% of equity · limited by{" "}
          <strong>{CONSTRAINT_LABELS[binding] ?? binding}</strong>
        </div>
      </div>

      {/* How we got here */}
      <div className="ee-risk-trace">
        <div className="ee-risk-trace__section-title">HOW WE GOT HERE</div>

        <Step
          n={1}
          title="Base risk"
          body={`Equity ${fmtMoney(preview.equity)} × R₀ ${r0Pct.toFixed(
            2
          )}% = ${fmtMoney(preview.r_base)}`}
        />
        <Step
          n={2}
          title="Drawdown multiplier"
          body={`Current DD ${fmtPct(preview.dd_pct)} → × ${preview.m_dd}`}
        />
        <Step
          n={3}
          title="Loss-streak multiplier"
          body={`Streak ${preview.consecutive_losses} → × ${preview.m_loss}`}
        />
        <Step
          n={4}
          title="Caps (min of all)"
          body=""
          highlight
          extra={
            <div className="ee-risk-caps">
              <Cap
                label="Base rate"
                value={preview.r_adaptive}
                binding={binding === "BaseRate"}
              />
              <Cap
                label="Portfolio cap"
                value={preview.r_portfolio_cap}
                binding={binding === "Portfolio"}
              />
              <Cap
                label="Daily loss cap"
                value={preview.r_daily_cap}
                binding={binding === "Daily"}
              />
              <Cap
                label="Floor risk cap"
                value={preview.r_floor_cap}
                binding={binding === "Floor"}
              />
              <Cap
                label="Prop firm cap"
                value={preview.r_prop_cap}
                binding={binding === "Prop"}
              />
            </div>
          }
        />
      </div>

      {/* What-if */}
      <div className="ee-risk-whatif">
        <div className="ee-risk-trace__section-title">WHAT IF</div>
        <div className="ee-risk-whatif__row">
          <span>If streak reaches {preview.n_max}:</span>
          <strong>⛔ paused</strong>
        </div>
        <div className="ee-risk-whatif__row">
          <span>If drawdown reaches {fmtPct(preview.dd_1_pct)}:</span>
          <strong>
            × {preview.m_dd_1} →{" "}
            {fmtMoney(
              (parseDec(preview.r_base) ?? 0) *
                (parseDec(preview.m_dd_1) ?? 1) *
                (parseDec(preview.m_loss) ?? 1)
            )}
          </strong>
        </div>
        <div className="ee-risk-whatif__row">
          <span>If drawdown reaches {fmtPct(preview.dd_2_pct)}:</span>
          <strong>
            × {preview.m_dd_2} →{" "}
            {fmtMoney(
              (parseDec(preview.r_base) ?? 0) *
                (parseDec(preview.m_dd_2) ?? 1) *
                (parseDec(preview.m_loss) ?? 1)
            )}
          </strong>
        </div>
        <div className="ee-risk-whatif__row">
          <span>If drawdown reaches {fmtPct(preview.dd_3_pct)}:</span>
          <strong>
            × {preview.m_dd_3} →{" "}
            {fmtMoney(
              (parseDec(preview.r_base) ?? 0) *
                (parseDec(preview.m_dd_3) ?? 1) *
                (parseDec(preview.m_loss) ?? 1)
            )}
          </strong>
        </div>
      </div>

      {/* Account floors summary */}
      <div className="ee-risk-floors">
        <div className="ee-risk-floors__item">
          <div className="ee-risk-floors__label">🛡 Protected</div>
          <div className="ee-risk-floors__value">
            {fmtMoney(preview.profit_retention)}
          </div>
        </div>
        <div className="ee-risk-floors__item">
          <div className="ee-risk-floors__label">🏔 Equity Floor</div>
          <div className="ee-risk-floors__value">{fmtMoney(preview.f_hybrid)}</div>
        </div>
        <div className="ee-risk-floors__item">
          <div className="ee-risk-floors__label">📊 Loss Capacity</div>
          <div className="ee-risk-floors__value">
            {fmtMoney(preview.loss_capacity)}
          </div>
        </div>
      </div>
    </section>
  );
}

function Step({
  n,
  title,
  body,
  extra,
  highlight,
}: {
  n: number;
  title: string;
  body: string;
  extra?: React.ReactNode;
  highlight?: boolean;
}) {
  return (
    <div className={`ee-risk-step ${highlight ? "is-highlight" : ""}`}>
      <div className="ee-risk-step__n">{n}</div>
      <div className="ee-risk-step__body">
        <div className="ee-risk-step__title">{title}</div>
        {body && <div className="ee-risk-step__text">{body}</div>}
        {extra}
      </div>
    </div>
  );
}

function Cap({
  label,
  value,
  binding,
}: {
  label: string;
  value: string;
  binding: boolean;
}) {
  return (
    <div className={`ee-risk-cap ${binding ? "is-binding" : ""}`}>
      <span className="ee-risk-cap__dot" />
      <span className="ee-risk-cap__label">{label}</span>
      <span className="ee-risk-cap__value mono">{fmtMoney(value)}</span>
      {binding && <span className="ee-risk-cap__badge">BINDING</span>}
    </div>
  );
}

function statusTone(phase: string): "pos" | "warn" | "neg" | "muted" {
  switch (phase) {
    case "normal":
      return "pos";
    case "caution":
      return "warn";
    case "recovery":
      return "warn";
    case "paused":
      return "neg";
    case "disabled":
      return "neg";
    default:
      return "muted";
  }
}
