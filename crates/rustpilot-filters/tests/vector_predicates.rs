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
    let r = Reference::new("vector-predicates", &[]);
    for h in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        r.copy_verified(&format!("libraries/AP_Math/{h}"), &format!("AP_Math/{h}"));
    }
    r.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    r.write("AP_HAL/AP_HAL.h", format!("#pragma once\n#define HAL_WITH_EKF_DOUBLE {}\n#define AP_MATH_ALLOW_DOUBLE_FUNCTIONS {}\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n",u8::from(ekf),u8::from(allow)));
    let math_h = String::from_utf8(r.read_verified("libraries/AP_Math/AP_Math.h")).unwrap();
    let signature = "template <typename Arithmetic1, typename Arithmetic2>\ntypename std::enable_if<std::is_floating_point<typename std::common_type<Arithmetic1, Arithmetic2>::type>::value, bool>::type\nis_equal(const Arithmetic1 v_1, const Arithmetic2 v_2)";
    let start = math_h
        .find(signature)
        .expect("missing equality declaration");
    let declaration = &math_h[start..start + signature.len() + 1];
    assert!(declaration.ends_with(';'));
    r.write("AP_Math/AP_Math.h", format!("#pragma once\n#include <cmath>\n#include <cfloat>\n#include <cstdint>\n#include <limits>\n#include <type_traits>\nusing std::isnan;\nusing std::isinf;\n#include \"ftype.h\"\n#include \"vector2.h\"\n#include \"vector3.h\"\n{declaration}\n"));
    let math_cpp = String::from_utf8(r.read_verified("libraries/AP_Math/AP_Math.cpp")).unwrap();
    let mut equality = "#include \"AP_Math/AP_Math.h\"\n".to_owned();
    equality.push_str(&extract(&math_cpp, signature));
    for t in ["float", "double"] {
        writeln!(
            equality,
            "\ntemplate bool is_equal<{t}>(const {t}, const {t});"
        )
        .unwrap();
    }
    r.write("equality.cpp", equality);
    let mut implementation =
        "#define HAL_DEBUG_BUILD 1\n#include \"AP_Math/AP_Math.h\"\n".to_owned();
    for n in [2, 3] {
        let source =
            String::from_utf8(r.read_verified(&format!("libraries/AP_Math/vector{n}.cpp")))
                .unwrap();
        for sig in [
            format!("bool Vector{n}<T>::is_nan(void) const"),
            format!("bool Vector{n}<T>::is_inf(void) const"),
            format!("bool Vector{n}<T>::operator ==(const Vector{n}<T> &v) const"),
            format!("bool Vector{n}<T>::operator !=(const Vector{n}<T> &v) const"),
        ] {
            implementation.push_str(&extract(&source, &format!("template <typename T>\n{sig}")));
            implementation.push('\n');
        }
        for t in ["float", "double"] {
            for sig in [
                format!("bool Vector{n}<{t}>::is_nan(void) const"),
                format!("bool Vector{n}<{t}>::is_inf(void) const"),
                format!("bool Vector{n}<{t}>::operator ==(const Vector{n}<{t}> &v) const"),
                format!("bool Vector{n}<{t}>::operator !=(const Vector{n}<{t}> &v) const"),
            ] {
                writeln!(implementation, "template {sig};").unwrap();
            }
        }
    }
    r.write("implementation.cpp", implementation);
    r.compile_sources_with_flags(
        include_str!("vector_predicates_reference.cpp"),
        &["implementation.cpp", "equality.cpp"],
        &[],
    );
    r
}

