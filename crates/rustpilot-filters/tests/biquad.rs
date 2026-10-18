// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::biquad::{BiquadError, BiquadParams, DigitalBiquadFilter, LowPassFilter2p};
use rustpilot_filters::vector::Vector;

fn bits(params: BiquadParams) -> [u32; 7] {
    [
        params.cutoff_freq,
        params.sample_freq,
        params.a1,
        params.a2,
        params.b0,
        params.b1,
        params.b2,
    ]
    .map(f32::to_bits)
}

#[test]
fn disabled_parameters_preserve_coefficients_and_input_bits() {
    let mut params = BiquadParams {
        cutoff_freq: 1.0,
        sample_freq: 2.0,
        a1: 3.0,
        a2: 4.0,
        b0: 5.0,
        b1: 6.0,
        b2: 7.0,
    };
    DigitalBiquadFilter::<f32>::compute_params(100.0, -0.0, &mut params);
    assert_eq!(
        bits(params),
        [
            0x80000000,
            100.0f32.to_bits(),
            3.0f32.to_bits(),
            4.0f32.to_bits(),
            5.0f32.to_bits(),
            6.0f32.to_bits(),
            7.0f32.to_bits()
        ]
    );
    let mut filter = LowPassFilter2p::<f32>::default();
    for input in [0x80000000, 0x7fa12345, 0xff812345, 0x7f800000] {
        assert_eq!(
            filter.apply(f32::from_bits(input)).unwrap().to_bits(),
            input
        );
    }
}

#[test]
fn cutoff_clamp_uses_double_comparison_and_second_operand_for_nan() {
    let filter = LowPassFilter2p::<f32>::new(100.0, f32::NAN);
    assert_eq!(filter.get_cutoff_freq().to_bits(), 40.0f32.to_bits());
    assert_eq!(filter.get_sample_freq().to_bits(), 100.0f32.to_bits());
    let filter = LowPassFilter2p::<f32>::new(f32::NAN, 1.0);
    assert!(filter.get_cutoff_freq().is_nan());
    let filter = LowPassFilter2p::<f32>::new(100.0, 1000.0);
    assert_eq!(filter.get_cutoff_freq().to_bits(), 40.0f32.to_bits());
}

#[test]
fn reset_then_apply_executes_recurrence() {
    let params = BiquadParams {
        cutoff_freq: 10.0,
        sample_freq: 100.0,
        a1: 0.25,
        a2: 0.5,
        b0: 1.0,
        b1: 2.0,
        b2: 3.0,
    };
    let mut filter = DigitalBiquadFilter::<f32>::new();
    let sample = 7.0;
    let delay = (f64::from(sample) * (1.0 / f64::from(1.0 + params.a1 + params.a2))) as f32;
    let d0 = sample - delay * params.a1 - delay * params.a2;
    let expected = d0 * params.b0 + delay * params.b1 + delay * params.b2;
    assert_eq!(
        filter.apply(sample, &params).unwrap().to_bits(),
        expected.to_bits()
    );
    assert_ne!(expected.to_bits(), sample.to_bits());
}

#[test]
fn scalar_and_vector_reset_have_distinct_rounding() {
    // Upstream scalar reset multiplies in double; its vector operator accepts
    // float and therefore rounds the reciprocal before component products.
    let params = BiquadParams {
        cutoff_freq: 10.0,
        sample_freq: 100.0,
        a1: 0.25,
        a2: 0.5,
        b0: 0.0,
        b1: 1.0,
        b2: 0.0,
    };
    let mut scalar = DigitalBiquadFilter::<f32>::new();
    let mut vector = DigitalBiquadFilter::<Vector<2>>::new();
    let value = f32::from_bits(0x3f800004);
    scalar.reset_to(value, &params).unwrap();
    vector
        .reset_to(
            Vector {
                components: [value, -value],
            },
            &params,
        )
        .unwrap();
    let scalar_out = scalar.apply(0.0, &params).unwrap();
    let vector_out = vector
        .apply(
            Vector {
                components: [0.0, 0.0],
            },
            &params,
        )
        .unwrap();
    assert_eq!(
        scalar_out.to_bits(),
        ((f64::from(value) * (1.0 / 1.75)) as f32).to_bits()
    );
    assert_eq!(
        vector_out.components[0].to_bits(),
        (value * (1.0f64 / 1.75) as f32).to_bits()
    );
    assert_ne!(scalar_out.to_bits(), vector_out.components[0].to_bits());
}

