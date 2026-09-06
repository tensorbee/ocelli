# F-013 metadata diff review, pass 3

**Reviewed**: staged diff from `8f84d04` in `/private/tmp/ocelli-f-013`, 24 paths, 2,339 insertions and 250 deletions
**Result**: 1 defect, 0 smells, 0 nitpicks
**Maximum-pass status**: the metadata-before-frame regression finding remains and requires operator attention under `.claude/WORKFLOW.md`

## Defects

### D1, The new lazy-helper test does not protect the production call site

**Where**: `tools/oracle/src/bin/ocelli-compare.rs:283`,
`tools/oracle/src/bin/ocelli-compare.rs:338`,
`tools/oracle/src/bin/ocelli-compare.rs:870`

**What**: `compare_runs` currently places both real `read_side` calls inside
the closure passed to `metadata_before_frames`, so the staged behavior is
correct. The new test calls `metadata_before_frames` directly with its own
closure. It proves that this helper does not invoke that closure after a truth
failure. It does not prove that `compare_runs` keeps the real frame reads
inside the closure.

**Why it is wrong**: Pass 2 required a durable test that turns red if either
production `read_side` is moved before the metadata verdict. A helper-only test
does not preserve that production ordering. A future edit can eagerly read
both frames in `compare_runs`, pass the already-read frames into the closure,
and retain a green test while restoring the exact pass 1 D4 failure. The plan's
new claim that moving frame I/O across the boundary makes the test red is not
mechanically true.

**Evidence**: I temporarily moved both production `read_side` calls from the
closure to the line immediately before `metadata_before_frames`. The helper,
test body and test closure were unchanged. Then I ran:

```text
bin/ocelli.sh test ocelli-oracle metadata_failure_does_not_invoke_a_failing_frame_reader
```

The named binary test passed. All code compiled, and the test still reported
that its injected closure was not called. The mutation was reverted and
`git diff --name-only` was empty afterward.

**Required repair**: Exercise the production per-view path rather than the
helper in isolation. One valid shape is to make the production per-view
operation accept injected frame readers and call that operation from both
`compare_runs` and the test. Another is a standing combined mutation that
changes committed metadata and makes the same target frame malformed, then
requires a `metadata-truth` record with no statistics. The test must fail when
a real production frame read moves ahead of the truth verdict.

This boundary was the implementation defect in pass 1, the missing regression
in pass 2, and an insufficiently connected regression in pass 3. This is the
maximum microscope pass, so the repeated finding is surfaced to the operator
rather than silently accepted.

## Smells

None.

## Nitpicks

None.

## Prior-defect recheck

All seven pass 1 behaviors remain repaired in the staged implementation.

- Metadata and display values have one executable owner in
  `metadata-truth.json`. Series geometry has one owner in
  `volume-truth.json`. Truncating the four-sample C.11 fixture is refused.
- Rust and Python distinguish missing from explicit null, preserve array order
  and distinguish positive from negative zero.
- Opposite-side, mixed and shared-plus-one-sided findings are unattributed.
- Metadata truth currently produces a failure record before production frame
  reads. The defect above is the missing production-connected regression.
- Real-row sensitivity comes from sidecar paths and complete parameter values
  are withheld before report serialization, including nested data.
- Resolved values bind to exact provenance pointers. Raw DICOM lookup applies
  per-frame precedence over shared and top-level declarations.
- The generated unsigned CT positively declares `INVERSE`. The independent
  reader transports it and the standing mutation changes it to `IDENTITY`.

Pass 2's behavioral proof was repeated through the current generated file.
Pydicom reads `PresentationLUTShape = INVERSE` and Explicit VR Little Endian
from `corpus/data/synthetic/ct_unsigned_16.dcm`. Its SHA-256 is
`dd732b865d40814d0cbcdd5ff05ca8ea00bf3d191ffd164e0a6af4d7cf520cd1`,
which exactly matches the staged manifest.

The canonical C.11 values still match PS3.3. LINEAR at inputs -160, 40, 240
and -60 is 0, 127.81954887218046, 255 and 63.90977443609023.
LINEAR_EXACT is 0, 127.5, 255 and 63.75. No duplicate evaluator or executable
expected-value table was introduced. F-X012's SIGMOID fixture remains green,
and no tolerance file changed.

## Verification

The complete staged candidate was inspected again, including both earlier
reviews, the plan correction, all metadata truth and provenance paths, report
redaction, browser transport, generator changes, mutation catalogue and LLD
updates.

`bin/ocelli.sh test ocelli-oracle` passed 81 library tests, the new binary
test, and all 53 integration fixtures. The integration split remains 8
geometry, 4 metadata, 5 render hash, 2 symmetry, 10 tolerance and 14 VOI
divergence tests. `check`, `clippy` and `fmt` passed for the oracle crate.

`bin/ocelli.sh compare` reported 99 views as 71 pass, 0 fail, 28 unmeasured
and 0 absent. Reference and candidate aggregate hashes were both
`413e030d7b202d5a274e85c725e70b53e40e154ed1beea3041f052b254e1c603`.
All 28 standing mutations were detected, including all seven F-013 metadata
mutations.

The Node oracle suite passed 223 tests. Python's metadata comparison and
redaction self-test passed. `bin/ocelli.sh gate corpus` verified all 92 rows,
with 0 missing and 0 mismatched. The staged `prose`, `content`, `provenance`
and `deviations` gates passed.

No implementation, verification state, ledger, commit, integration or push
was changed by this review.
