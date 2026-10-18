# Migration status

The full ArduPilot replacement remains unfinished. All targets, tools, and
dependencies are in scope. Nothing in this repository can currently operate a
vehicle. Implemented components include checksum functions, scalar and vector
first-order low-pass filters, two-pole low-pass filters and their tangent
backend, buffered/average/mode/derivative filters, fixed-coefficient Butterworth
filters, vector magnitudes/normalization, square roots, and migration inventory
tooling. Numerical comparisons qualify the
recorded component build profiles only.

## Baseline scope

Pinned ArduPilot: `afeb9c4941543d97f3e04fecf257b8f598f49a6a`.

| Source inventory item | Count |
| --- | ---: |
| Root tracked paths, including Git submodule pointers | 8,464 |
| Direct submodule pointers | 14 |
| Recursive submodule checkouts | 16 |
| Direct library directories | 156 |
| Main firmware `hwdef.dat` entries | 506 |
| Bootloader `hwdef-bl.dat` entries | 421 |
| Union of board directories with either definition | 508 |
| Names returned by upstream `./waf list_boards` | 509 |

The 506 main definitions are ChibiOS 459, Linux 25, ESP32 10, SITL 10, and
QURT 2. `MatekL431` and `TBS-L431` have bootloader-only definitions. These are
source counts. The complete matrix of buildable vehicle/board/feature
combinations still needs to be established. `AP_HAL_Empty`, dynamic Waf registrations, IOMCU firmware,
AP_Periph, replay, tests, examples and special programs require their own
coverage. BoardList's filtered autobuild lists are not a completeness baseline.

The upstream Waf listing was also executed separately and is saved in
[`provenance/waf-list-boards.txt`](../provenance/waf-list-boards.txt), sorted one
name per line. It includes `QURTBoard` in addition to the hardware-definition
names. This observation records upstream registration only. Builds and Rust
implementations remain outstanding. The Rust inventory tool does not execute Waf.

## Evidence levels

For each component distinguish inventory, Rust implementation, reference
comparison, integration, target build, hardware test, and release eligibility.
A component can pass one level and fail the next. Source counts do not measure
how much functional work remains; this project publishes no overall completion
percentage.

The generated inventory records every tracked root and available submodule
entry. Git object IDs identify exact inputs. A source row does not imply a
Rust implementation. The crate reports identify the implemented subsets:

- [Checksum port](../crates/rustpilot-checksums/REPORT.md).
- [Scalar filter port](../crates/rustpilot-filters/REPORT.md).
- [Buffer and average filters](../crates/rustpilot-filters/BUFFERED_REPORT.md).
- [Mode and derivative filters](../crates/rustpilot-filters/ORDERED_REPORT.md).
- [Butterworth filters](../crates/rustpilot-filters/BUTTERWORTH_REPORT.md).
- [First-order vector filters](../crates/rustpilot-filters/VECTOR_REPORT.md).
- [Two-pole filters](../crates/rustpilot-filters/BIQUAD_REPORT.md).
- [Tangent backend](../crates/rustpilot-filters/TANGENT_REPORT.md).
- [Vector magnitude operations](../crates/rustpilot-filters/VECTOR_MATH_REPORT.md).
- [Square-root backend](../crates/rustpilot-filters/SQUARE_ROOT_REPORT.md).
- [Inventory tool](../crates/rustpilot-migration/REPORT.md).

[`provenance/ports.tsv`](../provenance/ports.tsv) maps the 28 checksum functions
and implemented filter/math units to their upstream sources and test reports. It also
maps source aliases and coefficient sets; multiple rows can describe the same
implementation. Row counts are not component counts. The raw source inventory
starts every file at `not_ported`; these separate unit mappings record bounded
progress without marking entire libraries complete.

## Remaining work

| Area | Status and required work |
| --- | --- |
| Shared mathematics | CRC, selected float/double vector operations, tangent and square roots implemented for recorded host profiles; remaining vector, quaternion, matrix, geographic and other math pending |
| Filters | Scalar/vector first-order, two-pole, buffer/average/mode/derivative and fixed Butterworth components implemented for recorded comparison scopes; notch/harmonic, slew, remaining profiles and integration pending |
| Core infrastructure | Scheduler, time, parameters, storage, logging and fault reporting pending |
| Protocols | MAVLink, DroneCAN, DDS, device protocols and generators pending |
| Estimation and controls | AHRS, EKFs, attitude/position controllers and navigation pending |
| Vehicles | All vehicle programs, modes, arming and failsafe integration pending |
| HALs and drivers | ChibiOS, Linux, SITL, ESP32, QURT and Empty implementations pending |
| Auxiliary firmware | Bootloaders, AP_Periph and IOMCU pending |
| Simulation and tests | SITL models, replay, autotest and test generators pending |
| Tools | Waf replacement, setup, deployment, analysis, Python, shell, Lua and other tools pending |
| Dependencies | All 16 Git submodules and vendored runtime dependencies pending |
| Hardware qualification | No boards available; no board is qualified |
| Compiler and host OS | User-added scope: Rust replacements supporting a Linux-compatible environment; implementation pending |

## Scope gaps that must remain visible

Git inventory is not the complete dependency closure. External package installs,
system libraries, separately fetched SDKs (including ESP-IDF), toolchains,
firmware blobs, generated artifacts, and conditional build options need an
additional dependency audit. For example, a Rust replacement that still uses
ChibiOS or vendor C runtime code remains mixed-language. The user explicitly
included replacing compiler and host OS dependencies in the final goal, with
Linux compatibility sufficient to build and run RustPilot. This increment uses
existing tools to bootstrap the work. See [host scope](BOOTSTRAP.md).
The [interoperability assessment](INTEROPERABILITY.md) records representative
upstream platform dependencies and the evidence required to retire foreign
calls without hiding the services they still provide.

Hardware definitions also include other files and generate build configuration.
Their full include closure, per-vehicle combinations, flash/RAM/stack budgets,
interrupt timing, watchdog behavior and hardware I/O must be validated before
any corresponding support claim. No amount of host-only unit testing closes
those requirements.
