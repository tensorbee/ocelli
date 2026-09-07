# F-014 review, pass 2

**Reviewed**: current fully staged F-014 tree after pass 1 remediation
**Reviewer**: independent agent, did not write the implementation or remediation
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, Fixture-name presence does not bind the literals to the assertion

**Where**: `scripts/quirk_check.py:177`

**What**: `test_uses_names` accepts every `ast.Name` anywhere in the named
method. It does not establish that those values reach the formula or assertion.
A passing test can read all six declared constants into an irrelevant tuple,
assert only that the tuple exists, and still validate as complete.

**Why it is wrong**: Pass 1 required the registry values to be mechanically
bound to the fixture literals and their actual test. The approved plan says the
checker refuses evidence that has parted company from the tree. HLD section
27.2 R2 requires the fixture to derive its answer from the specification.
Counting unrelated name reads leaves the expected-value test free to prove
nothing while the permanent-capture gate stays green.

**Evidence**: In a temporary copy I replaced
`SigmoidFixture.test_selected_display_values_follow_the_sigmoid_formula` with
a body that assigns all six symbols to `observed_but_unchecked` and asserts
only `assertIsNotNone(observed_but_unchecked)`. `validate_document` returned
`[]`, reported as `irrelevant-live-reads ACCEPTED []`.

The repair needs a check over the literal data flow into the formula and
comparison, or an executable checker-owned mutation that changes each fixture
input or expected value and requires the named test to fail for its fixed
reason. Mere presence in the method is not a runtime link.

### D2, A case can declare paths that it never writes

**Where**: `scripts/quirk_check.py:126` and `scripts/corpus_synth.py:420`

**What**: `function_binds_quirk_path` proves only that the case writes the
value at `case["paths"][0]`. The registry and `QUIRK_CASES` may agree on two or
more paths even though the generator ignores every path after the first. Each
unwritten path is then accepted if another generator already owns a manifest
row for it.

**Why it is wrong**: Approved approach steps 1 and 3 require the exact paths
produced by one generator recipe. Equality between two declarations does not
prove the named case produces every declared output. This is the disconnected
generator-to-manifest evidence that pass 1 D2 required the remediation to
close.

**Evidence**: In a temporary copy I changed the SIGMOID `QUIRK_CASES.paths`
tuple and registry array to contain both
`synthetic/ct_sigmoid_width_half.dcm` and
`synthetic/cr_monochrome1.dcm`. Both have generator-owned manifest rows. The
case still writes only `paths[0]`, but `validate_document` returned `[]`,
reported as `extra-unwritten-path ACCEPTED []`.

The repair can make the generator-owned contract singular for this workflow,
or mechanically prove that every declared path is consumed by a write in the
named case.

### D3, The approved test plan describes the superseded mutation mechanism

**Where**: `.claude/plans/F-014-design.md:162`

**What**: The mutation row still says the standing proof lives in F-X012's
existing tests plus quirk-check probes. Those probes validate registry
presence. They do not apply a mutation. Pass 1 remediation instead introduced
`scripts/quirk_mutations.py`,
`scripts/quirk_mutation_boundaries.py`, their unit suite, a non-floor gate and
an unconditional CI invocation. None appears in the plan's test location, and
the new tracked files are absent from its anticipated write set.

**Why it is wrong**: `.claude/commands/implement-feature.md` makes the approved
plan authoritative on decisions and verifiable on facts. The microscope rule
treats a false factual claim in a plan as a defect. The mutation is now active
because of the new executable gate, not because the location still recorded in
the plan performs it.

**Evidence**: `bin/ocelli.sh gate quirk-mutations` printed all three `RED`
lines and exited zero. `bin/ocelli.sh gate guards` showed the quirk-check
probes remove mutation rows and expect the schema checker to refuse them. It
did not execute the three mutations through those probes.

Update the test table and write set to record the review-mandated executable
mechanism. This is an evidence correction, not a redesign of its contract.

## Smells

None beyond the blocking defects above.

## Nitpicks

None.

## Verified clean

- All four pass 1 record-only counterexamples are now refused. Changed display
  values, unrelated expectation inputs and fixture test, an unrelated manifest
  path, and the invented single mutation contract each produced named errors.
- `bin/ocelli.sh gate quirk-mutations` applied all three fixed edits in a
  disposable repository. Function and width failed with their exact recorded
  messages. Attribution failed with `left: Fail`. Both unmodified boundaries
  were green first.
- The checker-owned mutation rows are exact, no executable command is taken
  from JSON, and removal of each one has a separate guard probe.
- `python3 -B -m unittest discover -s scripts/tests -p 'test_quirk*.py'`
  passed all 25 tests. `python3 scripts/quirk_check.py` passed.
- `bin/ocelli.sh gate corpus-tests` passed 19 manifest tests and 50 generator
  tests with no skips. The named independent fixture ran.
- The five display values were recomputed directly from PS3.3
  C.11.2.1.3.1 with centre 40 and width 0.5. They match the tracked literals
  exactly.
- `bin/ocelli.sh gate ci guards prose content backlog deviations` passed. CI
  proved all 26 floor gates are invoked and proved the non-floor
  `quirk-mutations` gate runs on pull requests, pushes and manual dispatches.
  The content gate found no staged patient data or generated DICOM.
- `git diff --cached --check` passed.
- The staged change adds no Rust cast, pixel arithmetic implementation, unsafe
  block, wasm boundary, render-loop allocation, trait, generic parameter or
  dynamic dispatch. Tiers A, B and C remain not applicable.