#[test]
fn integer_failure_does_not_commit_pending_initialization_or_history() {
    let mut filter = DigitalBiquadFilter::<i32>::new();
    let overflow = BiquadParams {
        cutoff_freq: 10.0,
        sample_freq: 100.0,
        a1: -1.0,
        a2: 0.0,
        b0: 1.0,
        b1: 0.0,
        b2: 0.0,
    };
    assert_eq!(
        filter.apply(1, &overflow),
        Err(BiquadError::IntegerConversion)
    );
    let identity = BiquadParams {
        a1: 0.0,
        ..overflow
    };
    assert_eq!(filter.apply(17, &identity), Ok(17));
    let output_overflow = BiquadParams {
        b0: f32::MAX,
        ..identity
    };
    assert_eq!(
        filter.apply(2, &output_overflow),
        Err(BiquadError::IntegerConversion)
    );
    let history = BiquadParams {
        b0: 0.0,
        b1: 1.0,
        ..identity
    };
    assert_eq!(filter.apply(99, &history), Ok(17));
    assert_eq!(
        filter.reset_to(1, &overflow),
        Err(BiquadError::IntegerConversion)
    );
    assert_eq!(filter.apply(99, &history), Ok(99));
}

#[test]
fn integer_conversions_truncate_and_check_both_widths() {
    let params = BiquadParams {
        cutoff_freq: 10.0,
        sample_freq: 100.0,
        a1: 0.0,
        a2: 0.0,
        b0: 0.5,
        b1: 0.0,
        b2: 0.0,
    };
    assert_eq!(DigitalBiquadFilter::<i32>::new().apply(-3, &params), Ok(-1));
    assert_eq!(DigitalBiquadFilter::<i64>::new().apply(-3, &params), Ok(-1));
    assert_eq!(
        DigitalBiquadFilter::<i32>::new().apply(i32::MAX, &params),
        Err(BiquadError::IntegerConversion)
    );
    assert_eq!(
        DigitalBiquadFilter::<i64>::new().apply(i64::MAX, &params),
        Err(BiquadError::IntegerConversion)
    );
    assert_eq!(
        DigitalBiquadFilter::<i32>::new().apply(i32::MIN, &params),
        Ok(i32::MIN / 2)
    );
    assert_eq!(
        DigitalBiquadFilter::<i64>::new().apply(i64::MIN, &params),
        Ok(i64::MIN / 2)
    );
}

#[test]
fn disable_enable_and_pending_reset_preserve_required_state() {
    let mut retained = LowPassFilter2p::<f32>::new(100.0, 10.0);
    let mut control = LowPassFilter2p::<f32>::new(100.0, 10.0);
    for sample in [1.0, 7.0, -3.0] {
        assert_eq!(
            retained.apply(sample).unwrap().to_bits(),
            control.apply(sample).unwrap().to_bits()
        );
    }
    retained.set_cutoff_frequency(100.0, 0.0);
    assert_eq!(retained.apply(1234.0), Ok(1234.0));
    retained.set_cutoff_frequency(100.0, 10.0);
    assert_eq!(
        retained.apply(9.0).unwrap().to_bits(),
        control.apply(9.0).unwrap().to_bits()
    );
    retained.reset();
    control.reset();
    retained.set_cutoff_frequency(100.0, 0.0);
    retained.apply(1234.0).unwrap();
    retained.set_cutoff_frequency(100.0, 10.0);
    assert_eq!(
        retained.apply(2.0).unwrap().to_bits(),
        control.apply(2.0).unwrap().to_bits()
    );
}

mod common;
use common::Reference;
use rustpilot_filters::biquad::BiquadSample;
use std::fmt::Write as _;

fn extract(source: &str, signature: &str) -> String {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing source signature {signature}"));
    let end = start + source[start..].find("\n}").unwrap() + 2;
    source[start..end].to_owned()
}

fn assert_hash(path: &std::path::Path, expected: &str) {
    let output = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "cannot hash {}", path.display());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next(),
        Some(expected),
        "unqualified reference bytes at {}",
        path.display()
    );
}

