// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::vector::{
    DigitalVectorLpf, Vector, VectorLowPassFilter, VectorLowPassFilterConstDt,
};
use std::{fmt::Write as _, path::PathBuf};
mod common;
use common::{verified_source, Reference, PIN};

fn bits<const N: usize>(value: Vector<N>) -> [u32; N] {
    value.components.map(f32::to_bits)
}

#[test]
fn independent_components_and_unbounded_alpha() {
    let mut filter = DigitalVectorLpf::<3>::new();
    assert_eq!(bits(filter.get()), [0; 3]);
    assert_eq!(
        filter.apply(Vector::new([8.0, -4.0, 2.0]), 0.0).components,
        [8.0, -4.0, 2.0]
    );
    assert_eq!(
        filter
            .apply(Vector::new([12.0, 4.0, -2.0]), 0.25)
            .components,
        [9.0, -2.0, 1.0]
    );
    assert_eq!(
        filter.apply(Vector::new([10.0, -3.0, 4.0]), 2.0).components,
        [11.0, -4.0, 7.0]
    );
}

#[test]
fn reset_preserves_output_until_next_sample_and_copies_all_bits() {
    let mut filter = DigitalVectorLpf::<2>::new();
    let special = Vector::new([f32::from_bits(0x7f81_2345), -0.0]);
    filter.reset_to(special);
    assert_eq!(bits(filter.get()), bits(special));
    filter.reset();
    assert_eq!(bits(filter.get()), bits(special));
    let replacement = Vector::new([f32::from_bits(0xffc5_4321), 0.0]);
    assert_eq!(bits(filter.apply(replacement, f32::NAN)), bits(replacement));
}

#[test]
fn variable_dt_reconfiguration_retains_history() {
    let mut filter = VectorLowPassFilter::<2>::new(3.0);
    filter.reset_to(Vector::new([3.0, 9.0]));
    filter.set_cutoff_frequency(0.0);
    assert_eq!(filter.cutoff_frequency().to_bits(), 0);
    assert_eq!(filter.get().components, [3.0, 9.0]);
    assert_eq!(
        filter.apply(Vector::new([7.0, -1.0]), 0.1).components,
        [7.0, -1.0]
    );
    filter.set_cutoff_frequency(1.0);
    assert_eq!(
        filter.apply(Vector::new([70.0, -10.0]), 0.0).components,
        [7.0, -1.0]
    );
}

#[test]
fn constant_dt_reconfiguration_retains_history() {
    let mut filter = VectorLowPassFilterConstDt::<3>::new(100.0, 10.0);
    filter.reset_to(Vector::new([3.0, 9.0, -4.0]));
    filter.set_cutoff_frequency(0.0, 8.0);
    assert_eq!(filter.cutoff_frequency(), 8.0);
    assert_eq!(filter.get().components, [3.0, 9.0, -4.0]);
    assert_eq!(
        filter.apply(Vector::new([7.0, -1.0, 10.0])).components,
        [7.0, -1.0, 10.0]
    );
}

#[test]
fn vector_operations_follow_component_order() {
    let left = Vector::new([5.0, -1.0, 0.5]);
    let right = Vector::new([1.0, 3.0, -0.5]);
    assert_eq!((left - right).components, [4.0, -4.0, 1.0]);
    assert_eq!((left + right).components, [6.0, 2.0, 0.0]);
    assert_eq!((left * 2.0).components, [10.0, -2.0, 1.0]);
    let mut changed = left;
    changed += right;
    assert_eq!(changed.components, [6.0, 2.0, 0.0]);
}

fn extract(source: &str, signature: &str) -> String {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing source signature {signature}"));
    let end = start + source[start..].find("\n}").unwrap() + 2;
    source[start..end].to_owned()
}

fn reference() -> Reference {
    reference_with_flags(&[])
}

