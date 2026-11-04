//! Monte Carlo simulation harness (audit §10). Deterministic seeds. No floats.
//! TESTING ONLY.

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct SimConfig {
    pub initial_equity: Decimal,
    pub r_base_pct: Decimal,
    pub fixed_r_pct: Decimal,
    pub trades: usize,
    pub win_rate: Decimal,
    pub rr: Decimal,
    pub seed: u64,
    pub pause_threshold_r: Decimal,
    pub resume_threshold_r: Decimal,
    pub m_preserve_floor: Decimal,
    pub m_preserve_ceiling: Decimal,
}

#[derive(Debug, Clone)]
pub struct SimResult {
    pub strategy: &'static str,
    pub final_equity: Decimal,
    pub max_dd_pct: Decimal,
    pub avg_r: Decimal,
    pub rejected_trades: usize,
    pub paused_trades: usize,
    pub wins: usize,
    pub losses: usize,
}

/// Xorshift64 PRNG. Distinct seeds produce distinct streams (unlike LCG
/// where certain seeds can collide under wrapping arithmetic).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        if x == 0 {
            x = 0x9E3779B97F4A7C15;
        } // avoid zero-state lockup
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    /// Returns a value in [0, 1) using the low 53 bits (exact in u64).
    fn next_dec(&mut self) -> Decimal {
        let v = self.next() & ((1u64 << 53) - 1);
        Decimal::from(v) / Decimal::from(1u64 << 53)
    }
}

pub fn run(cfg: &SimConfig, adaptive: bool) -> SimResult {
    let mut rng = Lcg(cfg.seed);
    let mut equity = cfg.initial_equity;
    let mut peak = equity;
    let mut max_dd = Decimal::ZERO;
    let mut sum_r = Decimal::ZERO;
    let mut wins = 0usize;
    let mut losses = 0usize;
    let mut rejected = 0usize;
    let mut paused = 0usize;
    let mut paused_state = false;

    for _ in 0..cfg.trades {
        let r_base = equity * cfg.r_base_pct;
        let buffer_r = if r_base.is_zero() {
            Decimal::ZERO
        } else {
            equity / r_base
        };
        let mut m_preserve = Decimal::ONE;

        if adaptive {
            if paused_state {
                if buffer_r > cfg.resume_threshold_r {
                    paused_state = false;
                } else {
                    paused += 1;
                    continue;
                }
            } else if buffer_r <= cfg.pause_threshold_r {
                paused_state = true;
                paused += 1;
                continue;
            }
            m_preserve = if buffer_r >= Decimal::from(8) {
                cfg.m_preserve_ceiling
            } else if buffer_r > Decimal::from(2) {
                let frac = (buffer_r - Decimal::from(2)) / Decimal::from(6);
                cfg.m_preserve_floor + (cfg.m_preserve_ceiling - cfg.m_preserve_floor) * frac
            } else {
                cfg.m_preserve_floor
            };
        }

        let r_used = if adaptive {
            r_base * m_preserve
        } else {
            equity * cfg.fixed_r_pct
        };
        if r_used <= Decimal::ZERO {
            rejected += 1;
            continue;
        }

        let is_win = rng.next_dec() < cfg.win_rate;
        let pnl = if is_win { r_used * cfg.rr } else { -r_used };

        if is_win {
            wins += 1;
        } else {
            losses += 1;
        }
        sum_r += if is_win { cfg.rr } else { -Decimal::ONE };

        equity += pnl;
        if equity > peak {
            peak = equity;
        }
        if peak > Decimal::ZERO {
            let dd = (peak - equity) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }

    let avg_r = if cfg.trades == 0 {
        Decimal::ZERO
    } else {
        sum_r / Decimal::from(cfg.trades)
    };

    SimResult {
        strategy: if adaptive { "adaptive" } else { "fixed" },
        final_equity: equity,
        max_dd_pct: max_dd,
        avg_r,
        rejected_trades: rejected,
        paused_trades: paused,
        wins,
        losses,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn cfg(seed: u64) -> SimConfig {
        SimConfig {
            initial_equity: dec!(10000),
            r_base_pct: dec!(0.01),
            fixed_r_pct: dec!(0.01),
            trades: 100,
            win_rate: dec!(0.40),
            rr: dec!(2.0),
            seed,
            pause_threshold_r: dec!(2),
            resume_threshold_r: dec!(2.5),
            m_preserve_floor: dec!(0.25),
            m_preserve_ceiling: dec!(1.0),
        }
    }

    #[test]
    fn deterministic_seed() {
        let a = run(&cfg(42), false);
        let b = run(&cfg(42), false);
        assert_eq!(a.final_equity, b.final_equity);
        assert_eq!(a.wins, b.wins);
    }

    #[test]
    fn different_seeds_differ() {
        // 1 and u64::MAX produce disjoint streams under xorshift.
        let a = run(&cfg(1), false);
        let b = run(&cfg(u64::MAX), false);
        assert_ne!(a.final_equity, b.final_equity);
    }

    #[test]
    fn both_strategies_run_same_seed() {
        let fixed = run(&cfg(7), false);
        let adaptive = run(&cfg(7), true);
        assert_eq!(fixed.strategy, "fixed");
        assert_eq!(adaptive.strategy, "adaptive");
        assert_eq!(fixed.paused_trades, 0);
    }

    #[test]
    fn fixed_r_never_pauses() {
        let r = run(&cfg(3), false);
        assert_eq!(r.strategy, "fixed");
        assert_eq!(r.paused_trades, 0);
    }

    #[test]
    fn high_win_rate_grows() {
        let mut c = cfg(11);
        c.win_rate = dec!(0.60);
        c.rr = dec!(1.5);
        let r = run(&c, false);
        assert!(r.final_equity > c.initial_equity);
    }

    #[test]
    fn low_win_rate_shrinks_aggregate() {
        // Expected per-trade R = -0.80 (win_rate 10%, rr 1).
        // Use many trades + many seeds to overcome variance.
        let mut total = Decimal::ZERO;
        for seed in [1u64, 42, 777, 999_983, 999_999_937, 13, 314, 2718, 161803] {
            let mut c = cfg(seed);
            c.win_rate = dec!(0.10);
            c.rr = dec!(1.0);
            c.trades = 500;
            let r = run(&c, false);
            total += r.final_equity - c.initial_equity;
        }
        assert!(total < Decimal::ZERO, "expected net loss, got {}", total);
    }

    #[test]
    fn buffer_pauses_at_threshold() {
        use crate::equity::buffer;
        let b = buffer::compute(&buffer::BufferInput {
            equity: dec!(10000),
            f_internal: dec!(9800),
            r_open: dec!(0),
            execution_buffer: dec!(0),
            r_base: dec!(100),
        });
        assert_eq!(b.buffer_r, dec!(2));
        assert!(b.pause);

        let b2 = buffer::compute(&buffer::BufferInput {
            equity: dec!(10050),
            f_internal: dec!(9800),
            r_open: dec!(0),
            execution_buffer: dec!(0),
            r_base: dec!(100),
        });
        assert_eq!(b2.buffer_r, dec!(2.5));
        assert!(!b2.pause);
    }
}
