//! Live position risk calculator.
//!
//! Two modes:
//!   - "advanced": user supplies contract_size, value_per_point, point_size
//!     (numbers from their broker).
//!   - "simple":   user supplies position size in lots and the margin
//!     the broker holds for that position. Everything else is derived.
//!
//! This is a *calculator*, not enforcement.

use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;
use serde::{Deserialize, Serialize};
use thiserror::Error;

fn round2(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalcMode {
    Simple,
    Advanced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveCalcInput {
    pub mode: CalcMode,

    // Always required
    pub size: Decimal,
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub take_profit: Option<Decimal>,
    pub balance: Decimal,

    // Required for Advanced mode
    pub contract_size: Option<Decimal>,
    pub value_per_point: Option<Decimal>,
    pub point_size: Option<Decimal>,
    pub leverage: Option<Decimal>,

    // Required for Simple mode
    pub margin_usd: Option<Decimal>,
    pub value_per_price_unit: Option<Decimal>,

    // Optional: costs (per 1.0 lot, round trip)
    pub commission_per_lot: Option<Decimal>,
    pub spread_points: Option<Decimal>,

    // Optional: quantity validation
    pub min_quantity: Option<Decimal>,
    pub max_quantity: Option<Decimal>,
    pub quantity_step: Option<Decimal>,

    // Optional warning threshold
    pub max_risk_per_trade_pct: Option<Decimal>,
    pub risk_mode: Option<RiskMode>,        // pct (default) | fixed
    pub fixed_risk_amount: Option<Decimal>, // used when risk_mode = fixed
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskMode {
    Pct,
    Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveCalcResult {
    pub stop_distance_price: Decimal,
    pub risk_per_lot: Decimal,
    pub total_risk: Decimal, // gross risk at stop
    pub risk_pct: Decimal,   // gross risk % of balance

    pub commission_total: Decimal, // round-trip commission across all exits
    pub spread_cost: Decimal,      // spread cost in account currency
    pub total_costs: Decimal,      // commission + spread

    pub net_risk: Decimal,     // gross risk + costs
    pub net_risk_pct: Decimal, // net risk % of balance

    pub reward_per_lot: Option<Decimal>,
    pub total_reward: Option<Decimal>, // gross reward at TP
    pub net_reward: Option<Decimal>,   // gross reward - costs
    pub planned_rr: Option<Decimal>,   // net reward / net risk

    pub notional: Decimal,
    pub required_margin: Decimal,
    pub free_margin_after: Decimal,
    pub margin_used_pct: Decimal, // required_margin / balance * 100

    pub warnings: Vec<String>,
    pub verdict: String,
}

#[derive(Debug, Error)]
pub enum LiveCalcError {
    #[error("size must be > 0")]
    InvalidSize,
    #[error("balance must be > 0")]
    InvalidBalance,
    #[error("stop must differ from entry")]
    InvalidStop,
    #[error("take profit on wrong side of entry for {0} trade")]
    InvalidTpDirection(String),
    #[error("advanced mode needs contract_size, value_per_point, point_size, leverage")]
    MissingAdvanced,
    #[error("simple mode needs margin_usd and value_per_price_unit")]
    MissingSimple,
    #[error("all inputs must be > 0")]
    InvalidInput,
    #[error("quantity {0} is below minimum {1}")]
    QuantityBelowMin(Decimal, Decimal),
    #[error("quantity {0} is above maximum {1}")]
    QuantityAboveMax(Decimal, Decimal),
    #[error("quantity {0} is not a multiple of step {1}")]
    QuantityNotOnStep(Decimal, Decimal),
}

pub fn calculate(i: &LiveCalcInput) -> Result<LiveCalcResult, LiveCalcError> {
    if i.size <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidSize);
    }
    if i.balance <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidBalance);
    }

    // --- Quantity validation ---
    if let Some(min) = i.min_quantity {
        if i.size < min {
            return Err(LiveCalcError::QuantityBelowMin(i.size, min));
        }
    }
    if let Some(max) = i.max_quantity {
        if i.size > max {
            return Err(LiveCalcError::QuantityAboveMax(i.size, max));
        }
    }
    if let Some(step) = i.quantity_step {
        if step > Decimal::ZERO {
            let q =
                (i.size / step).round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
            let on_step = q * step;
            if on_step != i.size {
                return Err(LiveCalcError::QuantityNotOnStep(i.size, step));
            }
        }
    }

    let stop_distance_price = (i.entry_price - i.stop_price).abs();
    if stop_distance_price.is_zero() {
        return Err(LiveCalcError::InvalidStop);
    }

    // --- Direction validation for TP ---
    let is_long = match i.take_profit {
        Some(tp) => {
            let bullish = tp > i.entry_price;
            let stop_below = i.stop_price < i.entry_price;
            if bullish != stop_below {
                // entry-to-TP and entry-to-stop on opposite sides would be a
                // long with TP below entry (nonsense) OR a short with TP above.
                // Infer side from stop direction:
                let inferred_long = stop_below;
                if inferred_long && tp <= i.entry_price {
                    return Err(LiveCalcError::InvalidTpDirection("long".into()));
                }
                if !inferred_long && tp >= i.entry_price {
                    return Err(LiveCalcError::InvalidTpDirection("short".into()));
                }
            }
            stop_below
        }
        None => i.stop_price < i.entry_price,
    };
    let _ = is_long;

    // --- Value per price unit + required margin ---
    let (value_per_price_unit, required_margin, notional) = match i.mode {
        CalcMode::Simple => {
            let v = i.value_per_price_unit.ok_or(LiveCalcError::MissingSimple)?;
            let m = i.margin_usd.ok_or(LiveCalcError::MissingSimple)?;
            if v <= Decimal::ZERO || m <= Decimal::ZERO {
                return Err(LiveCalcError::InvalidInput);
            }
            // In simple mode we do not know leverage; margin is what the broker reports.
            // Notional is intentionally marked as unavailable (returned as margin for
            // reference only).
            (v, m, m)
        }
        CalcMode::Advanced => {
            let contract = i.contract_size.ok_or(LiveCalcError::MissingAdvanced)?;
            let vpp = i.value_per_point.ok_or(LiveCalcError::MissingAdvanced)?;
            let ps = i.point_size.ok_or(LiveCalcError::MissingAdvanced)?;
            let lev = i.leverage.ok_or(LiveCalcError::MissingAdvanced)?;
            if contract <= Decimal::ZERO
                || vpp <= Decimal::ZERO
                || ps <= Decimal::ZERO
                || lev <= Decimal::ZERO
            {
                return Err(LiveCalcError::InvalidInput);
            }
            let v = vpp / ps;
            let notional = i.size * contract * i.entry_price;
            let margin = notional / lev;
            (v, margin, notional)
        }
    };

    // --- Gross risk ---
    let gross_risk = stop_distance_price * value_per_price_unit * i.size;
    let gross_risk_pct = (gross_risk / i.balance) * Decimal::from(100);

    // --- Costs ---
    let commission = i.commission_per_lot.unwrap_or(Decimal::ZERO) * i.size;
    let spread_cost = if let Some(sp) = i.spread_points {
        // spread_cost = spread (in points) * value_per_point * size
        // But value_per_point is only available in Advanced mode.
        // In Simple mode we can approximate using point size via value_per_price_unit.
        match i.mode {
            CalcMode::Advanced => {
                let vpp = i.value_per_point.unwrap_or(Decimal::ZERO);
                sp * vpp * i.size
            }
            CalcMode::Simple => {
                // Without point_size we cannot convert spread points to price.
                // Return zero and add a warning below.
                Decimal::ZERO
            }
        }
    } else {
        Decimal::ZERO
    };
    let total_costs = commission + spread_cost;
    let net_risk = gross_risk + total_costs;
    let net_risk_pct = (net_risk / i.balance) * Decimal::from(100);

    // --- Reward ---
    let (reward_per_lot, total_reward, net_reward, planned_rr) = match i.take_profit {
        Some(tp) => {
            let tp_dist = (tp - i.entry_price).abs();
            let per_lot = tp_dist * value_per_price_unit;
            let total = per_lot * i.size;
            let net = total - total_costs;
            let rr = if net_risk.is_zero() {
                None
            } else {
                Some(net / net_risk)
            };
            (Some(per_lot), Some(total), Some(net), rr)
        }
        None => (None, None, None, None),
    };

    let risk_per_lot = if i.size.is_zero() {
        Decimal::ZERO
    } else {
        gross_risk / i.size
    };

    let free_margin_after = i.balance - required_margin;
    let margin_used_pct = (required_margin / i.balance) * Decimal::from(100);

    // --- Warnings ---
    let mut warnings: Vec<String> = Vec::new();
    let mut verdict = "ok";

    // Risk warning
    let risk_for_cap = match i.risk_mode.unwrap_or(RiskMode::Pct) {
        RiskMode::Pct => {
            if let Some(cap) = i.max_risk_per_trade_pct {
                if !cap.is_zero() && net_risk_pct > cap {
                    warnings.push(format!(
                        "This trade risks {}% (net) of your account, above your {}% limit.",
                        round2(net_risk_pct),
                        round2(cap)
                    ));
                    verdict = "danger";
                }
            }
            net_risk_pct
        }
        RiskMode::Fixed => {
            if let Some(fixed) = i.fixed_risk_amount {
                if !fixed.is_zero() && net_risk > fixed {
                    warnings.push(format!(
                        "This trade risks {} (net), above your fixed limit of {}.",
                        round2(net_risk),
                        round2(fixed)
                    ));
                    verdict = "danger";
                }
            }
            net_risk_pct
        }
    };
    let _ = risk_for_cap;

    // Margin warning
    if !required_margin.is_zero() && required_margin > i.balance {
        warnings.push(format!(
            "Margin required ({}) is larger than your balance ({}).",
            round2(required_margin),
            round2(i.balance)
        ));
        verdict = "danger";
    } else if !required_margin.is_zero() && free_margin_after < i.balance * Decimal::new(2, 1) {
        warnings.push(format!(
            "Only {} free margin left after opening this position.",
            round2(free_margin_after)
        ));
        if verdict == "ok" {
            verdict = "warn";
        }
    }

    // Spread cost warning in Simple mode
    if i.spread_points.is_some() && i.mode == CalcMode::Simple {
        warnings.push(
            "Spread cost not included because Simple mode lacks point_size. Use Advanced mode to include spread.".into()
        );
        if verdict == "ok" {
            verdict = "warn";
        }
    }

    // R:R warning
    if let Some(rr) = planned_rr {
        if rr < Decimal::ONE {
            warnings.push(format!(
                "Planned R:R is 1 : {} — net reward is smaller than net risk.",
                round2(rr)
            ));
            if verdict == "ok" {
                verdict = "warn";
            }
        }
    }

    Ok(LiveCalcResult {
        stop_distance_price,
        risk_per_lot: round2(risk_per_lot),
        total_risk: round2(gross_risk),
        risk_pct: round2(gross_risk_pct),
        commission_total: round2(commission),
        spread_cost: round2(spread_cost),
        total_costs: round2(total_costs),
        net_risk: round2(net_risk),
        net_risk_pct: round2(net_risk_pct),
        reward_per_lot: reward_per_lot.map(round2),
        total_reward: total_reward.map(round2),
        net_reward: net_reward.map(round2),
        planned_rr: planned_rr.map(round2),
        notional: round2(notional),
        required_margin: round2(required_margin),
        free_margin_after: round2(free_margin_after),
        margin_used_pct: round2(margin_used_pct),
        warnings,
        verdict: verdict.to_string(),
    })
}