#[derive(Clone, Copy, Debug)]
struct Case {
    storage: usize,
    n: usize,
    a: [u64; 3],
    b: [u64; 3],
}
impl Case {
    fn input(self) -> String {
        format!(
            "{} {} {} {} {} {} {} {}\n",
            self.storage, self.n, self.a[0], self.a[1], self.a[2], self.b[0], self.b[1], self.b[2]
        )
    }
    fn rust(self, allow: bool) -> Vec<u64> {
        match (self.storage, self.n, allow) {
            (32, 2, _) => self.float::<2>(),
            (32, 3, _) => self.float::<3>(),
            (64, 2, false) => self.double::<2, false>(),
            (64, 3, false) => self.double::<3, false>(),
            (64, 2, true) => self.double::<2, true>(),
            (64, 3, true) => self.double::<3, true>(),
            _ => unreachable!(),
        }
    }
    fn float<const N: usize>(self) -> Vec<u64> {
        let mut a = Vector::<N>::new(core::array::from_fn(|i| f32::from_bits(self.a[i] as u32)));
        let b = Vector::<N>::new(core::array::from_fn(|i| f32::from_bits(self.b[i] as u32)));
        let mut out = vec![
            a.is_nan().into(),
            a.is_inf().into(),
            a.is_zero().into(),
            a.equals(b).into(),
            a.not_equals(b).into(),
        ];
        for i in 0..N {
            out.push(a[i].to_bits() as u64);
        }
        for i in 0..N {
            let slot: &mut f32 = &mut a[i];
            out.push(slot.to_bits() as u64);
        }
        out.extend(a.components.map(|x| x.to_bits() as u64));
        for i in 0..N {
            let slot: &mut f32 = &mut a[i];
            *slot = b.components[i];
            out.extend(a.components.map(|x| x.to_bits() as u64));
        }
        a.zero();
        out.extend(a.components.map(|x| x.to_bits() as u64));
        out.extend(
            Vector::<N>::default()
                .components
                .map(|x| x.to_bits() as u64),
        );
        if N == 3 {
            out.extend(
                Vector::from_xy(
                    Vector::new([
                        f32::from_bits(self.a[0] as u32),
                        f32::from_bits(self.a[1] as u32),
                    ]),
                    f32::from_bits(self.a[2] as u32),
                )
                .components
                .map(|x| x.to_bits() as u64),
            );
        }
        out
    }
    fn double<const N: usize, const ALLOW: bool>(self) -> Vec<u64> {
        let mut a = Vector64::<N>::new(core::array::from_fn(|i| f64::from_bits(self.a[i])));
        let b = Vector64::<N>::new(core::array::from_fn(|i| f64::from_bits(self.b[i])));
        let mut out = vec![
            a.is_nan().into(),
            a.is_inf().into(),
            a.is_zero::<ALLOW>().into(),
            a.equals::<ALLOW>(b).into(),
            a.not_equals::<ALLOW>(b).into(),
        ];
        for i in 0..N {
            out.push(a[i].to_bits());
        }
        for i in 0..N {
            let slot: &mut f64 = &mut a[i];
            out.push(slot.to_bits());
        }
        out.extend(a.components.map(f64::to_bits));
        for i in 0..N {
            let slot: &mut f64 = &mut a[i];
            *slot = b.components[i];
            out.extend(a.components.map(f64::to_bits));
        }
        a.zero();
        out.extend(a.components.map(f64::to_bits));
        out.extend(Vector64::<N>::default().components.map(f64::to_bits));
        if N == 3 {
            out.extend(
                Vector64::from_xy(
                    Vector64::new([f64::from_bits(self.a[0]), f64::from_bits(self.a[1])]),
                    f64::from_bits(self.a[2]),
                )
                .components
                .map(f64::to_bits),
            );
        }
        out
    }
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for storage in [32, 64] {
        let set: &[u64] = if storage == 32 {
            &[
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
                0x7f7f_ffff,
                0xff7f_ffff,
                0x7f80_0000,
                0xff80_0000,
                0x7f80_0001,
                0xff80_0002,
                0x7fc1_2345,
                0xffc5_4321,
            ]
        } else {
            &[
                0,
                0x8000_0000_0000_0000,
                1,
                0x8000_0000_0000_0001,
                0x000f_ffff_ffff_ffff,
                0x0010_0000_0000_0000,
                0x3caf_ffff_ffff_ffff,
                0x3cb0_0000_0000_0000,
                0x3cb0_0000_0000_0001,
                0x3e7f_ffff_ffff_ffff,
                0x3e80_0000_0000_0000,
                0x3e80_0000_0000_0001,
                0xbcaf_ffff_ffff_ffff,
                0xbcb0_0000_0000_0000,
                0xbcb0_0000_0000_0001,
                0xbe7f_ffff_ffff_ffff,
                0xbe80_0000_0000_0000,
                0xbe80_0000_0000_0001,
                0x3fe0_0000_0000_0000,
                0x3ff0_0000_0000_0000,
                0xbff0_0000_0000_0000,
                0x7fef_ffff_ffff_ffff,
                0xffef_ffff_ffff_ffff,
                0x7ff0_0000_0000_0000,
                0xfff0_0000_0000_0000,
                0x7ff0_0000_0000_0001,
                0xfff0_0000_0000_0002,
                0x7ff8_1234_5678_9abc,
                0xfff8_9876_5432_1abc,
            ]
        };
        for n in [2, 3] {
            let mut emit = |a, b| cases.push(Case { storage, n, a, b });
            for &x in set {
                for &y in set {
                    emit([x, y, x], [y, x, y]);
                    emit([x, y, x], [x, y, x]);
                }
                for pos in 0..n {
                    let mut a = [0; 3];
                    a[pos] = x;
                    emit(a, [0; 3]);
                    emit([0; 3], a);
                }
            }
            // Each sign and fraction bit is exercised independently in both
            // operands and every component, including the quiet/signaling bit.
            let width = if storage == 32 { 23 } else { 52 };
            for bit in 0..width {
                for negative in [false, true] {
                    let raw = if storage == 32 {
                        0x7f80_0000 | (1 << bit) | if negative { 1 << 31 } else { 0 }
                    } else {
                        0x7ff0_0000_0000_0000 | (1 << bit) | if negative { 1 << 63 } else { 0 }
                    };
                    for pos in 0..n {
                        let mut a = [0; 3];
                        a[pos] = raw;
                        emit(a, [0; 3]);
                        emit([0; 3], a);
                    }
                }
            }
            if storage == 64 {
                // The float-narrowing zero/equality boundary lies halfway
                // between FLT_EPSILON and its predecessor; test double ULPs.
                let threshold = ((f32::EPSILON as f64)
                    + (f32::from_bits(f32::EPSILON.to_bits() - 1) as f64))
                    / 2.0;
                for delta in [-1_i64, 0, 1] {
                    for sign in [0, 1_u64 << 63] {
                        let raw = (threshold.to_bits() as i64 + delta) as u64 | sign;
                        for pos in 0..n {
                            let mut a = [0; 3];
                            a[pos] = raw;
                            emit(a, [0; 3]);
                            emit([0; 3], a);
                        }
                    }
                }
                for pos in 0..n {
                    let mut a = [0; 3];
                    let mut b = [0; 3];
                    a[pos] = 1073741824.0_f64.to_bits();
                    b[pos] = 1073741825.0_f64.to_bits();
                    emit(a, b);
                    emit(b, a);
                }
            }
            let mut state = 0x6af9_b804_8d3b_a99d_u64;
            for _ in 0..1000 {
                let mut values = [0; 6];
                for v in &mut values {
                    let raw = next(&mut state);
                    *v = if storage == 32 {
                        (raw & 0x807f_ffff) | 0x7f80_0001
                    } else {
                        (raw & 0x800f_ffff_ffff_ffff) | 0x7ff0_0000_0000_0001
                    };
                }
                emit(
                    [values[0], values[1], values[2]],
                    [values[3], values[4], values[5]],
                );
            }
            for _ in 0..1000 {
                let mut a = [0; 3];
                for v in &mut a {
                    let finite = ((next(&mut state) >> 32) as i32 as f64) / 65536.0;
                    *v = if storage == 32 {
                        (finite as f32).to_bits() as u64
                    } else {
                        finite.to_bits()
                    };
                }
                emit(a, a);
                let mut b = a;
                for v in &mut b {
                    *v = v.wrapping_add(1);
                }
                emit(a, b);
            }
            for _ in 0..5000 {
                let mut values = [0; 6];
                for v in &mut values {
                    let raw = next(&mut state);
                    *v = if storage == 32 {
                        raw as u32 as u64
                    } else {
                        raw
                    };
                }
                emit(
                    [values[0], values[1], values[2]],
                    [values[3], values[4], values[5]],
                );
            }
        }
    }
    cases
}

