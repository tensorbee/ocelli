# F-012 microscope review, pass 1

**Result**: pass
**Reviewer**: codex
**Date**: 2026-09-06

## Scope

- Candidate command parsing and distinct-directory refusal.
- Run coverage accounting and verdict precedence.
- Verification report validation, ledger storage and trailer parsing.
- Guard probes, controlled report execution and planning records.

## Questions applied

- Can identity output be recorded as candidate evidence?
- Can a failure, coverage loss or zero-view run be labelled green?
- Can missing views be printed as zero while still failing elsewhere?
- Can malformed counts, booleans or partial trailer fields pass validation?
- Does any record persist report content or a path rather than a digest and
  non-sensitive counts?
- Does the split leave the real-renderer obligation tracked and dependent on
  the first story that supplies every current oracle view kind?

## Findings

Zero open defects and zero open smells.

The pre-review implementation check found and corrected two contract gaps
before this pass. The stdout absent count now uses the same coverage count as
the JSON verdict. The report now records `operation: gate`, and the ledger
refuses identity output even when every pixel and count is otherwise green.

## Evidence

- Five command-contract tests pass.
- Seven coverage tests pass.
- The full `ocelli-oracle` test suite passes.
- `bin/ocelli.sh clippy ocelli-oracle` passes with warnings denied.
- Six new ledger refusal probes are red for their declared reasons with green
  controls.
- A controlled 99-view run reports 71 judged, 28 unmeasured, 0 absent, 2
  unsupported source rows and 1 declared volume refusal.
- The ledger accepts that gate report, records its SHA-256 digest and positive
  judged count, and satisfies `--require-comparison`.