// ============================================================
// LIVE TRADE LAB — interactive calculators
// ============================================================

/// Partial close: how much profit is locked if we close X% of a position
/// at a given favorable price move?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialCloseInput {
    pub size: Decimal,
    pub entry_price: Decimal,
    pub current_price: Decimal,
    pub value_per_price_unit: Decimal, // $ per 1.0 price unit for 1.0 size
    pub close_fraction: Decimal,       // 0.5 = close half
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialCloseResult {
    pub locked_profit: Decimal,
    pub remaining_size: Decimal,
    pub remaining_potential_per_1unit: Decimal,
    pub be_price_after: Decimal, // price where remaining is at breakeven vs original entry
}

pub fn partial_close(i: &PartialCloseInput) -> Result<PartialCloseResult, LiveCalcError> {
    if i.size <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidSize);
    }
    if i.value_per_price_unit <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidInput);
    }
    if i.close_fraction <= Decimal::ZERO || i.close_fraction > Decimal::ONE {
        return Err(LiveCalcError::InvalidInput);
    }

    let move_amt = (i.current_price - i.entry_price).abs();
    let direction = if i.current_price >= i.entry_price {
        Decimal::ONE
    } else {
        -Decimal::ONE
    };
    let closed_size = i.size * i.close_fraction;
    let locked_profit = move_amt * i.value_per_price_unit * closed_size;
    let remaining_size = i.size - closed_size;
    let remaining_potential_per_1unit = i.value_per_price_unit * remaining_size;

    // Breakeven for remaining: if locked_profit covers any potential loss on remainder,
    // then BE is at entry. Otherwise BE is entry ± (locked_profit / (value_per_price_unit * remaining_size)).
    let be_move = if remaining_size.is_zero() || i.value_per_price_unit.is_zero() {
        Decimal::ZERO
    } else {
        locked_profit / (i.value_per_price_unit * remaining_size)
    };
    let be_price_after = i.entry_price - direction * be_move;

    Ok(PartialCloseResult {
        locked_profit: round2(locked_profit),
        remaining_size: round2(remaining_size),
        remaining_potential_per_1unit: round2(remaining_potential_per_1unit),
        be_price_after: round2(be_price_after),
    })
}

