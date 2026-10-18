# RustPilot first increment implementation plan

**Goal:** Publish a tested initial migration increment while retaining the full
ArduPilot replacement as the unfinished project goal.

**Architecture:** Separate `no_std` checksum and filter crates, with a Rust host
inventory tool and direct upstream differential test harnesses.

**Tech stack:** Rust 1.95.0, Cargo, Git; C++ only for upstream reference tests.

**Spec:** `docs/superpowers/specs/2026-10-17-rustpilot-design.md`

## Global constraints

- Pinned upstream `afeb9c4941543d97f3e04fecf257b8f598f49a6a`.
- All targets, tools, and dependencies remain in scope.
- No unsafe Rust or third-party Cargo dependencies in this increment.
- Preserve license and upstream attribution; disclose AI involvement.
- No firmware, flight, full compatibility, or completion claims.
- Implementers own disjoint crate directories; the coordinator owns workspace,
  provenance, documentation and CI. Each crate must have tests before logic.

## Review focus

Empty and boundary-length buffers; integer overflow and streaming seeds;
filter initialization, reset and invalid floating-point inputs; missing or
changed dependency revisions; inventory omissions and false support claims.

## Task 1: CRC library

Create `crates/rustpilot-checksums/{Cargo.toml,src/lib.rs,tests/*}`. Port every
function in upstream `libraries/AP_Math/crc.h` using slices and fixed arrays
where sizes are fixed. Preserve numerical semantics. Build known-vector tests,
streaming and boundary tests, then a differential test using actual upstream
`crc.cpp`. Any unsupported functions or deviations must be explicit.

- [x] Add failing behavioral tests, then implement and verify the full crate.
- [x] Compare to a compiled, revision-checked upstream reference.
- [x] Review implementation and document verified scope.

## Task 2: Scalar low-pass filters

Create `crates/rustpilot-filters/{Cargo.toml,src/lib.rs,tests/*}`. Port scalar
DigitalLPF, LowPassFilter and LowPassFilterConstDt behavior, including reset,
first sample, and `calc_lowpass_alpha_dt`. Explicitly define initialization.
Finite-input behavior should match upstream. Invalid inputs must be documented
and tested, never silently characterized as equivalent. Include pinned C++
differential evidence where feasible, with all harness substitutions disclosed.

- [x] Add failing sequence and reset tests, then implement and verify.
- [x] Compare representative sequences against upstream reference behavior.
- [x] Review implementation and document limits.

## Task 3: Inventory and completion reporting

Create `crates/rustpilot-migration/{Cargo.toml,src/*,tests/*}`. A dependency-free
Rust CLI runs Git read commands against the supplied reference checkout and
emits deterministic TSV files with source paths, revisions, blob IDs, sizes,
and component names, including recursive pinned submodules. Reject dirty or
mismatched source and report unresolved dependencies. Enumerate hardware
definition paths and Waf board class evidence without implying target support.
Use isolated tiny Git repositories for tests of recursion, drift and missing
submodules. Make no completion claims based on file names or source counts.

- [x] Implement fixture tests, inventory, summary, and validation behavior.
- [x] Generate committed baseline inventory from the reference checkout.
- [x] Verify determinism and scope; retain unresolved items explicitly.

## Task 4: Integration and publication

- [x] Create workspace metadata, pinned toolchain, license and provenance.
- [x] Run tests, release build, formatting and Clippy with warnings denied.
- [x] Configure CI to repeat tests and direct upstream comparisons.
- [x] Obtain independent code review, fix material findings, and retest.
- [ ] Publish README, migration status and exact validation record.
- [ ] Create the public repository, push, and verify remote HEAD and key files.

All tasks above are an initial increment. The complete migration remains open.