fn reference(native_host_math: bool, sanitized: bool) -> Reference {
    assert_eq!(
        std::env::consts::ARCH,
        "x86_64",
        "this exact NaN profile is x86-64"
    );
    assert_eq!(
        std::env::consts::OS,
        "linux",
        "this reference profile is Linux"
    );
    let source_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/reference/glibc_tanf.c");
    assert_eq!(
        std::fs::read(&source_path).unwrap(),
        include_bytes!("reference/glibc_tanf.c")
    );
    assert_hash(
        &source_path,
        "e63af9308bb8e87d9fcdeba8e0d1ac99435e9ac3c12ffa69993c355a65e1d12b",
    );
    if native_host_math {
        assert_hash(
            std::path::Path::new("/usr/lib/libm.so.6"),
            "965106704753eefe8c31ae7da6daba3ead3d18c52b19b5fcc41178bcc4aa999d",
        );
    }
    let reference = Reference::new("biquad", &["LowPassFilter2p.h", "LowPassFilter2p.cpp"]);
    for header in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        reference.copy_verified(
            &format!("libraries/AP_Math/{header}"),
            &format!("AP_Math/{header}"),
        );
    }
    reference.write("AP_Math/AP_Math.h", include_str!("vector_reference_math.h"));
    reference.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    reference.write("AP_HAL/AP_HAL.h", "#pragma once\n#define HAL_WITH_EKF_DOUBLE 0\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n");
    let mut definitions = String::new();
    for dimension in [2, 3] {
        let source = String::from_utf8(
            reference.read_verified(&format!("libraries/AP_Math/vector{dimension}.cpp")),
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
    let math = String::from_utf8(reference.read_verified("libraries/AP_Math/AP_Math.h")).unwrap();
    let predicates = "struct AP_Float {};\n".to_owned()
        + &extract(
            &math,
            "template <typename T>\ninline bool is_positive(const T fVal1)",
        )
        + "\n"
        + &extract(
            &math,
            "template<typename A, typename B>\nstatic inline auto MIN(",
        )
        + "\n";
    reference.write("biquad_math.cpp", predicates);
    reference.write("glibc_tanf.c", include_str!("reference/glibc_tanf.c"));
    for (name, source) in [
        (
            "array_length.h",
            include_str!("reference/tangent_shims/array_length.h"),
        ),
        (
            "libm-alias-float.h",
            include_str!("reference/tangent_shims/libm-alias-float.h"),
        ),
        (
            "math_config.h",
            include_str!("reference/tangent_shims/math_config.h"),
        ),
        (
            "math_uint128.h",
            include_str!("reference/tangent_shims/math_uint128.h"),
        ),
    ] {
        reference.write(name, source);
    }
    let driver = if native_host_math {
        "#define RUSTPILOT_NATIVE_HOST_MATH 1\n".to_owned() + include_str!("biquad_reference.cpp")
    } else {
        include_str!("biquad_reference.cpp").to_owned()
    };
    let implementation = "#define HAL_DEBUG_BUILD 1\n#include <cstring>\n#include <type_traits>\n#include \"AP_Math/AP_Math.h\"\n#include \"vector_operations.cpp\"\n#include \"biquad_math.cpp\"\n".to_owned()
        + if native_host_math { "" } else { "#define __tanf rustpilot_pinned_tanf\n#include \"glibc_tanf.c\"\n#undef __tanf\n#define tanf rustpilot_pinned_tanf\n" }
        + "#include \"LowPassFilter2p.cpp\"\ntemplate class DigitalBiquadFilter<float>;\n";
    reference.write("implementation.cpp", implementation);
    let original =
        String::from_utf8(reference.read_verified("libraries/Filter/LowPassFilter2p.cpp")).unwrap();
    let start = original
        .find("template <class T>\nDigitalBiquadFilter<T>::DigitalBiquadFilter()")
        .unwrap();
    let end = original
        .find("template <class T>\nLowPassFilter2p<T>::LowPassFilter2p()")
        .unwrap();
    let direct = "#define HAL_DEBUG_BUILD 1\n#include <cstring>\n#include <type_traits>\n#include \"AP_Math/AP_Math.h\"\n#include \"LowPassFilter2p.h\"\n#include \"vector_operations.cpp\"\n#include \"biquad_math.cpp\"\n".to_owned()
        + if native_host_math { "" } else { "float rustpilot_pinned_tanf(float);\n#define tanf rustpilot_pinned_tanf\n" }
        + &original[start..end]
        + "template class DigitalBiquadFilter<int>;\ntemplate class DigitalBiquadFilter<long>;\ntemplate class DigitalBiquadFilter<Vector2f>;\ntemplate class DigitalBiquadFilter<Vector3f>;\n";
    reference.write("direct.cpp", direct);
    let mut flags = vec!["-fno-builtin-fmaf", "-fno-builtin-tanf"];
    if sanitized {
        flags.extend([
            "-fsanitize=undefined,float-cast-overflow",
            "-fno-sanitize-recover=all",
        ]);
    }
    reference.compile_sources_with_flags(&driver, &["implementation.cpp", "direct.cpp"], &flags);
    reference
}

trait FixtureSample: BiquadSample {
    const KIND: usize;
    fn input(self) -> [i64; 3];
    fn output(self) -> Vec<u64>;
}
impl FixtureSample for f32 {
    const KIND: usize = 0;
    fn input(self) -> [i64; 3] {
        [i64::from(self.to_bits()), 0, 0]
    }
    fn output(self) -> Vec<u64> {
        vec![u64::from(self.to_bits())]
    }
}
impl FixtureSample for i32 {
    const KIND: usize = 1;
    fn input(self) -> [i64; 3] {
        [i64::from(self), 0, 0]
    }
    fn output(self) -> Vec<u64> {
        vec![self as u64]
    }
}
impl FixtureSample for i64 {
    const KIND: usize = 2;
    fn input(self) -> [i64; 3] {
        [self, 0, 0]
    }
    fn output(self) -> Vec<u64> {
        vec![self as u64]
    }
}
impl<const N: usize> FixtureSample for Vector<N> {
    const KIND: usize = N + 1;
    fn input(self) -> [i64; 3] {
        core::array::from_fn(|i| {
            if i < N {
                i64::from(self.components[i].to_bits())
            } else {
                0
            }
        })
    }
    fn output(self) -> Vec<u64> {
        self.components
            .iter()
            .map(|v| u64::from(v.to_bits()))
            .collect()
    }
}

#[derive(Default)]
struct Fixture {
    input: String,
    expected: Vec<u64>,
    records: usize,
}
impl Fixture {
    fn emit<T: FixtureSample>(
        &mut self,
        model: &mut LowPassFilter2p<T>,
        op: char,
        sample: T,
        rate: f32,
        cutoff: f32,
    ) {
        let [x, y, z] = sample.input();
        writeln!(
            &mut self.input,
            "{} {op} {x} {y} {z} {} {}",
            T::KIND,
            rate.to_bits(),
            cutoff.to_bits()
        )
        .unwrap();
        let mut output = T::default();
        match op {
            'A' => {
                output = model
                    .apply(sample)
                    .expect("fixture must stay in defined integer domain")
            }
            'S' => model
                .reset_to(sample)
                .expect("fixture reset must stay in defined integer domain"),
            'R' => model.reset(),
            'C' => model.set_cutoff_frequency(rate, cutoff),
            _ => panic!("invalid operation"),
        }
        self.expected
            .extend(bits(*model.parameters()).map(u64::from));
        self.expected.extend(output.output());
        self.records += 1;
    }
    fn direct_emit<T: FixtureSample>(
        &mut self,
        model: &mut DigitalBiquadFilter<T>,
        params: &mut BiquadParams,
        op: char,
        sample: T,
        custom: [f32; 5],
    ) {
        let kind = T::KIND + 6;
        let [x, y, z] = if op == 'B' {
            [
                i64::from(custom[0].to_bits()),
                i64::from(custom[1].to_bits()),
                i64::from(custom[2].to_bits()),
            ]
        } else {
            sample.input()
        };
        writeln!(
            &mut self.input,
            "{kind} {op} {x} {y} {z} {} {}",
            custom[3].to_bits(),
            custom[4].to_bits()
        )
        .unwrap();
        let mut output = T::default();
        match op {
            'A' => {
                output = model
                    .apply(sample, params)
                    .expect("defined direct integer fixture")
            }
            'S' => model
                .reset_to(sample, params)
                .expect("defined direct integer reset fixture"),
            'R' => model.reset(),
            'B' => {
                *params = BiquadParams {
                    cutoff_freq: 10.0,
                    sample_freq: 100.0,
                    a1: custom[0],
                    a2: custom[1],
                    b0: custom[2],
                    b1: custom[3],
                    b2: custom[4],
                }
            }
            _ => panic!("invalid direct operation"),
        }
        self.expected.extend(bits(*params).map(u64::from));
        self.expected.extend(output.output());
        self.records += 1;
    }
    fn direct_floating<T: FixtureSample>(&mut self, convert: impl Fn([f32; 3]) -> T) {
        let mut model = DigitalBiquadFilter::<T>::new();
        let mut params = BiquadParams {
            cutoff_freq: 10.0,
            sample_freq: 100.0,
            ..BiquadParams::default()
        };
        // A concrete reset case where scalar-double and vector-float products
        // differ by one bit, observed through the first delayed contribution.
        self.direct_emit(
            &mut model,
            &mut params,
            'B',
            T::default(),
            [0.25, 0.5, 0.0, 1.0, 0.0],
        );
        let rounding_value = f32::from_bits(0x3f800004);
        self.direct_emit(
            &mut model,
            &mut params,
            'S',
            convert([rounding_value, -rounding_value, rounding_value]),
            [0.0; 5],
        );
        self.direct_emit(&mut model, &mut params, 'A', T::default(), [0.0; 5]);
        let edges = [
            0u32, 0x80000000, 1, 0x80000001, 0x3f800000, 0xbf800000, 0x7f7fffff, 0xff7fffff,
            0x7f800000, 0xff800000, 0x7f812345, 0xff823456, 0x7fc34567, 0xffc45678,
        ];
        for (i, &left) in edges.iter().enumerate() {
            for (j, &right) in edges.iter().enumerate() {
                let custom = [
                    left,
                    right,
                    edges[(i + j + 3) % edges.len()],
                    edges[(i + j + 7) % edges.len()],
                    edges[(i + j + 11) % edges.len()],
                ]
                .map(f32::from_bits);
                self.direct_emit(&mut model, &mut params, 'B', T::default(), custom);
                self.direct_emit(&mut model, &mut params, 'R', T::default(), [0.0; 5]);
                for sample in [
                    [left, right, edges[(i + j + 5) % edges.len()]],
                    [right, left, edges[(i + j + 9) % edges.len()]],
                    [0x7f856789, 0xff86789a, 0x7fc789ab],
                ] {
                    self.direct_emit(
                        &mut model,
                        &mut params,
                        'A',
                        convert(sample.map(f32::from_bits)),
                        [0.0; 5],
                    );
                }
                self.direct_emit(
                    &mut model,
                    &mut params,
                    'S',
                    convert([
                        f32::from_bits(right),
                        f32::from_bits(left),
                        f32::from_bits(0xffc89abc),
                    ]),
                    [0.0; 5],
                );
                self.direct_emit(
                    &mut model,
                    &mut params,
                    'A',
                    convert([
                        f32::from_bits(left),
                        f32::from_bits(right),
                        f32::from_bits(0x7fc9abcd),
                    ]),
                    [0.0; 5],
                );
            }
        }
    }
    fn direct_integers<T: FixtureSample>(&mut self, convert: impl Fn(i32) -> T) {
        let mut model = DigitalBiquadFilter::<T>::new();
        let mut params = BiquadParams {
            cutoff_freq: 10.0,
            sample_freq: 100.0,
            ..BiquadParams::default()
        };
        for custom in [
            [0.0, 0.0, 1.0, 0.0, 0.0],
            [0.5, 0.25, 0.125, 0.25, 0.5],
            [-0.25, 0.5, 0.25, -0.125, 0.75],
        ] {
            self.direct_emit(&mut model, &mut params, 'B', T::default(), custom);
            self.direct_emit(&mut model, &mut params, 'R', T::default(), [0.0; 5]);
            for sample in [-10001, -7, -1, 0, 1, 7, 10001] {
                self.direct_emit(&mut model, &mut params, 'A', convert(sample), [0.0; 5]);
            }
            self.direct_emit(&mut model, &mut params, 'S', convert(17), [0.0; 5]);
            self.direct_emit(&mut model, &mut params, 'A', convert(-17), [0.0; 5]);
        }
    }
    fn direct_integer_boundaries<T: FixtureSample>(&mut self, samples: &[T]) {
        let mut model = DigitalBiquadFilter::<T>::new();
        let mut params = BiquadParams {
            cutoff_freq: 10.0,
            sample_freq: 100.0,
            ..BiquadParams::default()
        };
        self.direct_emit(
            &mut model,
            &mut params,
            'B',
            T::default(),
            [0.0, 0.0, 1.0, 0.0, 0.0],
        );
        for &sample in samples {
            self.direct_emit(&mut model, &mut params, 'S', sample, [0.0; 5]);
            self.direct_emit(&mut model, &mut params, 'A', sample, [0.0; 5]);
            self.direct_emit(&mut model, &mut params, 'R', T::default(), [0.0; 5]);
            self.direct_emit(&mut model, &mut params, 'A', sample, [0.0; 5]);
        }
    }
    fn coefficients(&mut self, include_nonfinite: bool) {
        let mut params = BiquadParams {
            cutoff_freq: 1.0,
            sample_freq: 2.0,
            a1: 3.0,
            a2: 4.0,
            b0: 5.0,
            b1: 6.0,
            b2: 7.0,
        };
        let edges: [u32; 26] = [
            0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x00800000, 0x33ffffff, 0x34000000,
            0x34000001, 0xb4000000, 0x3f800000, 0xbf800000, 0x41200000, 0x42c80000, 0x7f7fffff,
            0xff7fffff, 0x7f800000, 0xff800000, 0x7fc00000, 0xffc00000, 0x7f812345, 0xff812345,
            0x7fc12345, 0xffc12345, 0x7fffffff, 0xffffffff,
        ];
        let limit = if include_nonfinite { edges.len() } else { 16 };
        let mut pairs = Vec::new();
        for &rate in &edges[..limit] {
            for &cutoff in &edges[..limit] {
                pairs.push((rate, cutoff));
            }
        }
        for rate in [1.0f32, 10.0, 50.0, 100.0, 400.0, 1000.0, 8000.0] {
            for fraction in 1..=10000 {
                pairs.push((rate.to_bits(), (rate * fraction as f32 / 10000.0).to_bits()));
            }
        }
        let mut state = 0x52555354u32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for _ in 0..200000 {
            let mut rate = next();
            let mut cutoff = next();
            if !include_nonfinite {
                rate &= 0x7f7fffff;
                cutoff &= 0x7f7fffff;
            }
            pairs.push((rate, cutoff));
        }
        for (rate, cutoff) in pairs {
            writeln!(&mut self.input, "5 P 0 0 0 {rate} {cutoff}").unwrap();
            DigitalBiquadFilter::<f32>::compute_params(
                f32::from_bits(rate),
                f32::from_bits(cutoff),
                &mut params,
            );
            self.expected.extend(bits(params).map(u64::from));
            self.expected.push(0);
            self.records += 1;
        }
    }
    fn floating<T: FixtureSample>(
        &mut self,
        convert: impl Fn([f32; 3]) -> T,
        include_nonfinite: bool,
    ) {
        let mut model = LowPassFilter2p::<T>::default();
        self.emit(&mut model, 'A', convert([-0.0, 0.0, 1.0]), 0.0, 0.0);
        self.emit(&mut model, 'C', T::default(), 100.0, 10.0);
        let mut state = 0x50494c4fu32;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            state
        };
        for i in 0..30000 {
            if i % 257 == 0 {
                self.emit(&mut model, 'R', T::default(), 0.0, 0.0);
            }
            if i % 1021 == 0 {
                self.emit(
                    &mut model,
                    'C',
                    T::default(),
                    100.0,
                    if i % 2 == 0 { 0.0 } else { 3.0 },
                );
            }
            if i % 2081 == 0 {
                self.emit(&mut model, 'S', convert([1.0, -1.0, -0.0]), 0.0, 0.0);
            }
            let samples = core::array::from_fn(|_| (next() as i32 % 2000001) as f32 / 64.0);
            self.emit(&mut model, 'A', convert(samples), 0.0, 0.0);
        }
        let edges: [u32; 14] = [
            0, 0x80000000, 1, 0x80000001, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000,
            0x7fc00000, 0xffc00000, 0x7f812345, 0xff812345, 0x7fc12345, 0xffc12345,
        ];
        let limit = if include_nonfinite { edges.len() } else { 6 };
        self.emit(&mut model, 'C', T::default(), 100.0, 10.0);
        for &left in &edges[..limit] {
            for &right in &edges[..limit] {
                self.emit(
                    &mut model,
                    'S',
                    convert([
                        f32::from_bits(left),
                        f32::from_bits(right),
                        f32::from_bits(left),
                    ]),
                    0.0,
                    0.0,
                );
                self.emit(
                    &mut model,
                    'A',
                    convert([
                        f32::from_bits(right),
                        f32::from_bits(left),
                        f32::from_bits(right),
                    ]),
                    0.0,
                    0.0,
                );
                self.emit(&mut model, 'R', T::default(), 0.0, 0.0);
                self.emit(
                    &mut model,
                    'A',
                    convert([
                        f32::from_bits(left),
                        f32::from_bits(right),
                        f32::from_bits(left),
                    ]),
                    0.0,
                    0.0,
                );
            }
        }
        if include_nonfinite {
            for &rate in &edges {
                for &cutoff in &edges {
                    self.emit(
                        &mut model,
                        'C',
                        T::default(),
                        f32::from_bits(rate),
                        f32::from_bits(cutoff),
                    );
                    self.emit(&mut model, 'R', T::default(), 0.0, 0.0);
                    for &sample in &edges {
                        self.emit(
                            &mut model,
                            'A',
                            convert([
                                f32::from_bits(sample),
                                f32::from_bits(sample ^ 0x80000000),
                                f32::from_bits(sample),
                            ]),
                            0.0,
                            0.0,
                        );
                    }
                }
            }
        }
    }
    fn integers<T: FixtureSample>(&mut self, convert: impl Fn(i32) -> T) {
        let mut model = LowPassFilter2p::<T>::default();
        self.emit(&mut model, 'C', T::default(), 100.0, 10.0);
        let mut state = 0x494e5453u32;
        for i in 0..30000 {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            if i % 257 == 0 {
                self.emit(&mut model, 'R', T::default(), 0.0, 0.0);
            }
            if i % 1021 == 0 {
                self.emit(
                    &mut model,
                    'C',
                    T::default(),
                    100.0,
                    if i % 2 == 0 { 0.0 } else { 10.0 },
                );
            }
            if i % 2081 == 0 {
                self.emit(&mut model, 'S', convert(-57), 0.0, 0.0);
            }
            self.emit(&mut model, 'A', convert(state as i32 % 100001), 0.0, 0.0);
        }
    }
}