/// Martingale detector: compare current size to previous losing trade size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartingaleCheckInput {
    pub current_size: Decimal,
    pub last_losing_size: Option<Decimal>,
    pub last_losing_was_recent: bool, // within last 24h / this session
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MartingaleCheckResult {
    pub multiplier: Option<Decimal>,
    pub is_risky: bool,
    pub verdict: String, // "ok" | "warn" | "danger"
    pub message: String,
}

pub fn martingale_check(i: &MartingaleCheckInput) -> MartingaleCheckResult {
    let Some(prev) = i.last_losing_size else {
        return MartingaleCheckResult {
            multiplier: None,
            is_risky: false,
            verdict: "ok".into(),
            message: "No recent losing trade to compare with.".into(),
        };
    };
    if prev.is_zero() {
        return MartingaleCheckResult {
            multiplier: None,
            is_risky: false,
            verdict: "ok".into(),
            message: "Previous size was zero — comparison skipped.".into(),
        };
    }
    let mult = i.current_size / prev;
    if !i.last_losing_was_recent {
        return MartingaleCheckResult {
            multiplier: Some(round2(mult)),
            is_risky: false,
            verdict: "ok".into(),
            message: "Last loss was not recent. Normal comparison only.".into(),
        };
    }
    let (verdict, message) = if mult >= Decimal::from(2) {
        (
            "danger",
            format!(
                "Current size is {}× the last losing size. This is a classic Martingale pattern.",
                round2(mult)
            ),
        )
    } else if mult > Decimal::new(15, 1) {
        ("warn", format!("Current size is {}× the last losing size. Increasing size after a loss raises ruin risk.", round2(mult)))
    } else {
        ("ok", "Size is not larger than the previous loss.".into())
    };
    MartingaleCheckResult {
        multiplier: Some(round2(mult)),
        is_risky: verdict != "ok",
        verdict: verdict.into(),
        message,
    }
}

