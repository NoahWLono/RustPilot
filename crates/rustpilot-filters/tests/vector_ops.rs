// SPDX-License-Identifier: GPL-3.0-or-later
use rustpilot_filters::{vector::Vector, vector_math::Vector64};

#[test]
fn finite_arithmetic_and_cross_api() {
    let a = Vector::new([1.0, 2.0, 3.0]);
    let b = Vector::new([4.0, 5.0, 6.0]);
    assert_eq!((-a).components, [-1.0, -2.0, -3.0]);
    let mut c = a;
    c -= b;
    c *= 2.0;
    c *= b;
    assert_eq!(c.components, [-24.0, -30.0, -36.0]);
    assert_eq!((a % b).components, [-3.0, 6.0, -3.0]);
    assert_eq!(a.cross(b), a % b);
    assert_eq!(a.scale(2.0).components, [2.0, 4.0, 6.0]);
    assert_eq!(a.rfu_to_frd().components, [2.0, 1.0, -3.0]);
    assert_eq!(Vector::new([1.0, 2.0]) % Vector::new([4.0, 5.0]), -3.0);
    let a = Vector64::new([1.0, 2.0, 3.0]);
    let b = Vector64::new([4.0, 5.0, 6.0]);
    assert_eq!((-a).components, [-1.0, -2.0, -3.0]);
    assert_eq!((a + b).components, [5.0, 7.0, 9.0]);
    assert_eq!((a - b).components, [-3.0; 3]);
    assert_eq!((a * 2.0).components, [2.0, 4.0, 6.0]);
    let mut c = a;
    c += b;
    c -= b;
    c *= 2.0;
    c *= b;
    assert_eq!(c.components, [8.0, 20.0, 36.0]);
    assert_eq!((a % b).components, [-3.0, 6.0, -3.0]);
    assert_eq!(a.cross(b), a % b);
    assert_eq!(a.scale(2.0).components, [2.0, 4.0, 6.0]);
    assert_eq!(a.rfu_to_frd().components, [2.0, 1.0, -3.0]);
    assert_eq!(Vector64::new([1.0, 2.0]) % Vector64::new([4.0, 5.0]), -3.0);
}

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
    let r = Reference::new("vector-ops", &[]);
    for h in ["vector2.h", "vector3.h", "ftype.h", "rotations.h"] {
        r.copy_verified(&format!("libraries/AP_Math/{h}"), &format!("AP_Math/{h}"));
    }
    r.write(
        "AP_Common/AP_Common.h",
        "#pragma once\n#define WARN_IF_UNUSED\n",
    );
    r.write("AP_HAL/AP_HAL.h", format!("#pragma once\n#define HAL_WITH_EKF_DOUBLE {}\n#define AP_MATH_ALLOW_DOUBLE_FUNCTIONS {}\n#define CONFIG_HAL_BOARD 1\n#define HAL_BOARD_QURT 2\n", u8::from(ekf), u8::from(allow)));
    r.write("AP_Math/AP_Math.h", "#pragma once\n#include <cmath>\n#include <cfloat>\n#include <cstdint>\n#include <type_traits>\nclass AP_Float {};\n#include \"ftype.h\"\n#include \"vector2.h\"\n#include \"vector3.h\"\n");
    let mut implementation =
        "#define HAL_DEBUG_BUILD 1\n#include \"AP_Math/AP_Math.h\"\n".to_owned();
    for n in [2, 3] {
        let source =
            String::from_utf8(r.read_verified(&format!("libraries/AP_Math/vector{n}.cpp")))
                .unwrap();
        let mut sigs = vec![];
        for op in ["+", "-"] {
            sigs.push(format!(
                "Vector{n}<T> Vector{n}<T>::operator {op}(const Vector{n}<T> &v) const"
            ));
            sigs.push(format!(
                "Vector{n}<T> &Vector{n}<T>::operator {op}=(const Vector{n}<T> &v)"
            ));
        }
        for op in ["*", "/"] {
            sigs.push(format!(
                "Vector{n}<T> Vector{n}<T>::operator {op}(const T num) const"
            ));
            sigs.push(format!(
                "Vector{n}<T> &Vector{n}<T>::operator {op}=(const T num)"
            ));
        }
        sigs.push(format!("Vector{n}<T> Vector{n}<T>::operator -(void) const"));
        sigs.push(format!(
            "T Vector{n}<T>::operator *(const Vector{n}<T> &v) const"
        ));
        let cross = if n == 2 {
            "T".into()
        } else {
            format!("Vector{n}<T>")
        };
        sigs.push(format!(
            "{cross} Vector{n}<T>::operator %(const Vector{n}<T> &v) const"
        ));
        for sig in &sigs {
            implementation.push_str(&extract(&source, &format!("template <typename T>\n{sig}")));
            implementation.push('\n');
        }
        for t in ["float", "double"] {
            for sig in &sigs {
                writeln!(
                    implementation,
                    "template {};",
                    sig.replace("<T>", &format!("<{t}>"))
                        .replace(" T ", &format!(" {t} "))
                        .replace("T ", &format!("{t} "))
                )
                .unwrap();
            }
        }
    }
    r.write("implementation.cpp", implementation);
    r.compile_sources_with_flags(
        include_str!("vector_ops_reference.cpp"),
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
    finite: bool,
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
    fn scalars(self) -> usize {
        if "*o".contains(self.op) || (self.n == 2 && "cC".contains(self.op)) {
            1
        } else {
            self.n
        }
    }
    // Returning and assignment contexts have independently observed NaN rules.
    #[allow(clippy::assign_op_pattern)]
    fn rust(self) -> [u64; 3] {
        macro_rules! run {
            ($vector:ident, $f:ident, $bits:ty, $n:tt) => {{
                let mut a =
                    $vector::<$n>::new(core::array::from_fn(|i| $f::from_bits(self.a[i] as $bits)));
                let b =
                    $vector::<$n>::new(core::array::from_fn(|i| $f::from_bits(self.b[i] as $bits)));
                let s = $f::from_bits(self.s as $bits);
                let mut out = [0; 3];
                match self.op {
                    'n' => a = -a,
                    '+' => a = a + b,
                    '-' => a = a - b,
                    'a' => a += b,
                    's' => a -= b,
                    'm' => a = a * s,
                    'M' => a *= s,
                    '/' => a = a / s,
                    '=' => a /= s,
                    '*' => return [(a * b).to_bits() as u64, 0, 0],
                    'o' => return [a.dot(b).to_bits() as u64, 0, 0],
                    'A' => a += a,
                    'B' => a -= a,
                    op => extra!(op, a, b, s, $n),
                }
                for (dst, x) in out.iter_mut().zip(a.components) {
                    *dst = x.to_bits() as u64;
                }
                out
            }};
        }
        macro_rules! extra {
            ($op:ident,$a:ident,$b:ident,$s:ident,2) => {
                match $op {
                    'c' => return [($a % $b).to_bits() as u64, 0, 0],
                    'C' => return [($a % $a).to_bits() as u64, 0, 0],
                    _ => panic!("unknown op"),
                }
            };
            ($op:ident,$a:ident,$b:ident,$s:ident,3) => {
                match $op {
                    'c' => $a = $a % $b,
                    'C' => $a = $a % $a,
                    'x' => $a = $a.cross($b),
                    'S' => $a = $a.scale($s),
                    'p' => $a *= $b,
                    'P' => $a *= $a,
                    'r' => $a = $a.rfu_to_frd(),
                    _ => panic!("unknown op"),
                }
            };
        }
        match (self.storage, self.n) {
            (32, 2) => run!(Vector, f32, u32, 2),
            (32, 3) => run!(Vector, f32, u32, 3),
            (64, 2) => run!(Vector64, f64, u64, 2),
            (64, 3) => run!(Vector64, f64, u64, 3),
            _ => panic!("unsupported"),
        }
    }
}
fn cases() -> Vec<Case> {
    let mut cases = vec![];
    let mut state = 0x46e5_c6b7_1357_2468_u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for storage in [32, 64] {
        let set: Vec<u64> = if storage == 32 {
            vec![
                0, 0x80000000, 1, 0x80000001, 0x007fffff, 0x00800000, 0x3f800000, 0xbf800000,
                0x3f800001, 0x3f7fffff, 0x7f7fffff, 0xff7fffff, 0x7f800000, 0xff800000, 0x7f800001,
                0xff812345, 0x7fc23456, 0xffc34567, 0x34000000, 0x33ffffff, 0x34000001,
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
                0x3ff0000000000001,
                0x3fefffffffffffff,
                0x7fefffffffffffff,
                0xffefffffffffffff,
                0x7ff0000000000000,
                0xfff0000000000000,
                0x7ff0000000000001,
                0xfff0123456789012,
                0x7ff8234567890123,
                0xfff8345678901234,
                0x3cb0000000000000,
                0x3cafffffffffffff,
                0x3cb0000000000001,
            ]
        };
        for n in [2, 3] {
            let mut emit = |v: [u64; 7], finite: bool| {
                let ops = if n == 2 {
                    "n+-asmM/=*ocABC"
                } else {
                    "n+-asmM/=*ocABCxSpPr"
                };
                for op in ops.chars() {
                    cases.push(Case {
                        storage,
                        n,
                        op,
                        a: [v[0], v[1], v[2]],
                        b: [v[3], v[4], v[5]],
                        s: v[6],
                        finite,
                    });
                }
            };
            // Special pairs exercise invalid operations, underflow/overflow,
            // cancellation, rounding neighbours and both operand orientations.
            for &a in &set {
                for &b in &set {
                    emit([a, b, a, b, a, b, b], false);
                }
            }
            // Move every exceptional bit pattern through each independent slot.
            for &v in &set {
                for i in 0..7 {
                    let mut slots = [set[6]; 7];
                    slots[i] = v;
                    emit(slots, false);
                }
            }
            // Every fixed finite/positive signaling/negative quiet NaN choice
            // across all seven independently labelled operand positions.
            for mut code in 0..2187 {
                let mut slots = [0; 7];
                for (i, x) in slots.iter_mut().enumerate() {
                    *x = match code % 3 {
                        0 => set[6],
                        1 => {
                            if storage == 32 {
                                0x7f800001 + i as u64 * 0x12345
                            } else {
                                0x7ff0000000000001 + i as u64 * 0x123456789
                            }
                        }
                        _ => {
                            if storage == 32 {
                                0xffc00007 + i as u64 * 0x23456
                            } else {
                                0xfff8000000000007 + i as u64 * 0x23456789a
                            }
                        }
                    };
                    code /= 3;
                }
                emit(slots, false);
            }
            // Negation and permutation must preserve each NaN fraction bit,
            // including the signaling/quiet distinction, at every position.
            for bit in 0..if storage == 32 { 23 } else { 52 } {
                for sign in [false, true] {
                    let raw = if storage == 32 {
                        0x7f800000 | (1_u64 << bit) | if sign { 1 << 31 } else { 0 }
                    } else {
                        0x7ff0000000000000 | (1_u64 << bit) | if sign { 1 << 63 } else { 0 }
                    };
                    for position in 0..7 {
                        let mut slots = [set[6]; 7];
                        slots[position] = raw;
                        emit(slots, false);
                    }
                }
            }
            for _ in 0..1024 {
                let slots = core::array::from_fn(|_| {
                    let raw = next();
                    if storage == 32 {
                        (raw & 0x807fffff) | 0x7f800001
                    } else {
                        (raw & 0x800fffffffffffff) | 0x7ff0000000000001
                    }
                });
                emit(slots, false);
            }
            // Independent finite signs/mantissas across controlled exponents:
            // retains finite normal/subnormal products and near cancellation.
            for _ in 0..2048 {
                let slots = core::array::from_fn(|_| {
                    let raw = next();
                    if storage == 32 {
                        (raw & 0x807fffff) | ((80 + ((raw >> 32) % 95)) << 23)
                    } else {
                        (raw & 0x800fffffffffffff) | ((600 + ((raw >> 52) % 800)) << 52)
                    }
                });
                emit(slots, true);
            }
            for _ in 0..4096 {
                emit(
                    core::array::from_fn(|_| {
                        let raw = next();
                        if storage == 32 {
                            raw as u32 as u64
                        } else {
                            raw
                        }
                    }),
                    false,
                );
            }
        }
    }
    cases
}
#[test]
#[ignore = "requires pinned upstream source and GCC 16.2.0"]
#[allow(clippy::assign_op_pattern)]
fn exact_cpp_all_arithmetic_operations() {
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
            let mut mismatches =
                std::collections::BTreeMap::<(usize, usize, char), Vec<String>>::new();
            for (i, (case, line)) in cases.iter().zip(lines).enumerate() {
                let cpp = line
                    .split_whitespace()
                    .map(|s| s.parse::<u64>().unwrap())
                    .collect::<Vec<_>>();
                let rust = case.rust();
                if case.finite {
                    for bits in &cpp[..case.scalars()] {
                        assert!(
                            if case.storage == 32 {
                                f32::from_bits(*bits as u32).is_finite()
                            } else {
                                f64::from_bits(*bits).is_finite()
                            },
                            "finite reference became nonfinite: {case:?}"
                        );
                    }
                }
                if cpp != rust {
                    let entries = mismatches
                        .entry((case.storage, case.n, case.op))
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
            let scalars: usize = cases.iter().map(|c| c.scalars()).sum();
            let finite: usize = cases.iter().filter(|c| c.finite).map(|c| c.scalars()).sum();
            eprintln!("EKF={ekf} ALLOW={allow}: {} exact records, {scalars} float observations ({finite} asserted finite), {} output words including padding",cases.len(),cases.len()*3);
        }
    }
}