fn reference_with_flags(flags: &[&str]) -> Reference {
    let reference = Reference::new("vector", &["LowPassFilter.h", "LowPassFilter.cpp"]);
    let root =
        PathBuf::from(std::env::var_os("ARDUPILOT_REFERENCE").expect("set ARDUPILOT_REFERENCE"));
    for header in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        let bytes = verified_source(&root, PIN, &format!("libraries/AP_Math/{header}")).unwrap();
        reference.write(&format!("AP_Math/{header}"), bytes);
    }
    reference.write("AP_Math/AP_Math.h", include_str!("vector_reference_math.h"));
    reference.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    reference.write("AP_HAL/AP_HAL.h", "#pragma once\n#define HAL_WITH_EKF_DOUBLE 0\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n");
    reference.write("AP_InternalError/AP_InternalError.h", "#pragma once\n");
    let mut definitions = String::new();
    for dimension in [2, 3] {
        let source = String::from_utf8(
            verified_source(
                &root,
                PIN,
                &format!("libraries/AP_Math/vector{dimension}.cpp"),
            )
            .unwrap(),
        )
        .unwrap();
        for signature in [
            format!("template <typename T>\nVector{dimension}<T> &Vector{dimension}<T>::operator +=(const Vector{dimension}<T> &v)"),
            format!("template <typename T>\nVector{dimension}<T> Vector{dimension}<T>::operator -(const Vector{dimension}<T> &v) const"),
            format!("template <typename T>\nVector{dimension}<T> Vector{dimension}<T>::operator +(const Vector{dimension}<T> &v) const"),
            format!("template <typename T>\nVector{dimension}<T> Vector{dimension}<T>::operator *(const T num) const"),
        ] {
            definitions.push_str(&extract(&source, &signature));
            definitions.push('\n');
        }
    }
    reference.write("vector_operations.cpp", definitions);
    let math =
        String::from_utf8(verified_source(&root, PIN, "libraries/AP_Math/AP_Math.cpp").unwrap())
            .unwrap();
    reference.write(
        "alpha.cpp",
        extract(
            &math,
            "float calc_lowpass_alpha_dt(float dt, float cutoff_freq)\n{",
        ),
    );
    let mut implementation = "#define HAL_DEBUG_BUILD 1\n#include \"AP_Math/AP_Math.h\"\n#include \"vector_operations.cpp\"\n#include \"alpha.cpp\"\n#include \"LowPassFilter.cpp\"\n".to_owned();
    for dimension in [2, 3] {
        for op in ["+", "-"] {
            writeln!(&mut implementation, "template Vector{dimension}<float> Vector{dimension}<float>::operator {op}(const Vector{dimension}<float>&) const;").unwrap();
        }
        writeln!(
            &mut implementation,
            "template Vector{dimension}<float> Vector{dimension}<float>::operator *(float) const;"
        )
        .unwrap();
        writeln!(&mut implementation, "template Vector{dimension}<float>& Vector{dimension}<float>::operator +=(const Vector{dimension}<float>&);").unwrap();
    }
    reference.write("implementation.cpp", implementation);
    reference.compile_sources_with_flags(
        include_str!("vector_reference.cpp"),
        &["implementation.cpp"],
        flags,
    );
    reference
}

struct Models<const N: usize> {
    digital: DigitalVectorLpf<N>,
    variable: VectorLowPassFilter<N>,
    fixed: VectorLowPassFilterConstDt<N>,
}
impl<const N: usize> Models<N> {
    fn new() -> Self {
        Self {
            digital: DigitalVectorLpf::new(),
            variable: VectorLowPassFilter::new(0.0),
            fixed: VectorLowPassFilterConstDt::new(0.0, 0.0),
        }
    }
    fn step(
        &mut self,
        kind: usize,
        op: char,
        sample: Vector<N>,
        arg: f32,
        rate: f32,
    ) -> (Vector<N>, f32) {
        match kind {
            0 => {
                match op {
                    'A' => {
                        self.digital.apply(sample, arg);
                    }
                    'R' => self.digital.reset(),
                    'S' => self.digital.reset_to(sample),
                    'C' => {}
                    _ => panic!("unknown operation"),
                }
                (self.digital.get(), 0.0)
            }
            1 => {
                match op {
                    'A' => {
                        self.variable.apply(sample, arg);
                    }
                    'R' => self.variable.reset(),
                    'S' => self.variable.reset_to(sample),
                    'C' => self.variable.set_cutoff_frequency(arg),
                    _ => panic!("unknown operation"),
                }
                (self.variable.get(), self.variable.cutoff_frequency())
            }
            2 => {
                match op {
                    'A' => {
                        self.fixed.apply(sample);
                    }
                    'R' => self.fixed.reset(),
                    'S' => self.fixed.reset_to(sample),
                    'C' => self.fixed.set_cutoff_frequency(rate, arg),
                    _ => panic!("unknown operation"),
                }
                (self.fixed.get(), self.fixed.cutoff_frequency())
            }
            3 => {
                let output = match op {
                    '+' => sample + self.digital.get(),
                    '-' => sample - self.digital.get(),
                    '*' => sample * arg,
                    '=' => {
                        let mut output = sample;
                        output += self.digital.get();
                        output
                    }
                    _ => panic!("unknown vector operation"),
                };
                (output, 0.0)
            }
            _ => panic!("unknown kind"),
        }
    }
}

