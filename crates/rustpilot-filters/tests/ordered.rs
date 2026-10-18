// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::ordered::{DerivativeFilter, ModeFilter};

#[test]
fn mode_selects_warmup_middle_then_requested_rank() {
    let mut filter = ModeFilter::<i16, 4>::new(0);
    assert_eq!(filter.get(), 0);
    assert_eq!(filter.apply(50), 50);
    assert_eq!(filter.apply(10), 50);
    assert_eq!(filter.apply(30), 30);
    assert_eq!(filter.apply(20), 10);
    assert_eq!(
        (0..4)
            .map(|i| filter.get_sample(i).unwrap())
            .collect::<Vec<_>>(),
        [10, 20, 30, 50]
    );
    assert_eq!(filter.get_sample(4), None);
    assert_eq!(filter.get_filter_size(), 4);
}

#[test]
fn mode_alternates_extreme_eviction_instead_of_oldest_eviction() {
    let mut filter = ModeFilter::<i16, 3>::new(1);
    for sample in [1, 2, 100] {
        filter.apply(sample);
    }
    // The fourth insertion drops the low extreme (1), not the high (100).
    assert_eq!(filter.apply(3), 3);
    assert_eq!(filter.get_sample(2), Some(100));
    // The fifth drops the high extreme, even though 2 is the oldest sample.
    assert_eq!(filter.apply(4), 3);
    assert_eq!(filter.get_sample(0), Some(2));
    assert_eq!(filter.get_sample(2), Some(4));
}

#[test]
fn mode_invalid_rank_and_full_reset() {
    let mut filter = ModeFilter::<u8, 3>::new(255);
    for sample in [8, 1, 3, 10, 2] {
        filter.apply(sample);
    }
    filter.reset();
    assert_eq!(filter.get(), 0);
    assert!((0..3).all(|i| filter.get_sample(i) == Some(0)));
    let mut fresh = ModeFilter::<u8, 3>::new(255);
    for sample in [8, 1, 3, 10, 2, 4, 20] {
        assert_eq!(filter.apply(sample), fresh.apply(sample));
    }
}

#[test]
fn mode_float_keeps_unordered_nan_and_signed_zero_comparisons() {
    let mut filter = ModeFilter::<f32, 3>::new(1);
    assert_eq!(filter.apply(-0.0).to_bits(), (-0.0_f32).to_bits());
    assert_eq!(filter.apply(0.0).to_bits(), 0.0_f32.to_bits());
    assert_eq!(filter.apply(f32::NAN).to_bits(), 0.0_f32.to_bits());
    assert!(filter.get_sample(2).unwrap().is_nan());
}

#[test]
fn mode_extended_capacity_and_single_slot() {
    let mut large = ModeFilter::<u16, 255>::new(127);
    for i in (0..255).rev() {
        large.apply(i);
    }
    assert_eq!(large.get(), 127);
    large.reset();
    assert_eq!(large.get_sample(254), Some(0));
    let mut small = ModeFilter::<i8, 1>::new(2);
    assert_eq!(small.apply(-8), -8);
    assert_eq!(small.apply(9), 9);
}

#[test]
#[should_panic(expected = "capacity must be in 1..=255")]
fn mode_rejects_zero_capacity() {
    let _ = ModeFilter::<f32, 0>::new(0);
}

#[test]
#[should_panic(expected = "capacity must be in 1..=255")]
fn mode_rejects_oversized_capacity() {
    let _ = ModeFilter::<f32, 256>::new(0);
}

fn check_linear<const N: usize>() {
    let mut filter = DerivativeFilter::<N>::new();
    assert_eq!(filter.slope(), 0.0);
    filter.update(100.0, 0); // Zero equals initialized previous timestamp, so ignored.
    assert_eq!(filter.get_sample(0), Some(0.0));
    for i in 1..=N {
        filter.update((i * 6) as f32, (i * 3) as u32);
    }
    assert_eq!(filter.slope(), 2.0);
    assert_eq!(filter.slope(), 2.0);
    filter.update(900.0, (N * 3) as u32); // Duplicate timestamp rejected.
    assert_eq!(filter.slope(), 2.0);
    assert_eq!(filter.get_sample(N), None);
    filter.reset();
    assert_eq!(filter.slope(), 0.0);
    assert!((0..N).all(|i| filter.get_sample(i) == Some(0.0)));
    filter.update(5.0, (N * 3) as u32); // Previous reset timestamp must not reject this.
    assert_eq!(filter.get_sample(0), Some(5.0));
}

