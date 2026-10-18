# Vector projection, reflection and perpendicular validation

`vector_geometry.rs` adds `project`, `projected`, `reflect` and
`perpendicular` for two- and three-component float and double vectors.
The reference is ArduPilot revision
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

## Source and behavior

| Rust entry point | Pinned upstream implementation |
| --- | --- |
| `Vector<2>` and `Vector64<2>` projection and reflection | `libraries/AP_Math/vector2.cpp`, `project`, `projected`, `reflect` |
| `Vector<3>` and `Vector64<3>` projection and reflection | `libraries/AP_Math/vector3.h`, `project`, `projected`, `reflect` |
| Two-component `perpendicular` | `libraries/AP_Math/vector2.cpp`, `perpendicular` |
| Three-component `perpendicular` | `libraries/AP_Math/vector3.h`, `perpendicular`; `libraries/AP_Math/ftype.h`, `is_zero` |

Projection evaluates `direction * dot(position, direction)` before dividing
by `dot(direction, direction)`. Moving the division into a scalar ratio
changes finite result bits and can avoid an overflow that the source produces.
Both kinds of counterexample are regression fixtures. Reflection is twice the
projection minus the original vector, so it reflects about an axis. Projection
and reflection retain the source division behavior for zero directions.

The two-component perpendicular method chooses between `[-y, x]` and `[y, -x]`
with the strict `d1 > d2` comparison. Ties and unordered comparisons choose
`[y, -x]`. Its output is an unnormalized perpendicular to the direction;
returning its components preserves NaN payloads, including signaling bits,
apart from the sign bit changed by negation.

The three-component method returns `position` unchanged when the dot product
has magnitude below `FLT_EPSILON`. Otherwise it subtracts the parallel
component. `Vector64<3>::perpendicular::<AP_MATH_ALLOW_DOUBLE_FUNCTIONS>`
exposes the upstream configuration: `true` compares in double precision,
while `false` narrows the dot product to float for this comparison. Both use
`FLT_EPSILON`. The double immediately below this threshold exercises the
different outcomes. `HAL_WITH_EKF_DOUBLE` does not change these operations.

Rust accepts the direction by value. The C++ reference separately exercises
`a.project(a)` and `a.reflect(a)` with a true aliased reference; these match
the Rust copy argument for every generated case. The generic projection and
reflection methods reject dimensions other than two or three.

## Exact output profile

The C++ build uses GCC 16.2.0 from
`docker.io/library/gcc@sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a`,
with C++17, `-O2 -ffp-contract=off` and `HAL_DEBUG_BUILD=1`. Driver and
implementation are separate translation units, without LTO or sanitizers.
The local qualification compiled in that image and ran the executable on
the x86-64 host with glibc 2.44. The runtime banner records the compiler,
libc, rounding mode, MXCSR and both configuration macros. The driver rejects
rounding other than nearest and enabled FTZ or DAZ.

The fixture copies the actual pinned `vector2.h`, `vector3.h`, `ftype.h` and
`rotations.h`. It extracts the required operator and Vector2 method bodies
verbatim from the pinned `.cpp` files. Every source read is checked against
the pinned Git object before compilation. Header-defined Vector3 methods
therefore retain their source context; the Vector2 definitions can inline
their arithmetic operators within the implementation translation unit.
The test shim supplies standard includes, configuration macros, an unused
`AP_Float` declaration and `WARN_IF_UNUSED`; it replaces no geometry algorithm.

That difference in compilation context affects NaN payload selection:

| Context | Recorded operand preference |
| --- | --- |
| Vector2 `project` and `projected`, float and double | Direction first in each dot multiplication; the dot result first in each component scaling |
| Vector2 `reflect`, float and double | Original vector first in each dot multiplication; the dot result first in each component scaling |
| Vector3 projection and reflection | Separately compiled operator behavior: original vector first in the dot product; component first in scalar scaling except float z, which prefers the scalar |
| Vector3 perpendicular | The same external dot, scaling, division and subtraction behavior, with the header-defined squared length and epsilon check |

The implementation explicitly selects and quiets the observed arithmetic NaN
payloads. Invalid arithmetic without an input NaN returns the negative default
quiet NaN. Copies and unary sign changes retain all other bits. Finite values,
infinities, signed zeros, NaN signs and NaN payloads are compared as raw bits;
there is no normalization or tolerance in the oracle.

## Validation

The initial test build failed because the requested geometry methods were
missing. After implementation, six ordinary regression tests pass in debug
and release, and targeted Clippy passes with warnings denied.

The differential corpus contains 29,060 input pairs. Each pair exercises
six contexts: mutating projection, returning projection, reflection,
perpendicular, aliased projection and aliased reflection. This gives
174,360 vector results per macro configuration: 42,630 each for float and
double Vector2, and 44,550 each for float and double Vector3.
The test derives the component count from each case's actual dimension and
prints 437,820 output components per macro configuration, excluding padding.

The corpus includes:

- Signed zeros, subnormal boundaries, finite extremes, infinities, epsilon
  neighbors, and distinct positive and negative quiet and signaling NaNs.
- Distinct NaNs in every pair of effective input component positions, plus
  1,600 cases with fresh independent NaN payloads in every operand for each
  dimension and storage precision.
- Another 1,600 cases each of raw bits, bounded finite inputs, and mixtures
  of NaNs with the structured edge values for each dimension and precision.
- Zero and negative-zero directions, strict-comparison ties, unordered 2D
  decisions, and the 3D early return with signed-zero components.
- Finite rounding regressions and multiplication overflow cases that differ
  from computing the scalar ratio first.

All four `HAL_WITH_EKF_DOUBLE` / `AP_MATH_ALLOW_DOUBLE_FUNCTIONS`
combinations match exactly in both Rust debug and release: 697,440 vector
results per build, comprising 1,751,280 actual output components per build.
The comparison also checks the zero padding used by the wire format for 2D.

Run the ordinary suite with:

```sh
cargo test -p rustpilot-filters --test vector_geometry
cargo clippy -p rustpilot-filters --test vector_geometry -- -D warnings
```

With `ARDUPILOT_REFERENCE` pointing to the pinned checkout and `CXX` to the
recorded GCC compiler, run the exact reference tests with:

```sh
cargo test -p rustpilot-filters --test vector_geometry -- --include-ignored
cargo test -p rustpilot-filters --release --test vector_geometry -- --include-ignored
```

## Scope

This evidence qualifies the recorded x86-64 output profile and the tested
macro configurations. It does not establish all possible input combinations,
other C++ compiler or optimization profiles, floating-point exception flags,
an upstream C++ ABI, or embedded target numerical equivalence. The Rust
library remains safe `no_std` code with no dependencies or runtime FFI.
The source 2D perpendicular method uses `operator[]`, whose implementation
indexes from `&x` across separate struct members. This raises the same C++
object-model limitation documented in `VECTOR_PREDICATES_REPORT.md`: the
reference records observed host behavior, while Rust uses safe array access.
It does not prove that the upstream member-pointer indexing is defined C++.
Other geometry operations, rotations, firmware integration and hardware
qualification remain outside this increment.
