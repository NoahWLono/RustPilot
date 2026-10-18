// SPDX-License-Identifier: GPL-3.0-or-later
// Derived from ArduPilot libraries/Filter/{FilterClass,FilterWithBuffer,AverageFilter}.h
// at afeb9c4941543d97f3e04fecf257b8f598f49a6a. Credit: the ArduPilot contributors.
//! Fixed-capacity circular buffers and averages from pinned ArduPilot source.
//!
//! Capacities 1 through 255 are accepted. The upstream reset loop uses a signed
//! eight-bit index and cannot safely initialize capacities 128 through 255;
//! this implementation corrects that loop. Empty/oversized buffers panic at
//! construction. All storage is initialized with `T::default()` (zero for the
//! supported arithmetic scalar types).
//!
//! Average updates return errors for signed accumulator overflow or an
//! out-of-range floating-to-integer result, without modifying the filter.
//! Unsigned accumulator arithmetic retains modulo wrapping. Floating arithmetic
//! retains IEEE propagation and the physical-slot summation order.

/// Common apply/reset interface corresponding to upstream `Filter<T>`.
///
/// `Output` makes checked average errors explicit while raw buffers return `T`.
pub trait Filter<T> {
    type Output;
    fn apply(&mut self, sample: T) -> Self::Output;
    fn reset(&mut self);
}

/// Rejected arithmetic. The attempted update leaves samples, index and count
/// unchanged, and leaves the integral accumulator unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticError {
    /// Signed accumulator addition/subtraction would exceed its type's range.
    AccumulatorOverflow,
    /// A floating average cannot be represented by the output integer type.
    OutputOutOfRange,
}

/// Circular storage whose apply operation returns the unmodified sample.
#[derive(Debug, Clone)]
pub struct FilterWithBuffer<T, const N: usize> {
    samples: [T; N],
    next: usize,
}

impl<T: Copy + Default, const N: usize> Default for FilterWithBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, const N: usize> FilterWithBuffer<T, N> {
    /// Construct initialized storage. Panics unless `1 <= N <= 255`.
    pub fn new() -> Self {
        assert!((1..=255).contains(&N), "filter capacity must be in 1..=255");
        Self {
            samples: [T::default(); N],
            next: 0,
        }
    }

    pub fn apply(&mut self, sample: T) -> T {
        self.samples[self.next] = sample;
        self.next += 1;
        if self.next == N {
            self.next = 0;
        }
        sample
    }

    pub fn reset(&mut self) {
        self.samples.fill(T::default());
        self.next = 0;
    }

    pub const fn get_filter_size(&self) -> u8 {
        N as u8
    }

    /// Read a physical slot, independent of circular insertion order.
    pub fn get_sample(&self, index: usize) -> Option<T> {
        self.samples.get(index).copied()
    }
}

impl<T: Copy + Default, const N: usize> Filter<T> for FilterWithBuffer<T, N> {
    type Output = T;
    fn apply(&mut self, sample: T) -> T {
        self.apply(sample)
    }
    fn reset(&mut self) {
        self.reset();
    }
}

/// Scalar arithmetic used by the average filters.
///
/// Implementations are supplied for the upstream alias pairs and integer
/// same-width/widening pairs through 32 bits. Custom implementations must use
/// `ZERO` as the additive identity and implement the desired source conversion
/// semantics. Built-in signed arithmetic is checked, unsigned is wrapping.
pub trait AverageAccumulator<T>: Copy {
    const ZERO: Self;
    fn add_sample(self, sample: T) -> Result<Self, ArithmeticError>;
    fn subtract_sample(self, sample: T) -> Result<Self, ArithmeticError>;
    /// Divide by a nonzero count. The caller must supply a count in 1..=255.
    fn average(self, count: u8) -> Result<T, ArithmeticError>;
}

/// Integral accumulators eligible for running-sum averages.
pub trait IntegralAccumulator<T>: AverageAccumulator<T> {
    fn as_f32(self) -> f32;
    fn as_f64(self) -> f64;
}