#[test]
fn derivatives_all_supported_sizes_linear_cache_and_reset() {
    check_linear::<5>();
    check_linear::<7>();
    check_linear::<9>();
    check_linear::<11>();
}

#[test]
fn derivative_warmup_preserves_upstream_physical_slot_check() {
    let mut filter = DerivativeFilter::<5>::new();
    for i in 1..4 {
        filter.update(i as f32, i);
        assert_eq!(filter.slope(), 0.0);
    }
    filter.update(4.0, 4);
    // The fifth slot is still zero, but the last two timestamps now differ.
    assert!(filter.slope() > 0.0);
}

#[test]
fn derivative_timestamp_wrap_and_nonfinite_recovery() {
    let mut filter = DerivativeFilter::<5>::new();
    for (i, ts) in [u32::MAX - 4, u32::MAX - 2, u32::MAX, 1, 3]
        .into_iter()
        .enumerate()
    {
        filter.update((i * 2) as f32, ts);
    }
    assert_eq!(filter.slope(), 1.0);
    filter.update(f32::INFINITY, 7);
    assert_eq!(filter.slope(), 0.0);
    filter.reset();
    for i in 1..=5 {
        filter.update(i as f32, i);
    }
    assert_eq!(filter.slope(), 1.0);
}

#[test]
fn inherited_derivative_apply_changes_only_samples_and_ring_position() {
    let mut filter = DerivativeFilter::<5>::new();
    for i in 1..=5 {
        filter.update(i as f32, i);
    }
    assert_eq!(filter.slope(), 1.0);
    assert_eq!(filter.apply(100.0), 100.0);
    assert_eq!(filter.get_sample(0), Some(100.0));
    assert_eq!(filter.slope(), 1.0); // Cache unchanged, as with the upstream inherited method.
    assert_eq!(filter.get_filter_size(), 5);
}

#[test]
#[should_panic(expected = "derivative capacity must be 5, 7, 9 or 11")]
fn derivative_rejects_unsupported_capacity() {
    let _ = DerivativeFilter::<3>::new();
}

mod common;

trait FixtureSample: rustpilot_filters::ordered::ModeSample + core::fmt::Debug {
    fn encode(self) -> String;
    fn decode(token: &str) -> Self;
    fn equivalent(self, other: Self) -> bool;
}
macro_rules! integer_fixtures {
    ($($t:ty),+) => { $(impl FixtureSample for $t {
        fn encode(self) -> String { self.to_string() }
        fn decode(token: &str) -> Self { token.parse().unwrap() }
        fn equivalent(self, other: Self) -> bool { self == other }
    })+ };
}
integer_fixtures!(i8, u8, i16, u16);
impl FixtureSample for f32 {
    fn encode(self) -> String {
        self.to_bits().to_string()
    }
    fn decode(token: &str) -> Self {
        f32::from_bits(token.parse().unwrap())
    }
    fn equivalent(self, other: Self) -> bool {
        self.to_bits() == other.to_bits() || (self.is_nan() && other.is_nan())
    }
}

fn reference() -> common::Reference {
    let reference = common::Reference::new(
        "ordered",
        &[
            "ModeFilter.h",
            "ModeFilter.cpp",
            "DerivativeFilter.h",
            "DerivativeFilter.cpp",
            "FilterClass.h",
            "FilterWithBuffer.h",
        ],
    );
    // Upstream DerivativeFilter.cpp needs only isnan/isinf from AP_Math and
    // no declarations from its Filter.h umbrella. No filter logic is shimmed.
    reference.write(
        "AP_Math/AP_Math.h",
        b"#pragma once\n#include <cmath>\nusing std::isnan; using std::isinf;\n",
    );
    reference.write("Filter.h", b"#pragma once\n");
    reference.compile(include_str!("ordered_reference.cpp"));
    reference
}

fn compare_mode<T: FixtureSample, const N: usize>(
    reference: &common::Reference,
    name: &str,
    rank: usize,
    values: &[T],
) -> usize {
    use std::fmt::Write;
    let mut input = format!("mode {name} {N} {rank}\nq\n");
    for &sample in values {
        writeln!(input, "a {}", sample.encode()).unwrap();
    }
    let output = reference.run(&input);
    let mut tokens = output.split_whitespace();
    assert_eq!(tokens.next().unwrap().parse::<usize>().unwrap(), N);
    let mut filter = ModeFilter::<T, N>::new(rank);
    for (step, sample) in std::iter::once(None)
        .chain(values.iter().copied().map(Some))
        .enumerate()
    {
        if let Some(sample) = sample {
            filter.apply(sample);
        }
        for value in
            std::iter::once(filter.get()).chain((0..N).map(|i| filter.get_sample(i).unwrap()))
        {
            let expected = T::decode(tokens.next().unwrap());
            assert!(
                value.equivalent(expected),
                "mode {name}/{N} rank {rank}, step {step}: Rust {value:?}, C++ {expected:?}"
            );
        }
    }
    assert!(tokens.next().is_none());
    1 + (values.len() + 1) * (N + 1)
}

