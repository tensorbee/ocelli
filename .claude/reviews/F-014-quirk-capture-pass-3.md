# F-014 review, pass 3

**Reviewed**: current fully staged F-014 tree after pass 2 remediation
**Reviewer**: independent agent, did not write the implementation or remediation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- All pass 1 and pass 2 counterexamples are now refused. The checker rejects
  changed expectation inputs or outputs, an unrelated fixture test, an
  unrelated manifest path, an invented or missing controlled mutation, and a
  regression whose kind, command or failure signature differs from its
  checker-owned contract.
- The named fixture unittest consists of one call to the checker-owned
  `sigmoid_expectation_boundary`. Redirecting only the registry boundary is
  refused. A coordinated redirect of the checker constant, registry, fixture
  path and named test to a no-op boundary made the static checker green, but
  the executable gate refused it because the first fixture-literal mutation
  stayed green. The static and executable halves therefore cannot both accept
  a vacuous fixture boundary.
- `bin/ocelli.sh gate quirk-mutations` passed on the healthy staged tree. The
  function and width mutations failed with their exact recorded messages, the
  attribution mutation failed with `left: Fail`, and independent mutations of
  modality values, centre, width, display minimum, display maximum and expected
  display values each failed with `SIGMOID expectation fixture mismatch`.
  Both the generator, fixture and attribution controls were green before their
  mutations.
- The fixture-mutation contract requires exactly the six checker-owned symbol
  targets. Each edit is an assignment to its claimed symbol in the
  checker-owned fixture file, invokes the shared boundary and carries the
  fixed failure signature. Unit probes refuse target, edit and invocation
  drift. Controlled mutation probes separately refuse changed kinds,
  replacement values and failure signatures.
- `function_binds_quirk_path` requires exactly one direct `write()` in the
  named case and requires that write to consume `QUIRK_CASES[case]["paths"][0]`.
  Adding an undeclared second write was refused. Coordinately declaring two
  valid manifest paths was refused because the initial contract is singular.
  Coordinately redirecting the registry and `QUIRK_CASES` to another existing
  path made the static checker green, but the executable generator control
  failed because the declared SIGMOID output was absent. The combined evidence
  therefore does not accept a disconnected generator path.
- `python3 -B -m unittest discover -s scripts/tests -p 'test_quirk*.py'` ran
  33 tests with no failures or skips. This is 25 checker tests and 8 executable
  harness tests. `bin/ocelli.sh gate quirks ci prose content` passed. CI proves
  the 26 floor gates run on pull requests and pushes, and separately proves
  `quirk-mutations` runs on pull requests, pushes and manual dispatches.
- The five display values were recomputed directly from PS3.3
  C.11.2.1.3.1 as `255 / (1 + exp(-4 * (x - 40) / 0.5))`. They exactly match
  `4.586483540333347`, `30.396745115639977`, `127.5`,
  `224.60325488436` and `250.41351645966665` for the five declared modality
  inputs.
- The approved plan now names `scripts/quirk_mutations.py`,
  `scripts/quirk_mutation_boundaries.py`, `scripts/tests/test_quirk_mutations.py`
  and the non-floor `quirk-mutations` CI gate. Its anticipated write set
  contains the executable harness files, and its dependency note correctly
  distinguishes the GPU-exclusive oracle from the CPU-only mutation gate.
- `git diff --cached --check` passed. The content gate found no patient data or
  generated DICOM. The change adds no pixel arithmetic implementation, Rust
  cast, unsafe block, wasm boundary, render-loop allocation, trait, generic
  parameter or dynamic dispatch. Tiers A, B and C remain not applicable.
