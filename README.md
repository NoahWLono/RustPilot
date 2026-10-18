# RustPilot

**An independent, unfinished effort to rewrite all of ArduPilot in Rust.**

**Status: early migration. No working vehicle firmware. No supported flight
controller. Do not use this code to control a real vehicle.**

The end goal is everything ArduPilot provides: every vehicle, shared library,
driver, hardware backend, peripheral, bootloader, build tool, test tool, script,
and dependency. The owner also requested eventual replacement of compiler and
host OS dependencies with a Linux-compatible environment that builds and runs
RustPilot. All existing targets are in scope from the start. This
repository publishes incremental work toward that goal. It does not yet replace
ArduPilot.

## Credit to ArduPilot

RustPilot builds on the work of the [ArduPilot project](https://ardupilot.org/)
and [its contributors](https://github.com/ArduPilot/ardupilot/graphs/contributors).
ArduPilot's algorithms, protocols, hardware knowledge, test suites, and years of
maintenance are the reference for this migration. This project is independent;
it is not an official ArduPilot release and does not imply their endorsement.

The source baseline is
[`afeb9c4941543d97f3e04fecf257b8f598f49a6a`](https://github.com/ArduPilot/ardupilot/tree/afeb9c4941543d97f3e04fecf257b8f598f49a6a).
Derived code retains the applicable upstream notices. The initial Rust code is
distributed under GPL-3.0-or-later; see [LICENSE](LICENSE) and
[NOTICE](NOTICE). Dependencies retain their own licenses and must be reviewed
as they are migrated.

## Why pursue a full Rust migration?

Memory safety matters in software that reads sensors, processes untrusted
messages, and controls physical outputs. Rust's ownership and borrowing rules
prevent many use-after-free, invalid-reference, and data-race errors in safe
code. Checked indexing and explicit initialization remove other common sources
of memory corruption. `Result` and enums make failure handling and state
transitions easier to express and review.

The reason to pursue the entire codebase is coverage: leaving parsers, drivers,
or support libraries in C/C++ leaves their memory-safety risks outside Rust's
checks. A Rust wrapper around a C++ flight stack would not achieve that goal.
Portable libraries can use `no_std`, fixed-size storage, and explicit ownership
without a garbage collector. The initial core crates forbid unsafe code and
have no third-party Cargo dependencies.

These benefits motivate the migration. Its safety still needs to be established
through verification. Rust does not prove control-law correctness, sensor
quality, deadlines, stack bounds, or vehicle stability. It does not inherently
use less RAM. Bounds failures and panics can still be operational failures.
Hardware access and some dependencies require separately reviewed unsafe
boundaries. Changes in numerical behavior can introduce new faults even when
memory access is safe. See the Rust project's guidance on
[unsafe code and safe abstractions](https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html)
and [embedded development](https://doc.rust-lang.org/embedded-book/).

The migration has not yet resolved FFI and hardware integration for a complete
target. Keeping a C RTOS or vendor SDK would retain a non-Rust dependency;
replacing it requires additional implementation and target verification.
Direct hardware access may need reviewed unsafe Rust without calling C.
See [FFI and hardware boundaries](docs/INTEROPERABILITY.md) for the current
evidence, outstanding platform dependencies, and rules for future boundaries.

## What exists now

| Area | Implemented components | Remaining scope |
| --- | --- | --- |
| Checksums | Rust port of the `AP_Math/crc` function collection | Integration into Rust protocols and drivers |
| Mathematics | Selected float/double vector operations, tangent and square-root backends | Remaining vector operations, other math functions and target profiles |
| Filters | Scalar and vector first-order low-pass, two-pole low-pass, buffers, averages, mode, derivative and fixed-coefficient Butterworth filters | Remaining filter families, other build profiles and vehicle integration |
| Migration tools | Rust inventory of pinned Git trees and target evidence | Full dependency closure and target qualification |
| Vehicle firmware | Not implemented | Copter, Plane, Rover, Sub, Tracker, Blimp and variants |
| Hardware support | Definitions inventoried; no supported boards | HALs, drivers, peripherals, bootloaders and timing tests |
| Other tools and dependencies | Recorded where present in Git | Rust replacements and compatibility verification |

Detailed scope and test results live in [migration status](docs/MIGRATION.md)
and [verification evidence](docs/VERIFICATION.md). A unit test, inventory row,
or cross-compiled library never counts as a supported aircraft or board.
The buffered-filter increment adds explicit reset fixes and checked average
arithmetic. Its [verification record](docs/BUFFERED_VERIFICATION.md) separates
reference matches from intentional behavior changes.

The [vector and two-pole increment](docs/EXACT_FILTER_VERIFICATION.md) adds a
native Rust tangent implementation and exact output-bit comparisons for a
recorded GCC/x86-64 component build, including NaN payloads and signed zeros.
The compiler and build settings matter: even adding sanitizer instrumentation
can change which NaN payload the same C++ source returns. Other target/compiler
builds still need their own comparisons. Earlier filter reports retain their
disclosed NaN-payload exception.

The [vector magnitude increment](docs/VECTOR_MATH_VERIFICATION.md) extends this
work to float and double vector lengths, normalization, dot products, division,
limiting and precision conversion. Its square roots use integer arithmetic in
Rust. Double-vector length calculations expose ArduPilot's EKF precision choice
explicitly, since storing double components does not always mean upstream uses
double intermediates.

## How this project is being done

The repository owner requested the full rewrite and authorized public,
incremental development. OpenAI Codex and delegated coding agents inspected the
pinned ArduPilot source, wrote Rust translations, created tests, and reviewed
the initial work. This is AI-assisted engineering. Automated checks and review
by another AI agent are not independent human certification.

Each port records its upstream source, implemented behavior, intentional
changes, and remaining limits. The owner authorized documented fixes during
conversion. The reference C++ is compiled only for comparison tests; it is not
linked into the Rust libraries. Test harness substitutions are documented in
the crate reports. Existing target definitions and dependencies stay in the
migration inventory even while their Rust implementations are missing.

There is no hardware available to the project yet. Hardware timing, I/O,
watchdogs, failsafes and flight behavior have not been qualified. Public commits
record research and development progress and are unsuitable for vehicle operation.

## Build and test

Install the Rust toolchain specified in `rust-toolchain.toml`. Builds and pure
behavior tests need Rust. The ordinary workspace test suite also needs Git;
differential tests additionally need a C++ compiler and the pinned ArduPilot
source.
The strict new filter comparisons use the GCC 16.2.0 build profile recorded in
[their verification instructions](docs/EXACT_FILTER_VERIFICATION.md). CI pins
that compiler's container image by digest; an arbitrary C++ compiler is not
an interchangeable reference for NaN bits.

```sh
cargo test --workspace --locked
cargo build --workspace --release --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
```

To fetch the comparison source, run these commands from the RustPilot root:

```sh
git clone https://github.com/ArduPilot/ardupilot.git reference/ardupilot
git -C reference/ardupilot checkout afeb9c4941543d97f3e04fecf257b8f598f49a6a
git -C reference/ardupilot submodule update --init --recursive --depth 1
env ARDUPILOT_REFERENCE="$PWD/reference/ardupilot" cargo test --workspace --locked -- --include-ignored
cargo run --locked -p rustpilot-migration -- inventory reference/ardupilot target/inventory
```

The comparison tests are explicitly ignored in an ordinary test run. The
`--include-ignored` invocation requires `ARDUPILOT_REFERENCE`; consult
the verification record before interpreting a green local test run. The
inventory command reads the reference Git repositories and writes metadata. It
does not convert code or establish board compatibility.

## Completing the migration

The work ahead includes the full math and filter stack, parameter persistence,
protocols, scheduler, estimators, controls, vehicle modes, HALs, drivers, build
system, simulation and test tooling, and all remaining dependencies. Preserve
parameter identities, protocol formats, failure behavior, configuration
combinations, and resource budgets. Intentional changes need specific tests
and compatibility notes.

The compiler/OS expansion is a separate unfinished workstream described in
[bootstrap and host scope](docs/BOOTSTRAP.md). Current builds still use Linux,
rustc's LLVM backend, native linking tools and Git. The comparison tests use a
C++ compiler. These disclosed bootstrap dependencies still need Rust replacements
to meet the expanded goal.

Completion requires implementation coverage, differential and integration
tests, target-specific builds, and appropriate hardware evidence for the full
scope. The project will not declare completion based on Rust line counts,
generated placeholders, wrappers, or compiling a small subset.
