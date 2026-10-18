# Vector arithmetic and cross products

The `vector_ops` module adds the remaining float and double component
arithmetic, cross products, and right/front/up frame conversion from ArduPilot
revision `afeb9c4941543d97f3e04fecf257b8f598f49a6a`. The compared dimensions are
2 and 3. Generic implementations for other dimensions are Rust extensions
without an upstream comparison.

The Rust source credits Michael Smith, Andrew Tridgell, Bill Perone, and the
ArduPilot contributors. It uses safe `no_std` Rust with no new Cargo dependency,
allocation, runtime C++ linkage, or explicit FFI. Arrays store components in
x/y/z order; this representation makes no C++ ABI promise.

## API and arithmetic

| Storage | Added operations |
| --- | --- |
| `Vector<N>` (`f32`) | Unary `-`, vector `-=`, scalar `*=` |
| `Vector64<N>` (`f64`) | Unary `-`, vector `+`, `-`, `+=`, `-=`, scalar `*`, `*=` |
| Both 2D types | `%` returning the scalar cross product `x*v.y - y*v.x` |
| Both 3D types | `%` and `cross` returning the vector cross product; `scale`; component-wise vector `*=`; `rfu_to_frd` |

`scale` returns a scaled vector. Component-wise `*=` has a vector operand and
is distinct from vector `*`, which remains a dot product. `rfu_to_frd` returns
`[y, x, -z]`, also corresponding to ENU-to-NED conversion. Negation flips only
the sign bit, preserving every signaling/quiet NaN payload bit and both zero
signs. Frame conversion preserves all x/y bits and flips only the z sign.

All arithmetic uses the component storage type. Neither `HAL_WITH_EKF_DOUBLE`
nor `AP_MATH_ALLOW_DOUBLE_FUNCTIONS` changes these operations. The test also
compares the existing float returning arithmetic and addition assignment, and
both storage types' scalar division, division assignment, dot operator, and
named `dot` entry points. Those existing implementations were not changed.

## Source and reference profile

The fixture checks the upstream checkout's revision and each consumed source
file against its pinned Git object. It copies the complete verified
`vector2.h`, `vector3.h`, `ftype.h`, and `rotations.h`. Arithmetic operator
bodies are extracted verbatim from verified `vector2.cpp` and `vector3.cpp`
into a separate implementation translation unit, with explicit float and
double instantiations. The driver sees the actual declarations and header
methods, so `cross`, `scale`, component multiplication assignment, frame
conversion, and `dot` retain their real header contexts.

The small `AP_Math.h` replacement includes standard type/math headers, the
actual vector/ftype headers, and an unused `AP_Float` declaration. HAL shims
provide the selected configuration macros and board constants; the common
header shim defines the annotation macro. No arithmetic expression is
reimplemented or normalized in C++.

The recorded compile profile is GCC 16.2.0 from
`docker.io/library/gcc@sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a`,
x86-64, C++17, `-O2 -ffp-contract=off`, `HAL_DEBUG_BUILD=1`, without LTO,
sanitizers, or fast math. The agent's targeted runs compiled in that container
and executed on the host with glibc 2.44. The reference printed rounding mode
0 and MXCSR 8064, and rejected non-nearest rounding or enabled FTZ/DAZ. Rust
ordinary tests separately exercise nearest rounding and gradual underflow.
The integration run records its own execution libc separately.

The harness runs all four combinations of the EKF-double and double-functions
macros. Comparisons use every emitted output bit, including zero sign, NaN
sign, payload, and signaling/quiet bit. They do not use tolerance or NaN
classification as a substitute for equality.

## Observed NaN selection

Arithmetic quiets a selected input NaN while retaining its sign and payload.
Invalid operations without an input NaN produce the negative default NaN
(`0xffc00000` or `0xfff8000000000000`). Unary negation and frame conversion
perform no arithmetic quieting.

The native method contexts produce the following preferences when both
candidate operands are NaNs. These choices are fixed explicitly because Rust
does not specify the selected arithmetic NaN payload.

| Entry point | Preferred operand |
| --- | --- |
| Returning vector `+` and `-`, both storage types | Left component |
| Float `+=`; double 3D `+=` | Left component |
| Double 2D `+=` | Right component |
| Both storage types' `-=` | Left component |
| Returning scalar `*`, float 2D and float 3D x/y | Vector component |
| Returning scalar `*`, float 3D z | Scalar |
| Returning scalar `*`, double | Vector component |
| Scalar `*=`, float 2D | Scalar |
| Scalar `*=`, float 3D | Component for x/y; scalar for z |
| Scalar `*=`, double | Scalar |
| Component `*=`, float 3D | Right x/y; left z |
| Component `*=`, double 3D | Right component |
| `scale` | Same as returning scalar `*`, independently compared |

Cross products preserve the source multiplication/subtraction association.
Each multiplication prefers its left source operand, except the float 3D
z result's `x*v.y` product, which prefers `v.y`. Each subtraction prefers
its first product. The `%` and `cross` contexts are independently compared.
Disassembly of the compiled upstream implementation confirmed the double 2D
addition-assignment destination and the float 3D cross-product multiply
operands. Self-aliasing calls use the actual C++ forms `a += a`, `a -= a`,
`a *= a`, and `a % a`; Rust passes its copied right operand.

## Verification

The test was first compiled before the APIs existed and failed on the missing
operators and methods. An initial native run then exposed the assignment and
cross-product payload differences above. The corrected implementation passes
all six arithmetic tests in Rust debug and release, including the ignored
native reference test when explicitly enabled.

For each of four macro configurations, each Rust build mode compares:

- **732,760 operation records**;
- **1,716,752 actual floating-point observations**, including **335,872**
  controlled-exponent results individually asserted finite;
- **2,198,280 output words** including protocol padding, which is not counted
  as additional floating-point evidence.

The fixture includes all 441 ordered pairs from a 21-value special set,
exceptional values moved through each of seven independent operand positions,
all `3^7 = 2,187` combinations of finite/positive-signaling/negative-quiet
choices with position-specific NaN payloads, 1,024 independently generated
seven-NaN inputs, 2,048 finite controlled-exponent inputs, and 4,096 raw-bit
inputs per storage/dimension pair. Every float/double NaN fraction bit is also
placed in every operand position with both signs. Special values include
signed zeros, minimum and maximum subnormals, minimum normals, neighbors of
one and epsilon, extrema, infinities, and multiple NaN payloads. They exercise
invalid arithmetic, overflow cancellation, finite rounding, and underflow.

Five ordinary tests check finite APIs, bit-preserving negation/frame
conversion, distinct returning/assignment payload rules, cross-product
payloads and invalid results, and the Rust floating-point environment.
`cargo test -p rustpilot-filters` and
`cargo clippy -p rustpilot-filters --all-targets -- -D warnings` passed after
this implementation. The parent integration run performs complete recorded
container and target-build checks.

These measurements establish agreement on the recorded fixtures and compile
profile. They do not prove all possible inputs, exception flags, execution
cost, or other compiler/FPU profiles. They do not qualify firmware or hardware
behavior. Advanced geometry and rotation operations remain outside this
arithmetic increment.
