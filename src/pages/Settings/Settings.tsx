import { useEffect, useState } from "react";
import {
  backupNow, backupToFileInteractive, restoreInteractive, getBackupDir, BackupResult,
} from "../../lib/backupApi";
import { getSetting, setSetting } from "../../lib/settingsApi";
import { testKey } from "../../lib/newsApi";
import { getStorageStats, StorageStats } from "../../lib/storageApi";
import {
  applyTheme, applyAccent, applyRadius, applyFontSize, applyMotion,
  ACCENTS, Theme, AccentId, RadiusId, DateFormat,
} from "../../lib/theme";
import { openUrl } from "@tauri-apps/plugin-opener";
import "./Settings.css";

type Tab = "appearance" | "behavior" | "data" | "integrations" | "about";

export function Settings() {
  const [tab, setTab] = useState<Tab>("appearance");
  const [error, setError] = useState<string | null>(null);
  const [savedMsg, setSavedMsg] = useState<string | null>(null);

  // Appearance
  const [theme, setTheme] = useState<Theme>("dark");
  const [accent, setAccent] = useState<AccentId>("turquoise");
  const [radius, setRadius] = useState<RadiusId>("soft");
  const [fontSize, setFontSize] = useState<"small" | "medium" | "large">("medium");
  const [motion, setMotion] = useState<"full" | "reduced" | "off">("full");
  const [dateFormat, setDateFormat] = useState<DateFormat>("iso");
  const [timeFormat, setTimeFormat] = useState<"24h" | "12h">("24h");
  const [weekStart, setWeekStart] = useState<"mon" | "sun">("mon");
  const [confirmDelete, setConfirmDelete] = useState(true);
  const [showPnlColors, setShowPnlColors] = useState(true);
  const [defaultAccount, setDefaultAccount] = useState("");
  const [accountsList, setAccountsList] = useState<{ id: number; name: string }[]>([]);

  // Backup
  const [backupDir, setBackupDir] = useState("");
  const [lastBackup, setLastBackup] = useState<BackupResult | null>(null);
  const [backupBusy, setBackupBusy] = useState(false);

  // Integrations
  const [marketauxKey, setMarketauxKey] = useState("");
  const [fmpKey, setFmpKey] = useState("");
  const [finnhubKey, setFinnhubKey] = useState("");
  const [keyTesting, setKeyTesting] = useState(false);
  const [keyTestMsg, setKeyTestMsg] = useState<string | null>(null);

  // Storage
  const [storage, setStorage] = useState<StorageStats | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const t = (await getSetting("ui.theme")) as Theme || "dark";
        const a = (await getSetting("ui.accent")) as AccentId || "turquoise";
        const r = (await getSetting("ui.radius")) as RadiusId || "soft";
        const fs = await getSetting("ui.font_size") || "medium";
        const m = await getSetting("ui.motion") || "full";
        const df = (await getSetting("ui.date_format")) as DateFormat || "iso";

        setTheme(t);
        setAccent(a);
        setRadius(r);
        setFontSize(fs as any);
        setMotion(m as any);
        setDateFormat(df);

        // Load behavior settings
        const tf = await getSetting("ui.time_format") || "24h";
        const ws = await getSetting("ui.week_start") || "mon";
        const cd = await getSetting("behavior.confirm_delete") || "1";
        const pc = await getSetting("behavior.pnl_colors") || "1";
        const da = await getSetting("behavior.default_account") || "";
        setTimeFormat(tf as any);
        setWeekStart(ws as any);
        setConfirmDelete(cd === "1");
        setShowPnlColors(pc === "1");
        setDefaultAccount(da);

        // Load accounts list
        try {
          const { listAccounts } = await import("../../lib/accountsApi");
          const accs = await listAccounts();
          setAccountsList(accs.map((a) => ({ id: a.id, name: a.name })));
        } catch { /* ignore */ }

        setMarketauxKey((await getSetting("marketaux_api_key")) || "");
        setFmpKey((await getSetting("fmp_api_key")) || "");
        setFinnhubKey((await getSetting("finnhub_api_key")) || "");
        setBackupDir(await getBackupDir());

        // Apply on load
        applyTheme(t);
        applyAccent(a);
        applyRadius(r);
        applyFontSize(fs as any);
        applyMotion(m as any);
      } catch (e) { setError(String(e)); }
    })();
  }, []);

  useEffect(() => {
    if (tab === "data") {
      getStorageStats().then(setStorage).catch(() => {});
    }
  }, [tab]);

  function flash(msg: string) {
    setSavedMsg(msg);
    setTimeout(() => setSavedMsg(null), 2000);
  }

  function setThemeAndApply(t: Theme) {
    setTheme(t);
    applyTheme(t);
    setSetting("ui.theme", t).catch(() => {});
  }

  function setAccentAndApply(a: AccentId) {
    setAccent(a);
    applyAccent(a);
    setSetting("ui.accent", a).catch(() => {});
  }

  function setRadiusAndApply(r: RadiusId) {
    setRadius(r);
    applyRadius(r);
    setSetting("ui.radius", r).catch(() => {});
  }

  function setFontSizeAndApply(s: "small" | "medium" | "large") {
    setFontSize(s);
    applyFontSize(s);
    setSetting("ui.font_size", s).catch(() => {});
  }

  function setMotionAndApply(m: "full" | "reduced" | "off") {
    setMotion(m);
    applyMotion(m);
    setSetting("ui.motion", m).catch(() => {});
  }

  function setTimeFormatAndSave(v: "24h" | "12h") {
    setTimeFormat(v);
    setSetting("ui.time_format", v).catch(() => {});
  }

  function setWeekStartAndSave(v: "mon" | "sun") {
    setWeekStart(v);
    setSetting("ui.week_start", v).catch(() => {});
  }

  function setConfirmDeleteAndSave(v: boolean) {
    setConfirmDelete(v);
    setSetting("behavior.confirm_delete", v ? "1" : "0").catch(() => {});
  }

  function setShowPnlColorsAndSave(v: boolean) {
    setShowPnlColors(v);
    setSetting("behavior.pnl_colors", v ? "1" : "0").catch(() => {});
  }

  function setDefaultAccountAndSave(v: string) {
    setDefaultAccount(v);
    setSetting("behavior.default_account", v).catch(() => {});
  }

  function setDateFormatAndSave(df: DateFormat) {
    setDateFormat(df);
    setSetting("ui.date_format", df).catch(() => {});
  }

  async function saveMarketauxKey() {
    await setSetting("marketaux_api_key", marketauxKey.trim());
    flash("Marketaux key saved");
  }

  async function saveFinnhubKey() {
    await setSetting("finnhub_api_key", finnhubKey.trim());
    flash("Finnhub key saved");
  }

  async function saveFmpKey() {
    await setSetting("fmp_api_key", fmpKey.trim());
    flash("FMP key saved");
  }

  async function testMarketaux() {
    setKeyTesting(true);
    setKeyTestMsg(null);
    try {
      await testKey();
      setKeyTestMsg("✓ Key works");
    } catch (e) {
      setKeyTestMsg("✗ " + String(e));
    } finally { setKeyTesting(false); }
  }

  async function doBackupNow() {
    setBackupBusy(true);
    try {
      setLastBackup(await backupNow());
      flash("Backup created");
    } catch (e) { setError(String(e)); }
    finally { setBackupBusy(false); }
  }

  async function doBackupToFile() {
    setBackupBusy(true);
    try {
      const r = await backupToFileInteractive();
      if (r) setLastBackup(r);
    } catch (e) { setError(String(e)); }
    finally { setBackupBusy(false); }
  }

  async function doRestore() {
    try {
      const path = await restoreInteractive();
      if (path) alert("Backup restored. Restart the app for changes to take effect.");
    } catch (e) { setError(String(e)); }
  }

  function formatBytes(b: number): string {
    if (b < 1024) return `${b} B`;
    if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
    if (b < 1024 * 1024 * 1024) return `${(b / 1024 / 1024).toFixed(2)} MB`;
    return `${(b / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  return (
    <div className="settings">
      <div className="settings-tabs">
        {([
          ["appearance", "Appearance"],
          ["behavior", "Behavior"],
          ["data", "Data"],
          ["integrations", "Integrations"],
          ["about", "About"],
        ] as const).map(([id, label]) => (
          <button key={id} className={`settings-tab ${tab === id ? "is-active" : ""}`} onClick={() => setTab(id as Tab)}>
            {label}
          </button>
        ))}
      </div>

      {error && <div className="card card--error">{error}</div>}
      {savedMsg && <div className="card settings-toast">{savedMsg}</div>}

      {tab === "appearance" && (
        <>
          <div className="card">
            <h2 className="section-title">Theme</h2>
            <div className="theme-grid">
              {([
                ["dark", "Dark", "#0b0f14", "#e6edf3"],
                ["light", "Light", "#f8fafc", "#0f172a"],
                ["cream", "Cream", "#f5f0e6", "#3a2f20"],
                ["dim", "Dim", "#1a1f26", "#e6edf3"],
                ["amoled", "AMOLED", "#000000", "#ffffff"],
              ] as const).map(([id, label, bg, fg]) => (
                <button
                  key={id}
                  className={`theme-preview ${theme === id ? "is-active" : ""}`}
                  onClick={() => setThemeAndApply(id as Theme)}
                >
                  <div className="theme-preview__dots" style={{ background: bg }}>
                    <span style={{ background: fg }}></span>
                    <span style={{ background: fg, opacity: 0.5 }}></span>
                    <span style={{ background: fg, opacity: 0.25 }}></span>
                  </div>
                  <span className="theme-preview__label">{label}</span>
                </button>
              ))}
            </div>
          </div>

          <div className="card">
            <h2 className="section-title">Accent color</h2>
            <div className="accent-grid">
              {ACCENTS.map((a) => (
                <button
                  key={a.id}
                  className={`accent-dot ${accent === a.id ? "is-active" : ""}`}
                  onClick={() => setAccentAndApply(a.id)}
                  title={a.label}
                  aria-label={a.label}
                  style={{ background: a.color }}
                />
              ))}
            </div>
          </div>

          <div className="card">
            <h2 className="section-title">Interface</h2>

            <div className="app-row">
              <div className="app-row__label">
                <div>Border radius</div>
                <div className="dim">Roundedness of cards, inputs, buttons</div>
              </div>
              <div className="seg-group">
                {(["square", "soft", "round"] as const).map((r) => (
                  <button key={r} className={`seg-btn ${radius === r ? "is-active" : ""}`} onClick={() => setRadiusAndApply(r)}>
                    {r}
                  </button>
                ))}
              </div>
            </div>

            <div className="app-row">
              <div className="app-row__label">
                <div>Font size</div>
                <div className="dim">Overall text scaling</div>
              </div>
              <div className="seg-group">
                {(["small", "medium", "large"] as const).map((s) => (
                  <button key={s} className={`seg-btn ${fontSize === s ? "is-active" : ""}`} onClick={() => setFontSizeAndApply(s)}>
                    {s}
                  </button>
                ))}
              </div>
            </div>

            <div className="app-row">
              <div className="app-row__label">
                <div>Motion</div>
                <div className="dim">Animation speed across the app</div>
              </div>
              <div className="seg-group">
                {(["full", "reduced", "off"] as const).map((m) => (
                  <button key={m} className={`seg-btn ${motion === m ? "is-active" : ""}`} onClick={() => setMotionAndApply(m)}>
                    {m}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </>
      )}

      {tab === "behavior" && (
        <div className="card">
          <h2 className="section-title">Behavior</h2>

          <div className="app-row">
            <div className="app-row__label">
              <div>Date format</div>
              <div className="dim">Used in trade lists, timestamps, exports</div>
            </div>
            <div className="seg-group">
              {([
                ["iso", "2026-10-22"],
                ["dmy", "22/10/2026"],
                ["mdy", "10/22/2026"],
              ] as const).map(([id, preview]) => (
                <button key={id} className={`seg-btn ${dateFormat === id ? "is-active" : ""}`} onClick={() => setDateFormatAndSave(id as DateFormat)}>
                  {preview}
                </button>
              ))}
            </div>
          </div>

          <div className="app-row">
            <div className="app-row__label">
              <div>Time format</div>
              <div className="dim">24-hour or 12-hour clock</div>
            </div>
            <div className="seg-group">
              {([
                ["24h", "24h"],
                ["12h", "12h AM/PM"],
              ] as const).map(([id, label]) => (
                <button key={id} className={`seg-btn ${timeFormat === id ? "is-active" : ""}`} onClick={() => setTimeFormatAndSave(id as any)}>
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="app-row">
            <div className="app-row__label">
              <div>Week starts on</div>
              <div className="dim">First day in calendars and analytics</div>
            </div>
            <div className="seg-group">
              {([
                ["mon", "Monday"],
                ["sun", "Sunday"],
              ] as const).map(([id, label]) => (
                <button key={id} className={`seg-btn ${weekStart === id ? "is-active" : ""}`} onClick={() => setWeekStartAndSave(id as any)}>
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="app-row">
            <div className="app-row__label">
              <div>Confirm before delete</div>
              <div className="dim">Show a confirm dialog for destructive actions</div>
            </div>
            <button
              className={`toggle ${confirmDelete ? "is-on" : ""}`}
              onClick={() => setConfirmDeleteAndSave(!confirmDelete)}
            >
              <span className="toggle__knob" />
            </button>
          </div>

          <div className="app-row">
            <div className="app-row__label">
              <div>Show P&L color coding</div>
              <div className="dim">Green for positive, red for negative</div>
            </div>
            <button
              className={`toggle ${showPnlColors ? "is-on" : ""}`}
              onClick={() => setShowPnlColorsAndSave(!showPnlColors)}
            >
              <span className="toggle__knob" />
            </button>
          </div>

          <div className="app-row">
            <div className="app-row__label">
              <div>Default account</div>
              <div className="dim">The account opened when the app starts</div>
            </div>
            <select
              className="settings-select"
              value={defaultAccount}
              onChange={(e) => setDefaultAccountAndSave(e.target.value)}
            >
              <option value="">— active account —</option>
              {accountsList.map((a) => (
                <option key={a.id} value={String(a.id)}>{a.name}</option>
              ))}
            </select>
          </div>
        </div>
      )}

      {tab === "data" && (
        <>
          <div className="card">
            <h2 className="section-title">Storage</h2>
            {storage ? (
              <div className="storage-grid">
                <div className="storage-item">
                  <div className="label">Database</div>
                  <div className="value mono">{formatBytes(storage.db_size_bytes)}</div>
                </div>
                <div className="storage-item">
                  <div className="label">Attachments</div>
                  <div className="value mono">{formatBytes(storage.attachments_bytes)}</div>
                  <div className="sub dim">{storage.attachments_count} files</div>
                </div>
                <div className="storage-item">
                  <div className="label">Trades</div>
                  <div className="value mono">{storage.trades_count}</div>
                </div>
                <div className="storage-item">
                  <div className="label">Strategies</div>
                  <div className="value mono">{storage.strategies_count}</div>
                </div>
                <div className="storage-item">
                  <div className="label">Accounts</div>
                  <div className="value mono">{storage.accounts_count}</div>
                </div>
              </div>
            ) : (
              <p className="dim">Loading…</p>
            )}
          </div>

          <div className="card">
            <h2 className="section-title">Backup & Restore</h2>
            <p className="settings__hint">Plain SQLite files. Keep copies outside the app folder.</p>
            <div className="backup-row">
              <button className="btn btn--primary" onClick={doBackupNow} disabled={backupBusy}>
                {backupBusy ? "…" : "Backup now"}
              </button>
              <button className="btn btn--ghost" onClick={doBackupToFile} disabled={backupBusy}>
                Save to file…
              </button>
              <button className="btn btn--danger" onClick={doRestore}>
                Restore from file…
              </button>
            </div>
            {backupDir && <p className="settings__meta">Auto-backup folder: <span className="mono">{backupDir}</span></p>}
            {lastBackup && (
              <p className="settings__meta">
                Last: <span className="mono">{lastBackup.path}</span> · {formatBytes(lastBackup.size_bytes)}
              </p>
            )}
          </div>
        </>
      )}

      {tab === "integrations" && (
        <div className="card">
          <h2 className="section-title">Marketaux API</h2>
          <p className="settings__hint">
            Free tier: 100 requests/day.{" "}
            <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://www.marketaux.com"); }}>Get your key →</a>
          </p>
          <div className="integrations-form">
            <label>
              <span>API Token</span>
              <input type="password" value={marketauxKey} onChange={(e) => setMarketauxKey(e.target.value)} placeholder="paste token" />
            </label>
            <div className="integrations-actions">
              <button className="btn btn--primary btn--small" onClick={saveMarketauxKey} disabled={!marketauxKey.trim()}>Save</button>
              <button className="btn btn--ghost btn--small" onClick={testMarketaux} disabled={keyTesting || !marketauxKey.trim()}>
                {keyTesting ? "Testing…" : "Test"}
              </button>
            </div>
            {keyTestMsg && <p className={keyTestMsg.startsWith("✓") ? "test-ok" : "test-fail"}>{keyTestMsg}</p>}
          </div>

          <h2 className="section-title" style={{ marginTop: 24 }}>Finnhub (News)</h2>
          <p className="settings__hint">
            Free tier: 60 requests/minute. Provides news with images and summaries.{" "}
            <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://finnhub.io"); }}>Get your key →</a>
          </p>
          <div className="integrations-form">
            <label>
              <span>Finnhub API Key</span>
              <input type="password" value={finnhubKey} onChange={(e) => setFinnhubKey(e.target.value)} placeholder="paste key" />
            </label>
            <div className="integrations-actions">
              <button className="btn btn--primary btn--small" onClick={saveFinnhubKey} disabled={!finnhubKey.trim()}>Save</button>
            </div>
          </div>

          <h2 className="section-title" style={{ marginTop: 24 }}>FMP (Financial Modeling Prep)</h2>
          <p className="settings__hint">
            Free tier: 250 requests/day.{" "}
            <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://financialmodelingprep.com"); }}>Get your key →</a>
          </p>
          <div className="integrations-form">
            <label>
              <span>FMP API Key</span>
              <input type="password" value={fmpKey} onChange={(e) => setFmpKey(e.target.value)} placeholder="paste key" />
            </label>
            <div className="integrations-actions">
              <button className="btn btn--primary btn--small" onClick={saveFmpKey} disabled={!fmpKey.trim()}>Save</button>
            </div>
          </div>
        </div>
      )}

      {tab === "about" && (
        <div className="card">
          <h2 className="section-title">About</h2>
          <div className="about-block">
            <div><strong>Trading Manager</strong></div>
            <div className="dim">Version 0.1.0 · local-first · offline</div>
            <p className="settings__hint" style={{ marginTop: 12 }}>
              A professional trading journal, risk calculator and analytics tool. All data is stored locally.
            </p>
            <div className="about-links">
              <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://www.marketaux.com"); }}>Marketaux</a>
              <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://financialmodelingprep.com"); }}>FMP</a>
              <a href="#" onClick={(e) => { e.preventDefault(); openUrl("https://www.forexfactory.com"); }}>ForexFactory</a>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
