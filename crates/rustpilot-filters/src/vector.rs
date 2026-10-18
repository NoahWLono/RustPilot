// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot libraries/Filter/LowPassFilter.{h,cpp} and
// libraries/AP_Math/vector{2,3}.{h,cpp}, revision
// afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Vector source credits: Copyright 2010 Michael Smith; Copyright (C)
// Andrew Tridgell 2012; derived from Bill Perone's 2003 vector classes.
//! Component vectors and first-order low-pass filters.
//!
//! This file implements construction, component addition, subtraction and
//! scalar multiplication, and the first-order vector filters. Magnitudes,
//! normalization, scalar division, dot products and precision conversion are
//! implemented in [`crate::vector_math`], which also defines double vectors.
//! Rotation and the remaining AP_Math vector operations are unfinished. The
//! Vector2f/Vector3f filter instantiations correspond to N=2 and N=3.
//!
//! Floating-point output bits follow the GCC 16.2.0 x86-64 component profile
//! described in `VECTOR_REPORT.md`: separate driver/implementation translation
//! units, `-O2 -ffp-contract=off`, no sanitizers or LTO, round to nearest and
//! gradual underflow. Explicit NaN selection preserves that profile across
//! Rust debug/release code generation. Other C++ build profiles and other
//! vector dimensions have not been qualified. Floating-point exception flags
//! are not part of this output-bit contract.

use crate::calc_lowpass_alpha_dt;
use core::ops::{Add, AddAssign, Mul, Sub};

// The GCC 16.2.0 x86-64 reference selects the destination NaN and quiets its
// payload. Rust deliberately leaves NaN payload selection unspecified; these
// helpers pin it for the documented component reference build.
fn arithmetic_result(lhs: f32, rhs: f32, finite: impl FnOnce() -> f32) -> f32 {
    if lhs.is_nan() {
        return f32::from_bits(lhs.to_bits() | 0x0040_0000);
    }
    if rhs.is_nan() {
        return f32::from_bits(rhs.to_bits() | 0x0040_0000);
    }
    let result = finite();
    if result.is_nan() {
        f32::from_bits(0xffc0_0000)
    } else {
        result
    }
}
pub(crate) fn add_first(lhs: f32, rhs: f32) -> f32 {
    arithmetic_result(lhs, rhs, || lhs + rhs)
}
pub(crate) fn sub_first(lhs: f32, rhs: f32) -> f32 {
    arithmetic_result(lhs, rhs, || lhs - rhs)
}
pub(crate) fn mul_first(lhs: f32, rhs: f32) -> f32 {
    arithmetic_result(lhs, rhs, || lhs * rhs)
}
fn vector_alpha(dt: f32, cutoff: f32) -> f32 {
    let result = calc_lowpass_alpha_dt(dt, cutoff);
    if !result.is_nan() {
        return result;
    }
    if dt.is_nan() {
        f32::from_bits(dt.to_bits() | 0x0040_0000)
    } else if cutoff.is_nan() {
        f32::from_bits(cutoff.to_bits() | 0x0040_0000)
    } else {
        f32::from_bits(0xffc0_0000)
    }
}

/// A fixed-size component vector. Array storage has no C++ ABI guarantee.
///
/// Arithmetic uses the specific reference NaN choices documented at
/// [`crate::vector`]. Only N=2 and N=3 have upstream comparison evidence.
/// The derived `PartialEq` compares components exactly; it does not implement
/// ArduPilot's epsilon-based vector equality operators.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector<const N: usize> {
    /// Components in x, y, then z order for the upstream vector aliases.
    pub components: [f32; N],
}

/// Two-component upstream float-vector representation.
pub type Vector2f = Vector<2>;
/// Three-component upstream float-vector representation.
pub type Vector3f = Vector<3>;

impl<const N: usize> Vector<N> {
    /// Construct a vector without modifying any component's bits.
    pub const fn new(components: [f32; N]) -> Self {
        Self { components }
    }
}
impl<const N: usize> Default for Vector<N> {
    fn default() -> Self {
        Self::new([0.0; N])
    }
}
impl<const N: usize> Add for Vector<N> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(core::array::from_fn(|i| {
            add_first(self.components[i], rhs.components[i])
        }))
    }
}
impl<const N: usize> AddAssign for Vector<N> {
    fn add_assign(&mut self, rhs: Self) {
        for i in 0..N {
            self.components[i] = add_first(self.components[i], rhs.components[i]);
        }
    }
}
impl<const N: usize> Sub for Vector<N> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(core::array::from_fn(|i| {
            sub_first(self.components[i], rhs.components[i])
        }))
    }
}
impl<const N: usize> Mul<f32> for Vector<N> {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(core::array::from_fn(|i| {
            // Standalone Vector3 z multiplication selects the scalar NaN.
            if N == 3 && i == 2 {
                mul_first(rhs, self.components[i])
            } else {
                mul_first(self.components[i], rhs)
            }
        }))
    }
}

