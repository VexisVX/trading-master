import { useEffect, useMemo, useState } from "react";
import {
  EquityConfig,
  RiskPreview,
  defaultConfig,
  getEquityConfig,
  setEquityConfig,
  fmtMoney,
  parseDec,
} from "../../lib/equityApi";

interface Props {
  onClose: () => void;
  preview: RiskPreview | null; // for live dollar examples
}

type Mode = "simple" | "advanced";
type AdvTab = "risk" | "caps" | "recovery" | "advanced";

interface SimpleField {
  key: keyof EquityConfig;
  label: string;
  hint: string;
  min: number;
  max: number;
  step: number;
  kind: "pct" | "int";
}

const SIMPLE_FIELDS: SimpleField[] = [
  {
    key: "r0",
    label: "Risk per trade (R₀)",
    hint: "How much of your equity you risk on a single trade",
    min: 0.001,
    max: 0.05,
    step: 0.001,
    kind: "pct",
  },
  {
    key: "daily_loss_limit",
    label: "Daily loss limit",
    hint: "Stop trading for the day after losing this much",
    min: 0,
    max: 0.2,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "r_portfolio_max",
    label: "Max total open risk",
    hint: "Total risk across all open trades combined",
    min: 0,
    max: 0.3,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "n_max",
    label: "Pause after N losses",
    hint: "Stop trading after this many consecutive losses",
    min: 1,
    max: 20,
    step: 1,
    kind: "int",
  },
];

interface AdvField {
  key: keyof EquityConfig;
  label: string;
  hint?: string;
  kind: "text" | "number";
}

const ADV_TABS: Record<AdvTab, AdvField[]> = {
  risk: [
    { key: "r0", label: "Base risk (R₀)", hint: "0.01 = 1% of equity", kind: "text" },
    { key: "r_min", label: "Minimum risk", kind: "text" },
    { key: "delta_r", label: "Step size (ΔR)", kind: "text" },
    { key: "dd_1", label: "DD tier 1 (caution)", kind: "text" },
    { key: "dd_2", label: "DD tier 2 (warning)", kind: "text" },
    { key: "dd_3", label: "DD tier 3 (severe)", kind: "text" },
    { key: "m_dd_1", label: "Multiplier tier 1", kind: "text" },
    { key: "m_dd_2", label: "Multiplier tier 2", kind: "text" },
    { key: "m_dd_3", label: "Multiplier tier 3", kind: "text" },
    { key: "m_dd_4", label: "Multiplier tier 4+", kind: "text" },
    { key: "m_loss_table", label: "Loss-streak table (comma-separated)", kind: "text" },
    { key: "n_max", label: "N_max (pause threshold)", kind: "number" },
  ],
  caps: [
    { key: "daily_loss_limit", label: "Daily loss limit", hint: "0 = disabled", kind: "text" },
    { key: "r_portfolio_max", label: "Portfolio risk max", hint: "0 = disabled", kind: "text" },
    { key: "r_prop_max", label: "Prop per-trade max", hint: "0 = disabled", kind: "text" },
    { key: "mu_max", label: "Margin usage max", kind: "text" },
  ],
  recovery: [
    { key: "q_retention", label: "Q — profit retention", kind: "text" },
    { key: "d_trailing", label: "d — trailing from HWM", kind: "text" },
    { key: "d_prop", label: "D_prop (prop budget)", kind: "text" },
    { key: "alpha", label: "α — floor risk factor", kind: "text" },
    { key: "recovery_target", label: "Recovery target", kind: "text" },
  ],
  advanced: [
    { key: "max_snapshot_age_hours", label: "Max snapshot age (h)", hint: "reject decisions if older", kind: "number" },
  ],
};

