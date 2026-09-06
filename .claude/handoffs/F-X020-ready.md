# F-X020 ready, protect hand-curated sprint plans

**Branch**: work/f-x020-codex
**Base**: 1e186479476431f98b1e711f0584df6aa2d80293
**Head**: 8d80b9ec8acab645795d0c0e2b0fe53db5914aa5, the reviewed and verified feature commit. The branch tip is one handoff-only commit above it because this file cannot name the commit that contains it.
**Review**: `.claude/reviews/F-X020-sprint-plan-guard-pass-1.md` found 1 defect, which was remediated. `.claude/reviews/F-X020-sprint-plan-guard-pass-2.md` is clean with 0 defects, 0 smells and 0 nitpicks.
**Verify tree**: 751014e7e61e619362d544044e7557befb78c9d0, feature profile with all 25 floor gates and `corpus`, recorded with `corpus=pass` and `Ocelli-Generated-By: codex`.

## What landed

`scripts/gen_sprint_plan.py` now treats its bare write mode as bootstrap-only.
It creates an absent plan, refuses to overwrite an existing hand-curated plan,
keeps `--check` read-only and permits deliberate replacement only through
`--force`. The refusal names the protected path and both explicit modes.

A temporary-directory suite proves all four modes and compares plan bytes where
non-mutation matters. Pass-1 review found that the new suite was not connected
to a required gate. The remediation adds it by exact file pattern to `guards`,
which is in the floor and directly invoked by CI.

The standing sprint-plan probe plants prose allocation data cannot reconstruct,
requires the bare writer to refuse it and uses `--force` as the green control.
The sprint-plan probe budget moves from 14 to 15, and the generated runbook and
living guard documentation record the new boundary.

## Files touched

- `.claude/reviews/F-X020-sprint-plan-guard-pass-1.md`
- `.claude/reviews/F-X020-sprint-plan-guard-pass-2.md`
- `bin/ocelli.sh`
- `ci/guard-probe-budget.json`
- `docs/lld/guards.md`
- `docs/runbooks/guard-verification.md`
- `docs/sprints/BACKLOG.md`
- `scripts/gen_sprint_plan.py`
- `scripts/guards/catalogue.py`
- `scripts/tests/test_gen_sprint_plan.py`

## Verification

`bin/ocelli.sh gate --floor` exited 0 with all 25 gates green:

`fmt`, `clippy`, `test`, `bindgen`, `unsafe`, `pins`, `nostd`, `errors`,
`panic`, `bench`, `provenance`, `prose`, `content`, `backlog`, `deviations`,
`skills`, `lint`, `types`, `wasm`, `native`, `device`, `packages`, `ci`,
`guards`, `corpus-tests`.

The `guards` gate ran the four new generator tests. Its standing harness drove
147 refusal probes red for their declared reason and completed 37 controls
green. The pre-existing declared G-04 handoff defect remains unrelated.

`bin/ocelli.sh gate corpus` exited 0. It verified 91 manifest rows, 44 real,
all 16 transfer syntaxes, 0 missing files, 0 digest mismatches and a clean
metadata audit. The known corpus coverage note remains: no real colour case
exercises chroma subsampling or YBR conversion.

The committed feature tree carries:

```text
Ocelli-Verify: profile=feature gates=backlog,bench,bindgen,ci,clippy,content,corpus,corpus-tests,deviations,device,errors,fmt,guards,lint,native,nostd,packages,panic,pins,prose,provenance,skills,test,types,unsafe,wasm corpus=pass tree=751014e7e61e
Ocelli-Generated-By: codex
```

## Review mutation evidence

Pass 2 temporarily changed a successful `--check` into a writer. The required
`guards` gate exited 1 specifically in
`test_check_reads_explicit_paths_without_rewriting_the_plan`, proving the new
suite contributes to the gate result. The production file was restored exactly
and the clean gate passed afterward.

The actual tracked `SPRINT_PLAN.md` also retained SHA-256
`c4b737b75d9aa4f243a23322d690a9daa12a2cb386f310ecd15146c20baad94b`
before and after the bare command refused it.

## Integrator notes

This prepare pass does not update `AS_BUILT.md`, `SPRINT_TRACKER.md`, sprint
totals, sprint state or the backlog row to `done`. Those remain integrator
work. Nothing was pushed or integrated.
