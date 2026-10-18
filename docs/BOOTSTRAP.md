# Compiler and host scope

The owner expanded the final goal to include compiler and host operating-system
dependencies. The requested environment is Linux-compatible and must build and
run RustPilot. This is additional work beyond converting the ArduPilot tree;
it is not implemented by this repository's initial libraries.

## Current bootstrap dependencies

The first increment uses Rust 1.95.0 with its standard library, Cargo, rustfmt,
Clippy, Git, a Linux host, native linker/runtime libraries, and a C++ compiler
for reference comparisons. CI additionally depends on GitHub Actions and its
runner environment. The host inventory tool invokes Git. `no_std` core crates
avoid Rust's standard library at runtime, but that alone says nothing about the
implementation language of the compiler, linker, host, firmware or boot chain.

The Rust compiler normally uses LLVM for code generation; its developer guide
also describes alternative backends. Selecting a different backend is not
proof that the compiler's entire dependency graph is Rust-only, supports every
required architecture, or reproduces the same machine behavior. See
[rustc code generation](https://rustc-dev-guide.rust-lang.org/backend/codegen.html).

## Work required before a completion claim

1. Pin and inventory the compiler, linker, runtime, host kernel, userspace,
   filesystem, process, network and build-tool dependencies actually required
   to build and run the migrated project. Record optional and architecture
   specific dependencies too.
2. Define the Linux interfaces and executable formats required by the build
   tools, simulation, and deployed Linux targets. Verify Linux compatibility
   with executable conformance tests for those interfaces.
3. Implement or select audited Rust replacements, retaining license notices,
   and verify their dependency closure. Any non-Rust bootstrap stage, assembly,
   firmware blob or external service must remain explicitly classified.
4. Build the replacement toolchain in that environment, reproduce its outputs,
   and establish a documented bootstrap/trust chain. Self-hosting alone does
   not prove compiler correctness or absence of non-Rust dependencies.
5. Repeat application and target verification with the replacement environment.
   A successful build on the existing Linux host does not satisfy this gate.

This scope is tracked in RustPilot. It does not authorize replacing the user's
running operating system or installing experimental firmware. The current
development environment remains the bootstrap environment while replacement
source and artifacts are developed and reviewed.