#[test]
#[ignore = "requires pinned upstream source and GCC 16.2.0"]
fn exact_cpp_predicates_copy_and_host_index_observations() {
    let cases = cases();
    let input = cases.iter().map(|c| c.input()).collect::<String>();
    for ekf in [false, true] {
        for allow in [false, true] {
            let r = reference(ekf, allow);
            let text = r.run(&input);
            let mut lines = text.lines();
            let profile = lines.next().expect("missing profile");
            assert!(
                profile.contains("GCC 16.2.0,"),
                "unexpected compiler: {profile}"
            );
            eprintln!("{profile}");
            let lines = lines.collect::<Vec<_>>();
            assert_eq!(lines.len(), cases.len());
            let mut words = 0;
            for (i, (case, line)) in cases.iter().zip(lines).enumerate() {
                let cpp = line
                    .split_whitespace()
                    .map(|s| s.parse::<u64>().unwrap())
                    .collect::<Vec<_>>();
                let rust = case.rust(allow);
                assert_eq!(cpp, rust, "EKF={ekf} ALLOW={allow} case {i}: {case:x?}");
                words += cpp.len();
            }
            eprintln!("EKF={ekf} ALLOW={allow}: {} exact records, {} booleans, {} floating bit patterns, {words} total words", cases.len(), cases.len() * 5, words - cases.len() * 5);
        }
    }
}

