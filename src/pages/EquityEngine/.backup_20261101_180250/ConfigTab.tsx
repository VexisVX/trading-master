import { useEffect, useState } from "react";
import {
  EquityConfig,
  defaultConfig,
  getEquityConfig,
  setEquityConfig,
} from "../../lib/equityApi";

interface Field {
  key: keyof EquityConfig;
  label: string;
  hint?: string;
  kind: "text" | "number";
}

const FIELDS: Field[] = [
  { key: "r0", label: "Base risk (R₀)", hint: "e.g. 0.01 = 1%", kind: "text" },
  { key: "r_min", label: "Minimum risk", kind: "text" },
  { key: "delta_r", label: "Step size (ΔR)", kind: "text" },
  { key: "dd_1", label: "DD tier 1", hint: "e.g. 0.05", kind: "text" },
  { key: "dd_2", label: "DD tier 2", kind: "text" },
  { key: "dd_3", label: "DD tier 3", kind: "text" },
  { key: "m_dd_1", label: "Multiplier tier 1", kind: "text" },
  { key: "m_dd_2", label: "Multiplier tier 2", kind: "text" },
  { key: "m_dd_3", label: "Multiplier tier 3", kind: "text" },
  { key: "m_dd_4", label: "Multiplier tier 4+", kind: "text" },
  {
    key: "m_loss_table",
    label: "Loss-streak table",
    hint: "comma-separated, index = consecutive losses",
    kind: "text",
  },
  { key: "n_max", label: "N_max (pause at)", kind: "number" },
  { key: "q_retention", label: "Q — profit retention", hint: "e.g. 0.3", kind: "text" },
  { key: "d_trailing", label: "d — trailing from HWM", kind: "text" },
  { key: "d_prop", label: "D_prop (prop budget)", kind: "text" },
  { key: "alpha", label: "α — floor risk factor", kind: "text" },
  { key: "daily_loss_limit", label: "Daily loss limit", hint: "0 = disabled", kind: "text" },
  { key: "r_portfolio_max", label: "Portfolio risk max", hint: "0 = disabled", kind: "text" },
  { key: "r_prop_max", label: "Prop per-trade max", hint: "0 = disabled", kind: "text" },
  { key: "mu_max", label: "MU_max", kind: "text" },
  {
    key: "max_snapshot_age_hours",
    label: "Max snapshot age (h)",
    kind: "number",
  },
  { key: "recovery_target", label: "Recovery target", kind: "text" },
];

export function ConfigTab() {
  const [cfg, setCfg] = useState<EquityConfig>(defaultConfig());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState(false);
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    getEquityConfig()
      .then((c) => {
        setCfg(c);
        setLoaded(true);
      })
      .catch((e) => setError(String(e)));
  }, []);

  async function save(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setOk(false);
    setBusy(true);
    try {
      await setEquityConfig(cfg);
      setOk(true);
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  function reset() {
    setCfg(defaultConfig());
    setOk(false);
  }

  if (!loaded) return <div className="ee-empty card">Loading config…</div>;

  return (
    <form className="ee-config card" onSubmit={save}>
      <div className="ee-config__hint">
        All values are Decimal strings. Percentages use 0.0–1.0 (e.g. 0.05 =
        5%). Save validates on the Rust side — invalid values are rejected.
      </div>

      <div className="ee-form-grid ee-form-grid--wide">
        {FIELDS.map((f) => (
          <label key={f.key}>
            <span>
              {f.label}
              {f.hint && <em className="ee-config__hint-inline"> — {f.hint}</em>}
            </span>
            {f.kind === "number" ? (
              <input
                type="number"
                value={cfg[f.key] as number}
                onChange={(e) =>
                  setCfg({ ...cfg, [f.key]: parseInt(e.target.value, 10) || 0 })
                }
              />
            ) : (
              <input
                className="mono"
                value={cfg[f.key] as string}
                onChange={(e) => setCfg({ ...cfg, [f.key]: e.target.value })}
              />
            )}
          </label>
        ))}
      </div>

      {error && <div className="ee-error">{error}</div>}
      {ok && <div className="ee-success">Config saved.</div>}

      <div className="ee-config__actions">
        <button className="btn btn--primary" disabled={busy}>
          {busy ? "Saving…" : "Save config"}
        </button>
        <button type="button" className="btn btn--ghost" onClick={reset}>
          Reset to defaults
        </button>
      </div>
    </form>
  );
}
