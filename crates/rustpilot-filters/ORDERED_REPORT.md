# Ordered and derivative filter port

This module translates ArduPilot `libraries/Filter/ModeFilter.h`,
`ModeFilter.cpp`, `DerivativeFilter.h` and `DerivativeFilter.cpp`, including their
inherited `FilterWithBuffer.h` sample access. The reference revision is
`afeb9c4941543d97f3e04fecf257b8f598f49a6a`. The algorithms and original GPL notices
belong to the ArduPilot contributors; the Rust translation is AI-assisted and
licensed GPL-3.0-or-later.

The production implementation uses fixed arrays, has no external dependencies,
and compiles under the crate's `forbid(unsafe_code)` and `no_std` restrictions.
These component tests establish neither vehicle integration nor flight safety.

## Source mapping

| Upstream interface | Rust interface |
| --- | --- |
| `ModeFilter<T,N>` constructor | `ordered::ModeFilter::<T,N>::new(return_element)` |
| `ModeFilter::apply`, `get` | `apply`, `get` with the same selected-rank behavior |
| Inherited `ModeFilter::reset` | `reset`, with complete-state correction below |
| `ModeFilterInt8_Size3` through `Size7` | `ModeFilter<i8, 3>` through `ModeFilter<i8, 7>` |
| `ModeFilterUInt8_Size3` through `Size7` | `ModeFilter<u8, 3>` through `ModeFilter<u8, 7>` |
| `ModeFilterInt16_Size3` through `Size7` | `ModeFilter<i16, 3>` through `ModeFilter<i16, 7>` |
| `ModeFilterUInt16_Size3` through `Size7` | `ModeFilter<u16, 3>` through `ModeFilter<u16, 7>` |
| `ModeFilterFloat_Size3` through `Size7` | `ModeFilter<f32, 3>` through `ModeFilter<f32, 7>` |
| `DerivativeFilterFloat_Size5`, `Size7`, `Size9` | `ordered::DerivativeFilter<5>`, `<7>`, `<9>` |
| Explicit float/11 implementation in the source `.cpp` | `DerivativeFilter<11>` |
| Derivative `update`, `slope`, `reset` | Same names; complete-state reset correction below |
| Inherited derivative `apply` | Same sample-only update; leaves timestamps and slope cache unchanged |
| Inherited `get_filter_size`, `get_sample` | Same names; size/index use `usize`, sample reads return `Option<T>` |
| Polymorphic `Filter<T>` operations | `buffered::Filter<T>` implementation; `Output = T` |

Mode supports all 25 declared aliases, including even capacities despite the
upstream comment recommending odd sizes. It also supports capacity 1 and sizes
through 255. Sizes zero or greater than 255 panic at construction. An invalid
return rank falls back to `N / 2`; unlike the upstream uint8 argument, Rust's
`usize` argument does not narrow values above 255. The reference matrix covers
all uint8-rank behaviors used by the fixtures.

`ModeSample` supplies `ZERO`, `Copy` and `PartialOrd`. All built-in primitive
numeric types implement it; custom types must provide the equivalent of
upstream `T(0)`. The tested upstream alias types are i8, u8, i16, u16 and f32.
The module does not provide the original C++ typedef spellings as extra Rust
aliases; the generic types in the table are their replacements.

Derivative covers every scalar instantiation supplied by the upstream `.cpp`.
Other capacities and non-f32 derivative types are not implemented. The source's
template bodies are visible to the reference driver, but its normal separately
compiled implementation supplies only those four float sizes.

## Preserved behavior

Mode keeps sorted physical slots, inserts each sample and alternates which
extreme is evicted once full. It starts with the high-eviction flag set. Warmup
insertion always uses the high-insertion path and returns the middle populated
slot. Once full, the configured rank is returned. This is not an oldest-sample
sliding median. Strict `>` and `<` comparisons retain source behavior for equal
values, signed zeros and NaNs; no total ordering is introduced.

Derivative stores u32 timestamps alongside its circular sample buffer. A sample
whose timestamp equals the preceding physical slot is ignored, including an
initial timestamp of zero. Subtractions wrap as unsigned 32-bit arithmetic,
and converting the difference to f32 follows the upstream arithmetic order.
There is no monotonicity check or implicit conversion from microseconds to
seconds. The result's units are sample units per timestamp unit.