fn compare_all_mode_sizes<T: FixtureSample>(
    reference: &common::Reference,
    name: &str,
    samples: &[T],
) -> usize {
    let mut count = 0;
    macro_rules! compare { ($($n:literal),+) => { $(
        for rank in [0, $n/2, $n-1, 255] {
            count += compare_mode::<T,$n>(reference, name, rank, samples);
        }
    )+ }; }
    compare!(1, 3, 4, 5, 6, 7, 127);
    count
}

#[derive(Clone, Copy)]
enum DerivativeOperation {
    Update(f32, u32),
    Apply(f32),
    Query,
}

fn derivative_cases() -> Vec<DerivativeOperation> {
    use DerivativeOperation::*;
    let mut operations = vec![Query, Update(5.0, 0), Update(-0.0, 1), Update(50.0, 1)];
    for i in 2..30 {
        operations.push(Update((i * 7) as f32, i));
    }
    // Repeated non-adjacent times exercise zero denominators while adjacent
    // duplicates are rejected. Then use wrap, non-monotonicity and nonfinite data.
    for i in 0..30 {
        operations.push(Update(i as f32, 1 + i % 2));
    }
    let mut state = 0x91e1_0da5_u32;
    let mut timestamp = u32::MAX - 150;
    for i in 0..1536 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let sample = match i % 83 {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            3 => -0.0,
            4 => f32::MAX,
            5 => f32::MIN_POSITIVE,
            _ => f32::from_bits(state),
        };
        if i % 7 != 0 {
            timestamp = timestamp.wrapping_add(1 + (state % 4096));
        }
        if i % 23 == 0 {
            timestamp = timestamp.wrapping_sub(100000);
        }
        operations.push(Update(sample, timestamp));
        if i % 47 == 0 {
            operations.push(Apply((i as f32) * 0.25));
        }
        if i % 13 == 0 {
            operations.push(Query);
        }
    }
    operations
}

fn compare_derivative<const N: usize>(
    reference: &common::Reference,
    cases: &[DerivativeOperation],
) -> usize {
    use std::fmt::Write;
    let mut input = format!("derivative {N}\n");
    let mut filter = DerivativeFilter::<N>::new();
    let mut expected = Vec::new();
    for operation in cases {
        match *operation {
            DerivativeOperation::Update(sample, timestamp) => {
                writeln!(input, "u {} {timestamp}", sample.to_bits()).unwrap();
                filter.update(sample, timestamp);
            }
            DerivativeOperation::Apply(sample) => {
                writeln!(input, "a {}", sample.to_bits()).unwrap();
                expected.push(filter.apply(sample));
            }
            DerivativeOperation::Query => {
                writeln!(input, "q").unwrap();
            }
        }
        expected.push(filter.slope());
        expected.push(filter.slope());
        expected.extend((0..N).map(|i| filter.get_sample(i).unwrap()));
    }
    let output = reference.run(&input);
    let mut tokens = output.split_whitespace();
    assert_eq!(tokens.next().unwrap().parse::<usize>().unwrap(), N);
    for (index, rust) in expected.iter().enumerate() {
        let cpp = f32::decode(tokens.next().unwrap());
        assert!(
            rust.equivalent(cpp),
            "derivative {N}, output {index}: Rust {rust:?} ({:08x}), C++ {cpp:?} ({:08x})",
            rust.to_bits(),
            cpp.to_bits()
        );
    }
    assert!(tokens.next().is_none());
    1 + expected.len()
}

