import { useCallback, useEffect, useState } from "react";
import {
  EquitySnapshotRow,
  RiskPreview,
  listSnapshots,
} from "../../lib/equityApi";
import { EquityCurveChart } from "./EquityCurveChart";
import { CapitalWaterfall } from "./CapitalWaterfall";

interface Props {
  preview: RiskPreview | null;
  refreshKey: number; // bump to re-fetch snapshots
}

export function CapitalProtectionCard({ preview, refreshKey }: Props) {
  const [snapshots, setSnapshots] = useState<EquitySnapshotRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await listSnapshots(30);
      setSnapshots(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load, refreshKey]);

  return (
    <section className="ee-card ee-capital-protection">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Capital Protection</h3>
          <p className="ee-card__subtitle">
            Equity, high-watermark, and floor over time
          </p>
        </div>
        <span className="ee-muted" style={{ fontSize: 11 }}>
          {snapshots.length} snapshot{snapshots.length === 1 ? "" : "s"}
        </span>
      </div>

      {error && <div className="ee-alert ee-alert--error">{error}</div>}

      <div className="ee-capital-protection__grid">
        <div className="ee-capital-protection__chart">
          <div className="ee-capital-protection__section-label">
            EQUITY CURVE
          </div>
          {loading ? (
            <div className="ee-empty-inline">Loading snapshots…</div>
          ) : (
            <EquityCurveChart snapshots={snapshots} maxPoints={30} />
          )}
        </div>

        <div className="ee-capital-protection__waterfall">
          <div className="ee-capital-protection__section-label">
            CAPITAL WATERFALL
          </div>
          {preview ? (
            <CapitalWaterfall
              equity={preview.equity}
              initialBalance={preview.initial_balance}
              floor={preview.f_hybrid}
              protectedProfit={preview.profit_retention}
              lossCapacity={preview.loss_capacity}
            />
          ) : (
            <div className="ee-empty-inline">
              No snapshot yet — click ⟳ Refresh.
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
