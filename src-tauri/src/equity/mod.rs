//! Adaptive Equity Management Engine — v1.0
//!
//! All R_* quantities are MONEY (Decimal). No floats.
//! Evaluation is Fail-fast; every rejection has a structured reason.

pub mod buffer;
pub mod cashflow;
pub mod config;
pub mod daily;
pub mod engine;
pub mod error;
pub mod nav;
pub mod profile;
pub mod prop_rules;
pub mod protection;
pub mod recovery;
pub mod reset;
pub mod risk_engine;
#[cfg(test)]
pub mod simulation;
pub mod sizing;
pub mod state;
pub mod state_machine;

pub use config::EquityConfig;
pub use engine::{evaluate, TradeDecision, TradeProposal};
pub use error::{DecisionReason, EquityError};
pub use risk_engine::RiskResult;
pub use sizing::{compute_size, SizingInput, SizingResult};
pub use state::{AccountPhase, AccountState, EquitySnapshot, ParsedSnapshot};
