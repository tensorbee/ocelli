# Sprint Tracker

Velocity log. One row per completed F-ID, written by `/complete-feature`
step 3. `scripts/backlog_check.py` refuses a `done` backlog row with no row
here, because a story marked done with no completion record is a status and
not a delivery.

## What the estimate column means, and what it does not

The `Est` column is the spreadsheet's engineer-week figure, unmodified. It was
made for a team and it is what the HLD's 397 and 352 totals are built from, so
changing it would break the only cross-check the plan has.

`Days actual` is the real measurement and it is the one that should drive
re-planning. Record it honestly, including when it is much smaller than the
estimate. Under agent-assisted delivery the ratio is the interesting number and
a plausible-looking figure written to make a row tidy destroys it.

**Never write an estimated value into `Days actual`.** Where a duration was not
measured, write `not measured`. A plausible-looking `<1d` written to fill a
cell is invented evidence, and it is worse than a gap, because a gap is
visibly a gap and an invented figure is not.

## Capacity calibration

Recalculate after S03 and again after S06, and write the result into
`SPRINT_PLAN.md` under "Capacity calibration". Do not recalculate off one
sprint. S01 to S03 are foundations and the oracle, which is not representative
of the volume port work in M2 onward.

| F-ID | Title | Sprint | Est | Days actual | Completed |
|------|-------|--------|-----|-------------|-----------|
| F-001 | Cargo workspace, crate skeleton, lint/CI baseline | S01 | 2w | 0.37d measured, 2h58m wall clock from the design commit to the F-ID commit, across four review passes | 2026-09-04 |
| F-009 | Golden corpus ingest and de-identified fixture store | S01 | 3w | 0.76d measured, 6h03m wall clock from the design commit to the F-ID commit, across seven review passes | 2026-09-04 |
| F-002 | wasm-pack build pipeline with a hard size budget gate | S02 | 2w | not measured, wall clock from the S02 design commit to this F-ID commit spans concurrent work on F-010 and is not attributable to this story | 2026-09-04 |
| F-007 | Cross-target build proof: native desktop + server binary | S02 | 2w | not measured, same reason as F-002, this lane ran beside the F-010 worker | 2026-09-05 |
| F-008 | ocelli-compute crate skeleton and GPU device-sharing contract | S02 | 2w | not measured, three review passes, and it also corrected F-007's feature guard | 2026-09-05 |
| F-003 | TS package scaffold, bundling, npm publish pipeline | S02 | 2w | not measured, two review passes | 2026-09-05 |
| F-010 | Headless cornerstone3D reference renderer | S02 | 4w | 0.31d measured, 7h29m wall clock from the design commit to the worker commit, across thirteen review passes and a strategy change, then integrated and reviewed once more | 2026-09-05 |
