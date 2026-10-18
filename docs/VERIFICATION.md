# Initial increment verification

Local verification on October 17, 2026 used Rust 1.95.0 on
`x86_64-unknown-linux-gnu`, LLVM 22.1.2, GCC 16.2.1 and Git 2.56.0.
Upstream input was ArduPilot
`afeb9c4941543d97f3e04fecf257b8f598f49a6a` with all 16 recursive submodules
checked out at their Git pins. There were no replacement refs in that source.

## Observed results

| Check | Result |
| --- | --- |
| Workspace tests with both upstream comparisons explicitly enabled | 29 passed, 0 failed, 0 ignored in debug |
| Same complete test suite in release | 29 passed, 0 failed, 0 ignored |
| CRC comparison | 5,936 matching outputs: 212 fixtures across 28 functions, in each profile |
| Scalar filter comparison | 5,006 matching outputs in each profile; NaN payloads treated as equivalent |
| Rust formatting | Passed |
| Clippy, all workspace targets, warnings denied | Passed |
| Host release build | Passed |
| Core library build for `thumbv6m-none-eabi` | Passed |
| Core library build for `thumbv7em-none-eabihf` | Passed |
| Deterministic recursive inventory | Repeated generations matched byte for byte |

The 29 tests comprise five checksum behavior tests, one checksum provenance
regression and its C++ comparison; seven filter behavior tests, two filter
provenance regressions and its C++ comparison; and twelve inventory fixtures.
Ordinary `cargo test` intentionally ignores the two C++ comparisons. Passing
that ordinary command alone does not establish differential results.

Normalized full outputs are saved in
[`provenance/verification-debug.log`](../provenance/verification-debug.log) and
[`provenance/verification-release.log`](../provenance/verification-release.log).
Paths are replaced with `$RUSTPILOT`; ordering and elapsed times vary by run.

## Reproduction

From the repository root, set `ARDUPILOT_REFERENCE` to the absolute path of the
pinned, clean reference checkout, then run:

```sh
env ARDUPILOT_REFERENCE=/absolute/path/to/ardupilot cargo test --workspace --locked -- --include-ignored --nocapture
env ARDUPILOT_REFERENCE=/absolute/path/to/ardupilot cargo test --workspace --release --locked -- --include-ignored --nocapture
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --release --locked
rustup target add --toolchain 1.95.0 thumbv6m-none-eabi thumbv7em-none-eabihf
cargo build --locked --release -p rustpilot-checksums -p rustpilot-filters --target thumbv6m-none-eabi
cargo build --locked --release -p rustpilot-checksums -p rustpilot-filters --target thumbv7em-none-eabihf
cargo run --locked -p rustpilot-migration -- inventory /absolute/path/to/ardupilot target/inventory
diff -ru crates/rustpilot-migration/baseline target/inventory
```

The CI workflow repeats these checks with the exact upstream pin. Remote CI
results belong to the specific GitHub Actions run; the table above records
local evidence only.

The first remote run exposed inherited Git filter configuration on the runner.
The inventory now isolates Git from system/global configuration while retaining
repository-local and explicit environment configuration checks. A subprocess
regression verifies that inherited filters do not run and local/explicit
executable filters remain rejected. No user Git configuration is modified.

## Limits

These checks establish bounded behavior on the observed inputs and successful
library compilation for two Cortex-M architectures. No Cortex-M binary was
executed. No complete firmware, vehicle simulation, physical board, timing,
stack-budget, hardware-in-the-loop, or flight verification was performed.
There are zero supported RustPilot vehicle targets.

The checksum harness compiles actual pinned `crc.cpp` with a disclosed empty
HAL header. The filter harness compiles actual pinned `LowPassFilter.cpp` and
the verbatim extracted alpha helper with disclosed supporting declarations and
placeholder vector types. It resets and configures C++ objects before use.
See the individual crate reports for scope, behavioral differences, and test
inputs. Tests do not establish universal equivalence or flight safety.

Compiler and host operating-system replacement remain unimplemented. The
verification environment still depends on the bootstrap tools listed in
[BOOTSTRAP.md](BOOTSTRAP.md).
