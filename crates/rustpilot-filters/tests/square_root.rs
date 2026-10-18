// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::square_root::{sqrt, sqrtf};

mod common;
use common::Reference;
use std::fmt::Write;

fn next_random(state: &mut u64) -> u64 {
    // SplitMix64 supplies raw bit patterns independently of the implementation.
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut result = *state;
    result = (result ^ (result >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    result = (result ^ (result >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    result ^ (result >> 31)
}

fn corpus32() -> Vec<u32> {
    let mut inputs = Vec::new();
    for exponent in 0..=255 {
        for mantissa in [
            0, 1, 2, 3, 0x0f_ffff, 0x1f_ffff, 0x20_0000, 0x3f_fffe, 0x3f_ffff, 0x40_0000,
            0x40_0001, 0x5f_ffff, 0x7f_fffd, 0x7f_fffe, 0x7f_ffff,
        ] {
            inputs.extend([
                exponent << 23 | mantissa,
                0x8000_0000 | exponent << 23 | mantissa,
            ]);
        }
    }
    for bits in (1..8192).chain(0x007f_e000..0x0080_2000) {
        inputs.extend([bits, bits | 0x8000_0000]);
    }
    let mut state = 0x5250_5351_5254_4633;
    for _ in 0..1_000_000 {
        inputs.push(next_random(&mut state) as u32);
    }
    // Squares of output midpoints are exactly representable in f64. Test the
    // nearest binary32 input and both neighbors on either side of each one.
    for _ in 0..16_384 {
        let lower = 0x3f80_0000 | next_random(&mut state) as u32 & 0x007f_ffff;
        let midpoint =
            (f64::from(f32::from_bits(lower)) + f64::from(f32::from_bits(lower + 1))) * 0.5;
        let nearest = ((midpoint * midpoint) as f32).to_bits();
        inputs.extend(nearest - 2..=nearest + 2);
    }
    for bit in 0..23 {
        for payload in [1 << bit, 0x007f_ffff ^ (1 << bit)] {
            inputs.extend([0x7f80_0000 | payload, 0xff80_0000 | payload]);
        }
    }
    inputs
}

fn corpus64() -> Vec<u64> {
    let mut inputs = Vec::new();
    for exponent in 0..=2047 {
        for mantissa in [
            0,
            1,
            2,
            3,
            0x0001_ffff_ffff_ffff,
            0x0003_ffff_ffff_ffff,
            0x0004_0000_0000_0000,
            0x0007_ffff_ffff_fffe,
            0x0007_ffff_ffff_ffff,
            0x0008_0000_0000_0000,
            0x0008_0000_0000_0001,
            0x000b_ffff_ffff_ffff,
            0x000f_ffff_ffff_fffd,
            0x000f_ffff_ffff_fffe,
            0x000f_ffff_ffff_ffff,
        ] {
            inputs.extend([
                exponent << 52 | mantissa,
                0x8000_0000_0000_0000 | exponent << 52 | mantissa,
            ]);
        }
    }
    for bits in (1..8192).chain(0x000f_ffff_ffff_e000..0x0010_0000_0000_2000) {
        inputs.extend([bits, bits | 0x8000_0000_0000_0000]);
    }
    let mut state = 0x5250_5351_5254_4636;
    for _ in 0..1_000_000 {
        inputs.push(next_random(&mut state));
    }
    // An exact integer square locates each output midpoint's square. Converting
    // that integer to f64 and scaling by a power of two gives the nearest input.
    // The generator does not compute a square root or share implementation code.
    for _ in 0..16_384 {
        let q = (1_u128 << 52) | u128::from(next_random(&mut state) & 0x000f_ffff_ffff_ffff);
        let odd = 2 * q + 1;
        let nearest = ((odd * odd) as f64 * f64::from_bits(917_u64 << 52)).to_bits(); // 2^-106
        inputs.extend(nearest - 2..=nearest + 2);
    }
    for bit in 0..52 {
        for payload in [1 << bit, 0x000f_ffff_ffff_ffff ^ (1 << bit)] {
            inputs.extend([
                0x7ff0_0000_0000_0000 | payload,
                0xfff0_0000_0000_0000 | payload,
            ]);
        }
    }
    inputs
}

fn reference_output(input: &str) -> String {
    assert_eq!(
        std::env::consts::ARCH,
        "x86_64",
        "recorded NaN profile requires x86-64"
    );
    assert_eq!(
        std::env::consts::OS,
        "linux",
        "recorded reference requires Linux"
    );
    let reference = Reference::new("square-root", &[]);
    reference.compile_with_flags(include_str!("square_root_reference.cpp"), &[]);
    reference.run(input)
}

fn check_profile<'a>(lines: &mut impl Iterator<Item = &'a str>) {
    let profile = lines.next().expect("missing reference profile");
    assert!(profile.starts_with("profile|compiler="), "{profile}");
    assert!(profile.contains("|round=FE_TONEAREST|"), "{profile}");
    assert!(profile.ends_with("|ftz=0|daz=0"), "{profile}");
    eprintln!("{profile}; flags=-std=c++17 -O2 -ffp-contract=off; no instrumentation");
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE and the recorded x86-64 Linux C++ profile"]
fn every_bit_matches_cpp_for_both_precisions() {
    // Detect rounding, normalization, exponent-boundary, NaN and sign errors.
    let single = corpus32();
    let double = corpus64();
    let mut input = String::new();
    for bits in &single {
        writeln!(input, "f {bits:08x}").unwrap();
    }
    for bits in &double {
        writeln!(input, "d {bits:016x}").unwrap();
    }
    let output = reference_output(&input);
    let mut lines = output.lines();
    check_profile(&mut lines);
    for bits in &single {
        let expected = u32::from_str_radix(lines.next().expect("missing f32 result"), 16).unwrap();
        assert_eq!(
            sqrtf(f32::from_bits(*bits)).to_bits(),
            expected,
            "sqrtf input={bits:08x}"
        );
    }
    for bits in &double {
        let expected = u64::from_str_radix(lines.next().expect("missing f64 result"), 16).unwrap();
        assert_eq!(
            sqrt(f64::from_bits(*bits)).to_bits(),
            expected,
            "sqrt input={bits:016x}"
        );
    }
    assert_eq!(lines.next(), None, "extra reference output");
    eprintln!(
        "exact C++ comparisons: f32={}, f64={}; no NaN exclusions",
        single.len(),
        double.len()
    );
}

#[test]
#[ignore = "exhaustive 25,165,823-input C++ comparison; run explicitly in release"]
fn exhaustive_positive_subnormals_and_two_binades_match_cpp() {
    let output = reference_output("r 0\n");
    let mut lines = output.lines();
    check_profile(&mut lines);
    let mut count = 0_u64;
    for bits in (1..0x0080_0000).chain(0x3f80_0000..0x4080_0000) {
        let expected =
            u32::from_str_radix(lines.next().expect("missing exhaustive result"), 16).unwrap();
        assert_eq!(
            sqrtf(f32::from_bits(bits)).to_bits(),
            expected,
            "sqrtf input={bits:08x}"
        );
        count += 1;
    }
    assert_eq!(lines.next(), None, "extra reference output");
    assert_eq!(count, 25_165_823);
    eprintln!("exhaustive exact C++ f32 comparisons={count}");
}

#[test]
#[ignore = "exhaustive 16,777,214-input C++ NaN comparison; run explicitly in release"]
fn exhaustive_binary32_nan_signs_and_payloads_match_cpp() {
    let output = reference_output("n 0\n");
    let mut lines = output.lines();
    check_profile(&mut lines);
    let mut count = 0_u64;
    for payload in 1..0x0080_0000 {
        for sign in [0, 0x8000_0000] {
            let bits = sign | 0x7f80_0000 | payload;
            let expected =
                u32::from_str_radix(lines.next().expect("missing NaN result"), 16).unwrap();
            assert_eq!(
                sqrtf(f32::from_bits(bits)).to_bits(),
                expected,
                "sqrtf NaN input={bits:08x}"
            );
            count += 1;
        }
    }
    assert_eq!(lines.next(), None, "extra reference output");
    assert_eq!(count, 16_777_214);
    eprintln!("exhaustive exact C++ f32 NaN comparisons={count}");
}

#[test]
fn exact_and_irrational_positive_roots() {
    // Catch returning the input, truncated roots, or incorrect exponent scaling.
    for (input, expected) in [
        (0x3f80_0000, 0x3f80_0000),
        (0x4080_0000, 0x4000_0000),
        (0x4000_0000, 0x3fb5_04f3),
        (0x4110_0000, 0x4040_0000),
        (0x3e80_0000, 0x3f00_0000),
        (0x0000_0001, 0x1a35_04f3),
        (0x0080_0000, 0x2000_0000),
        (0x7f7f_ffff, 0x5f7f_ffff),
    ] {
        assert_eq!(
            sqrtf(f32::from_bits(input)).to_bits(),
            expected,
            "{input:08x}"
        );
    }
    for (input, expected) in [
        (0x3ff0_0000_0000_0000, 0x3ff0_0000_0000_0000),
        (0x4010_0000_0000_0000, 0x4000_0000_0000_0000),
        (0x4000_0000_0000_0000, 0x3ff6_a09e_667f_3bcd),
        (0x4022_0000_0000_0000, 0x4008_0000_0000_0000),
        (0x3fd0_0000_0000_0000, 0x3fe0_0000_0000_0000),
        (0x0000_0000_0000_0001, 0x1e60_0000_0000_0000),
        (0x0010_0000_0000_0000, 0x2000_0000_0000_0000),
        (0x7fef_ffff_ffff_ffff, 0x5fef_ffff_ffff_ffff),
    ] {
        assert_eq!(
            sqrt(f64::from_bits(input)).to_bits(),
            expected,
            "{input:016x}"
        );
    }
}

#[test]
fn special_values_preserve_every_required_bit() {
    // Catch NaN canonicalization, sign loss, or wrong negative-input handling.
    for (input, expected) in [
        (0x0000_0000, 0x0000_0000),
        (0x8000_0000, 0x8000_0000),
        (0x7f80_0000, 0x7f80_0000),
        (0xff80_0000, 0xffc0_0000),
        (0xbf80_0000, 0xffc0_0000),
        (0x8000_0001, 0xffc0_0000),
        (0x7f80_0001, 0x7fc0_0001),
        (0xff80_0001, 0xffc0_0001),
        (0x7fc1_2345, 0x7fc1_2345),
        (0xffc5_4321, 0xffc5_4321),
    ] {
        assert_eq!(
            sqrtf(f32::from_bits(input)).to_bits(),
            expected,
            "{input:08x}"
        );
    }
    for (input, expected) in [
        (0x0000_0000_0000_0000, 0x0000_0000_0000_0000),
        (0x8000_0000_0000_0000, 0x8000_0000_0000_0000),
        (0x7ff0_0000_0000_0000, 0x7ff0_0000_0000_0000),
        (0xfff0_0000_0000_0000, 0xfff8_0000_0000_0000),
        (0xbff0_0000_0000_0000, 0xfff8_0000_0000_0000),
        (0x8000_0000_0000_0001, 0xfff8_0000_0000_0000),
        (0x7ff0_0000_0000_0001, 0x7ff8_0000_0000_0001),
        (0xfff0_0000_0000_0001, 0xfff8_0000_0000_0001),
        (0x7ff8_1234_5678_9abc, 0x7ff8_1234_5678_9abc),
        (0xfff8_fedc_ba98_7654, 0xfff8_fedc_ba98_7654),
    ] {
        assert_eq!(
            sqrt(f64::from_bits(input)).to_bits(),
            expected,
            "{input:016x}"
        );
    }
}
