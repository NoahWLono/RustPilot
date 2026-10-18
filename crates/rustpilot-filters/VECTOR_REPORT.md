# First-order vector filter translation

The `vector` module implements the `Vector2f` and `Vector3f` first-order
`DigitalLPF`, variable-step `LowPassFilter`, and cached-alpha
`LowPassFilterConstDt` behavior from ArduPilot revision
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`. Its Rust names are
`DigitalVectorLpf<N>`, `VectorLowPassFilter<N>`, and
`VectorLowPassFilterConstDt<N>`, with `N=2` or `N=3`.

The module also implements vector construction, addition, addition assignment,
subtraction, and scalar multiplication. `Vector<N>` stores a public
`components: [f32; N]` array in x/y/z order. Array storage makes no C++ ABI
promise. Other vector dimensions are Rust extensions without an upstream
instantiation comparison. Geometry, rotation, normalization, vector length,
and the remaining `AP_Math` vector API are unfinished.

Sources are `libraries/Filter/LowPassFilter.{h,cpp}`,
`libraries/AP_Math/vector{2,3}.{h,cpp}`, and the alpha calculation in
`libraries/AP_Math/AP_Math.cpp`. ArduPilot's vector sources credit Michael
Smith, Andrew Tridgell, and Bill Perone. Their work and the ArduPilot
contributors' filter implementations remain credited in the Rust source.

## State and arithmetic

Components keep separate histories. Configuration changes preserve history.
`reset()` preserves the stored output and accepts the next sample directly;
`reset_to(value)` copies every component bit and marks the filter initialized.
The default output is positive zero and the first sample is pending. These
explicit Rust initial values agree with the tested static-storage C++
instances. Upstream automatic-storage construction can leave initialization
and configuration fields unset; Rust gives them defined values.

The alpha branch order, epsilon thresholds, and finite expression order match
the existing scalar translation. Vector arithmetic uses Rust operations and
explicit NaN-bit selection. The Rust source has no explicit FFI calls, allocation,
unsafe blocks or new Cargo dependencies. It does not link the reference C++
algorithm. Compiler-provided arithmetic routines remain bootstrap dependencies.

## Exact reference profile

The accepted component comparison uses:

- GCC 16.2.0, x86-64, from the official `gcc:16.2.0-trixie` image with manifest
  digest `sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a`;
- `-std=c++17 -O2 -ffp-contract=off`, no sanitizers, no LTO, no fast math;
- `HAL_DEBUG_BUILD=1`, which selects the ordinary upstream vector definitions;
- separate driver and implementation translation units;
- round-to-nearest, ties-to-even, with gradual underflow and x86 MXCSR FTZ/DAZ
  disabled.

The fixture verifies each source byte against the pinned Git object before
copying it. It compiles the complete `LowPassFilter.h/.cpp` and the complete
vector headers. The required vector arithmetic definitions and alpha function
are extracted verbatim from their verified source files. The implementation
translation unit contains those definitions; the driver only sees declarations.
Configuration and diagnostic shims provide HAL macros, an annotation macro,
and an inert internal-error reporter. These shims leave the vector arithmetic in
the compiled upstream definitions. Rust does not emit the upstream
`INTERNAL_ERROR` diagnostic for invalid alpha inputs; that reporting integration
remains unfinished.
The actual `ftype.h` and `rotations.h` headers are also verified and compiled.

This is a component build profile. Complete ArduPilot firmware builds require
separate validation.
Other compilers, instrumentation, optimization settings, target ABIs, or FPU
modes require separate qualification. The comparisons establish output-bit
agreement on the recorded fixtures. They do not establish agreement for every
possible input sequence, floating-point status flags, execution time, or stack
usage. Cortex-M compilation does not qualify Cortex-M numerical behavior.

## Why NaN handling is explicit

Rust does not guarantee which operand's NaN payload an arithmetic expression
returns. The reference machine instructions select one operand, quiet its NaN,
and preserve its sign and payload. A non-NaN invalid operation produces
`0xffc00000` in this profile. The module reproduces these choices with bit
operations before evaluating arithmetic on the remaining values.

The compiled profile has several distinctions that source expression order
alone does not capture:

| Operation | NaN preference when both candidates are NaNs |
| --- | --- |
| Component subtraction | Sample/left operand |
| First-order difference multiplied by alpha | Difference |
| Digital filter accumulation | Scaled difference |
| Configured filter accumulation, Vector2 and Vector3 x/y | Scaled difference |
| Configured filter accumulation, Vector3 z | Previous output |
| Standalone vector addition and addition assignment | Left operand |
| Standalone scalar multiplication, Vector2 and Vector3 x/y | Vector component |
| Standalone scalar multiplication, Vector3 z | Scalar |

The variable and cached-alpha filters share the configured accumulation rule.
When the alpha calculation reaches a NaN result, a NaN time step takes
precedence over a NaN cutoff. The earlier alpha branches still take priority,
including disabled cutoff and negative-input cases.

The test `actual_pinned_cpp_nan_selection_changes_with_instrumentation`
demonstrates why instrumentation belongs to the profile. It compiles the same
verified source twice. Sanitizer flags are the only difference between the
two builds. Starting with
Vector2 output `[0, 0x7fc00000]`, it applies sample
`[0x7fc00000, 0xffffffff]` and alpha `0x7fc00000`. The uninstrumented y result
is `0xffffffff`; the UBSan build returns `0x7fc00000`. Both results are NaNs,
but the strict gate distinguishes their bits. The test asserts these exact
results; it does not classify them as equivalent.

## Verification

With Rust 1.95.0 and the profile above, all eleven vector tests pass in both debug
and release. Seven ordinary tests cover state transitions, independent
components, arithmetic, the specified NaN choices, and the Rust process's
rounding/underflow behavior. Four explicitly ignored tests require the
reference checkout and C++ compiler and are included in the reported runs.

The broad comparison checks **778,029 scalar bit patterns across 222,294
records**. It covers all three filters in both dimensions, direct vector
operations, signed zeros, subnormals, epsilon boundaries, cancellation,
extremes, infinities, positive and negative quiet/signaling NaN payloads,
reconfiguration, reset, explicit reset values, and deterministic generated
streams. No NaN payload or sign is excluded.

A separate comparison checks **845,747 scalar bit patterns across 241,642
records** of finite histories and direct operations. Its configuration stays
valid, and every compared result must remain finite. This prevents persistent
NaN history from obscuring the long-stream finite arithmetic checks. Each
filter/dimension receives 40,000 samples, with scheduled reconfiguration,
resets, explicit reset values, and distinct component inputs.

The counts include stored-output and cutoff observations and repeated states;
they are not counts of independent scenarios. An independent review supplied
an additional fresh-state generator, now kept as a regression test. It checks **1,890,000 scalar bit patterns across 540,000
records** with independently generated history, sample, alpha/time-step,
cutoff, and sample-rate operands. Each scenario starts from an explicit new
history. For each dimension, 20,000 scenarios draw unrestricted raw bits and
10,000 force independently generated NaNs, preserving arbitrary signs,
payloads, and quiet/signaling bits. This covers combinations that persistent
NaN histories might otherwise hide.

The fourth reference test is the instrumentation counterexample above. The
focused Clippy run also passes with warnings denied.

From the repository root, with `CXX` selecting the recorded compiler:

```sh
ARDUPILOT_REFERENCE=../ardupilot-upstream cargo test --locked -p rustpilot-filters --test vector -- --include-ignored --nocapture
ARDUPILOT_REFERENCE=../ardupilot-upstream cargo test --release --locked -p rustpilot-filters --test vector -- --include-ignored --nocapture
cargo clippy --locked -p rustpilot-filters --test vector -- -D warnings
```

These results qualify this bounded host component profile. Full vehicle,
embedded-target, compiler, and host-OS replacement work remains outstanding.
