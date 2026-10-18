# Scalar low-pass filter verification

This report covers the first-order low-pass port. Other filter families have
separate reports: [buffer and averages](BUFFERED_REPORT.md),
[mode and derivative filters](ORDERED_REPORT.md), and
[fixed-coefficient Butterworth filters](BUTTERWORTH_REPORT.md).

This crate translates the float specializations of ArduPilot's `DigitalLPF`,
`LowPassFilter`, and `LowPassFilterConstDt`, plus `calc_lowpass_alpha_dt`.
The Rust name for `DigitalLPF<float>` is `DigitalLpf`. Source revision:
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

Upstream sources are `libraries/Filter/LowPassFilter.h`,
`libraries/Filter/LowPassFilter.cpp`, and the alpha helper in
`libraries/AP_Math/AP_Math.cpp`. Epsilon predicates and the `M_2PI` definition
come from `AP_Math.h`, `ftype.h`, and `definitions.h`. Original algorithms and
license notices belong to the ArduPilot contributors. The translated code and
reference harness use GPL-3.0-or-later. Codex inspected source, translated these
algorithms, and wrote the tests. Human review remains outstanding.

## Defined behavior and changes

All Rust filter instances begin with output zero and a pending first sample.
The first apply accepts the sample exactly, even when alpha is zero or NaN.
`reset()` preserves the previous output until the next apply; `reset_to(value)`
sets the output and marks it initialized. Setters preserve filter history.
The default variable filter has cutoff zero; the default constant-step filter
has cutoff zero and alpha one.

These initial values are explicit Rust behavior. Upstream C++ constructors
initialize output but leave `initialised` unset; derived default constructors
also leave cutoff and alpha unset. Upstream callers may use zero-initialized
storage or explicitly configure/reset instances. The comparison harness resets
and configures automatic-storage C++ instances before use. It does not make
claims about reading uninitialized C++ state.

The alpha helper preserves upstream epsilon tests and branch order, including
negative inputs, tiny negative inputs, and the cutoff-zero case preceding the
dt-zero case. Rust preserves the double intermediate for RC and constant-step
reciprocal computation. Digital alpha is unbounded and retains the upstream
arithmetic order. Signed zero, infinity, NaN, and arithmetic overflow follow IEEE
operations. No finite-input saturation, validation, or NaN recovery was added.
NaN contamination can persist until reset. A reset with a finite value or a
pending reset followed by a finite sample permits recovery.

For dt/cutoff at or below minus float epsilon, Rust returns alpha one just as
the helper does, but omits the upstream `INTERNAL_ERROR` side effect. There is
no equivalent diagnostic facility in this crate. This is a documented behavior
difference. The result should not be interpreted as validated configuration.

## Commands and observed results

Run these commands from the workspace root with Rust 1.95.0:

```sh
cargo +1.95.0 test -p rustpilot-filters
cargo +1.95.0 clippy -p rustpilot-filters --all-targets -- -D warnings
cargo +1.95.0 build -p rustpilot-filters --release
ARDUPILOT_REFERENCE='/absolute/path/to/ardupilot-upstream' \
  cargo +1.95.0 test -p rustpilot-filters --test differential -- --ignored --nocapture
```

Observed locally: seven behavior tests passed, Clippy passed with warnings
denied, and the host release build passed. The optional C++ comparison test is
marked ignored so an ordinary Cargo test explicitly reports it as skipped.
Running it with the absolute reference path passed in debug and release: 5,006 scalar outputs
matched bit for bit. NaNs were compared as equivalent regardless of payload.
The comparison requires a clean tracked upstream checkout at the exact SHA
and a C++17 compiler (`CXX`, default `c++`). It also compares the working bytes
of `LowPassFilter.cpp`, `LowPassFilter.h`, and `AP_Math.cpp` with `git show`
objects at the pinned revision, independently of Git index flags. All harness
Git commands set `GIT_NO_REPLACE_OBJECTS=1`, so replacement refs cannot alter
the object selected by the pin. The verified
filter bytes are copied to the temporary build directory before compilation;
alpha extraction uses the verified AP_Math bytes. Relative reference paths resolve
from Cargo's test working directory, so an absolute path is recommended.

The seven behavioral tests cover alpha epsilon/negative/zero boundaries,
DigitalLpf initialization and unbounded alpha, variable-step reconfiguration
and resets, constant-step sequences and nonpositive sample rates, IEEE
nonfinite propagation, default initialization, and finite arithmetic overflow
with reset recovery. Before implementation, the first five tests failed at
intentional unimplemented stubs. A separate reference-integrity regression
fixture passed after first reproducing the missing integrity check: it verifies
that changes hidden by `skip-worktree` and `assume-unchanged` are rejected even
when Git status reports no tracked changes. A second regression fixture
installs a commit replacement ref and verifies that the original pinned source
bytes remain authoritative. This fixture reproduced the missing replacement
protection before the fix and now passes.

## Reference harness limits

The harness compiles the actual, unmodified pinned `LowPassFilter.cpp` and
includes its actual `LowPassFilter.h` from verified temporary copies. It extracts the complete alpha function
verbatim from pinned `AP_Math.cpp` into a separate compilation unit. It does
not compile all of AP_Math or build an ArduPilot vehicle.

`reference_math.h` substitutes the AP_Math include with declarations, matching
float epsilon predicates and PI constants, the no-copy macro, and placeholder
vector types needed by the explicit template instantiations. The internal-error
include is empty and its reporting macro is a no-op. These substitutions are
visible test fixtures; they are not Rust production dependencies. Vector
behavior is outside the tested scope. `HAL_DEBUG_BUILD` suppresses upstream's
local optimization pragma; the harness compiles at `-O2 -ffp-contract=off`
to compare ordinary, unfused floating-point operations.

The 5,006 outputs comprise a 14-by-14 alpha boundary matrix; 1,024 deterministic
three-filter sequence steps with resets and reconfiguration; 126 further
three-filter nonfinite/edge sequence outputs; 588 constant-filter outputs
across a 14-by-14 frequency/cutoff matrix; and 1,024 alpha outputs from random
float bit patterns. This is bounded evidence on the host compiler and target.
It does not establish every possible float input, vector equivalence, internal
error reporting equivalence, compiler-independent numerical identity, embedded
target behavior, vehicle integration, simulation, hardware, or flight safety.
