# S28 sprint review, pass 1

**Reviewed**: `27c227ebe8e16a5b52ae0058686f8ed103cc0f54...e759c40dd3e4de3c72c78f57cdf633a87ef3bbb7` on `sprint/s28`. HEAD is `e759c40`. The worktree was clean before this file. The only product edit was `sum.abs() * 0.5` changed to `* 1.0` in `crates/ocelli-geom/src/measure.rs`, then restored. SHA-1 before and after is `479befbd9f0bde427206f850e5099ff158986fd2`.
**Author / reviewer**: Story commits record `Ocelli-Generated-By: grok`. A generated trailer does not name a person. This reviewer did not write the sprint and did not resume `01a19bcc`, `01a19b80`, `01a19b8d`, `01a19b98`, `01a19bac`, or `01a19bba`.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The HLD names `ocelli-geom` and Appendix B counts tool classes. Neither gives these formulas. The local plans are the contract. Cornerstone3D 5.8.2 was read only to check that the F-073 plan does not adopt that viewer's height or line angle. Those behaviours are not the shipped functions.

F-073. Length of `(0,0,0)` to `(3,4,0)` is 5. Height of `(0,0,1)` and `(5,5,4)` is 3, not `sqrt(59)`. Cobb of the x axis and `(1,1,0)` is `atan2(1, 1) = PI/4`. The reversed signed dot is `-1`, so `atan2(1, -1) = 3*PI/4`. The shipped reversed assertion still expects `PI/4`, and `.abs()` is still on the dot. Bidirectional lengths 3 and 4 stay 4 and 3 in either order. One finite point is `Degenerate` from `length` and 0 from `polyline_length`. There is no `fn angle`.

F-074. Rectangle `(0,0,0)` and `(2,3,9)` is width 2, height 3, area 6. Ellipse radii 2 and 3 are `PI * 6`, including one negative radius. A zero radius, including `-0`, is `Degenerate`. Circle `(0,0,0)` to `(3,4,0)` is radius 5 and area `PI * 25`. `ellipse(5, 5)` is the same area by the two formulas. That pair was not a Rust call.

F-076. Origin shoelace sum is 2 and half is 1. The shifted square's open ring is half of 3, which is 1.5. Spline segment 0 at `t = 0.5` is `(0.4375, -0.0625)`. Basis weights at that `t` sum to 1. The three fixture points are not colinear. The cross term in z is 1.

