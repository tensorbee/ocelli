# S34 review, pass 1

**Reviewed**: Range `5c07ef9f15f399bd9ab64dc6e8a11f6eda95ff73..HEAD` on `sprint/s34`. HEAD is `9e3d356bba96518109b4090c1d0f4d6b1e1181ed` (F-093). Tree is `fb309332042bbc640b6a61872dd4efa70c2412d6`. `git status --short` was empty before this pass and empty again after the three restores, before this file existed. This review file is not part of that tree.

**Author / reviewer**: `git log` names Atul Sharma on every commit in the range. The author of the implementation is the parent implementation agent. I am independent subagent `01a19e69-c906-7121-b833-1a4edab085b0`. I did not write the open, the design, the feature diffs, or the story reviews. I did not resume an earlier reviewer. This pass is not a self-review.

**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

The approved plans are the contract. A missing HLD formula was not treated as specified. Cornerstone behaviour was not treated as specified.

F-089. In `packages/core/src/intersection.ts` the leaving assignment was changed from `t1 = t` to `t1 = 1`. `npm test -- packages/core/src/intersection.test.ts` exited 1. The viewport clip received `x1` 12 and expected `x1` 10. `x0` stayed 0 and both y values stayed 4. The other seven tests passed. The saved blob was copied back. `cmp` matched. sha256 returned to `af45373302f92d8fe0785fbf8515d863831e66bddde394e179ec43003eb4c6c6`.

F-092. In `packages/core/src/crop.ts` the step was changed from `(lastX - originX) / DRAG_SCALE` to `(originX - lastX) / DRAG_SCALE`. `npm test -- packages/core/src/crop.test.ts` exited 1. A 256 pixel drag of normal `(0, 0, 1)` received `[0, 0, -1]` and expected `[0, 0, 1]`. The left drag received `[0, 0, 1]` against `[0, 0, -1]`. The finite overflow normal received the negated maximum. Three tests failed and three passed. The saved blob was copied back. `cmp` matched. sha256 returned to `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`.

F-093. In `packages/core/src/sculpt.ts` the maximum test was changed from `sample > best` to `sample >= best`. `npm test -- packages/core/src/sculpt.test.ts` exited 1. `mipIndex([1, 5, 5, 2])` received 2 and expected 1. The other five tests passed. The saved blob was copied back. `cmp` matched. sha256 returned to `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8`.

One mutation was in place at a time. Each restore used the blob saved before that edit. After every restore the tree was still `fb309332042bbc640b6a61872dd4efa70c2412d6` and status was empty. The four-file run after those restores exited 0 with 24 passed.

## Evidence

