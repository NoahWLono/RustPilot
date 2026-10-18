// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot libraries/Filter/LowPassFilter.{h,cpp} and
// libraries/AP_Math/AP_Math.cpp, revision afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Original algorithms and GPL notices: the ArduPilot contributors.
//! Filters translated from pinned ArduPilot source.
//!
//! Vector, biquad, buffered, ordered and fixed-coefficient filters are exposed
//! through their modules. The tangent backend retains its separate MIT notice.
//! Numerical compatibility is specific to each module's recorded reference
//! build; an embedded library build does not qualify that target's behavior.
//!
//! The single-pole filters start with output zero and a pending first sample. The first
//! apply after construction or [`DigitalLpf::reset`] accepts the sample exactly;
//! reset preserves the previous output until that apply. This defines state
//! which C++ automatic-storage constructors may leave uninitialized.
//!
//! Floating-point arithmetic retains the upstream operation order and IEEE
//! propagation, including NaN/infinity and overflow. Inputs are not validated or
//! clamped; a poisoned output generally requires reset to recover. Negative
//! dt/cutoff beyond the upstream epsilon threshold yield alpha one, but this
//! crate does not reproduce the upstream INTERNAL_ERROR reporting side effect.
#![no_std]
#![forbid(unsafe_code)]

pub mod biquad;
pub mod buffered;
pub mod butterworth;
pub mod ordered;
pub mod tangent;
pub mod vector;

pub use buffered::Filter;

/// Compute alpha with the upstream epsilon thresholds and branch order.
///
/// dt or cutoff <= -EPSILON returns 1; |cutoff| < EPSILON returns 1;
/// |dt| < EPSILON returns 0. Other values use dt / (dt + RC). M_2PI
/// and the reciprocal calculation use f64 before RC is rounded to f32, as in
/// upstream C++. Nonfinite arguments follow these same branches and IEEE math.
pub fn calc_lowpass_alpha_dt(dt: f32, cutoff: f32) -> f32 {
    if dt <= -f32::EPSILON || cutoff <= -f32::EPSILON || cutoff.abs() < f32::EPSILON {
        return 1.0;
    }
    if dt.abs() < f32::EPSILON {
        return 0.0;
    }
    let rc = (1.0_f64 / (core::f64::consts::TAU * f64::from(cutoff))) as f32;
    dt / (dt + rc)
}

/// Scalar implementation of upstream DigitalLPF, with explicit initialization.
#[derive(Debug, Default)]
pub struct DigitalLpf {
    output: f32,
    initialized: bool,
}

impl DigitalLpf {
    /// Construct with output zero and pending first sample.
    pub const fn new() -> Self {
        Self {
            output: 0.0,
            initialized: false,
        }
    }
    /// Current output, including the previous value while reset is pending.
    pub const fn get(&self) -> f32 {
        self.output
    }
    /// Accept the next sample directly; leave current output unchanged.
    pub fn reset(&mut self) {
        self.initialized = false;
    }
    /// Set output and mark the filter initialized, including nonfinite values.
    pub fn reset_to(&mut self, value: f32) {
        self.output = value;
        self.initialized = true;
    }
    /// Apply an unbounded alpha. First sample is accepted exactly.
    pub fn apply(&mut self, sample: f32, alpha: f32) -> f32 {
        self.output += (sample - self.output) * alpha;
        if !self.initialized {
            self.initialized = true;
            self.output = sample;
        }
        self.output
    }
}

/// Scalar low-pass filter with dt supplied on each apply.
#[derive(Debug, Default)]
pub struct LowPassFilter {
    filter: DigitalLpf,
    cutoff: f32,
}

impl LowPassFilter {
    /// Construct with a configured cutoff and pending first sample.
    pub const fn new(cutoff: f32) -> Self {
        Self {
            filter: DigitalLpf::new(),
            cutoff,
        }
    }
    /// Current output.
    pub const fn get(&self) -> f32 {
        self.filter.get()
    }
    /// Accept the next sample directly, preserving output until then.
    pub fn reset(&mut self) {
        self.filter.reset();
    }
    /// Set output and mark the filter initialized.
    pub fn reset_to(&mut self, value: f32) {
        self.filter.reset_to(value);
    }
    /// Set cutoff without resetting output or initialization.
    pub fn set_cutoff_frequency(&mut self, cutoff: f32) {
        self.cutoff = cutoff;
    }
    /// Configured cutoff, including invalid/nonfinite values supplied by caller.
    pub const fn cutoff_frequency(&self) -> f32 {
        self.cutoff
    }
    /// Apply a sample with the provided time step.
    pub fn apply(&mut self, sample: f32, dt: f32) -> f32 {
        self.filter
            .apply(sample, calc_lowpass_alpha_dt(dt, self.cutoff))
    }
}

/// Scalar low-pass filter with alpha computed when frequencies are configured.
#[derive(Debug)]
pub struct LowPassFilterConstDt {
    filter: DigitalLpf,
    cutoff: f32,
    alpha: f32,
}

impl Default for LowPassFilterConstDt {
    fn default() -> Self {
        Self::new(0.0, 0.0)
    }
}

impl LowPassFilterConstDt {
    /// Configure sample frequency and cutoff, with pending first sample.
    pub fn new(sample_frequency: f32, cutoff: f32) -> Self {
        let mut result = Self {
            filter: DigitalLpf::new(),
            cutoff,
            alpha: 1.0,
        };
        result.set_cutoff_frequency(sample_frequency, cutoff);
        result
    }
    /// Current output.
    pub const fn get(&self) -> f32 {
        self.filter.get()
    }
    /// Accept the next sample directly, preserving output until then.
    pub fn reset(&mut self) {
        self.filter.reset();
    }
    /// Set output and mark the filter initialized.
    pub fn reset_to(&mut self, value: f32) {
        self.filter.reset_to(value);
    }
    /// Recompute alpha without resetting output or initialization.
    ///
    /// Nonpositive sample frequency selects alpha one. For positive/nonfinite
    /// frequency the reciprocal is computed in f64 then rounded to f32, matching
    /// upstream's `1.0/sample_freq` expression.
    pub fn set_cutoff_frequency(&mut self, sample_frequency: f32, cutoff: f32) {
        self.cutoff = cutoff;
        self.alpha = if sample_frequency <= 0.0 {
            1.0
        } else {
            calc_lowpass_alpha_dt((1.0_f64 / f64::from(sample_frequency)) as f32, cutoff)
        };
    }
    /// Configured cutoff.
    pub const fn cutoff_frequency(&self) -> f32 {
        self.cutoff
    }
    /// Apply a sample with the configured alpha.
    pub fn apply(&mut self, sample: f32) -> f32 {
        self.filter.apply(sample, self.alpha)
    }
}
