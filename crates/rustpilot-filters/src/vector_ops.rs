// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot AP_Math/vector{2,3}.{h,cpp}, revision
// afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Copyright 2010 Michael Smith; Copyright (C) Andrew Tridgell 2012;
// derived from Bill Perone's 2003 vector classes.
//! Component arithmetic, cross products, and right/front/up conversion.
//!
//! Arithmetic uses component storage precision, independently of `ftype`.
//! Exact output bits follow the GCC 16.2.0 x86-64 reference profile in
//! `VECTOR_OPS_REPORT.md`. Only dimensions two and three are qualified.
//! Other generic dimensions are Rust extensions without comparison evidence.
//! Unary negation and frame conversion preserve signaling NaN payloads.

use crate::vector::{mul_first, sub_first, Vector};
use crate::vector_math::Vector64;
use core::ops::{Add, AddAssign, Mul, MulAssign, Neg, Rem, Sub, SubAssign};

// Rust leaves NaN payload choice unspecified. The reference SSE destination
// wins, quiets its NaN, and retains its sign and payload. Invalid arithmetic
// without a NaN input produces the negative default NaN.
fn result64(lhs: f64, rhs: f64, finite: impl FnOnce() -> f64) -> f64 {
    if lhs.is_nan() {
        return f64::from_bits(lhs.to_bits() | 0x0008_0000_0000_0000);
    }
    if rhs.is_nan() {
        return f64::from_bits(rhs.to_bits() | 0x0008_0000_0000_0000);
    }
    let value = finite();
    if value.is_nan() {
        f64::from_bits(0xfff8_0000_0000_0000)
    } else {
        value
    }
}
fn add64(lhs: f64, rhs: f64) -> f64 {
    result64(lhs, rhs, || lhs + rhs)
}
fn sub64(lhs: f64, rhs: f64) -> f64 {
    result64(lhs, rhs, || lhs - rhs)
}
fn mul64(lhs: f64, rhs: f64) -> f64 {
    result64(lhs, rhs, || lhs * rhs)
}

impl<const N: usize> Neg for Vector<N> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(
            self.components
                .map(|x| f32::from_bits(x.to_bits() ^ 0x8000_0000)),
        )
    }
}
impl<const N: usize> SubAssign for Vector<N> {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.components[i] = sub_first(self.components[i], rhs.components[i]);
        }
    }
}
impl<const N: usize> MulAssign<f32> for Vector<N> {
    fn mul_assign(&mut self, rhs: f32) {
        for (i, x) in self.components.iter_mut().enumerate() {
            // Scalar assignment has a different destination from returning
            // multiplication: the scalar wins in Vector2 and Vector3 z.
            *x = if N == 2 || (N == 3 && i == 2) {
                mul_first(rhs, *x)
            } else {
                mul_first(*x, rhs)
            };
        }
    }
}
impl<const N: usize> Neg for Vector64<N> {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(
            self.components
                .map(|x| f64::from_bits(x.to_bits() ^ 0x8000_0000_0000_0000)),
        )
    }
}
impl<const N: usize> Add for Vector64<N> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(core::array::from_fn(|i| {
            add64(self.components[i], rhs.components[i])
        }))
    }
}
impl<const N: usize> Sub for Vector64<N> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(core::array::from_fn(|i| {
            sub64(self.components[i], rhs.components[i])
        }))
    }
}
impl<const N: usize> AddAssign for Vector64<N> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..N {
            // GCC loads the rhs into the Vector2 packed-add destination.
            // Vector3, and the returning operator, keep the lhs destination.
            self.components[i] = if N == 2 {
                add64(rhs.components[i], self.components[i])
            } else {
                add64(self.components[i], rhs.components[i])
            };
        }
    }
}
impl<const N: usize> SubAssign for Vector64<N> {
    fn sub_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.components[i] = sub64(self.components[i], rhs.components[i]);
        }
    }
}
impl<const N: usize> Mul<f64> for Vector64<N> {
    type Output = Self;
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.components.map(|x| mul64(x, rhs)))
    }
}
impl<const N: usize> MulAssign<f64> for Vector64<N> {
    fn mul_assign(&mut self, rhs: f64) {
        for x in &mut self.components {
            // Both double assignment instantiations prefer the scalar NaN.
            *x = mul64(rhs, *x);
        }
    }
}
macro_rules! cross_and_frame {
    ($vector:ident, $scalar:ident, $mul:ident, $sub:ident, $sign:expr, $float_profile:expr) => {
        impl Rem for $vector<2> {
            type Output = $scalar;
            fn rem(self, rhs: Self) -> $scalar {
                let [x, y] = self.components;
                let [u, v] = rhs.components;
                $sub($mul(x, v), $mul(y, u))
            }
        }
        impl Rem for $vector<3> {
            type Output = Self;
            fn rem(self, rhs: Self) -> Self {
                let [x, y, z] = self.components;
                let [u, v, w] = rhs.components;
                Self::new([
                    $sub($mul(y, w), $mul(z, v)),
                    $sub($mul(z, u), $mul(x, w)),
                    // In the float cross z expression GCC places rhs.y in
                    // the first multiply destination; double prefers lhs.x.
                    $sub(
                        if $float_profile {
                            $mul(v, x)
                        } else {
                            $mul(x, v)
                        },
                        $mul(y, u),
                    ),
                ])
            }
        }
        impl MulAssign for $vector<3> {
            fn mul_assign(&mut self, rhs: Self) {
                for i in 0..3 {
                    // This header-defined entry point prefers rhs except
                    // for float z, unlike returning scalar multiplication.
                    self.components[i] = if $float_profile && i == 2 {
                        $mul(self.components[i], rhs.components[i])
                    } else {
                        $mul(rhs.components[i], self.components[i])
                    };
                }
            }
        }
        impl $vector<3> {
            /// Right-handed vector cross product in storage precision.
            pub fn cross(self, rhs: Self) -> Self {
                self % rhs
            }
            /// Scale all components by the storage-precision scalar.
            pub fn scale(self, scalar: $scalar) -> Self {
                self * scalar
            }
            /// Reorder right/front/up into front/right/down (also ENU to NED).
            /// Negation flips only the z sign bit, including for signaling NaNs.
            pub fn rfu_to_frd(self) -> Self {
                let [x, y, z] = self.components;
                Self::new([y, x, $scalar::from_bits(z.to_bits() ^ $sign)])
            }
        }
    };
}
cross_and_frame!(Vector, f32, mul_first, sub_first, 0x8000_0000, true);
cross_and_frame!(Vector64, f64, mul64, sub64, 0x8000_0000_0000_0000, false);
