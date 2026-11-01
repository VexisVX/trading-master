//! Adaptive Equity Management Engine — v1.0
//!
//! All R_* quantities are MONEY (Decimal). No floats.
//! Evaluation is Fail-fast; every rejection has a structured reason.

pub mod config;
pub mod engine;
pub mod error;
pub mod risk_engine;
pub mod sizing;
pub mod state;

pub use config::EquityConfig;
pub use engine::{evaluate, TradeDecision, TradeProposal};
pub use error::{DecisionReason, EquityError};
pub use risk_engine::{compute_risk, RiskResult};
pub use sizing::{compute_size, SizingInput, SizingResult};
pub use state::{AccountPhase, AccountState, EquitySnapshot, ParsedSnapshot};