fn fixture(include_nonfinite: bool) -> Fixture {
    let mut fixture = Fixture::default();
    fixture.coefficients(include_nonfinite);
    fixture.floating(|v| v[0], include_nonfinite);
    fixture.integers(|v| v);
    fixture.integers(i64::from);
    fixture.floating(|v| Vector::<2>::new([v[0], v[1]]), include_nonfinite);
    fixture.floating(Vector::<3>::new, include_nonfinite);
    if include_nonfinite {
        fixture.direct_floating(|v| v[0]);
        fixture.direct_floating(|v| Vector::<2>::new([v[0], v[1]]));
        fixture.direct_floating(Vector::<3>::new);
        fixture.direct_integers(|v| v);
        fixture.direct_integers(i64::from);
        fixture.direct_integer_boundaries(&[i32::MIN, i32::MAX - 127, -16_777_217, 16_777_217]);
        fixture.direct_integer_boundaries(&[
            i64::MIN,
            9_223_371_487_098_961_920_i64,
            -9_007_199_254_740_993,
            9_007_199_254_740_993,
        ]);
    }
    fixture
}

fn compare(native: bool, include_nonfinite: bool) {
    let reference = reference(native, false);
    let fixture = fixture(include_nonfinite);
    let actual: Vec<u64> = reference
        .run(&fixture.input)
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
    let mut offset = 0;
    for (line_number, line) in fixture.input.lines().enumerate() {
        let kind: usize = line.split_whitespace().next().unwrap().parse().unwrap();
        let width = if kind == 3 || kind == 9 {
            9
        } else if kind == 4 || kind == 10 {
            10
        } else {
            8
        };
        for &(index, (actual, expected)) in differences
            .iter()
            .take(30)
            .filter(|(i, _)| *i >= offset && *i < offset + width)
        {
            eprintln!(
                "record {line_number} ({line}) slot {}: C++={actual:016x} Rust={expected:016x}",
                index - offset
            );
        }
        offset += width;
    }
    assert!(
        differences.is_empty(),
        "{} of {} exact-bit comparisons differ",
        differences.len(),
        actual.len()
    );
    println!("biquad native_math={native} nonfinite_inputs={include_nonfinite}: {} records, {} exact-bit scalar observations", fixture.records, actual.len());
}