`length` calls `polyline_length` and does not close a ring. `polygon_area` does. The same four square corners are area 1 and an open perimeter of 3. The perimeter was not executed as a Rust call.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom` | HEAD `e759c40`. `measure.rs` SHA-1 `479befbd9f0bde427206f850e5099ff158986fd2` | executed now | exit 0. 1 unit test and 29 integration tests passed |
| `the_unit_square_has_area_one` after `sum.abs() * 0.5` became `* 1.0` | `measure.rs` only. Restored after | executed now | test status 101 at `crates/ocelli-geom/tests/measure.rs:426`. Left bits `4611686018427387904` are 2. Right bits `4607182418800017408` are 1 |
| `shasum crates/ocelli-geom/src/measure.rs` | same path after restore | executed now | `479befbd9f0bde427206f850e5099ff158986fd2` matches the pre-edit hash. `git status --porcelain` was empty |
| Python `math` | not the Rust functions | executed now | `sqrt(25) = 5`. `sqrt(59)` is not 3. `atan2(1, 1) = PI/4`. `atan2(1, -1) = 3*PI/4`. Open shifted half is 1.5. Spline midpoint is `(0.4375, -0.0625)`. Weights sum to 1 |
| `git rev-parse s27` and the S28 backlog awk | `27c227e` and `docs/sprints/BACKLOG.md` | executed now | tag `s27` is `27c227e`. F-073, F-074 and F-076 print `done` |
| `sample_world`, volume `spacing`, and `PixelSpacing` | `ocelli-volume`, `ocelli-pixel`, `ocelli-dicom`. None are in the sprint diff | read now | `spacing[0]` is `column_mm` and divides `direction.col(0)`, which is `orientation.row()` |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). One cargo process at a time. wasm-pack was not run. No patient data was used.

## Defects

### D1, short polyline length is not always 0

**Where**: `.claude/plans/F-073-design.md:22` and `.claude/plans/F-073-design.md:30`
**What**: Line 22 says fewer than two points returns 0 from `polyline_length`. Line 30 says `polyline_length` of one point is 0. A one-point list with a non-finite coordinate is `Frame`.
**Why it is wrong**: `.claude/plans/F-070-design.md:34` says one finite point is 0 and a non-finite coordinate refuses the sum. The F-073 plan says this story does not change that function. `docs/lld/geometry.md:50` and `docs/lld/public-api.md:189` keep the finite qualifier. Lines 22 and 30 do not.
**Evidence**: `polyline_length` checks `finite_point` before any sum, at `crates/ocelli-geom/src/measure.rs:121`. There is no earlier `Ok(0)`. `length` returns `Degenerate` for `points.len() < 2` at line 324 before that call. The full suite's non-finite polyline case is a longer list, so this pass did not execute the one-element `NaN`. The source has no other branch.
**Scope and regression**: Present on HEAD. The functions match the LLD. The plan does not. An early `Ok(0)` for every short list would make one `NaN` point length 0, and `a_single_point_is_not_a_length` would stay green because its point is finite.

### D2, plan swaps the probe spacing cosine

**Where**: `.claude/plans/F-074-design.md:12`
**What**: The plan says `spacing[0]` divides the column cosine and `spacing[1]` divides the row cosine.
**Why it is wrong**: `Volume` stores `spacing[0]` as `column_mm` and `spacing[1]` as `row_mm` at `crates/ocelli-volume/src/lib.rs:584`. `direction` is `from_cols(orientation.row(), orientation.column(), normal)` at line 595. `sample_world` divides `col(0)` by `spacing[0]` and `col(1)` by `spacing[1]` at `crates/ocelli-volume/src/ortho.rs:108`. Column 0 is the row cosine. DICOM PS3.3 C.7.6.2, as named in `crates/ocelli-dicom/src/frame_geometry.rs:577`, puts between-rows first and between-columns second. `PixelSpacing::new(values[0], values[1])` keeps that order. The HLD does not give this formula. The plan sentence is not a transcription of the code.
**Evidence**: Read of those three sites. `ocelli-volume` is not in the sprint diff. The same swap is the pre-existing comment at `ortho.rs:98`. This pass did not sample a volume.
**Scope and regression**: The new geometry functions do not call `sample_world`. A later probe that follows the plan instead of the code swaps row and column spacing. The smallest case is unequal row and column spacing read off both axes. A plan-only edit would leave the volume comment wrong.

## Smells

None.

## Nitpicks

None.

## Verified clean

`length` refuses fewer than two points and then calls `polyline_length`. One finite point stays 0 from `polyline_length`. `height` is `(end.z - start.z).abs()` and returns `Frame` when that value is not finite. It does not read `x` or `y`. `cobb` uses `atan2` of the cross-product length and the absolute dot through `DVec2::new(dotted, cross_len).to_angle()`. `bidirectional` orders finite distances with `total_cmp`. Identical endpoints stay 0. Rectangle width and height are absolute `x` and `y` differences, `z` is not a factor, and the area is the product. A zero side is not `Degenerate` in the source. Ellipse area is `PI * (rx.abs() * ry.abs())` after the finite check and `exact_zero`, which includes both zero bit patterns. Circle radius is `distance`, `exact_zero` is `Degenerate`, and the area is `PI * (radius * radius)`.

`polygon_area` is `sum.abs() * 0.5` on `x` and `y`, and the loop wraps to index 0. Fewer than three points is `Degenerate` before the finite check. `spline_point` is the uniform Catmull-Rom basis in the plan. The half is `let q = combined * 0.5`. Segment 0 repeats the first point. A missing following point uses `unwrap_or(end)`. `t` outside `0..=1`, including `NaN`, is `Degenerate` before a read. A segment with no next point is `Degenerate`. Freehand length is `length`. There is one `fn length` and no second length. No mean, standard deviation, SUV, histogram, SVG, or SR tree was added. `packages/` is not in the diff. `packages/core/src/tools.ts` still says a length is not computed there.

The crate is `no_std` outside tests. The integration test compiled and linked that library. New code uses `core::f64::consts::PI` and glam `libm`. It does not call `f64::sqrt`, `f64::atan2`, or `f64::mul_add`. A search found no `unsafe`, `unwrap(`, `expect(`, `panic!`, slice index, or `as` cast in `measure.rs`. `unwrap_or` at line 597 is the end ghost. Workspace lints deny `float_cmp`, `unwrap_used`, `expect_used`, and `panic`. New tests compare `to_bits` or `Result` equality. Tier rows in the three plans and in `docs/lld/geometry.md` are n/a. The functions do not open a device.

`docs/lld/geometry.md` and `docs/lld/public-api.md` match the functions above, including one non-finite point as `Frame` from `polyline_length` and `Degenerate` from `length`. CHANGELOG entries for the three stories match those functions and do not say this sprint completes M8 or changes 0.1.0. The compile line was `ocelli-geom v0.1.0`. `docs/sprints/SPRINT_PLAN.md` says M8 is S28 to S29, 7 stories, 30 engineer-weeks. The backlog rows are F-073 through F-079 and the estimates sum to 30. F-075 and F-077 are S29 and pending. F-094 is pending. F-X003 is `in-progress`. F-X011 is pending. F-070 and F-071 are `done`. Tracker rows for F-073, F-074, and F-076 say `not measured`. Review files are pass counts 1/0, 1/0, 0/0 for F-073, 1/0 and 0/0 for F-074, and 0/0 for F-076. `measure.rs` sha256 values in the three AS_BUILT notes match those commits. Each feature trailer tree prefix matches that commit's tree. The wasm budget file is 2151196 bytes with tolerance 0.05, and `int(2151196 * 1.05)` is 2258755. The budget file is not in the sprint diff. `vendor/openjph-core-0.1.0` has no licence file. `max_review_passes` is stored in `scripts/sprint_workflow.py` and no other script reads it. S28 has no `work/f-073`, `work/f-074`, or `work/f-076` branch.

The unit-square test goes red when the half is dropped. The shifted square is in that same test, so dropping the close is not green. Opposite-sign ellipse radii are in the ellipse test, so dropping `abs` is not green for one negative radius.

## Limitations and handoff

This pass is not a clean sprint close. M8 is not complete. Nothing was published. Clippy was not re-run. The F-076 feature trailer on this HEAD lists the clippy gate. That receipt was not repeated here.

Not executed: one `NaN` point through `length` and `polyline_length`, a zero rectangle side, a non-zero equal bidirectional pair, Cobb of opposite parallel segments, height overflow, `Canvas` for the new functions, a non-finite spline control, segment 1 of the spline, and a volume sample that would show the spacing swap. The straight-line sentence in `.claude/plans/F-076-design.md:31` is not what the numeric fixture is. The fixture is bent. The basis weights sum to 1, so a colinear input stays on the line, and no shipped test uses a colinear input.

The next owner corrects the two plan sentences. Do not treat a cornerstone helper as the formula. Do not close M8 from this sprint.