/// Break-even: what price makes the remaining position breakeven after partial close?
/// (See `partial_close`'s be_price_after — that's the calculator.)
/// This helper answers: what price do I move stop to if I want to risk zero more?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakEvenInput {
    pub entry_price: Decimal,
    pub side: String,           // "long" | "short"
    pub locked_profit: Decimal, // $ already banked from partial close
    pub remaining_size: Decimal,
    pub value_per_price_unit: Decimal, // $ per 1.0 price unit for 1.0 size
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakEvenResult {
    pub be_price: Decimal,
    pub move_from_entry: Decimal,
}

pub fn break_even(i: &BreakEvenInput) -> Result<BreakEvenResult, LiveCalcError> {
    if i.remaining_size <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidSize);
    }
    if i.value_per_price_unit <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidInput);
    }
    let is_long = i.side == "long";
    let per_unit_remaining = i.value_per_price_unit * i.remaining_size;
    if per_unit_remaining.is_zero() {
        return Err(LiveCalcError::InvalidInput);
    }
    let be_move = i.locked_profit / per_unit_remaining;
    let be_price = if is_long {
        i.entry_price - be_move
    } else {
        i.entry_price + be_move
    };
    Ok(BreakEvenResult {
        be_price: round2(be_price),
        move_from_entry: round2(be_move),
    })
}

/// Reverse: given a desired risk % and a stop distance, what size should I take?
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeFromRiskInput {
    pub balance: Decimal,
    pub risk_pct: Decimal, // 1 = 1%
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub value_per_price_unit: Decimal, // $ per 1.0 price unit for 1.0 size
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeFromRiskResult {
    pub risk_amount: Decimal,
    pub size_lots: Decimal,
    pub stop_distance: Decimal,
}

pub fn size_from_risk(i: &SizeFromRiskInput) -> Result<SizeFromRiskResult, LiveCalcError> {
    if i.balance <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidBalance);
    }
    if i.risk_pct <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidInput);
    }
    if i.value_per_price_unit <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidInput);
    }
    let stop_dist = (i.entry_price - i.stop_price).abs();
    if stop_dist.is_zero() {
        return Err(LiveCalcError::InvalidStop);
    }
    let risk_amount = i.balance * (i.risk_pct / Decimal::from(100));
    let denom = stop_dist * i.value_per_price_unit;
    if denom.is_zero() {
        return Err(LiveCalcError::InvalidInput);
    }
    let size = risk_amount / denom;
    Ok(SizeFromRiskResult {
        risk_amount: round2(risk_amount),
        size_lots: round2(size),
        stop_distance: round2(stop_dist),
    })
}

