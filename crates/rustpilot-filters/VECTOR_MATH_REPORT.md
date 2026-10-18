# Vector magnitude and normalization verification

The magnitude family in `src/vector_math.rs` matches the tested output bits
of ArduPilot revision `afeb9c4941543d97f3e04fecf257b8f598f49a6a` for
`Vector2<float>`, `Vector3<float>`, `Vector2<double>` and `Vector3<double>`.
The comparison uses GCC 16.2.0 on x86-64 with the build and call contexts
specified below. Every tested floating result is compared as an integer bit
pattern, including NaN sign, payload and quiet bit, and the sign of zero.

## Implemented operations

`Vector<N>` retains its existing float storage, filter APIs and component
operators. The new module adds scalar division and division assignment, dot
product and its vector multiplication alias, squared length, length,
normalization, normalized copies, 2D length limiting, 3D xy length limiting,
3D squared distance, and float/double conversions. `Vector64<N>` supplies the
same magnitude family with double storage; `Vector2d` and `Vector3d` are its
supported upstream aliases. Arithmetic methods reject dimensions other than
two and three. Construction and same-type copies preserve component bits.
Array layout has no C++ ABI guarantee.

Double-vector length, normalization and limiting take an explicit const
parameter named `HAL_WITH_EKF_DOUBLE`. For example,
`v.length::<false>()` selects the upstream float `ftype`, while
`v.length::<true>()` selects double `ftype`.

| Operation | Arithmetic preserved from upstream |
| --- | --- |
| Dot product, squared length, squared distance | Storage precision; three-term addition is left associated |
| Float length | Float squares and float square root; three-term addition is left associated |
| Double length, EKF double enabled | Double squares and square root; `sq` sums right recursively |
| Double length, EKF double disabled | Narrow each component to float before squaring; float squares, right-recursive sum and square root; widen the result to double |
| Normalize and normalized | Divide each stored component by length; never multiply by a reciprocal |
| 2D limit and 3D xy limit | Require `length > max_length` and `length >= FLT_EPSILON`; multiply components by `max_length / length` |
| 3D xy limit | Preserve every z bit, including an unquieted signaling NaN |

Negative limits retain the source behavior, including direction reversal.
Zero vectors are divided by zero during normalization. Vectors whose computed
lengths are below the positivity threshold remain unchanged by limiting.
`AP_MATH_ALLOW_DOUBLE_FUNCTIONS`
does not change any helper used by this family: the double `is_positive`
overload compares directly against double-converted `FLT_EPSILON` in both
configurations. Both configurations are still compiled and tested.

`Vector<N>` and `Vector64<N>` use ordinary Rust floating-point `PartialEq`.
This differs from ArduPilot's epsilon-based vector equality and is outside
this compatibility claim. Other vector geometry, rotations and remaining
operators are also outside this increment.

## Reference fidelity and profile

The test verifies the reference checkout HEAD and compares every consumed
source file byte-for-byte against its pinned Git object before copying or
extracting it. It copies the actual `vector2.h`, `vector3.h`, `ftype.h` and
`rotations.h`. Selected out-of-line method bodies are extracted verbatim
from `vector2.cpp` and `vector3.cpp`. The AP_Math shim extracts the actual
single-argument and variadic `sq`, both float `norm` overloads, the variadic
`norm`, and both `is_positive` overloads from the pinned `AP_Math.h`.

The remaining shim declarations provide `AP_Float` for a template type check,
`WARN_IF_UNUSED`, and the HAL configuration macros needed to parse the real
headers. They do not replace a tested math algorithm. `HAL_DEBUG_BUILD=1`
selects the actual out-of-line component operators. Explicit instantiations
of selected methods and the driver are compiled in separate translation
units. Methods defined inline in upstream headers retain those definitions
and their driver call context. The test exercises dot aliases, division and
division assignment, and both normalization entry points separately.

The recorded compiler is the GCC 16.2.0 image pinned by digest
`sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a`,
invoked through the local CXX wrapper. Flags are
`-std=c++17 -O2 -ffp-contract=off`; no sanitizers, fast math or LTO are enabled.
The initial host-executed references reported glibc 2.44, rounding mode zero
(`FE_TONEAREST`) and MXCSR 8064 (`0x1f80`). The driver checks round-to-nearest
and rejects enabled flush-to-zero or denormals-are-zero modes. It reports its
compiler, runtime library and precision macros with each run.

The square-root backend is safe Rust integer arithmetic; its rounding
argument and separate exact C++ comparisons are in `SQUARE_ROOT_REPORT.md`.
No runtime C++ FFI, unsafe code or Cargo dependency is added by this module.

