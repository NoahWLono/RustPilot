// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::{vector::Vector, vector_math::Vector64};
use std::fmt::Write as _;
mod common;
use common::Reference;

fn extract(source: &str, signature: &str) -> String {
    let start = source
        .find(signature)
        .unwrap_or_else(|| panic!("missing {signature}"));
    let end = start + source[start..].find("\n}").unwrap() + 2;
    source[start..end].to_owned()
}
fn reference(ekf: bool, allow: bool) -> Reference {
    let r = Reference::new("vector-geometry", &[]);
    for h in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        r.copy_verified(&format!("libraries/AP_Math/{h}"), &format!("AP_Math/{h}"));
    }
    r.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    r.write("AP_HAL/AP_HAL.h", format!("#pragma once\n#define HAL_WITH_EKF_DOUBLE {}\n#define AP_MATH_ALLOW_DOUBLE_FUNCTIONS {}\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n",u8::from(ekf),u8::from(allow)));
    r.write("AP_Math/AP_Math.h", "#pragma once\n#include <cmath>\n#include <cfloat>\n#include <cstdint>\n#include <type_traits>\nclass AP_Float {};\n#include \"ftype.h\"\n#include \"vector2.h\"\n#include \"vector3.h\"\n");
    let mut implementation =
        "#define HAL_DEBUG_BUILD 1\n#include \"AP_Math/AP_Math.h\"\n".to_owned();
    for n in [2, 3] {
        let source =
            String::from_utf8(r.read_verified(&format!("libraries/AP_Math/vector{n}.cpp")))
                .unwrap();
        let mut sigs = vec![
            format!("T Vector{n}<T>::operator *(const Vector{n}<T> &v) const"),
            format!("Vector{n}<T> Vector{n}<T>::operator *(const T num) const"),
            format!("Vector{n}<T> Vector{n}<T>::operator /(const T num) const"),
            format!("Vector{n}<T> Vector{n}<T>::operator -(const Vector{n}<T> &v) const"),
        ];
        if n == 2 {
            sigs.extend([
            "void Vector2<T>::project(const Vector2<T> &v)".into(),
            "Vector2<T> Vector2<T>::projected(const Vector2<T> &v) const".into(),
            "void Vector2<T>::reflect(const Vector2<T> &n)".into(),
            "Vector2<T> Vector2<T>::perpendicular(const Vector2<T> &pos_delta, const Vector2<T> &v1)".into(),
        ]);
        }
        for sig in sigs {
            implementation.push_str(&extract(&source, &format!("template <typename T>\n{sig}")));
            implementation.push('\n');
        }
        for t in ["float", "double"] {
            for sig in [
                format!("{t} Vector{n}<{t}>::operator *(const Vector{n}<{t}>&) const"),
                format!("Vector{n}<{t}> Vector{n}<{t}>::operator *({t}) const"),
                format!("Vector{n}<{t}> Vector{n}<{t}>::operator /({t}) const"),
                format!("Vector{n}<{t}> Vector{n}<{t}>::operator -(const Vector{n}<{t}>&) const"),
            ] {
                writeln!(implementation, "template {sig};").unwrap();
            }
            if n == 2 {
                for sig in [
                format!("void Vector2<{t}>::project(const Vector2<{t}>&)"),
                format!("Vector2<{t}> Vector2<{t}>::projected(const Vector2<{t}>&) const"),
                format!("void Vector2<{t}>::reflect(const Vector2<{t}>&)"),
                format!("Vector2<{t}> Vector2<{t}>::perpendicular(const Vector2<{t}>&,const Vector2<{t}>&)"),
            ] { writeln!(implementation,"template {sig};").unwrap(); }
            }
        }
    }
    r.write("implementation.cpp", implementation);
    r.compile_sources_with_flags(
        include_str!("vector_geometry_reference.cpp"),
        &["implementation.cpp"],
        &[],
    );
    r
}

