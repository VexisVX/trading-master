# Roadmap

## Phase status

| Phase | Content | Status |
|-------|---------|--------|
| 0–1 | Scaffold, UI shell, SQLite init | ✅ done |
| 2 | Risk engine (Decimal, 20 tests) | ✅ done |
| 3 | Journal (create/close/list) | ✅ done |
| 4 | Live dashboard (equity, drawdown, R-distribution, PF) | ✅ done |
| 5 | Strategy Builder | ⏳ next |
| 6 | Risk Budget Panel (limits, exposure) | pending |
| 7 | Prop-firm rules (drawdown, daily loss) | pending |
| 8 | Advanced analytics (by strategy, session, plan-adherence) | pending |
| 9 | Journal completion (edit, screenshots, tags, CSV export) | pending |
| 10 | Packaging + install verification (Linux + Windows) | pending |

## Known technical debt

- `list_trades` limited to 500 rows in analytics — add pagination or incremental aggregation before >500 trades.
- `analytics_summary` recomputes from scratch every call — cache later, not now.
- Trade editing and deletion not implemented (Phase 9).
- Screenshots / attachments not implemented (Phase 9).
- Account selection hardcoded to the active default account (Phase 9).
- Strategy selection in Journal is disabled until Phase 5.
- No CSV import yet.

## Non-goals (by design)

- No broker order execution.
- No real-time market data.
- No Martingale, grid, or loss-recovery logic.
- No assumption of instrument specs or prop-firm rules.
