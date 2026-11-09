import { stateLabel, stateIcon, stateTone } from "../../lib/equityApi";

interface Props {
  stateEnum: string;
  phase: string;
  bufferR: string;
}

const ALL_STATES = [
  "normal_growth",
  "protected",
  "preservation_pause",
  "recovery_mode",
  "daily_lock",
  "prop_hard_lock",
] as const;

export function StateMachineCard({ stateEnum, phase, bufferR }: Props) {
  const bufferRNum = parseFloat(bufferR) || 0;

  return (
    <section className="ee-card ee-state-machine">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">State Machine</h3>
          <p className="ee-card__subtitle">
            6 states · driven by buffer_r, prop breach, daily lock
          </p>
        </div>
        <div className={`ee-state-badge ee-state-badge--${stateTone(stateEnum)}`}>
          <span className="ee-state-badge__icon">{stateIcon(stateEnum)}</span>
          <span className="ee-state-badge__label">{stateLabel(stateEnum)}</span>
        </div>
      </div>

      <div className="ee-sm-timeline">
        {ALL_STATES.map((s) => {
          const isActive = s === stateEnum;
          const isPast = isStatePast(stateEnum, s);
          return (
            <div
              key={s}
              className={`ee-sm-step ${isActive ? "is-active" : ""} ${isPast ? "is-past" : ""}`}
            >
              <div className={`ee-sm-step__dot ee-sm-step__dot--${stateTone(s)}`}>
                {isActive ? "●" : isPast ? "✓" : "○"}
              </div>
              <div className="ee-sm-step__label">{stateLabel(s)}</div>
            </div>
          );
        })}
      </div>

      <div className="ee-sm-detail">
        <div className="ee-sm-detail__row">
          <span>Current state</span>
          <strong>{stateLabel(stateEnum)}</strong>
        </div>
        <div className="ee-sm-detail__row">
          <span>Phase (legacy)</span>
          <strong>{phase}</strong>
        </div>
        <div className="ee-sm-detail__row">
          <span>Buffer_R</span>
          <strong>{bufferRNum.toFixed(2)}</strong>
        </div>
        <div className="ee-sm-detail__hint">
          <strong>Buffer_R thresholds:</strong>
          {" ≥ 8 → Normal Growth · 2–8 → Protected · ≤ 2 → Preservation Pause"}
        </div>
      </div>
    </section>
  );
}

function isStatePast(current: string, candidate: string): boolean {
  // If current is a blocking state, no "past" states (we're stuck)
  const blocking = ["preservation_pause", "daily_lock", "prop_hard_lock"];
  if (blocking.includes(current)) return false;
  // Otherwise mark transition steps as "past" if they appear before
  const order = [
    "prop_hard_lock",
    "preservation_pause",
    "protected",
    "normal_growth",
  ];
  const ci = order.indexOf(current);
  const vi = order.indexOf(candidate);
  if (ci === -1 || vi === -1) return false;
  return vi < ci;
}
