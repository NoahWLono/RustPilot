// SPDX-License-Identifier: GPL-3.0-or-later
// Translated from ArduPilot libraries/Filter/{ModeFilter,DerivativeFilter}.{h,cpp}
// and FilterWithBuffer.h at afeb9c4941543d97f3e04fecf257b8f598f49a6a.
// Original algorithms and GPL notices: the ArduPilot contributors.
//! Ordered-extreme and timestamped derivative filters.
//!
//! Construction initializes every field. Reset restores the complete initial
//! state, correcting the upstream retained output/eviction phase or timestamps/
//! derivative cache. Mode filtering alternates extreme eviction; it is not a
//! sliding-window median. See `ORDERED_REPORT.md` for the source mapping.

/// Values with a zero representation and upstream-style partial comparisons.
///
/// Custom implementations must provide the equivalent of upstream `T(0)`.
/// NaN comparisons remain unordered; sorting does not impose a total order.
pub trait ModeSample: Copy + PartialOrd {
    const ZERO: Self;
}

macro_rules! mode_samples {
    ($($t:ty),+ $(,)?) => { $(impl ModeSample for $t { const ZERO: Self = 0 as $t; })+ };
}
mode_samples!(i8, u8, i16, u16, i32, u32, i64, u64, i128, u128, isize, usize, f32, f64);

/// Preserve the selected rank after inserting a sample and evicting an extreme.
///
/// Capacities 1..=255 match the upstream uint8 parameter range, except that zero
/// is rejected and sizes 128..=255 fix the upstream signed indexing defect.
/// The selected rank applies only when full; warmup returns the middle sample.
#[derive(Clone, Debug)]
pub struct ModeFilter<T: ModeSample, const N: usize> {
    samples: [T; N],
    count: usize,
    return_element: usize,
    output: T,
    drop_high: bool,
}

impl<T: ModeSample, const N: usize> ModeFilter<T, N> {
    /// Construct with zero output; an out-of-range rank selects `N / 2`.
    ///
    /// Panics if capacity is outside 1..=255.
    pub const fn new(return_element: usize) -> Self {
        assert!(N > 0 && N <= 255, "capacity must be in 1..=255");
        Self {
            samples: [T::ZERO; N],
            count: 0,
            return_element: if return_element < N {
                return_element
            } else {
                N / 2
            },
            output: T::ZERO,
            drop_high: true,
        }
    }

    /// Add a sample, preserving the upstream strict-comparison ordering.
    pub fn apply(&mut self, sample: T) -> T {
        let mut drop_high = self.drop_high;
        if self.count < N {
            self.count += 1;
            drop_high = true;
        }
        if drop_high {
            let mut i = self.count - 1;
            while i > 0 && self.samples[i - 1] > sample {
                self.samples[i] = self.samples[i - 1];
                i -= 1;
            }
            self.samples[i] = sample;
        } else {
            let mut i = 0;
            while i < self.count - 1 && self.samples[i + 1] < sample {
                self.samples[i] = self.samples[i + 1];
                i += 1;
            }
            self.samples[i] = sample;
        }
        self.drop_high = !self.drop_high;
        self.output = self.samples[if self.count < N {
            self.count / 2
        } else {
            self.return_element
        }];
        self.output
    }

    /// Last selected value; zero before the first sample and after reset.
    pub const fn get(&self) -> T {
        self.output
    }

    /// Reset samples, rank-selection output and the extreme-eviction phase.
    pub fn reset(&mut self) {
        self.samples.fill(T::ZERO);
        self.count = 0;
        self.output = T::ZERO;
        self.drop_high = true;
    }

    pub const fn get_filter_size(&self) -> usize {
        N
    }

    /// Read a physical ordered-buffer slot, including unused zero slots.
    pub fn get_sample(&self, index: usize) -> Option<T> {
        self.samples.get(index).copied()
    }
}

impl<T: ModeSample, const N: usize> Default for ModeFilter<T, N> {
    fn default() -> Self {
        Self::new(N / 2)
    }
}

impl<T: ModeSample, const N: usize> crate::buffered::Filter<T> for ModeFilter<T, N> {
    type Output = T;
    fn apply(&mut self, sample: T) -> T {
        ModeFilter::apply(self, sample)
    }
    fn reset(&mut self) {
        ModeFilter::reset(self);
    }
}