struct Fixture {
    input: String,
    expected: Vec<u32>,
    records: usize,
}
impl Fixture {
    fn new() -> Self {
        Self {
            input: String::new(),
            expected: Vec::new(),
            records: 0,
        }
    }
    fn emit<const N: usize>(
        &mut self,
        models: &mut Models<N>,
        kind: usize,
        op: char,
        samples: [u32; 3],
        arg: u32,
        rate: u32,
    ) {
        writeln!(
            &mut self.input,
            "{N} {kind} {op} {} {} {} {arg} {rate}",
            samples[0], samples[1], samples[2]
        )
        .unwrap();
        let sample = Vector::new(core::array::from_fn(|i| f32::from_bits(samples[i])));
        let (output, cutoff) =
            models.step(kind, op, sample, f32::from_bits(arg), f32::from_bits(rate));
        self.expected.extend(bits(output));
        self.expected.push(cutoff.to_bits());
        self.records += 1;
    }
    fn finite_dimension<const N: usize>(&mut self) {
        let mut models = Models::<N>::new();
        let mut state = 0x56ab_1357_u32 ^ N as u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        let edge = [
            0u32,
            0x8000_0000,
            1,
            0x8000_0001,
            0x007f_ffff,
            0x0080_0000,
            0x33ff_ffff,
            0x3400_0000,
            0x3400_0001,
            0x3f80_0000,
            0xbf80_0000,
        ];
        for kind in 0..3 {
            self.emit(
                &mut models,
                kind,
                'C',
                [0; 3],
                3.0f32.to_bits(),
                100.0f32.to_bits(),
            );
            for index in 0..40_000 {
                let sample = if index < edge.len() * edge.len() {
                    [
                        edge[index % edge.len()],
                        edge[index / edge.len()],
                        edge[(index + 5) % edge.len()],
                    ]
                } else {
                    core::array::from_fn(|_| {
                        let raw = next();
                        (raw & 0x807f_ffff) | (((raw >> 24) % 191) << 23)
                    })
                };
                if index % 997 == 0 {
                    self.emit(&mut models, kind, 'R', [0; 3], 0, 0);
                }
                if index % 2039 == 0 {
                    self.emit(&mut models, kind, 'S', sample, 0, 0);
                }
                if index % 4093 == 0 {
                    self.emit(
                        &mut models,
                        kind,
                        'C',
                        [0; 3],
                        ((index % 17) as f32 + 1.0).to_bits(),
                        ((index % 103) as f32 + 30.0).to_bits(),
                    );
                }
                let arg = if kind == 0 {
                    0.125f32.to_bits()
                } else {
                    0.01f32.to_bits()
                };
                self.emit(&mut models, kind, 'A', sample, arg, 0);
            }
        }
        for &left in &edge {
            for &right in &edge {
                self.emit(&mut models, 0, 'S', [left, right, left], 0, 0);
                for op in ['+', '-', '*', '='] {
                    self.emit(
                        &mut models,
                        3,
                        op,
                        [right, left, right],
                        0.5f32.to_bits(),
                        0,
                    );
                }
            }
        }
    }
    fn dimension<const N: usize>(&mut self) {
        let mut models = Models::<N>::new();
        let edges: [u32; 30] = [
            0,
            0x8000_0000,
            1,
            0x8000_0001,
            0x007f_ffff,
            0x0080_0000,
            0x33ff_ffff,
            0x3400_0000,
            0x3400_0001,
            0xb3ff_ffff,
            0xb400_0000,
            0xb400_0001,
            0x3f00_0000,
            0x3f80_0000,
            0xbf80_0000,
            0x3f80_0001,
            0x3f7f_ffff,
            0x7f7f_ffff,
            0xff7f_ffff,
            0x7f80_0000,
            0xff80_0000,
            0x7fc0_0000,
            0xffc0_0000,
            0x7f80_0001,
            0xff80_0001,
            0x7fc1_2345,
            0xffc5_4321,
            0x7fff_ffff,
            0xffff_ffff,
            0x4120_0000,
        ];
        for (i, &left) in edges.iter().enumerate() {
            for (j, &right) in edges.iter().enumerate() {
                self.emit(&mut models, 0, 'S', [left, right, left], 0, 0);
                for op in ['+', '-', '*', '='] {
                    self.emit(
                        &mut models,
                        3,
                        op,
                        [right, edges[(i + j + 7) % edges.len()], left],
                        right,
                        0,
                    );
                }
            }
        }
        for kind in 0..3 {
            // Every input category is tested as stored history, new sample,
            // dt/alpha/cutoff, and configured sample frequency.
            for (i, &left) in edges.iter().enumerate() {
                for (j, &right) in edges.iter().enumerate() {
                    let sample = [right, edges[(i + j + 7) % edges.len()], left];
                    self.emit(&mut models, kind, 'S', [left, right, left], 0, 0);
                    self.emit(&mut models, kind, 'C', [0; 3], left, right);
                    self.emit(&mut models, kind, 'A', sample, right, 0);
                    self.emit(&mut models, kind, 'A', sample, left, 0);
                    self.emit(&mut models, kind, 'R', [0; 3], 0, 0);
                    self.emit(&mut models, kind, 'A', sample, right, 0);
                }
            }
            let mut state = 0x5255_5354_u32 ^ N as u32 ^ kind as u32;
            let mut next = || {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state
            };
            self.emit(&mut models, kind, 'R', [0; 3], 0, 0);
            self.emit(
                &mut models,
                kind,
                'C',
                [0; 3],
                3.0f32.to_bits(),
                100.0f32.to_bits(),
            );
            for index in 0..30_000 {
                let samples = if index < 20_000 {
                    core::array::from_fn(|_| ((next() as i32 % 2_000_001) as f32 / 64.0).to_bits())
                } else {
                    [next(), next(), next()]
                };
                let arg = if index < 20_000 {
                    if kind == 0 {
                        0.125f32.to_bits()
                    } else {
                        0.01f32.to_bits()
                    }
                } else {
                    next()
                };
                if index % 257 == 0 {
                    self.emit(&mut models, kind, 'R', [0; 3], 0, 0);
                }
                if index % 1021 == 0 {
                    self.emit(
                        &mut models,
                        kind,
                        'C',
                        [0; 3],
                        edges[index % edges.len()],
                        edges[(index + 9) % edges.len()],
                    );
                }
                self.emit(&mut models, kind, 'A', samples, arg, 0);
            }
        }
    }
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and C++ compiler; run explicitly"]
fn actual_pinned_cpp_vector_filters_match_every_output_bit() {
    let reference = reference();
    let mut fixture = Fixture::new();
    fixture.dimension::<2>();
    fixture.dimension::<3>();
    let output = reference.run(&fixture.input);
    let actual: Vec<u32> = output
        .split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect();
    assert_eq!(actual.len(), fixture.expected.len());
    let differences: Vec<_> = actual
        .iter()
        .zip(&fixture.expected)
        .enumerate()
        .filter(|(_, (actual, expected))| actual != expected)
        .collect();
    let non_nan_differences = differences
        .iter()
        .filter(|(_, (actual, expected))| {
            !(f32::from_bits(**actual).is_nan() && f32::from_bits(**expected).is_nan())
        })
        .count();
    eprintln!(
        "vector differences: {} NaN-bit differences; {} differences involving non-NaN outputs",
        differences.len() - non_nan_differences,
        non_nan_differences
    );
    for &(index, (actual, expected)) in differences.iter().take(12) {
        eprintln!("output {index}: C++={actual:08x} Rust={expected:08x}");
    }
    assert!(
        differences.is_empty(),
        "{} of {} exact-bit comparisons differ",
        differences.len(),
        actual.len()
    );
    println!("vector filters: {} records, {} exact-bit scalar observations (including NaN payload/sign and signed zero)", fixture.records, actual.len());
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and C++ compiler; run explicitly"]
fn actual_pinned_cpp_vector_finite_histories_match_every_output_bit() {
    let reference = reference();
    let mut fixture = Fixture::new();
    fixture.finite_dimension::<2>();
    fixture.finite_dimension::<3>();
    let output = reference.run(&fixture.input);
    let actual: Vec<u32> = output
        .split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect();
    assert_eq!(actual.len(), fixture.expected.len());
    for (index, (&actual, &expected)) in actual.iter().zip(&fixture.expected).enumerate() {
        assert!(
            f32::from_bits(actual).is_finite() && f32::from_bits(expected).is_finite(),
            "finite fixture poisoned at output {index}"
        );
        assert_eq!(actual, expected, "finite output {index}");
    }
    println!(
        "vector finite histories and operations: {} records, {} exact-bit scalar observations",
        fixture.records,
        actual.len()
    );
}

#[test]
fn documented_profile_selects_nan_payloads_without_rust_codegen_dependence() {
    let older = f32::from_bits(0x7fc1_2345);
    let incoming = f32::from_bits(0xff85_4321);
    let mut digital = DigitalVectorLpf::<3>::new();
    digital.reset_to(Vector::new([older; 3]));
    assert_eq!(
        bits(digital.apply(Vector::new([incoming; 3]), 0.5)),
        [0xffc5_4321; 3]
    );
    let mut variable = VectorLowPassFilter::<3>::new(10.0);
    variable.reset_to(Vector::new([older; 3]));
    assert_eq!(
        bits(variable.apply(Vector::new([incoming; 3]), 0.01)),
        [0xffc5_4321, 0xffc5_4321, 0x7fc1_2345]
    );
    let mut fixed = VectorLowPassFilterConstDt::<3>::new(100.0, 10.0);
    fixed.reset_to(Vector::new([older; 3]));
    assert_eq!(
        bits(fixed.apply(Vector::new([incoming; 3]))),
        [0xffc5_4321, 0xffc5_4321, 0x7fc1_2345]
    );
}

#[test]
fn reference_profile_requires_gradual_underflow_and_round_to_nearest() {
    use std::hint::black_box;
    assert_eq!(
        (black_box(f32::from_bits(1)) * black_box(1.0f32)).to_bits(),
        1,
        "DAZ/FTZ must be disabled"
    );
    let half = f32::from_bits(0x3380_0000);
    assert_eq!(
        (black_box(1.0f32) + black_box(half)).to_bits(),
        1.0f32.to_bits()
    );
    assert_eq!(
        (black_box(-1.0f32) + black_box(-half)).to_bits(),
        (-1.0f32).to_bits()
    );
    assert_eq!(
        (black_box(1.0f32) + black_box(3.0 * half)).to_bits(),
        0x3f80_0002
    );
}

#[test]
#[ignore = "requires pinned GCC reference profile; demonstrates instrumentation changes NaN bits"]
fn actual_pinned_cpp_nan_selection_changes_with_instrumentation() {
    let input = "2 0 S 0 2143289344 0 0 0\n2 0 A 2143289344 4294967295 0 2143289344 0\n";
    let plain = reference_with_flags(&[]).run(input);
    let instrumented = reference_with_flags(&[
        "-fsanitize=undefined,float-cast-overflow",
        "-fno-sanitize-recover=all",
    ])
    .run(input);
    assert_eq!(plain, "0 2143289344 0\n2143289344 4294967295 0\n");
    assert_eq!(instrumented, "0 2143289344 0\n2143289344 2143289344 0\n");
    assert_ne!(
        plain, instrumented,
        "instrumented builds need separate NaN qualification"
    );
}

#[test]
#[ignore = "requires ARDUPILOT_REFERENCE pinned checkout and C++ compiler; run explicitly"]
fn independent_random_nan_histories_coefficients_and_samples() {
    let reference = reference();
    let mut fixture = Fixture::new();
    fn generate<const N: usize>(fixture: &mut Fixture) {
        let mut models = Models::<N>::new();
        let mut rng = 0xbadcafe5u32 ^ N as u32;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        for _ in 0..20000 {
            let samples = core::array::from_fn(|_| next());
            let history = core::array::from_fn(|_| next());
            let alpha = next();
            let cutoff = next();
            let rate = next();
            for kind in 0..3 {
                fixture.emit(&mut models, kind, 'S', history, 0, 0);
                fixture.emit(&mut models, kind, 'C', [0; 3], cutoff, rate);
                fixture.emit(&mut models, kind, 'A', samples, alpha, 0);
            }
        }
        for _ in 0..10000 {
            let samples = core::array::from_fn(|_| (next() & 0x807fffff) | 0x7f800001);
            let history = core::array::from_fn(|_| (next() & 0x807fffff) | 0x7f800001);
            let alpha = (next() & 0x807fffff) | 0x7f800001;
            let cutoff = (next() & 0x807fffff) | 0x7f800001;
            let rate = (next() & 0x807fffff) | 0x7f800001;
            for kind in 0..3 {
                fixture.emit(&mut models, kind, 'S', history, 0, 0);
                fixture.emit(&mut models, kind, 'C', [0; 3], cutoff, rate);
                fixture.emit(&mut models, kind, 'A', samples, alpha, 0);
            }
        }
    }
    generate::<2>(&mut fixture);
    generate::<3>(&mut fixture);
    let actual: Vec<u32> = reference
        .run(&fixture.input)
        .split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect();
    assert_eq!(actual.len(), fixture.expected.len());
    for (i, (cpp, rust)) in actual.iter().zip(&fixture.expected).enumerate() {
        assert_eq!(cpp, rust, "observation {i}: C++={cpp:08x}, Rust={rust:08x}");
    }
    println!(
        "independent fresh-state randomized vector sequences: {} records; {} exact output bits",
        fixture.records,
        actual.len()
    );
}