#[test]
fn projection_and_axis_reflection() {
    let a = Vector::new([2.0, 3.0]);
    let d = Vector::new([1.0, 0.0]);
    assert_eq!(a.projected(d).components, [2.0, 0.0]);
    let mut p = a;
    p.project(d);
    assert_eq!(p.components, [2.0, 0.0]);
    let mut r = a;
    r.reflect(d);
    assert_eq!(r.components, [2.0, -3.0]);
    let a = Vector64::new([2.0, 3.0, 4.0]);
    let d = Vector64::new([0.0, 0.0, 2.0]);
    assert_eq!(a.projected(d).components, [0.0, 0.0, 4.0]);
    let mut r = a;
    r.reflect(d);
    assert_eq!(r.components, [-2.0, -3.0, 4.0]);
}
#[test]
fn perpendicular_ties_unordered_and_early_return() {
    assert_eq!(
        Vector::<2>::perpendicular(Vector::new([0.0, 0.0]), Vector::new([1.0, 2.0])).components,
        [2.0, -1.0]
    );
    let d = Vector64::new([f64::from_bits(0x7ff1_2345_6789_abcd), -0.0]);
    let r = Vector64::<2>::perpendicular(Vector64::new([1.0, 1.0]), d);
    assert_eq!(
        r.components.map(f64::to_bits),
        [(-0.0_f64).to_bits(), 0xfff1_2345_6789_abcd]
    );
    let p = Vector::new([-0.0, 0.0, -0.0]);
    assert_eq!(
        Vector::<3>::perpendicular(p, Vector::new([1.0, 2.0, 3.0]))
            .components
            .map(f32::to_bits),
        p.components.map(f32::to_bits)
    );
    let p = Vector64::new([
        f64::from_bits((f32::EPSILON as f64).to_bits() - 1),
        -0.0,
        0.0,
    ]);
    let d = Vector64::new([1.0, 0.0, 0.0]);
    assert_eq!(
        Vector64::<3>::perpendicular::<true>(p, d)
            .components
            .map(f64::to_bits),
        p.components.map(f64::to_bits)
    );
    assert_eq!(
        Vector64::<3>::perpendicular::<false>(p, d).components[0].to_bits(),
        0
    );
}
#[test]
fn projection_multiplies_before_dividing_and_zero_direction_is_unguarded() {
    let a = Vector::new([1.0, 2.0]);
    let d = Vector::new([3.0, 4.0]);
    assert_eq!(a.projected(d).components[0].to_bits(), 0x3fa8_f5c3);
    assert_eq!(
        (d.components[0] * (a.dot(d) / d.dot(d))).to_bits(),
        0x3fa8_f5c2
    );
    let a = Vector64::new([3.0, 7.0, 0.0]);
    let d = Vector64::new([5.0, 11.0, 0.0]);
    assert_eq!(
        a.projected(d).components[0].to_bits(),
        0x4009_349a_4d26_934a
    );
    assert_eq!(
        (d.components[0] * (a.dot(d) / d.dot(d))).to_bits(),
        0x4009_349a_4d26_9349
    );
    let a = Vector::new([1.0e20, 0.0]);
    let d = Vector::new([1.0e10, 0.0]);
    assert!(a.projected(d).components[0].is_infinite());
    assert!((d.components[0] * (a.dot(d) / d.dot(d))).is_finite());
    let a = Vector64::new([1.0e200, 0.0, 0.0]);
    let d = Vector64::new([1.0e100, 0.0, 0.0]);
    assert!(a.projected(d).components[0].is_infinite());
    assert!((d.components[0] * (a.dot(d) / d.dot(d))).is_finite());
    assert_eq!(
        Vector::new([1.0, 2.0])
            .projected(Vector::new([-0.0, 0.0]))
            .components
            .map(f32::to_bits),
        [0xffc0_0000; 2]
    );
}

#[test]
fn two_dimensional_method_contexts_have_distinct_nan_priority() {
    let a = Vector::new([f32::from_bits(0x7f81_2345), f32::from_bits(0xff82_3456)]);
    let b = Vector::new([f32::from_bits(0x7f83_4567), f32::from_bits(0xff84_5678)]);
    assert_eq!(
        a.projected(b).components.map(f32::to_bits),
        [0x7fc3_4567; 2]
    );
    let mut reflected = a;
    reflected.reflect(b);
    assert_eq!(reflected.components.map(f32::to_bits), [0x7fc1_2345; 2]);
    let mut alias = a;
    alias.project(alias);
    assert_eq!(alias.components.map(f32::to_bits), [0x7fc1_2345; 2]);
    let a = Vector64::new([
        f64::from_bits(0x7ff1_2345_6789_abcd),
        f64::from_bits(0xfff2_3456_789a_bcde),
    ]);
    let b = Vector64::new([
        f64::from_bits(0x7ff3_4567_89ab_cdef),
        f64::from_bits(0xfff4_5678_9abc_def0),
    ]);
    assert_eq!(
        a.projected(b).components.map(f64::to_bits),
        [0x7ffb_4567_89ab_cdef; 2]
    );
    let mut reflected = a;
    reflected.reflect(b);
    assert_eq!(
        reflected.components.map(f64::to_bits),
        [0x7ff9_2345_6789_abcd; 2]
    );
    let mut alias = a;
    alias.reflect(alias);
    assert_eq!(
        alias.components.map(f64::to_bits),
        [0x7ff9_2345_6789_abcd; 2]
    );
}

#[test]
#[should_panic(expected = "only Vector2 and Vector3 are qualified")]
fn unsupported_float_dimension_is_rejected() {
    Vector::new([1.0; 4]).project(Vector::new([1.0; 4]));
}

