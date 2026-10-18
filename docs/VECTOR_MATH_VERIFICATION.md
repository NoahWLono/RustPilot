# Vector magnitude and square-root verification

This increment adds float and double vector magnitudes, normalization, dot
products, scalar division, length limiting, 3D squared distance and precision
conversion. It also adds safe Rust square roots for binary32 and binary64.
The ArduPilot baseline remains
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

All new reference comparisons require exact returned bits, including signed
zeros and every NaN sign, payload and quiet bit. The older scalar and buffered
filter comparisons retain their disclosed NaN exception. The complete
migration remains unfinished, with no working vehicle firmware or qualified
hardware target.

## Reference profile

The coordinator ran the entire workspace in the same GCC 16.2.0 x86-64 Linux
container used by CI, including its glibc 2.41 runtime:

```text
docker.io/library/gcc@sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a
```

The C++ flags are `-std=c++17 -O2 -ffp-contract=off`, without LTO, fast math
or sanitizers. Vector references use `HAL_DEBUG_BUILD=1`, separate driver and
implementation translation units, and the actual inline upstream methods.
Every consumed upstream file is checked against its pinned Git object before
compilation or verbatim extraction. The tests cross both values of
`HAL_WITH_EKF_DOUBLE` and `AP_MATH_ALLOW_DOUBLE_FUNCTIONS`.

Reference drivers check round-to-nearest and reject enabled FTZ or DAZ. The
observed initial MXCSR was `0x00001f80`. Rust uses toolchain 1.95.0. The
[machine-readable profile](../provenance/vector-math-reference-profile.json)
records source and fixture hashes. The crate reports also retain earlier
checks compiled by the same GCC image but executed on host glibc 2.44; those
are a separate runtime profile.

## Coordinator checks

Verification date: October 18, 2026, America/Toronto.

| Check | Result |
| --- | --- |
| Entire workspace, all reference tests enabled, debug | 99 passed, 0 failed, 0 ignored |
| Entire workspace, all reference tests enabled, release | 99 passed, 0 failed, 0 ignored |
| Formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Host workspace release build | Passed |
| Core library compile, thumbv6m-none-eabi | Passed |
| Core library compile, thumbv7em-none-eabihf | Passed |
| Recursive inventory versus saved baseline | Byte-identical |

The ten new tests comprise five square-root and five vector-magnitude tests.
All previous component tests remain in the workspace suite. Normalized outputs
are in [debug](../provenance/verification-vector-math-debug.log) and
[release](../provenance/verification-vector-math-release.log) logs; other command
results are in [the check record](../provenance/verification-vector-math-checks.json).
Panic output from the older invalid-capacity tests is expected and caught by
those tests.

## Numerical coverage

| New corpus | Exact comparisons per Rust build profile |
| --- | ---: |
| Structured/random binary32 square roots | 1,138,842 |
| Structured/random binary64 square roots | 1,192,718 |
| Every positive binary32 subnormal and every binary32 input in `[1,4)` | 25,165,823 |
| Every binary32 NaN encoding, both signs | 16,777,214 |
| Vector matrix, all four precision-macro configurations | 6,938,096 floating results and 315,368 boolean flags |

The square-root corpora total 44,274,597 comparisons per Rust profile. The
vector matrix contains 906,683 operation records per macro configuration;
record padding is excluded from the floating-result counts above. Repeated
configurations and overlapping corpora are not independent scenario counts.

The vector fixtures exercise both dimensions and storage precisions, bounded
finite inputs, unrestricted raw bits, independently generated NaN operands,
conversion midpoints and extreme values. Named regressions preserve the
upstream association of sums, narrowing before double-vector float norms,
zero normalization, the limiting threshold, and unchanged z bits.

The [square-root report](../crates/rustpilot-filters/SQUARE_ROOT_REPORT.md)
gives the integer algorithm, arithmetic bounds and rounding argument. Its
base-four loop takes 24 iterations for binary32 or 53 for binary64, followed
by an exact remainder comparison. An independent agent reviewed this argument
and compared additional sampled roots with high-precision decimal calculations.
That review is not a machine-checked proof. Neither full floating-point domain
has been exhaustively compared with C++.

The [vector report](../crates/rustpilot-filters/VECTOR_MATH_REPORT.md)
explains each operation and its compiler-specific NaN choices. The independent
review also used a separately authored actual-source C++ driver, including
distinct NaNs in all seven input positions. Those scratch comparisons support
the review; the committed harness and counts above are the reproducible suite.

## Reproduction and limits

Use the pinned container setup in the earlier
[exact-filter reproduction instructions](EXACT_FILTER_VERIFICATION.md#evidence-and-reproduction).
With the reference checkout and compiler selected, the complete checks are:

```sh
cargo test --workspace --locked -- --include-ignored --nocapture
cargo test --workspace --release --locked -- --include-ignored --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

`ARDUPILOT_REFERENCE` must point to the pinned checkout. The new exhaustive
tests need several hundred megabytes when run concurrently. An ordinary test
run skips the C++ comparisons; a different reference compiler is not an
interchangeable source of NaN bits.

The returned-bit contract excludes floating-point exception flags, `errno`
and other rounding modes. The Rust vector types do not promise C++ ABI
compatibility, and their exact `PartialEq` remains different from upstream
epsilon equality. Geometry, rotations and remaining vector operators are
unfinished. Each firmware compiler, configuration and call context still
requires its own qualification.

The new algorithms add no explicit runtime FFI, unsafe code or Cargo
dependency. Current builds still use the existing compiler, linker, container
and operating system described in [BOOTSTRAP.md](BOOTSTRAP.md). The Cortex-M
checks compile libraries only; they do not execute these routines on a board,
measure their timing or establish vehicle safety.
