# Two-pole low-pass filter port

`src/biquad.rs` translates `DigitalBiquadFilter` and `LowPassFilter2p` from
ArduPilot `libraries/Filter/LowPassFilter2p.h` and `.cpp` at
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`. The original filter authors are
Leonard Hall and Daniel Frenzel, with subsequent work by ArduPilot contributors.
The translation retains GPL-3.0-or-later licensing. Codex implemented this port,
built the source comparison harness, investigated the numerical differences,
and recorded the intentional corrections below.

The Rust implementation covers coefficient generation, both reset operations,
configuration changes, frequency getters, and recurrence for `f32`, `i32`,
`i64`, `Vector<2>`, and `Vector<3>`. The five upstream aliases are available.
`LowPassFilter2pLong` represents the recorded LP64 host's 64-bit C++ `long`.
An ILP32 target's `long` corresponds to the `i32` implementation; the alias does
not establish compatibility with every target ABI.

`BiquadParams` exposes all seven upstream fields. `DigitalBiquadFilter` accepts
caller-owned parameters; `LowPassFilter2p` owns its parameters and exposes a
read-only accessor. Every `apply` and `reset_to` returns `Result`. Floating and
vector implementations always return `Ok`, including nonfinite results.
Integer implementations can return `BiquadError::IntegerConversion`.

## Arithmetic and state

The cutoff clamp preserves the mixed float/double comparison and its operand
ordering. A NaN cutoff with a finite sample rate therefore selects the clamp
operand, as the pinned C++ does. Frequencies below the upstream positive
threshold, `f32::EPSILON`, disable filtering. Disabled coefficient computation
updates both frequencies while retaining the five previous coefficients.
Disabled application returns the original sample bits without changing history
or initialization state.

Coefficient computation preserves the source expression order and double
`PI / fr` calculation before conversion to the tangent's float argument. The
Rust tangent implementation is described in `TANGENT_REPORT.md`. The only
cosine argument in this filter is `float(PI/4)`. Its result, `0x3f3504f3`, is
stored as a constant. Each C++ reference execution checks that both the native
runtime cosine call and the compiler's constant expression produce these bits.
This constant does not implement a general cosine function.

A pending reset initializes both delays from the next enabled sample and then
executes the recurrence. It does not simply return that sample. Explicit reset
multiplies in double precision for scalar samples. Vector reset converts the
double reciprocal to float before multiplying components, matching the actual
upstream vector operator's parameter type. The comparison corpus includes a
case where these paths differ by one output bit.

Integer recurrence uses float intermediates and truncates the new delay before
using it in the output expression. Reset uses a double intermediate. The range
checks follow truncation toward zero, including the valid fractional interval
just below `i32::MIN`. All potentially failing computations happen before
history or the initialization flag changes.

## Exact reference build

Exact NaN sign and payload behavior is qualified against GCC 16.2.0 on x86-64
Linux, with the pinned container recorded in the repository's verification
notes. The reference uses `-std=c++17 -O2 -ffp-contract=off`, disables builtin
`tanf`/`fmaf`, and uses no LTO or sanitizers for output-bit comparisons. The
algorithm and test driver are separate translation units. `HAL_DEBUG_BUILD`
prevents the upstream source from adding its own optimization pragma.

The low-pass reference compiles the actual pinned `.cpp`. The separate direct
biquad reference compiles verbatim extracted `DigitalBiquadFilter` definitions
with explicit instantiations. Headers and extracted operation bodies are
verified against pinned Git objects before compilation. The fixture uses the
actual `Vector2`/`Vector3` declarations and arithmetic definitions. Configuration
and attribute shims provide the surrounding build dependencies; vector
arithmetic is not replaced with a model.

The baseline links the recorded glibc tangent C algorithm, with its source hash
checked before compilation. It does not depend on the CI machine having the
same installed tangent implementation. The optional native-host comparison
requires `RUSTPILOT_TEST_HOST_GLIBC=1` and verifies `/usr/lib/libm.so.6` against
SHA-256 `965106704753eefe8c31ae7da6daba3ead3d18c52b19b5fcc41178bcc4aa999d`.
The C++ driver rejects a floating-point environment other than nearest-even
rounding with x86 FTZ and DAZ disabled.

The generated C++ uses different NaN operand preferences for direct biquad
calls and calls inlined into `LowPassFilter2p`. It also differs between
`Vector2`, `Vector3.x/y`, and `Vector3.z`. The Rust implementation preserves
these observed choices explicitly, using safe bit operations. The private
`apply_profile` context preserves the two public entry points' different
results without changing finite expression order. Pending and explicit vector
reset also preserve their separate NaN operand choices.

Those choices were established from generated assembly and exact comparisons.
They are specific to this compiler, target, build layout, and API context.
Other compiler versions, compiler flags, instrumentation, link-time
optimization, and target builds remain unqualified. These results are not a
portable claim about all possible C++ NaN results.

## Verification

The main release suite passed all ten tests with the pinned GCC compiler.
Independent review then added an eleventh regression test. Evidence includes:

- The full corpus contains 437,482 operation records and 3,606,236 exact scalar
  observations. It compares coefficient fields and results without a NaN
  exception or numerical tolerance. The optional native-host comparison passed
  the same corpus.
- A finite-input corpus contains 421,509 records and 3,462,999 exact scalar
  observations. Finite inputs can still produce infinities or NaNs; resulting
  output bits remain part of the comparison.
- The independent review corpus adds 150,000 direct-filter records and
  1,350,000 exact scalar observations. It generates each coefficient, sample,
  reset value and following sample independently, including 5,000 arbitrary-bit
  and 5,000 forced-NaN scenarios for each floating sample type. The retained
  regression passed in debug and release builds.
- A separately instrumented C++ build ran all 437,482 full-corpus records with
  undefined-behavior and float-cast-overflow sanitizers. Its successful exit
  checks that the reference fixture stays in the defined integer domain.
  Its floating output bits are not substituted for the uninstrumented profile.

The full corpus includes 270,676 sample-rate/cutoff pairs, all five low-pass
sample types, long streams, disable/re-enable transitions, reconfiguration,
pending and explicit resets, subnormals, signed zero, infinities, and positive
and negative signaling/quiet NaN encodings. Direct biquad cases independently
vary public coefficients and samples, including multiple different NaN payloads,
a zero reset denominator, and integer representation boundaries. Ordinary unit
tests separately exercise rejection and transactional state behavior.

These counts include repeated coefficient fields, state observations, and
related fixture cases. Exhaustive coverage of every possible input sequence
remains unproved. Debug verification and workspace checks are recorded in the repository's current verification log.

## Documented corrections and remaining limits

C++ automatic-storage constructors leave the initialization flag indeterminate.
The Rust constructor defines zero delays and a pending reset. The C++ comparison
uses static storage, where zero initialization defines the flag before the
constructor runs. A newly configured C++ filter with a disabled cutoff can also
leave coefficient fields indeterminate; Rust defines them as zero.

C++ floating-to-integer conversion is undefined when a truncated value cannot
fit. Rust returns `IntegerConversion` and retains all state instead of claiming
an exact reference match for that operation. Integer boundary tests remain
separate from the source comparison's defined-domain fixtures.

No board, complete vehicle, timing bound, interrupt behavior, or hardware
execution has been qualified by these tests. The no-std implementation uses no
FFI and no unsafe Rust; the C++ reference is a separate test process. Cross
compilation and any host successes do not establish embedded execution parity.
