//! Preservation buffer (audit §6).
//!
//! B = max(0, Equity - F_internal - R_open - ExecutionBuffer)
//! Buffer_R = B / R_base
//!
//! M_preserve:
//!   Buffer_R >= 8       → 1.0
//!   2 < Buffer_R < 8    → 0.25 + 0.75 * (Buffer_R - 2) / 6
//!   Buffer_R <= 2       → PRESERVATION_PAUSE (caller checks)
//!
//! R_buffer = max(0, B - R_base)
//!
//! Math is Decimal.

use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct BufferInput {
    pub equity: Decimal,
    pub f_internal: Decimal,
    pub r_open: Decimal,
    pub execution_buffer: Decimal,
    pub r_base: Decimal,
}

#[derive(Debug, Clone)]
pub struct BufferResult {
    pub b: Decimal,
    pub buffer_r: Decimal,
    pub m_preserve: Decimal,
    pub r_buffer: Decimal,
    pub pause: bool,
}

pub fn compute(i: &BufferInput) -> BufferResult {
    let raw = i.equity - i.f_internal - i.r_open - i.execution_buffer;
    let b = if raw > Decimal::ZERO {
        raw
    } else {
        Decimal::ZERO
    };

    let (buffer_r, m_preserve, pause) = if i.r_base.is_zero() {
        (Decimal::ZERO, Decimal::ZERO, true)
    } else {
        let br = b / i.r_base;
        let m = if br >= Decimal::from(8) {
            Decimal::ONE
        } else if br > Decimal::from(2) {
            let frac = (br - Decimal::from(2)) / Decimal::from(6);
            Decimal::new(25, 2) + Decimal::new(75, 2) * frac
        } else {
            Decimal::ZERO
        };
        let p = br <= Decimal::from(2);
        (br, m, p)
    };

    let r_buffer = if b > i.r_base {
        b - i.r_base
    } else {
        Decimal::ZERO
    };

    BufferResult {
        b,
        buffer_r,
        m_preserve,
        r_buffer,
        pause,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn base() -> BufferInput {
        BufferInput {
            equity: dec!(10000),
            f_internal: dec!(0),
            r_open: dec!(0),
            execution_buffer: dec!(0),
            r_base: dec!(100),
        }
    }

    #[test]
    fn no_buffer_when_below_floor() {
        let mut i = base();
        i.f_internal = dec!(10500);
        let r = compute(&i);
        assert_eq!(r.b, Decimal::ZERO);
        assert!(r.pause);
    }

    #[test]
    fn buffer_r_high() {
        // B = 10000, R_base = 100 → buffer_r = 100 >= 8 → m = 1
        let r = compute(&base());
        assert_eq!(r.buffer_r, dec!(100));
        assert_eq!(r.m_preserve, Decimal::ONE);
        assert!(!r.pause);
    }

    #[test]
    fn buffer_r_eight_is_one() {
        let mut i = base();
        i.f_internal = dec!(9200); // B = 800, buffer_r = 8
        let r = compute(&i);
        assert_eq!(r.buffer_r, dec!(8));
        assert_eq!(r.m_preserve, Decimal::ONE);
    }

    #[test]
    fn buffer_r_mid() {
        // B = 500, buffer_r = 5 → m = 0.25 + 0.75*(3/6) = 0.625
        let mut i = base();
        i.f_internal = dec!(9500);
        let r = compute(&i);
        assert_eq!(r.buffer_r, dec!(5));
        assert_eq!(r.m_preserve, dec!(0.625));
    }

    #[test]
    fn buffer_r_two_pause() {
        // B = 200, buffer_r = 2 → pause
        let mut i = base();
        i.f_internal = dec!(9800);
        let r = compute(&i);
        assert_eq!(r.buffer_r, dec!(2));
        assert!(r.pause);
        assert_eq!(r.m_preserve, Decimal::ZERO);
    }

    #[test]
    fn r_open_reduces_buffer() {
        let mut i = base();
        i.r_open = dec!(500);
        let r = compute(&i);
        assert_eq!(r.b, dec!(9500));
        assert_eq!(r.buffer_r, dec!(95));
    }

    #[test]
    fn r_buffer_is_b_minus_rbase() {
        let r = compute(&base());
        // B = 10000, R_base = 100 → R_buffer = 9900
        assert_eq!(r.r_buffer, dec!(9900));
    }

    #[test]
    fn zero_rbase_pauses() {
        let mut i = base();
        i.r_base = Decimal::ZERO;
        let r = compute(&i);
        assert!(r.pause);
        assert_eq!(r.m_preserve, Decimal::ZERO);
    }
}
