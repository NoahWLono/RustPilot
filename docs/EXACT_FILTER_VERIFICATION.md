# Exact vector, tangent and two-pole verification

This increment adds first-order Vector2f/Vector3f filters, complete two-pole
coefficient generation and filtering for all five supplied sample types, and
a native Rust tangent backend. The ArduPilot source remains pinned to
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

The owner requires exact output bits, including signed zero and NaN payload
and sign bits, for each target/compiler build. These new comparisons have no
NaN exemption or numerical tolerance. Earlier filter reports retain their
stated NaN exception. The complete migration and every firmware target remain
unfinished.

## Recorded component profile

The reference compiler is GCC 16.2.0 on x86-64 Linux, from Docker Official
Image `gcc:16.2.0-trixie`, pinned to its linux/amd64 manifest:

```text
sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a
```

The vector and biquad comparisons use C++17, `-O2`, `-ffp-contract=off`,
`HAL_DEBUG_BUILD=1`, separate algorithm/driver translation units, no LTO and
no sanitizer instrumentation for output-bit equality. A separate biquad run
uses undefined-behavior and float-cast-overflow sanitizers to check the integer
fixture domain. The FPU contract is nearest-even rounding with gradual
underflow, FTZ disabled and DAZ disabled. The observed initial x86 MXCSR was
`0x00001f80`.

The container contains glibc 2.41. The baseline explicitly compiles the exact
MIT tangent source from glibc commit
`1848099f063e99d4ffecbd7667766d54862398b9`; it does not use an arbitrary
installed tangent implementation. A supplemental comparison runs against the
development host's hash-checked glibc 2.44 library. Source and fixture hashes
are saved in [the reference profile](../provenance/exact-reference-profile.json).

Rust uses toolchain 1.95.0 and the x86_64 Linux target. The coordinator ran the
entire workspace inside the pinned GCC container in both Rust build profiles,
then separately checked the native-host math comparison. This is a host
component profile. Real firmware builds can select different NaN payloads
through different compilers, flags, inlining or target instructions. Each such
build still requires its own verification.

## Observed results

Verification took place on October 17, 2026 in America/Toronto (October 18 UTC).

| Check | Result |
| --- | --- |
| Entire workspace, all reference tests enabled, debug | 89 passed, 0 failed, 0 ignored |
| Entire workspace, all reference tests enabled, release | 89 passed, 0 failed, 0 ignored |
| Additional tangent/biquad tests against hash-pinned native host math | 15 passed in debug and 15 in release |
| Formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Host workspace release build | Passed |
| Core library compile, thumbv6m-none-eabi | Passed |
| Core library compile, thumbv7em-none-eabihf | Passed |
| Recursive inventory compared with saved baseline | Byte-identical |

The 26 new tests comprise four tangent, eleven vector and eleven biquad tests.
The remaining 63 tests retain the earlier checksum, filter and inventory
coverage. The full reference runs include tests explicitly ignored by an
ordinary `cargo test` invocation.

The following new numerical fixtures pass in both Rust profiles:

| Fixture | Scope and exact scalar observations |
| --- | --- |
| Tangent structured/random corpus | 1,010,258 input/output bit comparisons, including all float classes and the known rejected-library regression |
| Tangent tiny boundary binade | All 16,777,216 signed inputs at exponent field 114; every significand tested |
| Vector broad histories and operations | 222,294 records; 778,029 observations |
| Vector finite histories and operations | 241,642 records; 845,747 observations; results required to remain finite |
| Vector independent fresh-state stress | 540,000 records; 1,890,000 observations |
| Two-pole and direct biquad broad corpus | 437,482 records; 3,606,236 observations |
| Biquad finite-input corpus | 421,509 records; 3,462,999 observations; any resulting nonfinite bits also checked |
| Direct biquad independent stress | 150,000 records; 1,350,000 observations |
| Biquad sanitizer domain check | All 437,482 broad-corpus records completed without a sanitizer failure |

The biquad corpus includes 270,676 sample-rate/cutoff pairs and caller-supplied
coefficients. It covers scalar, both integer widths and both vector dimensions,
including reset, disable/re-enable, configuration changes and numerical
boundaries. Scalar reset and vector reset preserve their different rounding
points. Integer errors leave history and initialization unchanged.

The tangent and broad biquad corpora also match the audited native host library
in debug and release. The host check is explicitly enabled with
`RUSTPILOT_TEST_HOST_GLIBC=1` and fails if the installed library hash differs.
CI performs the pinned-source baseline and does not claim that host-specific
supplemental check.

Counts include repeated state and coefficient observations. They are not
independent scenario counts or an exhaustive proof over every input sequence.
Only the stated tangent binade is exhaustively enumerated. FPU exception
flags, C errno, timing, stack bounds and complete target operation are outside
these comparisons.

## Evidence and reproduction

Normalized outputs are saved in:

- [Full workspace debug](../provenance/verification-exact-debug.log).
- [Full workspace release](../provenance/verification-exact-release.log).
- [Supplemental native math debug](../provenance/verification-exact-native-debug.log).
- [Supplemental native math release](../provenance/verification-exact-native-release.log).
- [Other check results](../provenance/verification-exact-checks.json).

The logs contain expected panic messages from tests that reject invalid buffer
capacities. Their test results are passing. Source paths are normalized to
`$RUSTPILOT`, and the container build directory to `$CONTAINER_TARGET`.

The [CI workflow](../.github/workflows/verify.yml) records the pinned container,
Rust installation, upstream checkout and complete commands. Run comparisons
with that compiler profile, setting `CXX` to the corresponding compiler or
container wrapper and `ARDUPILOT_REFERENCE` to the pinned source checkout:

```sh
cargo test --workspace --locked -- --include-ignored --nocapture
cargo test --workspace --release --locked -- --include-ignored --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

The crate reports document exact source extraction, shims, intentional fixes
and API scope: [vector](../crates/rustpilot-filters/VECTOR_REPORT.md),
[biquad](../crates/rustpilot-filters/BIQUAD_REPORT.md), and
[tangent](../crates/rustpilot-filters/TANGENT_REPORT.md).
The [review record](REVIEW.md) distinguishes independent agent checks from the
coordinator's combined verification. These are AI-assisted component checks;
human review, integrated simulation, hardware timing and flight qualification
remain outstanding.
