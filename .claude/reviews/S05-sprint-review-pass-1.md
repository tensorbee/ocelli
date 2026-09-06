# S05 sprint review, pass 1

**Reviewed**: complete sprint diff `7c5e29c..7977444`
**Reviewer**: independent agent, did not write the sprint implementation
**Result**: 6 defects, 0 smells, 0 nitpicks

## Defects

### D1, The sprint design commit has no provenance or verification trailer

**Where**: commit `ddfbcc5`, `S05, approve sprint designs`

**What**: The commit that approved both sprint plans carries neither an
`Ocelli-Verify` trailer nor an `Ocelli-Generated-By` trailer.

**Why it is wrong**: HLD section 27.2 R6 requires a provenance trailer on
every commit. `CLAUDE.md`, `AGENTS.md` and `.claude/WORKFLOW.md` make the
pre-commit hook and verification ledger the only permitted source of those
trailers. This commit changed tracked plans, so it is not an empty marker that
could make no provenance claim.

**Evidence**:

```text
$ python3 scripts/verify_ledger.py check-commit ddfbcc5 --require-corpus
FAIL: ddfbcc5 carries no Ocelli-Verify trailer.
Every commit records how it was verified (HLD 27.2 R6), and
with no GPU in CI this trailer is the only evidence CI has
that the corpus ran at all (DEVIATIONS.md D-04).
```

The other three S05 commits passed the same check and carried tree-matching
corpus records.

### D2, The verification ledger accepts a comparison report with failed views

**Where**: `scripts/verify_ledger.py:88`

**What**: `comparison_evidence` accepts `fail > 0` as long as `pass + fail`
equals `claimedVerdictViews` and the report asserts `gateVerdict: pass` and
`green: true`. It also ignores a non-empty `coverageProblems` array. A
contradictory report that says one view failed while the run passed is
therefore recorded as green candidate evidence.

**Why it is wrong**: The approved F-012 plan requires the ledger to refuse a
red or malformed report. HLD section 11 requires an actual comparison within
the written tolerance, not a report whose self-declared summary contradicts
its per-run counts. This becomes the CI-side candidate attestation when
F-X021 enables `--require-comparison`, so accepting the contradiction defeats
the evidence contract before activation.

**Evidence**:

```text
$ python3 -c '<write operation=gate, pass=0, fail=1,
  claimedVerdictViews=1, gateVerdict=pass, green=true, absent=0, then call
  comparison_evidence>'
{'reportSha256': '486733e3dde61acce5785557df355a9a9e13cdee13b136ea4461436eba7abfff',
 'claimedVerdictViews': 1, 'verdict': 'pass'}
```

A second controlled report with a non-empty `coverageProblems` array was also
accepted and returned `verdict: pass`. The standing probes construct only
internally consistent summaries, so neither contradiction is covered.

### D3, A mixed invalid-input run is mislabeled as a comparison failure

**Where**: `tools/oracle/src/report.rs:569` and
`tools/oracle/src/bin/ocelli-compare.rs:394`

**What**: `gate_verdict` checks failed pixel outcomes before input and
structural problems. A run with an input digest disagreement on one view and a
failed pixel predicate on another is reported as `comparison-failure`, even
though the two sides were not compared under identical inputs. The same
`problems` vector also receives the text for every failed view, so it cannot
currently distinguish comparison failures from refusals after the fact.

**Why it is wrong**: HLD section 11 compares the same study through both
stacks. The approved F-012 plan and `docs/lld/comparator.md` say input or
structural problems report `refusal`, while predicate failures report
`comparison-failure`. Calling a mixed invalid comparison a measured pixel
failure makes a claim the input rung explicitly says cannot be made.

**Evidence**: A disposable repository added one focused integration test that
constructed a `RunReport` with `Outcome::Fail` and the input problem `input
digest disagrees`. The expected refusal failed as follows:

```text
test review_mixed_input_problem_is_a_refusal ... FAILED
assertion `left == right` failed
  left: ComparisonFailure
 right: Refusal
```

The existing tests cover isolated comparison, coverage and refusal states but
not their precedence when more than one state is present.

### D4, Candidate gate output may delete either input directory

**Where**: `tools/oracle/src/bin/ocelli-compare.rs:152` and
`tools/oracle/src/bin/ocelli-compare.rs:571`

**What**: `validate_gate_directories` proves only that reference and candidate
resolve to different locations. It does not compare `--out` with either input
or reject an output directory that contains them. `write_output` then calls
`remove_dir_all(out)` before writing the report. A valid-looking invocation
such as `ocelli-compare gate --reference REF --candidate CANDIDATE --out REF`
deletes the reference evidence after the comparison has read it. An ancestor
output can delete both inputs.