#[test]
fn negation_and_frame_conversion_preserve_signaling_payloads() {
    let a = Vector::new([0x80000000, 0xff812345, 0x7f800001].map(f32::from_bits));
    assert_eq!(
        (-a).components.map(f32::to_bits),
        [0, 0x7f812345, 0xff800001]
    );
    assert_eq!(
        a.rfu_to_frd().components.map(f32::to_bits),
        [0xff812345, 0x80000000, 0xff800001]
    );
    let a = Vector64::new(
        [0x8000000000000000, 0xfff0123456789012, 0x7ff0000000000001].map(f64::from_bits),
    );
    assert_eq!(
        (-a).components.map(f64::to_bits),
        [0, 0x7ff0123456789012, 0xfff0000000000001]
    );
    assert_eq!(
        a.rfu_to_frd().components.map(f64::to_bits),
        [0xfff0123456789012, 0x8000000000000000, 0xfff0000000000001]
    );
}
#[test]
fn native_assignment_nan_choices_differ_from_returning_operators() {
    let a = Vector64::new([0x7ff0000000000001, 0xfff0123456789012].map(f64::from_bits));
    let b = Vector64::new([0xfff0123456789012, 0x7ff0000000000001].map(f64::from_bits));
    assert_eq!(
        (a + b).components.map(f64::to_bits),
        [0x7ff8000000000001, 0xfff8123456789012]
    );
    let mut assigned = a;
    assigned += b;
    assert_eq!(
        assigned.components.map(f64::to_bits),
        [0xfff8123456789012, 0x7ff8000000000001]
    );
    let mut a = Vector::new([0x7f800001, 0xff812345, 0x7fc23456].map(f32::from_bits));
    a *= f32::from_bits(0xffc34567);
    assert_eq!(
        a.components.map(f32::to_bits),
        [0x7fc00001, 0xffc12345, 0xffc34567]
    );
    let mut a = Vector::new([0x7f800001, 0xff812345, 0x7fc23456].map(f32::from_bits));
    a *= Vector::new([0xffc34567, 0x7f812346, 0x7f812347].map(f32::from_bits));
    assert_eq!(
        a.components.map(f32::to_bits),
        [0xffc34567, 0x7fc12346, 0x7fc23456]
    );
}
#[test]
fn cross_product_retains_reference_nan_choice_and_invalid_results() {
    let a = Vector::new([0x7f800001, 0x3f800000, 0x3f800000].map(f32::from_bits));
    let b = Vector::new([0x3f800000, 0x7f848d15, 0x3f800000].map(f32::from_bits));
    assert_eq!(
        (a % b).components.map(f32::to_bits),
        [0x7fc48d15, 0x7fc00001, 0x7fc48d15]
    );
    let a = Vector::new([f32::INFINITY, 1.0]);
    assert_eq!((a % a).to_bits(), 0xffc00000);
    let a = Vector64::new([f64::MAX, f64::MAX]);
    assert_eq!((a % a).to_bits(), 0xfff8000000000000);
}
#[test]
fn arithmetic_profile_requires_nearest_rounding_and_gradual_underflow() {
    use std::hint::black_box;
    for (bits, scale, want) in [(1, 1.0, 1), (0x00800000, 0.5, 0x00400000)] {
        assert_eq!(
            (black_box(f32::from_bits(bits)) * black_box(scale)).to_bits(),
            want
        );
    }
    for (bits, scale, want) in [(1, 1.0, 1), (0x0010000000000000, 0.5, 0x0008000000000000)] {
        assert_eq!(
            (black_box(f64::from_bits(bits)) * black_box(scale)).to_bits(),
            want
        );
    }
    for sign in [1.0, -1.0] {
        assert_eq!(
            (black_box(sign) + black_box(sign * f32::EPSILON / 2.0)).to_bits(),
            sign.to_bits()
        );
    }
    for sign in [1.0, -1.0] {
        assert_eq!(
            (black_box(sign) + black_box(sign * f64::EPSILON / 2.0)).to_bits(),
            sign.to_bits()
        );
    }
}
