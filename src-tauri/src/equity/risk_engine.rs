//! Risk engine (audit §6, §8) + backward-compat shims (S4 → S5).
//!
//! NEW model (used by buffer.rs, protection.rs, prop_rules.rs):
//!   R_base = Equity * BaseRiskPercent
//!   R_target = R_base * M_preserve
//!   R_allowed = min(R_target, R_buffer, PortfolioRemaining, DailyRemaining, PropRemaining)
//!
//! LEGACY shims (compute_risk, m_dd, m_loss) are kept only so engine.rs
//! and commands/equity.rs can compile during the S4→S5 transition.
//! They will be removed in S5.

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

use super::config::EquityConfig;
use super::state::AccountState;

// ============================================================
// NEW model
// ============================================================

#[derive(Debug, Clone)]
pub struct RiskInput {
    pub equity: Decimal,
    pub r_base_pct: Decimal,
    pub m_preserve: Decimal,
    pub r_buffer: Decimal,
    pub r_portfolio_remaining: Decimal,
    pub r_daily_remaining: Decimal,
    pub r_prop_remaining: Decimal,
}

/// Unified result: contains BOTH the new fields (r_target, r_buffer, binding)
/// and the legacy fields (m_dd, m_loss, r_adaptive, r_floor_cap) so that
/// engine.rs and commands/equity.rs keep compiling until S5.
#[derive(Debug, Clone)]
pub struct RiskResult {
    // --- new ---
    pub r_base: Decimal,
    pub r_target: Decimal,
    pub r_buffer: Decimal,
    pub r_portfolio_cap: Decimal,
    pub r_daily_cap: Decimal,
    pub r_prop_cap: Decimal,
    pub r_allowed: Decimal,
    pub binding: &'static str,

    // --- legacy (S4 transition only) ---
    pub m_dd: Decimal,
    pub m_loss: Decimal,
    pub r_adaptive: Decimal,
    pub r_floor_cap: Decimal,
}

pub fn compute(i: &RiskInput) -> RiskResult {
    let r_base = i.equity * i.r_base_pct;
    let r_target = r_base * i.m_preserve;

    let mut r_allowed = r_target;
    let mut binding: &'static str = "Target";

    if i.r_buffer < r_allowed { r_allowed = i.r_buffer; binding = "Buffer"; }
    if i.r_portfolio_remaining < r_allowed { r_allowed = i.r_portfolio_remaining; binding = "Portfolio"; }
    if i.r_daily_remaining < r_allowed { r_allowed = i.r_daily_remaining; binding = "Daily"; }
    if i.r_prop_remaining < r_allowed { r_allowed = i.r_prop_remaining; binding = "Prop"; }

    if r_allowed < Decimal::ZERO { r_allowed = Decimal::ZERO; }

    RiskResult {
        r_base,
        r_target,
        r_buffer: i.r_buffer,
        r_portfolio_cap: i.r_portfolio_remaining,
        r_daily_cap: i.r_daily_remaining,
        r_prop_cap: i.r_prop_remaining,
        r_allowed,
        binding,
        m_dd: Decimal::ONE,
        m_loss: Decimal::ONE,
        r_adaptive: r_target,
        r_floor_cap: i.r_buffer,
    }
}

// ============================================================
// LEGACY shims (S4 → S5 transition). Do not add new callers.
// ============================================================

pub fn m_dd(_state: &AccountState, _cfg: &EquityConfig) -> Result<Decimal, String> {
    Ok(Decimal::ONE)
}

pub fn m_loss(_consecutive_losses: i64, _cfg: &EquityConfig) -> Result<Decimal, String> {
    Ok(Decimal::ONE)
}

pub fn compute_risk(
    state: &AccountState,
    cfg: &EquityConfig,
) -> Result<RiskResult, String> {
    let d = |s: &str, name: &str| -> Result<Decimal, String> {
        Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
    };
    let r0 = d(&cfg.r0, "r0")?;
    let alpha = d(&cfg.alpha, "alpha")?;
    let daily_limit = d(&cfg.daily_loss_limit, "daily_loss_limit")?;
    let portfolio_max = d(&cfg.r_portfolio_max, "r_portfolio_max")?;
    let prop_max = d(&cfg.r_prop_max, "r_prop_max")?;

    let r_base = state.snapshot.equity * r0;
    let r_target = r_base;

    let r_portfolio_cap = if portfolio_max.is_zero() {
        r_base
    } else {
        (portfolio_max - state.snapshot.open_portfolio_risk).max(Decimal::ZERO)
    };
    let r_daily_cap = if daily_limit.is_zero() {
        r_base
    } else {
        (daily_limit - state.snapshot.daily_loss).max(Decimal::ZERO)
    };
    let r_floor_cap = (state.loss_capacity * alpha).max(Decimal::ZERO);
    let r_prop_cap = if prop_max.is_zero() { r_base } else { prop_max };

    let r_allowed = r_target
        .min(r_portfolio_cap)
        .min(r_daily_cap)
        .min(r_floor_cap)
        .min(r_prop_cap);

    let mut binding: &'static str = "Target";
    if r_portfolio_cap <= r_allowed { binding = "Portfolio"; }
    if r_daily_cap < r_allowed { binding = "Daily"; }
    if r_floor_cap < r_allowed { binding = "Floor"; }
    if r_prop_cap < r_allowed { binding = "Prop"; }

    Ok(RiskResult {
        r_base,
        r_target,
        r_buffer: r_floor_cap,
        r_portfolio_cap,
        r_daily_cap,
        r_prop_cap,
        r_allowed,
        binding,
        m_dd: Decimal::ONE,
        m_loss: Decimal::ONE,
        r_adaptive: r_target,
        r_floor_cap,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base() -> RiskInput {
        RiskInput {
            equity: dec!(10000),
            r_base_pct: dec!(0.01),
            m_preserve: Decimal::ONE,
            r_buffer: dec!(1000),
            r_portfolio_remaining: dec!(1000),
            r_daily_remaining: dec!(1000),
            r_prop_remaining: dec!(1000),
        }
    }

    #[test]
    fn happy_path() {
        let r = compute(&base());
        assert_eq!(r.r_base, dec!(100));
        assert_eq!(r.r_target, dec!(100));
        assert_eq!(r.r_allowed, dec!(100));
        assert_eq!(r.binding, "Target");
    }

    #[test]
    fn m_preserve_halves() {
        let mut i = base();
        i.m_preserve = dec!(0.5);
        let r = compute(&i);
        assert_eq!(r.r_target, dec!(50));
        assert_eq!(r.r_allowed, dec!(50));
    }

    #[test]
    fn buffer_binds() {
        let mut i = base();
        i.r_buffer = dec!(30);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(30));
        assert_eq!(r.binding, "Buffer");
    }

    #[test]
    fn portfolio_binds() {
        let mut i = base();
        i.r_portfolio_remaining = dec!(20);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(20));
        assert_eq!(r.binding, "Portfolio");
    }

    #[test]
    fn daily_binds() {
        let mut i = base();
        i.r_daily_remaining = dec!(10);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(10));
        assert_eq!(r.binding, "Daily");
    }

    #[test]
    fn prop_binds() {
        let mut i = base();
        i.r_prop_remaining = dec!(5);
        let r = compute(&i);
        assert_eq!(r.r_allowed, dec!(5));
        assert_eq!(r.binding, "Prop");
    }
}
