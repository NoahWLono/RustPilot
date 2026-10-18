# Filter migration scope

The buffered-filter increment covers the buffered-filter family and fixed-coefficient
Butterworth filters at the existing upstream pin. It advances common vehicle
dependencies without runtime HAL, parameter storage, allocation, or a new
transcendental-math backend. It remains a component increment.

Its implementation follows the [buffered-filter plan](superpowers/plans/2026-10-17-buffered-filters.md).
See [verification](BUFFERED_VERIFICATION.md) for observed results. The remaining
Filter work below stays in scope for subsequent increments.

## Source scope

- `libraries/Filter/FilterClass.h`: apply/reset interface.
- `libraries/Filter/FilterWithBuffer.h`: circular buffer, physical-slot access,
  capacity, reset and pass-through apply.
- `libraries/Filter/AverageFilter.h`: ordinary and integral averages, including
  upstream accumulator/type aliases.
- `libraries/Filter/ModeFilter.h` and `.cpp`: ordered retained samples with
  alternating high/low eviction, output and return-index handling.
- `libraries/Filter/DerivativeFilter.h` and `.cpp`: float specializations for
  capacities 5, 7, 9 and 11, timestamp handling and slope cache.
- `libraries/Filter/Butter.h`: all nine coefficient sets and 26 aliases.

The upstream source remains the authority for constants and arithmetic order.
Read it before implementation. A fresh implementation plan and tests must
precede each port.

## Required decisions and tests

Buffer slots are physical slots. Reject zero capacity. The C++ reset loop uses
an `int8_t` counter, so direct comparison should stay within capacities 1 through
127. Any larger Rust capacity support needs separate documented semantics.

Ordinary averages sum the physical array in its original order on each apply;
a running sum would change float rounding. Preserve warmup counts, truncation
rules, and accumulator types. Some integer aliases use float accumulators.
Do not claim parity for undefined C++ overflow or out-of-range conversions.

Integral-average apply returns zero; `getf` and `getd` expose its average.
Upstream's inherited reset clears buffer/count but retains `_sum`. Specify a
documented corrected reset and prove the intentional difference with a test.
The owner has authorized documented fixes during conversion.

ModeFilter alternates eviction of the largest and smallest retained sample
after warmup. It is not a conventional sliding-window median. Preserve warmup
output indexing, invalid-index fallback, equality and NaN comparisons. Upstream
reset retains output and the eviction flag; initialization and any corrected
reset need explicit contracts and divergence tests.

DerivativeFilter uses wrapping unsigned 32-bit timestamp subtraction. It
rejects timestamps equal to the latest stored timestamp, including an initial
zero in zero-initialized reference state. Preserve exact ring indexing,
coefficients and arithmetic ordering. Outputs are per supplied timestamp unit.
Test duplicates, wraparound, warmup, caching and nonfinite results. Upstream
reset leaves timestamps/cache/new-data intact; specify corrected Rust reset
behavior separately from comparison cases.

Butterworth filters require all nine f32 coefficient triples and alias mappings.
Preserve recurrence ordering and history updates. Explicit Rust construction
and reset may define the zero state obtained through C++ zero-initialization.
Cover every coefficient set, long sequences, impulses and nonfinite values.

## Verification

Use tests before implementation, no unsafe code, no external Cargo dependencies,
and direct compilation of verified pinned C++ inputs. The derivative source can
use a disclosed minimal AP_Math shim; keep its actual algorithm. Use
`-ffp-contract=off`. Test integer boundaries and reset divergences explicitly.
Repeat debug/release comparisons, host checks and the two existing Cortex-M
library builds. Update unit mappings without adding firmware-support claims.

## Remaining Filter scope

The inspected Filter tree has 47 files: 24 root C++ source/header files, six
C++ examples, three C++ tests, seven Waf scripts, two MATLAB files and five
gnuplot files. This increment does not cover all of them.

The subsequent [vector/two-pole increment](EXACT_FILTER_VERIFICATION.md)
implements the first-order vector variants and two-pole coefficient design
using a native Rust tangent backend and the formula's fixed cosine value.
It qualifies a recorded host component profile, including NaN output bits.
The [magnitude increment](VECTOR_MATH_VERIFICATION.md) subsequently adds
float/double vector norms, normalization, division, dot products, limiting and
precision conversion, plus square-root routines. Remaining vector math and
general cosine support are unfinished.
Notch and harmonic-notch filters also need
parameter metadata, logging, dynamic storage and math functions. SlewLimiter
uses the HAL clock and an exponential. Filter registry/configuration, HAL
examples, tests and analysis/build tools each remain migration work.

The owner requires [bit-for-bit numerical matches](NUMERICAL_COMPATIBILITY.md),
including NaN payload and sign bits, for each target/compiler build. The
recorded GCC 16.2.0 profile does not qualify other builds. Characterize future
transcendental routines and compiler profiles before choosing them.
Tolerance-based acceptance does not satisfy this requirement. Older filter
comparisons retain their disclosed NaN exception and still need stricter
profile qualification if they are to meet this same gate.

The broader vehicle, hardware, dependency, compiler and Linux-compatible host
work remains in [MIGRATION.md](MIGRATION.md) and [BOOTSTRAP.md](BOOTSTRAP.md).
