# Vector arithmetic, predicates and projection verification

This increment adds the remaining float/double component arithmetic, cross
products, vector predicates, explicit upstream equality, checked indexing,
projection, axis reflection and perpendicular vectors. It extends the existing
`Vector<N>` and `Vector64<N>` types for the compared dimensions two and three.
The baseline is ArduPilot `afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

Every new numerical comparison checks the returned bits, including NaN signs,
payloads, quiet bits and signed zeros. Boolean predicates are checked exactly.
The overall migration is still unfinished: no vehicle firmware or hardware
target is qualified. Earlier filter comparisons retain their disclosed
NaN-payload exception.

## Behavior preserved and corrected

Returning a result and assigning it in place can select different NaN payloads
in the recorded C++ build. For example, double 2D addition prefers the left
component, while addition assignment prefers the right component. Projection
and reflection also have distinct inlined contexts. Rust implements those
observed choices explicitly; the oracle never normalizes NaNs.

Named `equals` and `not_equals` methods expose upstream epsilon equality.
Derived Rust `PartialEq` remains exact. For double equality with double
functions disabled, subtraction still occurs in double before the difference
is narrowed to float. The zero predicate and equality use different epsilon
rules when double functions are enabled.

Projection multiplies vector components before division. Rewriting it as a
scalar ratio changes finite rounding and overflow behavior; dedicated cases
protect the source order. Reflection is about an axis. Zero directions retain
the source arithmetic behavior, and perpendicular operations preserve the
strict comparison and epsilon early-return rules.

Indexing is a documented memory-safety correction. Rust borrows actual array
elements and rejects out-of-range indices by panic. The upstream operator
indexes from the scalar `x` member across separate struct members, outside
portable defined C++ pointer arithmetic. The fixture checks its host layout
and records valid-index observations. This limitation also applies to the
2D perpendicular source, which calls that index operator. Invalid C++ accesses
are never executed. The [predicate report](../crates/rustpilot-filters/VECTOR_PREDICATES_REPORT.md)
explains the source rules, correction and separate bounds tests.

## Recorded build profile

The coordinator's complete workspace checks use GCC 16.2.0 and glibc 2.41
inside the pinned x86-64 Linux image:

```text
docker.io/library/gcc@sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a
```

The C++ flags are `-std=c++17 -O2 -ffp-contract=off`, without LTO, fast math or
sanitizers. References set `HAL_DEBUG_BUILD=1` and compile the driver and
extracted vector definitions separately. Predicates also compile the actual
`AP_Math.cpp` equality helper in a separate translation unit. Header methods
retain their original definitions. Every consumed source is verified against
its pinned Git object before copying or verbatim extraction.

The tests compile all four combinations of `HAL_WITH_EKF_DOUBLE` and
`AP_MATH_ALLOW_DOUBLE_FUNCTIONS`. Driver guards require nearest rounding in
both the C floating environment and MXCSR, with FTZ and DAZ disabled. Observed
initial MXCSR is `0x1f80`. Rust uses toolchain 1.95.0. The
[machine-readable profile](../provenance/vector-primitives-reference-profile.json)
records source and fixture hashes. The crate reports also describe targeted
runs compiled in this image but executed on host glibc 2.44; those are a
separate runtime profile.

## Coordinator results

Verification date: October 18, 2026, America/Toronto.

| Check | Result |
| --- | --- |
| Complete workspace with all reference tests, debug | 118 passed, 0 failed, 0 ignored |
| Complete workspace with all reference tests, release | 118 passed, 0 failed, 0 ignored |
| Formatting and Clippy with warnings denied | Passed |
| Host release build | Passed |
| Core library compile, thumbv6m-none-eabi | Passed |
| Core library compile, thumbv7em-none-eabihf | Passed |
| Recursive inventory versus saved baseline | Byte-identical |

The increment adds 19 tests: six arithmetic, six predicate/indexing and seven
geometry tests. Three are reference matrices, enabled explicitly in the full
suite. Saved [debug](../provenance/verification-vector-primitives-debug.log) and
[release](../provenance/verification-vector-primitives-release.log) logs record
the complete runs. The [check record](../provenance/verification-vector-primitives-checks.json)
lists the other commands. Expected panic output comes from tests that verify
invalid capacities, indices or unsupported geometry dimensions.

Each implementation received a separate agent review, including source
extraction, independent entry-point comparisons and output counts. The
coordinator reviewed the combined integration. These automated reviews are
not independent human certification.
A further review used a separately generated geometry corpus combining zeros,
one, infinities and distinct NaNs in every effective input component. Those
scratch comparisons passed both double-function settings; they supplement
the review and are excluded from the committed corpus counts below.

## Numerical coverage

| New comparison family | Records per macro configuration | Actual floating outputs per configuration | Boolean outputs per configuration |
| --- | ---: | ---: | ---: |
| Arithmetic and cross products | 732,760 | 1,716,752 | 0 |
| Predicates, copies and host index observations | 39,570 | 813,902 | 197,850 |
| Projection, reflection and perpendicular vectors | 174,360 | 437,820 | 0 |
| Total | 946,690 | 2,968,474 | 197,850 |

Across all four configurations, each Rust build profile compares 11,873,896
floating outputs and 791,400 boolean outputs in these new families. These
counts exclude protocol padding. They include copies, zeroing, defaults and
existing operators exercised by the new fixtures; they are not counts of
unique scenarios or newly implemented functions. Repeated macro configurations
are not independent input coverage.

The corpora include bounded finite values, unrestricted raw bits, signed zeros,
subnormal boundaries, infinities, distinct quiet/signaling NaNs and true C++
self-aliasing calls. Arithmetic additionally checks all `3^7` combinations of
finite/positive-signaling/negative-quiet operand states and asserts that
335,872 outputs per configuration remain finite in its controlled-exponent
subset. Predicate fixtures cover every NaN fraction bit, epsilon neighbors and
the double-to-float rounding midpoint. Geometry fixtures test independent NaN
operands, finite rounding differences, overflow order and zero directions.

Details are in the [arithmetic report](../crates/rustpilot-filters/VECTOR_OPS_REPORT.md),
[predicate report](../crates/rustpilot-filters/VECTOR_PREDICATES_REPORT.md), and
[geometry report](../crates/rustpilot-filters/VECTOR_GEOMETRY_REPORT.md).

## Reproduction and limits

Use the pinned container setup in the
[exact-filter reproduction instructions](EXACT_FILTER_VERIFICATION.md#evidence-and-reproduction).
Set `ARDUPILOT_REFERENCE` to the pinned checkout, then run:

```sh
cargo test --workspace --locked -- --include-ignored --nocapture
cargo test --workspace --release --locked -- --include-ignored --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

An ordinary test run skips the C++ comparisons. A different compiler,
optimization setting, inlining context or target needs its own qualification.
The returned-bit contract excludes floating-point status flags, `errno`, other
rounding modes and a C++ ABI guarantee. Floating input coverage is finite.
Advanced geometry, rotations, matrices, integer vector variants and remaining
math are still unfinished.

The implementation adds no unsafe code, explicit runtime FFI or Cargo
dependency. Current builds still use the compiler, linker and host OS listed
in [BOOTSTRAP.md](BOOTSTRAP.md). The Cortex-M checks compile libraries; they do
not establish hardware execution, timing, stack budgets or vehicle safety.
