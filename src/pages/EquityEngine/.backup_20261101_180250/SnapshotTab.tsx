import { useState } from "react";
import { createSnapshot, SnapshotCreateInput, autoSnapshotFromTrades } from "../../lib/equityApi";

interface Props {
  onCreated: () => void;
}

export function SnapshotTab({ onCreated }: Props) {
  const [equity, setEquity] = useState("");
  const [initial, setInitial] = useState("");
  const [low, setLow] = useState("");
  const [dayStart, setDayStart] = useState("");
  const [openRisk, setOpenRisk] = useState("0");
  const [dailyLoss, setDailyLoss] = useState("0");
  const [consecutive, setConsecutive] = useState("0");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [ok, setOk] = useState(false);

  async function submit(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setOk(false);
    if (!equity || !initial) {
      setError("Equity and initial balance are required");
      return;
    }
    const input: SnapshotCreateInput = {
      equity,
      initial_balance: initial,
      low_watermark: low || equity,
      day_start_equity: dayStart || equity,
      open_portfolio_risk: openRisk || "0",
      daily_loss: dailyLoss || "0",
      consecutive_losses: parseInt(consecutive || "0", 10) || 0,
    };
    setBusy(true);
    try {
      await createSnapshot(input);
      setOk(true);
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="ee-snapshot card" onSubmit={submit}>
      <div className="ee-snapshot__hint">
        Snapshot is the real broker equity right now. HWM is auto-maintained.
        If you leave Low / Day Start empty, they default to the equity value.
      </div>

      <div className="ee-form-grid">
        <label>
          <span>Equity (now) *</span>
          <input
            className="mono"
            value={equity}
            onChange={(e) => setEquity(e.target.value)}
            placeholder="10450.00"
          />
        </label>
        <label>
          <span>Initial balance *</span>
          <input
            className="mono"
            value={initial}
            onChange={(e) => setInitial(e.target.value)}
            placeholder="10000.00"
          />
        </label>
        <label>
          <span>Low watermark</span>
          <input
            className="mono"
            value={low}
            onChange={(e) => setLow(e.target.value)}
            placeholder="(= equity)"
          />
        </label>
        <label>
          <span>Day start equity</span>
          <input
            className="mono"
            value={dayStart}
            onChange={(e) => setDayStart(e.target.value)}
            placeholder="(= equity)"
          />
        </label>
        <label>
          <span>Open portfolio risk</span>
          <input
            className="mono"
            value={openRisk}
            onChange={(e) => setOpenRisk(e.target.value)}
          />
        </label>
        <label>
          <span>Today's realized loss</span>
          <input
            className="mono"
            value={dailyLoss}
            onChange={(e) => setDailyLoss(e.target.value)}
          />
        </label>
        <label>
          <span>Consecutive losses</span>
          <input
            type="number"
            min={0}
            value={consecutive}
            onChange={(e) => setConsecutive(e.target.value)}
          />
        </label>
      </div>

      {error && <div className="ee-error">{error}</div>}
      {ok && <div className="ee-success">Snapshot saved.</div>}

      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button className="btn btn--primary" disabled={busy}>
          {busy ? "Saving…" : "Create snapshot"}
        </button>
        <button
          type="button"
          className="btn btn--ghost"
          disabled={busy}
          onClick={async () => {
            setBusy(true);
            setError(null);
            setOk(false);
            try {
              await autoSnapshotFromTrades();
              setOk(true);
              onCreated();
            } catch (err) {
              setError(String(err));
            } finally {
              setBusy(false);
            }
          }}
          title="Derive equity from trades in the database"
        >
          ⟳ Auto-compute from trades
        </button>
      </div>
    </form>
  );
}
