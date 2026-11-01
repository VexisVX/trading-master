//! Dynamic risk engine — formulas 11–20 (see checklist v1.0).
//! ALL quantities are MONEY (Decimal). No percentages leave this module.
//! Formula 19's correction: R_allowed is money; RA = R_allowed (no × E_t).

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

use super::config::EquityConfig;
use super::state::AccountState;

/// Multiplier from drawdown tier (formula 12).
pub fn m_dd(state: &AccountState, cfg: &EquityConfig) -> Result<Decimal, String> {
    let d = |s: &str, name: &str| -> Result<Decimal, String> {
        Decimal::from_str(s).map_err(|e| format!("{name}: {e}"))
    };
    let dd_1 = d(&cfg.dd_1, "dd_1")?;
    let dd_2 = d(&cfg.dd_2, "dd_2")?;
    let dd_3 = d(&cfg.dd_3, "dd_3")?;
    let m1 = d(&cfg.m_dd_1, "m_dd_1")?;
    let m2 = d(&cfg.m_dd_2, "m_dd_2")?;
    let m3 = d(&cfg.m_dd_3, "m_dd_3")?;
    let m4 = d(&cfg.m_dd_4, "m_dd_4")?;
    let dd = state.dd_pct;
    Ok(if dd < dd_1 {
        m1
    } else if dd < dd_2 {
        m2
    } else if dd < dd_3 {
        m3
    } else {
        m4
    })
}

/// Multiplier from consecutive losses (formula 13).
pub fn m_loss(consecutive_losses: i64, cfg: &EquityConfig) -> Result<Decimal, String> {
    let table = cfg.loss_table()?;
    if table.is_empty() {
        return Err("loss_table is empty".into());
    }
    let idx = consecutive_losses.max(0) as usize;
    Ok(if idx >= table.len() {
        *table.last().unwrap()
    } else {
        table[idx]
    })
}

/// Result of risk computation.
#[derive(Debug, Clone)]
pub struct RiskResult {
    pub r_base: Decimal,          // money
    pub m_dd: Decimal,            // multiplier
    pub m_loss: Decimal,          // multiplier
    pub r_adaptive: Decimal,      // money
    pub r_portfolio_cap: Decimal, // money
    pub r_daily_cap: Decimal,     // money
    pub r_floor_cap: Decimal,     // money
    pub r_prop_cap: Decimal,      // money
    pub r_allowed: Decimal,       // money (may be <= 0 → reject)
}