export function SettingsModal({ onClose, preview }: Props) {
  const [mode, setMode] = useState<Mode>("simple");
  const [advTab, setAdvTab] = useState<AdvTab>("risk");
  const [cfg, setCfg] = useState<EquityConfig>(defaultConfig());
  const [original, setOriginal] = useState<EquityConfig>(defaultConfig());
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState(false);

  useEffect(() => {
    getEquityConfig()
      .then((c) => {
        setCfg(c);
        setOriginal(c);
        setLoaded(true);
      })
      .catch((e) => setError(String(e)));
  }, []);

  const hasChanges = useMemo(
    () => JSON.stringify(cfg) !== JSON.stringify(original),
    [cfg, original]
  );

  const validation = useMemo(() => validate(cfg), [cfg]);

  const equity = preview ? parseDec(preview.equity) ?? 0 : 0;

  async function save() {
    if (validation.errors.length > 0) {
      setError("Fix errors before saving.");
      return;
    }
    setError(null);
    setOk(false);
    setBusy(true);
    try {
      await setEquityConfig(cfg);
      setOriginal(cfg);
      setOk(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  function reset() {
    setCfg(defaultConfig());
    setOk(false);
  }

  function discard() {
    setCfg(original);
    setOk(false);
    setError(null);
  }

  if (!loaded) {
    return (
      <div className="ee-modal-backdrop" onClick={onClose}>
        <div className="ee-modal ee-modal--wide" onClick={(e) => e.stopPropagation()}>
          <div className="ee-modal__header">
            <div>
              <h3 className="ee-modal__title">Settings</h3>
            </div>
            <button className="ee-modal__close" onClick={onClose}>✕</button>
          </div>
          <div className="ee-modal__body">
            <div className="ee-empty-inline">Loading…</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div className="ee-modal ee-modal--wide" onClick={(e) => e.stopPropagation()}>
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Equity Engine Settings</h3>
            <p className="ee-modal__subtitle">
              Tune how the engine sizes your trades
            </p>
          </div>
          <button className="ee-modal__close" onClick={onClose} aria-label="Close">
            ✕
          </button>
        </div>

        {/* Mode toggle */}
        <div style={{ padding: "14px 22px 0" }}>
          <div className="ee-mode-toggle">
            <button
              className={`ee-mode-toggle__btn ${mode === "simple" ? "is-active" : ""}`}
              onClick={() => setMode("simple")}
            >
              Simple
            </button>
            <button
              className={`ee-mode-toggle__btn ${mode === "advanced" ? "is-active" : ""}`}
              onClick={() => setMode("advanced")}
            >
              Advanced
            </button>
          </div>
        </div>

        {/* Simple mode */}
        {mode === "simple" && (
          <div className="ee-modal__body">
            <div className="ee-settings-simple">
              {SIMPLE_FIELDS.map((f) => {
                const raw = cfg[f.key];
                const num = f.kind === "int"
                  ? parseInt(String(raw), 10) || 0
                  : parseFloat(String(raw)) || 0;
                const displayValue = f.kind === "int" ? num : num * 100;
                const dollars =
                  f.kind === "pct" && equity > 0 ? equity * num : null;

                return (
                  <div key={f.key} className="ee-simple-field">
                    <div className="ee-simple-field__header">
                      <div>
                        <div className="ee-simple-field__label">{f.label}</div>
                        <div className="ee-simple-field__hint">{f.hint}</div>
                      </div>
                      <div className="ee-simple-field__value">
                        {f.kind === "pct"
                          ? `${displayValue.toFixed(2)}%`
                          : `${num}`}
                      </div>
                    </div>

                    <input
                      type="range"
                      min={f.min * (f.kind === "pct" ? 100 : 1)}
                      max={f.max * (f.kind === "pct" ? 100 : 1)}
                      step={f.step * (f.kind === "pct" ? 100 : 1)}
                      value={displayValue}
                      onChange={(e) => {
                        const v = parseFloat(e.target.value) || 0;
                        const newValue = f.kind === "int"
                          ? String(Math.round(v))
                          : String(v / 100);
                        setCfg({ ...cfg, [f.key]: newValue });
                      }}
                      className="ee-simple-slider"
                    />

                    {dollars !== null && equity > 0 && (
                      <div className="ee-simple-field__example">
                        = <strong>{fmtMoney(dollars)}</strong> on {fmtMoney(equity)} equity
                      </div>
                    )}

                    {f.kind === "pct" && num === 0 && f.key === "daily_loss_limit" && (
                      <div className="ee-simple-field__example ee-muted">
                        (disabled — no daily cap)
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Advanced mode */}
        {mode === "advanced" && (
          <>
            <nav className="ee-modal__tabs">
              {(["risk", "caps", "recovery", "advanced"] as AdvTab[]).map((t) => (
                <button
                  key={t}
                  className={`ee-modal__tab ${advTab === t ? "is-active" : ""}`}
                  onClick={() => setAdvTab(t)}
                >
                  {t.charAt(0).toUpperCase() + t.slice(1)}
                </button>
              ))}
            </nav>
            <div className="ee-modal__body">
              <div className="ee-form-grid">
                {ADV_TABS[advTab].map((f) => (
                  <label key={f.key} className="ee-field">
                    <span>
                      {f.label}
                      {f.hint && <em className="ee-hint"> — {f.hint}</em>}
                    </span>
                    {f.kind === "number" ? (
                      <input
                        type="number"
                        value={cfg[f.key] as number}
                        onChange={(e) =>
                          setCfg({
                            ...cfg,
                            [f.key]: parseInt(e.target.value, 10) || 0,
                          })
                        }
                      />
                    ) : (
                      <input
                        className="mono"
                        value={cfg[f.key] as string}
                        onChange={(e) =>
                          setCfg({ ...cfg, [f.key]: e.target.value })
                        }
                      />
                    )}
                  </label>
                ))}
              </div>
            </div>
          </>
        )}

        {/* Validation errors */}
        {validation.errors.length > 0 && (
          <div style={{ padding: "0 22px 12px" }}>
            <div className="ee-alert ee-alert--error">
              <strong>Validation:</strong>
              <ul style={{ margin: "4px 0 0 16px", padding: 0 }}>
                {validation.errors.map((e, i) => (
                  <li key={i}>{e}</li>
                ))}
              </ul>
            </div>
          </div>
        )}

        {/* Preview */}
        {preview && validation.errors.length === 0 && hasChanges && (
          <div style={{ padding: "0 22px 12px" }}>
            <div className="ee-alert ee-alert--warn">
              <strong>Preview on current equity {fmtMoney(equity)}:</strong>
              <div style={{ marginTop: 6, fontSize: 12 }}>
                Base risk = {(parseDec(cfg.r0) ?? 0) * 100}% →{" "}
                <strong>{fmtMoney(equity * (parseDec(cfg.r0) ?? 0))}</strong>
                {(parseDec(cfg.daily_loss_limit) ?? 0) > 0 && (
                  <>
                    {" "}· Daily cap →{" "}
                    <strong>
                      {fmtMoney(equity * (parseDec(cfg.daily_loss_limit) ?? 0))}
                    </strong>
                  </>
                )}
              </div>
            </div>
          </div>
        )}

        {error && (
          <div style={{ padding: "0 22px 12px" }}>
            <div className="ee-alert ee-alert--error">{error}</div>
          </div>
        )}
        {ok && (
          <div style={{ padding: "0 22px 12px" }}>
            <div className="ee-alert ee-alert--success">Saved.</div>
          </div>
        )}

        <div className="ee-modal__footer">
          {hasChanges && (
            <button className="ee-btn ee-btn--ghost" onClick={discard}>
              Discard changes
            </button>
          )}
          <button className="ee-btn ee-btn--ghost" onClick={reset}>
            Reset defaults
          </button>
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Cancel
          </button>
          <button
            className="ee-btn ee-btn--primary"
            onClick={save}
            disabled={busy || validation.errors.length > 0 || !hasChanges}
          >
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}

/* ============================================================
 * Validation
 * ============================================================ */
function validate(cfg: EquityConfig): { errors: string[] } {
  const errors: string[] = [];

  const dec = (k: keyof EquityConfig) => {
    const v = parseDec(cfg[k] as string | undefined);
    return v;
  };

  const r0 = dec("r0");
  if (r0 === null || r0 <= 0 || r0 > 0.2) {
    errors.push("R₀ must be > 0 and ≤ 20%");
  }
  const rMin = dec("r_min");
  if (rMin !== null && rMin < 0) {
    errors.push("r_min cannot be negative");
  }
  const daily = dec("daily_loss_limit");
  if (daily !== null && (daily < 0 || daily > 1)) {
    errors.push("Daily loss limit must be 0–100% (0 = disabled)");
  }
  const portMax = dec("r_portfolio_max");
  if (portMax !== null && (portMax < 0 || portMax > 1)) {
    errors.push("Portfolio risk max must be 0–100% (0 = disabled)");
  }
  const propMax = dec("r_prop_max");
  if (propMax !== null && (propMax < 0 || propMax > 1)) {
    errors.push("Prop per-trade max must be 0–100% (0 = disabled)");
  }
  const q = dec("q_retention");
  if (q !== null && (q < 0 || q > 1)) {
    errors.push("Q (profit retention) must be 0–100%");
  }
  const dTrail = dec("d_trailing");
  if (dTrail !== null && (dTrail < 0 || dTrail > 1)) {
    errors.push("d (trailing) must be 0–100%");
  }
  const alpha = dec("alpha");
  if (alpha !== null && (alpha < 0 || alpha > 1)) {
    errors.push("α (floor risk factor) must be 0–100%");
  }

  // DD tiers monotonic
  const dd1 = dec("dd_1");
  const dd2 = dec("dd_2");
  const dd3 = dec("dd_3");
  if (dd1 !== null && dd2 !== null && dd1 >= dd2) {
    errors.push("DD tier 1 must be < DD tier 2");
  }
  if (dd2 !== null && dd3 !== null && dd2 >= dd3) {
    errors.push("DD tier 2 must be < DD tier 3");
  }

  if (cfg.n_max < 1) {
    errors.push("N_max must be ≥ 1");
  }

  return { errors };
}
