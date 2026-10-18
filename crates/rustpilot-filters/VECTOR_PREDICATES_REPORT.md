# Vector predicate, equality and indexing verification

`src/vector_predicates.rs` adds predicates, named upstream equality, zeroing,
checked indexing, and construction of a 3D vector from xy and z. For dimensions
two and three with float or double storage, the recorded tests match the
boolean results and copied floating bits of ArduPilot revision
`afeb9c4941543d97f3e04fecf257b8f598f49a6a` under the profile below. Indexing
observations have a separate limitation: the upstream implementation traverses
scalar struct members using pointer arithmetic, while Rust uses a safe array.

## Source contracts and Rust mapping

| Upstream entry point | Rust entry point | Behavior |
| --- | --- | --- |
| `is_nan`, `is_inf` | Same names on both vector types | True when any component is a NaN or infinity, respectively |
| Float `is_zero` specialization | `Vector::is_zero()` | Every absolute component must be strictly below `FLT_EPSILON` |
| Double `is_zero` specialization, double functions enabled | `Vector64::is_zero::<true>()` | Double absolute values compared against double-converted `FLT_EPSILON` |
| Double `is_zero` specialization, double functions disabled | `Vector64::is_zero::<false>()` | Narrow each component to float, then compare its absolute value against `FLT_EPSILON` |
| Float `operator ==` and `operator !=` | `equals(rhs)` and `not_equals(rhs)` | Component differences computed in float; absolute difference strictly below `FLT_EPSILON` |
| Double equality, double functions enabled | `equals::<true>(rhs)` and `not_equals::<true>(rhs)` | Double differences and `DBL_EPSILON` |
| Double equality, double functions disabled | `equals::<false>(rhs)` and `not_equals::<false>(rhs)` | Subtract in double, then narrow the difference to float and compare against `FLT_EPSILON` |
| `zero()` | Same name on both vector types | Set every component to positive zero |
| Const and mutable `operator[]` | `Index<usize>` and `IndexMut<usize>` | Borrow the array component at the requested index; reject out-of-range indices by panic |
| `Vector3(Vector2, z)` | `Vector::from_xy(xy, z)` and `Vector64::from_xy(xy, z)` | Copy all component bits without arithmetic |

The generic C++ `is_zero` method compares components with exact zero, but
the float and double specializations at the ends of `vector2.h` and
`vector3.h` call the epsilon helpers in `ftype.h`. These specializations are
the source for this implementation. `HAL_WITH_EKF_DOUBLE` does not affect the
tested entry points; both settings are compiled to verify that independence.

Rust's existing derived `PartialEq` remains unchanged. Named equality exposes
the upstream relation explicitly because it is nontransitive and matching
infinities compare false. NaNs also compare false; `not_equals` is the logical
complement of `equals`. Double equality with double functions disabled must
not narrow the operands before subtraction: `2^30` and `2^30 + 1` would then
collapse to the same float. A dedicated regression test covers this case.

Zeroing and xy construction preserve their source behavior for signed zero
and signaling NaNs: zeroing replaces all bits with positive zero; construction
copies signaling payload bits without quieting them. The differential suite
also verifies the existing default constructors for both dimensions and both
storage types against the upstream positive-zero defaults.

## Indexing is a documented memory-safety correction

