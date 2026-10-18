# Review record

## Initial increment

Separate Codex agents reviewed the checksum/filter ports and the migration
inventory. This was AI code review; human review and hardware qualification
remain outstanding.

The core reviewer inspected the algorithms against pinned upstream source and
found no concrete numerical mismatch in the declared scope. The inventory
reviewer checked recursive accounting, deterministic output, pinning, and the
limits on target-support claims.

Four provenance issues were addressed before publication:

| Finding | Correction and regression |
| --- | --- |
| Hidden tracked changes could evade the filter reference check | Compare all consumed source bytes with pinned Git objects; test skip-worktree and assume-unchanged flags |
| Staged gitlink changes could evade the inventory cleanliness check | Compare index against HEAD without ignoring submodules; test present and absent child checkouts |
| Git status could execute configured clean/process filters | Reject those configured commands before checking the worktree; prove fixture script markers are absent |
| Replacement objects could alter the meaning of a pinned SHA | Set `GIT_NO_REPLACE_OBJECTS=1`; test actual replacement refs in each relevant crate |

The inventory wrapper also disables Git filesystem-monitor commands, with a
fixture checking that no monitor script executes. Reviewers inspected the fixes
to their reported findings. The coordinator reran the complete tests after
integration; results are recorded in [VERIFICATION.md](VERIFICATION.md).

Review found no proof of whole-program equivalence. The filter reference uses
documented supporting stubs and an extracted upstream alpha helper. CRC timing
can change because some table lookups became bitwise computations. These
limitations remain in the crate reports and require later target verification.

## Buffered-filter increment

A separate Codex agent inspected the buffer/average, mode/derivative and
Butterworth implementations against pinned upstream source, reviewed the actual
C++ reference drivers and their input verifier, and independently ran all 34
new tests with reference comparisons enabled. That run passed without failures
or ignored tests. The review reported no blocking algorithm or harness finding.

Two qualifications were incorporated into the reports. Narrow signed C++
accumulator overflow can involve implementation-defined narrowing, whereas
i32 arithmetic overflow is undefined; Rust's checked error policy rejects both.
Supported generic APIs, compared alias configurations and arbitrary C++ template
instantiations are distinct scopes. The source map records these distinctions.

During implementation, the coordinator corrected a derivative test fixture
whose intended equal timestamp spacing was inconsistent across wraparound.
The production algorithm was unchanged. The final coordinator checks cover the
whole workspace in both profiles and both existing Cortex-M library builds;
see [buffered-filter verification](BUFFERED_VERIFICATION.md). This review does
not establish firmware integration, hardware timing or flight safety.

## Exact vector, tangent and two-pole increment

A separate Codex reviewer compared the tangent translation with the pinned
original C source, checked the tiny-input rounding argument and independently
ran its debug and release comparisons. It also reviewed vector and biquad
arithmetic, scalar-versus-vector reset rounding, the mixed-precision coefficient
formula, integer truncation boundaries and transactional failure behavior.
The final review reported no remaining blocking finding.

Strict comparisons exposed compiler-dependent NaN choices, including different
results from the same C++ source when sanitizer instrumentation changed. The
owner required each target/compiler build's bits to be preserved. The accepted
component profile therefore records GCC 16.2.0, the container digest, flags,
translation-unit boundaries and floating-point environment. Rust explicitly
preserves that profile's NaN choices, including differences between direct
biquad calls and calls inlined into the owning filter. Other builds remain
unqualified.

Review prompted the following corrections and additional evidence:

- Correct the i32 reset conversion check to admit the defined fractional
  interval immediately below `i32::MIN` before truncation.
- Expand comparisons to the direct biquad API with arbitrary public
  coefficients, alongside the owning filter's generated coefficients.
- Preserve independently generated vector and direct-biquad stress corpora as
  permanent tests. They add fresh, independent NaN payloads to histories,
  samples and configuration rather than relying on already-poisoned histories.
- Make the optional native-host math comparison explicit and enforce the
  installed library hash. Clear dynamic-loader overrides in reference execution.
- Document the narrow component/compiler profile in public APIs and reports.

The reviewer independently passed the component tests available at review time
and the two new stress generators. The coordinator checks the final combined workspace suite and the retained
generators; observed results
are recorded in [exact-filter verification](EXACT_FILTER_VERIFICATION.md).
This remains AI review. Human certification, exhaustive sequence equivalence,
firmware integration, execution-time bounds and hardware evidence are outstanding.

## Vector magnitude and square-root increment

A separate Codex agent reviewed the integer square-root invariant, rounding
criterion, normalization and arithmetic bounds. It found no implementation
defect and independently compared sampled results with 200-digit decimal
calculations. This is a reviewed argument and a numerical check, not a
machine-checked proof.

The same reviewer inspected the actual-source vector reference and used a
separately authored driver to compare finite inputs and NaN combinations for
both dimensions, storage precisions and EKF settings. Review confirmed the
right-recursive double norm, component narrowing under float `ftype`, division
during normalization, epsilon limiting and untouched z bits. It also supplied
a finite association counterexample now retained as a named regression.

The committed matrix includes fresh NaNs in all operands and checks both
values of each precision macro. Reference drivers reject incompatible floating
environments. The reports distinguish compiler and runtime-library profiles,
ordinary exact Rust equality, output padding and actual floating observations.
The final review reported no actionable correctness finding in production
code or reference extraction. Coordinator results are in
[vector magnitude verification](VECTOR_MATH_VERIFICATION.md).