macro_rules! signed_pairs {
    ($(($sample:ty, $sum:ty)),* $(,)?) => {$(
        impl AverageAccumulator<$sample> for $sum {
            const ZERO: Self = 0;
            fn add_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                self.checked_add(sample as Self).ok_or(ArithmeticError::AccumulatorOverflow)
            }
            fn subtract_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                self.checked_sub(sample as Self).ok_or(ArithmeticError::AccumulatorOverflow)
            }
            fn average(self, count: u8) -> Result<$sample, ArithmeticError> {
                // C++ promotes narrow integers before division. In particular,
                // counts 128..255 remain positive even for an i8 accumulator.
                Ok((i64::from(self) / i64::from(count)) as $sample)
            }
        }
        impl IntegralAccumulator<$sample> for $sum {
            fn as_f32(self) -> f32 { self as f32 }
            fn as_f64(self) -> f64 { self as f64 }
        }
    )*};
}

macro_rules! unsigned_pairs {
    ($(($sample:ty, $sum:ty)),* $(,)?) => {$(
        impl AverageAccumulator<$sample> for $sum {
            const ZERO: Self = 0;
            fn add_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                Ok(self.wrapping_add(sample as Self))
            }
            fn subtract_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                Ok(self.wrapping_sub(sample as Self))
            }
            fn average(self, count: u8) -> Result<$sample, ArithmeticError> {
                Ok((u64::from(self) / u64::from(count)) as $sample)
            }
        }
        impl IntegralAccumulator<$sample> for $sum {
            fn as_f32(self) -> f32 { self as f32 }
            fn as_f64(self) -> f64 { self as f64 }
        }
    )*};
}

signed_pairs!(
    (i8, i8),
    (i8, i16),
    (i8, i32),
    (i16, i16),
    (i16, i32),
    (i32, i32)
);
unsigned_pairs!(
    (u8, u8),
    (u8, u16),
    (u8, u32),
    (u16, u16),
    (u16, u32),
    (u32, u32)
);

macro_rules! float_integer_pair {
    ($sample:ty, $lower:expr, $upper:expr) => {
        impl AverageAccumulator<$sample> for f32 {
            const ZERO: Self = 0.0;
            fn add_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                Ok(self + sample as f32)
            }
            fn subtract_sample(self, sample: $sample) -> Result<Self, ArithmeticError> {
                Ok(self - sample as f32)
            }
            fn average(self, count: u8) -> Result<$sample, ArithmeticError> {
                let value = self / f32::from(count);
                // Bounds are exactly representable for these integer types.
                // Requiring both comparisons also rejects NaN/infinity.
                if ($lower..$upper).contains(&value) {
                    Ok(value as $sample)
                } else {
                    Err(ArithmeticError::OutputOutOfRange)
                }
            }
        }
    };
}
float_integer_pair!(i32, -2_147_483_648.0_f32, 2_147_483_648.0_f32);
float_integer_pair!(u32, 0.0_f32, 4_294_967_296.0_f32);

impl AverageAccumulator<f32> for f32 {
    const ZERO: Self = 0.0;
    fn add_sample(self, sample: f32) -> Result<Self, ArithmeticError> {
        Ok(self + sample)
    }
    fn subtract_sample(self, sample: f32) -> Result<Self, ArithmeticError> {
        Ok(self - sample)
    }
    fn average(self, count: u8) -> Result<f32, ArithmeticError> {
        Ok(self / f32::from(count))
    }
}

/// Average recomputed over physical slots after each accepted sample.
///
/// `T` is the sample/output type; `U` is the source accumulator type. All
/// upstream aliases map directly to these generic types (see BUFFERED_REPORT).
#[derive(Debug, Clone)]
pub struct AverageFilter<T, U, const N: usize> {
    buffer: FilterWithBuffer<T, N>,
    count: u8,
    accumulator: core::marker::PhantomData<U>,
}

