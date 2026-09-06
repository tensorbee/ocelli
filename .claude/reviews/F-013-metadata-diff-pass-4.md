# F-013 metadata diff review, pass 4

**Reviewed**: staged diff from `8f84d04` in `/private/tmp/ocelli-f-013`, 25 paths, 2,448 insertions and 252 deletions
**Result**: 1 defect, 0 smells, 1 nitpick

## Defects

### D1, The combined mutation watches only the candidate frame reader

**Where**: `tools/oracle/src/mutations.rs:393`,
`tools/oracle/src/bin/ocelli-compare.rs:360`,
`.claude/plans/F-013-design.md:154`

**What**: `metadata-failure-precedes-frame-refusal` changes the candidate
sidecar and carries one injected frame refusal. `read_side` activates that
refusal only when `mutation.side == side`. Since the mutation side is
`Candidate`, an eager candidate frame read turns the mutation red, but an eager
reference frame read succeeds and remains invisible.

**Why it is wrong**: The metadata truth contract says both sidecars are judged
before either frame is opened. Pass 3 required the production-connected proof
to fail if either real `read_side` crosses that boundary. A guard covering only
the second read leaves half of the original precedence boundary unwatched. A
malformed reference frame can again pre-empt a known metadata result without
making the standing catalogue red.

**Evidence**: I tested the production reads independently and reverted both
mutations afterward.

First, I moved only the candidate `read_side` immediately above
`metadata_before_frames`. `bin/ocelli.sh compare` failed as intended:

```text
metadata-failure-precedes-frame-refusal NOT DETECTED
the run refused with "the combined metadata mutation reached malformed frame I/O"
compare: 29 mutations, 1 not detected
```

Next, from the restored candidate, I moved only the reference `read_side`
above `metadata_before_frames`. The comparator stayed green:

```text
metadata-failure-precedes-frame-refusal ok
compare: 29 mutations, 0 not detected
```

This is not a theoretical asymmetry. The condition in `read_side` explicitly
requires the current side to equal the mutation's candidate side. After the
second experiment was reverted, both `git diff --name-only` and
`git diff --stat` were empty.

**Required repair**: Make the production mutation exercise both readers. The
clearest shape is two combined entries, one with a candidate metadata mismatch
and candidate frame refusal, and one with a reference metadata mismatch and
reference frame refusal. Each must expect the correct one-sided
`metadata-truth` attribution. An alternative may inject refusal on either
reader independently of the sidecar mutation, but it must retain unambiguous
side semantics. Independently moving either production read above truth must
make at least one standing mutation red.

The implementation itself currently keeps both reads behind metadata truth.
This finding is about the incomplete durable proof of that production
ordering.

## Smells

None.

## Nitpicks

### N1, Pass 3 overstates the integration fixture count

**Where**: `.claude/reviews/F-013-metadata-diff-pass-3.md:99`

The review says the oracle ran 53 integration fixtures. Its own breakdown is
8 geometry, 4 metadata, 5 render hash, 2 symmetry, 10 tolerance and 14 VOI
divergence tests, which totals 43. The current test output also totals 43.

## Prior-defect recheck

All seven pass 1 implementation defects remain closed. Metadata and display
truth has one owner, volume geometry has one owner, missing and null remain
distinct, numeric comparisons preserve signed zero, mixed findings remain
unattributed, real-row parameter values remain withheld, scope binds to raw
functional-group provenance, and the generated fixture positively declares
`INVERSE`.

Pass 2's ordering behavior is also implemented correctly. The current
`compare_runs` computes truth, creates the metadata failure record, and only
invokes the closure containing both frame reads when truth is clean. Pass 3's
production-connection gap is partly repaired by the new combined standing
mutation. D1 identifies the uncovered reference half.

The new mutation plumbing is otherwise coherent. The combined effect changes
the declared Presentation LUT Shape through the same checked sidecar setter as
the existing string mutation. `frame_read_refusal` exposes only the combined
effect. `read_side` checks target and side before returning the declared
refusal. The expectation requires `Fail`, only `MetadataTruth`, and `Ours`.
The fixed view target resolves against the identity records rather than a
filesystem name guessed by the mutation.

The comparator LLD correctly reports 29 mutations and describes eight F-013
metadata probes. The plan records the production-connected combined mutation.
Its later pass-2 paragraph still says the removed helper-only focused test
makes moved frame I/O red. That stale claim is part of D1's documentation
surface and should be removed when the two-sided proof is repaired.

## Verification

After restoring both adversarial edits, `bin/ocelli.sh test ocelli-oracle`
passed 81 library tests and all 43 integration fixtures. There are no binary
unit tests in the restored candidate. `check`, `clippy` and `fmt` passed for
`ocelli-oracle`.

The restored `bin/ocelli.sh compare` reported 99 views as 71 pass, 0 fail, 28
unmeasured and 0 absent. Both aggregate hashes were
`413e030d7b202d5a274e85c725e70b53e40e154ed1beea3041f052b254e1c603`.
All 29 standing mutations were detected.

The staged `prose`, `content`, `provenance` and `deviations` gates passed.
Pass 3 already established the unchanged Node, Python, corpus and actual
generated-INVERSE evidence. No tolerance changed.

No implementation, verification state, ledger, commit, integration or push
was changed by this review.
