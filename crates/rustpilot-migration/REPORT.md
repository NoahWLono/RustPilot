# Migration inventory scope

The CLI inventories Git source objects at the reviewed ArduPilot revision
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`. It runs Git read commands and never
runs Waf, upstream scripts, dependency installers, or upstream executable code.
The host tool uses safe Rust and the standard library without Cargo dependencies.

Run from the RustPilot workspace:

```sh
cargo +1.95.0 run -p rustpilot-migration -- inventory ../ardupilot-upstream crates/rustpilot-migration/baseline
```

The CLI always requires the reviewed root revision. The library function
`inventory(source, output, expected_revision)` takes an explicit full revision
for isolated Git fixtures or a future reviewed baseline. Every available nested
checkout must match its parent's gitlink revision. A changed revision or tracked
index/worktree change aborts before output. Missing checkouts remain explicit
unresolved dependency rows. The inventory rejects index flags that could hide
tracked changes, including assume-unchanged and skip-worktree. Nonempty Git
clean/process filter commands are rejected before status or diff checks.
Git calls disable filesystem-monitor hooks and replacement-object semantics. Untracked files
are excluded. Output must be outside the source checkout.

## Baseline observed locally

| Scope | Count |
| --- | ---: |
| Root blobs and gitlinks | 8,464 |
| Root gitlinks | 14 |
| Nested gitlinks | 2 |
| Inventoried repositories including root | 17 |
| All blobs and gitlinks across these repositories | 25,366 |
| Missing Git checkouts | 0 |
| Root library directories | 156 |
| Root hardware definition directories, union of both definition kinds | 508 |
| Hardware definition files, `hwdef.dat` and `hwdef-bl.dat` | 927 |
| Static Waf class declarations | 9 |

These are source accounting counts. Every source row is `not_ported`; every
hardware and Waf row is `unsupported`. The CLI does not infer implementation,
compatibility, or board support from paths, placeholders, or these counts.
The separately reviewed migration mappings must identify actual implemented
units; this source inventory does not count those units as complete.

## Output contract and limits

`sources.tsv` records repository-relative checkout identity, revision, path,
Git object ID, blob/gitlink kind, object byte size, component, extension category,
and migration status. Gitlink byte sizes are `-`, as supplied by Git, because
a commit link has no blob byte size. Component grouping follows the root path
and the first child below `libraries` or `Tools`.

`languages.tsv` counts every entry by lowercased filename extension, including
all tool languages and unknown extensions. Extensionless files and gitlinks have
explicit categories. These categories group file names without inspecting source syntax. A `.h` file does not establish which language consumes
it, and an extensionless script is not assigned a language from its shebang.

`hardware.tsv` records both firmware and bootloader definition files and their
containing directories. `waf-classes.tsv` records column-zero Python `class`
declarations in the pinned `Tools/ardupilotwaf/boards.py` blob, with object IDs
and line numbers. This is static syntax evidence. Dynamic Waf registration,
class inheritance semantics, valid build combinations, and generated targets
are not evaluated. Neither table establishes working Rust firmware.

`dependencies.tsv` records every direct and recursive gitlink, its exact pin,
and whether its checkout was inventoried or missing. It also records unresolved
external SDK, package, toolchain, and vendor-blob closure. Complete Git submodule
coverage does not mean all dependencies have been captured or converted.

Rows are sorted and output contains no clock times or machine-specific absolute
source paths. TSV fields escape backslash, tab, newline, and carriage return;
non-ASCII bytes use `\xNN`, so arbitrary Unix Git names remain reversible.
The Windows path conversion rejects non-UTF8 Git paths. Output writes are not a
transaction: an I/O failure can leave a partial destination. Source verification
finishes before output writing; a concurrent process can still change a checkout
after verification. Run against an otherwise idle reference checkout.

## Verification

Eleven tests use temporary local Git repositories: all-object accounting and
byte-for-byte deterministic output, hardware and bootloader evidence, odd
filenames, root revision pinning through the CLI, tracked dirt rejection,
index flags that can hide dirt, missing submodules, nested pinned recursion,
changed dependency revisions, staged gitlink changes with present and missing
checkouts, and changed nested tracked content. Additional fixtures verify that
clean/process commands never execute, filesystem-monitor hooks are disabled,
and replacement objects cannot substitute another tree under the pinned ID.
No network or upstream code is needed for these tests.

The following completed successfully with Rust 1.95.0:

```sh
cargo +1.95.0 test -p rustpilot-migration
cargo +1.95.0 clippy -p rustpilot-migration --all-targets -- -D warnings
cargo +1.95.0 run -p rustpilot-migration -- inventory ../ardupilot-upstream /tmp/rustpilot-inventory-verify
diff -rq crates/rustpilot-migration/baseline /tmp/rustpilot-inventory-verify
```

The full baseline generations were identical, including a fresh generation
after both review fixes for staged gitlinks and executable Git filters. Workspace integration verification
is recorded separately by the coordinator.
