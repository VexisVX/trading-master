import { useState } from "react";
import {
  Instrument,
  InstrumentInput,
  SpecSource,
  upsertInstrument,
  deleteInstrument,
} from "../../lib/equityApi";

interface Props {
  initial?: Instrument | null;
  onClose: () => void;
  onSaved: () => void;
}

const DEFAULT_INPUT: InstrumentInput = {
  symbol: "",
  currency: "USD",
  tick_size: "0.00001",
  tick_value: "1",
  contract_size: "100000",
  min_qty: "0.01",
  max_qty: "100",
  lot_step: "0.01",
  commission_per_lot: "0",
  slippage_per_unit: "0",
  spec_source: "user_entered",
};

export function InstrumentModal({ initial, onClose, onSaved }: Props) {
  const [input, setInput] = useState<InstrumentInput>(
    initial
      ? {
          symbol: initial.symbol,
          currency: initial.currency,
          tick_size: initial.tick_size,
          tick_value: initial.tick_value,
          contract_size: initial.contract_size,
          min_qty: initial.min_qty,
          max_qty: initial.max_qty,
          lot_step: initial.lot_step,
          commission_per_lot: initial.commission_per_lot,
          slippage_per_unit: initial.slippage_per_unit,
          spec_source: initial.spec_source,
        }
      : DEFAULT_INPUT
  );
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const isEdit = !!initial;

  function set<K extends keyof InstrumentInput>(k: K, v: InstrumentInput[K]) {
    setInput({ ...input, [k]: v });
  }

  async function save() {
    setBusy(true);
    setErr(null);
    try {
      await upsertInstrument({
        ...input,
        symbol: input.symbol.trim().toUpperCase(),
      });
      onSaved();
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function del() {
    if (!initial) return;
    if (!confirm(`Delete ${initial.symbol}?`)) return;
    setBusy(true);
    setErr(null);
    try {
      await deleteInstrument(initial.symbol);
      onSaved();
      onClose();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
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
            <h3 className="ee-modal__title">
              {isEdit ? `Edit ${initial?.symbol}` : "Add Instrument"}
            </h3>
            <p className="ee-modal__subtitle">
              Specs from your broker's Symbol Specification window
            </p>
          </div>
          <button className="ee-modal__close" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="ee-modal__body">
          <div className="ee-inst-hint">
            <strong>Where to find these values:</strong>
            <ul>
              <li>MT4/MT5 → Market Watch → right-click symbol → <em>Specification</em></li>
              <li><strong>Tick size</strong>: Symbol info → Tick size (e.g. 0.00001 for 5-digit EURUSD)</li>
              <li><strong>Tick value</strong>: Contract size × tick size (usually $1 for forex majors)</li>
              <li><strong>Contract size</strong>: 100,000 for standard forex lot</li>
              <li><strong>Lot step</strong>: usually 0.01 — minimum change in lot size</li>
              <li><strong>Min/Max qty</strong>: smallest/largest order allowed</li>
            </ul>
            <p className="ee-inst-hint__warn">
              ⚠ Never use <strong>estimated</strong> source for live sizing.
              Use <strong>verified</strong> (broker-confirmed) or{" "}
              <strong>user_entered</strong> (you copied from MT4/MT5).
            </p>
          </div>

          <div className="ee-inst-grid">
            <Field label="Symbol *" hint="e.g. EURUSD, XAUUSD">
              <input
                className="mono"
                value={input.symbol}
                onChange={(e) => set("symbol", e.target.value)}
                placeholder="EURUSD"
                disabled={isEdit}
              />
            </Field>
            <Field label="Currency">
              <select
                value={input.currency}
                onChange={(e) => set("currency", e.target.value)}
              >
                <option value="USD">USD</option>
                <option value="EUR">EUR</option>
                <option value="GBP">GBP</option>
                <option value="JPY">JPY</option>
              </select>
            </Field>
            <Field label="Tick size" hint="min price increment">
              <input
                className="mono"
                value={input.tick_size}
                onChange={(e) => set("tick_size", e.target.value)}
              />
            </Field>
            <Field label="Tick value" hint="$ per tick per lot (in account currency)">
              <input
                className="mono"
                value={input.tick_value}
                onChange={(e) => set("tick_value", e.target.value)}
              />
            </Field>
            <Field label="Contract size" hint="100000 for forex">
              <input
                className="mono"
                value={input.contract_size}
                onChange={(e) => set("contract_size", e.target.value)}
              />
            </Field>
            <Field label="Lot step" hint="e.g. 0.01">
              <input
                className="mono"
                value={input.lot_step}
                onChange={(e) => set("lot_step", e.target.value)}
              />
            </Field>
            <Field label="Min qty">
              <input
                className="mono"
                value={input.min_qty}
                onChange={(e) => set("min_qty", e.target.value)}
              />
            </Field>
            <Field label="Max qty">
              <input
                className="mono"
                value={input.max_qty}
                onChange={(e) => set("max_qty", e.target.value)}
              />
            </Field>
            <Field label="Commission / lot" hint="$ per round-trip lot">
              <input
                className="mono"
                value={input.commission_per_lot}
                onChange={(e) => set("commission_per_lot", e.target.value)}
              />
            </Field>
            <Field label="Slippage / unit" hint="$ per unit">
              <input
                className="mono"
                value={input.slippage_per_unit}
                onChange={(e) => set("slippage_per_unit", e.target.value)}
              />
            </Field>
            <Field label="Source" hint="provenance">
              <select
                value={input.spec_source}
                onChange={(e) =>
                  set("spec_source", e.target.value as SpecSource)
                }
              >
                <option value="verified">verified</option>
                <option value="user_entered">user_entered</option>
                <option value="estimated">estimated (⚠ unsafe)</option>
              </select>
            </Field>
          </div>

          {err && <div className="ee-alert ee-alert--error">{err}</div>}
        </div>

        <div className="ee-modal__footer">
          {isEdit && (
            <button className="ee-btn ee-btn--danger" onClick={del} disabled={busy}>
              Delete
            </button>
          )}
          <div style={{ flex: 1 }} />
          <button className="ee-btn ee-btn--ghost" onClick={onClose}>
            Cancel
          </button>
          <button className="ee-btn ee-btn--primary" onClick={save} disabled={busy}>
            {busy ? "Saving…" : isEdit ? "Save" : "Add"}
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <label className="ee-inst-field">
      <span className="ee-inst-field__label">
        {label}
        {hint && <em className="ee-inst-field__hint"> — {hint}</em>}
      </span>
      {children}
    </label>
  );
}
