# F-X020 review, pass 1

**Reviewed**: working tree against `1e18647`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the generator unit tests are not executed by a required gate

**Where**: `scripts/tests/test_gen_sprint_plan.py`, `bin/ocelli.sh` `guards`
arm

**What**: The feature adds four deterministic unit tests for absent-plan
bootstrap, existing-plan refusal, forced replacement and check mode. No gate
executes that file. The `guards` arm deliberately names only
`test_guard_catalogue.py` and `test_guard_readers.py`, and repository search
finds no other runner for `test_gen_sprint_plan.py`.

**Why it is wrong**: The approved plan requires these unit proofs. A test that
only passes when invoked manually is not regression protection and violates
the review rule that tests must be connected to execution. The catalogue probe
covers the existing-plan refusal, but it does not replace the other three unit
proofs or assert the replacement contents.

**Evidence**: `rg -n "test_gen_sprint_plan" bin/ocelli.sh .github scripts`
finds only the test file itself. The `guards` arm runs two explicitly named
test modules and therefore cannot discover the new module. Add the generator
test module to an appropriate required gate, then run that gate and show that
a production mutation of forced replacement makes the gated command fail.

## Smells

None.

## Nitpicks

None.

## Verified clean

- A bare invocation creates an absent plan and refuses an existing plan before
  rendering or writing it.
- `--force` is mutually exclusive with `--check` and explicitly replaces the
  destination.
- `--check` remains read-only.
- The catalogue probe plants a hand-curated paragraph and expects the bare
  invocation to refuse it.
- The generated runbook and budget reflect the additional probe.
- No patient data, unsafe code, runtime hot-path change, wasm boundary change,
  pixel transfer, LUT logic, or tolerance change was introduced.
