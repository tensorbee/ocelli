# F-088 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s33` at HEAD `43a894b91abb878b68e722f78540588524fd3d7e` (`S33, approve the manipulation designs`). Paths read: `.claude/plans/F-088-design.md`, `packages/core/src/crosshair.ts`, `packages/core/src/crosshair.test.ts`, the F-088 paragraph and F-ID header in `docs/lld/public-api.md`, the crosshair exports in `packages/core/src/index.ts`, the matching `PUBLISHED` names in `packages/core/src/index.test.ts`, and the `public-api.md` row in `docs/lld/README.md`. `git diff` and `git status` for `packages/core/src/annotation.ts` were empty.
**Author / reviewer**: Author is the parent implementation agent. Reviewer is independent subagent `01a19dfc-c6f1-77a1-a62a-d95a7b4a4438` and did not write the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 from the approved plan. The HLD does not define the segments, and that absence is not a defect.

AC1 was challenged by changing only the horizontal end from `width` to `width - 1` and re-running `packages/core/src/crosshair.test.ts`. The AC1 case went red at `packages/core/src/crosshair.test.ts:11`. Expected `x1` was 10 and received `x1` was 9. The vertical segment was still `(4, 0)-(4, 8)`. AC2 and AC4 also went red on that same horizontal end, which is the shared function and the same width. AC3 stayed green because it expects null. The original line was restored before this file was written.

AC2 keeps the edge point `(0, 8)` on a 10 by 8 view. Both segments stay, and `referenceCursor` returns `{ x: 0, y: 8 }`. The closed tests are `x >= 0`, `y >= 0`, `x <= width` and `y <= height`.

AC3 returns null for `(-1, 3)` and for width 0, from both `crosshairLines` and `referenceCursor`.

AC4 expects the AC1 pair from `referenceLines`, and `referenceLines` is the same function object as `crosshairLines`.

The module does not project a patient point. It does not read a frame of reference. It does not edit the annotation layer. `annotation.ts` is not in the working diff.

## Evidence

| Command / selected cases | Source and input identity | Executed now | Exit and observation |
|--------------------------|---------------------------|--------------|----------------------|
| `npm test -- packages/core/src/crosshair.test.ts` from the repo root | Untracked `crosshair.ts` hash `9c6958200e14268f7cfc9b5868fb59d8f5cee545` and `crosshair.test.ts` hash `72e64669718b80bc9c2be0e23d837bae476e5647` | Yes, before the mutation | Exit 0. 1 file passed. 4 tests passed |
| Same command after only the horizontal end became `width - 1` | Working copy of `crosshair.ts` only. No other test file was run | Yes | Vitest reported 1 file failed, 3 tests failed and 1 passed. AC1 failed at line 11 with `x1` 9 against 10. AC2 failed at line 15 with `x1` 9 against 10. AC4 failed at line 33 with `x1` 9 against 10. The outside and zero-width case passed |
| `cp` of the pre-mutation file back, then `cmp` and `git hash-object` | Backup `/tmp/f088-crosshair.ts` against `packages/core/src/crosshair.ts` | Yes | `cmp` matched. Hash returned to `9c6958200e14268f7cfc9b5868fb59d8f5cee545`. `grep` found no `width - 1`. The horizontal literal is `x1: width` |
| Same `npm test` after that restore | Restored untracked source, same two hashes as the first run | Yes | Exit 0. 4 tests passed |
| `git diff --name-only`, `git status --short` and `git diff` for `annotation.ts` | Branch `sprint/s33` | Yes | `annotation.ts` had no status line and no diff. `crosshair.ts` and `crosshair.test.ts` are untracked. Tracked F-088 hunks sit in `index.ts`, `index.test.ts`, `docs/lld/public-api.md` and the `public-api.md` row of `docs/lld/README.md` |

The red run's shell wrapper stopped on a later assignment before it echoed npm's status. The vitest summary above is the record of that run. The product file was already restored, and the later green run is the restored tree.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`inside` accepts the point only when `x`, `y`, `width` and `height` are finite, both sides are greater than 0, and the point lies in the closed viewport. `crosshairLines` then returns the horizontal segment `(0, y)-(width, y)` and the vertical segment `(x, 0)-(x, height)`, in that order. Any other input returns null. `referenceLines` is the binding `crosshairLines`, not a second geometry. `referenceCursor` uses that same test and returns `{ x, y }` or null.

A segment is `{ x0, y0, x1, y1 }`. `index.ts` exports `crosshairLines`, `referenceCursor`, `referenceLines` and the `Segment` type. `PUBLISHED` lists those three runtime names once. A type export does not appear in `Object.keys`, so the missing type name is not a publish gap.

The F-088 paragraph says the horizontal span is the viewport width at the projected y, the vertical span is the viewport height at the projected x, the edges are inside, and an outside point, a non-finite coordinate or a non-positive side returns nothing. It says `referenceLines` is `crosshairLines`, and that the functions do not project and do not read a frame of reference. Those sentences match the code. Null is the nothing the plan names. The README row names crosshair and reference lines and lists F-088. The paragraph, the row clause and the module comments have no em-dash and no prose semicolon.

There is no import in `crosshair.ts`. There is no call into `annotation.ts`, no patient point and no frame-of-reference read. F-055 is not reopened. The diff does not touch `wasm-bindgen` or `unsafe`. Tiers A, B and C stay not applicable.

The test file also refuses a NaN x, a zero height and an x past the width. Those cases agree with the plan and do not stand in for AC1.

## Limitations and handoff

This pass did not run `bin/ocelli.sh`, cargo, wasm-pack, the feature profile, any test file other than `crosshair.test.ts`, the oracle, the corpus or a GPU. It did not open dwv, Horos or the medical viewer named Grok. Other uncommitted S33 stories share `index.ts`, `index.test.ts`, `docs/lld/public-api.md` and `docs/lld/README.md`. Those stories were not reviewed. No patient data was read. The segment ends are the approved plan, not an HLD formula.

The horizontal-end mutation is not left in place. `x1` is `width` again, and `width - 1` is absent. This review file is new and was not committed. No other file was edited. A clean bounded review is not the feature profile and is not sprint readiness.
