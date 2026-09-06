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

## Capacity calibration, and why there is nothing to calibrate from

The plan was to recalculate after S03 and again after S06, off the `Days
actual` column, and not off one sprint, because S01 to S03 are foundations and
the oracle rather than the volume port work of M2 onward.

**S03 produced no measurement to recalculate from.** All seven of its rows read
`not measured`, each with the reason recorded in the cell: three stories ran
concurrently in worker worktrees whose agents terminated on a session rate
limit, two ran beside each other with one story's oracle runs inside the other's
wall clock, one overlapped the sprint review's remediation, and one ran serial
in the canonical worktree with several full oracle runs inside its wall clock.
Most rows in this file read `not measured` and the measured ones are all S01 and
S02. No count is transcribed into this sentence, because the table below is the
count:

```bash
grep -E '^\| F-' docs/sprints/SPRINT_TRACKER.md | grep -c 'not measured'
```

**This section used to direct the reader to write the result into a "Capacity
calibration" section of `SPRINT_PLAN.md`. That section does not exist**, and
naming a destination that was never created is how an instruction survives
without ever being followed. Nothing is written there until a sprint produces at
least three measured rows, at which point the section is created by the change
that has something to put in it.

| F-ID | Title | Sprint | Est | Days actual | Completed |
|------|-------|--------|-----|-------------|-----------|
| F-001 | Cargo workspace, crate skeleton, lint/CI baseline | S01 | 2w | 0.37d measured, 2h58m wall clock from the design commit to the F-ID commit, across four review passes | 2026-09-04 |
| F-009 | Golden corpus ingest and de-identified fixture store | S01 | 3w | 0.76d measured, 6h03m wall clock from the design commit to the F-ID commit, across seven review passes | 2026-09-04 |
| F-002 | wasm-pack build pipeline with a hard size budget gate | S02 | 2w | not measured, wall clock from the S02 design commit to this F-ID commit spans concurrent work on F-010 and is not attributable to this story | 2026-09-04 |
| F-007 | Cross-target build proof: native desktop + server binary | S02 | 2w | not measured, same reason as F-002, this lane ran beside the F-010 worker | 2026-09-05 |
| F-008 | ocelli-compute crate skeleton and GPU device-sharing contract | S02 | 2w | not measured, three review passes, and it also corrected F-007's feature guard | 2026-09-05 |
| F-003 | TS package scaffold, bundling, npm publish pipeline | S02 | 2w | not measured, two review passes | 2026-09-05 |
| F-010 | Headless cornerstone3D reference renderer | S02 | 4w | 0.31d measured, 7h29m wall clock from the design commit to the worker commit, across thirteen review passes and a strategy change, then integrated and reviewed once more | 2026-09-05 |
| F-004 | Runtime capability detection and tiering | S03 | 2w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-005 | Error model, panic-to-JS mapping, structured logging | S03 | 2w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-X006 | Answer Appendix A gates A1 and A2 against our own decoders | S03 | 3w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-006 | Benchmark harness: decode, first frame, interaction latency | S03 | 2w | not measured, it ran concurrently with F-X007 in a worker worktree and the wall clock covers the other story's oracle runs | 2026-09-05 |
| F-X007 | Oracle volume and MPR reference renders | S03 | 3w | not measured, it ran concurrently with F-006 and its wall clock includes several full oracle runs plus one remediation round after the integrator's review | 2026-09-05 |
| F-011 | Pixel-diff comparator with per-modality tolerance policy | S03 | 3w | not measured, it ran serial in the canonical worktree and its wall clock includes several full oracle runs | 2026-09-05 |
| F-X009 | A standing test for every repository guard | S03 | 3w | not measured, it ran concurrently with the sprint review's remediation and its wall clock includes waiting on that | 2026-09-05 |
| F-X017 | Close the measured escapes from the wasm linear memory view ban | S04 | 2w | not measured, work overlapped F-013 verification and included repeated full floor runs | 2026-09-06 |
| F-X018 | Bind sprint review and verification evidence to the current tree | S04 | 1w | not measured, work ran serially during a whole-sprint execution with repeated floor probe and gate runs | 2026-09-06 |
| F-X019 | Require CI gates to run or make their step fail | S04 | 1w | not measured, work ran serially during a whole-sprint execution and included exhaustive bash cross-checks plus full floor runs | 2026-09-06 |
| F-013 | Metadata diff harness for LUT, geometry and spacing | S04 | 2w | not measured, worker implementation was replayed and followed by repeated oracle and full-gate runs | 2026-09-06 |
| F-015 | Stable render-hash emission from the comparator | S04 | 2w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X001 | Tier C and feature-availability contract | S04 | 4w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X008 | One parity target and published wasm licences | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X010 | Structural CI floor equivalence | S04 | 2w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X012 | Measured SIGMOID reference divergence | S04 | 2w | not measured, worker ran concurrently and used the serial oracle resource | 2026-09-06 |
| F-X013 | Price the HTJ2K decoder route | S04 | 3w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X014 | Close declared guard holes | S04 | 2w | not measured, worker ran concurrently and included repeated mutation profiles | 2026-09-06 |
| F-X015 | Execute marked skill examples | S04 | 1w | not measured, worker ran concurrently and included repeated mutation profiles | 2026-09-06 |
| F-X016 | Adapter fallback after device-open failure | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X020 | Protect the hand-curated sprint plan | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-012 | Candidate comparison gate contract and verification plumbing | S05 | 3w | not measured, the wall clock includes sprint replanning, controlled corpus-scale comparisons and repeated full floor runs | 2026-09-06 |
