# F-076 review, pass 1

**Reviewed**: HEAD `da6047e9acadeb93a9c9165befd9ef5f8240f155` on `sprint/s28`. Nothing was staged. The product diff is unstaged and limited to `crates/ocelli-geom/src/lib.rs` (`d494c0df7bca5247570903d6a686c9cc7d10c43e`), `crates/ocelli-geom/src/measure.rs` (`479befbd9f0bde427206f850e5099ff158986fd2`), and `crates/ocelli-geom/tests/measure.rs` (`8be8a156877af9aee11cb115d4fab27a5e2a2ce3`). Against HEAD that is 3 files, 222 insertions, and 7 deletions. Mutations in this pass were restored. Those three hashes match the tree pinned before the first edit.
**Author / reviewer**: The F-076 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change and is not continuing an earlier review. Independence covers this unstaged `ocelli-geom` diff only.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. The unit square is `(0,0,0), (1,0,0), (1,1,0), (0,1,0)`. The cross terms are `0 + 1 + 1 + 0 = 2`, and half of that is 1. Changing `sum.abs() * 0.5` to `sum.abs() * 1.0` made `the_unit_square_has_area_one` fail at `crates/ocelli-geom/tests/measure.rs:426` with the bits of 2. The reversed ring sums to -2, so dropping `abs` leaves -1. That edit failed at line 433 with the bits of -1. The origin square's closing term is 0, so line 426 stays green when the ring is not closed. The shifted square `(1,1,9), (2,1,9), (2,2,9), (1,2,9)` is the close check. Aiming the last edge at the last point instead of index 0 left lines 426 and 433 green and failed at line 440 with the bits of 1.5.

AC2. Two points and an empty slice are `Degenerate`, not 0. Both assertions compare with `Err` directly. They were not mutated. Returning `Ok(0)` would fail them.

AC3. `spline_point` of `(0,0,0), (1,0,0), (2,1,0)`, segment 0, `t = 0.5`. The previous ghost repeats the first point and the next control is the third point. The basis sum before the half is `(0.875, -0.125, 0)`. Half is `(0.4375, -0.0625, 0)`. A polyline midpoint would be `(0.5, 0, 0)`, which is a different bit pattern. Changing the spline scale from `0.5` to `1.0` failed at line 468 with the bits of `0.875` and `-0.125`.

AC4. In exact arithmetic, `t = 0` leaves twice the segment start and `t = 1` leaves twice the segment end, because every other control coefficient sums to 0. The half is the knot. The shipped test locks that on the small segment-0 fixture. It does not cover a finite coordinate large enough for `2 * P` to overflow.

AC5. `t = 1.5`, a segment index with no next point, and non-finite `t` are `Degenerate`. `NaN` is outside `0..=1` because `RangeInclusive::contains` is false when the comparison is false. The three assertions expect `Err` and would fail if the function returned a point.

An independent probe was compiled against this tree and then deleted. The triangle `(0,0,0), (4,0,100), (0,3,-7)` has an XY shoelace sum of 12 and area 6. The 3D edge cross is `(-300, 28, 12)` and its length squared is 90928, so half of that length is not 6. Area 6 rejects a 3D triangle area. Replacing `y` with `z` makes the cross sum -28, so the absolute half is 14, not 6. Segment 1 at `t = 0.5` on `(0,0,1), (1,0,2), (2,1,4)`, with the next ghost repeating the last point, is `(1.5625, 0.5, 3.0625)`. That probe passed. The shipped fixtures never call segment 1.

