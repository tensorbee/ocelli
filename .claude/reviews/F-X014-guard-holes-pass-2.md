# F-X014 review, pass 2

**Reviewed**: remediated staged tree against `72eab18`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the focused handoff grammar unit suite is not run by any gate

**Where**: `scripts/tests/test_sprint_workflow.py`, `bin/ocelli.sh:209-226`

**What**: The remediation adds four focused tests covering valid plain and
code-span values, absent fields, eight malformed value shapes and duplicate
fields. No gate names this test module. The `guards` arm runs three explicitly
named Python suites, and repository search finds no other runner for
`test_sprint_workflow.py`.

**Why it is wrong**: The remediated plan names the focused unit tests as part
of the proof for the exact field grammar. The catalogue probes cover important
representatives, but do not cover every independent parser condition. A test
that only passes when invoked manually is not regression protection.

**Evidence**: In a disposable copy, removing only
`value.startswith("`")` from the production code-span condition made
`test_malformed_spans_are_refused` fail on the unmatched closing form
`012345` followed by a backtick. All nine handoff catalogue probes remained
green in the same mutant, including eight refusal probes and the valid
backticked accept probe. The ordinary `guards` and `guards-deep` gates also do
not discover this file because their Python suite lists are explicit. Add
`test_sprint_workflow.py` to the chained `guards` arm so this production
mutation makes the required gate fail.

## Smells

None.

## Nitpicks

None.

## Pass-1 remediation verified

- `handoff_field` now requires each field exactly once. It accepts non-empty,
  backtick-free plain text or one complete non-empty code span and rejects
  unmatched, embedded, multiple and empty span shapes.
- Branch validation runs on the unwrapped value and retains the exact
  `work/<fid>-` prefix check.
- The catalogue has refusal probes for a missing `Files touched` field,
  multiple spans, an unmatched span, an embedded span, an empty span, forged
  suffix content and a duplicate field. It retains the wrong-branch refusal
  and valid backticked-branch accept probe.
- Removing the inner-backtick condition made the multiple-span probe fail.
  Widening the length condition made the empty-span probe fail. Removing the
  duplicate-count condition made the duplicate-field probe fail.
- `docs/lld/README.md` now includes F-X014 in both the benchmarks and guards
  rows. The plan includes that index in its exact write set and `## LLD
  impact` list.

## Full feature verified clean

- The explicit nine-crate `no_std` set still compares declarations in both
  directions. Its losing-crate, gaining-crate and recorded-set mutation probes
  passed.
- G-02 and G-04 remain absent. `DEFECTS` is empty.
- The census reports 636 refusal sites across 61 files, 246 probes, 0
  uncovered sites and `sweep_complete: true`.
- Benchmark runner coverage remains registered in both exact suite lists. The
  focused runner and path tests passed 18 of 18.
- The generated runbook matches the catalogue and all 20 generated skill
  adapters match their canonical sources.
- The paired moved-crate-root probes retain the structural proof. The refusal
  catches a group allow in the moved root, and the legal-layout accept probe
  uses the derived `cargo target root(s) seeded` fragment.
- `bin/ocelli.sh gate nostd bench skills guards guards-deep` passed. The floor
  harness drove 159 refusals red with 24 accepts and 38 controls green. The
  deep harness drove 208 refusals red with 38 accepts and 41 controls green.
- The benchmark gate passed 78 tests with one expected opt-in browser skip.
- The prose, staged-content, deviation, CI-floor and staged-diff checks passed.
- No patient data, unsafe code, tolerance change, wasm boundary change, pixel
  transfer, LUT logic or render-loop allocation was introduced.
