// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot libraries/Filter/LowPassFilter2p.{h,cpp},
// revision afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Original authors: Leonard Hall and Daniel Frenzel; ArduPilot contributors.
//! Two-pole low-pass filters, including reference coefficient generation.
//!
//! The scalar and vector floating variants return `Ok` for every input,
//! preserving IEEE arithmetic. Integer variants reject nonfinite/out-of-range
//! conversions before changing filter state. Constructors initialize the state
//! that upstream C++ automatic-storage constructors can leave indeterminate.
//!
//! Exact NaN sign/payload rules follow the recorded GCC 16.2.0 x86-64 SSE
//! reference, including differences between direct biquad and inlined low-pass
//! calls. See `BIQUAD_REPORT.md` for flags, translation-unit boundaries and
//! qualified tests. Other target/compiler builds need separate qualification.

use crate::vector::{add_first, mul_first, sub_first, Vector};

fn is_positive(value: f32) -> bool {
    value >= f32::EPSILON
}

/// Upstream's seven public coefficient/configuration fields.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BiquadParams {
    pub cutoff_freq: f32,
    pub sample_freq: f32,
    pub a1: f32,
    pub a2: f32,
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
}

/// A checked correction for an upstream undefined floating-to-integer cast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiquadError {
    /// A computed delay or output is nonfinite or outside the integer range.
    IntegerConversion,
}

mod sealed {
    pub trait Sealed {}
}

/// Sample types supplied by upstream (and component-wise vector dimensions).
///
/// This trait is sealed to keep integer conversion and vector rounding rules
/// part of the implementation contract. Floating types never return an error.
pub trait BiquadSample: sealed::Sealed + Copy + Default {
    #[doc(hidden)]
    fn reset_delay(
        value: Self,
        params: &BiquadParams,
        initializing: bool,
    ) -> Result<Self, BiquadError>;
    #[doc(hidden)]
    fn step(
        sample: Self,
        delay1: Self,
        delay2: Self,
        params: &BiquadParams,
        wrapped: bool,
    ) -> Result<(Self, Self), BiquadError>;
}

impl sealed::Sealed for f32 {}
impl BiquadSample for f32 {
    fn reset_delay(
        value: Self,
        params: &BiquadParams,
        _initializing: bool,
    ) -> Result<Self, BiquadError> {
        let denominator = add_first(add_first(1.0_f32, params.a1), params.a2);
        if denominator.is_nan() {
            return Ok(denominator);
        }
        if value.is_nan() {
            return Ok(f32::from_bits(value.to_bits() | 0x00400000));
        }
        let result = (f64::from(value) * (1.0_f64 / f64::from(denominator))) as f32;
        Ok(if result.is_nan() {
            f32::from_bits(0xffc00000)
        } else {
            result
        })
    }
    fn step(
        sample: Self,
        delay1: Self,
        delay2: Self,
        p: &BiquadParams,
        _wrapped: bool,
    ) -> Result<(Self, Self), BiquadError> {
        let delay0 = sub_first(
            sub_first(sample, mul_first(p.a1, delay1)),
            mul_first(p.a2, delay2),
        );
        let output = add_first(
            add_first(mul_first(p.b0, delay0), mul_first(p.b1, delay1)),
            mul_first(delay2, p.b2),
        );
        Ok((delay0, output))
    }
}

