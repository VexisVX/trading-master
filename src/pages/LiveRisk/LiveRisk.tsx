import { useEffect, useState } from "react";
import {
  calculateLiveRisk, LiveCalcInput, LiveCalcResult,
  calcPartialClose, calcMartingaleCheck, calcBreakEven, calcSizeFromRisk,
  getAssistant, AssistantResult,
  getPortfolioRisk, PortfolioRiskResult, PortfolioTradeInput,
} from "../../lib/liveCalcApi";
import {
  listPresets, createPreset, deletePreset, LivePreset, createDraft,
} from "../../lib/livePresetsApi";
import { getRiskSettings } from "../../lib/riskSettingsApi";
import { getActiveAccountId } from "../../lib/accountsApi";
import { HelpTip } from "../../components/ui/HelpTip";
import { Select } from "../../components/ui/Select";
import "./LiveRisk.css";

interface Preset {
  label: string;
  value_per_price_unit: string;
  approx_price: string;
  margin_1lot: string;
}

const PRESETS: Record<string, Preset> = {
  custom:  { label: "Custom",  value_per_price_unit: "1",      approx_price: "",       margin_1lot: "1" },
  eurusd:  { label: "EURUSD",  value_per_price_unit: "100000", approx_price: "1.1000", margin_1lot: "1100" },
  gbpusd:  { label: "GBPUSD",  value_per_price_unit: "100000", approx_price: "1.2700", margin_1lot: "1270" },
  usdjpy:  { label: "USDJPY",  value_per_price_unit: "667",    approx_price: "150.00", margin_1lot: "1500" },
  xauusd:  { label: "XAUUSD",  value_per_price_unit: "100",    approx_price: "2400",   margin_1lot: "2400" },
  nas100:  { label: "NAS100",  value_per_price_unit: "1",      approx_price: "18000",  margin_1lot: "1800" },
  btcusd:  { label: "BTCUSD",  value_per_price_unit: "1",      approx_price: "65000",  margin_1lot: "650" },
};

