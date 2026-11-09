import { useCallback, useEffect, useState } from "react";
import {
  getLatestSnapshot,
  listDecisions,
  EquitySnapshotRow,
  DecisionRowOut,
} from "../../lib/equityApi";
import { OverviewTab } from "./OverviewTab";
import { SnapshotTab } from "./SnapshotTab";
import { DecisionTab } from "./DecisionTab";
import { ConfigTab } from "./ConfigTab";
import { AuditTab } from "./AuditTab";
import "./EquityEngine.css";

type Tab = "overview" | "snapshot" | "decision" | "config" | "audit";

export function EquityEngine() {
  const [tab, setTab] = useState<Tab>("overview");
  const [snapshot, setSnapshot] = useState<EquitySnapshotRow | null>(null);
  const [decisions, setDecisions] = useState<DecisionRowOut[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);

  const reload = useCallback(async () => {
    try {
      const [snap, dec] = await Promise.all([
        getLatestSnapshot(),
        listDecisions(50),
      ]);
      setSnapshot(snap);
      setDecisions(dec);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    reload();
  }, [reload, tick]);

  function bump() {
    setTick((t) => t + 1);
  }

  return (
    <div className="equity-engine">
      <header className="ee-header">
        <div>
          <h2 className="ee-title">Equity Engine</h2>
          <p className="ee-subtitle">
            Adaptive risk & position sizing. All math computed in Rust with
            Decimal — never in the browser.
          </p>
        </div>
      </header>

      <nav className="ee-tabs" role="tablist">
        {(
          [
            ["overview", "Overview"],
            ["snapshot", "Snapshot"],
            ["decision", "Decision"],
            ["config", "Config"],
            ["audit", "Audit"],
          ] as [Tab, string][]
        ).map(([id, label]) => (
          <button
            key={id}
            role="tab"
            aria-selected={tab === id}
            className={`ee-tab ${tab === id ? "is-active" : ""}`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </nav>

      {error && <div className="ee-error">{error}</div>}

      {tab === "overview" && (
        <OverviewTab snapshot={snapshot} onGoSnapshot={() => setTab("snapshot")} />
      )}
      {tab === "snapshot" && <SnapshotTab onCreated={bump} />}
      {tab === "decision" && <DecisionTab hasSnapshot={!!snapshot} />}
      {tab === "config" && <ConfigTab />}
      {tab === "audit" && <AuditTab decisions={decisions} />}
    </div>
  );
}