impl<const N: usize> sealed::Sealed for Vector<N> {}
impl<const N: usize> BiquadSample for Vector<N> {
    fn reset_delay(
        value: Self,
        params: &BiquadParams,
        initializing: bool,
    ) -> Result<Self, BiquadError> {
        let first = add_first(params.a1, 1.0_f32);
        let denominator = if N == 2 && initializing {
            add_first(params.a2, first)
        } else {
            add_first(first, params.a2)
        };
        // Vector2/Vector3 operator*(float) converts the double reciprocal before
        // multiplying components; the scalar version multiplies in double.
        let scale = (1.0_f64 / f64::from(denominator)) as f32;
        Ok(Vector {
            components: core::array::from_fn(|i| {
                let sample_first = (initializing && i < 2) || (!initializing && N == 3 && i == 2);
                if sample_first {
                    mul_first(value.components[i], scale)
                } else {
                    mul_first(scale, value.components[i])
                }
            }),
        })
    }
    fn step(
        sample: Self,
        delay1: Self,
        delay2: Self,
        p: &BiquadParams,
        wrapped: bool,
    ) -> Result<(Self, Self), BiquadError> {
        let delay0 = Vector {
            components: core::array::from_fn(|i| {
                sub_first(
                    sub_first(sample.components[i], mul_first(p.a1, delay1.components[i])),
                    mul_first(p.a2, delay2.components[i]),
                )
            }),
        };
        let output = Vector {
            components: core::array::from_fn(|i| {
                let current = mul_first(p.b0, delay0.components[i]);
                let previous = mul_first(p.b1, delay1.components[i]);
                // Inlining changes NaN operand selection in the recorded build.
                // Low-pass: previous for Vector2/Vector3.z, current for Vector3.x/y.
                // Direct: current for Vector2/Vector3.z, previous for Vector3.x/y.
                let previous_first = if wrapped {
                    N == 2 || (N == 3 && i == 2)
                } else {
                    N == 3 && i < 2
                };
                let sum = if previous_first {
                    add_first(previous, current)
                } else {
                    add_first(current, previous)
                };
                add_first(sum, mul_first(p.b2, delay2.components[i]))
            }),
        };
        Ok((delay0, output))
    }
}

macro_rules! integer_sample {
    ($ty:ty, $bound:expr, $lower:expr) => {
        impl sealed::Sealed for $ty {}
        impl BiquadSample for $ty {
            fn reset_delay(
                value: Self,
                params: &BiquadParams,
                _initializing: bool,
            ) -> Result<Self, BiquadError> {
                let denominator = 1.0_f32 + params.a1 + params.a2;
                let value = value as f64 * (1.0_f64 / f64::from(denominator));
                if $lower(value) && value < $bound {
                    Ok(value as Self)
                } else {
                    Err(BiquadError::IntegerConversion)
                }
            }
            fn step(
                sample: Self,
                delay1: Self,
                delay2: Self,
                p: &BiquadParams,
                _wrapped: bool,
            ) -> Result<(Self, Self), BiquadError> {
                let delay0 = sample as f32 - delay1 as f32 * p.a1 - delay2 as f32 * p.a2;
                if !(f64::from(delay0) >= -$bound && f64::from(delay0) < $bound) {
                    return Err(BiquadError::IntegerConversion);
                }
                let delay0 = delay0 as Self;
                let output = delay0 as f32 * p.b0 + delay1 as f32 * p.b1 + delay2 as f32 * p.b2;
                if !(f64::from(output) >= -$bound && f64::from(output) < $bound) {
                    return Err(BiquadError::IntegerConversion);
                }
                Ok((delay0, output as Self))
            }
        }
    };
}
integer_sample!(i32, 2_147_483_648.0_f64, |value: f64| value
    > -2_147_483_649.0_f64);
integer_sample!(i64, 9_223_372_036_854_775_808.0_f64, |value: f64| value
    >= -9_223_372_036_854_775_808.0_f64);

/// Direct-form biquad with separate, caller-owned parameters.
#[derive(Debug)]
pub struct DigitalBiquadFilter<T: BiquadSample> {
    delay1: T,
    delay2: T,
    initialized: bool,
}

impl<T: BiquadSample> Default for DigitalBiquadFilter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: BiquadSample> DigitalBiquadFilter<T> {
    /// Zero delays and a pending reset to the first enabled sample.
    pub fn new() -> Self {
        Self {
            delay1: T::default(),
            delay2: T::default(),
            initialized: false,
        }
    }

    /// Configure frequencies and coefficients with the upstream expression order.
    ///
    /// Disabled cutoff updates both frequencies but preserves all five old
    /// coefficients. The clamp compares in double precision, including its NaN
    /// branch ordering. The cosine is evaluated only at upstream's fixed
    /// `float(PI/4)` argument: its reference result is `0x3f3504f3`. This constant
    /// is not a general-purpose cosine implementation.
    pub fn compute_params(sample_freq: f32, cutoff_freq: f32, ret: &mut BiquadParams) {
        let upper = f64::from(sample_freq) * 0.4_f64;
        ret.cutoff_freq = if f64::from(cutoff_freq) < upper {
            cutoff_freq
        } else {
            upper as f32
        };
        ret.sample_freq = sample_freq;
        if !is_positive(ret.cutoff_freq) {
            return;
        }
        let fr = ret.sample_freq / ret.cutoff_freq;
        let ohm = crate::tangent::tanf((core::f64::consts::PI / f64::from(fr)) as f32);
        let cosine = f32::from_bits(0x3f3504f3);
        let c = 1.0_f32 + 2.0_f32 * cosine * ohm + ohm * ohm;
        ret.b0 = ohm * ohm / c;
        ret.b1 = 2.0_f32 * ret.b0;
        ret.b2 = ret.b0;
        ret.a1 = 2.0_f32 * (ohm * ohm - 1.0_f32) / c;
        ret.a2 = (1.0_f32 - 2.0_f32 * cosine * ohm + ohm * ohm) / c;
    }

