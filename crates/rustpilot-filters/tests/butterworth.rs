// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::butterworth::*;

#[test]
fn all_coefficient_sets_begin_at_zero() {
    macro_rules! check {
        ($($coeff:ty),+ $(,)?) => {$({
            let mut filter = Butter2::<$coeff>::new();
            assert_eq!(filter.filter(0.0).to_bits(), 0.0_f32.to_bits());
            assert_eq!(filter.filter(1.0).to_bits(), (1.0 / <$coeff>::GAIN).to_bits());
        })+};
    }
    check!(
        butter100_025_coeffs,
        butter100_05_coeffs,
        butter100_1_coeffs,
        butter100_1_5_coeffs,
        butter100_2_coeffs,
        butter100_3_coeffs,
        butter100_4_coeffs,
        butter100_8_coeffs,
        butter50_8_coeffs
    );
}

#[test]
fn reset_discards_history_and_nonfinite_contamination() {
    let mut filter = butter100hz1_0::default();
    for input in [1.0, 2.0, -3.0, f32::NAN] {
        filter.filter(input);
    }
    assert!(filter.filter(0.0).is_nan());
    filter.reset();
    let mut fresh = butter100hz1_0::new();
    for input in [-0.0, 1.0, 0.0, -0.5, 2.0, 0.0] {
        assert_eq!(
            filter.filter(input).to_bits(),
            fresh.filter(input).to_bits()
        );
    }
}

#[test]
fn nonfinite_values_propagate_instead_of_being_clamped() {
    let mut filter = butter50hz8_0::new();
    assert_eq!(filter.filter(f32::INFINITY), f32::INFINITY);
    assert!(filter.filter(f32::NEG_INFINITY).is_nan());
    assert!(filter.filter(1.0).is_nan());
}

#[test]
fn aliases_select_the_declared_coefficients() {
    macro_rules! check {
        ($coeff:ty; $($alias:ty),+ $(,)?) => {$({
            let mut alias = <$alias>::new();
            let mut direct = Butter2::<$coeff>::new();
            for input in [1.0, 0.0, -1.0, 4.0, 0.0] {
                assert_eq!(alias.filter(input).to_bits(), direct.filter(input).to_bits());
            }
        })+};
    }
    check!(butter100_025_coeffs; butter100hz0_25, butter50hz0_125, butter10hz0_025);
    check!(butter100_05_coeffs; butter100hz0_5, butter50hz0_25, butter10hz0_05);
    check!(butter100_1_coeffs; butter100hz1_0, butter50hz0_5, butter10hz0_1);
    check!(butter100_1_5_coeffs; butter100hz1_5, butter50hz0_75, butter10hz0_15);
    check!(butter100_2_coeffs; butter100hz2_0, butter50hz1_0, butter10hz0_2);
    check!(butter100_3_coeffs; butter100hz3_0, butter50hz1_5, butter10hz0_3);
    check!(butter100_4_coeffs; butter100hz4_0, butter50hz2_0, butter10hz0_4);
    check!(butter100_8_coeffs; butter100hz8_0, butter50hz4_0, butter10hz0_8);
    check!(butter50_8_coeffs; butter50hz8_0, butter10hz1_6);
}

#[test]
fn generic_coefficients_keep_the_two_delayed_states() {
    struct FiniteImpulse;
    impl ButterCoefficients for FiniteImpulse {
        const A1: f32 = 0.0;
        const A2: f32 = 0.0;
        const GAIN: f32 = 4.0;
    }
    let mut filter = Butter2::<FiniteImpulse>::new();
    assert_eq!(filter.filter(1.0), 0.25);
    assert_eq!(filter.filter(0.0), 0.5);
    assert_eq!(filter.filter(0.0), 0.25);
    assert_eq!(filter.filter(0.0), 0.0);
}

mod common;

trait RunFilter {
    fn filter(&mut self, input: f32) -> f32;
    fn reset(&mut self);
}
impl<C: ButterCoefficients> RunFilter for Butter2<C> {
    fn filter(&mut self, input: f32) -> f32 {
        Self::filter(self, input)
    }
    fn reset(&mut self) {
        Self::reset(self);
    }
}

