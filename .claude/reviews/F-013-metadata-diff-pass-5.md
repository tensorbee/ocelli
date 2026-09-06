# F-013 metadata diff review, pass 5

**Reviewed**: staged diff from `8f84d04` in `/private/tmp/ocelli-f-013`, 26 paths, 2,572 insertions and 252 deletions
**Result**: 1 defect, 0 smells, 1 nitpick

## Defects

### D1, The design plan still claims a removed helper test proves ordering

**Where**: `.claude/plans/F-013-design.md:223`

**What**: The plan says `metadata_before_frames` has a focused test with a
failing closure and invocation flag, then says moving frame I/O across the
boundary makes that test red. That helper-only test was removed after pass 3
proved it remained green when production frame reads moved. The Rust binary
now runs zero unit tests. The production-connected standing mutation is the
replacement proof, but this paragraph still describes the rejected proof as
current implementation evidence.

**Why it is wrong**: The design plan is the reviewed explanation of why the
candidate is safe. It cannot claim a test that is absent, especially when that
test was removed because it did not protect the production call site. The
earlier pass-3 correction in the same plan correctly describes the combined
mutation, so the file now gives two contradictory accounts of the repair.

**Evidence**: `bin/ocelli.sh test ocelli-oracle` reports zero tests for
`src/bin/ocelli-compare.rs`. Searching the binary finds no
`metadata_failure_does_not_invoke_a_failing_frame_reader` test. The current
proof is `metadata-failure-precedes-frame-refusal` in the mutation catalogue.

**Required repair**: Replace the stale pass-2 paragraph with the implemented
production-connected proof. State that the combined mutation changes committed
metadata and makes either target frame reader refuse if reached. Do not retain
the removed closure-test claim.

## Smells

None.

## Nitpicks

### N1, Pass 3 still overstates the integration fixture count

**Where**: `.claude/reviews/F-013-metadata-diff-pass-3.md:102`

Pass 3 says 53 integration fixtures. The listed suites and current output are
8 geometry, 4 metadata, 5 render hash, 2 symmetry, 10 tolerance and 14 VOI
divergence tests. They total 43.

## Pass 4 defect recheck

Pass 4 D1 is closed in the implementation and mutation harness. The combined
effect still mutates only the candidate sidecar, which makes the expected
metadata attribution unambiguously `Ours`. Its frame refusal now applies to
either reader for the selected target, independently of `mutation.side`.
Ordinary frame-byte mutations retain their side check. This separation makes
the combined mutation sensitive to an eager reference read and an eager
candidate read without changing other mutation semantics.

I independently moved the production readers one at a time. Each experiment
started from and returned to an empty unstaged diff.

With only the reference `read_side` moved above `metadata_before_frames`, the
combined mutation produced:

```text
metadata-failure-precedes-frame-refusal NOT DETECTED
the run refused with "the combined metadata mutation reached malformed frame I/O"
compare: 29 mutations, 1 not detected
```

After restoring exactly, moving only the candidate `read_side` above the
boundary produced the same red result. After the second restoration,
`git diff --name-only` and `git diff --stat` were both empty.

The restored production path first compares committed metadata, then invokes
the closure containing the reference and candidate reads only when metadata is
clean. `bin/ocelli.sh compare` returned 29 mutations with 0 not detected.

## Earlier finding recheck

All seven pass-1 implementation defects remain closed. The pass-2 ordering
gap and pass-3 production-connection gap are now mechanically closed on both
frame readers. Metadata and display truth still has one executable owner,
series geometry has one owner, truncation is refused, exact missing and numeric
semantics match across Rust and Python, mixed attribution is conservative,
real-row parameter values are withheld, raw functional-group precedence is
checked, and the generated fixture positively declares `INVERSE`.

The comparator LLD correctly records 29 mutations and eight F-013 metadata
probes. It accurately says the combined entry refuses either input frame. The
plan's earlier pass-3 correction says the same. D1 is limited to the later
stale helper-test paragraph.

No tolerance changed. F-X012's SIGMOID fixture, the four canonical C.11 values,
the 92-row corpus and the 99-view census remain unchanged from the independently
verified evidence in the earlier passes.

## Verification

After restoration, `bin/ocelli.sh test ocelli-oracle` passed 81 library tests
and all 43 integration fixtures. `check`, `clippy` and `fmt` passed for
`ocelli-oracle`.

The restored `bin/ocelli.sh compare` reported 99 views as 71 pass, 0 fail, 28
unmeasured and 0 absent. Reference and candidate aggregate hashes were both
`413e030d7b202d5a274e85c725e70b53e40e154ed1beea3041f052b254e1c603`.
All 29 standing mutations were detected.

The staged `prose`, `content`, `provenance` and `deviations` gates passed. No
implementation, verification state, ledger, commit, integration or push was
changed by this review.
