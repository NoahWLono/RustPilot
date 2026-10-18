# FFI and hardware boundaries

The migration intends to replace ArduPilot's implementation and its non-Rust
dependencies. It has not yet resolved interoperability for a complete hardware
target. There is no supported board or integrated vehicle firmware today.

## What the current code proves

The checksum and scalar-filter crates contain Rust implementations, use
`no_std`, forbid unsafe Rust, and declare no third-party Cargo dependencies.
They do not call the original ArduPilot algorithms through FFI. Their comparison
tests compile pinned C++ into separate executables and compare outputs through
process input/output. Those executables are separate test dependencies. The
Rust libraries do not link them.

The inventory tool uses Rust's standard library and starts Git subprocesses.
The bootstrap environment still includes native libraries, Linux, LLVM and
other non-Rust software. Forbidding unsafe Rust in these crates does not establish
that their entire execution environment is implemented in safe Rust. See
[compiler and host scope](BOOTSTRAP.md).

## The distinction that matters

FFI lets Rust call implementations through a foreign interface, such as a C
ABI. Retaining a C RTOS or vendor SDK can require many such calls. Generated
bindings describe that interface; they do not prove the foreign implementation
memory-safe or discharge pointer, ownership, concurrency and lifetime obligations.
The [Embedded Rust Book's C interoperability chapter](https://doc.rust-lang.org/stable/embedded-book/interoperability/c-with-rust.html)
describes these interfaces and their raw-pointer requirements.

Hardware access can also require unsafe Rust: memory-mapped registers,
interrupt entry, DMA buffers and context switching have constraints outside
ordinary ownership checking. These operations need not call C code. Rust
peripheral and HAL implementations can encapsulate them behind safe interfaces,
provided those interfaces enforce the necessary invariants. The
[Embedded Rust Book's register-access chapter](https://doc.rust-lang.org/stable/embedded-book/start/registers.html)
illustrates this separation. A safe-looking wrapper alone is insufficient;
incorrect unsafe code can invalidate the safety of its callers.

An `extern "C"` calling convention also does not establish the implementation
language of the callee. A Rust implementation can expose a C-compatible ABI.
Linux compatibility similarly specifies interfaces and behavior, rather than
requiring that every service behind those interfaces be written in C.

If `embedded_ffi` means the specific [`embedded-ffi` crate](https://docs.rs/embedded-ffi/0.1.2/embedded_ffi/),
that crate supplies interoperability utilities, including string types.
RustPilot does not depend on it. Embedded Rust does not inherently require that
particular crate.

## Known upstream dependencies still to replace

These are representative source observations at the pinned ArduPilot revision.
The complete dependency audit and each row's replacement remain unfinished.

| Area | Upstream evidence | Consequence for this migration |
| --- | --- | --- |
| ChibiOS | [`Scheduler.cpp`](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_HAL_ChibiOS/Scheduler.cpp#L108) calls `chThdCreateStatic` and other RTOS APIs. | Keeping ChibiOS would retain a non-Rust runtime dependency. Rust scheduler, synchronization and driver replacements need target verification. |
| ESP32 | [`Scheduler.cpp`](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_HAL_ESP32/Scheduler.cpp#L23) includes FreeRTOS and ESP-IDF APIs, including watchdog services and a [ROM delay call](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_HAL_ESP32/Scheduler.cpp#L220). | Replacing the application alone does not replace the SDK, RTOS or ROM services. Audit each target's actual dependency closure. |
| Linux and SITL | [`Linux scheduler`](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_HAL_Linux/Scheduler.cpp#L8) includes system interfaces; [`SITL scheduler`](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_HAL_SITL/Scheduler.cpp#L384) creates POSIX threads. | Using Rust's standard library can hide host interoperability from application source. The requested Rust host replacement remains a separate requirement. |
| QURT | [`Board configuration`](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/Tools/ardupilotwaf/boards.py#L1608) selects a Hexagon SDK/toolchain and QURT headers. | Compiler support and SDK/runtime replacement must be established for these targets too. |

There are firmware boundaries beyond ordinary FFI. The Pixart3900 optical-flow
driver [ships a sensor SROM payload](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_OpticalFlow/AP_OpticalFlow_Pixart_SROM.h#L1)
and [downloads it to the sensor](https://github.com/ArduPilot/ardupilot/blob/afeb9c4941543d97f3e04fecf257b8f598f49a6a/libraries/AP_OpticalFlow/AP_OpticalFlow_Pixart.cpp#L214).
Translating the byte array declaration into Rust would preserve that sensor
program unchanged. Its replacement remains unresolved.

## Rules for the remaining implementation

Keep portable algorithms independent of platform ABI details. Put necessary
hardware operations and any temporary foreign calls in explicit platform
modules. The owner permits narrowly reviewed unsafe boundaries; the existing
portable crates continue to forbid unsafe code.

For each new boundary, record the target and feature configuration, exact
dependency/version, calling convention, buffer layout and ownership, callback
lifetime, interrupt/thread context, failure behavior, and panic/unwind policy.
DMA and shared-memory operations also need synchronization and cache-coherency
contracts. Expose a safe API only when it enforces the relevant obligations.
Require focused tests and separate review of the unsafe implementation.

Any temporary C/C++ bridge must remain recorded as an outstanding replacement,
with the source it retains and an explicit condition for removing it. A wrapper
is not completion of the wrapped dependency. This policy does not establish
that a bridge is needed for every port, or authorize widening unsafe code
throughout the portable libraries.

Before claiming a target meets the full goal, inspect its linked image, linker
map and resolved symbols alongside its build inputs and dependency provenance.
Include runtime-loaded code, SDK archives, boot components and configuration
variants; symbol names alone cannot prove implementation language. Verify the
replacement's resource use and behavior, including timing, on the target. Host unit tests
and successful library cross-compilation cannot provide that evidence.

Vendor blobs, unavailable source and immutable chip ROM may prevent complete
replacement on a particular target. Their necessity and replaceability have not
been established for all ArduPilot boards. Keep such dependencies visible and
unresolved; neither a Rust wrapper nor an exception invented during development
satisfies the owner's full-conversion requirement.

## Assessment

Retaining most of ArduPilot's C/C++ platform stack while adding Rust components
could create a substantial FFI maintenance burden. Replacing that stack can
remove those foreign calls, but requires implementing and verifying the services
they provided. The current ports demonstrate that shared algorithms can be
translated without runtime C++ bindings. They do not yet demonstrate the same
result for the complete flight stack, every platform, or the expanded toolchain
and operating-system scope.