    /// Apply one sample. Disabled configuration returns its bits unchanged and
    /// does not change history or initialization. Integer failure is transactional.
    pub fn apply(&mut self, sample: T, params: &BiquadParams) -> Result<T, BiquadError> {
        self.apply_profile(sample, params, false)
    }

    // The recorded C++ build chooses different NaN operands after inlining
    // into LowPassFilter2p. Preserve both public entry points' observed bits.
    fn apply_profile(
        &mut self,
        sample: T,
        params: &BiquadParams,
        wrapped: bool,
    ) -> Result<T, BiquadError> {
        if !is_positive(params.cutoff_freq) || !is_positive(params.sample_freq) {
            return Ok(sample);
        }
        let (delay1, delay2) = if self.initialized {
            (self.delay1, self.delay2)
        } else {
            let delay = T::reset_delay(sample, params, true)?;
            (delay, delay)
        };
        let (delay0, output) = T::step(sample, delay1, delay2, params, wrapped)?;
        self.delay2 = delay1;
        self.delay1 = delay0;
        self.initialized = true;
        Ok(output)
    }

    /// Keep delay values but reset from the next enabled sample before recurrence.
    pub fn reset(&mut self) {
        self.initialized = false;
    }

    /// Reset both delays immediately. Scalar and vector rounding follow their
    /// respective upstream overloads. Integer failure leaves all state unchanged.
    pub fn reset_to(&mut self, value: T, params: &BiquadParams) -> Result<(), BiquadError> {
        let delay = T::reset_delay(value, params, false)?;
        self.delay1 = delay;
        self.delay2 = delay;
        self.initialized = true;
        Ok(())
    }
}

/// Two-pole filter that owns its configuration and recurrence state.
#[derive(Debug)]
pub struct LowPassFilter2p<T: BiquadSample> {
    params: BiquadParams,
    filter: DigitalBiquadFilter<T>,
}

impl<T: BiquadSample> Default for LowPassFilter2p<T> {
    fn default() -> Self {
        Self {
            params: BiquadParams::default(),
            filter: DigitalBiquadFilter::new(),
        }
    }
}
impl<T: BiquadSample> LowPassFilter2p<T> {
    /// Configure a new filter. Initially disabled coefficients are defined as zero.
    pub fn new(sample_freq: f32, cutoff_freq: f32) -> Self {
        let mut result = Self::default();
        result.set_cutoff_frequency(sample_freq, cutoff_freq);
        result
    }
    pub fn set_cutoff_frequency(&mut self, sample_freq: f32, cutoff_freq: f32) {
        DigitalBiquadFilter::<T>::compute_params(sample_freq, cutoff_freq, &mut self.params);
    }
    pub const fn get_cutoff_freq(&self) -> f32 {
        self.params.cutoff_freq
    }
    pub const fn get_sample_freq(&self) -> f32 {
        self.params.sample_freq
    }
    pub const fn parameters(&self) -> &BiquadParams {
        &self.params
    }
    pub fn apply(&mut self, sample: T) -> Result<T, BiquadError> {
        self.filter.apply_profile(sample, &self.params, true)
    }
    pub fn reset(&mut self) {
        self.filter.reset();
    }
    pub fn reset_to(&mut self, value: T) -> Result<(), BiquadError> {
        self.filter.reset_to(value, &self.params)
    }
}

pub type LowPassFilter2pInt = LowPassFilter2p<i32>;
/// C++ `long` on the recorded LP64 host. On ILP32 targets use the i32 variant.
pub type LowPassFilter2pLong = LowPassFilter2p<i64>;
pub type LowPassFilter2pFloat = LowPassFilter2p<f32>;
pub type LowPassFilter2pVector2f = LowPassFilter2p<Vector<2>>;
pub type LowPassFilter2pVector3f = LowPassFilter2p<Vector<3>>;
