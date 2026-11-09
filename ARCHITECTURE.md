# Architecture Rules (AI must read before any change)

## Money & Decimal
- NEVER use f64/f32 for money, P&L, R-multiple, drawdown, prices.
- All financial math lives in src-tauri/src/risk/ using rust_decimal::Decimal.
- Values cross the Tauri boundary as STRINGS, not numbers.
- Rounding: always explicit (MidpointAwayFromZero for money, MidpointEven for stats).

## Data
- SQLite access ONLY from Rust (src-tauri/src/db). Frontend never touches SQL.
- All schema changes via versioned migrations in db/migrations/. Never edit old migrations.
- Before any migration runs at startup, create a backup file automatically.

## Safety
- No order execution to brokers. No real-time market data.
- Never assume instrument specs or prop-firm rules. Distinguish verified / user-entered / estimated.
- No Martingale, grid, or loss-recovery logic.

## AI workflow
- Touch only files related to the current task.
- Every new financial function needs at least one unit test.
- Do not modify Cargo.toml, package.json, or migrations without explicit approval.
- If unsure, stop and ask. Do not guess.
