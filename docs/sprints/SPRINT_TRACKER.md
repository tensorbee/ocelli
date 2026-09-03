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
