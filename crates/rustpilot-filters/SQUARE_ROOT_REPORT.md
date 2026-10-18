# Square-root implementation and reference evidence

`square_root::sqrtf` and `square_root::sqrt` return correctly rounded binary32
and binary64 square roots using safe integer arithmetic. They require no Cargo
dependencies, floating-point arithmetic, external math function, or runtime FFI.
Both APIs work in the crate's `no_std` build.

The returned bits match the recorded x86-64 C++ reference on the inputs below.
This includes NaN signs and payloads, signed zeros, and negative finite inputs.
The contract excludes C `errno` and floating-point exception flags. The integer
backend always computes round-to-nearest, ties-to-even; matching another C++
target, rounding mode, or flush-to-zero configuration requires separate evidence.
Compiling for an embedded target does not establish its numerical or timing
behavior.

## Algorithm and rounding argument

This is an original implementation derived from integer square identities.
No external square-root implementation was copied or translated. Its code and
test harness use the repository's GPL-3.0-or-later license.

Let `F` be the number of stored fraction bits: 23 for binary32 and 52 for
binary64. Every positive finite input can be written as

```text
x = m * 2^(e-F), where 2^F <= m < 2^(F+1).
```

For a normal input, `m` includes the implicit leading bit and `e` is the
unbiased exponent. For a subnormal, shifting the fraction left until its
highest bit is at position `F`, while decreasing `e` by the same shift,
preserves this equality. The smallest positive inputs give `e = -149` and
`e = -1074`, respectively.

If `e` is odd, double `m` and decrease `e` by one. The exponent is then even,
`2^F <= m < 2^(F+2)`, and

```text
sqrt(x) = sqrt(N) * 2^(e/2-F), where the integer N = m * 2^F.
```

The routine reads `N` in base four, from its most significant digit down.
After each digit it maintains `P = q^2 + r` and `0 <= r < 2q+1`, where `P`
is the prefix read so far. This makes `q = floor(sqrt(P))`. Appending a digit
`d` produces `P' = 4P+d`. Trying the new root `2q` gives remainder `4r+d`.
Increasing that root by one costs `(2q+1)^2 - (2q)^2 = 4q+1`. A comparison
and at most one subtraction selects the floor root of the longer prefix.
After `F+1` iterations, `q = floor(sqrt(N))` and `r = N-q^2`.

The nearest representable output is selected exactly:

```text
sqrt(N) > q + 1/2
    iff N > q^2 + q + 1/4
    iff r > q                         (r and q are integers).
```

A tie would require the integer `N` to equal `q^2+q+1/4`, which is impossible.
Thus adding one precisely when `r > q` implements round-to-nearest,
ties-to-even, with no floating-point approximation or double rounding.

The largest possible `N` is below `2^(2F+2)`, at most `2^106`, so it fits in
`u128`. The algorithm reads at most 53 base-four digits. The root is below
`2^53`; its trial divisor is below `2^54`; the remainder before a subtraction
is below `2^55`. All remaining arithmetic fits in `u64`. The largest right
shift is 104. Positive finite square roots are normal: their unbiased output
exponents are in `[-75,63]` for binary32 and `[-537,511]` for binary64.
Assembling exponent and significand therefore needs no subnormal-output
rounding. These arguments were reviewed independently, but are not a
machine-checked proof.

## Special values

NaNs are detected before the sign check. The implementation sets the quiet
bit and preserves every other input bit, including the sign. Positive
infinity and either signed zero are returned unchanged. Any other negative
input, including negative infinity, returns the recorded x86-64 negative
indefinite NaN: `0xffc00000` or `0xfff8000000000000`.

## Recorded reference profile

The C++ driver calls `::sqrtf` and `::sqrt` in separate, non-inlined wrappers,
with runtime inputs supplied as integer encodings. The test uses
`tests/common` to compile and execute a separate process. It checks the
ArduPilot checkout's pinned revision, but this scalar driver contains no
extracted upstream implementation: its reference is the C++ math operation
used by the vector methods. The vector differential tests separately compile
the pinned upstream methods in their actual expression context.

