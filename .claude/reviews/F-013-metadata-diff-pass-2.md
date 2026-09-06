# F-013 metadata diff review, pass 2

**Reviewed**: staged diff from `8f84d04` in `/private/tmp/ocelli-f-013`, 23 paths, 2,102 insertions and 247 deletions
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, The metadata-before-frame boundary has no durable regression test

**Where**: `tools/oracle/src/bin/ocelli-compare.rs:254`,
`tools/oracle/src/metadata.rs:57`, `tools/oracle/src/mutations.rs:289`

**What**: The repaired comparator now creates and records a metadata-truth
failure before calling either `read_side`, which fixes pass 1 D4. The committed
tests prove the failure-record shape, and the catalogue proves seven metadata
mutations against valid frame files. No committed test combines a truth
divergence with an unreadable or malformed frame. Moving the truth verdict
back below `read_side` would therefore leave the unit suite and all 28 standing
mutations green.

**Why it is wrong**: Pass 1 found this exact precedence failure. The contract
in `docs/lld/comparator.md` says a metadata failure remains reportable even
when frame bytes are absent or malformed. Code order proves today's behavior,
but it does not preserve the repaired boundary against regression. This
repository requires load-bearing refusal and precedence claims to have watched
red evidence.

**Evidence**: I copied the generated oracle output to a disposable directory,
changed `synthetic__ct_unsigned_16` from the true `INVERSE` declaration to
`IDENTITY`, and truncated that candidate's raw frame to 16 bytes. The current
release comparator returned a 99-view report with one failure at
`metadata-truth`, attributed it to `ours`, and did not return a frame-length or
digest error. This proves the implementation repair works. Searching the Rust
tests, browser tests and mutation catalogue found no equivalent combined case.
The existing `missing_member_is_not_explicit_null` test calls
`into_failure_record` directly and cannot detect reordered calls in
`compare_runs`.

**Required repair**: Add a committed integration test or standing comparator
mutation that supplies a metadata-divergent sidecar and an unreadable or
malformed frame for the same view. It must require a `metadata-truth` record
with no statistics and must turn red if either `read_side` is moved before the
metadata verdict. A mutation against only a valid frame is insufficient.

## Smells

None.

## Nitpicks

None.

## Pass 1 defect recheck

- D1 is repaired. `metadata-truth.json` is the sole metadata and display-value
  owner, while `volume-truth.json` is the sole series-geometry owner. Rust and
  Python project those documents without a second executable expected-value
  table. The display fixture validator requires exactly four C.11 samples.
- D2 is repaired. Rust distinguishes `Observed::Missing` from present null and
  compares numbers by `f64::to_bits`. Python uses a missing sentinel and packed
  IEEE-754 bytes. Ordered arrays remain ordered in both consumers.
- D3 is repaired. Opposite-side findings, mixed findings and a shared finding
  combined with a one-sided finding all become `Unattributed`.
- D4 behaves correctly in the staged implementation. Metadata truth is
  converted to a failure record and the loop continues before either frame is
  opened. The missing durable boundary test is the defect above.
- D5 is repaired. Sensitivity derives from the sidecar row path or volume
  series directory. A sensitive record replaces both complete parameter
  values before JSON serialization, so nested objects and arrays cannot leak.
- D6 is repaired. Resolved truth binds to an exact `metadataSources` pointer.
  The browser derives provenance from raw functional groups with per-frame
  precedence over shared and top-level declarations. Both value and scope are
  compared.
- D7 is repaired. The generated unsigned CT positively declares
  `PresentationLUTShape = INVERSE`, its manifest digest matches the generated
  file, the independent parser carries the present value, and a standing
  mutation changes it to `IDENTITY`.

## Independent evidence

The canonical soft-tissue values remain correct under PS3.3 C.11. At inputs
-160, 40, 240 and -60, LINEAR gives 0, 127.81954887218046, 255 and
63.90977443609023. LINEAR_EXACT gives 0, 127.5, 255 and 63.75. The truth file
contains exactly those four triples and no LUT evaluator was added.

The ignored generated file
`corpus/data/synthetic/ct_unsigned_16.dcm` was read with pydicom. It declares
Explicit VR Little Endian and `PresentationLUTShape = INVERSE`. Its SHA-256 is
`dd732b865d40814d0cbcdd5ff05ca8ea00bf3d191ffd164e0a6af4d7cf520cd1`,
which is the staged manifest digest. The generator fixture that reads the
written file passed.

The source-precedence browser fixture contains conflicting top-level, shared
and per-frame declarations. It proves per-frame wins, and a separate fixture
proves shared data crosses the sidecar boundary. The scope-only standing
mutation reaches `synthetic__ct_multiframe_perframe`, fails at
`metadata-truth`, and attributes the candidate.

The report redaction unit supplied nested strings, a nested number and signed
zero. None survived serialization. Python's mismatch-only self-test exercised
real-row redaction for scalar, string, list and null shapes. The cross-language
comparison tests distinguish missing from null, positive zero from negative
zero, shorter arrays, reordered arrays and scalar from one-element array.

I deliberately weakened four repaired guards and reverted every mutation:

- accepting fewer than four display samples made
  `truncating_the_c11_truth_is_refused` fail,
- ordinary Python numeric equality made the signed-zero self-test fail,
- assigning a shared plus one-sided failure to its one side made the mixed
  attribution test fail,
- ignoring `sourcePointer` made the changed-scope test fail.

The escalated native oracle gate rendered all 92 corpus rows twice. It wrote 90
stack views and 9 volume reformats, then reported 99 views as 71 pass, 0 fail,
28 unmeasured and 0 absent. Both aggregate render hashes were
`413e030d7b202d5a274e85c725e70b53e40e154ed1beea3041f052b254e1c603`.
All 28 standing mutations were detected. The first seven are the F-013
metadata mutations, including scope-only and positive inversion cases.

`bin/ocelli.sh gate corpus` verified 92 manifest rows with no missing or
mismatched files. `bin/ocelli.sh gate corpus-tests` passed 19 corpus checks and
50 generator tests. The oracle Node suite passed 223 tests. The Rust oracle
suite passed 81 unit tests plus 8 geometry, 4 metadata, 5 render-hash, 2
symmetry, 10 tolerance and 14 VOI-divergence fixture tests. `check`, `clippy`
and `fmt` passed for `ocelli-oracle`. The staged `content`, `prose`,
`provenance` and `deviations` gates passed.

F-X012's SIGMOID generator fixture and all tolerance tests remain green. No
tolerance file changed. The LLD contributor index matches the F-013 additions,
the corpus and comparator counts match the generated run, and the approved
plan records the seven implementation corrections. No implementation,
verification state, ledger, commit, integration or push was changed by this
review.
