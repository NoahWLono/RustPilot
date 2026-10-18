// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot AP_Math/vector{2,3}.{h,cpp}, AP_Math.h and
// ftype.h, revision afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Copyright 2010 Michael Smith; Copyright (C) Andrew Tridgell 2012;
// derived from Bill Perone's 2003 vector classes.
//! Vector magnitudes, division, normalization, limiting and conversions.
//!
//! The output-bit contract is the GCC 16.2.0 x86-64 profile documented in
//! `VECTOR_MATH_REPORT.md`. Only dimensions two and three are supported here.
//! The double-vector norm uses explicit `HAL_WITH_EKF_DOUBLE` selection because
//! upstream `ftype` controls its intermediate precision. Squared length and
//! dot products always use the component storage precision. Equality retains
//! ordinary Rust exact floating-point comparisons, unlike upstream epsilon
//! equality; no equality compatibility is claimed.

use crate::square_root::{sqrt, sqrtf};
use crate::vector::{add_first, mul_first, sub_first, Vector};
use core::ops::{Div, DivAssign, Mul};

/// Double-precision component storage; no C++ ABI guarantee.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector64<const N: usize> {
    /// Components in x, y, then z order.
    pub components: [f64; N],
}
/// Upstream two-component double representation.
pub type Vector2d = Vector64<2>;
/// Upstream three-component double representation.
pub type Vector3d = Vector64<3>;

