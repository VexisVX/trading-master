#!/usr/bin/env bash
set -euo pipefail

if [ -f "src-tauri/Cargo.toml" ]; then PROJECT_DIR="$(pwd)"
elif [ -d "trading-manager" ]; then PROJECT_DIR="$(pwd)/trading-manager"
else echo "FATAL"; exit 1; fi
cd "${PROJECT_DIR}"

echo "==> [1/3] Show current .sidebar__item rules"
grep -n "sidebar__item" src/components/layout/Sidebar.css

echo ""
echo "==> [2/3] Append no-border nav-style for sidebar__item"
cat >> src/components/layout/Sidebar.css <<'CSS_EOF'

/* ============================================================
 * Sidebar item — nav-style (no border, like Gmail/Notion)
 * ============================================================ */

.sidebar__item,
button.sidebar__item {
  background: transparent !important;
  border: none !important;
  color: var(--text-1, #9aa7b4) !important;
  padding: 8px 12px !important;
  border-radius: var(--radius-sm, 6px) !important;
  text-align: left;
  cursor: pointer;
  font-family: inherit;
  font-size: inherit;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: background 0.12s ease, color 0.12s ease;
}

.sidebar__item:hover,
button.sidebar__item:hover {
  background: var(--bg-2, #161d26) !important;
  color: var(--text-0, #e6edf3) !important;
  border: none !important;
}

.sidebar__item.is-active,
.sidebar__item.active,
button.sidebar__item.is-active {
  background: var(--accent-soft, rgba(45, 212, 191, 0.12)) !important;
  color: var(--accent, #2dd4bf) !important;
  border: none !important;
}

.sidebar__item:focus,
button.sidebar__item:focus {
  outline: none !important;
  box-shadow: none !important;
}

.sidebar__item:focus-visible,
button.sidebar__item:focus-visible {
  outline: 2px solid var(--accent, #2dd4bf) !important;
  outline-offset: 2px;
}

.sidebar__item:active,
button.sidebar__item:active {
  background: var(--bg-3, #1d2530) !important;
  color: var(--accent, #2dd4bf) !important;
  border: none !important;
}

/* Light theme */
[data-theme="light"] .sidebar__item,
[data-theme="light"] button.sidebar__item {
  color: #475569 !important;
}
[data-theme="light"] .sidebar__item:hover,
[data-theme="light"] button.sidebar__item:hover {
  background: #f1f5f9 !important;
  color: #0f172a !important;
}
[data-theme="light"] .sidebar__item.is-active,
[data-theme="light"] button.sidebar__item.is-active {
  background: #cffafe !important;
  color: #0891b2 !important;
}

/* Cream theme */
[data-theme="cream"] .sidebar__item,
[data-theme="cream"] button.sidebar__item {
  color: #4a3f2e !important;
}
[data-theme="cream"] .sidebar__item:hover,
[data-theme="cream"] button.sidebar__item:hover {
  background: #efe7d8 !important;
  color: #2e2418 !important;
}
[data-theme="cream"] .sidebar__item.is-active,
[data-theme="cream"] button.sidebar__item.is-active {
  background: #d4ecec !important;
  color: #0e7490 !important;
}

/* Dim theme */
[data-theme="dim"] .sidebar__item,
[data-theme="dim"] button.sidebar__item {
  color: var(--text-1, #b8c0cc) !important;
}
[data-theme="dim"] .sidebar__item:hover,
[data-theme="dim"] button.sidebar__item:hover {
  background: #2b323d !important;
  color: #e6edf3 !important;
}
[data-theme="dim"] .sidebar__item.is-active,
[data-theme="dim"] button.sidebar__item.is-active {
  background: rgba(45, 212, 191, 0.15) !important;
  color: var(--accent, #2dd4bf) !important;
}

/* Amoled theme */
[data-theme="amoled"] .sidebar__item,
[data-theme="amoled"] button.sidebar__item {
  color: #b0b0b0 !important;
}
[data-theme="amoled"] .sidebar__item:hover,
[data-theme="amoled"] button.sidebar__item:hover {
  background: #0a0a0a !important;
  color: #ffffff !important;
}
[data-theme="amoled"] .sidebar__item.is-active,
[data-theme="amoled"] button.sidebar__item.is-active {
  background: rgba(45, 212, 191, 0.15) !important;
  color: var(--accent, #2dd4bf) !important;
}

CSS_EOF
echo "  ✓ Sidebar item override appended"

echo ""
echo "==> [3/3] Build"
npm run build 2>&1 | tail -8
RC=${PIPESTATUS[0]:-$?}
if [ "$RC" -ne 0 ]; then
    echo "FATAL"
    exit 4
fi

echo ""
echo "============================================================"
echo "SIDEBAR ITEM FIXED"
echo "  ✓ .sidebar__item: no border, nav-style"
echo "  ✓ Active: accent-soft bg"
echo "  ✓ All 5 themes (dark/light/cream/dim/amoled)"
echo "============================================================"