#[test]
fn float_predicates_use_strict_epsilon_and_inspect_every_component() {
    let below = f32::from_bits(f32::EPSILON.to_bits() - 1);
    assert!(Vector::new([-below, -0.0]).is_zero());
    assert!(!Vector::new([0.0, f32::EPSILON]).is_zero());
    for i in 0..3 {
        let mut a = Vector::new([0.0; 3]);
        a.components[i] = f32::from_bits(0x7f80_0001);
        assert!(a.is_nan());
        assert!(!a.is_inf());
        assert!(!a.is_zero());
        a.components[i] = f32::NEG_INFINITY;
        assert!(!a.is_nan());
        assert!(a.is_inf());
        assert!(!a.is_zero());
    }
}

#[test]
fn double_zero_and_equality_have_distinct_configuration_thresholds() {
    let below = f64::from_bits((f32::EPSILON as f64).to_bits() - 1);
    let v = Vector64::new([below, -0.0]);
    assert!(v.is_zero::<true>());
    assert!(!v.is_zero::<false>());
    let v = Vector64::new([f64::EPSILON, 0.0]);
    let z = Vector64::new([0.0; 2]);
    assert!(v.is_zero::<true>());
    assert!(v.is_zero::<false>());
    assert!(!v.equals::<true>(z));
    assert!(v.equals::<false>(z));
    assert!(v.not_equals::<true>(z));
    assert!(!v.not_equals::<false>(z));
    let a = Vector64::new([1073741824.0, 0.0]);
    let b = Vector64::new([1073741825.0, 0.0]);
    assert!(!a.equals::<false>(b), "subtract doubles before narrowing");
    assert!(a.not_equals::<false>(b));
    for i in 0..3 {
        let mut a = Vector64::new([0.0; 3]);
        a.components[i] = f64::from_bits(0x7ff0_0000_0000_0001);
        assert!(a.is_nan());
        assert!(!a.is_inf());
        a.components[i] = f64::INFINITY;
        assert!(!a.is_nan());
        assert!(a.is_inf());
    }
}

