# F-089 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s34` at HEAD `599ddfcc28a6d4c1356d8079e08d7d8aa96275d3`. Paths read: `.claude/plans/F-089-design.md`, untracked `packages/core/src/intersection.ts` (blob `0bafb416917fff023880aa05730a1d46b553c2db`), and untracked `packages/core/src/intersection.test.ts` (blob `ae1535da7d6d7bcebd55ec10be6536a531f47b2c`). The `Segment` declaration in `packages/core/src/crosshair.ts` was read to compare the local type. `packages/core/src/index.ts` does not export this module and was not modified.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e3d-e6e5-7aa1-825b-ee076c3fbfd0`. The reviewer did not write the plan or the diff and does not resume any earlier reviewer.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC8 from the approved plan. The HLD does not define these formulas. That absence is not a defect. The approved plan is the contract.

AC1 was challenged by changing only the leaving-bound update from `t1 = t` to the constant `1`, then re-running `packages/core/src/intersection.test.ts`. The case went red at `packages/core/src/intersection.test.ts:12`. Expected `x1` was 10 and received `x1` was 12. The entering end stayed `(0, 4)`. The other seven tests stayed green. That mutation keeps the unclipped end, which is the wrong implementation AC1 names. The original bytes were restored before this file was written.

AC2 stays on the input segment `(1, 1)-(2, 2)` and on the closed corner segment `(0, 0)-(10, 8)`. The same leaving-bound mutation left both green, because neither segment needs a tighter leaving `t`.

AC3 returns null for `(-5, -5)-(-1, -1)` and for the zero delta `(1, 1)-(1, 1)`. The suite also returns null for width 0 and for a `NaN` x. A zero delta outside the viewport also returns null.

AC4 keeps the inside segment and drops the miss. A separate call with a miss, the AC1 segment, another miss, and an inside segment returns the clipped AC1 segment and then the inside segment. A non-finite coordinate on the last segment returns null, not the earlier prefix. An all-miss list returns `[]`. A zero delta is dropped and does not refuse the call.

AC5 returns `[]` for an empty list on a 10 by 8 viewport. An empty list with a non-positive side returns null. The plan says an empty list returns an empty list, and it also says a bad number refuses the call. It does not rank that pair. AC5's stated trigger is the empty list on a positive viewport. The unranked pair is not a finding.

AC6 returns the vertical `(5, 0)-(5, 8)` and then the horizontal `(0, 4)-(10, 4)`. `overlayGrid(3, 2, 12, 9)` returns verticals at x 4 and x 8, then y 4.5. `overlayGrid(4, 1, 10, 8)` returns x 2.5, 5, and 7.5. None of those results includes a border.

AC7 returns `[]` for one column and one row. `overlayGrid(1, 3, 9, 9)` returns only the two interior horizontals.

AC8 returns null for columns 2.5. The suite also returns null for column count 0 and for width 0. Rows 2.5, a negative count, `NaN`, and `Infinity` also return null. The integer value `2.0` is accepted and matches AC6.

Further clip cases, hand-computed from the plan and executed on the saved source:

- `(12, 4)` to `(-2, 4)` returns `(10, 4)` to `(0, 4)`.
- `(3, -4)` to `(3, 20)` returns `(3, 0)` to `(3, 8)`.
- `(5, 4)` to `(15, 4)` returns `(5, 4)` to `(10, 4)`.
- `(0, -2)` to `(0, 10)` returns `(0, 0)` to `(0, 8)`. The same line at x -1 returns null.
- `(0, 0)` to `(10, 0)` and `(0, 8)` to `(10, 8)` stay. A line at y -0.1 returns null.
- `(-2, -2)` to `(12, 8)` matches an independent evaluation at entering `t` 0.2 and leaving `t` 6/7, including the IEEE x value `0.8000000000000003`.
- A corner touch `(-2, -2)` to `(0, 0)` returns the point `(0, 0)-(0, 0)`. The input delta is not zero, and the edge is inside. The plan does not call that point a miss.

The module does not project. It does not read a frame of reference. It does not import `crosshair.ts`. `crosshair.ts` was not modified.

## Evidence

