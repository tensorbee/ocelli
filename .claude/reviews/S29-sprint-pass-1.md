# S29 review, pass 1

**Reviewed**: `f5e9bbcadc3201a5aa3cfcf3a22d482405f571a7...f7c842d96fb701ec08c1a78c3c2bd79d2e40e0d5` on `sprint/s29`. HEAD is `f7c842d96fb701ec08c1a78c3c2bd79d2e40e0d5`. HEAD tree is `e61b6ec38144100ce5d961596413fd5801ab66d9`. The worktree was clean. This pass edited no product file.
**Author / reviewer**: The story trailers record `Ocelli-Generated-By: grok`. A generated trailer does not name a person. This reviewer did not write the commits, the plans, or the story reviews.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The HLD does not define ROI mean, population standard deviation, histogram bins, livewire cost, ETDRS radii, ultrasound direction, or a redaction mask. The approved plans are the contract. This pass did not invent a DICOM formula for those numbers.

F-075. Samples 1 and 3 with area 4 are mean 2 and population sd 1. An empty list is `Degenerate`. One sample has sd 0. Samples 0, 0.5 and 1 in two bins are counts 1 and 2, and 0.5 is the higher bin. Equal samples stay in bin 0. SUV at elapsed 0 is 140 and at one half-life is 280. Units other than BQML and a zero weight are `Unavailable`. The area is not recomputed. The same edge walk that accepts the two-bin fixture does not put every maximum in the last bin. That is D1.

F-077. Costs 1, 100, 1, 1 from `(0, 0)` to `(1, 1)` go through `(0, 1)`. The mask is 1, 0, 1, 1. A seed is one point. A negative cost is `Degenerate`. An empty path is `Degenerate`. A fractional index is `Degenerate`. Equal costs keep the lower index, so the uniform grid goes through `(1, 0)`. `z` on the input is not stored. `polygon_area` is not called.

F-078. The horizontal shaft of length 10 has head 2.5 and the two plan barb literals. A zero shaft is `degenerate`. A blank label is `blank-text`. A key image stores `KEY` and a second call is equal. `(0, 0)` is central. `(1, 1)` is inner superior for both temporal flags. `(0, 0.5)` is inner superior. `(0, 1.5)` is outer superior. `(0, 3)` and `(4, 0)` are outside. Centre `(10, 20)` at 0.5 mm per pixel starts the outer circle at `(16, 20)`. The positive spokes are axis offsets. The flag does not move the marks. The existing layer draws the five arrow points. `annotation.ts` is not in the diff.

F-079. `(0, 0, 0)` to `(0, 2, 9)` is `PI / 2` and the square root of `4 + 81`, not 2. Identical points, including both signs of zero, are `Degenerate`. The row-major left column of `1, 2, 3, 4` becomes `0, 2, 0, 4`. A second pass reports already filled. A rectangle at `(5, 5)` sets `applied` false. The buffer `1, 2, NaN, 4` returns `Frame` and does not replace the first sample. `distance` is not replaced.

