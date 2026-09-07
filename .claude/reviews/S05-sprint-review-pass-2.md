# S05 sprint review, pass 2

**Reviewed**: complete remediated sprint diff `7c5e29c..17902da`
**Reviewer**: independent agent, did not write the sprint implementation or pass 1 remediation
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, The ledger still accepts contradictory and incomplete green comparison reports

**Where**: `scripts/verify_ledger.py:88`

**What**: The pass 1 repair rejects a nonzero `fail` count and a non-empty
`coverageProblems` array, but it still accepts other report states that the
comparator itself classifies as red. A report with a non-empty `problems`
array, a non-empty `absorbedDivergences` array, or top-level `absent: 1` is
accepted when the duplicated summary fields say pass. The parser also accepts
a report that omits `coverageProblems` entirely because line 119 treats
`None` as equivalent to an empty array.

**Why it is wrong**: The approved F-012 plan requires malformed or red reports
to be refused. `RunReport::gate_verdict()` makes `problems`, absorbed
divergences and absent views run-level failures. `RunReport::to_json()` always
emits `coverageProblems`, so omission is not a valid green report shape. The
ledger digest becomes the candidate attestation when F-X021 enables
`--require-comparison`. Accepting mutually contradictory copies of the
run-level state lets a report modified after comparison be recorded as green.

**Evidence**:

```text
$ python3 <controlled calls to comparison_evidence over four temporary reports>
non-empty problems: ACCEPTED pass views=1
non-empty absorbed divergences: ACCEPTED pass views=1
omitted coverageProblems: ACCEPTED pass views=1
top-level absent contradicts coverage: ACCEPTED pass views=1
```

Each report had `operation: gate`, `gateVerdict: pass`, `green: true`, one
passing view, zero failed views, and `coverage.absent: 0`. The four named
changes were applied one at a time. In the first, second and fourth cases the
state contradicts `tools/oracle/src/report.rs:569-580`. In the third case a
required field emitted by `RunReport::to_json()` was absent.

The two exact pass 1 examples are repaired. Controlled reports with
`fail: 1` or a non-empty `coverageProblems` array now exit through the new
refusals, and the standing guard catalogue covers both. The defect is the
remaining equivalent contradictions, not a recurrence of those two examples.

### D2, The delivery record calls an open object validator a closed schema

**Where**: `docs/sprints/AS_BUILT.md:1920` and
`scripts/quirk_check.py:285-532`

**What**: The completion record says F-014 built a closed-schema quirk
registry. The checker validates required values but does not reject unknown
keys at the document, quirk, intake, generator, expectation, authority or
fixture levels. Only the checker-owned regression and mutation objects are
closed by whole-object equality.

**Why it is wrong**: A closed schema rejects properties outside its declared
vocabulary. Microscope review treats a false AS_BUILT statement as a defect.
This distinction is material for a permanent intake record because a later
author can add data outside the reviewed evidence graph while the `quirks`
gate still reports the capture complete. Either the allowed keys need to be
enforced at each declared object level, or the completion claim needs to state
the narrower contract actually built.

**Evidence**:

```text
$ python3 <validate four copies of corpus/quirks.json with one extra key>
top-level field: []
quirk field: []
intake field: []
expectation field: []
```

In every case `validate_document()` returned no errors. Running
`bin/ocelli.sh gate quirks` on the unmodified tree still passed all 34 tests,
and none of those tests asks the validator to refuse an unknown property.

## Smells

None beyond the blocking defects above.

## Nitpicks

None.

## Verified clean

- All five commits in `7c5e29c..17902da` passed
  `scripts/verify_ledger.py check-commit --require-corpus`. The rewritten
  design commit `a3b450c` now carries both required trailers and its tree
  matches the recorded verification, resolving pass 1 D1.
- `bin/ocelli.sh test ocelli-oracle` passed 81 library tests, 9 comparator
  binary tests, 8 coverage tests and all 43 other oracle integration and
  fixture tests. The mixed input-problem plus pixel-failure case reports
  `refusal`, resolving pass 1 D3.
- The comparator argument tests independently exercised output equal to an
  input, output below an input and output containing both inputs. All three
  were refused. The resolver canonicalizes the nearest existing ancestor for
  a not-yet-existing output and `write_output()` repeats the validation before
  cleanup, resolving pass 1 D4.
- `bin/ocelli.sh gate quirks quirk-mutations ci guards` passed. The live quirk
  harness made all three controlled mutations and all six fixture-binding
  mutations red. The registry regression command and failure signature equal
  the executable attribution mutation, resolving pass 1 D5.
- The CI checker now states four floor exclusions and proved all 26 floor
  gates plus the three non-GPU exclusions on pull requests, pushes and manual
  dispatch. Its exclusion set agrees with the runner, resolving pass 1 D6.
- The SIGMOID fixture values were recomputed independently from PS3.3
  C.11.2.1.3.1 using centre 40, width 0.5 and output range 0 to 255. The five
  results reproduce the tracked literals, including 127.5 at the centre.
- `git diff --check 7c5e29c..17902da` passed. The tracked-content, prose,
  deviation and backlog checks passed with no patient data or tracked DICOM,
  17 resolved deviations and consistent records for all 211 F-IDs.
- The Rust diff adds no `unsafe`, `wasm_bindgen`, `queue.submit()`, dynamic
  dispatch, trait or generic parameter. Neither story adds a render-loop or
  tier-specific execution path, and no tolerance changed.
