# Numerical compatibility requirements

On October 17, 2026, the owner required bit-for-bit reference matches for the
next trigonometric, square-root and dependent filter ports. Numerical tolerances
must not turn mismatches into passing compatibility tests.

The next implementation must compare the actual output bits, including signed
zero and NaN encodings. Record mismatches explicitly. A correct mathematical
approximation, a small error bound or a passing recurrence test with supplied
coefficients does not establish compatibility of the full coefficient-generating
filter. A candidate Rust math library must be characterized before adoption.

## Reference configuration

The source pin alone does not define every floating-point result. Record the
target architecture, ABI, C/C++ compiler and flags, math-library implementation,
rounding mode and relevant floating-point control state. Compare Rust and C++
under the recorded configuration, including operation ordering and contraction
settings. Host matches do not prove embedded execution matches. Qualification
of each target remains required by the full migration goal.

The owner previously authorized documented bug fixes during conversion. Where
upstream executes undefined operations, such as an out-of-range floating-to-
integer conversion, there is no portable reference result to preserve. Specify
the corrected behavior separately and test it. Do not present an intentional
correction as a successful bitwise match.

## Current evidence and next work

The published checksum and filter reports retain their stated evidence limits.
Existing floating comparison tests accept NaNs regardless of payload; those
results do not prove equality of every output bit. The stricter next-port gate
must not inherit that exception silently.

The owner subsequently confirmed that NaN payload and sign bits must match
each target/compiler build too. A deterministic Rust NaN policy is not an
authorized substitute. The same pinned C++ vector source produced different
NaN bits when compiled with and without UBSan, so the compiler, flags and
translation-unit boundaries belong to the reference profile. A component
profile does not establish the behavior of a complete firmware build. Each
such build still needs its own recorded and tested reference.

The vector/two-pole increment includes `LowPassFilter2p` coefficient generation
and its supplied scalar/vector instantiations, plus the first-order vector
variants. It checks the tangent backend, the fixed cosine result used by this
formula, and scalar-versus-vector rounding in reset. Broader vector math still
needs a square-root implementation and other functions. The existing recurrence
and coefficient comparisons do not complete that broader math scope.

## First candidate result

The cached Rust `libm` 0.2.16 implementation, built with default features
disabled, fails the required equality check against this host's C math library.
It has not been added to the production workspace. A separately compiled Rust
probe and C++ probe produced this counterexample:

| Item | f32 bits |
| --- | --- |
| Sample frequency, 100 Hz | `0x42c80000` |
| Cutoff frequency, approximately 0.508 Hz | `0x3f020c4a` |
| Resulting `tanf` argument | `0x3c82bd0f` |
| Host C++/glibc `tanf` output | `0x3c82bfe7` |
| Rust `libm::tanf` output | `0x3c82bfe6` |

The argument is reachable through the two-pole filter's ordinary positive
frequency calculation, below its cutoff clamp. The difference is in a finite
result. The coordinator independently reran the unary counterexample and
observed those same output bits. No tolerance was applied.

The probe used Rust 1.95.0 / LLVM 22.1.2 and GCC 16.2.1 / glibc 2.44 on
x86_64 Linux. C++ was compiled with `-std=c++17 -O2 -ffp-contract=off` and
`-fno-builtin-tanf -fno-builtin-cosf -fno-builtin-sqrtf`; separate processes
exchanged raw input/output bits. This probe characterizes the math candidate. Full upstream-filter comparison
and target qualification remain separate work. The reference configuration for a future qualified port must also record the
floating-point control state required above.

The cached crate archive has SHA-256
`b6d2cec3eae94f9f509c767b45932f1ada8350c4bdb85af2fcab4a3c14807981`.
Its 155 archived source files matched the extracted cache byte for byte.
This establishes local cache consistency; upstream authenticity was not
independently established by that check. A future dependency choice still
needs license, provenance, feature and unsafe-boundary review.

The subsequent [tangent port](../crates/rustpilot-filters/TANGENT_REPORT.md)
translates the installed reference algorithm into Rust and retains its MIT
notice. Its exact comparisons and the dependent vector/two-pole work are
recorded in [the increment verification](EXACT_FILTER_VERIFICATION.md).
The rejected library remains rejected; its counterexample is preserved as a
regression. Other math routines and target/compiler profiles remain unfinished.
