// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot AP_Math/vector{2,3}.{h,cpp}, revision
// afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Copyright 2010 Michael Smith; Copyright (C) Andrew Tridgell 2012;
// derived from Bill Perone's 2003 vector classes.
//! Projection, axis reflection and perpendicular vectors.
//!
//! The output-bit contract is the GCC 16.2.0 x86-64 build described in
//! `VECTOR_GEOMETRY_REPORT.md`; only dimensions two and three are supported.
//! Projection deliberately multiplies before dividing and does not guard
//! zero directions. Floating-point exception flags are outside the contract.

use crate::{
    vector::{mul_first, Vector},
    vector_math::Vector64,
};

fn supported<const N: usize>() {
    assert!(N == 2 || N == 3, "only Vector2 and Vector3 are qualified");
}

fn mul64_first(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        return f64::from_bits(a.to_bits() | 0x0008_0000_0000_0000);
    }
    if b.is_nan() {
        return f64::from_bits(b.to_bits() | 0x0008_0000_0000_0000);
    }
    let result = a * b;
    if result.is_nan() {
        f64::from_bits(0xfff8_0000_0000_0000)
    } else {
        result
    }
}

impl<const N: usize> Vector<N> {
    /// Project onto `direction`, multiplying each component before division.
    pub fn project(&mut self, direction: Self) {
        *self = self.projected(direction);
    }
    /// Return the source projection, including its zero-direction behavior.
    pub fn projected(self, direction: Self) -> Self {
        supported::<N>();
        if N == 2 {
            // Vector2's out-of-line body inlines its operators: direction
            // wins each dot multiplication, then the dot wins each scaling.
            let d = direction.dot(self);
            return Self::new(direction.components.map(|x| mul_first(d, x)))
                / direction.dot(direction);
        }
        direction * self.dot(direction) / direction.dot(direction)
    }
    /// Reflect about the axis through `axis`: twice the projection minus self.
    pub fn reflect(&mut self, axis: Self) {
        let original = *self;
        if N == 2 {
            // The separately compiled reflect body instead places the
            // original vector first in its inlined dot multiplications.
            let d = self.dot(axis);
            *self = Self::new(axis.components.map(|x| mul_first(d, x))) / axis.dot(axis);
        } else {
            self.project(axis);
        }
        *self = *self * 2.0 - original;
    }
}
impl<const N: usize> Vector64<N> {
    /// Project onto `direction`, retaining double multiplication before division.
    pub fn project(&mut self, direction: Self) {
        *self = self.projected(direction);
    }
    /// Return the source projection, including its zero-direction behavior.
    pub fn projected(self, direction: Self) -> Self {
        supported::<N>();
        if N == 2 {
            let d = direction.dot(self);
            return Self::new(direction.components.map(|x| mul64_first(d, x)))
                / direction.dot(direction);
        }
        direction * self.dot(direction) / direction.dot(direction)
    }
    /// Reflect about the axis through `axis`: twice the projection minus self.
    pub fn reflect(&mut self, axis: Self) {
        let original = *self;
        if N == 2 {
            let d = self.dot(axis);
            *self = Self::new(axis.components.map(|x| mul64_first(d, x))) / axis.dot(axis);
        } else {
            self.project(axis);
        }
        *self = *self * 2.0 - original;
    }
}
impl Vector<2> {
    /// Select a perpendicular to `direction` using the source strict comparison.
    /// Ties and unordered comparisons select `[direction.y, -direction.x]`.
    pub fn perpendicular(position: Self, direction: Self) -> Self {
        let [x, y] = direction.components;
        let first = Self::new([f32::from_bits(y.to_bits() ^ (1 << 31)), x]);
        let second = Self::new([y, f32::from_bits(x.to_bits() ^ (1 << 31))]);
        if first.dot(position) > second.dot(position) {
            first
        } else {
            second
        }
    }
}
impl Vector64<2> {
    /// Select a perpendicular; ties and unordered comparisons choose `[y, -x]`.
    pub fn perpendicular(position: Self, direction: Self) -> Self {
        let [x, y] = direction.components;
        let first = Self::new([f64::from_bits(y.to_bits() ^ (1 << 63)), x]);
        let second = Self::new([y, f64::from_bits(x.to_bits() ^ (1 << 63))]);
        if first.dot(position) > second.dot(position) {
            first
        } else {
            second
        }
    }
}
impl Vector<3> {
    /// Return the component of `position` perpendicular to `direction`.
    /// A dot product with magnitude below FLT_EPSILON returns position unchanged.
    pub fn perpendicular(position: Self, direction: Self) -> Self {
        let d = position.dot(direction);
        if d.abs() < f32::EPSILON {
            return position;
        }
        position - direction * d / direction.length_squared()
    }
}
impl Vector64<3> {
    /// Return the perpendicular component, using the selected upstream is_zero.
    /// The threshold is FLT_EPSILON even with double functions enabled; disabling
    /// them narrows only the dot product used by the early-return comparison.
    pub fn perpendicular<const AP_MATH_ALLOW_DOUBLE_FUNCTIONS: bool>(
        position: Self,
        direction: Self,
    ) -> Self {
        let d = position.dot(direction);
        let zero = if AP_MATH_ALLOW_DOUBLE_FUNCTIONS {
            d.abs() < f64::from(f32::EPSILON)
        } else {
            (d as f32).abs() < f32::EPSILON
        };
        if zero {
            return position;
        }
        position - direction * d / direction.length_squared()
    }
}