The source index operators take a pointer to scalar member `x` and index it
to reach `y` or `z`. These members are not elements of an array. C++ treats a
nonarray object as a single-element array for pointer arithmetic, and a pointer
past that object does not designate a different object at the same address.
Consequently, cross-member indexing is outside portable defined C++ behavior.
See the C++ draft's [pointer arithmetic rules](https://eel.is/c++draft/expr.add#4.3)
and [object pointer rules](https://eel.is/c++draft/basic.compound#3).

The C++ driver asserts standard layout, total size, and the offsets of all
members before recording host index results. It tests const and mutable
index reads, then writes every valid component through the mutable reference
and emits every field after each write. These checks describe observed
GCC/x86-64 behavior; matching them does not make the C++ source portable or
defined. They are not an ABI guarantee for Rust's array representation.

Rust returns real references into `[f32; N]` or `[f64; N]`. Indexing panics
when `index >= N`; indices are `usize`, with no narrowing to upstream's
`uint8_t`. Unit tests exercise reads and writes at each dimension's first
invalid index, 255, 256, and `usize::MAX`, and verify failed writes leave all
components unchanged. No out-of-range C++ access is executed or claimed to
match Rust.

## Reference fidelity and measured profile

The harness verifies the upstream HEAD and checks every consumed file against
its pinned Git object before copying or extracting it. It copies actual
`vector2.h`, `vector3.h`, `ftype.h`, and `rotations.h`. It extracts the exact
out-of-line `is_nan`, `is_inf`, equality, and inequality bodies from the two
vector implementation files. The actual floating `is_equal` helper is
extracted from `AP_Math.cpp` and compiled in a third translation unit,
separate from both the vector methods and the driver. Its declaration is
extracted from `AP_Math.h`.

The parsing shims supply `WARN_IF_UNUSED`, the HAL configuration macros,
standard C++ includes, and standard `isnan`/`isinf` name imports. They replace
no tested numerical algorithm. `HAL_DEBUG_BUILD=1` selects the source's
out-of-line method context. Header-defined zero predicates, zeroing,
construction, and indexing retain their actual definitions and driver context.

The compiler is GCC 16.2.0 from image
`sha256:3f73339bf0d3d64081cac3f6d9fdafc6dd4cef2b9c49e0a09f91e97cb0eb139a`.
The local `/tmp/rustpilot-gcc16-cxx` wrapper compiled the references using
`-std=c++17 -O2 -ffp-contract=off`, without fast math, LTO, or sanitizers.
References executed on the host with glibc 2.44. Every run reported rounding
mode zero (`FE_TONEAREST`) and MXCSR 8064 (`0x1f80`); the driver rejects other
rounding modes or enabled FTZ/DAZ. Rust 1.95.0 debug and release builds both
passed. A full-container runtime profile is outside this initial host result.

The contract covers returned booleans and stored floating bits. It excludes
floating-point exception flags, timing, other compiler profiles, and target
hardware qualification. The production module uses safe `no_std` Rust and
adds no Cargo dependency, runtime FFI, allocation, or C++ ABI assumption.

## Corpus and results

Each record tests all five predicate/equality booleans, both index read
overloads, unchanged source components, every component write, zeroing, the
default constructor, and the xy constructor for 3D vectors. All floating
outputs are compared as integer bit patterns, including NaN signs, payloads,
quiet bits, and signed zero. The default and zero outputs are real fields;
there is no output padding included in these counts.

| Storage and dimension | Records per configuration |
| --- | ---: |
| Float, 2D | 9,334 |
| Float, 3D | 9,472 |
| Double, 2D | 10,242 |
| Double, 3D | 10,522 |
| Total | 39,570 |

The corpus includes zero signs, subnormals, normal limits, finite overflow
inputs, infinities, quiet and signaling NaNs, epsilon values and their
immediate neighbors with both signs, and the double neighbors around the
float-rounding midpoint below `FLT_EPSILON`. Every NaN fraction bit and sign
is tested independently in both operands and each component. Each storage
and dimension also gets 1,000 independent all-operand NaN cases, 2,000 bounded
finite cases including identical and adjacent values, and 5,000 independent
raw-bit cases. Ordinary tests separately establish epsilon nontransitivity,
unchanged Rust `PartialEq`, reference identity, and panic behavior.

All four combinations of `HAL_WITH_EKF_DOUBLE` and
`AP_MATH_ALLOW_DOUBLE_FUNCTIONS` passed in both debug and release. Per
configuration and Rust profile, 39,570 exact records produced 197,850 boolean
comparisons and 813,902 floating-bit comparisons, totaling 1,011,752 output
words. Across the eight runs, that is 316,560 records, 1,582,800 booleans,
6,511,216 floating bit patterns, and 8,094,016 total words, with zero
mismatches. Floating-input coverage remains finite.

The five ordinary tests were written before the production implementation.
The initial targeted `cargo test` exited 101 with 43 missing-method or
missing-index errors. After implementation it passed all five tests. The
completed differential harness then passed the actual upstream comparisons
above. Select the pinned reference/compiler environment using the
[container instructions](../../docs/EXACT_FILTER_VERIFICATION.md#evidence-and-reproduction),
then run these commands from the repository root:

```sh
cargo test -p rustpilot-filters --test vector_predicates
cargo test -p rustpilot-filters --test vector_predicates -- --ignored --nocapture
cargo test -p rustpilot-filters --release --test vector_predicates -- --include-ignored
cargo test --workspace
cargo clippy -p rustpilot-filters --test vector_predicates -- -D warnings
cargo clippy --workspace --all-targets -- -D warnings
rustfmt --edition 2021 --check crates/rustpilot-filters/src/vector_predicates.rs crates/rustpilot-filters/tests/vector_predicates.rs
```

The ordinary workspace suite, targeted and workspace Clippy checks, and
formatting check passed. The initial concurrent workspace Clippy check
reported 16 `assign_op_pattern` diagnostics in `tests/vector_ops.rs`.
That worker added a scoped allowance to preserve the intentional returning
operator coverage, and the subsequent workspace Clippy run passed.
