# F-014 review, pass 1

**Reviewed**: current staged F-014 working tree before this review artefact
**Reviewer**: independent agent, did not write the implementation
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, The recorded expectation can part company from its fixture

**Where**: `scripts/quirk_check.py:205`

**What**: The checker proves only that an expectation has a non-empty input
object, a numeric value array, and the name of an existing test. It does not
prove that the recorded inputs and expected values are the literals exercised
by that test. A record naming an unrelated existing test, an unrelated input,
and the expected value `999` is accepted as complete. Replacing the real five
expected values with `[0.0]` is accepted too.

**Why it is wrong**: The approved plan says the checker refuses a record whose
evidence has parted company from the tree, and approach steps 1 and 2 require
the independently written expected-value fixture to be part of the evidence
graph. HLD section 27.2 R2 requires tests to derive from the specification or
oracle. Merely proving that some test-shaped symbol exists does not connect
that independent evidence to the registry values. The claims in
`corpus/README.md`, `docs/lld/corpus.md`, and the `quirks` gate description are
therefore false about this link.

**Evidence**:

```text
$ python3 -c '<load valid_record, replace expectedDisplayValues with [0.0], validate>'
[]

$ python3 -c '<load valid_record, name SigmoidFixture.test_stored_values_cross_the_window_centre, replace inputs with {"unrelated": 999} and values with [999], validate>'
[]
```

The repair needs one mechanically checkable source of these literals. For
example, record the fixture constant symbols and compare their AST literal
values with the registry, or remove the duplicated values from the registry
and validate an equally exact fixture-owned representation. A symbol-existence
check alone cannot satisfy the plan.

### D2, The declared generator case is not linked to its declared paths

**Where**: `scripts/quirk_check.py:142`

**What**: The checker separately proves that the named function exists and is
called somewhere by `generate`, then proves that every declared path is some
generator-owned manifest row. It never proves that the named case writes those
paths. The SIGMOID record remains valid when its only path is changed to the
unrelated `synthetic/cr_monochrome1.dcm` row.

**Why it is wrong**: Approved approach steps 1 and 3 require one generator
recipe, its exact manifest paths, and generator-to-manifest linkage. A future
rename, split, or copy can silently leave a quirk record pointing at bytes its
recipe never creates while `bin/ocelli.sh gate quirks` stays green. This is
the exact disconnected-evidence defect the story exists to prevent.

**Evidence**:

```text
$ python3 -c '<load valid_record, replace generator.paths with ["synthetic/cr_monochrome1.dcm"], validate>'
[]
```

The checker should derive the output path or paths of the named case from the
parsed generator source and compare that exact set with the record. A shared
generator-owned declarative mapping is another valid design if both generation
and checking consume it without importing or executing registry commands.

### D3, Mutation evidence is accepted without proving any mutation

**Where**: `scripts/quirk_check.py:246`

**What**: For a known mutation kind the checker accepts any replacement value,
any non-empty failure signature, and any existing test name of the broad
language-specific shape. It does not require the three recorded mutations to
remain present, does not apply the replacement, and does not compare the
observed failure with the recorded signature. One mutation with the string
value `not-a-width`, an unrelated existing SIGMOID test, and the signature
`invented` replaces all three real rows and validates as complete.

**Why it is wrong**: Approved approach step 6 requires all three F-X012
controlled mutations as standing evidence and says each named regression must
fail at its declared boundary for its declared reason. The plan's mutation test
row requires the function, width, and attribution mutations to make the worked
regression red. Checking that a label belongs to a closed vocabulary and that
some function name exists does not establish active mutation evidence. The
`bin/ocelli.sh` description currently claims exactly that stronger property.

**Evidence**:

```text
$ python3 -c '<load valid_record, replace mutations with one generator-window-width row whose value is "not-a-width", boundary is test_stored_values_cross_the_window_centre, and signature is "invented", validate>'
[]
```

The checker needs kind-specific, checker-owned semantics. It must validate the
allowed replacement type and target, require the intended evidence set, apply
each known mutation through fixed code rather than a command from JSON, run
the fixed boundary, and compare the observed red reason with the record. The
guard catalogue should carry one probe per standing mutation so removal of any
one is observed independently.

## Smells

None beyond the blocking defects above.

## Nitpicks

None.

## Verified clean

- `python3 -B -m unittest discover -s scripts/tests -p test_quirk_check.py`
  passed all 13 tests.
- `bin/ocelli.sh gate quirks` passed its checker and all 13 tests.
- `bin/ocelli.sh gate guards` passed. Its five new quirk probes each drove the
  guard red for the declared reason, the guard census reported 680 refusals in
  63 files with none watched by nothing, and the complete gate exited zero.
- `bin/ocelli.sh gate ci` proved all 26 floor gates are invoked by CI for
  pull requests and pushes. The new `quirks` gate is in that floor and has its
  own workflow step.
- `python3 scripts/staged_content_check.py --tracked` reported no patient data
  or build artefacts staged. The tracked-DICOM quirk probe also drove the new
  checker red at its own boundary. The registry contains only the synthetic
  recipe's non-sensitive constants and no field input or identifier.
- `python3 scripts/prose_check.py` passed over 180 files.
- `git diff --cached --check` passed.
- The registry's five modality inputs and five expected display values were
  recomputed independently from PS3.3 C.11.2.1.3.1. They exactly equal the
  current fixture constants: `4.586483540333347`, `30.396745115639977`,
  `127.5`, `224.60325488436`, and `250.41351645966665`.
- All three inherited F-X012 mutation boundaries were rerun against a temporary
  copy of the staged tree. Changing `VOILUTFunction` to `LINEAR` made
  `SigmoidFixture.test_width_half_is_declared_as_sigmoid` fail with
  `'LINEAR' != 'SIGMOID'`. Changing `WindowWidth` to `1.0` made it fail with
  `1.0 != 0.5`. Disabling the register-effect branch made
  `an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ` fail
  with `left: Fail` and `right: Unmeasured`. The production files in the shared
  worktree were not modified.
- The staged change adds no pixel arithmetic, Rust cast, unsafe block, wgpu
  call, wasm boundary, render-loop allocation, trait, generic parameter, or
  dynamic dispatch. Tier A, tier B, and tier C remain not applicable as the
  approved plan states.
