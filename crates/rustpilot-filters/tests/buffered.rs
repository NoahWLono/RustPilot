// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::buffered::{
    ArithmeticError, AverageFilter, AverageIntegralFilter, Filter, FilterWithBuffer,
};

#[test]
fn physical_slots_wrap_without_exposing_unchecked_access() {
    let mut f = FilterWithBuffer::<i16, 3>::new();
    assert_eq!(f.get_filter_size(), 3);
    assert_eq!(f.get_sample(0), Some(0));
    assert_eq!(f.get_sample(3), None);
    assert_eq!(f.get_sample(usize::MAX), None);
    for i in 1..=4 {
        assert_eq!(f.apply(i), i);
    }
    assert_eq!(
        (f.get_sample(0), f.get_sample(1), f.get_sample(2)),
        (Some(4), Some(2), Some(3))
    );
    f.reset();
    assert_eq!(f.get_sample(0), Some(0));
    assert_eq!(f.apply(-9), -9);
    assert_eq!(f.get_sample(0), Some(-9));
}

#[test]
fn all_capacities_including_corrected_signed_index_range_work() {
    fn check<const N: usize>() {
        let mut f = FilterWithBuffer::<u32, N>::new();
        for i in 0..N * 3 {
            f.apply(i as u32);
        }
        for i in 0..N {
            assert_eq!(f.get_sample(i), Some((N * 2 + i) as u32));
        }
        f.reset();
        for i in 0..N {
            assert_eq!(f.get_sample(i), Some(0));
        }
    }
    check::<1>();
    check::<127>();
    check::<128>();
    check::<255>();
    let mut avg = AverageFilter::<u8, u16, 255>::new();
    for _ in 0..800 {
        assert_eq!(avg.apply(255), Ok(255));
    }
    let mut integral = AverageIntegralFilter::<i16, i32, 255>::new();
    for _ in 0..800 {
        assert_eq!(integral.apply(-32768), Ok(0));
    }
    assert_eq!(integral.getf(), -32768.0);
}

#[test]
fn invalid_capacities_fail_at_construction() {
    assert!(std::panic::catch_unwind(FilterWithBuffer::<i32, 0>::new).is_err());
    assert!(std::panic::catch_unwind(FilterWithBuffer::<i32, 256>::new).is_err());
    assert!(std::panic::catch_unwind(AverageFilter::<i32, f32, 0>::new).is_err());
    assert!(std::panic::catch_unwind(AverageIntegralFilter::<i32, i32, 256>::new).is_err());
}

#[test]
fn integer_average_uses_warmup_and_truncation_toward_zero() {
    let mut f = AverageFilter::<i8, i16, 3>::new();
    for (sample, want) in [(-8, -8), (3, -2), (7, 0), (9, 6), (-20, -1)] {
        assert_eq!(f.apply(sample), Ok(want));
    }
    f.reset();
    assert_eq!(f.apply(5), Ok(5));
    assert_eq!(f.get_sample(0), Some(5));
    assert_eq!(f.get_sample(1), Some(0));
}

#[test]
fn float_average_preserves_physical_summation_order_and_nonfinite_results() {
    let mut f = AverageFilter::<f32, f32, 3>::new();
    assert_eq!(f.apply(16_777_216.0), Ok(16_777_216.0));
    f.apply(1.0).unwrap();
    assert_eq!(f.apply(-16_777_216.0), Ok(0.0));
    // Ring now holds [1, 1, -16777216]; summing physical slots is intentional.
    assert_eq!(
        f.apply(1.0).unwrap().to_bits(),
        ((1.0_f32 + 1.0 - 16_777_216.0) / 3.0).to_bits()
    );
    f.reset();
    assert_eq!(f.apply(f32::INFINITY), Ok(f32::INFINITY));
    assert!(f.apply(f32::NEG_INFINITY).unwrap().is_nan());
    f.reset();
    assert!(f.apply(f32::NAN).unwrap().is_nan());
}

#[test]
fn signed_overflow_and_float_integer_range_errors_are_transactional() {
    let mut signed = AverageFilter::<i32, i32, 2>::new();
    assert_eq!(signed.apply(i32::MAX), Ok(i32::MAX));
    assert_eq!(signed.apply(1), Err(ArithmeticError::AccumulatorOverflow));
    assert_eq!(signed.get_sample(1), Some(0));
    assert_eq!(signed.apply(-i32::MAX), Ok(0));
    let mut float_signed = AverageFilter::<i32, f32, 2>::new();
    assert_eq!(
        float_signed.apply(i32::MAX),
        Err(ArithmeticError::OutputOutOfRange)
    );
    assert_eq!(float_signed.get_sample(0), Some(0));
    assert_eq!(float_signed.apply(i32::MIN), Ok(i32::MIN));
    let mut float_unsigned = AverageFilter::<u32, f32, 2>::new();
    assert_eq!(
        float_unsigned.apply(u32::MAX),
        Err(ArithmeticError::OutputOutOfRange)
    );
    assert_eq!(float_unsigned.apply(7), Ok(7));
}

