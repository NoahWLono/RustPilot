# Vector magnitudes implementation plan

**Goal:** Translate the magnitude and normalization operations of ArduPilot's
two- and three-component float and double vectors, with exact output-bit
comparisons for the recorded host compiler profile.

**Architecture:** Preserve the existing `Vector<N>` float representation and
filter behavior. Add a `Vector64<N>` double representation with explicit
`HAL_WITH_EKF_DOUBLE` selection for operations whose intermediates use `ftype`.
Implement square roots in safe Rust using integer significands and exact
rounding. Compile verified upstream methods in separate reference processes.

**Tech stack:** Rust 1.95.0, `no_std`, no Cargo dependencies, pinned GCC 16.2.0
container, pinned ArduPilot `afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

**Spec:** [Numerical compatibility](../../NUMERICAL_COMPATIBILITY.md) and the
pinned `AP_Math/vector{2,3}.{h,cpp}`, `AP_Math.h` and `ftype.h` sources.

## Constraints and review focus

- Compare every result bit, including NaN signs and payloads and signed zeros.
- Scope evidence to the recorded compiler, flags, call context and precision
  settings. Hardware builds alone do not qualify behavioral compatibility.
- Preserve division in normalization, source expression association, float
  intermediates in double-vector norms, and the source length-limit predicate.
- Exercise subnormals, overflowed squares, zero vectors, negative limits,
  distinct signaling/quiet NaNs, and double-to-float conversion boundaries.
- Keep all incomplete vector methods and all firmware targets explicitly open.

## Square-root backend

Files: `src/square_root.rs`, `tests/square_root.rs`, reference driver and
`SQUARE_ROOT_REPORT.md` under `crates/rustpilot-filters`.

- [x] Add failing tests for `sqrtf(f32) -> f32` and `sqrt(f64) -> f64`.
- [x] Implement exact integer-significand square roots, round to nearest/even,
  preserving the reference profile's signed zeros and NaN behavior.
- [x] Compare structured boundaries and independent random bit patterns to
  compiled C++ square roots in debug and release; document the rounding proof.

## Vector magnitude family

Files: `src/vector_math.rs`, `tests/vector_math.rs`, reference fixtures and
`VECTOR_MATH_REPORT.md` under `crates/rustpilot-filters`.

- [x] Add failing tests for scalar division/assignment, dot products,
  `length_squared`, `length`, `normalize`, `normalized`, 2D `limit_length`,
  3D `limit_length_xy` and `distance_squared`, and float/double conversion.
- [x] Implement float operations on `Vector<N>` and double operations on
  `Vector64<N>`. Use explicit const-generic precision selection on double
  norms, normalization and limiting rather than a process-wide setting.
- [x] Extract actual pinned C++ methods and math helpers verbatim, record
  compile settings, and compare every output bit for both dimensions and
  both storage precisions. Cover both EKF precision settings; record and
  characterize `AP_MATH_ALLOW_DOUBLE_FUNCTIONS` where used.
- [x] Keep epsilon equality and other unimplemented vector semantics outside
  the port claim; document the existing exact Rust `PartialEq` distinction.

## Integration and publication

- [x] Independently review algorithms, reference fidelity, and coverage.
- [x] Run workspace tests including references in debug and release inside
  the pinned container, formatting, Clippy, and both existing Cortex-M builds.
- [x] Update unit mappings, provenance, README and verification records using
  measured results. Copyedit the new prose.
- [ ] Commit and publish the verified increment, verify remote identity, and
  check the exact commit's public CI result. Keep the overall goal active.