#[test]
#[should_panic(expected = "only Vector2 and Vector3 are qualified")]
fn unsupported_double_dimension_is_rejected() {
    Vector64::new([1.0; 1]).reflect(Vector64::new([1.0; 1]));
}

#[derive(Clone, Copy, Debug)]
struct Case {
    storage: usize,
    n: usize,
    op: char,
    a: [u64; 3],
    b: [u64; 3],
}
impl Case {
    fn input(self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {}\n",
            self.storage,
            self.n,
            self.op,
            self.a[0],
            self.a[1],
            self.a[2],
            self.b[0],
            self.b[1],
            self.b[2]
        )
    }
    fn run32<const N: usize>(self) -> [u64; 3] {
        let mut a = Vector::<N>::new(core::array::from_fn(|i| f32::from_bits(self.a[i] as u32)));
        let b = Vector::new(core::array::from_fn(|i| f32::from_bits(self.b[i] as u32)));
        match self.op {
            'p' => a.project(b),
            'P' => a = a.projected(b),
            'r' => a.reflect(b),
            's' => a.project(a),
            't' => a.reflect(a),
            'n' => {
                if N == 2 {
                    let p = Vector::<2>::perpendicular(
                        Vector::new([a.components[0], a.components[1]]),
                        Vector::new([b.components[0], b.components[1]]),
                    );
                    a.components[..2].copy_from_slice(&p.components);
                } else {
                    let p = Vector::<3>::perpendicular(
                        Vector::new([a.components[0], a.components[1], a.components[2]]),
                        Vector::new([b.components[0], b.components[1], b.components[2]]),
                    );
                    a.components[..3].copy_from_slice(&p.components);
                }
            }
            _ => panic!(),
        }
        let mut out = [0; 3];
        for (i, x) in a.components.iter().enumerate() {
            out[i] = u64::from(x.to_bits());
        }
        out
    }
    fn run64<const N: usize, const A: bool>(self) -> [u64; 3] {
        let mut a = Vector64::<N>::new(core::array::from_fn(|i| f64::from_bits(self.a[i])));
        let b = Vector64::new(core::array::from_fn(|i| f64::from_bits(self.b[i])));
        match self.op {
            'p' => a.project(b),
            'P' => a = a.projected(b),
            'r' => a.reflect(b),
            's' => a.project(a),
            't' => a.reflect(a),
            'n' => {
                if N == 2 {
                    let p = Vector64::<2>::perpendicular(
                        Vector64::new([a.components[0], a.components[1]]),
                        Vector64::new([b.components[0], b.components[1]]),
                    );
                    a.components[..2].copy_from_slice(&p.components);
                } else {
                    let p = Vector64::<3>::perpendicular::<A>(
                        Vector64::new([a.components[0], a.components[1], a.components[2]]),
                        Vector64::new([b.components[0], b.components[1], b.components[2]]),
                    );
                    a.components[..3].copy_from_slice(&p.components);
                }
            }
            _ => panic!(),
        }
        let mut out = [0; 3];
        for (i, x) in a.components.iter().enumerate() {
            out[i] = x.to_bits();
        }
        out
    }
    fn run(self, allow: bool) -> [u64; 3] {
        match (self.storage, self.n, allow) {
            (32, 2, _) => self.run32::<2>(),
            (32, 3, _) => self.run32::<3>(),
            (64, 2, true) => self.run64::<2, true>(),
            (64, 2, false) => self.run64::<2, false>(),
            (64, 3, true) => self.run64::<3, true>(),
            (64, 3, false) => self.run64::<3, false>(),
            _ => panic!(),
        }
    }
}
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for storage in [32, 64] {
        for n in [2, 3] {
            let cv = |x: f64| {
                if storage == 32 {
                    u64::from((x as f32).to_bits())
                } else {
                    x.to_bits()
                }
            };
            let special = if storage == 32 {
                vec![
                    0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x00800000, 0x3f800000, 0xbf800000,
                    0x40000000, 0x34000000, 0x33ffffff, 0x34000001, 0x7f7fffff, 0xff7fffff,
                    0x7f800000, 0xff800000, 0x7f812345, 0xff823456, 0x7fc34567, 0xffc45678,
                ]
            } else {
                vec![
                    0,
                    0x8000000000000000,
                    1,
                    0x8000000000000001,
                    0x000fffffffffffff,
                    0x0010000000000000,
                    0x3ff0000000000000,
                    0xbff0000000000000,
                    0x4000000000000000,
                    0x3e80000000000000,
                    0x3e7fffffffffffff,
                    0x3e80000000000001,
                    0x7fefffffffffffff,
                    0xffefffffffffffff,
                    0x7ff0000000000000,
                    0xfff0000000000000,
                    0x7ff123456789abcd,
                    0xfff23456789abcde,
                    0x7ffb456789abcdef,
                    0xfffc56789abcdef0,
                ]
            };
            let mut emit = |a, b| {
                for op in "pPrnst".chars() {
                    cases.push(Case {
                        storage,
                        n,
                        op,
                        a,
                        b,
                    });
                }
            };
            for &x in &special {
                for &y in &special {
                    emit([x, y, x], [y, x, y]);
                }
            }
            for pos in 0..2 * n {
                for other in 0..2 * n {
                    for &x in &special[16..] {
                        for &y in &special[16..] {
                            let mut v = [cv(1.0); 6];
                            v[pos] = x;
                            v[other] = y;
                            emit(
                                [v[0], v[1], if n == 3 { v[2] } else { 0 }],
                                [v[n], v[n + 1], if n == 3 { v[5] } else { 0 }],
                            );
                        }
                    }
                }
            }
            // Threshold neighbors with exact dot=d, including the double value
            // below FLT_EPSILON that rounds up when double functions are disabled.
            for &x in &special[9..12] {
                for sign in [cv(0.0), cv(-0.0)] {
                    emit([x, sign, sign], [cv(1.0), 0, 0]);
                }
            }
            for &x in &special {
                emit([cv(1.0), x, cv(-2.0)], [0, 0, 0]);
                emit([cv(1.0), x, cv(-2.0)], [cv(-0.0); 3]);
            }
            let big = if storage == 32 { 1e20 } else { 1e200 };
            let direction = if storage == 32 { 1e10 } else { 1e100 };
            emit([cv(big), 0, 0], [cv(direction), 0, 0]);
            emit([cv(1.0), cv(2.0), 0], [cv(3.0), cv(4.0), 0]);
            emit([cv(3.0), cv(7.0), 0], [cv(5.0), cv(11.0), 0]);
            let mut state = 0x7132_a9dc_b55f_dac9u64;
            for mode in 0..4 {
                for _ in 0..1600 {
                    let mut v = [0; 6];
                    for x in &mut v {
                        state ^= state << 13;
                        state ^= state >> 7;
                        state ^= state << 17;
                        *x = match mode {
                            0 => {
                                if storage == 32 {
                                    (state & 0x807fffff) | 0x7f800001
                                } else {
                                    (state & 0x800fffffffffffff) | 0x7ff0000000000001
                                }
                            }
                            1 => {
                                if storage == 32 {
                                    state & 0xffffffff
                                } else {
                                    state
                                }
                            }
                            2 => cv((state as i32 as f64) / 1048576.0),
                            _ => {
                                if state & 3 == 0 {
                                    special[(state as usize >> 2) % special.len()]
                                } else if storage == 32 {
                                    (state & 0x807fffff) | 0x7f800001
                                } else {
                                    (state & 0x800fffffffffffff) | 0x7ff0000000000001
                                }
                            }
                        };
                    }
                    emit([v[0], v[1], v[2]], [v[3], v[4], v[5]]);
                }
            }
        }
    }
    cases
}
#[test]
#[ignore = "requires pinned upstream source and GCC 16.2.0"]
fn actual_upstream_geometry_matches_every_output_bit() {
    let cases = cases();
    let components: usize = cases.iter().map(|case| case.n).sum();
    let input: String = cases.iter().map(|c| c.input()).collect();
    for ekf in [false, true] {
        for allow in [false, true] {
            let r = reference(ekf, allow);
            let output = r.run(&input);
            let mut lines = output.lines();
            let banner = lines.next().unwrap();
            assert!(banner.contains("GCC 16.2.0"), "{banner}");
            eprintln!("{banner}");
            let lines: Vec<_> = lines.collect();
            assert_eq!(lines.len(), cases.len());
            let mut failures = 0;
            let mut counts = std::collections::BTreeMap::new();
            for (i, (&case, line)) in cases.iter().zip(&lines).enumerate() {
                let expected: Vec<u64> = line
                    .split_whitespace()
                    .map(|s| s.parse().unwrap())
                    .collect();
                let actual = case.run(allow);
                if expected != actual {
                    let count = counts.entry((case.storage, case.n, case.op)).or_insert(0);
                    if *count < 2 {
                        eprintln!("case {i} {case:?}: expected {expected:x?}, actual {actual:x?}");
                    }
                    failures += 1;
                    *count += 1;
                }
            }
            if failures != 0 {
                eprintln!("failures {counts:?}");
            }
            assert_eq!(failures, 0, "EKF={ekf} ALLOW={allow}");
            eprintln!(
                "{} geometry cases ({components} output components) matched exactly",
                cases.len()
            );
        }
    }
}