#[test]
fn unsigned_accumulators_retain_defined_modular_arithmetic() {
    let mut avg = AverageFilter::<u32, u32, 2>::new();
    assert_eq!(avg.apply(u32::MAX), Ok(u32::MAX));
    assert_eq!(avg.apply(1), Ok(0));
    let mut integral = AverageIntegralFilter::<u32, u32, 2>::new();
    assert_eq!(integral.apply(u32::MAX), Ok(0));
    assert_eq!(integral.apply(1), Ok(0));
    assert_eq!(integral.getd(), 0.0);
    assert_eq!(integral.apply(5), Ok(0));
    assert_eq!(integral.getf(), 3.0);
}

#[test]
fn integral_average_returns_zero_and_resets_sum() {
    let mut f = AverageIntegralFilter::<i32, i32, 10>::new();
    assert_eq!((f.getf(), f.getd()), (0.0, 0.0));
    assert_eq!(f.apply(100), Ok(0));
    assert_eq!(f.apply(200), Ok(0));
    assert_eq!((f.getf(), f.getd()), (150.0, 150.0));
    f.reset();
    assert_eq!((f.getf(), f.getd()), (0.0, 0.0));
    assert_eq!(f.apply(8), Ok(0));
    assert_eq!((f.getf(), f.getd()), (8.0, 8.0));
}

#[test]
fn integral_overflow_rejects_subtraction_or_addition_without_state_change() {
    let mut f = AverageIntegralFilter::<i32, i32, 3>::new();
    f.apply(i32::MIN).unwrap();
    f.apply(i32::MAX).unwrap();
    f.apply(i32::MAX).unwrap();
    let before = (f.getf(), f.getd(), f.get_sample(0));
    assert_eq!(f.apply(i32::MIN), Err(ArithmeticError::AccumulatorOverflow));
    assert_eq!((f.getf(), f.getd(), f.get_sample(0)), before);
    f.reset();
    f.apply(i32::MAX).unwrap();
    assert_eq!(f.apply(1), Err(ArithmeticError::AccumulatorOverflow));
    assert_eq!(f.get_sample(1), Some(0));
    f.apply(-i32::MAX).unwrap();
    assert_eq!(f.getd(), 0.0);
}

#[test]
fn common_filter_interface_preserves_fallibility() {
    fn reset_apply<F: Filter<i16>>(f: &mut F, sample: i16) -> F::Output {
        f.reset();
        f.apply(sample)
    }
    let mut buffer = FilterWithBuffer::<i16, 2>::new();
    assert_eq!(reset_apply(&mut buffer, 17), 17);
    let mut average = AverageFilter::<i16, i32, 2>::new();
    assert_eq!(reset_apply(&mut average, 17), Ok(17));
}

mod common;
use common::Reference;
use rustpilot_filters::buffered::{AverageAccumulator, IntegralAccumulator};

trait FixtureValue: Copy + Default {
    fn input(self) -> String;
    fn output(self) -> String;
}
macro_rules! fixture_integers {
    ($($ty:ty),*) => {$(
        impl FixtureValue for $ty {
            fn input(self) -> String { self.to_string() }
            fn output(self) -> String { self.to_string() }
        }
    )*};
}
fixture_integers!(i8, u8, i16, u16, i32, u32);
impl FixtureValue for f32 {
    fn input(self) -> String {
        self.to_bits().to_string()
    }
    fn output(self) -> String {
        if self.is_nan() {
            "nan".into()
        } else {
            self.to_bits().to_string()
        }
    }
}
fn double_output(value: f64) -> String {
    if value.is_nan() {
        "nan".into()
    } else {
        value.to_bits().to_string()
    }
}

