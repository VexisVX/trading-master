import { useEffect, useMemo, useState } from "react";
import {
  EquityConfig,
  RiskPreview,
  AccountProfile,
  AccountType,
  ProfileInput,
  defaultConfig,
  getEquityConfig,
  setEquityConfig,
  getAccountProfile,
  setAccountProfile,
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
    hint: "Stop trading for the day after losing this much (0 = disabled)",
    min: 0,
    max: 0.2,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "r_portfolio_max",
    label: "Max total open risk",
    hint: "Total risk across all open trades combined (0 = disabled)",
    min: 0,
    max: 0.3,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "r_prop_max",
    label: "Prop per-trade max",
    hint: "Per-trade risk cap for prop firms (0 = disabled)",
    min: 0,
    max: 0.1,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "profit_protect_activation",
    label: "Profit protection activation",
    hint: "NAV growth at which profit protection starts (0.05 = 5%)",
    min: 0.01,
    max: 0.20,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "profit_protect_full",
    label: "Full protection activation",
    hint: "NAV growth at which protection reaches max retention (0.10 = 10%)",
    min: 0.02,
    max: 0.30,
    step: 0.005,
    kind: "pct",
  },
  {
    key: "max_profit_retention",
    label: "Max profit retention",
    hint: "Fraction of accumulated profit locked as protected (0.40 = 40%)",
    min: 0.10,
    max: 0.60,
    step: 0.05,
    kind: "pct",
  },
  {
    key: "trailing_dd_pct",
    label: "Trailing drawdown (internal)",
    hint: "Internal trailing floor from NAV HWM (0.10 = 10%)",
    min: 0.02,
    max: 0.20,
    step: 0.005,
    kind: "pct",
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
    { key: "profit_protect_activation", label: "Profit protection activation", hint: "0.05 = 5% growth", kind: "text" },
    { key: "profit_protect_full", label: "Full protection activation", hint: "0.10 = 10% growth", kind: "text" },
    { key: "max_profit_retention", label: "Max profit retention", hint: "0.40 = 40%", kind: "text" },
    { key: "trailing_dd_pct", label: "Trailing DD (internal)", hint: "0.10 = 10%", kind: "text" },
    { key: "execution_buffer_pct", label: "Execution buffer", hint: "0.001 = 0.1%", kind: "text" },
  ],
  caps: [
    { key: "daily_loss_limit", label: "Daily loss limit", hint: "0 = disabled", kind: "text" },
    { key: "r_portfolio_max", label: "Portfolio risk max", hint: "0 = disabled", kind: "text" },
    { key: "r_prop_max", label: "Prop per-trade max", hint: "0 = disabled", kind: "text" },
  ],
  recovery: [
    { key: "alpha", label: "α — floor risk factor", hint: "0.5 = 50% of loss capacity", kind: "text" },
    { key: "d_prop", label: "D_prop (prop budget)", hint: "0 = disabled", kind: "text" },
    { key: "recovery_probe_pct", label: "Recovery probe risk", hint: "0.25 = 25% of R_base", kind: "text" },
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

  // S14: account profile state
  const [profile, setProfile] = useState<AccountProfile | null>(null);
  const [profileType, setProfileType] = useState<AccountType>("personal");
  const [profileTarget, setProfileTarget] = useState<string>("0.08");
  const [profileDD, setProfileDD] = useState<string>("0.05");
  const [profileBusy, setProfileBusy] = useState(false);
  const [profileErr, setProfileErr] = useState<string | null>(null);
  const [profileSavedAt, setProfileSavedAt] = useState<string | null>(null);
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
  }, [])

  // S14: load account profile
  useEffect(() => {
    (async () => {
      try {
        const activeId = await (async () => {
          try {
            const { getActiveAccountId } = await import("../../lib/accountsApi");
            return await getActiveAccountId();
          } catch { return null; }
        })();
        if (!activeId) return;
        const p = await getAccountProfile(activeId);
        if (p) {
          setProfile(p);
          setProfileType(p.account_type);
          setProfileTarget(p.profit_target_pct);
          setProfileDD(p.max_dd_pct);
        }
      } catch (e) {
        // non-fatal — settings still work without profile
        console.warn("profile load failed", e);
      }
    })();
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
      // S14: if profile exists and type is personal/prop, also save it
      if (profile) {
        // Profile is saved separately via the Account Profile section
      }
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

  async function saveProfile() {
    setProfileBusy(true);
    setProfileErr(null);
    try {
      const activeId = await (async () => {
        try {
          const { getActiveAccountId } = await import("../../lib/accountsApi");
          return await getActiveAccountId();
        } catch { return null; }
      })();
      if (!activeId) {
        setProfileErr("No active account selected");
        return;
      }
      const input: ProfileInput = {
        account_type: profileType,
        profit_target_pct: profileTarget,
        max_dd_pct: profileDD,
      };
      const saved = await setAccountProfile(activeId, input);
      setProfile(saved);
      setProfileSavedAt(new Date().toLocaleTimeString());
    } catch (e) {
      setProfileErr(String(e));
    } finally {
      setProfileBusy(false);
    }
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

        <div className="ee-modal__body"><div className="settings__section ee-profile-section">
              <h2 className="section-title">Account Profile</h2>
              <p className="settings__hint">
                Adaptive preset for the active account. Internal protection settings
                are derived automatically — they are <strong>not</strong> the actual
                prop-firm rules (those live in the prop rules table).
              </p>

              <div className="ee-profile-row">
                <label className="ee-profile-row__label">Account type</label>
                <select
                  value={profileType}
                  onChange={(e) => setProfileType(e.target.value as AccountType)}
                  className="input"
                >
                  <option value="personal">Personal</option>
                  <option value="prop_firm">Prop firm</option>
                </select>
              </div>

              {profileType === "prop_firm" && (
                <>
                  <div className="ee-profile-row">
                    <label className="ee-profile-row__label">Profit target</label>
                    <input
                      type="number"
                      step="0.01"
                      min="0.01"
                      max="0.50"
                      value={profileTarget}
                      onChange={(e) => setProfileTarget(e.target.value)}
                      className="input"
                      
                    />
                    <span className="ee-profile-row__hint">e.g. 0.08 = 8%</span>
                  </div>
                  <div className="ee-profile-row">
                    <label className="ee-profile-row__label">Max drawdown</label>
                    <input
                      type="number"
                      step="0.01"
                      min="0.01"
                      max="0.20"
                      value={profileDD}
                      onChange={(e) => setProfileDD(e.target.value)}
                      className="input"
                      
                    />
                    <span className="ee-profile-row__hint">must be &lt; profit target</span>
                  </div>
                </>
              )}

              <div className="ee-profile-actions">
                <button
                  className="btn btn--primary"
                  onClick={saveProfile}
                  disabled={profileBusy}
                >
                  {profileBusy ? "Saving…" : "Save profile"}
                </button>
                {profileSavedAt && <span className="ee-profile-saved">Saved at {profileSavedAt}</span>}
              </div>

              {profileErr && <div className="ee-profile-error">{profileErr}</div>}

              {profile && (
                <div className="ee-profile-derived">
                  <div className="ee-profile-derived__title">Derived values</div>
                  <table className="ee-profile-derived__table">
                    <tbody>
                      <tr><td>Activation</td><td className="mono">{profile.profit_protect_activation}</td></tr>
                      <tr><td>Full activation</td><td className="mono">{profile.profit_protect_full}</td></tr>
                      <tr><td>Max retention</td><td className="mono">{profile.max_profit_retention}</td></tr>
                      <tr><td>Trailing DD</td><td className="mono">{profile.trailing_dd_pct}</td></tr>
                    </tbody>
                  </table>
                </div>
              )}
            </div></div>

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
    const v = cfg[k];
    if (typeof v !== "string") return NaN;
    const n = parseFloat(v);
    return Number.isFinite(n) ? n : NaN;
  };

  const r0 = dec("r0");
  if (!Number.isFinite(r0) || r0 <= 0 || r0 > 0.05) errors.push("R₀ must be in (0, 5%]");

  const pAct = dec("profit_protect_activation");
  const pFull = dec("profit_protect_full");
  if (!Number.isFinite(pAct) || pAct <= 0) errors.push("Profit activation must be > 0");
  if (!Number.isFinite(pFull) || pFull <= pAct) errors.push("Full activation must be > activation");

  const retention = dec("max_profit_retention");
  if (!Number.isFinite(retention) || retention <= 0 || retention > 0.6) errors.push("Retention must be in (0, 60%]");

  const trailing = dec("trailing_dd_pct");
  if (!Number.isFinite(trailing) || trailing <= 0 || trailing > 0.2) errors.push("Trailing DD must be in (0, 20%]");

  return { errors };
}