| Command | Identity | Executed now | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --short`, `git rev-parse HEAD^{tree}` | Branch `sprint/s34` before any edit | Yes | Exit 0. HEAD `9e3d356bba96518109b4090c1d0f4d6b1e1181ed`. Status empty. Tree `fb309332042bbc640b6a61872dd4efa70c2412d6` |
| `git diff --name-only` for the review range | 23 paths. Forbidden names checked by path | Yes | Exit 0. None of the eleven forbidden paths are in the diff |
| Pre-edit `shasum -a 256` of the three modules | Compared with the three expected hashes | Yes | All three matched before the first edit |
| Contributing F-ID line against the public-api row | `docs/lld/public-api.md` header and the last cell of the `public-api.md` row in `docs/lld/README.md` | Yes | The two strings are equal, both length 223, and both end in F-093 |
| Added markdown and the five commit messages | Fences, table rows, and inline code excluded from the semicolon scan | Yes | 15 markdown files. No em-dash. No prose semicolon. Commit messages clean. Changelog bullets clean |
| F-089 leaving bound forced to 1, then the intersection test | Only `intersection.ts` differed from its blob | Yes | Exit 1. Vitest 4.1.11. Received `x1` 12 against 10. 7 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `intersection.ts` | Yes | `cmp` exit 0. Hash `af45373302f92d8fe0785fbf8515d863831e66bddde394e179ec43003eb4c6c6`. Tree unchanged. Status empty |
| F-092 drag sign flipped, then the crop test | Only `crop.ts` differed from its blob | Yes | Exit 1. Received `[0, 0, -1]` against `[0, 0, 1]`. 3 failed and 3 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `crop.ts` | Yes | `cmp` exit 0. Hash `4f145f47e977fb2ee0effbe785e94811fb899c802d50fb75e3b56fe13d133340`. Tree unchanged. Status empty |
| F-093 `sample > best` changed to `sample >= best`, then the sculpt test | Only `sculpt.ts` differed from its blob | Yes | Exit 1. Received 2 against 1. 5 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `sculpt.ts` | Yes | `cmp` exit 0. Hash `12bcf3bcabb54f3322f91be62d0eb4b757f237e3c47f6e0ab455dea575bf0cc8`. Tree unchanged. Status empty |
| Four-file `npm test` on the restored modules | Intersection, crop, sculpt, and the published index | Yes | Exit 0. Vitest 4.1.11. 4 files passed. 24 tests passed |
| Node probe of the restored modules | `node --experimental-strip-types`. No file written in the repo | Yes | Exit 0. The checks named below printed `PROBE_OK` |
| Three `shasum -a 256`, `git status --short`, `git rev-parse HEAD^{tree}` | After the clean run and the probe | Yes | Hashes match the three values above. Status empty. Tree unchanged |

## Defects

None.

## Smells

None.

## Nitpicks

### N1. Two crop titles do not name the finite overflow normal

File: `packages/core/src/crop.test.ts` lines 21 and 43.

The planes case is titled as a refusal of seven planes, a zero normal, and a non-finite component. The expect at lines 27 to 29 refuses normal `[Number.MAX_VALUE, 0, 0]`. That component is finite. Its square is not. The drag case is titled as a refusal of a zero normal and a non-finite result. The expect at lines 45 to 49 accepts that same normal, from origin 0 and a 256 pixel drag, as `[Number.MAX_VALUE, 0, 0]`. Both assertions lock the plan. Neither title says so.

## Verified clean

`sliceIntersection` clips one caller segment with Liang-Barsky. The viewport runs from 0 to the width and from 0 to the height. The edges are inside. A zero delta returns null. A miss returns null. A probe of the left edge from `(0, 1)` to `(0, 3)` returned that same segment. A parallel line at `x` -0.5 returned null. A vertical segment from `(3, -4)` to `(3, 2)` on a 10 by 8 view returned `(3, 0)` to `(3, 2)`. A segment from `(8, 2)` to `(14, 2)` returned `(8, 2)` to `(10, 2)`. The reverse of the AC1 segment returned `(10, 4)` to `(0, 4)`, so the parameter order is kept. The module has no import. It does not project. `index.ts` does not export the intersection `Segment`. The diff does not add a `Segment` export. The file still has the earlier `export type { Segment } from "./crosshair.js"`, and that line is outside this diff. The runtime published list does not name `Segment`.

`segmentationIntersection` walks in order, drops a miss, and refuses the whole call on one non-finite coordinate. A probe of an inside segment, a miss, and a second inside segment returned the two inside segments. A list of only a miss returned an empty list. A `NaN` after a valid segment returned null. An empty list with width 0 returned null. An empty list on a positive viewport is the tested empty list.

`overlayGrid` draws interior lines only. Counts must be integers of at least 1. A probe of 3 columns and 1 row on a width of 9 returned vertical lines at `x` 3 and `x` 6 and no horizontal line. One by one is the tested empty list. A fractional count returns null.

`cropPlanes` copies 0 to 6 planes. It refuses a longer list, a non-finite component, and a squared length that is not finite or is not greater than 0. A normal `(3, 4, 0)` was copied. A normal of `Number.MAX_VALUE` was null. An empty list returned a new array, not the caller array. The module does not call `set_clip_planes`. That name appears only in the comment that says it does not. There is no dot product and no `Math.hypot`.

`cropDrag` adds `normal * (lastX - originX) / 256`. `DRAG_SCALE` is 256. The normal is not normalised. The function does not read a squared length, so a finite square is not a reason to refuse. A probe of normal `(3, 4, 0)` and a 256 pixel drag returned `(3, 4, 0)`. A zero normal returns null. A non-finite component returns null. A non-finite result returns null. The kept side stays in F-062.

`sculptVertex` adds the raw pixel delta and does not divide by 256. At least three points are required. The index must be an integer inside the list. A probe of four points, index 2, and delta `(1, 1)` moved `(0, 1)` to `(1, 2)`. The caller point stayed `(0, 1)`. A short list, an outside index, and a non-finite point return null.

`eraseNearest` names the nearest mark at a squared distance no greater than the squared radius. The comparison is `distance < bestDistance`, so an equal distance keeps the earlier id. A mark on the radius was kept. A mark past the radius was null. A farther mark placed before two marks at equal distance did not replace the earlier of those two. The function returns an id. It does not delete the mark. A non-finite coordinate, a non-string id, an empty id, or a radius that is not greater than 0 refuses the call.

`mipIndex` returns the earlier index of the maximum finite sample. The comparison is `sample > best`. A probe of `[5, 4, 5]` returned 0. A probe of `[-5, -1, -3]` returned 1. An empty list and one `NaN` return null. The samples are caller numbers. The module does not read a frame, does not compute a polygon area, and does not edit an annotation.

`index.ts` exports `overlayGrid`, `segmentationIntersection`, `sliceIntersection`, `cropDrag`, `cropPlanes`, `eraseNearest`, `mipIndex`, and `sculptVertex`. `PUBLISHED` names those same runtime values. The four-file run includes that list. `CropPlane` is a type on `crop.ts` and is not a runtime published name. `VERSION` is the literal `0.1.0`. `packages/core/package.json` is `0.1.0` and is not in the sprint diff. The workspace `Cargo.toml` version line is `0.1.0`.

There is no `unsafe` and no `wasm-bindgen` in the three modules. They do not open a device and they do not return pixels. Tiers A, B, and C are not branched. The plans and the as-built rows mark all three n/a.

The three feature commits carry `profile=feature` verify trailers. Their tree prefixes are `be0ac0c73b49`, `c48864dbd52a`, and `fb309332042b`, and those prefixes are the trees of those commits. None of the feature messages carries a comparison report. That absence matches this profile. The trailers were read. The 27 gates were not re-run. A trailer is not proof of behaviour.

The workspace version stays 0.1.0. `CHANGELOG.md` under Unreleased still says nothing is published. Its three new bullets name the vertex move, the earlier mark, and the earlier maximum, then the six-plane copy and the 256 pixel drag, then the slice clip, the dropped miss, and the interior grid. `CURRENT_SPRINT.md` says closing this sprint completes M10 and does not publish. The sprint table marks F-089, F-092, and F-093 done. The backlog marks those three done. F-087, F-088, F-090, and F-091 are done, so the seven M10 rows are done. F-062, F-076, and F-088, the dependencies named by this sprint, are done. F-094 stays pending. F-X003 stays `in-progress`. Backlog story rows are 119 done, 93 pending, and 1 in progress, which is 213. D-22 is still the named publication block. The carried S33 identities match the tag and the merge. Tag `s33` is `b1c32477` and peels to `5c07ef9`. That merge's second parent is `8fe1642`, whose tree is `f9d348a6c699`.

The sprint diff does not edit `pointer.ts`, `annotation.ts`, `tools.ts`, `commands.ts`, `events.ts`, `cursor.ts`, `crosshair.ts`, `manipulation.ts`, `orientation.ts`, `magnify.ts`, or `crates/ocelli-render/src/clip.rs`. It does not edit `compare.json`, `DEVIATIONS.md`, a tolerance, or the wasm size budget.

## Limitations and handoff

The sprint gate was not run. No cargo, wasm-pack, or `bin/ocelli.sh` run was started. The four-file vitest run is the shell suite for these modules. A diagonal that is not dyadic was not probed. The approved AC1 ends are exact, and the leaving-bound mutation moved `x1` from 10 to 12. The node probe did not write a file in the repo.

This file is untracked until the parent stages it. A pass with no defect and no smell is not sprint closure.

dwv, Horos, and the medical viewer Grok were not opened.
