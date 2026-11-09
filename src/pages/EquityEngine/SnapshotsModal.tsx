import { useEffect, useState } from "react";
import {
  EquitySnapshotRow,
  clearSnapshots,
  cleanupOldSnapshots,
  deleteSnapshot,
  fmtMoney,
  listSnapshots,
  timeAgo,
} from "../../lib/equityApi";

interface Props {
  onClose: () => void;
  onChange: () => void;
}

export function SnapshotsModal({ onClose, onChange }: Props) {
  const [snapshots, setSnapshots] = useState<EquitySnapshotRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState<null | "all" | "old" | { id: number }>(
    null
  );

  async function load() {
    setLoading(true);
    setError(null);
    try {
      const list = await listSnapshots(undefined, 200);
      setSnapshots(list);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function doDeleteOne(id: number) {
    setBusy(true);
    try {
      await deleteSnapshot(id);
      await load();
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setConfirm(null);
    }
  }

  async function doClearAll() {
    setBusy(true);
    try {
      await clearSnapshots();
      await load();
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setConfirm(null);
    }
  }

  async function doCleanupOld() {
    setBusy(true);
    try {
      await cleanupOldSnapshots(30);
      await load();
      onChange();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      setConfirm(null);
    }
  }

  return (
    <div className="ee-modal-backdrop" onClick={onClose}>
      <div
        className="ee-modal ee-modal--wide"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="ee-modal__header">
          <div>
            <h3 className="ee-modal__title">Snapshot Management</h3>
            <p className="ee-modal__subtitle">
              {snapshots.length} snapshots · latest{" "}
              {snapshots[0] ? timeAgo(snapshots[0].taken_at) : "—"}
            </p>
          </div>
          <button
            className="ee-modal__close"
            onClick={onClose}
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        <div className="ee-modal__body">
          {error && <div className="ee-alert ee-alert--error">{error}</div>}

          <div
            style={{
              display: "flex",
              gap: 8,
              marginBottom: 16,
              flexWrap: "wrap",
            }}
          >
            <button
              className="ee-btn ee-btn--ghost"
              onClick={() => setConfirm("old")}
              disabled={busy || snapshots.length === 0}
            >
              🗑 Delete older than 30 days
            </button>
            <button
              className="ee-btn ee-btn--ghost"
              onClick={() => setConfirm("all")}
              disabled={busy || snapshots.length === 0}
              style={{ color: "#ff6b6b" }}
            >
              🗑 Delete all ({snapshots.length})
            </button>
          </div>

          {loading ? (
            <div className="ee-empty-inline">Loading…</div>
          ) : snapshots.length === 0 ? (
            <div className="ee-empty-inline">No snapshots yet.</div>
          ) : (
            <table className="ee-table">
              <thead>
                <tr>
                  <th>ID</th>
                  <th>Equity</th>
                  <th>Initial</th>
                  <th>Peak</th>
                  <th>Taken</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {snapshots.map((s) => (
                  <tr key={s.id}>
                    <td className="mono ee-muted">#{s.id}</td>
                    <td className="mono">{fmtMoney(s.equity)}</td>
                    <td className="mono">{fmtMoney(s.initial_balance)}</td>
                    <td className="mono">{fmtMoney(s.high_watermark)}</td>
                    <td className="ee-muted">
                      {timeAgo(s.taken_at)}
                      <div
                        className="ee-muted"
                        style={{ fontSize: 10, marginTop: 2 }}
                      >
                        {new Date(s.taken_at).toLocaleString()}
                      </div>
                    </td>
                    <td>
                      <button
                        className="ee-btn ee-btn--ghost"
                        style={{ padding: "4px 10px", fontSize: 12 }}
                        onClick={() => setConfirm({ id: s.id })}
                        disabled={busy}
                        title="Delete this snapshot"
                      >
                        🗑
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}

          {confirm && (
            <div className="ee-confirm">
              <div className="ee-confirm__msg">
                {confirm === "all"
                  ? `Delete ALL ${snapshots.length} snapshots? This cannot be undone.`
                  : confirm === "old"
                  ? "Delete snapshots older than 30 days?"
                  : `Delete snapshot #${(confirm as { id: number }).id}?`}
              </div>
              <div className="ee-confirm__actions">
                <button
                  className="ee-btn ee-btn--ghost"
                  onClick={() => setConfirm(null)}
                  disabled={busy}
                >
                  Cancel
                </button>
                <button
                  className="ee-btn ee-btn--primary"
                  style={{
                    background: "#ff6b6b",
                    borderColor: "#ff6b6b",
                    color: "#fff",
                  }}
                  onClick={() => {
                    if (confirm === "all") doClearAll();
                    else if (confirm === "old") doCleanupOld();
                    else doDeleteOne((confirm as { id: number }).id);
                  }}
                  disabled={busy}
                >
                  {busy ? "Deleting…" : "Delete"}
                </button>
              </div>
            </div>
          )}
        </div>

        <div className="ee-modal__footer">
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
