import { useCallback, useEffect, useState } from "react";
import {
  AccountInfo,
  autoSnapshotFromTrades,
  deriveStatus,
  getLatestSnapshot,
  listAccounts,
  listDecisions,
  previewRisk,
  DecisionOut,
  DecisionRowOut,
  EquitySnapshotRow,
  RiskPreview,
} from "../../lib/equityApi";
import { AccountStatusCard } from "./AccountStatusCard";
import { RiskIntelligenceCard } from "./RiskIntelligenceCard";
import { CapitalProtectionCard } from "./CapitalProtectionCard";
import { NewTradeCard } from "./NewTradeCard";
import { RecentDecisionsCard } from "./RecentDecisionsCard";
import { InstrumentsCard } from "./InstrumentsCard";
import { WhyThisSizeModal } from "./WhyThisSizeModal";
import { SettingsModal } from "./SettingsModal";
import { AllDecisionsModal } from "./AllDecisionsModal";
import { SnapshotsModal } from "./SnapshotsModal";
import "./EquityEngine.css";

import { getActiveAccountId } from "../../lib/accountsApi";
const LS_KEY = "equity_selected_account_id";

export function EquityEngine() {
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [selectedAccountId, setSelectedAccountId] = useState<number | null>(null);
  const [snapshot, setSnapshot] = useState<EquitySnapshotRow | null>(null);
  const [decisions, setDecisions] = useState<DecisionRowOut[]>([]);
  const [riskPreview, setRiskPreview] = useState<RiskPreview | null>(null);
  const [loadingPreview, setLoadingPreview] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // modals
  const [showSettings, setShowSettings] = useState(false);
  const [showAll, setShowAll] = useState(false);
  const [showSnapshots, setShowSnapshots] = useState(false);
  const [whyResult, setWhyResult] = useState<DecisionOut | null>(null);

  const loadAll = useCallback(async (accountId?: number) => {
    try {
      const [snap, dec] = await Promise.all([
        getLatestSnapshot(accountId),
        listDecisions(200),
      ]);
      setSnapshot(snap);
      setDecisions(dec);
      setError(null);
    } catch (e) {
      setError(String(e));
    }

    setLoadingPreview(true);
    try {
      const preview = await previewRisk(accountId);
      setRiskPreview(preview);
    } catch {
      setRiskPreview(null);
    } finally {
      setLoadingPreview(false);
    }
  }, []);

  useEffect(() => {
    (async () => {
      setLoading(true);
      try {
        const accs = await listAccounts();
        setAccounts(accs);

        // ★ Prefer getActiveAccountId() from DB — it's the single source of truth
        // that Journal and other pages use.
        let activeId: number | null = null;
        try {
          activeId = await getActiveAccountId();
        } catch {
          // Fallback: use localStorage
          const stored = Number(localStorage.getItem(LS_KEY));
          const validStored = accs.find((a) => a.id === stored);
          activeId = validStored?.id ?? null;
        }

        // If DB didn't give a valid account, fallback to first active
        if (!activeId || !accs.find((a) => a.id === activeId)) {
          const firstActive = accs.find((a) => a.is_active) ?? accs[0];
          activeId = firstActive?.id ?? null;
        }

        setSelectedAccountId(activeId);

        if (activeId) {
          localStorage.setItem(LS_KEY, String(activeId));

          // ★ Check for snapshot specific to THIS account
          const existing = await getLatestSnapshot(activeId);
          if (!existing) {
            await autoSnapshotFromTrades(activeId).catch(() => {});
          }
        }
      } catch (e) {
        setError(String(e));
      }
      // ★ Pass the resolved accountId to loadAll
      const finalId = await getActiveAccountId().catch(() => null);
      await loadAll(finalId ?? undefined);
      setLoading(false);
    })();
  }, [loadAll]);

  async function handleSelectAccount(id: number) {
    setSelectedAccountId(id);
    localStorage.setItem(LS_KEY, String(id));
    setRefreshing(true);
    try {
      // ★ Also set as active in DB so Journal uses the same account
      await import("../../lib/accountsApi").then((m) => m.setActiveAccountId(id));
      await autoSnapshotFromTrades(id);
      await loadAll(id);
      setRefreshKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  async function refresh() {
    setRefreshing(true);
    setError(null);
    try {
      await autoSnapshotFromTrades(selectedAccountId ?? undefined);
      await loadAll(selectedAccountId ?? undefined);
      setRefreshKey((k) => k + 1);
    } catch (e) {
      setError(String(e));
    } finally {
      setRefreshing(false);
    }
  }

  // ★ Listen to account changes
  useEffect(() => {
    const handler = async (e: Event) => {
      const detail = (e as CustomEvent).detail as { id?: number } | undefined;
      if (detail?.id && detail.id !== selectedAccountId) {
        setSelectedAccountId(detail.id);
        localStorage.setItem(LS_KEY, String(detail.id));
        setRefreshing(true);
        try {
          await autoSnapshotFromTrades(detail.id);
          await loadAll(detail.id);
          setRefreshKey((k) => k + 1);
        } catch (err) {
          setError(String(err));
        } finally {
          setRefreshing(false);
        }
      }
    };
    window.addEventListener("activeAccountChanged", handler);
    return () => window.removeEventListener("activeAccountChanged", handler);
  }, [selectedAccountId, loadAll]);

  if (loading) {
    return (
      <div className="equity-engine">
        <div className="ee-loading">Loading Equity Engine…</div>
      </div>
    );
  }

  let status;
  try {
    status = deriveStatus(snapshot);
  } catch (e) {
    console.error("deriveStatus failed:", e);
    status = {
      label: "Disabled" as const,
      tone: "neg" as const,
      snapshot: null,
      equity: 0,
      hwm: 0,
      lwm: 0,
      ddPct: 0,
      netProfit: 0,
      openRisk: 0,
      dailyLoss: 0,
      streak: 0,
      snapshotAgeHours: Infinity,
    };
  }

  return (
    <div className="equity-engine">
      <header className="ee-header">
        <div>
          <h2 className="ee-title">Equity Engine</h2>
          <p className="ee-subtitle">
            Adaptive risk & position sizing — derived from your real trades
          </p>
        </div>
        <button
          className="ee-btn ee-btn--ghost ee-btn--icon"
          onClick={() => setShowSettings(true)}
          title="Settings"
        >
          ⚙
        </button>
      </header>

      {error && <div className="ee-alert ee-alert--error">{error}</div>}

      <AccountStatusCard
        status={status}
        accounts={accounts}
        selectedAccountId={selectedAccountId}
        onSelectAccount={handleSelectAccount}
        refreshing={refreshing}
        onRefresh={refresh}
        onOpenSnapshots={() => setShowSnapshots(true)}
      />

      <RiskIntelligenceCard preview={riskPreview} loading={loadingPreview} />

      <CapitalProtectionCard
        preview={riskPreview}
        accountId={selectedAccountId ?? undefined}
        refreshKey={refreshKey}
      />

      <NewTradeCard
        hasSnapshot={!!snapshot}
        accountId={selectedAccountId ?? undefined}
        allowedRisk={
          riskPreview
            ? parseFloat(riskPreview.r_allowed) || 0
            : snapshot
            ? (parseFloat(snapshot.equity) || 0) * 0.01
            : 0
        }
        onOpenWhyThisSize={setWhyResult}
      />

      <RecentDecisionsCard
        decisions={decisions}
        onViewAll={() => setShowAll(true)}
      />
        <InstrumentsCard />

      {whyResult && (
        <WhyThisSizeModal
          result={whyResult}
          onClose={() => setWhyResult(null)}
        />
      )}
      {showSettings && (
        <SettingsModal
          onClose={() => setShowSettings(false)}
          preview={riskPreview}
        />
      )}
      {showAll && (
        <AllDecisionsModal
          decisions={decisions}
          onClose={() => setShowAll(false)}
        />
      )}
      {showSnapshots && (
        <SnapshotsModal
          onClose={() => setShowSnapshots(false)}
          onChange={async () => {
            await loadAll();
          }}
        />
      )}
    </div>
  );
}