#[test]
#[ignore = "requires pinned ARDUPILOT_REFERENCE and C++ compiler; run explicitly"]
fn actual_pinned_cpp_biquad_matches_every_output_bit() {
    compare(false, true);
    if let Some(value) = std::env::var_os("RUSTPILOT_TEST_HOST_GLIBC") {
        assert_eq!(value, "1", "host comparison is enabled only by value 1");
        compare(true, true);
    }
}

#[test]
#[ignore = "requires pinned ARDUPILOT_REFERENCE and C++ compiler; run explicitly"]
fn actual_pinned_cpp_biquad_finite_inputs_match_every_output_bit() {
    compare(false, false);
}

#[test]
#[ignore = "requires pinned checkout and C++ undefined-behavior sanitizers"]
fn actual_pinned_cpp_biquad_domain_has_no_undefined_conversions() {
    let reference = reference(false, true);
    let fixture = fixture(true);
    let output = reference.run(&fixture.input);
    assert_eq!(output.split_whitespace().count(), fixture.expected.len());
    println!("biquad sanitizer check: {} defined-domain records; payload equality belongs to the separate uninstrumented profile", fixture.records);
}

// Retained from the independent review: coefficients and samples are generated
// independently, including a separate corpus containing only NaN operands.
#[test]
#[ignore = "requires pinned ARDUPILOT_REFERENCE and C++ compiler; independent review corpus"]
fn independent_direct_random_params_and_nan_operands() {
    let reference = reference(false, false);
    let mut fixture = Fixture::default();
    fn generate<T: FixtureSample>(f: &mut Fixture, convert: impl Fn([f32; 3]) -> T) {
        let mut model = DigitalBiquadFilter::<T>::new();
        let mut params = BiquadParams::default();
        let mut rng = 0xaaaabbbbu32 ^ T::KIND as u32;
        let mut next = || {
            rng ^= rng << 13;
            rng ^= rng >> 17;
            rng ^= rng << 5;
            rng
        };
        for forced in [false, true] {
            for _ in 0..5000 {
                let mut value = || {
                    let bits = next();
                    f32::from_bits(if forced {
                        (bits & 0x807fffff) | 0x7f800001
                    } else {
                        bits
                    })
                };
                let custom = core::array::from_fn(|_| value());
                let sample = core::array::from_fn(|_| value());
                let reset = core::array::from_fn(|_| value());
                let followup = core::array::from_fn(|_| value());
                f.direct_emit(&mut model, &mut params, 'B', T::default(), custom);
                f.direct_emit(&mut model, &mut params, 'R', T::default(), [0.0; 5]);
                f.direct_emit(&mut model, &mut params, 'A', convert(sample), [0.0; 5]);
                f.direct_emit(&mut model, &mut params, 'S', convert(reset), [0.0; 5]);
                f.direct_emit(&mut model, &mut params, 'A', convert(followup), [0.0; 5]);
            }
        }
    }
    generate(&mut fixture, |v| v[0]);
    generate(&mut fixture, |v| Vector::<2>::new([v[0], v[1]]));
    generate(&mut fixture, Vector::<3>::new);
    let actual: Vec<u64> = reference
        .run(&fixture.input)
        .split_whitespace()
        .map(|word| word.parse().unwrap())
        .collect();
    assert_eq!(actual.len(), fixture.expected.len());
    for (i, (cpp, rust)) in actual.iter().zip(&fixture.expected).enumerate() {
        assert_eq!(
            cpp, rust,
            "observation {i}: C++={cpp:016x}, Rust={rust:016x}"
        );
    }
    println!(
        "independent direct biquad: {} records, {} exact-bit observations",
        fixture.records,
        actual.len()
    );
}