fn all_alias_filters() -> Vec<Box<dyn RunFilter>> {
    macro_rules! filters {
        ($($alias:ty),+ $(,)?) => { vec![$(Box::new(<$alias>::new()) as Box<dyn RunFilter>),+] };
    }
    filters!(
        butter100hz0_25,
        butter50hz0_125,
        butter10hz0_025,
        butter100hz0_5,
        butter50hz0_25,
        butter10hz0_05,
        butter100hz1_0,
        butter50hz0_5,
        butter10hz0_1,
        butter100hz1_5,
        butter50hz0_75,
        butter10hz0_15,
        butter100hz2_0,
        butter50hz1_0,
        butter10hz0_2,
        butter100hz3_0,
        butter50hz1_5,
        butter10hz0_3,
        butter100hz4_0,
        butter50hz2_0,
        butter10hz0_4,
        butter100hz8_0,
        butter50hz4_0,
        butter10hz0_8,
        butter50hz8_0,
        butter10hz1_6,
    )
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE and a C++ compiler"]
fn all_aliases_match_actual_pinned_butter_header() {
    use std::fmt::Write;
    let reference = common::Reference::new("butterworth", &["Butter.h"]);
    reference.compile(include_str!("butterworth_reference.cpp"));
    let mut filters = all_alias_filters();
    assert_eq!(filters.len(), 26);
    let mut expected = Vec::<u32>::new();
    macro_rules! coefficient_bits {
        ($($coeff:ty),+ $(,)?) => {$({
            expected.extend([<$coeff>::A1.to_bits(), <$coeff>::A2.to_bits(), <$coeff>::GAIN.to_bits()]);
        })+};
    }
    coefficient_bits!(
        butter100_025_coeffs,
        butter100_05_coeffs,
        butter100_1_coeffs,
        butter100_1_5_coeffs,
        butter100_2_coeffs,
        butter100_3_coeffs,
        butter100_4_coeffs,
        butter100_8_coeffs,
        butter50_8_coeffs
    );
    let mut commands = vec![Some(1.0)];
    commands.extend(core::iter::repeat_n(Some(0.0), 1024));
    commands.push(None);
    commands.extend(core::iter::repeat_n(Some(1.0), 2048));
    commands.push(None);
    let mut random = 0x93a4_f62b_u32;
    for i in 0..16384 {
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let scale = match i % 4 {
            0 => 0.001,
            1 => 1.0,
            2 => 1000.0,
            _ => 1e10,
        };
        commands.push(Some((random as i32 as f32 / i32::MAX as f32) * scale));
    }
    commands.push(None);
    commands.extend((0..512).map(|i| Some(if i % 2 == 0 { -1.0 } else { 1.0 })));
    for bits in [
        0,
        0x8000_0000,
        1,
        0x8000_0001,
        0x007f_ffff,
        0x807f_ffff,
        0x0080_0000,
        0x8080_0000,
        0x3f7f_ffff,
        0xbf7f_ffff,
        0x3f80_0000,
        0xbf80_0000,
        0x3f80_0001,
        0xbf80_0001,
        0x7f7f_ffff,
        0xff7f_ffff,
        0x7f80_0000,
        0xff80_0000,
        0x7fc0_0000,
        0xffc1_2345,
        0x7f80_0001,
        0xff80_0001,
    ] {
        commands.push(None);
        commands.extend([Some(f32::from_bits(bits)), Some(-0.0), Some(1.0), Some(0.0)]);
    }
    commands.push(None);
    commands.extend(core::iter::repeat_n(Some(f32::MAX), 16));
    commands.push(None);
    commands.extend([Some(-0.0), Some(0.0), Some(1.0), Some(0.0)]);

    let mut input = String::new();
    let mut samples = 0;
    let mut resets = 0;
    for command in commands {
        if let Some(value) = command {
            writeln!(input, "s {}", value.to_bits()).unwrap();
            for filter in &mut filters {
                expected.push(filter.filter(value).to_bits());
            }
            samples += 1;
        } else {
            writeln!(input, "r").unwrap();
            for filter in &mut filters {
                filter.reset();
            }
            resets += 1;
        }
    }
    let output = reference.run(&input);
    let actual: Vec<u32> = output
        .split_whitespace()
        .map(|v| v.parse().unwrap())
        .collect();
    assert_eq!(actual.len(), expected.len(), "reference output count");
    for (index, (&rust, &cpp)) in expected.iter().zip(&actual).enumerate() {
        if f32::from_bits(rust).is_nan() && f32::from_bits(cpp).is_nan() {
            continue;
        }
        assert_eq!(
            rust, cpp,
            "output index {index} (first 27 are coefficient bits)"
        );
    }
    eprintln!("Butterworth: 27 coefficient constants; {samples} samples x 26 aliases = {} outputs; {resets} resets to fresh C++ state; exact bits except NaN payloads", samples * 26);
}