The measured profile on 2026-10-18 was:

| Component | Recorded value |
| --- | --- |
| ArduPilot pin | `afeb9c4941543d97f3e04fecf257b8f598f49a6a` |
| Compiler | GCC 16.2.0, container digest below |
| C++ flags | `-std=c++17 -O2 -ffp-contract=off` |
| Instrumentation | None |
| Runtime architecture | x86-64 Linux |
| Runtime C library | Host glibc 2.44 |
| Rounding | `FE_TONEAREST` |
| Initial MXCSR | `0x00001f80`, FTZ and DAZ clear |
| Rust | 1.95.0, LLVM 22.1.2, `x86_64-unknown-linux-gnu` |

Compiler container:

```text
docker.io/library/gcc@sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a
```

The executable was compiled in that container and run on the host, using
host glibc. The measured host `/usr/lib/libm.so.6` SHA-256 was:

```text
965106704753eefe8c31ae7da6daba3ead3d18c52b19b5fcc41178bcc4aa999d
```

The driver rejects unsupported architecture, non-IEEE binary formats,
rounding modes other than nearest, FTZ, or DAZ. It prints the actual compiler
and glibc versions in each test result. The tests permit other compiler and
library versions to run; a passing result records evidence for that run and
does not retroactively qualify all builds with similar version labels.

## Verification

The tests were written before the implementation. An initial input-returning
stub failed the ordinary finite and special-value tests. The compiled C++
differential test also failed on the smallest positive binary32 subnormal
before the integer implementation was added.

All five square-root tests passed in release mode. The ordinary tests and
the structured/random differential test also passed in debug mode.

The coordinator subsequently ran all five tests in both debug and release
entirely inside the pinned GCC container with glibc 2.41. Both runs passed,
including both exhaustive corpora. The complete workspace results and logs
are in [vector magnitude verification](../../docs/VECTOR_MATH_VERIFICATION.md).

| Corpus | Exact comparisons per run |
| --- | ---: |
| Structured and random binary32 | 1,138,842 |
| Structured and random binary64 | 1,192,718 |
| Every positive binary32 subnormal and every significand in `[1,4)` | 25,165,823 |
| Every binary32 NaN payload, with both signs | 16,777,214 |

The structured/random corpora each contain one million independently generated
raw bit patterns, all exponent fields with selected significands and both
signs, dense subnormal-boundary neighborhoods, quiet and signaling NaNs, and
16,384 output midpoints with five nearby input values each. Output midpoint
squares are located independently of the square-root implementation: exactly
in binary64 for binary32, and with integer squares for binary64. Every
comparison uses `to_bits()` equality; there is no NaN exception.

The two exhaustively compared normal binades exercise every binary32
significand at both exponent parities. Normal inputs in other binades share
the same integer radicands and rounding decisions; only the assembled output
exponent changes. The proof above covers that scaling. The complete binary32
domain and the complete binary64 domain have not been exhaustively compared.

The unignored workspace suite also passed after this implementation. The
recorded release square-root run completed its tests in 6.90 seconds on the
development host. Embedded performance has not been measured.

Reproduce the ordinary tests from the repository root:

```sh
cargo test --locked -p rustpilot-filters --test square_root
```

Set `ARDUPILOT_REFERENCE` to the pinned checkout and `CXX` to a wrapper that
invokes the compiler profile above. The wrapper must expose the test's
temporary directory to the compiler. Then run:

```sh
cargo test --locked -p rustpilot-filters --test square_root every_bit_matches_cpp_for_both_precisions -- --ignored --nocapture
cargo test --locked --release -p rustpilot-filters --test square_root -- --include-ignored --nocapture
```

Reference processes use files for input and output to avoid pipe deadlocks.
The exhaustive tests retain their text output in memory while comparing it;
allow several hundred megabytes for running them in parallel.