export function LiveRisk() {
  const [lrTab, setLrTab] = useState<"single" | "multi">("single");

  const [preset, setPreset] = useState("eurusd");
  const [balance, setBalance] = useState("10000");
  const [maxRiskPct, setMaxRiskPct] = useState("1");
  const [size, setSize] = useState("1");
  const [entry, setEntry] = useState("");
  const [stop, setStop] = useState("");
  const [tp, setTp] = useState("");
  const [marginUsd, setMarginUsd] = useState("1000");
  const [valuePerUnit, setValuePerUnit] = useState("100000");

  const [commissionPerLot, setCommissionPerLot] = useState("0");
  const [spreadPoints, setSpreadPoints] = useState("0");
  const [riskMode, setRiskMode] = useState<"pct" | "fixed">("pct");
  const [fixedRiskAmount, setFixedRiskAmount] = useState("");
  const [minQty, setMinQty] = useState("");
  const [maxQty, setMaxQty] = useState("");
  const [qtyStep, setQtyStep] = useState("");

  const [presets, setPresets] = useState<LivePreset[]>([]);
  const [accountId, setAccountId] = useState<number | null>(null);

  const [result, setResult] = useState<LiveCalcResult | null>(null);
  const [assistant, setAssistant] = useState<AssistantResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const id = await getActiveAccountId();
        setAccountId(id);
        const rs = await getRiskSettings(id);
        if (rs) setMaxRiskPct(rs.max_risk_per_trade_pct);
        listPresets(id).then(setPresets).catch(() => {});
      } catch { /* ignore */ }
    })();
  }, []);

  function applyPreset(name: string) {
    setPreset(name);
    const p = PRESETS[name];
    if (!p || name === "custom") return;
    setValuePerUnit(p.value_per_price_unit);
    setMarginUsd(p.margin_1lot);
    if (!entry && p.approx_price) setEntry(p.approx_price);
  }

  useEffect(() => {
    (async () => {
      if (!entry || !stop || !size) { setResult(null); setAssistant(null); return; }
      setError(null);
      try {
        const input: LiveCalcInput = {
          mode: "simple",
          size, entry_price: entry, stop_price: stop,
          take_profit: tp || null, balance,
          contract_size: null, value_per_point: null, point_size: null, leverage: null,
          margin_usd: marginUsd,
          value_per_price_unit: valuePerUnit,
          commission_per_lot: commissionPerLot || null,
          spread_points: spreadPoints || null,
          min_quantity: minQty || null,
          max_quantity: maxQty || null,
          quantity_step: qtyStep || null,
          max_risk_per_trade_pct: riskMode === "pct" ? (maxRiskPct || null) : null,
          risk_mode: riskMode,
          fixed_risk_amount: riskMode === "fixed" ? (fixedRiskAmount || null) : null,
        };
        setResult(await calculateLiveRisk(input));

        const a = await getAssistant({
          size, entry_price: entry, stop_price: stop,
          take_profit: tp || null, balance,
          value_per_price_unit: valuePerUnit,
          max_risk_pct: maxRiskPct,
        });
        setAssistant(a);
      } catch (e) {
        setError(String(e));
        setResult(null);
        setAssistant(null);
      }
    })();
  }, [size, entry, stop, tp, balance, marginUsd, valuePerUnit, maxRiskPct,
      commissionPerLot, spreadPoints, riskMode, fixedRiskAmount, minQty, maxQty, qtyStep]);

  function applySuggestedSize(s: string) { setSize(s); }

  async function savePreset() {
    if (!accountId) return;
    const name = prompt("Preset name:");
    if (!name) return;
    const payload = JSON.stringify({
      preset, size, entry, stop, tp, marginUsd, valuePerUnit, maxRiskPct,
      commissionPerLot, spreadPoints, minQty, maxQty, qtyStep,
      riskMode, fixedRiskAmount, balance,
    });
    try {
      await createPreset(accountId, { name, instrument: preset, payload_json: payload });
      setPresets(await listPresets(accountId));
    } catch (e) { setError(String(e)); }
  }

  async function saveDraft() {
    if (!accountId) return;
    const payload = JSON.stringify({
      preset, size, entry, stop, tp, marginUsd, valuePerUnit, maxRiskPct,
      commissionPerLot, spreadPoints, minQty, maxQty, qtyStep,
      riskMode, fixedRiskAmount, balance,
    });
    try {
      await createDraft(accountId, payload);
      alert("Draft saved. Open Journal to create the trade.");
    } catch (e) { setError(String(e)); }
  }

  function loadPreset(p: LivePreset) {
    try {
      const d = JSON.parse(p.payload_json);
      if (d.preset) setPreset(d.preset);
      if (d.size) setSize(d.size);
      if (d.entry) setEntry(d.entry);
      if (d.stop) setStop(d.stop);
      if (d.tp) setTp(d.tp);
      if (d.marginUsd) setMarginUsd(d.marginUsd);
      if (d.valuePerUnit) setValuePerUnit(d.valuePerUnit);
      if (d.maxRiskPct) setMaxRiskPct(d.maxRiskPct);
      if (d.commissionPerLot) setCommissionPerLot(d.commissionPerLot);
      if (d.spreadPoints) setSpreadPoints(d.spreadPoints);
      if (d.minQty) setMinQty(d.minQty);
      if (d.maxQty) setMaxQty(d.maxQty);
      if (d.qtyStep) setQtyStep(d.qtyStep);
      if (d.riskMode) setRiskMode(d.riskMode);
      if (d.fixedRiskAmount) setFixedRiskAmount(d.fixedRiskAmount);
      if (d.balance) setBalance(d.balance);
    } catch (e) { setError(String(e)); }
  }

  async function removePreset(id: number) {
    if (!confirm("Delete this preset?")) return;
    try {
      await deletePreset(id);
      if (accountId) setPresets(await listPresets(accountId));
    } catch (e) { setError(String(e)); }
  }

  return (
    <div className="live-risk">
      <div className="lr-tabs">
        <button className={`lr-tab ${lrTab === "single" ? "is-active" : ""}`} onClick={() => setLrTab("single")}>
          Single Trade
        </button>
        <button className={`lr-tab ${lrTab === "multi" ? "is-active" : ""}`} onClick={() => setLrTab("multi")}>
          Multi-Position
        </button>
      </div>

      {lrTab === "single" && (
        <>
          <div className="card lr-setup">
            <div className="lr-setup__head">
              <h2>Live Trade Setup</h2>
              <p className="dim">All numbers below feed every calculator on this page.</p>
            </div>
            <div className="lr-setup__grid">
              <label className="lr-field">
                <span>Instrument</span>
                <Select
                  value={preset}
                  onChange={applyPreset}
                  options={Object.entries(PRESETS).map(([k, v]) => ({ value: k, label: v.label }))}
                />
              </label>
              <label className="lr-field">
                <span>Balance</span>
                <input value={balance} onChange={(e) => setBalance(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Max risk %<HelpTip title="Max risk per trade">From your active account. Warnings only.</HelpTip></span>
                <input value={maxRiskPct} onChange={(e) => setMaxRiskPct(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Size (lots)</span>
                <input value={size} onChange={(e) => setSize(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Entry</span>
                <input value={entry} onChange={(e) => setEntry(e.target.value)} placeholder="1.1000" />
              </label>
              <label className="lr-field">
                <span>Stop</span>
                <input value={stop} onChange={(e) => setStop(e.target.value)} placeholder="1.0950" />
              </label>
              <label className="lr-field">
                <span>Take profit</span>
                <input value={tp} onChange={(e) => setTp(e.target.value)} placeholder="1.1150" />
              </label>
              <label className="lr-field">
                <span>Margin held ($)</span>
                <input value={marginUsd} onChange={(e) => setMarginUsd(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Value per 1.0 move</span>
                <input value={valuePerUnit} onChange={(e) => setValuePerUnit(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Commission / lot</span>
                <input value={commissionPerLot} onChange={(e) => setCommissionPerLot(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Spread (points)</span>
                <input value={spreadPoints} onChange={(e) => setSpreadPoints(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Min qty</span>
                <input value={minQty} onChange={(e) => setMinQty(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Max qty</span>
                <input value={maxQty} onChange={(e) => setMaxQty(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Qty step</span>
                <input value={qtyStep} onChange={(e) => setQtyStep(e.target.value)} />
              </label>
              <label className="lr-field">
                <span>Risk mode</span>
                <Select
                  value={riskMode}
                  onChange={(v) => setRiskMode(v as "pct" | "fixed")}
                  options={[
                    { value: "pct", label: "Percent" },
                    { value: "fixed", label: "Fixed $" },
                  ]}
                />
              </label>
              {riskMode === "fixed" && (
                <label className="lr-field">
                  <span>Fixed risk ($)</span>
                  <input value={fixedRiskAmount} onChange={(e) => setFixedRiskAmount(e.target.value)} />
                </label>
              )}
            </div>

            <div className="lr-preset-row">
              <button className="btn btn--ghost btn--small" onClick={savePreset} disabled={!accountId}>
                Save preset
              </button>
              <button className="btn btn--ghost btn--small" onClick={saveDraft} disabled={!accountId}>
                Save as draft
              </button>
              {presets.length > 0 && (
                <>
                  <Select
                    value=""
                    onChange={(v) => {
                      const p = presets.find((x) => String(x.id) === v);
                      if (p) loadPreset(p);
                    }}
                    options={[
                      { value: "", label: `Load preset (${presets.length})` },
                      ...presets.map((p) => ({ value: String(p.id), label: p.name })),
                    ]}
                  />
                  <Select
                    value=""
                    onChange={(v) => {
                      if (v) removePreset(Number(v));
                    }}
                    options={[
                      { value: "", label: "Delete preset…" },
                      ...presets.map((p) => ({ value: String(p.id), label: p.name })),
                    ]}
                  />
                </>
              )}
            </div>

            {error && <div className="lr-error">{error}</div>}
          </div>

          <div className="lr-dashboard">
            <div className={`card lr-assistant lr-assistant--${assistant?.verdict ?? "idle"}`}>
              <div className="lr-assistant__head">
                <h3>Assistant</h3>
                {assistant && (
                  <span className={`lr-verdict lr-verdict--${assistant.verdict}`}>
                    {assistant.verdict === "ok" ? "Safe" : assistant.verdict === "warn" ? "Caution" : "Danger"}
                  </span>
                )}
              </div>
              {!assistant ? (
                <p className="dim">Fill entry and stop to get advice.</p>
              ) : (
                <>
                  <p className="lr-assistant__headline">{assistant.headline}</p>
                  <p className="lr-assistant__risk">
                    Current risk: <span className="mono">{assistant.current_risk_pct}%</span> of balance
                  </p>
                  {assistant.suggested_size && assistant.suggested_size !== size && (
                    <div className="lr-assistant__suggest">
                      <span>Suggested size for {maxRiskPct}%:</span>
                      <span className="mono">{assistant.suggested_size} lots</span>
                      <button
                        className="btn btn--primary btn--small"
                        onClick={() => applySuggestedSize(assistant.suggested_size!)}
                      >
                        Apply
                      </button>
                    </div>
                  )}
                  <ul className="lr-advice">
                    {assistant.advices.map((a, i) => (
                      <li key={i} className={`lr-advice__item lr-advice__item--${a.severity}`}>
                        <span className="lr-advice__dot" />
                        <span>{a.message}</span>
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </div>

            {result && (
              <div className="card lr-snapshot">
                <h3>Snapshot</h3>
                <div className="lr-snapshot__grid">
                  <Snap label="Stop distance" value={result.stop_distance_price} />
                  <Snap label="Risk per lot" value={`${result.risk_per_lot} USD`} />
                  <Snap label="Gross risk" value={`${result.total_risk} USD`} />
                  <Snap label="Net risk" value={`${result.net_risk} USD`} accent />
                  <Snap label="Risk % (net)" value={`${result.net_risk_pct}%`} accent />
                  <Snap label="Costs" value={`${result.total_costs} USD`} />
                  <Snap label="Reward (net)" value={result.net_reward ? `${result.net_reward} USD` : "—"} />
                  <Snap label="R:R (net)" value={result.planned_rr ? `1 : ${result.planned_rr}` : "—"} />
                  <Snap label="Required margin" value={`${result.required_margin} USD`} />
                  <Snap label="Margin used %" value={`${result.margin_used_pct}%`} />
                  <Snap label="Free margin after" value={`${result.free_margin_after} USD`} />
                  <Snap label="Size" value={`${size} lots`} />
                </div>
                <RiskBar riskPct={Number(result.net_risk_pct)} limitPct={Number(maxRiskPct)} />
              </div>
            )}
          </div>

          <LiveLab
            defaultBalance={balance}
            defaultValuePerUnit={valuePerUnit}
            defaultSize={size}
            defaultEntry={entry}
            defaultStop={stop}
          />
        </>
      )}

      {lrTab === "multi" && (
        <PortfolioAssistant balance={balance} valuePerUnit={valuePerUnit} />
      )}
    </div>
  );
}

function Snap({ label, value, accent }: { label: string; value: string; accent?: boolean }) {
  return (
    <div className="lr-snap">
      <span className="lr-snap__label">{label}</span>
      <span className={`lr-snap__value mono ${accent ? "is-accent" : ""}`}>{value}</span>
    </div>
  );
}

function RiskBar({ riskPct, limitPct }: { riskPct: number; limitPct: number }) {
  const max = Math.max(limitPct * 2, riskPct * 1.2, 2);
  const pct = Math.min(100, (riskPct / max) * 100);
  const limitLine = (limitPct / max) * 100;
  const color = riskPct > limitPct ? "var(--danger)" : riskPct > limitPct * 0.75 ? "var(--warning)" : "var(--accent)";
  return (
    <div className="lr-rb">
      <div className="lr-rb__track">
        <div className="lr-rb__fill" style={{ width: `${pct}%`, background: color }} />
        <div className="lr-rb__limit" style={{ left: `${limitLine}%` }} />
      </div>
      <div className="lr-rb__labels">
        <span>0%</span>
        <span className="dim">limit {limitPct}%</span>
        <span>{max.toFixed(1)}%</span>
      </div>
    </div>
  );
}

interface LabProps {
  defaultBalance: string;
  defaultValuePerUnit: string;
  defaultSize: string;
  defaultEntry: string;
  defaultStop: string;
}

function LiveLab({ defaultBalance, defaultValuePerUnit, defaultSize, defaultEntry, defaultStop }: LabProps) {
  const [tab, setTab] = useState<"partial" | "martingale" | "breakeven" | "sizing">("partial");

  return (
    <div className="card lr-lab">
      <div className="lr-lab__head">
        <h3>Live Trade Lab</h3>
        <p className="dim">Same setup — try what-if scenarios without re-entering numbers.</p>
      </div>
      <div className="lr-lab__tabs">
        <button className={`lr-lab__tab ${tab === "partial" ? "is-active" : ""}`} onClick={() => setTab("partial")}>Save profit</button>
        <button className={`lr-lab__tab ${tab === "martingale" ? "is-active" : ""}`} onClick={() => setTab("martingale")}>Martingale check</button>
        <button className={`lr-lab__tab ${tab === "breakeven" ? "is-active" : ""}`} onClick={() => setTab("breakeven")}>Break-even</button>
        <button className={`lr-lab__tab ${tab === "sizing" ? "is-active" : ""}`} onClick={() => setTab("sizing")}>Size from risk</button>
      </div>
      <div className="lr-lab__body">
        {tab === "partial" && <PartialCalc vpu={defaultValuePerUnit} size={defaultSize} entry={defaultEntry} />}
        {tab === "martingale" && <MartingaleCalc size={defaultSize} />}
        {tab === "breakeven" && <BreakEvenCalc vpu={defaultValuePerUnit} entry={defaultEntry} size={defaultSize} />}
        {tab === "sizing" && <SizeFromRiskCalc vpu={defaultValuePerUnit} balance={defaultBalance} entry={defaultEntry} stop={defaultStop} />}
      </div>
    </div>
  );
}

function PartialCalc({ vpu, size, entry }: { vpu: string; size: string; entry: string }) {
  const [current, setCurrent] = useState("");
  const [fraction, setFraction] = useState("0.5");
  const [res, setRes] = useState<{ locked_profit: string; remaining_size: string; remaining_potential_per_1unit: string; be_price_after: string } | null>(null);

  useEffect(() => {
    (async () => {
      if (!entry || !current || !size) { setRes(null); return; }
      try {
        setRes(await calcPartialClose({
          size, entry_price: entry, current_price: current,
          value_per_price_unit: vpu, close_fraction: fraction,
        }));
      } catch { setRes(null); }
    })();
  }, [size, entry, current, vpu, fraction]);

  return (
    <div className="lr-calc">
      <div className="lr-calc__fixed">
        <Fixed label="Size" value={`${size} lots`} />
        <Fixed label="Entry" value={entry} />
        <Fixed label="Value/unit" value={vpu} />
      </div>
      <div className="lr-calc__row">
        <Field label="Close at price" value={current} onChange={setCurrent} placeholder="1.1150" />
        <Field label="Close fraction (0-1)" value={fraction} onChange={setFraction} />
      </div>
      {res && (
        <div className="lr-calc__out">
          <Out label="Locked profit" value={`${res.locked_profit} USD`} tone="pos" />
          <Out label="Remaining size" value={`${res.remaining_size} lots`} />
          <Out label="Remaining BE price" value={res.be_price_after} tone="accent" />
        </div>
      )}
    </div>
  );
}

function MartingaleCalc({ size }: { size: string }) {
  const [prev, setPrev] = useState("1");
  const [recent, setRecent] = useState(true);
  const [res, setRes] = useState<{ verdict: string; message: string; multiplier: string | null } | null>(null);

  useEffect(() => {
    (async () => {
      if (!size) { setRes(null); return; }
      try {
        setRes(await calcMartingaleCheck({
          current_size: size,
          last_losing_size: prev || null,
          last_losing_was_recent: recent,
        }));
      } catch { setRes(null); }
    })();
  }, [size, prev, recent]);

  return (
    <div className="lr-calc">
      <div className="lr-calc__fixed">
        <Fixed label="Current size" value={`${size} lots`} />
      </div>
      <div className="lr-calc__row">
        <Field label="Last losing size" value={prev} onChange={setPrev} />
        <label className="lr-calc__check">
          <input type="checkbox" checked={recent} onChange={(e) => setRecent(e.target.checked)} />
          <span>Last loss was in this session</span>
        </label>
      </div>
      {res && (
        <div className={`lr-calc__verdict lr-calc__verdict--${res.verdict}`}>
          <strong>
            {res.verdict === "danger" ? "⚠ Martingale pattern detected" :
             res.verdict === "warn" ? "⚠ Size increased after loss" :
             "✓ Not a Martingale pattern"}
          </strong>
          <p>{res.message}</p>
        </div>
      )}
    </div>
  );
}

function BreakEvenCalc({ vpu, entry, size }: { vpu: string; entry: string; size: string }) {
  const [side, setSide] = useState("long");
  const [locked, setLocked] = useState("");
  const [remaining, setRemaining] = useState(size);
  const [res, setRes] = useState<{ be_price: string; move_from_entry: string } | null>(null);

  useEffect(() => {
    (async () => {
      if (!entry || !locked || !remaining) { setRes(null); return; }
      try {
        setRes(await calcBreakEven({
          entry_price: entry, side, locked_profit: locked,
          remaining_size: remaining, value_per_price_unit: vpu,
        }));
      } catch { setRes(null); }
    })();
  }, [entry, side, locked, remaining, vpu]);

  return (
    <div className="lr-calc">
      <div className="lr-calc__fixed">
        <Fixed label="Entry" value={entry} />
        <Fixed label="Value/unit" value={vpu} />
        <Fixed label="Total size" value={`${size} lots`} />
      </div>
      <div className="lr-calc__row">
        <label className="lr-calc__field">
          <span>Side</span>
          <select value={side} onChange={(e) => setSide(e.target.value)}>
            <option value="long">Long</option>
            <option value="short">Short</option>
          </select>
        </label>
        <Field label="Locked profit ($)" value={locked} onChange={setLocked} />
        <Field label="Remaining size" value={remaining} onChange={setRemaining} />
      </div>
      {res && (
        <div className="lr-calc__out">
          <Out label="Move stop to" value={res.be_price} tone="accent" />
          <Out label="Distance from entry" value={res.move_from_entry} />
        </div>
      )}
    </div>
  );
}

function SizeFromRiskCalc({ vpu, balance, entry, stop }: {
  vpu: string; balance: string; entry: string; stop: string;
}) {
  const [riskPct, setRiskPct] = useState("1");
  const [res, setRes] = useState<{ risk_amount: string; size_lots: string; stop_distance: string } | null>(null);

  useEffect(() => {
    (async () => {
      if (!balance || !riskPct || !entry || !stop) { setRes(null); return; }
      try {
        setRes(await calcSizeFromRisk({
          balance, risk_pct: riskPct, entry_price: entry, stop_price: stop,
          value_per_price_unit: vpu,
        }));
      } catch { setRes(null); }
    })();
  }, [balance, riskPct, entry, stop, vpu]);

  return (
    <div className="lr-calc">
      <div className="lr-calc__fixed">
        <Fixed label="Balance" value={balance} />
        <Fixed label="Entry" value={entry} />
        <Fixed label="Stop" value={stop} />
        <Fixed label="Value/unit" value={vpu} />
      </div>
      <div className="lr-calc__row">
        <Field label="Risk %" value={riskPct} onChange={setRiskPct} />
      </div>
      {res && (
        <div className="lr-calc__out">
          <Out label="Risk amount" value={`${res.risk_amount} USD`} tone="accent" />
          <Out label="Suggested size" value={`${res.size_lots} lots`} tone="accent" />
          <Out label="Stop distance" value={res.stop_distance} />
        </div>
      )}
    </div>
  );
}

interface PortfolioRow {
  id: number;
  symbol: string;
  side: "long" | "short";
  size: string;
  entry: string;
  stop: string;
  vpu: string;
}

function PortfolioAssistant({ balance, valuePerUnit }: { balance: string; valuePerUnit: string }) {
  const [rows, setRows] = useState<PortfolioRow[]>([
    { id: 1, symbol: "EURUSD", side: "long", size: "0.5", entry: "", stop: "", vpu: valuePerUnit },
  ]);
  const [result, setResult] = useState<PortfolioRiskResult | null>(null);
  const [maxPortfolio, setMaxPortfolio] = useState("5");
  const [maxPositions, setMaxPositions] = useState(3);

  function updateRow(id: number, patch: Partial<PortfolioRow>) {
    setRows((rs) => rs.map((r) => r.id === id ? { ...r, ...patch } : r));
  }
  function addRow() {
    setRows((rs) => [
      ...rs,
      { id: Date.now(), symbol: "GBPUSD", side: "long", size: "0.5", entry: "", stop: "", vpu: valuePerUnit },
    ]);
  }
  function removeRow(id: number) {
    setRows((rs) => rs.filter((r) => r.id !== id));
  }

  useEffect(() => {
    (async () => {
      const valid = rows.filter((r) => r.entry && r.stop && r.size);
      if (valid.length === 0) { setResult(null); return; }
      try {
        const trades: PortfolioTradeInput[] = valid.map((r) => ({
          symbol: r.symbol,
          side: r.side,
          size: r.size,
          entry_price: r.entry,
          stop_price: r.stop,
          value_per_price_unit: r.vpu,
        }));
        setResult(await getPortfolioRisk({
          trades, balance,
          max_portfolio_risk_pct: maxPortfolio,
          max_open_positions: maxPositions,
        }));
      } catch { setResult(null); }
    })();
  }, [rows, balance, maxPortfolio, maxPositions]);

  const totalBalance = Number(balance) || 0;
  const totalRisk = result ? Number(result.total_risk) : 0;
  const maxPortfolioNum = Number(maxPortfolio) || 5;
  const maxRiskDollar = totalBalance * (maxPortfolioNum / 100);

  const perTradePcts = result?.per_trade.map((t) => ({
    symbol: t.symbol,
    risk: Number(t.risk),
    pct: totalRisk > 0 ? (Number(t.risk) / totalRisk) * 100 : 0,
  })) ?? [];

  return (
    <div className="mp">
      <div className="card mp-header">
        <div>
          <h2>Multi-Position Monitor</h2>
          <p className="dim">Add every open position to see portfolio risk and advice.</p>
        </div>
        <div className="mp-limits">
          <label>
            <span>Portfolio cap %</span>
            <input value={maxPortfolio} onChange={(e) => setMaxPortfolio(e.target.value)} />
          </label>
          <label>
            <span>Max positions</span>
            <input type="number" min={1} max={50} value={maxPositions} onChange={(e) => setMaxPositions(Number(e.target.value))} />
          </label>
        </div>
      </div>

      {result && (
        <div className={`mp-verdict mp-verdict--${result.verdict}`}>
          <div className="mp-verdict__left">
            <span className="mp-verdict__label">Portfolio verdict</span>
            <span className="mp-verdict__status">
              {result.verdict === "danger" ? "Reduce exposure" :
               result.verdict === "warn" ? "Approaching limits" :
               "Within limits"}
            </span>
          </div>
          <div className="mp-verdict__numbers">
            <div>
              <span className="mp-verdict__num-label">Total risk</span>
              <span className="mp-verdict__num-value mono">${result.total_risk}</span>
            </div>
            <div>
              <span className="mp-verdict__num-label">Risk %</span>
              <span className="mp-verdict__num-value mono">{result.total_risk_pct}%</span>
            </div>
            <div>
              <span className="mp-verdict__num-label">Positions</span>
              <span className="mp-verdict__num-value mono">{result.positions}</span>
            </div>
            <div>
              <span className="mp-verdict__num-label">Budget</span>
              <span className="mp-verdict__num-value mono">${maxRiskDollar.toFixed(0)}</span>
            </div>
          </div>
        </div>
      )}

      <div className="mp-grid">
        <div className="mp-left">
          <div className="card mp-positions">
            <div className="mp-positions__head">
              <h3>Open positions</h3>
              <button className="btn btn--ghost btn--small" onClick={addRow}>+ Add</button>
            </div>
            <div className="mp-rows">
              {rows.map((r, idx) => {
                const perTrade = result?.per_trade[idx];
                const riskNum = perTrade ? Number(perTrade.risk) : 0;
                return (
                  <div key={r.id} className="mp-row">
                    <div className="mp-row__idx">{idx + 1}</div>
                    <div className="mp-row__grid">
                      <input value={r.symbol} onChange={(e) => updateRow(r.id, { symbol: e.target.value })} placeholder="Symbol" />
                      <select value={r.side} onChange={(e) => updateRow(r.id, { side: e.target.value as "long" | "short" })}>
                        <option value="long">Long</option>
                        <option value="short">Short</option>
                      </select>
                      <input value={r.size} onChange={(e) => updateRow(r.id, { size: e.target.value })} placeholder="Size" />
                      <input value={r.entry} onChange={(e) => updateRow(r.id, { entry: e.target.value })} placeholder="Entry" />
                      <input value={r.stop} onChange={(e) => updateRow(r.id, { stop: e.target.value })} placeholder="Stop" />
                      <input value={r.vpu} onChange={(e) => updateRow(r.id, { vpu: e.target.value })} placeholder="$/unit" />
                    </div>
                    <div className="mp-row__risk">
                      {riskNum > 0 && (
                        <>
                          <span className="mp-row__risk-value mono">${perTrade?.risk}</span>
                          <span className="mp-row__risk-pct">{totalRisk > 0 ? ((riskNum / totalRisk) * 100).toFixed(0) : 0}%</span>
                        </>
                      )}
                    </div>
                    <button className="icon-btn icon-btn--danger" onClick={() => removeRow(r.id)} title="Remove">×</button>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        <div className="mp-right">
          {result && perTradePcts.length > 0 && (
            <div className="card mp-chart">
              <div className="mp-chart__head">
                <h3>Risk distribution</h3>
                <span className="dim">${result.total_risk} across {result.positions} positions</span>
              </div>
              <div className="mp-chart__bar">
                {perTradePcts.map((t, i) => {
                  const colors = ["var(--accent)", "#60a5fa", "#a78bfa", "#fbbf24", "#f472b6", "#34d399"];
                  return (
                    <div
                      key={i}
                      className="mp-chart__segment"
                      style={{ width: `${t.pct}%`, background: colors[i % colors.length] }}
                      title={`${t.symbol}: $${t.risk} (${t.pct.toFixed(1)}%)`}
                    />
                  );
                })}
              </div>
              <div className="mp-chart__legend">
                {perTradePcts.map((t, i) => {
                  const colors = ["var(--accent)", "#60a5fa", "#a78bfa", "#fbbf24", "#f472b6", "#34d399"];
                  return (
                    <div key={i} className="mp-chart__legend-item">
                      <span className="mp-chart__dot" style={{ background: colors[i % colors.length] }} />
                      <span>{t.symbol}</span>
                      <span className="mono dim">${t.risk}</span>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {result && (
            <div className={`card mp-assistant mp-assistant--${result.verdict}`}>
              <div className="mp-assistant__head">
                <h3>Portfolio Assistant</h3>
                <span className={`lr-verdict lr-verdict--${result.verdict}`}>
                  {result.verdict === "ok" ? "Safe" : result.verdict === "warn" ? "Caution" : "Danger"}
                </span>
              </div>

              <div className="mp-budget">
                <div className="mp-budget__head">
                  <span>Budget used</span>
                  <span className="mono">{result.total_risk_pct}% / {maxPortfolio}%</span>
                </div>
                <div className="mp-budget__track">
                  <div
                    className="mp-budget__fill"
                    style={{
                      width: `${Math.min(100, (Number(result.total_risk_pct) / maxPortfolioNum) * 100)}%`,
                      background: result.portfolio_limit_hit ? "var(--danger)" : result.verdict === "warn" ? "var(--warning)" : "var(--accent)",
                    }}
                  />
                  <div className="mp-budget__cap" />
                </div>
              </div>

              {result.advices.length > 0 && (
                <ul className="mp-advice">
                  {result.advices.map((a, i) => (
                    <li key={i} className="mp-advice__item">
                      <span className="mp-advice__dot" />
                      <span>{a}</span>
                    </li>
                  ))}
                </ul>
              )}

              {result.suggested_trim_pct && (
                <div className="mp-trim">
                  <div className="mp-trim__label">To respect your cap</div>
                  <div className="mp-trim__value mono">trim {result.suggested_trim_pct}% of total exposure</div>
                </div>
              )}
            </div>
          )}

          <div className="card mp-patterns">
            <h3 className="mp-patterns__title">Professional patterns</h3>
            <div className="mp-patterns__list">
              <Pattern
                name="1% rule"
                desc="Risk no more than 1% of account per trade."
                current={result && result.positions > 0 ? `${(Number(result.total_risk_pct) / result.positions).toFixed(2)}% per position avg` : "—"}
                ok={result ? Number(result.total_risk_pct) / Math.max(1, result.positions) <= 1 : true}
              />
              <Pattern
                name="2% ceiling"
                desc="Total portfolio risk should stay under 2% for conservative sizing."
                current={result ? `${result.total_risk_pct}%` : "—"}
                ok={result ? Number(result.total_risk_pct) <= 2 : true}
              />
              <Pattern
                name="Portfolio heat"
                desc="Open risk across all positions should stay below 5%."
                current={result ? `${result.total_risk_pct}%` : "—"}
                ok={result ? Number(result.total_risk_pct) <= maxPortfolioNum : true}
              />
              <Pattern
                name="Max concurrent"
                desc="Limit simultaneous positions to keep focus."
                current={result ? `${result.positions} / ${maxPositions}` : "—"}
                ok={result ? result.positions <= maxPositions : true}
              />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function Pattern({ name, desc, current, ok }: { name: string; desc: string; current: string; ok: boolean }) {
  return (
    <div className={`mp-pattern ${ok ? "is-ok" : "is-bad"}`}>
      <div className="mp-pattern__head">
        <span className="mp-pattern__name">{name}</span>
        <span className={`mp-pattern__status ${ok ? "ok" : "bad"}`}>{ok ? "✓" : "✕"}</span>
      </div>
      <p className="mp-pattern__desc">{desc}</p>
      <span className="mp-pattern__current mono">{current}</span>
    </div>
  );
}

function Field({ label, value, onChange, placeholder }: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string;
}) {
  return (
    <label className="lr-calc__field">
      <span>{label}</span>
      <input value={value} onChange={(e) => onChange(e.target.value)} placeholder={placeholder} />
    </label>
  );
}

function Fixed({ label, value }: { label: string; value: string }) {
  return (
    <div className="lr-fixed">
      <span className="lr-fixed__label">{label}</span>
      <span className="lr-fixed__value mono">{value || "—"}</span>
    </div>
  );
}

function Out({ label, value, tone }: { label: string; value: string; tone?: "pos" | "accent" }) {
  return (
    <div className={`lr-calc__out-item ${tone ? `is-${tone}` : ""}`}>
      <span className="lr-calc__out-label">{label}</span>
      <span className="lr-calc__out-value mono">{value}</span>
    </div>
  );
}