**Why it is wrong**: F-012 makes this the production candidate-evidence
command and documents all three directory arguments together. Destruction of
the evidence being attested is wrong command behavior and can leave the report
writer silently skipping difference frames because its later frame reads no
longer exist.

**Evidence**: Argument parsing stores `--out` at lines 123 to 129. The only
gate validation at lines 152 to 176 canonicalizes and compares reference with
candidate. The unconditional removal at lines 578 to 580 is reached before
the difference-frame loop rereads either side.

### D5, The recorded production-regression failure signature is never observed

**Where**: `corpus/quirks.json:59`, `scripts/quirk_check.py:38` and
`scripts/quirk_mutations.py:34`

**What**: The quirk record names `bin/ocelli.sh gate oracle` as its production
regression and records the failure signature `the SIGMOID divergence is
attributed to the reference only for the declared monochrome inversion`.
Nothing executes that command under controlled damage and nothing emits that
signature. The executable harness runs a focused generator boundary for two
mutations and one Rust unit test for the attribution mutation. It compares
those narrower outputs with three different mutation signatures.

**Why it is wrong**: The approved F-014 plan says each named regression must
fail at its declared boundary. `corpus/README.md` tells an operator to record
the signature observed when the mutation makes the regression red. HLD
section 11 requires the bug to become a permanent fixture, and a production
regression row held only by duplicated string equality is not active evidence
that the named regression detects the bug.

**Evidence**:

```text
$ rg -n '<the production signature>' .
scripts/quirk_check.py:42
scripts/tests/test_quirk_check.py:77
corpus/quirks.json:62
```

`bin/ocelli.sh gate quirk-mutations` did drive all three controlled mutations
and all six fixture-literal mutations red, but its output contained only the
focused signatures `expected SIGMOID, found LINEAR`, `expected width 0.5,
found 1.0`, `SIGMOID expectation fixture mismatch` and `left: Fail`. The
production-regression signature was never exercised.

### D6, The CI floor documentation still describes three excluded gates

**Where**: `scripts/ci_floor_check.py:190`

**What**: The module documentation says there are three excluded gates and
calls `corpus` and `guards-deep` the other two after `oracle`. F-014 added
`quirk-mutations` to `NOT_IN_FLOOR`, making four excluded gates and three
non-GPU exclusions.

**Why it is wrong**: Microscope review treats a false factual sentence as a
defect. This paragraph exists to explain why each non-floor gate is or is not
run by CI. Omitting the newly excluded gate from that explanation recreates
the exact stale-list problem the surrounding comments say the checker is
designed to prevent.

**Evidence**:

```text
scripts/ci_floor_check.py:190: The three excluded gates ...
scripts/ci_floor_check.py:193: The other two are excluded ...
scripts/ci_floor_check.py:458:
NOT_IN_FLOOR = {"oracle", "corpus", "guards-deep", "quirk-mutations"}
```

The executable check itself is correct. It reported all 26 floor gates in CI
and separately proved the three non-GPU exclusions run on pull requests,
pushes and workflow dispatch.

## Smells

None beyond the blocking defects above.

## Nitpicks

None.

## Verified clean

- `git diff --check 7c5e29c..7977444` passed.
- `python3 scripts/staged_content_check.py --tracked` found no patient data,
  tracked DICOM or build artefacts.
- `python3 scripts/prose_check.py` passed over 183 files.
- `python3 scripts/deviation_check.py` resolved all 17 declared deviations.
- `python3 scripts/backlog_check.py` found 211 F-IDs, 30 done stories and
  consistent sprint records.
- `python3 scripts/ci_floor_check.py` proved all 26 floor gates are invoked by
  CI and that all four floor exclusions match the runner.
- `python3 scripts/quirk_check.py` accepted the one complete quirk record.
- `bin/ocelli.sh gate quirk-mutations` made all three controlled mutations and
  all six fixture-binding mutations red for their fixed focused signatures.
- A clean-build focused Rust run in a new target directory passed
  `an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ`.
- The complete Rust and Python diff adds no `unsafe`, Rust `as` cast,
  `wasm_bindgen` use, `queue.submit()`, dynamic dispatch, trait or generic
  parameter. No render-loop or tier path is added by either story.
- The five SIGMOID fixture values agree with the PS3.3 C.11.2.1.3.1 formula
  for centre 40, width 0.5 and display range 0 to 255. The fixture remains
  independent of both renderers.
- The candidate command requires explicit distinct reference and candidate
  directories. Identity output cannot satisfy the ledger's operation check.
  Judged views remain exactly `pass + fail`, with unmeasured and absent views
  named separately.
- The quirk checker binds the current singular generator path to
  `QUIRK_CASES`, the generator-owned manifest row and the ignored DICOM
  location. It binds the registry expectation to six AST literals and the
  fixture test to the executable expectation boundary. Coordinated vacuous
  fixture changes are refused by the active literal mutations.
