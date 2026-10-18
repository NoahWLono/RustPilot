// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot libraries/Filter/Butter.h at revision
// afeb9c4941543d97f3e04fecf257b8f598f49a6a. Credit: ArduPilot contributors.
//! Second-order Butterworth recurrences with the upstream fixed coefficients.
//!
//! Names retain the upstream sample-rate/cutoff aliases. They select a fixed
//! coefficient set; they do not measure or enforce the actual sample rate.
//! Construction and reset explicitly zero both delayed states, matching a
//! zero-initialized C++ object. Upstream automatic-storage objects could instead
//! have uninitialized history, and the upstream type has no reset method.
//!
//! Arithmetic uses f32 and the upstream expression order, with no fused
//! multiply-add, input validation or nonfinite clamping. A nonfinite input or
//! overflowing history can contaminate later output until reset.

use core::marker::PhantomData;

/// Coefficients of the upstream two-history recurrence.
///
/// Custom sets are permitted, as with the C++ template. They are not checked
/// for stability, cutoff, finite values or a nonzero gain.
pub trait ButterCoefficients {
    const A1: f32;
    const A2: f32;
    const GAIN: f32;
}

/// Second-order recurrence corresponding to upstream `Butter2<Coefficients>`.
#[derive(Debug, Clone)]
pub struct Butter2<C: ButterCoefficients> {
    history: [f32; 2],
    coefficients: PhantomData<C>,
}

impl<C: ButterCoefficients> Default for Butter2<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: ButterCoefficients> Butter2<C> {
    /// Start with two zero history entries.
    pub const fn new() -> Self {
        Self {
            history: [0.0; 2],
            coefficients: PhantomData,
        }
    }

    /// Restore the same history as a newly constructed filter.
    pub fn reset(&mut self) {
        self.history = [0.0; 2];
    }

    /// Apply one input, preserving the upstream f32 recurrence order.
    pub fn filter(&mut self, input: f32) -> f32 {
        let new_history = input + C::A1 * self.history[1] + C::A2 * self.history[0];
        let result = (new_history + 2.0 * self.history[1] + self.history[0]) / C::GAIN;
        self.history[0] = self.history[1];
        self.history[1] = new_history;
        result
    }
}

// Keep original coefficient literals, identifiers and alias spellings so they
// can be checked directly against the pinned source. Literals round to f32.
macro_rules! coefficients {
    ($name:ident, $a1:expr, $a2:expr, $gain:expr; $($alias:ident),+ $(,)?) => {
        #[doc = concat!("Fixed coefficients from upstream `", stringify!($name), "`.")]
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy)]
        pub struct $name;
        #[allow(clippy::excessive_precision)]
        impl ButterCoefficients for $name {
            const A1: f32 = $a1;
            const A2: f32 = $a2;
            const GAIN: f32 = $gain;
        }
        $(
            #[doc = concat!("Upstream `", stringify!($alias), "` alias using `", stringify!($name), "`.")]
            #[allow(non_camel_case_types)]
            pub type $alias = Butter2<$name>;
        )+
    };
}

coefficients!(butter100_025_coeffs, 1.9777864838_f32, -0.9780305085_f32, 1.639178228e+04_f32;
    butter100hz0_25, butter50hz0_125, butter10hz0_025);
coefficients!(butter100_05_coeffs, 1.9555782403_f32, -0.9565436765_f32, 4.143204922e+03_f32;
    butter100hz0_5, butter50hz0_25, butter10hz0_05);
coefficients!(butter100_1_coeffs, 1.9111970674_f32, -0.9149758348_f32, 1.058546241e+03_f32;
    butter100hz1_0, butter50hz0_5, butter10hz0_1);
coefficients!(butter100_1_5_coeffs, 1.8668922797_f32, -0.8752145483_f32, 4.806381793e+02_f32;
    butter100hz1_5, butter50hz0_75, butter10hz0_15);
coefficients!(butter100_2_coeffs, 1.8226949252_f32, -0.8371816513_f32, 2.761148367e+02_f32;
    butter100hz2_0, butter50hz1_0, butter10hz0_2);
coefficients!(butter100_3_coeffs, 1.7347257688_f32, -0.7660066009_f32, 1.278738361e+02_f32;
    butter100hz3_0, butter50hz1_5, butter10hz0_3);
coefficients!(butter100_4_coeffs, 1.6474599811_f32, -0.7008967812_f32, 7.485478157e+01_f32;
    butter100hz4_0, butter50hz2_0, butter10hz0_4);
coefficients!(butter100_8_coeffs, 1.3072850288_f32, -0.4918122372_f32, 2.167702007e+01_f32;
    butter100hz8_0, butter50hz4_0, butter10hz0_8);
coefficients!(butter50_8_coeffs, 0.6710290908_f32, -0.2523246263_f32, 6.881181354e+00_f32;
    butter50hz8_0, butter10hz1_6);
