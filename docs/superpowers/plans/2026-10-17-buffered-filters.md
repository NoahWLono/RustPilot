# Buffered-filter conversion

Implement the eight upstream files listed in [NEXT.md](../../NEXT.md) at the
existing pinned revision. The user has authorized incremental public work,
documented behavior fixes, and no change to the all-target final scope.

## Interface and behavior

Expose independent `buffered`, `ordered`, and `butterworth` modules in the
existing no_std filter crate. Use fixed-size arrays and no external dependencies
or unsafe Rust. Keep the low-pass API intact. Each module owns its state; reuse
storage only where doing so preserves the upstream distinction between circular
and ordered buffers.

Buffer capacities cover 1 through 255, with invalid const-generic sizes rejected
at construction. Use safe indexes and initialized arrays; the 128 through 255
range is a documented correction of upstream's signed reset-loop index.
Reference comparisons cover defined C++ behavior through capacity 127.
Physical-slot access returns an option for out-of-range requests.

Average filters preserve their source accumulator types, physical summation
order, warmup and truncation behavior. Use checked arithmetic and explicit
errors for signed overflow and out-of-range float-to-integer results. Failed
updates must leave state intact. Document defined unsigned wrapping separately.
Cover the real i32/i32 integral-average use as well as every declared scalar
alias family. Correct integral reset to clear the retained sum.

Mode filters preserve alternating extreme eviction, including the initial
high-eviction flag, even capacities, invalid return-index fallback and floating
comparison behavior. Construction initializes output; reset restores constructor
state. Derivative filters cover float capacities 5, 7, 9 and 11, timestamp wrap,
duplicate rejection, warmup and slope caching. Their corrected reset clears
samples, timestamps and cached state.

Butterworth filters include all nine coefficient sets and 26 aliases. Preserve
the source f32 constants and recurrence order. Explicit zero construction and
reset define the reference's zero-initialized state.

## Implementation and evidence

1. Add shared test support for immutable pinned C++ inputs. Compile actual
   source with `-ffp-contract=off`; disclose any dependency shims.
2. In parallel, implement buffered/average, mode/derivative, and Butterworth
   modules. Write behavioral tests first, then reference comparisons. Keep
   module files, test drivers and reports disjoint between implementers.
3. Review each implementation and its tests against the source. Prove reset
   differences explicitly; do not silently compare against a corrected C++
   algorithm. Compare exact float bits except NaN payloads.
4. Run host tests and comparisons in debug/release, formatting, Clippy and both
   existing Cortex-M library builds. Resolve failures before publication.
5. Update the source-to-port map, reports and public status with verified
   coverage. Publish the increment and verify the remote commit and CI result.

These component checks do not establish integrated firmware, timing, hardware
support, remaining Filter-file coverage or compiler/OS replacement.
