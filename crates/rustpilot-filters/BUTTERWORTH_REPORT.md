# Butterworth conversion evidence

`src/butterworth.rs` translates `libraries/Filter/Butter.h` from ArduPilot
revision `afeb9c4941543d97f3e04fecf257b8f598f49a6a`. The ArduPilot
contributors wrote the original recurrence and coefficient tables. RustPilot
retains their attribution and GPL-3.0-or-later licensing. The implementation and
tests were produced with AI assistance and checked against the pinned source.

The module implements `Butter2<C>` with a public `ButterCoefficients` trait,
all nine upstream coefficient sets, and all 26 upstream alias names. Every
coefficient literal retains the source decimal spelling and rounds to `f32`.
Each filter stores two `f32` history values, uses no allocator, and inherits the
crate's prohibition on unsafe Rust. It has no third-party Cargo dependencies.

## Behavior and initialization

The recurrence preserves upstream multiplication, addition and division order.
It does not use fused multiply-add. Floating overflow, signed zero, infinity
and NaN follow the floating-point operations; inputs and outputs are not
clamped. Custom coefficients remain possible through the trait, including
unstable or nonfinite sets and zero gain, as in the C++ template.

Construction initializes both history entries to positive zero. This matches
a value-initialized or static-storage C++ object. The upstream class leaves
history uninitialized when an object is default-initialized in automatic
storage; RustPilot does not reproduce that undefined state. The added `reset`
method restores zero history and can recover from nonfinite contamination.
Upstream has no reset method. The comparison driver therefore replaces its
objects with fresh, value-initialized upstream objects for each reset command.

Alias names describe a sample-rate/cutoff ratio with fixed coefficients. They
do not measure or enforce an application's sampling rate. Multiple aliases
select the same coefficient set.

## Tests

Five ordinary tests cover zero state for all coefficient sets, all alias
mappings, reset after NaN contamination, infinity propagation, and the two
history delays with custom coefficients. These tests were written before the
implementation; their initial run failed because the module did not yet exist.

The additional differential test compiles the actual unmodified `Butter.h`
with a driver that instantiates every source alias. Shared test support checks
its bytes against the pinned Git object and compiles a verified copy. This
header requires no dependency shims. The C++ compiler uses `-O2` and
`-ffp-contract=off`, with undefined-behavior and float-cast-overflow sanitizers
configured to stop on the first finding. The production Rust recurrence uses
ordinary operations.

In both Rust debug and release profiles, the differential test checked all
27 coefficient constants and 20,077 samples for each of 26 aliases, for
522,002 output comparisons per profile. It covers an impulse and decay,
constant input, a long deterministic stream at several amplitude scales,
alternating signs, subnormals, signed zeros, float extremes, infinities,
quiet/signaling NaN encodings, overflow, and 27 resets. Comparisons require
identical float bits except that two NaNs are accepted regardless of payload.
The test asserts the exact returned output count.

Run the six tests, including the explicitly opt-in reference comparison, from
the repository root:

```sh
ARDUPILOT_REFERENCE=/absolute/path/to/ardupilot-upstream \
  cargo test --locked -p rustpilot-filters --test butterworth -- --include-ignored --nocapture
ARDUPILOT_REFERENCE=/absolute/path/to/ardupilot-upstream \
  cargo test --release --locked -p rustpilot-filters --test butterworth -- --include-ignored --nocapture
```

The source aliases produce duplicate numerical results when they share
coefficients; the output count includes those aliases. These finite test
streams do not prove equivalence for every float history. Host comparisons
do not verify target-specific floating-point execution, timing, integration
with upstream callers, hardware operation or flight readiness.