| Command / selected cases | Source and input identity | Executed now | Exit and observation |
|--------------------------|---------------------------|--------------|----------------------|
| `npm test -- packages/core/src/intersection.test.ts` from the repo root | Untracked `intersection.ts` hash `0bafb416917fff023880aa05730a1d46b553c2db` and `intersection.test.ts` hash `ae1535da7d6d7bcebd55ec10be6536a531f47b2c`. Node `v24.13.0`. Vitest `4.1.11` | Yes, before the mutation | Exit 0. 1 file passed. 8 tests passed |
| Same command after only the leaving update became the constant `1` | Working copy of `intersection.ts` only. No other test file was run | Yes | Exit 1. 1 file failed. 1 test failed and 7 passed. AC1 failed at line 12 with received `x1` 12 against expected `x1` 10. Received `x0` stayed 0 |
| `node --experimental-strip-types` import of `sliceIntersection`, `segmentationIntersection`, and `overlayGrid` | Same saved source, before the mutation | Yes | Exit 0. Reverse AC1, the vertical clip, the left edge, the segmentation order, the whole-call `NaN`, and the interior grid lines matched the plan arithmetic recorded above |
| `cp` of `/tmp/f089-intersection.ts` back, then `cmp` and `git hash-object` | Blob written with `git hash-object -w` before the edit | Yes | `cmp` matched. Hash returned to `0bafb416917fff023880aa05730a1d46b553c2db`. The leaving update is `t1 = t` again |
| Same `npm test` after that restore | Restored untracked source, same two hashes as the first run | Yes | Exit 0. 8 tests passed |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`clipSegment` is Liang-Barsky on x from 0 to the width and y from 0 to the height. The four edges are `(-dx, x0)`, `(dx, width - x0)`, `(-dy, y0)`, and `(dy, height - y0)`. A parallel edge with `q < 0` returns null. `q == 0` stays, so the boundary is inside. An entering `t` raises `t0`. A leaving `t` lowers `t1`. A zero input delta returns null before those edges. A non-finite coordinate or a non-positive side returns null. AC1's entering `t` is 1/7 and its leaving `t` is 6/7, and the restored points are exactly `(0, 4)` and `(10, 4)`.

`sliceIntersection` returns that clip. `segmentationIntersection` uses the same clip, in input order, and drops a null result. One non-finite coordinate returns null for the whole call. An empty list on a positive viewport returns `[]`. `overlayGrid` accepts integer counts of at least 1 and a positive finite size. Vertical lines use `width * c / columns` for `c` from 1 through `columns - 1`. Horizontal lines use `height * r / rows` for `r` from 1 through `rows - 1`. Verticals come first. One by one returns `[]`, which is not null. A non-integer count returns null.

A segment is `{ x0, y0, x1, y1 }`. This file declares that type again. There is no import. The only mention of the crosshair module is the comment that this file does not import it. The crosshair `Segment` is a separate readonly interface with the same four fields. The plan asks for the second declaration.

The test comments restate the plan's hand values for AC1 and AC6. Those values match the independent arithmetic above. The extra suite cases for a closed corner, a zero side, a `NaN` coordinate, and a zero column count agree with the plan and do not stand in for AC1. The AC1 mutation went red, so that test does not pass both ways.

There is no `unsafe` and no `wasm-bindgen`. The functions do not project and do not read a frame of reference. Tiers A, B, and C are not applicable. The functions allocate only their returned lists. They are not on a render loop. The module comments have no em-dash and no prose semicolon.

## Limitations and handoff

This pass did not run `bin/ocelli.sh`, cargo, wasm-pack, the feature profile, any test file other than `intersection.test.ts`, the oracle, the corpus, or a GPU. dwv, Horos, and the medical viewer Grok were not opened. Other untracked story files were not read. No patient data was read.

`index.ts` does not export this module. That is a later commit slice and is not a finding. The LLD paragraph named by the plan is a completion step and is not in this diff. The segment ends are the approved plan, not an HLD formula.

The leaving-bound mutation is not left in place. `cmp` matched `/tmp/f089-intersection.ts`. The hash is `0bafb416917fff023880aa05730a1d46b553c2db`. This review file is new and was not committed. No other file was edited. A clean bounded review is not the feature profile and is not sprint readiness.