// ============================================================
// ASSISTANT — holistic advice for a single setup
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantInput {
    pub size: Decimal,
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub take_profit: Option<Decimal>,
    pub balance: Decimal,
    pub value_per_price_unit: Decimal,
    pub max_risk_pct: Decimal, // user's per-trade cap
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advice {
    pub kind: String,     // "risk" | "size" | "rr" | "partial" | "ok"
    pub severity: String, // "info" | "warn" | "danger"
    pub message: String,
    /// Optional suggested value the user can apply (e.g. suggested size).
    pub suggested_size: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantResult {
    pub verdict: String, // "ok" | "warn" | "danger"
    pub headline: String,
    pub current_risk_pct: Decimal,
    pub suggested_size: Option<Decimal>,
    pub advices: Vec<Advice>,
}

pub fn assistant(a: &AssistantInput) -> Result<AssistantResult, LiveCalcError> {
    if a.balance <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidBalance);
    }
    if a.value_per_price_unit <= Decimal::ZERO {
        return Err(LiveCalcError::InvalidInput);
    }

    let stop_dist = (a.entry_price - a.stop_price).abs();
    if stop_dist.is_zero() {
        return Err(LiveCalcError::InvalidStop);
    }

    let risk_amount = stop_dist * a.value_per_price_unit * a.size;
    let risk_pct = (risk_amount / a.balance) * Decimal::from(100);

    // Suggested size that would exactly meet max_risk_pct
    let target_risk_amount = a.balance * (a.max_risk_pct / Decimal::from(100));
    let denom = stop_dist * a.value_per_price_unit;
    let suggested_size = if denom.is_zero() {
        None
    } else {
        Some(round2(target_risk_amount / denom))
    };

    let mut advices: Vec<Advice> = Vec::new();
    let mut verdict = "ok";
    let mut headline = "Setup looks reasonable".to_string();

    if a.max_risk_pct > Decimal::ZERO && risk_pct > a.max_risk_pct {
        verdict = "danger";
        headline = format!(
            "Risking {}% — above your {}% cap",
            round2(risk_pct),
            round2(a.max_risk_pct)
        );
        advices.push(Advice {
            kind: "risk".into(),
            severity: "danger".into(),
            message: format!(
                "This setup risks ${} which is {}% of your balance. Your per-trade cap is {}%.",
                round2(risk_amount),
                round2(risk_pct),
                round2(a.max_risk_pct)
            ),
            suggested_size: None,
        });
        if let Some(s) = suggested_size {
            advices.push(Advice {
                kind: "size".into(),
                severity: "info".into(),
                message: format!(
                    "To respect your {}% cap, use {} lots instead.",
                    round2(a.max_risk_pct),
                    s
                ),
                suggested_size: Some(s),
            });
        }
    } else if a.max_risk_pct > Decimal::ZERO && risk_pct > a.max_risk_pct * Decimal::new(7, 1) {
        verdict = "warn";
        headline = format!("Approaching your risk cap ({}%)", round2(risk_pct));
        advices.push(Advice {
            kind: "risk".into(),
            severity: "warn".into(),
            message: format!(
                "Risk is {}% of balance — within cap but close to {}%.",
                round2(risk_pct),
                round2(a.max_risk_pct)
            ),
            suggested_size: None,
        });
    }

    // R:R advice
    if let Some(tp) = a.take_profit {
        let tp_dist = (tp - a.entry_price).abs();
        let reward = tp_dist * a.value_per_price_unit * a.size;
        if !risk_amount.is_zero() {
            let rr = reward / risk_amount;
            if rr < Decimal::ONE {
                if verdict == "ok" {
                    verdict = "warn";
                }
                advices.push(Advice {
                    kind: "rr".into(),
                    severity: "warn".into(),
                    message: format!(
                        "Planned R:R is 1 : {} — the potential reward is smaller than the risk.",
                        round2(rr)
                    ),
                    suggested_size: None,
                });
                if headline == "Setup looks reasonable" {
                    headline = "Reward smaller than risk".into();
                }
            } else if rr >= Decimal::from(2) {
                advices.push(Advice {
                    kind: "rr".into(),
                    severity: "info".into(),
                    message: format!("Planned R:R is 1 : {} — favorable.", round2(rr)),
                    suggested_size: None,
                });
            }
        }
    } else {
        advices.push(Advice {
            kind: "rr".into(),
            severity: "info".into(),
            message: "Add a take-profit to see the reward and R:R of this setup.".into(),
            suggested_size: None,
        });
    }

    // Partial-close advice (if in profit and something to lock)
    if let Some(tp) = a.take_profit {
        let direction = if tp > a.entry_price {
            Decimal::ONE
        } else {
            -Decimal::ONE
        };
        let halfway = a.entry_price + direction * ((tp - a.entry_price) / Decimal::from(2));
        advices.push(Advice {
            kind: "partial".into(),
            severity: "info".into(),
            message: format!(
                "If price reaches {} (halfway to TP), closing 50% locks ~${} and lets the rest run risk-free.",
                round2(halfway),
                round2(((halfway - a.entry_price).abs() * a.value_per_price_unit * a.size) / Decimal::from(2))
            ),
            suggested_size: None,
        });
    }

    if advices.iter().all(|x| x.severity == "info") && verdict == "ok" {
        advices.push(Advice {
            kind: "ok".into(),
            severity: "info".into(),
            message: "Risk, size, and R:R are all within your stated limits.".into(),
            suggested_size: None,
        });
    }

    Ok(AssistantResult {
        verdict: verdict.into(),
        headline,
        current_risk_pct: round2(risk_pct),
        suggested_size,
        advices,
    })
}

