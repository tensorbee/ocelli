# S05 sprint review, pass 4

**Reviewed**: complete remediated sprint diff `7c5e29c..12ad1a5`
**Reviewer**: independent agent, did not write the sprint implementation or pass 3 remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, Candidate evidence is not bound to the report records it claims to attest

**Where**: `scripts/verify_ledger.py:88-163`,
`scripts/guard_probe.py:169-188` and
`scripts/guards/catalogue.py:861-881`

**What**: `comparison_evidence()` now validates the summary fields found by
passes 1 through 3, but it never validates the per-view `records` array or the
remaining emitted report shape. It accepts a report with `records` missing,
null or empty while recording 71 judged views. It also accepts duplicated
records, duplicate record identifiers, and a record changed from `pass` to
`fail` while the summary still says zero failures. The accepted summary counts
are not related to `views` or to record outcomes, so `views: 0` with 71 claimed
views and `pass: 1000` with 1,000 claimed views over 99 records both attest as
green.

The same parser accepts reports that the comparator cannot emit for other
structural reasons. Examples include equal reference and candidate directory
strings, missing `reference` or `candidate`, missing or invalid aggregate
render hashes, a qualifier histogram inconsistent with the records, unknown
top-level and coverage properties, and duplicate JSON member names. The guard
harness's green controls omit `story`, `reference`, `candidate`, `views`,
`qualifiers`, `renderHashes` and `records`, so the controls themselves are not
reports `RunReport::to_json()` can produce.

**Why it is wrong**: The approved F-012 plan requires the ledger to refuse a
malformed report. HLD section 11 requires frames to be compared, and
`docs/lld/comparator.md:1087-1093` defines `compare.json` as the whole report
carrying every view's result and hashes. A digest of a file with no records is
not evidence that the claimed views were compared. In the production
serializer, `views` is the record count, `pass`, `fail` and `unmeasured` are
derived from those records, and `claimedVerdictViews` is `pass + fail`.
`ocelli-compare gate` also refuses identical resolved input directories before
it can write a report. The ledger currently checks duplicated summaries while
ignoring their source, so a post-comparison edit can replace the evidence and
retain a green attestation.

**Evidence**: A genuine green report was first produced by running
`ocelli-compare gate` over two distinct copies of `tools/oracle/out`. It
contained 99 records, 71 passes, 28 unmeasured views, zero failures and zero
absent views. One mutation was then applied at a time before calling
`comparison_evidence()`:

```text
ACCEPTED missing records: pass 71
ACCEPTED records null: pass 71
ACCEPTED records empty: pass 71
ACCEPTED duplicate whole record: pass 71
ACCEPTED record outcome contradicts counts: pass 71
ACCEPTED duplicate record id: pass 71
ACCEPTED missing views: pass 71
ACCEPTED zero views with 71 claimed: pass 71
ACCEPTED claimed and pass exceed views: pass 1000
ACCEPTED reference equals candidate: pass 71
ACCEPTED missing renderHashes: pass 71
ACCEPTED invalid aggregate hash: pass 71
ACCEPTED qualifiers contradict records: pass 71
ACCEPTED unknown top-level field: pass 71
ACCEPTED unknown coverage field: pass 71
ACCEPTED duplicate JSON fail key: pass 71
```

`tools/oracle/src/report.rs:650-684` mechanically derives all of those
relationships from the records and emits one fixed top-level object. None of
the accepted mutations above can be emitted by that function as a green run.

## Smells

None beyond the blocking defect above.

## Nitpicks

None.

## Verified clean

- Every commit in `7c5e29c..12ad1a5` passed
  `scripts/verify_ledger.py check-commit --require-corpus`. The rewritten
  design commit still carries both required trailers and a matching tree, so
  sprint pass 1 D1 remains resolved.
- The exact contradictory summaries from sprint passes 1 and 2 remain
  refused. Missing, malformed or non-empty `problems`, `coverageProblems` and
  `absorbedDivergences` arrays fail. Both absent counts must be integer zero,
  and a nonzero failure count cannot be called green.
- The pass 3 coverage repair was exercised as a 16-case mutation matrix.
  Missing, null, boolean, negative and string forms of
  `coverage.unmeasured`, `coverage.unsupportedSourceRows` and
  `coverage.declaredVolumeRefusals` all failed. A disagreement between the
  top-level and coverage unmeasured counts also failed. Sprint pass 3 D1 is
  resolved at its stated boundary.
- `bin/ocelli.sh gate --floor` exited 0 over all 26 floor gates. This includes
  the complete Rust suite, all 35 quirk contract tests, CI equivalence,
  formatting, clippy, content, provenance, prose and the floor guard harness.
  The mixed input-problem and pixel-failure test still reports `refusal`, so
  sprint pass 1 D3 remains resolved.
- The comparator command tests still refuse output equal to an input, output
  below an input and output containing both inputs. Validation is repeated
  immediately before cleanup, so sprint pass 1 D4 remains resolved.
- `bin/ocelli.sh gate quirk-mutations` made all three controlled mutations and
  all six fixture-binding mutations red for their fixed signatures. The
  regression command remains bound to the executed attribution mutation, so
  sprint pass 1 D5 remains resolved.
- Unknown fields were injected independently at all 13 current quirk object
  locations. Registry, quirk, intake, generator, expectation, authority,
  fixture, symbols, inputs, regression and all three mutation objects were
  refused. Sprint pass 2 D2 remains resolved.
- The CI floor checker still states four exclusions and proves the 26 floor
  gates plus every non-GPU exclusion on the declared events. Sprint pass 1 D6
  remains resolved.
- `git diff --check 7c5e29c..12ad1a5` passed. The sprint diff adds no new
  `unsafe`, `wasm_bindgen`, `queue.submit()`, dynamic dispatch, trait or
  generic parameter. It changes no tolerance and adds no render-loop or tier
  path.
