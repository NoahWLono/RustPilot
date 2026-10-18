use rustpilot_filters::{
    calc_lowpass_alpha_dt as alpha, DigitalLpf, LowPassFilter, LowPassFilterConstDt,
};

#[test]
fn alpha_boundaries_use_upstream_epsilon_and_branch_order() {
    assert_eq!(alpha(0.0, 2.0), 0.0);
    assert_eq!(alpha(0.0, 0.0), 1.0);
    assert_eq!(alpha(-1.0, 2.0), 1.0);
    assert_eq!(alpha(1.0, -2.0), 1.0);
    assert_eq!(alpha(f32::EPSILON / 2.0, 2.0), 0.0);
    assert_eq!(alpha(-f32::EPSILON / 2.0, 2.0), 0.0);
    assert_eq!(alpha(1.0, f32::EPSILON / 2.0), 1.0);
    assert_eq!(alpha(-f32::EPSILON, 2.0), 1.0);
    assert!(alpha(f32::EPSILON, 2.0) > 0.0);
    assert!((alpha(1.0, 2.0) - 0.926288).abs() < 0.000001);
}

#[test]
fn digital_initialization_reset_and_unclamped_alpha() {
    let mut f = DigitalLpf::new();
    assert_eq!(f.get(), 0.0);
    assert_eq!(f.apply(8.0, 0.0), 8.0);
    assert_eq!(f.apply(0.0, 0.25), 6.0);
    assert_eq!(f.apply(2.0, 0.5), 4.0);
    f.reset();
    assert_eq!(f.get(), 4.0);
    assert_eq!(f.apply(12.0, 0.0), 12.0);
    f.reset_to(2.0);
    assert_eq!(f.get(), 2.0);
    assert_eq!(f.apply(4.0, 2.0), 6.0);
    assert_eq!(f.apply(4.0, -1.0), 8.0);
}

#[test]
fn variable_cutoff_and_reset_sequence() {
    let mut f = LowPassFilter::new(2.0);
    assert_eq!(f.get(), 0.0);
    assert_eq!(f.cutoff_frequency(), 2.0);
    assert_eq!(f.apply(8.0, 0.0), 8.0);
    assert_eq!(f.apply(9.0, 0.0), 8.0);
    f.reset_to(0.0);
    assert_eq!(f.apply(10.0, 1.0), 10.0 * alpha(1.0, 2.0));
    f.set_cutoff_frequency(0.0);
    assert_eq!(f.cutoff_frequency(), 0.0);
    assert_eq!(f.apply(3.0, 0.0), 3.0);
    f.reset();
    assert_eq!(f.apply(11.0, 0.0), 11.0);
}

#[test]
fn constant_step_matches_variable_step_and_handles_disabled_sample_rate() {
    let mut c = LowPassFilterConstDt::new(50.0, 2.0);
    let mut v = LowPassFilter::new(2.0);
    assert_eq!(c.get(), 0.0);
    assert_eq!(c.cutoff_frequency(), 2.0);
    for value in [3.0, 5.0, -8.0, 12.0, 0.0] {
        assert_eq!(c.apply(value), v.apply(value, 0.02));
    }
    c.reset_to(0.0);
    assert_eq!(c.apply(10.0), 10.0 * alpha(0.02, 2.0));
    for sample_rate in [0.0, -50.0] {
        c.set_cutoff_frequency(sample_rate, 7.0);
        assert_eq!(c.cutoff_frequency(), 7.0);
        assert_eq!(c.apply(4.0), 4.0);
    }
    c.reset();
    assert_eq!(c.apply(19.0), 19.0);
}

#[test]
fn nonfinite_values_have_explicit_ieee_propagation() {
    assert!(alpha(f32::NAN, 2.0).is_nan());
    assert!(alpha(1.0, f32::NAN).is_nan());
    assert!(alpha(f32::INFINITY, 2.0).is_nan());
    assert_eq!(alpha(1.0, f32::INFINITY), 1.0);
    assert_eq!(alpha(f32::NEG_INFINITY, 2.0), 1.0);
    assert_eq!(alpha(f32::NAN, 0.0), 1.0);
    let mut d = DigitalLpf::new();
    assert_eq!(d.apply(3.0, f32::NAN), 3.0);
    assert!(d.apply(4.0, f32::NAN).is_nan());
    assert!(d.apply(5.0, 1.0).is_nan());
    d.reset();
    assert_eq!(d.apply(6.0, 1.0), 6.0);
    d.reset_to(f32::INFINITY);
    assert!(d.apply(0.0, 0.5).is_nan());
    d.reset();
    assert!(d.apply(f32::NAN, 1.0).is_nan());
    let mut c = LowPassFilterConstDt::new(f32::NAN, 2.0);
    assert_eq!(c.apply(1.0), 1.0);
    assert!(c.apply(2.0).is_nan());
    let mut v = LowPassFilter::new(f32::NAN);
    assert_eq!(v.apply(1.0, 1.0), 1.0);
    assert!(v.apply(2.0, 1.0).is_nan());
}

#[test]
fn defaults_have_defined_zero_output_and_disabled_cutoff() {
    let mut d = DigitalLpf::default();
    let mut v = LowPassFilter::default();
    let mut c = LowPassFilterConstDt::default();
    assert_eq!(d.get(), 0.0);
    assert_eq!(v.get(), 0.0);
    assert_eq!(c.get(), 0.0);
    assert_eq!(v.cutoff_frequency(), 0.0);
    assert_eq!(c.cutoff_frequency(), 0.0);
    for sample in [3.0, 8.0, -12.0] {
        assert_eq!(d.apply(sample, 1.0), sample);
        assert_eq!(v.apply(sample, 0.02), sample);
        assert_eq!(c.apply(sample), sample);
    }
}

#[test]
fn finite_arithmetic_overflow_is_preserved_and_reset_recovers() {
    let mut d = DigitalLpf::new();
    d.reset_to(-f32::MAX);
    assert_eq!(d.apply(f32::MAX, 0.5), f32::INFINITY);
    assert!(d.apply(0.0, 1.0).is_nan());
    d.reset_to(0.0);
    assert_eq!(d.apply(1.0, 0.5), 0.5);
}
