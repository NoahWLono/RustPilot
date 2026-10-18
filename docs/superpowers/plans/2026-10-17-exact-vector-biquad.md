# Exact vector and two-pole filter conversion

Complete the vector instantiations of `LowPassFilter` and the supplied
instantiations of `LowPassFilter2p` at the pinned ArduPilot revision. Apply the
owner's exact-bit requirement to all new comparisons, including signed zeros
and NaN encodings. Existing reports retain their disclosed NaN exception.

## Implementation boundaries

Keep the existing scalar first-order API intact. Add fixed-size vector values
and the component operations needed by these filters. The rest of AP_Math's
vector methods remain migration work. Compile real upstream vector operations
in comparison tests; shims may provide configuration and declarations only.

Implement the complete two-pole filter, including coefficient generation,
configuration, initialization and reset. Cover f32, i32, i64 and two- and
three-component f32 vectors. Preserve C++ expression types and rounding points,
especially the scalar versus vector reset multiplier. Specify checked failures
for undefined integer conversions and leave state intact on error. Explicit
initialization defines state the upstream constructors can leave unset.

The installed reference math package is glibc
`2.44+r50+g1848099f063e-1`. Inspect its source and runtime dispatch before
selecting the tangent implementation. The rejected libm candidate remains
rejected. Translate required routines into safe Rust, retain their actual
licenses, and compare exact bits. Document the floating-point environment and
the configurations for which comparisons were performed. A host backend must
not be presented as matching every embedded target's math library.

## Execution and acceptance

1. Extend immutable-source test support for AP_Math files. Independently
   implement vector filters, the required math routine, and complete biquads
   with tests preceding implementation.
2. Run strict comparisons of actual pinned upstream code. Include ordinary
   histories, extremes, NaNs, signed zero, reset and reconfiguration, coefficient
   boundaries and every supplied type. Use separate tests for deliberate fixes
   to undefined upstream behavior.
3. Review implementation and harness source against the pin. Resolve
   mismatches with concrete counterexamples; never relax equality or omit a
   failing input class to make a compatibility check pass.
4. Run debug and release workspace comparisons, formatting, lint, host release
   and the existing Cortex-M library compile checks. Record observed coverage
   and limitations rather than claiming universal numerical equivalence.
5. Update attribution, source mappings and public evidence. Publish only with
   explicit component status, verify remote bytes and CI, and retain the full
   vehicle, target, dependency, compiler and host scope as unfinished.
