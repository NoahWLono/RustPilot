# RustPilot migration scope

The requested final result is a Rust replacement for everything provided by
ArduPilot: vehicles, libraries, hardware backends, peripherals, bootloaders,
build and test tools, scripts, and dependencies. All existing targets are in
scope from the start. Incremental publication is authorized; partial work must
remain visibly incomplete and must never be described as flight-ready.

Upstream baseline: `afeb9c4941543d97f3e04fecf257b8f598f49a6a` from
<https://github.com/ArduPilot/ardupilot>. The upstream checkout remains a separate
reference. RustPilot is a new repository owned by NoahWLono and is independent
of the ArduPilot project. Upstream authors retain credit and license notices.

## Engineering decisions

Use a Rust workspace with dependency-free `no_std` core libraries and a host
tool using `std`. Forbid unsafe code in the first libraries. Any future unsafe
hardware boundary needs a documented invariant and separate review. Keep
upstream algorithms and numerical ordering where possible; label changes in
input handling and initialization explicitly. The user authorized documented
fixes during conversion and currently has no hardware available. Test against actual pinned C++
where feasible. Pure Rust unit tests are insufficient evidence of equivalence.

Maintain a deterministic source inventory containing Git object IDs, sizes,
component classifications, and submodule revisions. Recursively inventory
available pinned submodules; report missing submodules as unresolved. Record
hardware definitions and build registration evidence without claiming those
entries are supported firmware. Explicit mappings identify the small portions
actually implemented and tested. No blanket completion percentages.

The first increment ports the CRC collection and scalar low-pass filters,
alongside the inventory tool, tests, provenance, and public documentation.
This increment does not satisfy the full migration request. Flight stacks,
drivers, bootloaders, and remaining tools/dependencies stay outstanding.

## Completion requirements

Every upstream functional unit must have a Rust implementation or an explicit,
reviewed disposition for non-executable data and documentation. Existing
behavior, configuration formats, protocols, parameter identities, failure
handling, and timing constraints must be accounted for. All target/vehicle and
feature combinations require build coverage and appropriate simulation and
hardware evidence. Inventory counts are not validation. Cross-compiling a
library is not a board port. No complete migration claim until these gates pass.

The user explicitly added compiler and host OS dependencies to the goal, with
a Linux-compatible environment that builds and runs RustPilot. Existing
toolchains and the current host remain temporary bootstrap dependencies.
This adds a separate unfinished workstream; no host OS installation is part
of the current increment. Hardware vendor blobs also require explicit treatment.
Do not silently treat use of C++ dependencies behind Rust wrappers as conversion.

## Publication

Publish as `NoahWLono/RustPilot`, public, with prominent incomplete status.
Explain that Codex and delegated agents inspected and translated pinned source,
wrote tests, and performed local checks. Distinguish AI review, human review,
builds, differential tests, simulation, hardware verification, and flight tests.
Argue for Rust's ownership, borrowing, initialization and concurrency checks
without claiming that a rewrite is automatically safer, uses less RAM, or
preserves all behavior. Keep exact verification results with each increment.