/// Smooth low-noise derivative using the upstream sample/time weighting.
///
/// Supports the four instantiated upstream float capacities (5, 7, 9 and 11).
/// Timestamps are u32, with wrapping subtraction. The output is sample units per
/// timestamp unit; callers passing microseconds receive units per microsecond.
/// An initially zero timestamp is ignored because it equals initialized state.
#[derive(Clone, Debug)]
pub struct DerivativeFilter<const N: usize> {
    samples: [f32; N],
    sample_index: usize,
    timestamps: [u32; N],
    new_data: bool,
    last_slope: f32,
}

impl<const N: usize> DerivativeFilter<N> {
    /// Construct fully initialized; panics for capacities other than 5/7/9/11.
    pub const fn new() -> Self {
        assert!(
            N == 5 || N == 7 || N == 9 || N == 11,
            "derivative capacity must be 5, 7, 9 or 11"
        );
        Self {
            samples: [0.0; N],
            sample_index: 0,
            timestamps: [0; N],
            new_data: false,
            last_slope: 0.0,
        }
    }

    /// Record a sample unless its timestamp equals the previous physical slot.
    ///
    /// Monotonicity is not enforced, matching the source's u32 wrap behavior.
    pub fn update(&mut self, sample: f32, timestamp: u32) {
        let i = self.sample_index;
        let previous = if i == 0 { N - 1 } else { i - 1 };
        if self.timestamps[previous] == timestamp {
            return;
        }
        self.timestamps[i] = timestamp;
        self.apply(sample);
        self.new_data = true;
    }

    /// Upstream's inherited sample-only update, retained for API compatibility.
    ///
    /// This changes the ring position and sample but leaves timestamps and slope
    /// cache untouched. Use `update` for paired sample/timestamp observations.
    pub fn apply(&mut self, sample: f32) -> f32 {
        self.samples[self.sample_index] = sample;
        self.sample_index += 1;
        if self.sample_index >= N {
            self.sample_index = 0;
        }
        sample
    }

    fn term(&self, offset: usize, weight: f32) -> f32 {
        let center = self.sample_index + 3 * N / 2;
        let upper = (center + offset) % N;
        let lower = (center - offset) % N;
        weight * (self.samples[upper] - self.samples[lower])
            / self.timestamps[upper].wrapping_sub(self.timestamps[lower]) as f32
    }

    /// Recalculate when new data exists, or return the cached derivative.
    ///
    /// Preserves the physical last-two-timestamps warmup check, including cases
    /// where it permits a slope before every slot has a nonzero timestamp.
    /// Nonfinite arithmetic results become zero, as in upstream.
    pub fn slope(&mut self) -> f32 {
        if !self.new_data {
            return self.last_slope;
        }
        if self.timestamps[N - 1] == self.timestamps[N - 2] {
            return 0.0;
        }
        let mut result = match N {
            5 => (self.term(1, 4.0) + self.term(2, 4.0)) / 8.0,
            7 => (self.term(1, 10.0) + self.term(2, 16.0) + self.term(3, 6.0)) / 32.0,
            9 => {
                (self.term(1, 28.0) + self.term(2, 56.0) + self.term(3, 36.0) + self.term(4, 8.0))
                    / 128.0
            }
            11 => {
                (self.term(1, 84.0)
                    + self.term(2, 192.0)
                    + self.term(3, 162.0)
                    + self.term(4, 64.0)
                    + self.term(5, 10.0))
                    / 512.0
            }
            _ => unreachable!("constructor checks capacity"),
        };
        if !result.is_finite() {
            result = 0.0;
        }
        self.new_data = false;
        self.last_slope = result;
        result
    }

    /// Clear samples, timestamps, position, pending update and cached slope.
    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.timestamps.fill(0);
        self.sample_index = 0;
        self.new_data = false;
        self.last_slope = 0.0;
    }

    pub const fn get_filter_size(&self) -> usize {
        N
    }

    /// Read a physical ring-buffer slot; out-of-bounds reads return None.
    pub fn get_sample(&self, index: usize) -> Option<f32> {
        self.samples.get(index).copied()
    }
}

impl<const N: usize> Default for DerivativeFilter<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> crate::buffered::Filter<f32> for DerivativeFilter<N> {
    type Output = f32;
    fn apply(&mut self, sample: f32) -> f32 {
        DerivativeFilter::apply(self, sample)
    }
    fn reset(&mut self) {
        DerivativeFilter::reset(self);
    }
}