`length` and `polyline_length` are outside the diff. The full crate run still passed `length_of_a_three_four_five_segment_is_five` and `polyline_length_sums_segments`. The crate has one `fn length` and one `fn polyline_length`. No mean, deviation, or SUV was added. Area closes its own ring. F-073 length does not. The design leaves that split to the later tool.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom -- --test-threads=8` | pinned unstaged `ocelli-geom`, full crate | executed now | exit 0. 1 unit test and 29 integration tests passed, including the five F-076 tests and the existing length tests |
| same command, filter `the_unit_square_has_area_one`, area scale `0.5` changed to `1.0`, then restored | `polygon_area` in `measure.rs` | executed now | exit 101 at `tests/measure.rs:426`. Bits of 2 against bits of 1 |
| same filter, `abs` removed, then restored | `polygon_area` in `measure.rs` | executed now | exit 101 at `tests/measure.rs:433`. Bits of -1 against bits of 1. Line 426 had passed |
| same filter, closing index changed from `0` to the last index, then restored | `polygon_area` in `measure.rs` | executed now | exit 101 at `tests/measure.rs:440`. Bits of 1.5 against bits of 1. Lines 426 and 433 had passed |
| filter `the_first_spline_midpoint_is_hand_computed`, spline scale `0.5` changed to `1.0`, then restored | `spline_point` in `measure.rs` | executed now | exit 101 at `tests/measure.rs:468`. Bits of `0.875` and `-0.125` against `0.4375` and `-0.0625` |
| filter `review_probe_independent_cases` on a temporary test that was then deleted | triangle area 6 with unequal `z`, and segment 1 at `t = 0.5` | executed now, source removed, hashes restored | exit 0 |
| Python evaluation of the plan formula, not of the Rust functions | the same squares and the segment-0 midpoint | executed now | cross terms, half, reversed sign, missing close `1.5`, and basis `(0.875, -0.125, 0)` matched the plan |
| `bin/ocelli.sh clippy ocelli-geom` | not this tree's command | prior scratch receipt only | not run here. `.claude/scratch/F-076-progress.md` says exit 0. That receipt is not evidence for this pass |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14). Cargo ran one process at a time. wasm-pack was not run. No patient data was used.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The contract is the local plan in `.claude/plans/F-076-design.md`. A search of `docs/hld/` found no polygon, spline, freehand, Catmull-Rom, or shoelace formula. Appendix B counts tool classes and does not define these two. No DICOM section applies. The fixture numbers were recomputed from the plan before they were accepted.

`polygon_area` (`measure.rs:520`) rejects fewer than three points, then rejects a non-finite point as `Frame`. The ring wrap is index 0 (`measure.rs:533`). The cross is `x * next.y - next.x * current.y`, so `z` is not a term. The return is `sum.abs() * 0.5` (`measure.rs:552`). A non-finite cross or a non-finite running sum is `Frame`. The module and function comments match those rules. The test comment at `tests/measure.rs:410` matches the cross terms, the half, and the shifted close of 1.5.

`spline_point` (`measure.rs:571`) uses the uniform basis from the plan, on `glam::DVec3`, with the half applied once (`measure.rs:614`). Segment 0 repeats the start as the previous ghost (`measure.rs:584`). A missing following point repeats the segment end (`measure.rs:596`). `t` outside `0..=1` is `Degenerate` before any read. A missing segment or a missing next point is `Degenerate`. A non-finite used point or a non-finite result is `Frame`. In exact arithmetic the endpoint coefficients reduce to the knots. The shipped test locks the small fixture.

`lib.rs:8` says F-076 adds a polygon area and one spline point, and that freehand length stays `length`. Both sentences match the diff. The new names are re-exported from the crate root, and the integration tests call those names.

The crate stays `no_std` outside tests (`lib.rs:15`). The integration test links that library, and it compiled. The new code does not call `std`, `f64::mul_add`, `unsafe`, `unwrap`, `expect`, `panic`, or a slice index. `unwrap_or` at `measure.rs:597` is the end ghost and does not panic. The deleted segment-1 probe ran that path. Workspace `float_cmp` is deny. The new tests compare `to_bits` values or `Result` equality. There is no `as` cast. There is no heap allocation, no `wasm-bindgen`, no pixel transfer, and no queue submit. Tiers A, B, and C are not applicable. Refusal returns `Err` and stores nothing.

The `continue` branches after `slice::get` in the area loop are the same shape as `polyline_length`. On a real slice those gets cannot fail, so they do not drop an edge. The `checked_add` `Frame` inside that loop is also unreachable for a slice length. Neither is treated as a live guard.

## Limitations and handoff

This pass is clean for the pinned unstaged `ocelli-geom` tree. It is not feature completion and it is not a sprint review. The design still adds `polygon_area` and `spline_point` to `docs/lld/public-api.md` at completion. That file was not in this diff.

Clippy was not run. The scratch receipt above must not be reused as this pass's clippy evidence. Overflow of a finite cross product, overflow of the spline result, and a non-finite spline control were not executed. One shipped interior sample does not by itself pin every cubic coefficient. The end-ghost and unequal-`z` cases were executed only through the deleted probe. A one-point ring was not given its own test. It uses the same length check as the empty slice and the two-point case. `polygon_area` checks the count before finiteness, so two non-finite points are `Degenerate`. The plan's two-point row does not ask for `Frame` first, and that pair was not executed.

Area closes a ring that `length` does not close. A later freehand caller has to choose the point list it passes to `length`. No consumer outside this crate was in the diff. The review file is unstaged. Do not attach these results to a later tree.
