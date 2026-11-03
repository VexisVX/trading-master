//! End-to-end integration tests for the equity engine.

use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

use trading_manager_lib::equity::buffer;
use trading_manager_lib::equity::protection;
use trading_manager_lib::equity::prop_rules;
use trading_manager_lib::equity::risk_engine;
use trading_manager_lib::equity::state_machine;

fn d(s: &str) -> Decimal { Decimal::from_str(s).unwrap() }

#[test]
fn scenario_normal_growth_then_drawdown() {
    // Start: equity 10000, f_internal 0, r_open 0, r_base 100.
    // Buffer_r = 100 → m_preserve = 1.0 → R_target = 100 → allowed = 100.
    let b = buffer::compute(&buffer::BufferInput {
        equity: d("10000"),
        f_internal: d("0"),
        r_open: d("0"),
        execution_buffer: d("0"),
        r_base: d("100"),
    });
    assert_eq!(b.m_preserve, Decimal::ONE);
    assert!(!b.pause);

    // Simulate 5 losses of 100 each → equity 9500.
    let b2 = buffer::compute(&buffer::BufferInput {
        equity: d("9500"),
        f_internal: d("0"),
        r_open: d("0"),
        execution_buffer: d("0"),
        r_base: d("95"),
    });
    // buffer_r = 9500/95 = 100 → still m=1
    assert_eq!(b2.m_preserve, Decimal::ONE);

    // 1000 losses later: equity 2000, r_base 20 → buffer_r = 100. Still m=1.
    // To force pause: equity 300, r_base 3 → buffer_r = 100. Still fine.
    // To truly pause: equity 150, r_base 3 → buffer_r = 50. Still fine.
    // We need buffer_r <= 2. Achieve by: equity 100, r_base 100 → buffer_r = 1.
    let b3 = buffer::compute(&buffer::BufferInput {
        equity: d("100"),
        f_internal: d("0"),
        r_open: d("0"),
        execution_buffer: d("0"),
        r_base: d("100"),
    });
    assert_eq!(b3.buffer_r, Decimal::ONE);
    assert!(b3.pause);
}

#[test]
fn scenario_protection_floor_progression() {
    let cfg = protection::ProtectionConfig::default_v1();
    // NAV 1.0 → F_trail = 9000, F_profit = None
    let r0 = protection::compute(d("10000"), d("1.0"), &cfg);
    assert!(r0.f_profit.is_none());
    assert_eq!(r0.f_trail, d("9000"));

    // NAV 1.10 → F_profit = 10400 (Q=0.4), F_trail = 9900
    let r10 = protection::compute(d("10000"), d("1.10"), &cfg);
    assert_eq!(r10.f_profit.unwrap(), d("10400.00"));
    assert_eq!(r10.f_internal, d("10400.00"));
}

#[test]
fn scenario_prop_hard_lock_is_permanent() {
    use state_machine::{next, State, TransitionInput};

    let mut ti = TransitionInput {
        previous: State::NormalGrowth,
        prop_breached: true,
        daily_locked: false,
        buffer_r: d("10"),
        recovery_authorized: false,
    };
    let locked = next(&ti);
    assert_eq!(locked, State::PropHardLock);

    // Once locked, even with huge buffer + no breach, stays locked.
    ti.previous = State::PropHardLock;
    ti.prop_breached = false;
    ti.buffer_r = d("100");
    ti.recovery_authorized = true;
    assert_eq!(next(&ti), State::PropHardLock);
}

#[test]
fn scenario_full_pipeline_allowed() {
    // Allowed pipeline: buffer → risk → prop.
    let b = buffer::compute(&buffer::BufferInput {
        equity: d("10000"),
        f_internal: d("9000"),
        r_open: d("0"),
        execution_buffer: d("0"),
        r_base: d("100"),
    });
    // B = 1000, buffer_r = 10 → m = 1.0

    let rk = risk_engine::compute(&risk_engine::RiskInput {
        equity: d("10000"),
        r_base_pct: d("0.01"),
        m_preserve: b.m_preserve,
        r_buffer: b.r_buffer,
        r_portfolio_remaining: d("500"),
        r_daily_remaining: d("300"),
        r_prop_remaining: d("200"),
    });

    // R_target = 100 * 1.0 = 100; buffer = 900; portfolio = 500; daily = 300; prop = 200.
    // min = 100 → binding = Target.
    assert_eq!(rk.r_allowed, d("100"));
    assert_eq!(rk.binding, "Target");

    // Prop rule: DD 1000 from initial 10000 → f_prop = 9000.
    let pr = prop_rules::evaluate(
        &prop_rules::PropRuleSet {
            enabled: true,
            dd_type: prop_rules::DdType::StaticEquity,
            dd_amount: d("1000"),
        },
        &prop_rules::PropContext {
            initial_balance: d("10000"),
            current_equity: d("10000"),
            peak_equity: d("10000"),
            peak_balance: d("10000"),
            current_balance: d("10000"),
        },
    );
    assert_eq!(pr.f_prop, d("9000"));
    assert!(!pr.breached);
}
