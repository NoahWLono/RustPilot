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
    let r = Reference::new("vector-math", &[]);
    for h in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        r.copy_verified(&format!("libraries/AP_Math/{h}"), &format!("AP_Math/{h}"));
    }
    r.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    r.write("AP_HAL/AP_HAL.h", format!("#pragma once\n#define HAL_WITH_EKF_DOUBLE {}\n#define AP_MATH_ALLOW_DOUBLE_FUNCTIONS {}\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n",u8::from(ekf),u8::from(allow)));
    let math = String::from_utf8(r.read_verified("libraries/AP_Math/AP_Math.h")).unwrap();
    let mut shim="#pragma once\n#include <cmath>\n#include <cfloat>\n#include <cstdint>\n#include <type_traits>\nclass AP_Float {};\n#include \"ftype.h\"\n#include \"vector2.h\"\n#include \"vector3.h\"\n".to_owned();
    for signature in [
        "template <typename T>\ninline bool is_positive(const T fVal1)",
        "inline bool is_positive(const double fVal1)",
        "template<typename T>\nftype sq(const T val)",
        "static inline constexpr float sq(const float val)",
        "template<typename T, typename... Params>\nftype sq(const T first, const Params... parameters)",
        "template<typename T, typename U, typename... Params>\nftype norm(const T first, const U second, const Params... parameters)",
        "inline float norm(const float first, const float second)",
        "inline float norm(const float first, const float second, const float third)",
    ] { shim.push_str(&extract(&math,signature)); shim.push('\n'); }
    r.write("AP_Math/AP_Math.h", shim);
    let mut implementation =
        "#define HAL_DEBUG_BUILD 1\n#include \"AP_Math/AP_Math.h\"\n".to_owned();
    for n in [2, 3] {
        let source =
            String::from_utf8(r.read_verified(&format!("libraries/AP_Math/vector{n}.cpp")))
                .unwrap();
        let mut sigs = vec![
            format!("T Vector{n}<T>::operator *(const Vector{n}<T> &v) const"),
            format!("T Vector{n}<T>::length(void) const"),
            format!("Vector{n}<T> &Vector{n}<T>::operator /=(const T num)"),
            format!("Vector{n}<T> Vector{n}<T>::operator /(const T num) const"),
        ];
        if n == 2 {
            sigs.extend([
                "T Vector2<T>::length_squared() const".into(),
                "bool Vector2<T>::limit_length(T max_length)".into(),
                "void Vector2<T>::normalize()".into(),
                "Vector2<T> Vector2<T>::normalized() const".into(),
            ]);
        } else {
            sigs.push("bool Vector3<T>::limit_length_xy(T max_length)".into());
        }
        for sig in sigs {
            implementation.push_str(&extract(&source, &format!("template <typename T>\n{sig}")));
            implementation.push('\n');
        }
        for t in ["float", "double"] {
            for sig in [
                format!("{t} Vector{n}<{t}>::operator *(const Vector{n}<{t}>&) const"),
                format!("{t} Vector{n}<{t}>::length() const"),
                format!("Vector{n}<{t}>& Vector{n}<{t}>::operator /=({t})"),
                format!("Vector{n}<{t}> Vector{n}<{t}>::operator /({t}) const"),
            ] {
                writeln!(implementation, "template {sig};").unwrap();
            }
            if n == 2 {
                for sig in [
                    format!("{t} Vector2<{t}>::length_squared() const"),
                    format!("bool Vector2<{t}>::limit_length({t})"),
                    format!("void Vector2<{t}>::normalize()"),
                    format!("Vector2<{t}> Vector2<{t}>::normalized() const"),
                ] {
                    writeln!(implementation, "template {sig};").unwrap();
                }
            } else {
                writeln!(
                    implementation,
                    "template bool Vector3<{t}>::limit_length_xy({t});"
                )
                .unwrap();
            }
        }
    }
    r.write("implementation.cpp", implementation);
    r.compile_sources_with_flags(
        include_str!("vector_math_reference.cpp"),
        &["implementation.cpp"],
        &[],
    );
    r
}
#[derive(Clone, Copy, Debug)]
struct Case {
    storage: usize,
    n: usize,
    op: char,
    a: [u64; 3],
    b: [u64; 3],
    s: u64,
}
impl Case {
    fn input(self) -> String {
        format!(
            "{} {} {} {} {} {} {} {} {} {}\n",
            self.storage,
            self.n,
            self.op,
            self.a[0],
            self.a[1],
            self.a[2],
            self.b[0],
            self.b[1],
            self.b[2],
            self.s
        )
    }
    fn rust(self, ekf: bool) -> [u64; 4] {
        if self.storage == 32 {
            if self.n == 2 {
                self.float::<2>()
            } else {
                self.float::<3>()
            }
        } else if ekf {
            if self.n == 2 {
                self.double::<2, true>()
            } else {
                self.double::<3, true>()
            }
        } else if self.n == 2 {
            self.double::<2, false>()
        } else {
            self.double::<3, false>()
        }
    }
    // Exercise Div separately from DivAssign; their upstream call contexts differ.
    #[allow(clippy::assign_op_pattern)]
    fn float<const N: usize>(self) -> [u64; 4] {
        let mut a = Vector::<N>::new(core::array::from_fn(|i| f32::from_bits(self.a[i] as u32)));
        let b = Vector::<N>::new(core::array::from_fn(|i| f32::from_bits(self.b[i] as u32)));
        let s = f32::from_bits(self.s as u32);
        let mut out = [0; 4];
        match self.op {
            '/' => a = a / s,
            '=' => a /= s,
            '*' => out[0] = (a * b).to_bits() as u64,
            'o' => out[0] = a.dot(b).to_bits() as u64,
            's' => out[0] = a.length_squared().to_bits() as u64,
            'l' => out[0] = a.length().to_bits() as u64,
            'm' => a.normalize(),
            'n' => a = a.normalized(),
            'k' => {
                if N == 2 {
                    let mut v = Vector::new([a.components[0], a.components[1]]);
                    out[3] = v.limit_length(s) as u64;
                    a.components[..2].copy_from_slice(&v.components);
                } else {
                    let mut v = Vector::new([a.components[0], a.components[1], a.components[2]]);
                    out[3] = v.limit_length_xy(s) as u64;
                    a.components[..3].copy_from_slice(&v.components);
                }
            }
            'd' => {
                let v = Vector::new([a.components[0], a.components[1], a.components[2]]);
                out[0] = v
                    .distance_squared(Vector::new([
                        b.components[0],
                        b.components[1],
                        b.components[2],
                    ]))
                    .to_bits() as u64;
            }
            'f' => a = a.tofloat(),
            'g' => {
                let v = a.todouble();
                for (i, x) in v.components.iter().enumerate() {
                    out[i] = x.to_bits();
                }
                return out;
            }
            _ => panic!(),
        }
        if "/=mnkf".contains(self.op) {
            for (i, x) in a.components.iter().enumerate() {
                out[i] = x.to_bits() as u64;
            }
        }
        out
    }
    // Exercise Div separately from DivAssign; their upstream call contexts differ.
    #[allow(clippy::assign_op_pattern)]
    fn double<const N: usize, const E: bool>(self) -> [u64; 4] {
        let mut a = Vector64::<N>::new(core::array::from_fn(|i| f64::from_bits(self.a[i])));
        let b = Vector64::<N>::new(core::array::from_fn(|i| f64::from_bits(self.b[i])));
        let s = f64::from_bits(self.s);
        let mut out = [0; 4];
        match self.op {
            '/' => a = a / s,
            '=' => a /= s,
            '*' => out[0] = (a * b).to_bits(),
            'o' => out[0] = a.dot(b).to_bits(),
            's' => out[0] = a.length_squared().to_bits(),
            'l' => out[0] = a.length::<E>().to_bits(),
            'm' => a.normalize::<E>(),
            'n' => a = a.normalized::<E>(),
            'k' => {
                if N == 2 {
                    let mut v = Vector64::new([a.components[0], a.components[1]]);
                    out[3] = v.limit_length::<E>(s) as u64;
                    a.components[..2].copy_from_slice(&v.components);
                } else {
                    let mut v = Vector64::new([a.components[0], a.components[1], a.components[2]]);
                    out[3] = v.limit_length_xy::<E>(s) as u64;
                    a.components[..3].copy_from_slice(&v.components);
                }
            }
            'd' => {
                let v = Vector64::new([a.components[0], a.components[1], a.components[2]]);
                out[0] = v
                    .distance_squared(Vector64::new([
                        b.components[0],
                        b.components[1],
                        b.components[2],
                    ]))
                    .to_bits();
            }
            'g' => a = a.todouble(),
            'f' => {
                let v = a.tofloat();
                for (i, x) in v.components.iter().enumerate() {
                    out[i] = x.to_bits() as u64;
                }
                return out;
            }
            _ => panic!(),
        }
        if "/=mnkg".contains(self.op) {
            for (i, x) in a.components.iter().enumerate() {
                out[i] = x.to_bits();
            }
        }
        out
    }
}
fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let sets: [Vec<u64>; 2] = [
        vec![
            0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x00800000, 0x3f800000, 0xbf800000,
            0x40000000, 0x40400000, 0x40800000, 0x34000000, 0x33ffffff, 0x34000001, 0x7f7fffff,
            0xff7fffff, 0x7f800000, 0xff800000, 0x7f812345, 0xff823456, 0x7fc34567, 0xffc45678,
        ],
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
            0x4008000000000000,
            0x4010000000000000,
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
            0x36a0000000000000,
            0x3690000000000000,
            0x380fffffe0000000,
            0x47efffffe0000000,
            0x47effffff0000000,
        ],
    ];
    for (j, storage) in [32, 64].iter().enumerate() {
        for n in [2, 3] {
            let set = &sets[j];
            let mut emit = |a, b, s| {
                for op in "/=*oslmnkfgd".chars() {
                    if n == 2 && op == 'd' {
                        continue;
                    }
                    cases.push(Case {
                        storage: *storage,
                        n,
                        op,
                        a,
                        b,
                        s,
                    });
                }
            };
            for &x in set {
                for &y in set {
                    emit([x, y, x], [y, x, y], y);
                }
            }
            // Put independent distinct NaNs in every pair of input positions.
            for pos in 0..7 {
                for other in 0..7 {
                    for &x in &set[18..22] {
                        for &y in &set[18..22] {
                            let mut v = [set[6]; 7];
                            v[pos] = x;
                            v[other] = y;
                            emit([v[0], v[1], v[2]], [v[3], v[4], v[5]], v[6]);
                        }
                    }
                }
            }
            // Three distinct exceptional inputs test complete payload priority chains.
            for &x in &set[18..22] {
                for &y in &set[18..22] {
                    for &z in &set[18..22] {
                        emit([x, y, z], [z, x, y], x);
                    }
                }
            }
            if *storage == 64 {
                emit(
                    [0x3ffbb08d0fcc4337, 0x3e500898dce86dbc, 0x3e5f80fcd84a82ce],
                    [0; 3],
                    1.0_f64.to_bits(),
                );
                emit([1e20_f64.to_bits(), 0, 0], [0; 3], 1.0_f64.to_bits());
                // Exact float midpoints and their double neighbors, both signs.
                for bits in [
                    0_u32, 1, 0x007fffff, 0x00800000, 0x3f7fffff, 0x3f800000, 0x3fffffff,
                    0x4affffff, 0x7f7ffffe,
                ] {
                    let midpoint =
                        ((f32::from_bits(bits) as f64) + (f32::from_bits(bits + 1) as f64)) / 2.0;
                    for delta in [-1_i64, 0, 1] {
                        for sign in [0, 1_u64 << 63] {
                            let b = (midpoint.to_bits() as i64 + delta) as u64 | sign;
                            emit([b, b, b], [0; 3], 1.0_f64.to_bits());
                        }
                    }
                }
                // Overflow midpoint is max_float + half an ulp at its exponent.
                let middle = (f32::MAX as f64) + 2.0_f64.powi(103);
                for delta in [-1_i64, 0, 1] {
                    for sign in [0, 1_u64 << 63] {
                        let b = (middle.to_bits() as i64 + delta) as u64 | sign;
                        emit([b, b, b], [0; 3], 1.0_f64.to_bits());
                    }
                }
            }
            let mut state = 0x71d4_51d9_9a12_66c3_u64;
            // Fresh independent NaN payloads in all seven input operands.
            for _ in 0..2000 {
                let mut v = [0; 7];
                for x in &mut v {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    *x = if *storage == 32 {
                        (state as u32 as u64 & 0x807f_ffff) | 0x7f80_0001
                    } else {
                        (state & 0x800f_ffff_ffff_ffff) | 0x7ff0_0000_0000_0001
                    };
                }
                emit([v[0], v[1], v[2]], [v[3], v[4], v[5]], v[6]);
            }
            // Every NaN fraction bit reaches both directions of conversion.
            let width = if *storage == 32 { 23 } else { 52 };
            for bit in 0..width {
                for negative in [false, true] {
                    let raw = if *storage == 32 {
                        0x7f80_0000 | (1 << bit) | if negative { 1 << 31 } else { 0 }
                    } else {
                        0x7ff0_0000_0000_0000 | (1 << bit) | if negative { 1 << 63 } else { 0 }
                    };
                    for pos in 0..3 {
                        let mut a = [set[6]; 3];
                        a[pos] = raw;
                        emit(a, [set[6]; 3], set[6]);
                    }
                }
            }
            // Bounded finite independent components retain useful finite arithmetic.
            for _ in 0..4000 {
                let mut v = [0; 7];
                for x in &mut v {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    let finite = ((state >> 32) as i32 as f64) / 65536.0;
                    *x = if *storage == 32 {
                        (finite as f32).to_bits() as u64
                    } else {
                        finite.to_bits()
                    };
                }
                emit([v[0], v[1], v[2]], [v[3], v[4], v[5]], v[6]);
            }
            for _ in 0..12000 {
                let mut v = [0; 7];
                for x in &mut v {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    *x = if *storage == 32 {
                        state as u32 as u64
                    } else {
                        state
                    };
                }
                emit([v[0], v[1], v[2]], [v[3], v[4], v[5]], v[6]);
            }
        }
    }
    cases
}
#[test]
fn finite_magnitude_contracts() {
    let v = Vector::new([3.0, 4.0]);
    assert_eq!(v.length(), 5.0);
    assert_eq!(v.normalized().components, [0.6, 0.8]);
    let mut v = Vector::new([3.0, 4.0, f32::from_bits(0x7f812345)]);
    assert!(v.limit_length_xy(-5.0));
    assert_eq!(
        v.components.map(f32::to_bits),
        [(-3.0_f32).to_bits(), (-4.0_f32).to_bits(), 0x7f812345]
    );
    let v = Vector64::new([3.0, 4.0]);
    assert_eq!(v.length::<false>(), 5.0);
    assert_eq!(v.length::<true>(), 5.0);
}
#[test]
fn length_precision_and_association_are_explicit() {
    let v = Vector64::new([1e20, 0.0]);
    assert_eq!(v.length::<false>().to_bits(), f64::INFINITY.to_bits());
    assert_eq!(v.length::<true>(), 1e20);
    let v = Vector64::new(
        [0x3ffbb08d0fcc4337, 0x3e500898dce86dbc, 0x3e5f80fcd84a82ce].map(f64::from_bits),
    );
    assert_eq!(v.length::<true>().to_bits(), 0x3ffbb08d0fcc4338);
}
#[test]
fn length_and_squared_length_keep_distinct_nan_choices() {
    let v = Vector::new([0x7f800001, 0xff800002, 0x7fc12345].map(f32::from_bits));
    assert_eq!(v.length().to_bits(), 0x7fc12345);
    assert_eq!(v.length_squared().to_bits(), 0x7fc00001);
    let v = Vector64::new(
        [0x7ff0000000000001, 0xfff0000000000002, 0x7ff8123456789012].map(f64::from_bits),
    );
    assert_eq!(v.length::<false>().to_bits(), 0xfff8000000000000);
    assert_eq!(v.length::<true>().to_bits(), 0x7ff8000000000001);
}
#[test]
fn normalization_divides_zero_and_limit_obeys_epsilon() {
    let v = Vector::new([-0.0, 0.0, -0.0]).normalized();
    assert_eq!(v.components.map(f32::to_bits), [0xffc00000; 3]);
    let v = Vector64::new([-0.0, 0.0]).normalized::<true>();
    assert_eq!(v.components.map(f64::to_bits), [0xfff8000000000000; 2]);
    let mut below = Vector::new([f32::from_bits(f32::EPSILON.to_bits() - 1), 0.0]);
    let original = below.components.map(f32::to_bits);
    assert!(!below.limit_length(-1.0));
    assert_eq!(below.components.map(f32::to_bits), original);
    let mut at = Vector::new([f32::EPSILON, 0.0]);
    assert!(at.limit_length(-1.0));
    assert_eq!(
        at.components.map(f32::to_bits),
        [(-1.0_f32).to_bits(), (-0.0_f32).to_bits()]
    );
    let mut below = Vector64::new([f64::from_bits((f32::EPSILON as f64).to_bits() - 1), 0.0]);
    assert!(!below.limit_length::<true>(-1.0));
    assert!(below.limit_length::<false>(-1.0));
}
#[test]
#[ignore = "requires pinned upstream source and GCC 16.2.0"]
fn exact_cpp_all_magnitude_operations() {
    let cases = cases();
    let input = cases.iter().map(|c| c.input()).collect::<String>();
    for ekf in [false, true] {
        for allow in [false, true] {
            let r = reference(ekf, allow);
            let text = r.run(&input);
            let mut lines = text.lines();
            let profile = lines.next().expect("missing reference profile");
            assert!(
                profile.contains("GCC 16.2.0,"),
                "unexpected compiler: {profile}"
            );
            eprintln!("{profile}");
            let lines = lines.collect::<Vec<_>>();
            assert_eq!(lines.len(), cases.len());
            let mut mismatches =
                std::collections::BTreeMap::<(usize, usize, char, bool), Vec<String>>::new();
            for (i, (case, line)) in cases.iter().zip(lines).enumerate() {
                let cpp = line
                    .split_whitespace()
                    .map(|s| s.parse::<u64>().unwrap())
                    .collect::<Vec<_>>();
                let rust = case.rust(ekf);
                if cpp != rust {
                    let entries = mismatches
                        .entry((
                            case.storage,
                            case.n,
                            case.op,
                            case.a[..case.n].iter().any(|&x| {
                                if case.storage == 32 {
                                    f32::from_bits(x as u32).is_nan()
                                } else {
                                    f64::from_bits(x).is_nan()
                                }
                            }),
                        ))
                        .or_default();
                    if entries.len() < 3 {
                        entries.push(format!("case {i} {case:?}: C++ {cpp:x?}, Rust {rust:x?}"));
                    }
                }
            }
            assert!(
                mismatches.is_empty(),
                "EKF={ekf} ALLOW={allow}\n{}",
                mismatches
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            let scalars: usize = cases
                .iter()
                .map(|c| if "/=mnkfg".contains(c.op) { c.n } else { 1 })
                .sum();
            let flags = cases.iter().filter(|c| c.op == 'k').count();
            eprintln!("EKF={ekf} ALLOW={allow}: {} exact records, {scalars} float results, {flags} flags, {} output words including padding",cases.len(),cases.len()*4);
        }
    }
}
