# F-X010 review, pass 1

**Reviewed**: complete staged implementation against `408c866`
**Reviewer**: independent root agent, did not write the implementation
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Behaviour verified

- A floor gate with several visible commands is covered only by a reachable
  named invocation of that gate.
- Splitting an arm into ordered steps, reversing the steps, or moving the
  commands into an independent job is refused even though all argument vectors
  remain in the workflow.
- A descriptive named step invoking the gate inside the existing area job is
  accepted.
- Single-command arms retain exact argument-vector equivalence and continue to
  refuse narrowed commands.
- The older rule for an arm containing an unextractable command remains
  independently reachable. Its reshaped probe reduces the arm to one visible
  command before expanding the named step, so F-X010's multi-command rule
  cannot mask that older defence.
- D-04's `corpus`, `guards-deep`, and `oracle` exclusions remain unchanged and
  are still compared with the runner's exclusion set.
- `--sprint` and `--all` retain both public names but share one case arm. The
  runtime test stubs `run_gate` and proves both select the full declared list
  in order.

## Independent checks

```text
git diff --cached --check
python3 -m unittest scripts.tests.test_guard_readers
python3 scripts/ci_floor_check.py
python3 scripts/guard_probe.py --only ci-floor.multi-command-split-steps
python3 scripts/guard_probe.py --only ci-floor.multi-command-reordered
python3 scripts/guard_probe.py --only ci-floor.multi-command-split-jobs
python3 scripts/guard_probe.py --only ci-floor.multi-command-named-in-area-job
python3 scripts/guard_probe.py --only ci-floor.gate-with-an-unextractable-arm-command
python3 scripts/guard_census.py --check
bash -n bin/ocelli.sh
```

The reader suite passed 75 tests. All four F-X010 boundary probes produced
their declared result, the older unextractable-arm probe remained red for its
own reason, the CI floor passed with 25 gates, and the census remained
internally consistent at 630 refusal sites.

The expanded tracked write set is justified by the measured workflow migration
and generated runbook update recorded in the approved plan. No HLD, tolerance,
GPU path, pixel path, unsafe allowance, wasm-bindgen boundary, verification
record, or commit is changed by the feature.