The four stories do not call one another. `measure.rs` is unchanged from the F-076 completion hash. The drawing layer is unchanged.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff --name-status` and `git write-tree` | Range above. Clean worktree | Executed now | 29 paths. Tree `e61b6ec38144100ce5d961596413fd5801ab66d9`. No `measure.rs`, `annotation.ts`, `DEVIATIONS.md`, oracle parameter file, or wasm budget file |
| `git rev-parse` of the four story trees | Commit messages name `0623deb81f5b`, `a25ffcb4324e`, `5d69505201c4`, and `e61b6ec38144` | Executed now | Full trees match those prefixes |
| `shasum -a 256` of `stats.rs`, `livewire.rs`, `redact.rs`, `marks.ts`, and `measure.rs` | HEAD | Executed now | The four completion notes match. `measure.rs` is still `d111418fc11027fea712520cd542625cc45415c41827599856d5279e573c240a` |
| Python float64 walk of the F-075 edge loop | Same adds and `>=` as `stats.rs` lines 217 to 221. Not a Rust call | Executed now | Minimum `10000000` and maximum `10000000 + 1e-8` in 8 bins stop at index 5. The last index is 7. Width is not 0. Samples 0, 0.5 and 1 in 2 bins still put 1 in the last bin |
| Story-review cargo and vitest runs | Pins in the F-075, F-077, F-078, and F-079 reviews | Prior receipts. Not run again | Those receipts report the named fixtures green and the named mutations red. This pass did not repeat them |
| Backlog rows F-073 through F-079 | `docs/sprints/BACKLOG.md` | Executed now | Seven rows, all `done`. Estimates 5, 5, 4, 5, 5, 3 and 3 sum to 30 |
| `ci/wasm-size-budget.json` | Not in the diff | Read now | 2151196 bytes, tolerance 0.05. The carried ceiling 2258755 is `int(2151196 * 1.05)` |

Environment: macOS, zsh, Python 3 float64. No `bin/ocelli.sh` command, no cargo process, and no wasm-pack. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

### D1, a histogram maximum can miss the last bin

**Where**: `crates/ocelli-geom/src/stats.rs:217` and the claim at lines 161 to 162. The plan rule is `.claude/plans/F-075-design.md:60`.
**What**: After a non-zero width is accepted, each sample starts at `min + width` and then adds `width` until the sample is below the edge or the index is the last bin. For minimum `10000000`, maximum `10000000 + 1e-8`, and 8 bins, the maximum stops at index 5. Index 7 is empty. The two samples are not an equal-value histogram, because the width is not 0 and the walk moves.
**Why it is wrong**: The approved plan says the maximum belongs to the last bin. The function comment repeats that sentence. This is the local rule. It is not an HLD formula. The two-bin fixture uses width `0.5`, which does not drift, so `an_interior_edge_belongs_to_the_higher_bin` stays green for a walker that already fails the maximum rule on the other input.
**Evidence**: The Python walk above. At index 5 the running edge is already greater than the maximum, so the `>=` test stops. The same walk on samples 0, 0.5 and 1 with 2 bins still returns the last index for the maximum. No Rust process was started. The loop body is only addition and `total_cmp`, and these values are ordinary positive finite numbers, so `>=` matches `is_ge`.
**Scope and regression**: Present on HEAD. `roi_statistics` and `suv_bw` do not use this walk. A later ROI tool that histograms a narrow range at a large magnitude reports the peak in an earlier bin. The smallest regression is this 8-bin pair. The shipped stats tests do not contain it.

## Smells

None.

## Nitpicks

None.

## Verified clean

`roi_statistics` divides the sum and the squared deviations by the count. Samples 1 and 3 produce mean 2 and sd 1. An empty list returns `Degenerate` before a mean of 0. One sample takes the zero-variance path. The area argument is stored and `rectangle`, `ellipse`, `circle`, and `polygon_area` are not called. `suv_bw` is `activity * weight_kg * 1000 / decayed_dose` with `decayed_dose = dose * 2 ^ (-elapsed / half_life)`. It does not call the modality LUT. Non-finite inputs return `Frame` before the unavailable checks. A false BQML flag, a non-positive weight, dose, or half-life, and a negative elapsed time return `Unavailable`. `SuvError` numbers 402 and 403 match `number`. No stats test calls `number`.

`livewire_path` is 4-connected Dijkstra. The start distance is 0, so the start cost is not added. A candidate replaces a parent only when its cost is strictly smaller. The next unsettled pixel is the lowest index at the lowest finite cost. Start equal to goal returns one point before search. `livewire_mask` rejects an empty path before it allocates, then marks visited pixels and does not fill an interior. Returned points store `z` 0. There is no diagonal. The new files do not name `polygon_area` or `distance`.

`arrowMark` stores start, end, the positive barb, end, and the negative barb. Head length is `min(12, length / 4)`. The length-10 barbs match the plan literals under `Math.cos` and `Math.sin` of `PI / 6`, which the F-078 pass already evaluated outside `marks.ts`. Refusal order is blank id, blank text, non-finite point, then a zero shaft. `labelMark` and `keyImageMark` follow the same applicable order. `etdrsRegion` uses 0.5, 1.5 and 3 mm, and a boundary belongs to the outer side. `(1, 1)` stays superior. `etdrsMarks` draws 33-point circles and four axis spokes from 0.5 mm to 3 mm. The temporal flag does not move them. `marks.ts` does not build SVG and does not import a second drawing layer. The sync test reads the existing polyline attribute.

`ultrasound_direction` uses `DVec2::to_angle`, which in glam 0.30.10 is `atan2(y, x)`, and `DVec3::length` for the span. `z` is in the length only. A zero length, including both zero bit patterns, is `Degenerate`. It does not call `distance`, and `distance` still returns 0 for a point and itself. `redact_rect` classifies the intersection, scans it, and only then writes. A non-finite sample in that scan returns `Frame` with no assignment. A miss or a zero side returns `applied` false and does not write. `same_finite` matches `==` for finite values, including the two zeros, and the negative-zero test keeps the stored bits. The blocking `NaN` in the plan is the third sample of `1, 2, NaN, 4`.

`lib.rs` re-exports the new names and still exports one `distance`, one `polygon_area`, and one `length`. The diff has no `unsafe` and no `wasm_bindgen`. Tier rows in the four plans are n/a. None of these functions open a device. CHANGELOG describes the four behaviours and does not say this sprint publishes. The version is not edited. `CURRENT_SPRINT.md` says closing S29 completes M8 and does not publish, and the carried wasm sentence matches the budget file. AS_BUILT test counts are 10, 7, 9 and 11, which match the test functions. The recorded sha256 values match HEAD. F-079's plan correction is the square root of `4 + 81` and the third-sample `NaN`. Both are what the test locks.

## Limitations and handoff

This pass is not sprint verification and not a close. M8 is not closed by this file. Nothing was published. The suite was not run again. D1 was not executed inside `histogram`. It was executed as the same floating-point operations. A one-line Rust fixture of that pair is the regression to add.

Not executed here: a vertical arrow, a shaft longer than 48 pixels, the 12 pixel cap as the selected head, an interior circle sample, the `c3` radius in pixels, a non-blank arrow string through `sync`, `SuvError::number`, a pure `z` ultrasound span, and a `NaN` outside a redaction rectangle. The length-10 arrow cannot select the cap of 12, so changing only that cap would leave the marks tests green. The cap is present at `marks.ts` line 98. That is uncovered behaviour, not a false claim, and it is not filed. The F-078 type names are still absent from the older type parenthetical in `index.test.ts`. That parenthetical was already incomplete, and the value-export list does include the six runtime names.

The next owner fixes the histogram walk so the maximum lands in the last bin, with a regression the current two-bin fixture does not cover. Do not widen a tolerance and do not edit the wasm budget. Do not publish from this sprint.
