//! Preservation state machine (audit §7, §8).
//!
//! 6 states:
//!   NormalGrowth      — full base risk
//!   Protected         — reduced risk by M_preserve
//!   PreservationPause — no new trades
//!   RecoveryMode      — explicit user auth, probe risk
//!   DailyLock         — until configured reset
//!   PropHardLock      — never auto-clears
//!
//! Hysteresis: pause threshold < resume threshold. See tests.

use rust_decimal::Decimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    NormalGrowth,
    Protected,
    PreservationPause,
    RecoveryMode,
    DailyLock,
    PropHardLock,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::NormalGrowth => "normal_growth",
            State::Protected => "protected",
            State::PreservationPause => "preservation_pause",
            State::RecoveryMode => "recovery_mode",
            State::DailyLock => "daily_lock",
            State::PropHardLock => "prop_hard_lock",
        }
    }
    pub fn is_blocking(self) -> bool {
        matches!(self,
            State::PreservationPause | State::DailyLock | State::PropHardLock)
    }
}

#[derive(Debug, Clone)]
pub struct TransitionInput {
    pub previous: State,
    pub prop_breached: bool,      // hard rule
    pub daily_locked: bool,
    pub buffer_r: Decimal,        // B / R_base
    pub recovery_authorized: bool,
}

/// Pause thresholds — no hysteresis band here; per spec §6.
const BUFFER_R_PAUSE: Decimal = Decimal::from_parts(2, 0, 0, false, 0);
const BUFFER_R_RESUME: Decimal = Decimal::from_parts(25, 0, 0, false, 1); // 2.5

pub fn next(i: &TransitionInput) -> State {
    // 1. Hard locks — no auto-clear.
    if i.previous == State::PropHardLock {
        return State::PropHardLock; // never clears
    }
    if i.prop_breached {
        return State::PropHardLock;
    }
    if i.daily_locked {
        return State::DailyLock;
    }

    // 2. Recovery / Pause with hysteresis.
    if i.previous == State::PreservationPause {
        // Resume only if buffer_r > 2.5 AND authorized (or naturally above threshold).
        if i.buffer_r > BUFFER_R_RESUME {
            // Resume to Protected (not NormalGrowth — must earn back).
            return State::Protected;
        }
        // Still in pause; recovery auth can override to RecoveryMode.
        if i.recovery_authorized {
            return State::RecoveryMode;
        }
        return State::PreservationPause;
    }

    if i.previous == State::RecoveryMode {
        if i.buffer_r > BUFFER_R_RESUME {
            return State::Protected;
        }
        return State::RecoveryMode;
    }

    // 3. Enter pause on low buffer_r.
    if i.buffer_r <= BUFFER_R_PAUSE {
        if i.recovery_authorized {
            return State::RecoveryMode;
        }
        return State::PreservationPause;
    }

    // 4. Normal / Protected.
    if i.buffer_r < Decimal::from(8) {
        return State::Protected;
    }
    State::NormalGrowth
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn i(prev: State, br: Decimal) -> TransitionInput {
        TransitionInput {
            previous: prev,
            prop_breached: false,
            daily_locked: false,
            buffer_r: br,
            recovery_authorized: false,
        }
    }

    #[test]
    fn normal_to_pause() {
        assert_eq!(next(&i(State::NormalGrowth, dec!(1))), State::PreservationPause);
    }

    #[test]
    fn normal_to_protected() {
        assert_eq!(next(&i(State::NormalGrowth, dec!(5))), State::Protected);
    }

    #[test]
    fn protected_to_normal() {
        assert_eq!(next(&i(State::Protected, dec!(10))), State::NormalGrowth);
    }

    #[test]
    fn pause_requires_hysteresis_to_resume() {
        // buffer_r = 2.2 — above pause (2) but below resume (2.5)
        assert_eq!(next(&i(State::PreservationPause, dec!(2.2))), State::PreservationPause);
        // buffer_r = 3 — resumes to Protected
        assert_eq!(next(&i(State::PreservationPause, dec!(3))), State::Protected);
    }

    #[test]
    fn recovery_auth_from_pause() {
        let mut input = i(State::PreservationPause, dec!(1));
        input.recovery_authorized = true;
        assert_eq!(next(&input), State::RecoveryMode);
    }

    #[test]
    fn prop_hard_lock_never_clears() {
        let mut input = i(State::PropHardLock, dec!(100));
        input.prop_breached = false;
        assert_eq!(next(&input), State::PropHardLock);
    }

    #[test]
    fn prop_breach_triggers_lock() {
        let mut input = i(State::NormalGrowth, dec!(100));
        input.prop_breached = true;
        assert_eq!(next(&input), State::PropHardLock);
    }

    #[test]
    fn daily_lock_takes_precedence() {
        let mut input = i(State::Protected, dec!(5));
        input.daily_locked = true;
        assert_eq!(next(&input), State::DailyLock);
    }
}
