// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot AP_Math/vector{2,3}.{h,cpp}, AP_Math.cpp
// and ftype.h, revision afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Copyright 2010 Michael Smith; Copyright (C) Andrew Tridgell 2012;
// derived from Bill Perone's 2003 vector classes.
//! Vector predicates, upstream epsilon equality and checked component access.
//!
//! [`Vector::equals`] and [`Vector64::equals`] map upstream epsilon equality
//! explicitly. The existing Rust `PartialEq` retains exact component equality.
//! Epsilon equality is not transitive, and infinities do not equal themselves.
//! Only dimensions two and three have upstream comparison evidence, recorded
//! in `VECTOR_PREDICATES_REPORT.md`; exception flags are outside that contract.
//!
//! Indexing returns references into the component array and panics when the
//! index is outside `0..N`. This corrects upstream pointer arithmetic from its
//! scalar `x` member across separate members. Host observations for valid
//! component indices do not establish portable defined C++ behavior. Rust's
//! array storage has no C++ ABI guarantee.

use crate::{vector::Vector, vector_math::Vector64};
use core::ops::{Index, IndexMut};

impl<const N: usize> Vector<N> {
    /// Whether any component is a NaN, including signaling NaNs.
    pub fn is_nan(&self) -> bool {
        self.components.iter().any(|x| x.is_nan())
    }

    /// Whether any component is positive or negative infinity.
    pub fn is_inf(&self) -> bool {
        self.components.iter().any(|x| x.is_infinite())
    }

    /// Whether every component's absolute value is strictly below `FLT_EPSILON`.
    pub fn is_zero(&self) -> bool {
        self.components.iter().all(|x| x.abs() < f32::EPSILON)
    }

    /// Replace every component with positive zero.
    pub fn zero(&mut self) {
        self.components = [0.0; N];
    }

    /// Upstream epsilon equality: each absolute difference is below `FLT_EPSILON`.
    ///
    /// This differs from Rust `==`. NaNs and matching infinities compare false.
    pub fn equals(&self, rhs: Self) -> bool {
        self.components
            .iter()
            .zip(rhs.components)
            .all(|(&a, b)| (a - b).abs() < f32::EPSILON)
    }

    /// Upstream epsilon inequality, the logical complement of [`Self::equals`].
    pub fn not_equals(&self, rhs: Self) -> bool {
        !self.equals(rhs)
    }
}

impl<const N: usize> Vector64<N> {
    /// Whether any component is a NaN, including signaling NaNs.
    pub fn is_nan(&self) -> bool {
        self.components.iter().any(|x| x.is_nan())
    }

    /// Whether any component is positive or negative infinity.
    pub fn is_inf(&self) -> bool {
        self.components.iter().any(|x| x.is_infinite())
    }

    /// Upstream zero predicate, using `FLT_EPSILON` in both configurations.
    ///
    /// When double functions are disabled, narrow each component to float
    /// before the absolute-value comparison, as the upstream helper does.
    #[allow(non_snake_case)]
    pub fn is_zero<const AP_MATH_ALLOW_DOUBLE_FUNCTIONS: bool>(&self) -> bool {
        self.components.iter().all(|&x| {
            if AP_MATH_ALLOW_DOUBLE_FUNCTIONS {
                x.abs() < f32::EPSILON as f64
            } else {
                (x as f32).abs() < f32::EPSILON
            }
        })
    }

    /// Replace every component with positive zero.
    pub fn zero(&mut self) {
        self.components = [0.0; N];
    }

    /// Upstream epsilon equality with explicit double-function configuration.
    ///
    /// Enabled uses double differences and `DBL_EPSILON`. Disabled still
    /// subtracts in double, then narrows the difference to float and compares
    /// against `FLT_EPSILON`. Narrowing the operands first would be incorrect.
    /// NaNs and matching infinities compare false. Rust `==` remains exact.
    #[allow(non_snake_case)]
    pub fn equals<const AP_MATH_ALLOW_DOUBLE_FUNCTIONS: bool>(&self, rhs: Self) -> bool {
        self.components.iter().zip(rhs.components).all(|(&a, b)| {
            let difference = a - b;
            if AP_MATH_ALLOW_DOUBLE_FUNCTIONS {
                difference.abs() < f64::EPSILON
            } else {
                (difference as f32).abs() < f32::EPSILON
            }
        })
    }

    /// Upstream epsilon inequality, the logical complement of [`Self::equals`].
    #[allow(non_snake_case)]
    pub fn not_equals<const AP_MATH_ALLOW_DOUBLE_FUNCTIONS: bool>(&self, rhs: Self) -> bool {
        !self.equals::<AP_MATH_ALLOW_DOUBLE_FUNCTIONS>(rhs)
    }
}

impl Vector<3> {
    /// Copy xy and z into a three-component vector, preserving every input bit.
    pub const fn from_xy(xy: Vector<2>, z: f32) -> Self {
        Self::new([xy.components[0], xy.components[1], z])
    }
}

impl Vector64<3> {
    /// Copy xy and z into a three-component vector, preserving every input bit.
    pub const fn from_xy(xy: Vector64<2>, z: f64) -> Self {
        Self::new([xy.components[0], xy.components[1], z])
    }
}

impl<const N: usize> Index<usize> for Vector<N> {
    type Output = f32;

    /// Borrow a component; panics when `index >= N`.
    fn index(&self, index: usize) -> &Self::Output {
        &self.components[index]
    }
}

impl<const N: usize> IndexMut<usize> for Vector<N> {
    /// Mutably borrow a component; panics when `index >= N`.
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.components[index]
    }
}

impl<const N: usize> Index<usize> for Vector64<N> {
    type Output = f64;

    /// Borrow a component; panics when `index >= N`.
    fn index(&self, index: usize) -> &Self::Output {
        &self.components[index]
    }
}

impl<const N: usize> IndexMut<usize> for Vector64<N> {
    /// Mutably borrow a component; panics when `index >= N`.
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.components[index]
    }
}
