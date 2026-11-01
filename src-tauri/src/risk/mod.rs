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

pub mod live_calc;
pub mod money;
pub mod news;
pub mod position;
pub mod rr;
pub mod prop_firm;
pub mod r_multiple;
pub mod budget;
pub mod drawdown;

// -- Public surface ---------------------------------------------------
pub use live_calc::{
    portfolio_risk, PortfolioRiskInput, PortfolioRiskResult,
    PortfolioTradeInput, PortfolioTradeResult,
    assistant, AssistantInput, AssistantResult, Advice,
    calculate as calculate_live_risk, LiveCalcInput, LiveCalcResult,
    partial_close, PartialCloseInput, PartialCloseResult,
    martingale_check, MartingaleCheckInput, MartingaleCheckResult,
    break_even, BreakEvenInput, BreakEvenResult,
    size_from_risk, SizeFromRiskInput, SizeFromRiskResult,
};
pub use money::{
    round_money, round_stat, parse_decimal, safe_div, MoneyError,
};
pub use position::{
    compute_position_size, PositionSizeInput, PositionSizeResult, PositionError,
};
pub use r_multiple::{
    compute_r, TradeR, RResult, RError, Exit,
};
pub use rr::{planned_rr, realized_r, trade_planned_rr};
pub use prop_firm::{evaluate as evaluate_prop_firm};
pub use budget::{trade_open_risk, trade_realized_pnl, compute_risk_status};
pub use drawdown::{
    compute_drawdown, DrawdownResult, DrawdownPoint, DrawdownError,
};