The derivative's warmup condition compares only the last two physical timestamp
slots, as upstream does. It can permit a result before all slots have been
filled. New data triggers recomputation; ordinary repeated slope calls return
the cached result. Nonfinite arithmetic results become zero. The inherited
sample-only `apply` is intentionally available, including its unusual behavior
of changing samples and the ring position without changing timestamps or the
cached slope. Normal timestamped observations should use `update`.

## Defined initialization and reset corrections

All Rust fields start initialized. This matches zero-initialized upstream
objects while defining values that automatic-storage C++ constructors can leave
indeterminate. Mode starts with output zero. Derivative starts with zero
samples, timestamps, cached slope and pending-data flag.

Upstream mode reset clears samples and count but retains its output and eviction
phase. Rust reset also clears the output and restores the initial high-eviction
phase. The regression compares actual upstream output: after samples 1, 2, 100,
upstream reset leaves output 2 while Rust returns zero. Inserting 1, 2, 100, 3
again produces upstream 2 versus Rust 3 because the upstream phase was retained.

Upstream derivative reset clears samples and position but retains timestamps,
pending-data state and cached slope. Rust clears all of them. After a cached
unit slope, upstream reset still returns 1 while Rust returns zero. Upstream
also rejects the first post-reset sample when its timestamp equals the former
last timestamp; Rust accepts that sample. The Rust reset intentionally differs
from the original behavior for these state fields.

Mode's inherited reset loop and insertion indices use signed 8-bit values in
C++, which cannot safely handle capacities 128 through 255. Rust uses bounded
`usize` indexing and initializes the array directly. Behavior at these sizes is
a defined extension, tested locally rather than compared against undefined C++
behavior. Physical-slot reads outside the buffer return `None` instead of
performing an unchecked read.

## Verification

`tests/ordered.rs` has 16 tests: 14 ordinary behavior tests and two explicitly
ignored reference tests requiring `ARDUPILOT_REFERENCE` and a C++ compiler.
The suite checks warmup, every supported derivative size, duplicate timestamps,
wraparound, nonfinite recovery, cached slope, pending-update reset, mode eviction,
even capacities, invalid ranks/capacities, signed-zero/NaN behavior, the size-255
correction, and trait dispatch.

The reference driver includes the verified upstream `.cpp` files directly, so
all filter algorithms are the original source. Both constructors use static
storage to guarantee zero initialization. The driver only parses fixtures,
invokes public operations and prints outputs. It substitutes an AP_Math header
that supplies `std::isnan` and `std::isinf`, plus an empty `Filter.h` umbrella;
those shims provide no filter algorithms. Shared test support verifies each input
against its pinned Git object with replacement objects disabled, then compiles
copied bytes in a temporary directory. The reference uses `-ffp-contract=off`
and undefined-behavior/float-cast-overflow sanitizers. IEEE division by zero is
not included in those sanitizer checks.

The comparison matrix checks 140 mode configurations: five scalar alias types,
seven capacities (1, 3, 4, 5, 6, 7, 127) and four rank selections. Each observes
zero initialization followed by 520 samples. The derivative matrix runs 1,750
operations at each of capacities 5, 7, 9 and 11, including irregular and
non-monotonic times, duplicate rejection, timestamp wrap, inherited sample-only
updates and arbitrary f32 bit patterns. Every output and physical slot is
compared exactly; NaN payloads are ignored. Mode comparisons cover 1,667,340
serialized values and derivative comparisons 70,136 values. These totals include
repeated state/cache reads and capacity metadata; they are not independent
algorithm counts or exhaustive proofs.

The second reference test verifies the four intentional reset divergences
described above against unmodified upstream methods. Main reference comparisons
do not replace the upstream reset with a corrected implementation.

Run from the repository root:

```sh
ARDUPILOT_REFERENCE="$PWD/../ardupilot-upstream" cargo test -p rustpilot-filters --test ordered --locked -- --include-ignored
ARDUPILOT_REFERENCE="$PWD/../ardupilot-upstream" cargo test -p rustpilot-filters --test ordered --release --locked -- --include-ignored
```

Host debug and release comparisons check the fixture domain. They do not prove
hardware timing, all floating-point targets, all possible input histories or
integration into any vehicle. No hardware is available for target operation
checks at this stage.