fn supported<const N: usize>() {
    assert!(N == 2 || N == 3, "only Vector2 and Vector3 are qualified");
}
// Match the destination-operand NaN selection of the recorded SSE profile.
// Invalid arithmetic without an input NaN produces the negative default NaN.
fn result64(lhs: f64, rhs: f64, op: impl FnOnce() -> f64) -> f64 {
    if lhs.is_nan() {
        return f64::from_bits(lhs.to_bits() | 0x0008_0000_0000_0000);
    }
    if rhs.is_nan() {
        return f64::from_bits(rhs.to_bits() | 0x0008_0000_0000_0000);
    }
    let v = op();
    if v.is_nan() {
        f64::from_bits(0xfff8_0000_0000_0000)
    } else {
        v
    }
}
fn add64(a: f64, b: f64) -> f64 {
    result64(a, b, || a + b)
}
fn sub64(a: f64, b: f64) -> f64 {
    result64(a, b, || a - b)
}
fn mul64(a: f64, b: f64) -> f64 {
    result64(a, b, || a * b)
}
fn div64(a: f64, b: f64) -> f64 {
    result64(a, b, || a / b)
}
fn div32(a: f32, b: f32) -> f32 {
    if a.is_nan() {
        return f32::from_bits(a.to_bits() | 0x0040_0000);
    }
    if b.is_nan() {
        return f32::from_bits(b.to_bits() | 0x0040_0000);
    }
    let v = a / b;
    if v.is_nan() {
        f32::from_bits(0xffc0_0000)
    } else {
        v
    }
}
// SSE conversions quiet NaNs and preserve sign plus the representable payload.
// Rust casts do not promise these payload choices, so handle NaNs by bits.
fn widen(v: f32) -> f64 {
    if v.is_nan() {
        f64::from_bits(
            ((v.to_bits() as u64 & 0x8000_0000) << 32)
                | 0x7ff8_0000_0000_0000
                | ((v.to_bits() as u64 & 0x007f_ffff) << 29),
        )
    } else {
        v as f64
    }
}
fn narrow(v: f64) -> f32 {
    if v.is_nan() {
        f32::from_bits(
            ((v.to_bits() >> 32) as u32 & 0x8000_0000)
                | 0x7fc0_0000
                | ((v.to_bits() >> 29) as u32 & 0x007f_ffff),
        )
    } else {
        v as f32
    }
}
// In the standalone float Vector3 length method the final z-square occupies
// the add destination: z wins over x, then y. Swapping the final add operands
// preserves finite addition association while fixing the observed NaN choice.
fn norm32<const N: usize>(a: [f32; N]) -> f32 {
    supported::<N>();
    let mut sum = mul_first(a[0], a[0]);
    for (i, &x) in a[1..].iter().enumerate() {
        sum = if i == 1 {
            add_first(mul_first(x, x), sum)
        } else {
            add_first(sum, mul_first(x, x))
        };
    }
    sqrtf(sum)
}
// Generic AP_Math sq is right recursive. For double storage with float ftype,
// GCC chooses the inner y/z sum as the outer add destination (y, then z, then x).
// With double ftype, x wins, followed by y and z.
fn norm64<const N: usize, const E: bool>(a: [f64; N]) -> f64 {
    supported::<N>();
    if E {
        let mut sum = mul64(a[N - 1], a[N - 1]);
        for &x in a[..N - 1].iter().rev() {
            sum = add64(mul64(x, x), sum);
        }
        sqrt(sum)
    } else {
        let a = a.map(narrow);
        let mut sum = mul_first(a[N - 1], a[N - 1]);
        for (i, &x) in a[..N - 1].iter().rev().enumerate() {
            sum = if N == 3 && i == 1 {
                add_first(sum, mul_first(x, x))
            } else {
                add_first(mul_first(x, x), sum)
            };
        }
        widen(sqrtf(sum))
    }
}
impl<const N: usize> Vector<N> {
    /// Dot product in storage precision and source expression order.
    pub fn dot(self, rhs: Self) -> f32 {
        supported::<N>();
        let mut sum = mul_first(self.components[0], rhs.components[0]);
        for i in 1..N {
            sum = add_first(sum, mul_first(self.components[i], rhs.components[i]));
        }
        sum
    }
    /// Squared length in storage precision.
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }
    /// Euclidean length using float squares and float square root.
    pub fn length(self) -> f32 {
        norm32(self.components)
    }
    /// Divide every component by the length, including for zero vectors.
    pub fn normalize(&mut self) {
        *self /= self.length();
    }
    /// Return a component-wise division by this vector's length.
    pub fn normalized(self) -> Self {
        self / self.length()
    }
    /// Preserve all component bits, including signaling NaNs.
    pub const fn tofloat(self) -> Self {
        self
    }
    /// Convert each component to double with the reference NaN payload mapping.
    pub fn todouble(self) -> Vector64<N> {
        supported::<N>();
        Vector64::new(self.components.map(widen))
    }
}
impl<const N: usize> Div<f32> for Vector<N> {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        supported::<N>();
        Self::new(self.components.map(|x| div32(x, rhs)))
    }
}
impl<const N: usize> DivAssign<f32> for Vector<N> {
    fn div_assign(&mut self, rhs: f32) {
        *self = *self / rhs;
    }
}
impl<const N: usize> Mul for Vector<N> {
    type Output = f32;
    fn mul(self, rhs: Self) -> f32 {
        self.dot(rhs)
    }
}
impl Vector<2> {
    /// Scale to `max_length` when length exceeds it and reaches FLT_EPSILON.
    /// Negative limits retain the source behavior and can reverse direction.
    pub fn limit_length(&mut self, max_length: f32) -> bool {
        let len = self.length();
        if len > max_length && len >= f32::EPSILON {
            let scale = div32(max_length, len);
            for x in &mut self.components {
                *x = mul_first(*x, scale);
            }
            true
        } else {
            false
        }
    }
}
impl Vector<3> {
    /// Limit the xy length while preserving every z bit.
    pub fn limit_length_xy(&mut self, max_length: f32) -> bool {
        let len = norm32([self.components[0], self.components[1]]);
        if len > max_length && len >= f32::EPSILON {
            let scale = div32(max_length, len);
            for x in &mut self.components[..2] {
                *x = mul_first(*x, scale);
            }
            true
        } else {
            false
        }
    }
    /// Squared component distance in source subtraction and addition order.
    pub fn distance_squared(self, rhs: Self) -> f32 {
        let d = core::array::from_fn(|i| sub_first(self.components[i], rhs.components[i]));
        Vector::<3>::new(d).length_squared()
    }
}
impl<const N: usize> Vector64<N> {
    /// Construct without modifying any component bits.
    pub const fn new(components: [f64; N]) -> Self {
        Self { components }
    }
    /// Dot product in double storage precision.
    pub fn dot(self, rhs: Self) -> f64 {
        supported::<N>();
        let mut sum = mul64(self.components[0], rhs.components[0]);
        for i in 1..N {
            sum = add64(sum, mul64(self.components[i], rhs.components[i]));
        }
        sum
    }
    /// Squared length in double storage precision, independent of EKF precision.
    pub fn length_squared(self) -> f64 {
        self.dot(self)
    }
    /// Length with the upstream `HAL_WITH_EKF_DOUBLE` intermediate precision.
    /// False narrows components before squaring, then widens the float result.
    pub fn length<const HAL_WITH_EKF_DOUBLE: bool>(self) -> f64 {
        norm64::<N, HAL_WITH_EKF_DOUBLE>(self.components)
    }
    /// Divide components in double precision by the selected-precision length.
    pub fn normalize<const HAL_WITH_EKF_DOUBLE: bool>(&mut self) {
        *self /= self.length::<HAL_WITH_EKF_DOUBLE>();
    }
    /// Return a component-wise double division by the selected-precision length.
    pub fn normalized<const HAL_WITH_EKF_DOUBLE: bool>(self) -> Self {
        self / self.length::<HAL_WITH_EKF_DOUBLE>()
    }
    /// Convert components to float with the reference NaN payload mapping.
    pub fn tofloat(self) -> Vector<N> {
        supported::<N>();
        Vector::new(self.components.map(narrow))
    }
    /// Preserve component bits, including signaling NaNs.
    pub const fn todouble(self) -> Self {
        self
    }
}
impl<const N: usize> Default for Vector64<N> {
    fn default() -> Self {
        Self::new([0.0; N])
    }
}
impl<const N: usize> Div<f64> for Vector64<N> {
    type Output = Self;
    fn div(self, rhs: f64) -> Self {
        supported::<N>();
        Self::new(self.components.map(|x| div64(x, rhs)))
    }
}
impl<const N: usize> DivAssign<f64> for Vector64<N> {
    fn div_assign(&mut self, rhs: f64) {
        *self = *self / rhs;
    }
}
impl<const N: usize> Mul for Vector64<N> {
    type Output = f64;
    fn mul(self, rhs: Self) -> f64 {
        self.dot(rhs)
    }
}
impl Vector64<2> {
    /// Limit length using explicit upstream EKF precision and FLT_EPSILON.
    pub fn limit_length<const HAL_WITH_EKF_DOUBLE: bool>(&mut self, max_length: f64) -> bool {
        let len = self.length::<HAL_WITH_EKF_DOUBLE>();
        if len > max_length && len >= f32::EPSILON as f64 {
            let scale = div64(max_length, len);
            for x in &mut self.components {
                *x = mul64(*x, scale);
            }
            true
        } else {
            false
        }
    }
}
impl Vector64<3> {
    /// Limit xy length while preserving every z bit, including signaling NaNs.
    pub fn limit_length_xy<const HAL_WITH_EKF_DOUBLE: bool>(&mut self, max_length: f64) -> bool {
        let len = norm64::<2, HAL_WITH_EKF_DOUBLE>([self.components[0], self.components[1]]);
        if len > max_length && len >= f32::EPSILON as f64 {
            let scale = div64(max_length, len);
            for x in &mut self.components[..2] {
                *x = mul64(*x, scale);
            }
            true
        } else {
            false
        }
    }
    /// Squared component distance in double storage precision.
    pub fn distance_squared(self, rhs: Self) -> f64 {
        let d = core::array::from_fn(|i| sub64(self.components[i], rhs.components[i]));
        Vector64::<3>::new(d).length_squared()
    }
}
