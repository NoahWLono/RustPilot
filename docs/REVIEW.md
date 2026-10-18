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
