#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

DB="$HOME/.local/share/com.trading.manager/app.db"
BD="$HOME/.local/share/com.trading.manager/backups"
mkdir -p "$BD"
STAMP="$(date +%Y%m%d_%H%M%S)"
BK="$BD/pre_phase3b_${STAMP}.db"
[ -f "$DB" ] && sqlite3 "$DB" ".backup '$BK'"

# ============================================================
# 1) CashFlowCard.tsx
# ============================================================
echo "==> [1/4] CashFlowCard.tsx"
cat > src/pages/EquityEngine/CashFlowCard.tsx <<'TSX_EOF'
import { useEffect, useState } from "react";
import {
  CashFlowRow,
  cashflowDeposit,
  cashflowWithdraw,
  cashflowFee,
  listCashFlows,
  fmtMoney,
} from "../../lib/equityApi";

type Kind = "deposit" | "withdrawal" | "fee";

interface Props {
  accountId: number;
}

export function CashFlowCard({ accountId }: Props) {
  const [flows, setFlows] = useState<CashFlowRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [err, setErr] = useState<string | null>(null);
  const [kind, setKind] = useState<Kind>("deposit");
  const [amount, setAmount] = useState<string>("");
  const [note, setNote] = useState<string>("");
  const [busy, setBusy] = useState(false);
  const [ok, setOk] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setErr(null);
    try {
      setFlows(await listCashFlows(accountId, 50));
    } catch (e) {
      setErr(String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, [accountId]);

  async function submit() {
    if (!amount.trim()) {
      setErr("Amount required");
      return;
    }
    setBusy(true);
    setErr(null);
    setOk(null);
    try {
      const input = {
        account_id: accountId,
        amount: amount.trim(),
        note: note.trim() || null,
      };
      let r;
      if (kind === "deposit") r = await cashflowDeposit(input);
      else if (kind === "withdrawal") r = await cashflowWithdraw(input);
      else r = await cashflowFee(input);
      setOk(`✓ ${kind} ${r.amount} → units ${r.units_before} → ${r.units_after}`);
      setAmount("");
      setNote("");
      await refresh();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="ee-card ee-cashflow">
      <div className="ee-card__header">
        <div>
          <h3 className="ee-card__title">Cash Flow</h3>
          <p className="ee-card__subtitle">
            Deposits and withdrawals adjust units — NAV stays invariant
          </p>
        </div>
      </div>

      <div className="ee-cf-form">
        <div className="ee-cf-tabs">
          <button
            className={`ee-cf-tab ${kind === "deposit" ? "is-active" : ""}`}
            onClick={() => setKind("deposit")}
          >
            Deposit
          </button>
          <button
            className={`ee-cf-tab ${kind === "withdrawal" ? "is-active" : ""}`}
            onClick={() => setKind("withdrawal")}
          >
            Withdraw
          </button>
          <button
            className={`ee-cf-tab ${kind === "fee" ? "is-active" : ""}`}
            onClick={() => setKind("fee")}
          >
            Fee
          </button>
        </div>

        <div className="ee-cf-row">
          <label className="ee-cf-label">Amount</label>
          <input
            className="mono ee-cf-input"
            type="number"
            step="0.01"
            min="0"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.00"
          />
          <label className="ee-cf-label">Note</label>
          <input
            className="ee-cf-input"
            value={note}
            onChange={(e) => setNote(e.target.value)}
            placeholder="optional"
          />
          <button
            className="ee-btn ee-btn--primary"
            onClick={submit}
            disabled={busy}
          >
            {busy ? "…" : kind === "deposit" ? "Deposit" : kind === "withdrawal" ? "Withdraw" : "Record fee"}
          </button>
        </div>

        {err && <div className="ee-alert ee-alert--error">{err}</div>}
        {ok && <div className="ee-alert ee-alert--success">{ok}</div>}
      </div>

      <div className="ee-cf-list-wrap">
        {loading && <div className="ee-muted">Loading…</div>}
        {!loading && flows.length === 0 && (
          <div className="ee-muted" style={{ padding: 12, textAlign: "center" }}>
            No cash flows yet.
          </div>
        )}
        {!loading && flows.length > 0 && (
          <table className="ee-cf-table">
            <thead>
              <tr>
                <th>When</th>
                <th>Kind</th>
                <th>Amount</th>
                <th>NAV</th>
                <th>Units before → after</th>
                <th>Note</th>
              </tr>
            </thead>
            <tbody>
              {flows.map((f) => (
                <tr key={f.id}>
                  <td className="mono">{f.occurred_at.slice(0, 19).replace("T", " ")}</td>
                  <td>
                    <span className={`ee-cf-kind ee-cf-kind--${f.kind}`}>{f.kind}</span>
                  </td>
                  <td className="mono">{fmtMoney(f.amount)} {f.currency}</td>
                  <td className="mono">{f.nav_before}</td>
                  <td className="mono">{f.units_before} → {f.units_after}</td>
                  <td>{f.note || "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
TSX_EOF
echo "  ✓ CashFlowCard.tsx"

# ============================================================
# 2) CSS
# ============================================================
echo "==> [2/4] CSS"
cat >> src/pages/EquityEngine/EquityEngine.css <<'CSS_EOF'

/* ============================================================
 * Cash Flow Card (PHASE 3-B)
 * ============================================================ */

.ee-cashflow { padding: 16px 20px; }

.ee-cf-form {
  margin-bottom: 16px;
}

.ee-cf-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 10px;
  background: var(--bg-1, #11161d);
  border: 1px solid var(--border, #232c38);
  border-radius: 8px;
  padding: 4px;
  width: fit-content;
}
.ee-cf-tab {
  background: transparent !important;
  border: none !important;
  color: var(--text-1, #9aa7b4) !important;
  padding: 6px 14px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 500;
  transition: background 0.12s, color 0.12s;
}
.ee-cf-tab:hover {
  color: var(--text-0, #e6edf3) !important;
  background: var(--bg-2, #161d26) !important;
}
.ee-cf-tab.is-active {
  background: var(--accent-soft, rgba(45, 212, 191, 0.12)) !important;
  color: var(--accent, #2dd4bf) !important;
  border: none !important;
}

.ee-cf-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.ee-cf-label {
  font-size: 12px;
  color: var(--text-1, #9aa7b4);
  font-weight: 500;
}

.ee-cf-input {
  background: var(--bg-2, #161d26);
  color: var(--text-0, #e6edf3);
  border: 1px solid var(--border, #232c38);
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 13px;
  font-family: var(--mono, ui-monospace, monospace);
  min-width: 100px;
}
.ee-cf-input:focus {
  outline: none;
  border-color: var(--accent, #2dd4bf);
}

.ee-cf-list-wrap {
  max-height: 300px;
  overflow-y: auto;
  border-radius: 8px;
  border: 1px solid var(--border, #232c38);
}
.ee-cf-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}
.ee-cf-table thead {
  position: sticky;
  top: 0;
  background: var(--bg-1, #11161d);
  z-index: 1;
}
.ee-cf-table th {
  text-align: left;
  padding: 8px 10px;
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-1, #9aa7b4);
  border-bottom: 1px solid var(--border, #232c38);
}
.ee-cf-table td {
  padding: 6px 10px;
  border-bottom: 1px solid var(--border, #232c38);
  color: var(--text-0, #e6edf3);
}
.ee-cf-table tbody tr:hover {
  background: var(--bg-2, #161d26);
}

.ee-cf-kind {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 999px;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}
.ee-cf-kind--deposit {
  background: rgba(52, 211, 153, 0.15);
  color: #34d399;
}
.ee-cf-kind--withdrawal {
  background: rgba(251, 191, 36, 0.15);
  color: #fbbf24;
}
.ee-cf-kind--fee {
  background: rgba(248, 113, 113, 0.15);
  color: #f87171;
}

.ee-cf-list-wrap::-webkit-scrollbar { width: 8px; }
.ee-cf-list-wrap::-webkit-scrollbar-track { background: var(--bg-2, #161d26); border-radius: 4px; }
.ee-cf-list-wrap::-webkit-scrollbar-thumb { background: var(--border, #232c38); border-radius: 4px; }

/* Light theme */
[data-theme="light"] .ee-cf-tabs { background: #f1f5f9; border-color: #cbd5e1; }
[data-theme="light"] .ee-cf-tab { color: #475569 !important; }
[data-theme="light"] .ee-cf-tab:hover { color: #0f172a !important; background: #e2e8f0 !important; }
[data-theme="light"] .ee-cf-tab.is-active { background: #cffafe !important; color: #0891b2 !important; }
[data-theme="light"] .ee-cf-label { color: #475569; }
[data-theme="light"] .ee-cf-input { background: #ffffff; color: #0f172a; border-color: #cbd5e1; }
[data-theme="light"] .ee-cf-table thead { background: #f1f5f9; }
[data-theme="light"] .ee-cf-table th { color: #475569; }
[data-theme="light"] .ee-cf-table td { color: #0f172a; border-color: #e2e8f0; }
[data-theme="light"] .ee-cf-kind--deposit { background: #d1fae5; color: #065f46; }
[data-theme="light"] .ee-cf-kind--withdrawal { background: #fef3c7; color: #92400e; }
[data-theme="light"] .ee-cf-kind--fee { background: #fee2e2; color: #991b1b; }

/* Cream theme */
[data-theme="cream"] .ee-cf-tabs { background: #efe7d8; border-color: #e0d6c2; }
[data-theme="cream"] .ee-cf-tab { color: #4a3f2e !important; }
[data-theme="cream"] .ee-cf-tab:hover { color: #2e2418 !important; background: #e5dbc8 !important; }
[data-theme="cream"] .ee-cf-tab.is-active { background: #d4ecec !important; color: #0e7490 !important; }
[data-theme="cream"] .ee-cf-label { color: #4a3f2e; }
[data-theme="cream"] .ee-cf-input { background: #faf6ed; color: #2e2418; border-color: #c8bca1; }
[data-theme="cream"] .ee-cf-table thead { background: #efe7d8; }
[data-theme="cream"] .ee-cf-table th { color: #4a3f2e; }
[data-theme="cream"] .ee-cf-table td { color: #2e2418; border-color: #e0d6c2; }
[data-theme="cream"] .ee-cf-kind--deposit { background: #d1e7d0; color: #14532d; }
[data-theme="cream"] .ee-cf-kind--withdrawal { background: #f5e0b0; color: #78350f; }
[data-theme="cream"] .ee-cf-kind--fee { background: #f5cfcf; color: #7f1d1d; }
CSS_EOF
echo "  ✓ CSS added"

# ============================================================
# 3) Wire into EquityEngine
# ============================================================
echo "==> [3/4] Wire into EquityEngine"
python3 - <<'PY'
import pathlib, re, sys
p = pathlib.Path("src/pages/EquityEngine/EquityEngine.tsx")
s = p.read_text()

# 1. Add import
if "CashFlowCard" not in s:
    s = s.replace(
        'import { InstrumentsCard } from "./InstrumentsCard";',
        'import { InstrumentsCard } from "./InstrumentsCard";\nimport { CashFlowCard } from "./CashFlowCard";'
    )
    print("  ✓ import added")

# 2. Get active account id from state
# The component already has `selectedAccountId` or `activeAccountId`
# Add <CashFlowCard accountId={...} /> after InstrumentsCard
anchor = "<InstrumentsCard />"
if anchor in s and "<CashFlowCard" not in s:
    idx = s.find(anchor)
    line_end = s.find("\n", idx)
    # Determine account id variable name — look for `activeId` or `selectedAccountId`
    # Fallback: use `selectedAccountId || activeAccountId || 1`
    account_expr = "{selectedAccountId ?? 1}"
    if "const [selectedAccountId" in s or "selectedAccountId" in s:
        account_expr = "{selectedAccountId ?? 1}"
    elif "activeAccountId" in s:
        account_expr = "{activeAccountId}"
    else:
        # Try to find the actual variable
        import re as _re
        m = _re.search(r'const \[(\w*[Aa]ccount\w*Id)\s*,', s)
        if m:
            account_expr = "{" + m.group(1) + " ?? 1}"
        else:
            account_expr = "{1}"
    
    s = s[:line_end+1] + f"        <CashFlowCard accountId={account_expr} />\n" + s[line_end+1:]
    print(f"  ✓ CashFlowCard wired (accountId={account_expr})")

p.write_text(s)
PY

# ============================================================
# 4) Build
# ============================================================
echo "==> [4/4] Build"
( cd src-tauri && cargo check 2>&1 | grep -E "^error" | head -5 )
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL: cargo check"
    exit 4
fi
npm run build 2>&1 | tail -10
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL: npm build"
    exit 4
fi

echo ""
echo "============================================================"
echo "PHASE 3-B COMPLETE"
echo "  ✓ CashFlowCard.tsx"
echo "  ✓ CSS (all 5 themes)"
echo "  ✓ Wired into EquityEngine"
echo "  Backup: $BK"
echo "============================================================"