// ============================================================
// PORTFOLIO ASSISTANT
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradeInput {
    pub symbol: String,
    pub side: String,
    pub size: Decimal,
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub value_per_price_unit: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskInput {
    pub trades: Vec<PortfolioTradeInput>,
    pub balance: Decimal,
    pub max_portfolio_risk_pct: Decimal,
    pub max_open_positions: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioTradeResult {
    pub symbol: String,
    pub side: String,
    pub size: Decimal,
    pub risk: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRiskResult {
    pub positions: i64,
    pub total_risk: Decimal,
    pub total_risk_pct: Decimal,
    pub per_trade: Vec<PortfolioTradeResult>,
    pub positions_limit_hit: bool,
    pub portfolio_limit_hit: bool,
    pub warnings: Vec<String>,
    pub verdict: String,
    pub advices: Vec<String>,
    pub suggested_trim_pct: Option<Decimal>,
}

pub fn portfolio_risk(i: &PortfolioRiskInput) -> PortfolioRiskResult {
    let mut per_trade = Vec::new();
    let mut total_risk = Decimal::ZERO;

    for t in &i.trades {
        let stop_dist = (t.entry_price - t.stop_price).abs();
        let risk = stop_dist * t.value_per_price_unit * t.size;
        total_risk += risk;
        per_trade.push(PortfolioTradeResult {
            symbol: t.symbol.clone(),
            side: t.side.clone(),
            size: t.size,
            risk: round2(risk),
        });
    }

    let total_risk_pct = if i.balance.is_zero() {
        Decimal::ZERO
    } else {
        round2((total_risk / i.balance) * Decimal::from(100))
    };

    let positions = i.trades.len() as i64;
    let positions_limit_hit = i.max_open_positions > 0 && positions >= i.max_open_positions;
    let portfolio_limit_hit =
        i.max_portfolio_risk_pct > Decimal::ZERO && total_risk_pct > i.max_portfolio_risk_pct;

    let mut warnings: Vec<String> = Vec::new();
    let mut verdict = "ok";

    if positions_limit_hit {
        warnings.push(format!(
            "You have {} open positions — at or above your limit of {}.",
            positions, i.max_open_positions
        ));
        verdict = "danger";
    }
    if portfolio_limit_hit {
        warnings.push(format!(
            "Total risk across all positions is {}% — above your {}% portfolio cap.",
            total_risk_pct, i.max_portfolio_risk_pct
        ));
        verdict = "danger";
    } else if i.max_portfolio_risk_pct > Decimal::ZERO
        && total_risk_pct > i.max_portfolio_risk_pct * Decimal::new(7, 1)
    {
        warnings.push(format!(
            "Total risk is {}% — approaching your {}% portfolio cap.",
            total_risk_pct, i.max_portfolio_risk_pct
        ));
        if verdict == "ok" {
            verdict = "warn";
        }
    }

    let mut advices: Vec<String> = Vec::new();
    let mut suggested_trim_pct: Option<Decimal> = None;

    if portfolio_limit_hit {
        if !total_risk_pct.is_zero() && i.max_portfolio_risk_pct > Decimal::ZERO {
            let excess_pct = total_risk_pct - i.max_portfolio_risk_pct;
            let trim_pct = (excess_pct / total_risk_pct) * Decimal::from(100);
            suggested_trim_pct = Some(round2(trim_pct));
            advices.push(format!(
                "To respect your {}% portfolio cap, close or trim about {}% of total exposure.",
                round2(i.max_portfolio_risk_pct),
                round2(trim_pct)
            ));
        }
    }

    if !positions_limit_hit && !portfolio_limit_hit && !total_risk.is_zero() {
        advices.push(
            "Portfolio risk is within limits. Keep monitoring before adding new positions.".into(),
        );
    }

    if i.max_portfolio_risk_pct.is_zero() {
        advices.push("Set a portfolio risk cap in Risk settings to get warnings here.".into());
    }

    if !total_risk_pct.is_zero() && total_risk_pct > Decimal::from(2) {
        advices.push(format!(
            "Total risk of {}% exceeds the common 2% ceiling for aggregate exposure.",
            round2(total_risk_pct)
        ));
    }

    let zero_risk = per_trade.iter().filter(|t| t.risk.is_zero()).count();
    if zero_risk > 0 {
        advices.push(format!(
            "{} positions have zero risk — check their stops.",
            zero_risk
        ));
    }

    PortfolioRiskResult {
        positions,
        total_risk: round2(total_risk),
        total_risk_pct,
        per_trade,
        positions_limit_hit,
        portfolio_limit_hit,
        warnings,
        verdict: verdict.to_string(),
        advices,
        suggested_trim_pct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ============================================================
    // Test fixtures
    // ============================================================

    fn simple() -> LiveCalcInput {
        LiveCalcInput {
            mode: CalcMode::Simple,
            size: dec!(1),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            take_profit: Some(dec!(1.1150)),
            balance: dec!(10000),
            contract_size: None,
            value_per_point: None,
            point_size: None,
            leverage: None,
            margin_usd: Some(dec!(1000)),
            value_per_price_unit: Some(dec!(100000)),
            commission_per_lot: None,
            spread_points: None,
            min_quantity: None,
            max_quantity: None,
            quantity_step: None,
            max_risk_per_trade_pct: Some(dec!(1)),
            risk_mode: None,
            fixed_risk_amount: None,
        }
    }

    fn advanced() -> LiveCalcInput {
        LiveCalcInput {
            mode: CalcMode::Advanced,
            size: dec!(1),
            entry_price: dec!(1.1000),
            stop_price: dec!(1.0950),
            take_profit: Some(dec!(1.1150)),
            balance: dec!(10000),
            contract_size: Some(dec!(100000)),
            value_per_point: Some(dec!(10)),
            point_size: Some(dec!(0.0001)),
            leverage: Some(dec!(100)),
            margin_usd: None,
            value_per_price_unit: None,
            commission_per_lot: None,
            spread_points: None,
            min_quantity: None,
            max_quantity: None,
            quantity_step: None,
            max_risk_per_trade_pct: Some(dec!(1)),
            risk_mode: None,
            fixed_risk_amount: None,
        }
    }

    // ============================================================
    // Basic calculation tests
    // ============================================================

    #[test]
    fn simple_mode_matches_broker_margin() {
        let r = calculate(&simple()).unwrap();
        assert_eq!(r.required_margin, dec!(1000));
        assert_eq!(r.total_risk, dec!(500));
        assert_eq!(r.risk_pct, dec!(5.00));
        assert_eq!(r.planned_rr, Some(dec!(3.00)));
    }

    #[test]
    fn advanced_mode_equivalent() {
        let s = calculate(&simple()).unwrap();
        let a = calculate(&advanced()).unwrap();
        // Risk and R:R should match across modes
        assert_eq!(s.net_risk, a.net_risk);
        assert_eq!(s.planned_rr, a.planned_rr);
        // Margin differs: simple is user-reported, advanced is computed
        assert!(a.required_margin > Decimal::ZERO);
        assert!(s.required_margin > Decimal::ZERO);
    }

    #[test]
    fn rejects_zero_size() {
        let mut i = simple();
        i.size = Decimal::ZERO;
        assert!(calculate(&i).is_err());
    }

    #[test]
    fn warns_above_limit() {
        let r = calculate(&simple()).unwrap();
        assert_eq!(r.verdict, "danger");
    }

    // ============================================================
    // Cost tests
    // ============================================================

    #[test]
    fn commission_reduces_net_reward() {
        let mut i = simple();
        i.commission_per_lot = Some(dec!(10));
        let r = calculate(&i).unwrap();
        assert_eq!(r.commission_total, dec!(10));
        // gross risk 500 + 10 commission = net 510
        assert_eq!(r.net_risk, dec!(510));
        // gross reward 1500 - 10 = 1490
        assert_eq!(r.net_reward, Some(dec!(1490)));
        // R:R = 1490 / 510 ≈ 2.92
        let rr = r.planned_rr.unwrap();
        assert!(rr > dec!(2.9) && rr < dec!(3.0));
    }

    // ============================================================
    // Risk mode tests
    // ============================================================

    #[test]
    fn fixed_risk_cap_warning() {
        let mut i = simple();
        i.risk_mode = Some(RiskMode::Fixed);
        i.fixed_risk_amount = Some(dec!(300)); // 500 net risk > 300 → danger
        i.max_risk_per_trade_pct = None;
        let r = calculate(&i).unwrap();
        assert_eq!(r.verdict, "danger");
        assert!(r.warnings.iter().any(|w| w.contains("fixed limit")));
    }

    // ============================================================
    // Direction validation
    // ============================================================

    #[test]
    fn rejects_tp_on_wrong_side_long() {
        let mut i = simple();
        // long (stop below entry) but TP below entry
        i.take_profit = Some(dec!(1.0900));
        assert!(matches!(
            calculate(&i),
            Err(LiveCalcError::InvalidTpDirection(_))
        ));
    }

    #[test]
    fn rejects_tp_on_wrong_side_short() {
        let mut i = simple();
        // short (stop above entry) but TP above entry
        i.entry_price = dec!(1.1000);
        i.stop_price = dec!(1.1050);
        i.take_profit = Some(dec!(1.1200));
        assert!(matches!(
            calculate(&i),
            Err(LiveCalcError::InvalidTpDirection(_))
        ));
    }

    // ============================================================
    // Quantity validation
    // ============================================================

    #[test]
    fn quantity_below_min_rejected() {
        let mut i = simple();
        i.size = dec!(0.05);
        i.min_quantity = Some(dec!(0.1));
        assert!(matches!(
            calculate(&i),
            Err(LiveCalcError::QuantityBelowMin(_, _))
        ));
    }

    #[test]
    fn quantity_above_max_rejected() {
        let mut i = simple();
        i.size = dec!(10);
        i.max_quantity = Some(dec!(5));
        assert!(matches!(
            calculate(&i),
            Err(LiveCalcError::QuantityAboveMax(_, _))
        ));
    }

    #[test]
    fn quantity_step_enforced() {
        let mut i = simple();
        i.size = dec!(0.55);
        i.quantity_step = Some(dec!(0.1));
        assert!(matches!(
            calculate(&i),
            Err(LiveCalcError::QuantityNotOnStep(_, _))
        ));
    }

    #[test]
    fn quantity_step_accepts_exact_multiple() {
        let mut i = simple();
        i.size = dec!(0.5);
        i.quantity_step = Some(dec!(0.1));
        let r = calculate(&i).unwrap();
        assert!(r.total_risk > Decimal::ZERO);
    }

    // ============================================================
    // Margin label correctness
    // ============================================================

    #[test]
    fn margin_used_pct_is_broker_side() {
        // 1000 margin on 10000 balance → 10% used
        let r = calculate(&simple()).unwrap();
        assert_eq!(r.margin_used_pct, dec!(10.00));
    }

    // ============================================================
    // Lab: partial close
    // ============================================================

    #[test]
    fn partial_close_locks_profit() {
        let r = partial_close(&PartialCloseInput {
            size: dec!(1),
            entry_price: dec!(1.10),
            current_price: dec!(1.12),
            value_per_price_unit: dec!(100000),
            close_fraction: dec!(0.5),
        })
        .unwrap();
        assert_eq!(r.locked_profit, dec!(1000));
        assert_eq!(r.remaining_size, dec!(0.5));
    }

    // ============================================================
    // Lab: martingale
    // ============================================================

    #[test]
    fn martingale_detects_doubling() {
        let r = martingale_check(&MartingaleCheckInput {
            current_size: dec!(2),
            last_losing_size: Some(dec!(1)),
            last_losing_was_recent: true,
        });
        assert_eq!(r.verdict, "danger");
        assert_eq!(r.multiplier, Some(dec!(2)));
    }

    #[test]
    fn martingale_ok_when_not_increasing() {
        let r = martingale_check(&MartingaleCheckInput {
            current_size: dec!(1),
            last_losing_size: Some(dec!(1)),
            last_losing_was_recent: true,
        });
        assert_eq!(r.verdict, "ok");
    }

    // ============================================================
    // Lab: break-even
    // ============================================================

    #[test]
    fn break_even_moves_stop_favorably() {
        let r = break_even(&BreakEvenInput {
            entry_price: dec!(1.10),
            side: "long".into(),
            locked_profit: dec!(1000),
            remaining_size: dec!(0.5),
            value_per_price_unit: dec!(100000),
        })
        .unwrap();
        assert_eq!(r.be_price, dec!(1.08));
    }

    // ============================================================
    // Lab: size from risk
    // ============================================================

    #[test]
    fn calc_size_from_risk() {
        let r = size_from_risk(&SizeFromRiskInput {
            balance: dec!(10000),
            risk_pct: dec!(1),
            entry_price: dec!(1.10),
            stop_price: dec!(1.09),
            value_per_price_unit: dec!(100000),
        })
        .unwrap();
        assert_eq!(r.risk_amount, dec!(100));
        assert_eq!(r.size_lots, dec!(0.10));
    }

    // ============================================================
    // Portfolio
    // ============================================================

    fn port_trade(symbol: &str, size: Decimal, stop_dist: Decimal) -> PortfolioTradeInput {
        PortfolioTradeInput {
            symbol: symbol.into(),
            side: "long".into(),
            size,
            entry_price: dec!(1.10),
            stop_price: dec!(1.10) - stop_dist,
            value_per_price_unit: dec!(100000),
        }
    }

    #[test]
    fn empty_portfolio() {
        let r = portfolio_risk(&PortfolioRiskInput {
            trades: vec![],
            balance: dec!(10000),
            max_portfolio_risk_pct: dec!(5),
            max_open_positions: 3,
        });
        assert_eq!(r.positions, 0);
        assert_eq!(r.total_risk, Decimal::ZERO);
        assert_eq!(r.verdict, "ok");
    }

    #[test]
    fn sum_risk_across_positions() {
        let r = portfolio_risk(&PortfolioRiskInput {
            trades: vec![
                port_trade("EURUSD", dec!(0.1), dec!(0.05)),
                port_trade("GBPUSD", dec!(0.1), dec!(0.05)),
            ],
            balance: dec!(10000),
            max_portfolio_risk_pct: dec!(5),
            max_open_positions: 3,
        });
        assert_eq!(r.positions, 2);
        assert_eq!(r.total_risk, dec!(1000));
        assert_eq!(r.total_risk_pct, dec!(10.00));
        assert!(r.portfolio_limit_hit);
    }

    #[test]
    fn position_limit() {
        let r = portfolio_risk(&PortfolioRiskInput {
            trades: vec![
                port_trade("A", dec!(0.1), dec!(0.01)),
                port_trade("B", dec!(0.1), dec!(0.01)),
                port_trade("C", dec!(0.1), dec!(0.01)),
            ],
            balance: dec!(10000),
            max_portfolio_risk_pct: dec!(10),
            max_open_positions: 3,
        });
        assert!(r.positions_limit_hit);
    }
}