## NaN and conversion behavior

Rust does not promise which NaN operand's payload an arithmetic expression
returns. The module therefore selects the observed reference operands
explicitly, quiets their payloads, and uses the negative default NaN for
invalid arithmetic without a NaN input. Division prefers a NaN numerator
before a NaN denominator. Dot products prefer the left operand within each
product, then earlier products in source addition order.

Length needs separate rules because GCC chooses different destination
registers in the recorded methods. The table lists priority among NaN
components, after any required float conversion.

| Length context | NaN component priority |
| --- | --- |
| Two components, either storage precision and EKF setting | x, then y |
| Three float components | z, then x, then y |
| Three double components, EKF double disabled | y, then z, then x |
| Three double components, EKF double enabled | x, then y, then z |

These choices do not change finite expression association. In particular,
float3 length cannot reuse dot-product NaN selection, and double3 length
cannot reuse left-associated squared length.

Cross-precision conversions preserve the sign and representable NaN payload,
set the destination quiet bit and discard unrepresentable low payload bits
when narrowing. Same-precision conversions copy all bits and do not quiet a
signaling NaN. NaNs are handled through integer bits rather than relying on
Rust's unspecified cast payload behavior. Finite casts use Rust's ordinary
IEEE conversions under the recorded round-to-nearest profile.

## Corpus and evidence

The test was written before the module; its initial targeted build failed
because `vector_math` was absent. The completed differential test covers every
operation for both dimensions, both storage precisions, and all four
combinations of `HAL_WITH_EKF_DOUBLE` and
`AP_MATH_ALLOW_DOUBLE_FUNCTIONS`.

Each dimension/storage pair receives 12,000 independent raw-bit input sets,
4,000 bounded finite input sets and 2,000 input sets with fresh independently
generated NaNs in all seven component/scalar positions. Additional fixtures
cover signed zero, subnormals, extrema, infinities, epsilon neighbors,
negative limits, distinct quiet/signaling NaNs in every pair of positions,
three-component NaN combinations and every NaN fraction bit with both signs.
Double-to-float fixtures include exact midpoint and adjacent double values
around normal, subnormal and overflow boundaries, with both signs.

Named regressions also check zero normalization, the limit threshold,
unchanged z bits, float versus double intermediate overflow and a finite
case whose result changes if the recursive double sum is left associated.
For that last case the components have bits
`[3ffbb08d0fcc4337, 3e500898dce86dbc, 3e5f80fcd84a82ce]`; upstream double length
has bits `3ffbb08d0fcc4338`.

Each macro configuration compares 906,683 operation records: 1,734,524 actual
floating results and 78,842 boolean limit flags. The four-word output format
contains 3,626,732 words including padding; padding is not counted as a
floating result. The four configurations are repeated in Rust debug and
release. On 2026-10-18, both targeted runs passed all five tests with zero
mismatches: the debug matrix took 44.77 seconds and release took 35.17 seconds.
Across both runs the matrix compared 13,876,192 floating results and 630,736
flags. This is a finite corpus, not an exhaustive proof over all possible
operand combinations.

A separate review used a separately authored actual-source C++ driver and
compared 145,536 special-input vectors per EKF setting, including all
`4^7` combinations of four distinct signed quiet/signaling NaNs across all
seven input positions, plus 80,216 finite vectors per setting. The reviewer
reported no output-bit mismatches. This independent scratch-corpus review is
additional evidence; it is separate from the committed test counts above.

The coordinator subsequently ran the complete workspace in debug and release
inside the pinned GCC container with glibc 2.41. All five vector magnitude
tests passed in both runs, with the same corpus counts and no mismatches.
See [the combined verification record](../../docs/VECTOR_MATH_VERIFICATION.md)
for logs, the complete test totals and the additional build checks.

Run the committed comparison with the pinned compiler and checkout:

```sh
ARDUPILOT_REFERENCE=/path/to/pinned/ardupilot CXX=/path/to/pinned-g++ \
  cargo test -p rustpilot-filters --test vector_math -- --include-ignored --nocapture
ARDUPILOT_REFERENCE=/path/to/pinned/ardupilot CXX=/path/to/pinned-g++ \
  cargo test -p rustpilot-filters --release --test vector_math -- --include-ignored --nocapture
```

Different compiler versions, optimization/inlining contexts, floating-point
modes and target architectures need their own comparisons. Cortex-M builds
establish compilability only. Floating-point exception flags, errno, other
rounding modes, timing and hardware behavior are outside this output-bit
claim. The larger RustPilot firmware port remains incomplete.