#[derive(Default)]
struct Fixtures {
    input: String,
    rows: Vec<String>,
    scalars: usize,
    cases: usize,
}
impl Fixtures {
    fn record(&mut self, values: Vec<String>) {
        self.scalars += values.len();
        self.rows.push(values.join(" "));
    }
    fn buffer<T: FixtureValue, const N: usize>(&mut self, tag: &str, values: &[T]) {
        self.cases += 1;
        self.input += &format!("b_{tag}_{N} {}\n", values.len());
        let mut filter = FilterWithBuffer::<T, N>::new();
        for (index, &sample) in values.iter().enumerate() {
            let reset = index % 53 == 52;
            self.input += &format!("{} {}\n", if reset { 'R' } else { 'A' }, sample.input());
            let result = if reset {
                filter.reset();
                T::default()
            } else {
                filter.apply(sample)
            };
            let mut row = vec![result.output()];
            for slot in 0..N {
                row.push(filter.get_sample(slot).unwrap().output());
            }
            self.record(row);
        }
    }
    fn average<T: FixtureValue, U: AverageAccumulator<T>, const N: usize>(
        &mut self,
        tag: &str,
        values: &[T],
    ) {
        self.cases += 1;
        self.input += &format!("a_{tag}_{N} {}\n", values.len());
        let mut filter = AverageFilter::<T, U, N>::new();
        for (index, &sample) in values.iter().enumerate() {
            let reset = index % 53 == 52;
            self.input += &format!("{} {}\n", if reset { 'R' } else { 'A' }, sample.input());
            let result = if reset {
                filter.reset();
                T::default()
            } else {
                filter.apply(sample).unwrap()
            };
            let mut row = vec![result.output()];
            for slot in 0..N {
                row.push(filter.get_sample(slot).unwrap().output());
            }
            self.record(row);
        }
    }
    fn integral<T: FixtureValue, U: IntegralAccumulator<T>, const N: usize>(
        &mut self,
        tag: &str,
        values: &[T],
    ) {
        self.cases += 1;
        self.input += &format!("i_{tag}_{N} {}\n", values.len());
        let mut filter = AverageIntegralFilter::<T, U, N>::new();
        for &sample in values {
            self.input += &format!("A {}\n", sample.input());
            let result = filter.apply(sample).unwrap();
            let mut row = vec![
                result.output(),
                filter.getf().output(),
                double_output(filter.getd()),
            ];
            for slot in 0..N {
                row.push(filter.get_sample(slot).unwrap().output());
            }
            self.record(row);
        }
    }
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and a C++ compiler"]
fn buffered_and_average_match_actual_pinned_headers() {
    let reference = Reference::new(
        "buffered",
        &["FilterClass.h", "FilterWithBuffer.h", "AverageFilter.h"],
    );
    reference.compile(include_str!("buffered_reference.cpp"));
    let mut fixtures = Fixtures::default();
    let mut state = 0x91b4_2e77_u32;
    let random: Vec<u32> = (0..180)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        })
        .collect();
    let mut signed8: Vec<i8> = random.iter().map(|&v| v as i8).collect();
    signed8.extend([i8::MIN, i8::MAX, 0, -1, 1]);
    let mut unsigned8: Vec<u8> = random.iter().map(|&v| v as u8).collect();
    unsigned8.extend([u8::MAX, 0, 1]);
    let mut signed16: Vec<i16> = random.iter().map(|&v| v as i16).collect();
    signed16.extend([i16::MIN, i16::MAX, 0, -1, 1]);
    let mut unsigned16: Vec<u16> = random.iter().map(|&v| v as u16).collect();
    unsigned16.extend([u16::MAX, 0, 1]);
    let mut signed32: Vec<i32> = random.iter().map(|&v| v as i32).collect();
    signed32.extend([i32::MIN, i32::MAX, 0, -1, 1]);
    let mut unsigned32 = random.clone();
    unsigned32.extend([u32::MAX, 0, 1]);
    // The f32 alias conversion must remain representable in the original C++.
    let mut signed_float: Vec<i32> = random.iter().map(|&v| (v as i32) / 2).collect();
    signed_float.extend([i32::MIN, 2_147_483_520, 0, -1, 1]);
    let mut unsigned_float: Vec<u32> = random.iter().map(|&v| v / 2).collect();
    unsigned_float.extend([4_294_967_040, 0, 1]);
    let small_signed: Vec<i8> = random.iter().map(|&v| ((v % 61) as i8) - 30).collect();
    let small_signed16: Vec<i16> = random
        .iter()
        .map(|&v| ((v % 16001) as i16) - 8000)
        .collect();
    let small_signed32: Vec<i32> = random
        .iter()
        .map(|&v| ((v % 20_000_001) as i32) - 10_000_000)
        .collect();
    let mut floats: Vec<f32> = random.iter().map(|&v| f32::from_bits(v)).collect();
    floats.extend([
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ]);
    macro_rules! buffers {
        ($tag:expr,$t:ty,$samples:expr) => {{
            fixtures.buffer::<$t, 2>($tag, $samples);
            fixtures.buffer::<$t, 3>($tag, $samples);
            fixtures.buffer::<$t, 4>($tag, $samples);
            fixtures.buffer::<$t, 5>($tag, $samples);
            fixtures.buffer::<$t, 6>($tag, $samples);
            fixtures.buffer::<$t, 7>($tag, $samples);
        }};
    }
    buffers!("i16", i16, &signed16);
    buffers!("u16", u16, &unsigned16);
    buffers!("i32", i32, &signed32);
    buffers!("u32", u32, &unsigned32);
    fixtures.buffer::<i32, 1>("i32", &signed32);
    fixtures.buffer::<i32, 127>("i32", &signed32);
    macro_rules! averages {
        ($tag:expr,$t:ty,$u:ty,$samples:expr) => {{
            fixtures.average::<$t, $u, 2>($tag, $samples);
            fixtures.average::<$t, $u, 3>($tag, $samples);
            fixtures.average::<$t, $u, 4>($tag, $samples);
            fixtures.average::<$t, $u, 5>($tag, $samples);
        }};
    }
    averages!("i8i16", i8, i16, &signed8);
    averages!("u8u16", u8, u16, &unsigned8);
    averages!("i16i32", i16, i32, &signed16);
    averages!("u16u32", u16, u32, &unsigned16);
    averages!("i32f32", i32, f32, &signed_float);
    averages!("u32f32", u32, f32, &unsigned_float);
    fixtures.average::<f32, f32, 5>("f32f32", &floats);
    fixtures.average::<i32, i32, 10>("i32i32", &small_signed32);
    fixtures.average::<u32, u32, 5>("u32u32", &unsigned32);
    fixtures.average::<u8, u8, 3>("u8u8", &unsigned8);
    fixtures.average::<u16, u16, 3>("u16u16", &unsigned16);
    fixtures.average::<i8, i8, 3>("i8i8", &small_signed);
    fixtures.average::<i16, i16, 3>("i16i16", &small_signed16);
    fixtures.average::<i8, i32, 3>("i8i32", &signed8);
    fixtures.average::<u8, u32, 3>("u8u32", &unsigned8);
    fixtures.integral::<i8, i8, 3>("i8i8", &small_signed);
    fixtures.integral::<i8, i16, 3>("i8i16", &signed8);
    fixtures.integral::<i8, i32, 3>("i8i32", &signed8);
    fixtures.integral::<u8, u8, 3>("u8u8", &unsigned8);
    fixtures.integral::<u8, u16, 3>("u8u16", &unsigned8);
    fixtures.integral::<u8, u32, 3>("u8u32", &unsigned8);
    fixtures.integral::<i16, i16, 3>("i16i16", &small_signed16);
    fixtures.integral::<i16, i32, 3>("i16i32", &signed16);
    fixtures.integral::<u16, u16, 3>("u16u16", &unsigned16);
    fixtures.integral::<u16, u32, 3>("u16u32", &unsigned16);
    fixtures.integral::<i32, i32, 10>("i32i32", &small_signed32);
    fixtures.integral::<u32, u32, 5>("u32u32", &unsigned32);
    let actual = reference.run(&fixtures.input);
    let actual_rows: Vec<&str> = actual.lines().map(str::trim).collect();
    assert_eq!(actual_rows.len(), fixtures.rows.len());
    for (index, (expected, actual)) in fixtures.rows.iter().zip(&actual_rows).enumerate() {
        assert_eq!(expected, actual, "reference record {index}");
    }
    eprintln!(
        "buffered differential: {} cases, {} records, {} scalar observations",
        fixtures.cases,
        fixtures.rows.len(),
        fixtures.scalars
    );
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and a C++ compiler"]
fn integral_reset_correction_is_an_explicit_reference_divergence() {
    let reference = Reference::new(
        "integral-reset",
        &["FilterClass.h", "FilterWithBuffer.h", "AverageFilter.h"],
    );
    reference.compile(include_str!("buffered_reference.cpp"));
    // Original reset clears count/buffer but keeps sum=300. Thus next apply(8)
    // gives 308, while the corrected Rust reset gives 8.
    let output = reference.run("i_i32i32_10 4\nA 100\nA 200\nR 0\nA 8\n");
    let rows: Vec<Vec<&str>> = output
        .lines()
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(rows[2][1], 0.0_f32.output());
    assert_eq!(rows[3][1], 308.0_f32.output());
    assert_eq!(rows[3][2], double_output(308.0));
    let mut filter = AverageIntegralFilter::<i32, i32, 10>::new();
    filter.apply(100).unwrap();
    filter.apply(200).unwrap();
    filter.reset();
    filter.apply(8).unwrap();
    assert_eq!(filter.getf(), 8.0);
    assert_eq!(filter.getd(), 8.0);
    assert_ne!(filter.getf().output(), rows[3][1]);
}
