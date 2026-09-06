# S05 sprint review, pass 3

**Reviewed**: complete remediated sprint diff `7c5e29c..a5a45bd`
**Reviewer**: independent agent, did not write the sprint implementation or either sprint remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, The ledger accepts an incomplete or contradictory coverage block

**Where**: `scripts/verify_ledger.py:136`, `scripts/guard_probe.py:177` and
`scripts/guards/catalogue.py:870`

**What**: The pass 2 repair validates that `coverage` is an object and that
its `absent` member is the integer zero. It does not require or type-check the
other three coverage counts that the comparator always emits and the candidate
evidence contract names. Reports whose `coverage.unmeasured`,
`coverage.unsupportedSourceRows` or `coverage.declaredVolumeRefusals` member is
missing, null, boolean, negative or a string are accepted. A report whose
top-level `unmeasured` count disagrees with `coverage.unmeasured` is also
accepted. Both standing green controls contain only `coverage.absent`, so the
guard harness itself relies on the incomplete shape.

**Why it is wrong**: The approved F-012 plan says the coverage object keeps
unmeasured views, absent views, unsupported source rows and declared volume
refusals in separate named counts, and says a malformed report is refused.
`docs/lld/comparator.md` records the same four-member coverage object as the
run-level candidate-evidence contract. `RunReport::to_json()` always emits all
four non-negative integer counts and emits the same unmeasured count at the top
level. Accepting a partial or contradictory copy lets a report altered after
comparison acquire a green ledger attestation even though it is not a report
the comparator can produce.

**Evidence**: A controlled matrix started from the current complete 99-view
report and presented it to `comparison_evidence()` with one field changed at a
time. For each of `coverage.unmeasured`,
`coverage.unsupportedSourceRows` and
`coverage.declaredVolumeRefusals`, the values missing, null, `false`, `-1` and
`"0"` were all accepted. Changing only top-level `unmeasured` from 28 to 0
while leaving `coverage.unmeasured` at 28 was also accepted.

The requested pass 2 reproductions are repaired. For each of `problems`,
`coverageProblems` and `absorbedDivergences`, missing, null, object, string and
non-empty-array forms were refused. Both top-level `absent` and
`coverage.absent` refused missing, null, boolean, negative, positive and string
forms. The remaining defect is the rest of the stated coverage shape and its
unmeasured accounting, not one of those repaired examples.

## Smells

None beyond the blocking defect above.

## Nitpicks

None.

## Verified clean

- Every commit in `7c5e29c..a5a45bd` passed
  `scripts/verify_ledger.py check-commit --require-corpus`. The rewritten
  design commit carries both required trailers and its tree matches, so sprint
  pass 1 D1 remains resolved.
- A green report with a failed view or a non-empty `coverageProblems` array is
  refused, so the two exact sprint pass 1 D2 examples remain resolved. The
  wider ledger defect is D1 above.
- `bin/ocelli.sh test ocelli-oracle` passed 81 library tests, 9 comparator
  command tests, 8 coverage tests and all 43 other integration and fixture
  tests. The mixed input-problem and pixel-failure case reports `refusal`, so
  sprint pass 1 D3 remains resolved.
- The comparator tests independently refused output equal to an input, output
  below an input and output containing both inputs. The validation is repeated
  before cleanup, so sprint pass 1 D4 remains resolved.
- `bin/ocelli.sh gate quirks quirk-mutations ci guards` passed. All three
  controlled quirk mutations and all six fixture-literal mutations went red
  for their fixed signatures. The registry regression equals the executable
  attribution mutation, so sprint pass 1 D5 remains resolved.
- The CI checker states four floor exclusions and proved all 26 floor gates.
  It also proved the three non-GPU exclusions run on pull requests, pushes and
  manual dispatches, so sprint pass 1 D6 remains resolved.
- Unknown-field mutations were applied to every current quirk JSON object:
  registry, quirk, intake, generator, expectation, authority, fixture,
  fixture symbols, expectation inputs, regression and each of the three
  mutation rows. All thirteen locations were refused. Direct vocabulary checks
  close the first seven. Exact checker-owned equality closes symbols, inputs,
  regression and mutation rows. Sprint pass 2 D2 remains resolved.
- The five SIGMOID display values were recomputed independently as
  `255 / (1 + exp(-4 * (x - 40) / 0.5))` from PS3.3 C.11.2.1.3.1. Every result
  exactly matches the tracked fixture literal.
- `bin/ocelli.sh gate --floor` completed green. The focused quirk and guard
  run executed 35 quirk tests, 190 red refusal probes, 26 green acceptance
  probes and 44 green controls. The tracked-content, prose, deviation and
  backlog checks also passed, with no patient data or tracked DICOM and all 17
  deviations resolved.
- `git diff --check 7c5e29c..a5a45bd` passed. The Rust and Python diff adds no
  `unsafe`, `wasm_bindgen`, `queue.submit()`, dynamic dispatch, trait or generic
  parameter. Neither story adds a render-loop or tier-specific execution path,
  and no tolerance changed.