/// Component-wise upstream DigitalLPF with explicit initialization.
///
/// Output-bit semantics follow the GCC 16.2.0 component profile at
/// [`crate::vector`]; other build profiles require qualification.
#[derive(Debug)]
pub struct DigitalVectorLpf<const N: usize> {
    output: Vector<N>,
    initialized: bool,
}
impl<const N: usize> Default for DigitalVectorLpf<N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const N: usize> DigitalVectorLpf<N> {
    /// Start at positive zero with the first sample pending.
    pub const fn new() -> Self {
        Self {
            output: Vector::new([0.0; N]),
            initialized: false,
        }
    }
    /// Current output, retained while reset is pending.
    pub const fn get(&self) -> Vector<N> {
        self.output
    }
    /// Accept the next sample directly without changing current output.
    pub fn reset(&mut self) {
        self.initialized = false;
    }
    /// Copy component bits and mark the filter initialized.
    pub fn reset_to(&mut self, value: Vector<N>) {
        self.output = value;
        self.initialized = true;
    }
    /// Apply unbounded alpha, retaining the source operation order.
    pub fn apply(&mut self, sample: Vector<N>, alpha: f32) -> Vector<N> {
        self.apply_profile(sample, alpha, false)
    }
    fn apply_profile(&mut self, sample: Vector<N>, alpha: f32, configured: bool) -> Vector<N> {
        self.output = Vector::new(core::array::from_fn(|i| {
            let previous = self.output.components[i];
            let difference = sub_first(sample.components[i], previous);
            let scaled = mul_first(difference, alpha);
            // In the documented configured Vector3 builds GCC places the z
            // history in the add destination, unlike the other components.
            if configured && N == 3 && i == 2 {
                add_first(previous, scaled)
            } else {
                add_first(scaled, previous)
            }
        }));
        if !self.initialized {
            self.initialized = true;
            self.output = sample;
        }
        self.output
    }
}

/// Component-wise first-order filter with a time step supplied per sample.
///
/// Output-bit semantics follow the GCC 16.2.0 component profile at
/// [`crate::vector`]; other build profiles require qualification.
#[derive(Debug)]
pub struct VectorLowPassFilter<const N: usize> {
    filter: DigitalVectorLpf<N>,
    cutoff: f32,
}
impl<const N: usize> Default for VectorLowPassFilter<N> {
    fn default() -> Self {
        Self::new(0.0)
    }
}
impl<const N: usize> VectorLowPassFilter<N> {
    /// Configure cutoff with an initially pending sample.
    pub const fn new(cutoff: f32) -> Self {
        Self {
            filter: DigitalVectorLpf::new(),
            cutoff,
        }
    }
    /// Current output.
    pub const fn get(&self) -> Vector<N> {
        self.filter.get()
    }
    /// Accept the next sample directly, preserving the stored output meanwhile.
    pub fn reset(&mut self) {
        self.filter.reset();
    }
    /// Set output and mark the filter initialized.
    pub fn reset_to(&mut self, value: Vector<N>) {
        self.filter.reset_to(value);
    }
    /// Update cutoff without resetting output or initialization.
    pub fn set_cutoff_frequency(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }
    /// Configured cutoff, including nonfinite values.
    pub const fn cutoff_frequency(&self) -> f32 {
        self.cutoff
    }
    /// Calculate alpha using the source epsilon thresholds and expression order.
    pub fn apply(&mut self, sample: Vector<N>, dt: f32) -> Vector<N> {
        self.filter
            .apply_profile(sample, vector_alpha(dt, self.cutoff), true)
    }
}

/// Component-wise first-order filter with cached alpha.
///
/// Output-bit semantics follow the GCC 16.2.0 component profile at
/// [`crate::vector`]; other build profiles require qualification.
#[derive(Debug)]
pub struct VectorLowPassFilterConstDt<const N: usize> {
    filter: DigitalVectorLpf<N>,
    cutoff: f32,
    alpha: f32,
}
impl<const N: usize> Default for VectorLowPassFilterConstDt<N> {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}
impl<const N: usize> VectorLowPassFilterConstDt<N> {
    /// Configure sample frequency and cutoff with an initially pending sample.
    pub fn new(sample_frequency: f32, cutoff: f32) -> Self {
        let mut result = Self {
            filter: DigitalVectorLpf::new(),
            cutoff,
            alpha: 1.0,
        };
        result.set_cutoff_frequency(sample_frequency, cutoff);
        result
    }
    /// Current output.
    pub const fn get(&self) -> Vector<N> {
        self.filter.get()
    }
    /// Accept the next sample directly, preserving the stored output meanwhile.
    pub fn reset(&mut self) {
        self.filter.reset();
    }
    /// Set output and mark the filter initialized.
    pub fn reset_to(&mut self, value: Vector<N>) {
        self.filter.reset_to(value);
    }
    /// Cache a new alpha without resetting output or initialization.
    pub fn set_cutoff_frequency(&mut self, sample_frequency: f32, cutoff: f32) {
        self.cutoff = cutoff;
        self.alpha = if sample_frequency <= 0.0 {
            1.0
        } else {
            vector_alpha((1.0_f64 / f64::from(sample_frequency)) as f32, cutoff)
        };
    }
    /// Configured cutoff, including nonfinite values.
    pub const fn cutoff_frequency(&self) -> f32 {
        self.cutoff
    }
    /// Apply cached alpha separately to every component.
    pub fn apply(&mut self, sample: Vector<N>) -> Vector<N> {
        self.filter.apply_profile(sample, self.alpha, true)
    }
}
