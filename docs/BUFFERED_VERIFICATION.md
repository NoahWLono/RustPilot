# Buffered-filter increment verification

The complete workspace suite passed locally on October 17, 2026 with the
buffered-filter increment: 63 tests in debug and 63 in release, with no failures
or ignored tests. The environment used Rust 1.95.0, host
`x86_64-unknown-linux-gnu`, and the pinned ArduPilot reference
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

## Implemented scope

The filter crate now includes the apply/reset trait, circular buffer, ordinary
and integral averages, mode filter, all four supplied scalar derivative sizes,
and fixed-coefficient Butterworth family. The previous first-order scalar
low-pass implementation remains available. All use safe Rust, fixed-size
storage and no third-party Cargo dependencies.

The reports define the exact supported generic types and test configurations:
[buffered](../crates/rustpilot-filters/BUFFERED_REPORT.md),
[ordered](../crates/rustpilot-filters/ORDERED_REPORT.md), and
[Butterworth](../crates/rustpilot-filters/BUTTERWORTH_REPORT.md).
Source aliases map to equivalent generic types except Butterworth, which also
retains the original alias spellings. This is not unrestricted equivalence for
arbitrary C++ template arguments or downstream custom arithmetic traits.

## Observed checks

| Check | Local result |
| --- | --- |
| Complete workspace tests, reference tests explicitly enabled | 63 passed in each profile; 0 failed, 0 ignored |
| Buffer/average main comparison | 71 cases, 13,041 records, 91,582 scalar observations in each profile |
| Mode main comparison | 140 configurations, 1,667,340 serialized values in each profile |
| Derivative main comparison | Sizes 5, 7, 9 and 11; 70,136 serialized values in each profile |
| Butterworth comparison | 27 coefficient constants and 522,002 outputs across 26 aliases in each profile |
| Earlier checksum comparison | 5,936 outputs in each profile |
| Earlier scalar low-pass comparison | 5,006 outputs in each profile |
| Formatting and Clippy, all workspace targets with warnings denied | Passed |
| Host workspace release build | Passed |
| Core library builds for `thumbv6m-none-eabi` and `thumbv7em-none-eabihf` | Passed |
| Recursive inventory regenerated from all 17 repositories | Matches the committed baseline byte for byte |
| Reference tracked worktree state | Clean after verification |

The 63 tests consist of the previous 29 plus 12 buffered, 16 ordered and six
Butterworth tests. Seven reference tests are explicitly ignored by an ordinary
Cargo run; use `--include-ignored` for the results above. The new reference
drivers compile actual verified upstream files with undefined-behavior and
float-cast-overflow sanitizers, configured to stop on a finding. They disable
floating-point contraction. The existing low-pass and checksum harnesses retain
their previously documented compilation settings.

Floating comparisons require equal bits except NaN payloads. Observation counts
include physical-slot reads, cached values and aliases sharing coefficients.
They do not count distinct algorithms, independent scenarios or exhaustive
coverage. The C++ algorithms are unchanged; the ordered driver substitutes only
an AP_Math classification header and an unused Filter umbrella. Other new
reference drivers need no dependency shims.

## Intentional corrections

Integral reset now clears its running sum. The original method produces 308
after samples 100 and 200, reset, then sample 8; the Rust filter produces 8.
Mode reset clears the previous output and restores its initial eviction phase.
Derivative reset clears timestamps, pending state and cached slope. Separate
tests demonstrate these differences using the original C++ reset methods.

Rust initializes all state. Buffer and mode capacities 128 through 255 use safe
indexing instead of the source's invalid signed index range. Out-of-range sample
access returns `None`. Checked average updates report signed accumulator or
conversion errors and leave state unchanged. Call-site conversion must handle
those results; no vehicle failsafe policy has yet been implemented.

## Reproduce and inspect

Run the commands in [initial verification](VERIFICATION.md#reproduction) against
the current workspace. They now include all seven reference tests. The new
reference tests require a C++17 compiler with the sanitizer flags described in
the crate reports. The shared input verifier is exercised by the existing
hidden-worktree-change and Git replacement-object regression fixtures.

Saved full outputs:

- [Debug](../provenance/verification-buffered-debug.log).
- [Release](../provenance/verification-buffered-release.log).

Absolute project paths are normalized to `$RUSTPILOT`. Test ordering and elapsed
times vary. Expected panic messages come from invalid-capacity rejection tests;
their result lines confirm those tests passed. The CI workflow reruns the
workspace tests, comparisons, inventory and cross-compilation checks. Remote
results must be checked against the particular published commit.

## Remaining evidence

A separate Codex reviewer inspected source, reference drivers and claims, and
independently passed the 34 new tests. Human review and certification remain
outstanding.
See the [review record](REVIEW.md).

No Cortex-M executable was run. There is still no complete RustPilot vehicle
firmware or supported board. Target timing, stack/flash/RAM budgets, integration,
simulation and hardware operation remain unverified. The rest of the Filter
tree, all remaining ArduPilot components, dependencies, compiler and requested
host environment remain in the full migration scope.