/// Compute all risk components (formulas 11–19).
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

    // 11. R_base = E_t × R_0  (money)
    let r_base = state.snapshot.equity * r0;

    // 12-14
    let mdd = m_dd(state, cfg)?;
    let mloss = m_loss(state.snapshot.consecutive_losses, cfg)?;
    let r_adaptive = r_base * mdd * mloss;

    // 16. RPC = portfolio_max − portfolio_current
    let r_portfolio_cap = if portfolio_max.is_zero() {
        // disabled → no cap
        r_adaptive
    } else {
        (portfolio_max - state.snapshot.open_portfolio_risk).max(Decimal::ZERO)
    };

    // 17. DLC = daily_limit − daily_loss
    let r_daily_cap = if daily_limit.is_zero() {
        // disabled → no cap
        r_adaptive
    } else {
        (daily_limit - state.snapshot.daily_loss).max(Decimal::ZERO)
    };

    // 18. FRC = LC × alpha
    let r_floor_cap = (state.loss_capacity * alpha).max(Decimal::ZERO);

    // R_prop — per-trade prop-firm cap (0 = disabled → no cap)
    let r_prop_cap = if prop_max.is_zero() { r_adaptive } else { prop_max };

    // 19. R_allowed = min(...)  — all money
    let r_allowed = r_adaptive
        .min(r_portfolio_cap)
        .min(r_daily_cap)
        .min(r_floor_cap)
        .min(r_prop_cap);

    Ok(RiskResult {
        r_base,
        m_dd: mdd,
        m_loss: mloss,
        r_adaptive,
        r_portfolio_cap,
        r_daily_cap,
        r_floor_cap,
        r_prop_cap,
        r_allowed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::equity::config::EquityConfig;
    use crate::equity::state::ParsedSnapshot;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromStr;

    fn snap(equity: &str, initial: &str, hwm: &str, day_start: &str, losses: i64) -> ParsedSnapshot {
        ParsedSnapshot {
            id: 1,
            equity: Decimal::from_str(equity).unwrap(),
            initial_balance: Decimal::from_str(initial).unwrap(),
            high_watermark: Decimal::from_str(hwm).unwrap(),
            low_watermark: Decimal::from_str(equity).unwrap(),
            day_start_equity: Decimal::from_str(day_start).unwrap(),
            open_portfolio_risk: Decimal::ZERO,
            daily_loss: Decimal::ZERO,
            consecutive_losses: losses,
            taken_at: Utc::now(),
        }
    }

    #[test]
    fn m_dd_zero_drawdown_is_one() {
        let cfg = EquityConfig::default_v1();
        let st = AccountState::from_snapshot(snap("10000", "10000", "10000", "10000", 0), &cfg).unwrap();
        assert_eq!(m_dd(&st, &cfg).unwrap(), Decimal::ONE);
    }

    #[test]
    fn m_dd_tier_2() {
        let cfg = EquityConfig::default_v1();
        // HWM 10000, equity 9300 → DD 7% → tier 2 (dd_1=5%, dd_2=10%) → 0.75
        let st = AccountState::from_snapshot(snap("9300", "10000", "10000", "10000", 0), &cfg).unwrap();
        assert_eq!(m_dd(&st, &cfg).unwrap(), Decimal::from_str("0.75").unwrap());
    }

    #[test]
    fn m_loss_table() {
        let cfg = EquityConfig::default_v1();
        assert_eq!(m_loss(0, &cfg).unwrap(), Decimal::ONE);
        assert_eq!(m_loss(1, &cfg).unwrap(), Decimal::from_str("0.8").unwrap());
        assert_eq!(m_loss(3, &cfg).unwrap(), Decimal::from_str("0.4").unwrap());
        assert_eq!(m_loss(99, &cfg).unwrap(), Decimal::ZERO);
    }

    #[test]
    fn full_risk_happy_path() {
        // Equity 10000, no DD, no losses → R_base = 100 → R_adaptive = 100
        let cfg = EquityConfig::default_v1();
        let st = AccountState::from_snapshot(snap("10000", "10000", "10000", "10000", 0), &cfg).unwrap();
        let r = compute_risk(&st, &cfg).unwrap();
        assert_eq!(r.r_base, Decimal::from_str("100").unwrap());
        assert_eq!(r.r_allowed, Decimal::from_str("100").unwrap());
    }

    #[test]
    fn risk_clamped_by_loss_capacity() {
        // Force a tight floor: equity 10000, but HWM very high → floor above equity
        // Easier: use a big d_trailing so F_trail ≈ HWM*(1-d) is high
        let mut cfg = EquityConfig::default_v1();
        cfg.d_trailing = "0.02".into();  // 2%
        cfg.q_retention = "0.5".into();  // keep 50% of gains
        // Initial 10000, HWM 10500 → profit_retention = 250, f_profit = 10250
        // f_trail = 10500 * 0.98 = 10290
        // equity 10300 → LC = 10300 - 10290 = 10 → FRC = 5
        let st = AccountState::from_snapshot(snap("10300", "10000", "10500", "10000", 0), &cfg).unwrap();
        let r = compute_risk(&st, &cfg).unwrap();
        // r_adaptive ≈ 103, but floor_cap is small → r_allowed = 5
        assert!(r.r_allowed < Decimal::from_str("20").unwrap());
    }
}
