//! Risk engine public API.
//!
//! All financial math uses `rust_decimal::Decimal` — never `f64`/`f32`.
//! Values cross the Tauri boundary as strings (see `serde-with-str`).
//!
//! ## Stability
//!
//! The re-exports below form the **public surface** of the risk engine.
//! Some names may not yet have a consumer (e.g. `PositionError` is only
//! matched in tests today, but the Journal will use it in later phases).
//! We intentionally re-export the full set so callers can rely on a stable
//! path (`risk::X`) even as internal modules are reorganized.
//!
//! `#![allow(unused_imports)]` here is **deliberate and local** to this
//! file: it suppresses warnings for re-exports that are not yet consumed
//! inside this crate. Do not copy this pattern elsewhere.
#![allow(unused_imports)]

pub mod budget;
pub mod drawdown;
pub mod live_calc;
pub mod money;
pub mod news;
pub mod position;
pub mod prop_firm;
pub mod r_multiple;
pub mod rr;

// -- Public surface ---------------------------------------------------
pub use budget::{compute_risk_status, trade_open_risk, trade_realized_pnl};
pub use drawdown::{compute_drawdown, DrawdownError, DrawdownPoint, DrawdownResult};
pub use live_calc::{
    assistant, break_even, calculate as calculate_live_risk, martingale_check, partial_close,
    portfolio_risk, size_from_risk, Advice, AssistantInput, AssistantResult, BreakEvenInput,
    BreakEvenResult, LiveCalcInput, LiveCalcResult, MartingaleCheckInput, MartingaleCheckResult,
    PartialCloseInput, PartialCloseResult, PortfolioRiskInput, PortfolioRiskResult,
    PortfolioTradeInput, PortfolioTradeResult, SizeFromRiskInput, SizeFromRiskResult,
};
pub use money::{parse_decimal, round_money, round_stat, safe_div, MoneyError};
pub use position::{compute_position_size, PositionError, PositionSizeInput, PositionSizeResult};
pub use prop_firm::evaluate as evaluate_prop_firm;
pub use r_multiple::{compute_r, Exit, RError, RResult, TradeR};
pub use rr::{planned_rr, realized_r, trade_planned_rr};