#[test]
#[ignore = "requires pinned ARDUPILOT_REFERENCE and C++ compiler"]
fn actual_upstream_ordered_filters_match() {
    let reference = reference();
    let mut state = 0xa1b2_c3d4_u32;
    let mut bits = vec![
        0, 0x80000000, 0x7f800000, 0xff800000, 0x7fc00001, 0x7fa00001, 1, 0x7f7fffff,
    ];
    for _ in 0..512 {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        bits.push(state);
    }
    let signed8: Vec<_> = bits.iter().map(|&v| v as i8).collect();
    let unsigned8: Vec<_> = bits.iter().map(|&v| v as u8).collect();
    let signed16: Vec<_> = bits.iter().map(|&v| v as i16).collect();
    let unsigned16: Vec<_> = bits.iter().map(|&v| v as u16).collect();
    let floats: Vec<_> = bits.iter().map(|&v| f32::from_bits(v)).collect();
    let mode_outputs = compare_all_mode_sizes(&reference, "i8", &signed8)
        + compare_all_mode_sizes(&reference, "u8", &unsigned8)
        + compare_all_mode_sizes(&reference, "i16", &signed16)
        + compare_all_mode_sizes(&reference, "u16", &unsigned16)
        + compare_all_mode_sizes(&reference, "f32", &floats);
    let cases = derivative_cases();
    let derivative_outputs = compare_derivative::<5>(&reference, &cases)
        + compare_derivative::<7>(&reference, &cases)
        + compare_derivative::<9>(&reference, &cases)
        + compare_derivative::<11>(&reference, &cases);
    eprintln!("ordered reference comparisons: {mode_outputs} mode values; {derivative_outputs} derivative values");
}

#[test]
#[ignore = "requires pinned ARDUPILOT_REFERENCE and C++ compiler"]
fn reset_fixes_have_explicit_upstream_divergences() {
    let reference = reference();
    let output = reference.run("mode i16 3 1\na 1\na 2\na 100\nr\na 1\na 2\na 100\na 3\n");
    let rows: Vec<Vec<i16>> = output
        .lines()
        .skip(1)
        .map(|line| {
            line.split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect()
        })
        .collect();
    let mut mode = ModeFilter::<i16, 3>::new(1);
    for value in [1, 2, 100] {
        mode.apply(value);
    }
    mode.reset();
    assert_eq!(mode.get(), 0);
    assert_eq!(rows[3][0], 2, "upstream reset retains selected output");
    for value in [1, 2, 100] {
        mode.apply(value);
    }
    assert_eq!(mode.apply(3), 3);
    assert_eq!(
        rows[7][0], 2,
        "upstream reset retains opposite eviction phase"
    );

    let output = reference.run("derivative 5\nu 1065353216 1\nu 1073741824 2\nu 1077936128 3\nu 1082130432 4\nu 1084227584 5\nr\nu 1091567616 5\n");
    let rows: Vec<Vec<f32>> = output
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().map(f32::decode).collect())
        .collect();
    let mut derivative = DerivativeFilter::<5>::new();
    for i in 1..=5 {
        derivative.update(i as f32, i);
        derivative.slope();
    }
    derivative.reset();
    assert_eq!(derivative.slope(), 0.0);
    assert_eq!(rows[5][0], 1.0, "upstream reset retains cached slope");
    derivative.update(9.0, 5);
    assert_eq!(derivative.get_sample(0), Some(9.0));
    assert_eq!(
        rows[6][2], 0.0,
        "upstream retains old timestamp and rejects first post-reset sample"
    );
    eprintln!("verified intentional reset differences: mode output and phase; derivative slope cache and timestamps");
}

#[test]
fn derivative_reset_with_pending_update_matches_fresh_state() {
    let mut filter = DerivativeFilter::<7>::new();
    for i in 1..=12 {
        filter.update((i * i) as f32, i);
    }
    filter.reset();
    let mut fresh = DerivativeFilter::<7>::new();
    for operation in derivative_cases().into_iter().take(90) {
        match operation {
            DerivativeOperation::Update(sample, timestamp) => {
                filter.update(sample, timestamp);
                fresh.update(sample, timestamp);
            }
            DerivativeOperation::Apply(sample) => {
                filter.apply(sample);
                fresh.apply(sample);
            }
            DerivativeOperation::Query => {}
        }
        assert!(filter.slope().equivalent(fresh.slope()));
        for slot in 0..7 {
            assert!(filter
                .get_sample(slot)
                .unwrap()
                .equivalent(fresh.get_sample(slot).unwrap()));
        }
    }
}

#[test]
fn ordered_filters_preserve_filter_trait_dispatch() {
    use rustpilot_filters::buffered::Filter;
    let mut mode = ModeFilter::<u16, 3>::new(1);
    let polymorphic: &mut dyn Filter<u16, Output = u16> = &mut mode;
    assert_eq!(polymorphic.apply(100), 100);
    polymorphic.reset();
    assert_eq!(mode.get(), 0);
    let mut derivative = DerivativeFilter::<5>::new();
    let polymorphic: &mut dyn Filter<f32, Output = f32> = &mut derivative;
    assert_eq!(polymorphic.apply(20.0), 20.0);
    polymorphic.reset();
    assert_eq!(derivative.get_sample(0), Some(0.0));
}
