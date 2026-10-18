# Buffered and average filter conversion

Source: ArduPilot `libraries/Filter/FilterClass.h`, `FilterWithBuffer.h`, and
`AverageFilter.h`, pinned at `afeb9c4941543d97f3e04fecf257b8f598f49a6a`.
The algorithms are credited to the ArduPilot contributors and remain under
GPL-3.0-or-later. This increment was translated and checked with AI assistance.

## Source-to-Rust interfaces

`buffered::Filter<T>` retains apply/reset as a common interface. Its associated
`Output` type permits the buffer's direct return and the averages' checked
`Result` return. The source interface aliases `FilterInt8`, `FilterUInt8`,
`FilterInt16`, `FilterUInt16`, `FilterInt32`, and `FilterUInt32` correspond to
`Filter<i8>`, `Filter<u8>`, `Filter<i16>`, `Filter<u16>`, `Filter<i32>`, and
`Filter<u32>`. Rust callers also specify the implementer's output type when
using a trait object.

| Upstream family | Rust equivalent | Source alias capacities |
| --- | --- | --- |
| `FilterWithBufferInt16_SizeN` | `FilterWithBuffer<i16, N>` | 2 through 7 |
| `FilterWithBufferUInt16_SizeN` | `FilterWithBuffer<u16, N>` | 2 through 7 |
| `FilterWithBufferInt32_SizeN` | `FilterWithBuffer<i32, N>` | 2 through 7 |
| `FilterWithBufferUInt32_SizeN` | `FilterWithBuffer<u32, N>` | 2 through 7 |
| `AverageFilterInt8_SizeN` | `AverageFilter<i8, i16, N>` | 2 through 5 |
| `AverageFilterUInt8_SizeN` | `AverageFilter<u8, u16, N>` | 2 through 5 |
| `AverageFilterInt16_SizeN` | `AverageFilter<i16, i32, N>` | 2 through 5 |
| `AverageFilterUInt16_SizeN` | `AverageFilter<u16, u32, N>` | 2 through 5 |
| `AverageFilterInt32_SizeN` | `AverageFilter<i32, f32, N>` | 2 through 5 |
| `AverageFilterUInt32_SizeN` | `AverageFilter<u32, f32, N>` | 2 through 5 |
| `AverageFilterFloat_Size5` | `AverageFilter<f32, f32, 5>` | 5 |
| `AverageIntegralFilter<T, U, N>` | `AverageIntegralFilter<T, U, N>` | generic; BMP085 uses `i32, i32, 10` |

All names in this table are available through generic Rust types; separate
aliases for each capacity are unnecessary. The raw buffer supports any
`Copy + Default` sample type and initializes storage with its default value.
Defaults for the arithmetic scalar types above are zero.

Built-in average accumulator implementations cover these sample/sum pairs:
`i8/i8`, `i8/i16`, `i8/i32`, `i16/i16`, `i16/i32`, `i32/i32`,
`u8/u8`, `u8/u16`, `u8/u32`, `u16/u16`, `u16/u32`, `u32/u32`,
`i32/f32`, `u32/f32`, and `f32/f32`. The first twelve pairs also support
`AverageIntegralFilter`. Applications can implement the public arithmetic
traits for additional numeric types. Those implementations are outside this
verification result; this report does not claim every possible C++ template
instantiation has been converted.

## Preserved behavior

The buffer returns its input, stores samples in a circular array, and exposes
physical slots. Average filters sum those physical slots in their original
order on every apply. During warmup, zero-filled unused slots contribute to the
sum while the divisor counts actual samples. Counts saturate at capacity.
Integer division truncates toward zero. Floating accumulators retain `f32`
addition and conversion order; `f32/f32` averages retain nonfinite propagation.

Integral averages subtract the evicted sample before adding the new sample.
Their apply result is zero; `getf()` casts the sum to `f32` before division, and
`getd()` casts to `f64` before division. Empty averages return zero. Unsigned
accumulator arithmetic wraps modulo its width, as in the defined C++ behavior.
Narrow integer promotions are accounted for when dividing by an eight-bit count.

## Documented corrections and API changes

- Capacities are checked at construction and must be 1 through 255. Safe array
  iteration corrects the signed eight-bit reset-loop index for capacities 128
  through 255. Zero and larger capacities panic at construction.
- `get_sample(usize)` returns `Option<T>`. Out-of-range access returns `None`.
- Average apply returns `Result<T, ArithmeticError>`. Signed accumulator
  overflow and out-of-range floating-to-integer results are rejected without
  modifying samples, insertion index, count or integral sum. Source `i32`
  arithmetic overflow and out-of-range floating-to-integer conversion are
  undefined. Narrow signed arithmetic can instead involve integer promotion
  followed by implementation-defined narrowing in C++17; the Rust error policy
  intentionally rejects that case too.
- Integral reset clears the sum as well as the buffer and sample count. The
  source inherits a reset that leaves its sum intact. A reference test applies
  100 and 200, resets, then applies 8. Original C++ reports 308; Rust reports 8.
  The reference implementation is unmodified in this test.

Each signed integral update checks subtraction and addition separately. An
update can therefore fail even when its final mathematical sum would fit, if
an intermediate source operation would overflow. These explicit failures need
handling when call sites are ported. They do not establish a vehicle-level
fallback policy.

## Verification scope

Ten behavioral tests cover circular wrap, physical access, warmup, truncation,
capacity limits, full-capacity count saturation, floating summation order,
nonfinite propagation, checked transactional errors, unsigned wrapping, reset,
and use through the common Filter trait.

Two opt-in reference tests require `ARDUPILOT_REFERENCE` and a C++ compiler.
The comparison compiles verified copies of the three actual pinned headers,
with no algorithm or dependency shims. Compilation uses `-O2`,
`-ffp-contract=off`, `-fsanitize=undefined,float-cast-overflow`, and
`-fno-sanitize-recover=all`. Inputs include fixed-seed random values, numeric
boundaries, negative samples, ring wrap, and resets where source behavior is
intended to match.

The main comparison covers 71 cases, 13,041 operation records and 91,582 scalar
observations. It exercises all declared raw-buffer and average alias families,
all implemented integral type pairs, the actual BMP085 `i32/i32/10` type, and
raw-buffer capacities 1 and 127. Values, physical slots and floating bit
patterns match exactly; NaN payloads are excluded from bit equality. Integral
reset is checked separately as the explicit correction described above.

Reference execution is restricted to defined source arithmetic and capacities
at most 127. Corrected capacities and error paths have Rust behavioral tests;
no undefined C++ result is used as an expected value. These tests do not verify
all possible inputs, a target ABI, instruction timing, a sensor driver, a
vehicle, firmware integration, or flight readiness.
