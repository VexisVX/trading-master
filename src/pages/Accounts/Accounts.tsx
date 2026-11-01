import { useCallback, useEffect, useState } from "react";
import {
  Account, AccountKind, listAccounts, createAccount,
  deleteAccount, getActiveAccountId, setActiveAccountId,
} from "../../lib/accountsApi";
import {
  getRiskSettings, updateRiskSettings, UpdateRiskSettings,
} from "../../lib/riskSettingsApi";
import {
  getPropFirmRules, updatePropFirmRules, UpdatePropFirmRules,
} from "../../lib/propFirmApi";
import { Select } from "../../components/ui/Select";
import { FieldWithHelp } from "../../components/ui/FieldWithHelp";
import { ConfirmDialog } from "../../components/ui/ConfirmDialog";
import { getSummary } from "../../lib/analyticsApi";
import "./Accounts.css";

export function Accounts() {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [activeId, setActiveId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showNew, setShowNew] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<{ id: number; name: string } | null>(null);
  const [tab, setTab] = useState<"list" | "risk" | "prop">("list");

  const [riskForm, setRiskForm] = useState<UpdateRiskSettings | null>(null);
  const [riskSaving, setRiskSaving] = useState(false);
  const [riskSavedAt, setRiskSavedAt] = useState<string | null>(null);

  const [pfForm, setPfForm] = useState<UpdatePropFirmRules | null>(null);
  const [pfSaving, setPfSaving] = useState(false);
  const [pfSavedAt, setPfSavedAt] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setAccounts(await listAccounts());
      setActiveId(await getActiveAccountId());
    } catch (e) { setError(String(e)); }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  useEffect(() => {
    if (activeId == null) return;
    setRiskForm(null); setPfForm(null);
    (async () => {
      try {
        const rs = await getRiskSettings(activeId);
        if (rs) setRiskForm({
          max_risk_per_trade_pct: rs.max_risk_per_trade_pct,
          daily_loss_limit_pct: rs.daily_loss_limit_pct,
          max_open_positions: rs.max_open_positions,
          max_portfolio_risk_pct: rs.max_portfolio_risk_pct,
        });
        const pfr = await getPropFirmRules(activeId);
        if (pfr) setPfForm({
          enabled: pfr.enabled,
          max_dd_pct: pfr.max_dd_pct,
          trailing_dd: pfr.trailing_dd,
          daily_loss_limit_pct: pfr.daily_loss_limit_pct,
          max_total_loss_pct: pfr.max_total_loss_pct,
          profit_target_pct: pfr.profit_target_pct,
          min_trading_days: pfr.min_trading_days,
        });
      } catch (e) { setError(String(e)); }
    })();
  }, [activeId]);

  async function selectAccount(id: number) {
    try {
      await setActiveAccountId(id);
      setActiveId(id);
      setAccounts((prev) => prev.map((a) => ({ ...a, is_active: a.id === id })));
    } catch (e) { setError(String(e)); }
  }

  function removeAccount(id: number) {
    if (accounts.length <= 1) {
      alert("This is your only account. Create another first.");
      return;
    }
    const acct = accounts.find((a) => a.id === id);
    setConfirmDelete({ id, name: acct?.name ?? `#${id}` });
  }

  async function doDelete() {
    if (!confirmDelete) return;
    try {
      await deleteAccount(confirmDelete.id);
      setConfirmDelete(null);
      await refresh();
    } catch (e) { setError(String(e)); }
  }

  async function saveRisk(e: React.FormEvent) {
    e.preventDefault();
    if (!activeId || !riskForm) return;
    setRiskSaving(true);
    try {
      await updateRiskSettings(activeId, riskForm);
      setRiskSavedAt(new Date().toLocaleTimeString());
    } catch (e) { setError(String(e)); }
    finally { setRiskSaving(false); }
  }

  async function savePf(e: React.FormEvent) {
    e.preventDefault();
    if (!activeId || !pfForm) return;
    setPfSaving(true);
    try {
      await updatePropFirmRules(activeId, pfForm);
      setPfSavedAt(new Date().toLocaleTimeString());
    } catch (e) { setError(String(e)); }
    finally { setPfSaving(false); }
  }

  const active = accounts.find((a) => a.id === activeId);

  return (
    <div className="accounts">
      <div className="accounts__tabs">
        <button className={`acc-tab ${tab === "list" ? "is-active" : ""}`} onClick={() => setTab("list")}>
          Accounts ({accounts.length})
        </button>
        <button className={`acc-tab ${tab === "risk" ? "is-active" : ""}`} onClick={() => setTab("risk")} disabled={!active}>
          Risk · {active?.name ?? "—"}
        </button>
        {active?.kind === "prop_firm" && (
          <button className={`acc-tab ${tab === "prop" ? "is-active" : ""}`} onClick={() => setTab("prop")}>
            Prop-firm rules
          </button>
        )}
        <button className="btn btn--primary btn--small" style={{ marginLeft: "auto" }} onClick={() => setShowNew(true)}>
          + New account
        </button>
      </div>

      {error && <div className="card card--error">{error}</div>}

      {confirmDelete && (
        <ConfirmDialog
          title={`Delete account "${confirmDelete.name}"?`}
          body="All trades, rules and settings for this account will be permanently removed. This cannot be undone."
          confirmLabel="Delete"
          cancelLabel="Keep"
          tone="danger"
          onConfirm={doDelete}
          onCancel={() => setConfirmDelete(null)}
        />
      )}

      {showNew && (
        <div className="card pop-in">
          <NewAccountForm
            onCreated={async () => { setShowNew(false); await refresh(); }}
            onCancel={() => setShowNew(false)}
            setError={setError}
          />
        </div>
      )}

      {tab === "list" && (
        <div className="accounts__grid">
          {accounts.map((a) => (
            <AccountCard
              key={a.id}
              account={a}
              active={a.id === activeId}
              onClick={() => selectAccount(a.id)}
              onDelete={() => removeAccount(a.id)}
            />
          ))}
        </div>
      )}

      {tab === "risk" && activeId && riskForm && (
        <div className="card">
          <h2 className="section-title">Risk settings · {active?.name}</h2>
          <p className="settings__hint">
            Warnings only — does not block trades or enforce broker rules.
          </p>
          <form className="settings-form" onSubmit={saveRisk}>
            <FieldWithHelp label="Max risk per trade (%)" help={{ title: "Risk per trade", body: "Maximum % of equity to risk on one trade." }}>
              <input value={riskForm.max_risk_per_trade_pct} onChange={(e) => setRiskForm({ ...riskForm, max_risk_per_trade_pct: e.target.value })} />
            </FieldWithHelp>
            <FieldWithHelp label="Daily loss limit (%)" help={{ title: "Daily loss", body: "If today's loss reaches this %, show a warning." }}>
              <input value={riskForm.daily_loss_limit_pct} onChange={(e) => setRiskForm({ ...riskForm, daily_loss_limit_pct: e.target.value })} />
            </FieldWithHelp>
            <FieldWithHelp label="Max open positions" help={{ title: "Concurrency", body: "Maximum simultaneous open trades." }}>
              <input type="number" min={1} max={100} value={riskForm.max_open_positions} onChange={(e) => setRiskForm({ ...riskForm, max_open_positions: Number(e.target.value) })} />
            </FieldWithHelp>
            <FieldWithHelp label="Max portfolio risk (%)" help={{ title: "Portfolio risk", body: "Sum of open risk across all positions." }}>
              <input value={riskForm.max_portfolio_risk_pct} onChange={(e) => setRiskForm({ ...riskForm, max_portfolio_risk_pct: e.target.value })} />
            </FieldWithHelp>
            <div className="settings-form__actions">
              {riskSavedAt && <span className="settings__saved">Saved at {riskSavedAt}</span>}
              <button type="submit" className="btn btn--primary" disabled={riskSaving}>
                {riskSaving ? "Saving…" : "Save risk settings"}
              </button>
            </div>
          </form>
        </div>
      )}

      {tab === "prop" && activeId && pfForm && (
        <div className="card">
          <h2 className="section-title">Prop-firm rules · {active?.name}</h2>
          <form className="settings-form" onSubmit={savePf}>
            <label style={{ gridColumn: "1 / -1", flexDirection: "row", alignItems: "center", gap: 8 }}>
              <input type="checkbox" checked={pfForm.enabled} onChange={(e) => setPfForm({ ...pfForm, enabled: e.target.checked })} />
              <span style={{ textTransform: "none", fontSize: 13 }}>Enable prop-firm evaluation</span>
            </label>
            <FieldWithHelp label="Max drawdown (%)" help={{ title: "Max DD", body: "Largest decline from peak equity." }}>
              <input value={pfForm.max_dd_pct} onChange={(e) => setPfForm({ ...pfForm, max_dd_pct: e.target.value })} />
            </FieldWithHelp>
            <label style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
              <input type="checkbox" checked={pfForm.trailing_dd} onChange={(e) => setPfForm({ ...pfForm, trailing_dd: e.target.checked })} />
              <span style={{ textTransform: "none", fontSize: 13 }}>Trailing</span>
            </label>
            <FieldWithHelp label="Daily loss limit (%)" help={{ title: "Daily loss", body: "Max loss per UTC day." }}>
              <input value={pfForm.daily_loss_limit_pct} onChange={(e) => setPfForm({ ...pfForm, daily_loss_limit_pct: e.target.value })} />
            </FieldWithHelp>
            <FieldWithHelp label="Max total loss (%)" help={{ title: "Total loss", body: "Cumulative loss from initial balance." }}>
              <input value={pfForm.max_total_loss_pct} onChange={(e) => setPfForm({ ...pfForm, max_total_loss_pct: e.target.value })} />
            </FieldWithHelp>
            <FieldWithHelp label="Profit target (%)" help={{ title: "Target", body: "Gain needed to pass challenge." }}>
              <input value={pfForm.profit_target_pct} onChange={(e) => setPfForm({ ...pfForm, profit_target_pct: e.target.value })} />
            </FieldWithHelp>
            <FieldWithHelp label="Min trading days" help={{ title: "Min days", body: "Distinct days with at least one closed trade." }}>
              <input type="number" min={0} max={365} value={pfForm.min_trading_days} onChange={(e) => setPfForm({ ...pfForm, min_trading_days: Number(e.target.value) })} />
            </FieldWithHelp>
            <div className="settings-form__actions">
              {pfSavedAt && <span className="settings__saved">Saved at {pfSavedAt}</span>}
              <button type="submit" className="btn btn--primary" disabled={pfSaving}>
                {pfSaving ? "Saving…" : "Save prop-firm rules"}
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
}

function AccountCard({ account, active, onClick, onDelete }: {
  account: Account;
  active: boolean;
  onClick: () => void;
  onDelete: () => void;
}) {
  const [stats, setStats] = useState<{ pnl: string; wins: number; losses: number } | null>(null);

  useEffect(() => {
    // Load summary only for active account to avoid hitting rate limits
    if (!active) return;
    getSummary().then((s) => {
      setStats({ pnl: s.net_pnl, wins: s.wins, losses: s.losses });
    }).catch(() => {});
  }, [active]);

  return (
    <div className={`acc-card ${active ? "is-active" : ""}`} onClick={onClick}>
      <div className="acc-card__head">
        <span className="acc-card__name">{account.name}</span>
        <span className={`acc-kind acc-kind--${account.kind}`}>
          {account.kind === "prop_firm" ? "Prop-firm" : "Personal"}
        </span>
      </div>
      <div className="acc-card__meta">
        <span className="mono">{account.currency}</span>
        <span className="dim">·</span>
        <span className="mono">{Number(account.initial_balance).toLocaleString()}</span>
      </div>
      {stats && active && (
        <div className="acc-card__stats">
          <div>
            <span className="label">P&L</span>
            <span className={`value mono ${Number(stats.pnl) >= 0 ? "pos" : "neg"}`}>
              {Number(stats.pnl) >= 0 ? "+" : ""}{Number(stats.pnl).toFixed(2)}
            </span>
          </div>
          <div>
            <span className="label">W/L</span>
            <span className="value mono">{stats.wins}/{stats.losses}</span>
          </div>
        </div>
      )}
      <div className="acc-card__actions">
        <button
          className="btn btn--small btn--danger"
          onClick={(e) => { e.stopPropagation(); onDelete(); }}
        >
          Delete
        </button>
      </div>
    </div>
  );
}

function NewAccountForm({ onCreated, onCancel, setError }: {
  onCreated: () => void;
  onCancel: () => void;
  setError: (s: string | null) => void;
}) {
  const [name, setName] = useState("");
  const [currency, setCurrency] = useState("USD");
  const [balance, setBalance] = useState("10000");
  const [kind, setKind] = useState<AccountKind>("personal");
  const [busy, setBusy] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    if (!name.trim()) { setError("Name is required."); return; }
    if (Number(balance) <= 0) { setError("Balance > 0."); return; }
    setBusy(true);
    try {
      await createAccount({ name: name.trim(), currency, initial_balance: balance, kind });
      onCreated();
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }

  return (
    <form className="settings-form" onSubmit={submit}>
      <h2 className="section-title" style={{ gridColumn: "1 / -1" }}>New account</h2>
      <label><span>Name</span><input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. FTMO 100k" /></label>
      <label>
        <span>Kind</span>
        <Select value={kind} onChange={(v) => setKind(v as AccountKind)}
          options={[
            { value: "personal", label: "Personal" },
            { value: "prop_firm", label: "Prop-firm" },
          ]}
        />
      </label>
      <label><span>Currency</span><input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} maxLength={4} /></label>
      <label><span>Initial balance</span><input value={balance} onChange={(e) => setBalance(e.target.value)} /></label>
      <div className="settings-form__actions" style={{ gridColumn: "1 / -1" }}>
        <button type="button" className="btn btn--ghost" onClick={onCancel}>Cancel</button>
        <button type="submit" className="btn btn--primary" disabled={busy}>
          {busy ? "Creating…" : "Create account"}
        </button>
      </div>
    </form>
  );
}