impl<T: Copy + Default, U: AverageAccumulator<T>, const N: usize> Default
    for AverageFilter<T, U, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, U: AverageAccumulator<T>, const N: usize> AverageFilter<T, U, N> {
    /// Construct a zero-sample average; panics unless `1 <= N <= 255`.
    pub fn new() -> Self {
        Self {
            buffer: FilterWithBuffer::new(),
            count: 0,
            accumulator: core::marker::PhantomData,
        }
    }

    pub fn apply(&mut self, sample: T) -> Result<T, ArithmeticError> {
        let count = if usize::from(self.count) < N {
            self.count + 1
        } else {
            self.count
        };
        let mut sum = U::ZERO;
        // Compute the candidate result without mutating storage. Keeping this
        // physical order matters for floating accumulators after ring wrap.
        for (index, &stored) in self.buffer.samples.iter().enumerate() {
            sum = sum.add_sample(if index == self.buffer.next {
                sample
            } else {
                stored
            })?;
        }
        let result = sum.average(count)?;
        self.buffer.apply(sample);
        self.count = count;
        Ok(result)
    }

    pub fn reset(&mut self) {
        self.buffer.reset();
        self.count = 0;
    }
    pub const fn get_filter_size(&self) -> u8 {
        self.buffer.get_filter_size()
    }
    pub fn get_sample(&self, index: usize) -> Option<T> {
        self.buffer.get_sample(index)
    }
}

impl<T: Copy + Default, U: AverageAccumulator<T>, const N: usize> Filter<T>
    for AverageFilter<T, U, N>
{
    type Output = Result<T, ArithmeticError>;
    fn apply(&mut self, sample: T) -> Self::Output {
        self.apply(sample)
    }
    fn reset(&mut self) {
        self.reset();
    }
}

/// Running-sum integral average. Successful apply returns `Ok(T::default())`,
/// corresponding to upstream's zero return; read the average using getf/getd.
///
/// Reset also clears the sum, correcting the upstream inherited-reset defect.
#[derive(Debug, Clone)]
pub struct AverageIntegralFilter<T, U, const N: usize> {
    buffer: FilterWithBuffer<T, N>,
    count: u8,
    sum: U,
}

impl<T: Copy + Default, U: IntegralAccumulator<T>, const N: usize> Default
    for AverageIntegralFilter<T, U, N>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + Default, U: IntegralAccumulator<T>, const N: usize> AverageIntegralFilter<T, U, N> {
    pub fn new() -> Self {
        Self {
            buffer: FilterWithBuffer::new(),
            count: 0,
            sum: U::ZERO,
        }
    }

    pub fn apply(&mut self, sample: T) -> Result<T, ArithmeticError> {
        // Preserve separate subtraction/addition checks: even a representable
        // final sum can have an overflowing source intermediate operation.
        let sum = self
            .sum
            .subtract_sample(self.buffer.samples[self.buffer.next])?
            .add_sample(sample)?;
        self.buffer.apply(sample);
        if usize::from(self.count) < N {
            self.count += 1;
        }
        self.sum = sum;
        Ok(T::default())
    }

    pub fn getf(&self) -> f32 {
        if self.count == 0 {
            0.0
        } else {
            self.sum.as_f32() / f32::from(self.count)
        }
    }
    pub fn getd(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum.as_f64() / f64::from(self.count)
        }
    }
    pub fn reset(&mut self) {
        self.buffer.reset();
        self.count = 0;
        self.sum = U::ZERO;
    }
    pub const fn get_filter_size(&self) -> u8 {
        self.buffer.get_filter_size()
    }
    pub fn get_sample(&self, index: usize) -> Option<T> {
        self.buffer.get_sample(index)
    }
}

impl<T: Copy + Default, U: IntegralAccumulator<T>, const N: usize> Filter<T>
    for AverageIntegralFilter<T, U, N>
{
    type Output = Result<T, ArithmeticError>;
    fn apply(&mut self, sample: T) -> Self::Output {
        self.apply(sample)
    }
    fn reset(&mut self) {
        self.reset();
    }
}