#[test]
fn named_equality_preserves_rust_partial_eq_and_is_nontransitive() {
    let a = Vector::new([0.0, 0.0]);
    let b = Vector::new([f32::EPSILON * 0.75, 0.0]);
    let c = Vector::new([f32::EPSILON * 1.5, 0.0]);
    assert_ne!(a, b);
    assert!(a.equals(b));
    assert!(b.equals(c));
    assert!(!a.equals(c));
    assert!(a.not_equals(c));
    assert!(!a.not_equals(b));
    let a = Vector::new([f32::INFINITY, 0.0]);
    assert_eq!(a, a);
    assert!(!a.equals(a));
    assert!(a.not_equals(a));
    let a = Vector64::new([f64::INFINITY, 0.0]);
    assert_eq!(a, a);
    assert!(!a.equals::<true>(a));
    assert!(!a.equals::<false>(a));
    assert!(a.not_equals::<true>(a));
    assert!(a.not_equals::<false>(a));
}

#[test]
fn indexing_and_xy_construction_preserve_bits_and_zero_clears_them() {
    let xy = Vector::new([f32::from_bits(0x7f80_0001), -0.0]);
    let mut a = Vector::from_xy(xy, f32::from_bits(0xffc1_2345));
    assert_eq!(
        a.components.map(f32::to_bits),
        [0x7f80_0001, 0x8000_0000, 0xffc1_2345]
    );
    for i in 0..3 {
        assert!(core::ptr::eq(&a[i], &a.components[i]));
        let slot: &mut f32 = &mut a[i];
        *slot = f32::from_bits(0xff80_0010 + i as u32);
        assert_eq!(a.components[i].to_bits(), 0xff80_0010 + i as u32);
    }
    a.zero();
    assert_eq!(a.components.map(f32::to_bits), [0; 3]);
    let xy = Vector64::new([f64::from_bits(0x7ff0_0000_0000_0001), -0.0]);
    let mut a = Vector64::from_xy(xy, f64::from_bits(0xfff8_1234_5678_9abc));
    assert_eq!(
        a.components.map(f64::to_bits),
        [
            0x7ff0_0000_0000_0001,
            0x8000_0000_0000_0000,
            0xfff8_1234_5678_9abc
        ]
    );
    for i in 0..3 {
        assert!(core::ptr::eq(&a[i], &a.components[i]));
        let slot: &mut f64 = &mut a[i];
        *slot = f64::from_bits(0xfff0_0000_0000_0010 + i as u64);
        assert_eq!(a.components[i].to_bits(), 0xfff0_0000_0000_0010 + i as u64);
    }
    a.zero();
    assert_eq!(a.components.map(f64::to_bits), [0; 3]);
    assert_eq!(
        Vector64::<2>::default().components.map(f64::to_bits),
        [0; 2]
    );
    assert_eq!(
        Vector64::<3>::default().components.map(f64::to_bits),
        [0; 3]
    );
}

#[test]
fn out_of_bounds_indexing_panics_for_read_and_write_without_modification() {
    for index in [2, 3, 255, 256, usize::MAX] {
        let mut a = Vector::new([1.0, 2.0]);
        assert!(std::panic::catch_unwind(|| a[index]).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a[index] = 4.0)).is_err());
        assert_eq!(a.components, [1.0, 2.0]);
        let mut a = Vector64::new([1.0, 2.0]);
        assert!(std::panic::catch_unwind(|| a[index]).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a[index] = 4.0)).is_err());
        assert_eq!(a.components, [1.0, 2.0]);
    }
    for index in [3, 255, 256, usize::MAX] {
        let mut a = Vector::new([1.0, 2.0, 3.0]);
        assert!(std::panic::catch_unwind(|| a[index]).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a[index] = 4.0)).is_err());
        assert_eq!(a.components, [1.0, 2.0, 3.0]);
        let mut a = Vector64::new([1.0, 2.0, 3.0]);
        assert!(std::panic::catch_unwind(|| a[index]).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| a[index] = 4.0)).is_err());
        assert_eq!(a.components, [1.0, 2.0, 3.0]);
    }
}
