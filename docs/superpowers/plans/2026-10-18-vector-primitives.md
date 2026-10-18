# Vector arithmetic, predicates and projections implementation plan

**Goal:** Complete the remaining float/double component arithmetic and add
cross products, vector predicates, checked indexing, explicit upstream
equality, projection, reflection and perpendicular-vector operations.

**Architecture:** Extend the existing `Vector<N>` and `Vector64<N>` in three
separate modules. Preserve the previously qualified operations and exact Rust
`PartialEq`; expose upstream epsilon equality through named methods, with
explicit double-function configuration where it changes behavior. Compile
verified upstream functions in their original header or out-of-line contexts.

**Spec:** Pinned ArduPilot `afeb9c4941543d97f3e04fecf257b8f598f49a6a`,
`AP_Math/vector{2,3}.{h,cpp}`, `AP_Math.{h,cpp}` and `ftype.h`, under
[the numerical compatibility requirements](../../NUMERICAL_COMPATIBILITY.md).

**Tools:** Rust 1.95.0, safe `no_std`, no Cargo dependencies, recorded GCC 16.2.0
container, C++17, `-O2 -ffp-contract=off`, `HAL_DEBUG_BUILD=1`, no LTO or
sanitizers for output-bit qualification, nearest rounding and gradual underflow.

## Arithmetic and cross products

Owned files: `src/vector_ops.rs`, `tests/vector_ops.rs`, its C++ reference
driver and `VECTOR_OPS_REPORT.md` in `crates/rustpilot-filters`.

- [x] Write failing tests for missing f32/f64 operators, scalar and component
  assignment, 3D scale, 2D/3D cross products and 3D coordinate reordering.
- [x] Implement `Neg`, missing `Add`/`Sub`/assignment forms, scalar `Mul` and
  `MulAssign`, 3D component `MulAssign`, `Rem` for cross products, the 3D
  `cross`/`scale` methods and `rfu_to_frd`. Preserve signaling bits on negation.
- [x] Compare returning and assignment entry points independently, including
  self-aliasing, finite cancellation, overflow and independent NaN payloads.

## Predicates, indexing and upstream equality

Owned files: `src/vector_predicates.rs`, `tests/vector_predicates.rs`, its
reference driver and `VECTOR_PREDICATES_REPORT.md`.

- [x] Write failing tests for `is_nan`, `is_inf`, `is_zero`, `zero`, indexing,
  `equals`/`not_equals` and 3D construction from xy plus z.
- [x] Implement `Index<usize>` and `IndexMut<usize>` over the Rust component
  arrays. Document bounds failures and the upstream struct-member pointer
  arithmetic separately from observed host component access.
- [x] Implement float predicates/equality and double variants with explicit
  `AP_MATH_ALLOW_DOUBLE_FUNCTIONS` where needed. Preserve double subtraction
  before narrowing in equality when double functions are disabled.
- [x] Compile the real epsilon helper from `AP_Math.cpp`, test threshold
  neighbors, nontransitive equality, infinities, signaling NaNs, every valid
  index, mutations, positive-zero clearing and out-of-range Rust behavior.

## Projection, reflection and perpendicular vectors

Owned files: `src/vector_geometry.rs`, `tests/vector_geometry.rs`, its
reference driver and `VECTOR_GEOMETRY_REPORT.md`.

- [x] Write failing tests for `project`, `projected`, `reflect` and static
  `perpendicular`, for float/double storage and dimensions two/three.
- [x] Preserve multiply-before-divide order, reflection about an axis,
  source behavior on zero directions, unordered/tied 2D choices, and the 3D
  epsilon early return. Double 3D perpendicular selects the explicit
  `AP_MATH_ALLOW_DOUBLE_FUNCTIONS` setting.
- [x] Compare actual method contexts independently; composing existing
  operator NaN policies is not sufficient evidence. Add fresh all-operand
  NaNs, self-aliasing cases, finite arithmetic-order regressions and thresholds.

## Integration and publication

- [x] Independently review each implementation and the final combined diff.
- [x] Run complete debug/release reference suites in the pinned container,
  formatting, Clippy, release and both Cortex-M library builds, and inventory.
- [x] Update mappings, exact profile and measured evidence; copyedit reports.
- [ ] Publish the verified increment, check remote identity and public CI.
  Keep the overall migration goal active; advanced geometry, rotations,
  remaining math, firmware and all hardware qualification remain unfinished.
