# F-085 review, pass 1

**Reviewed**: branch `sprint/s31`, HEAD `b1baaea9c97c5af37594cd64a5efdb69f9264d92` (`F-083, rectangle and circle threshold`). `git write-tree` is `3008e847758d31f43a54348eb579767adb3387d9`, the same tree as `HEAD^{tree}`, because the F-085 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256 after the mutation was restored and the probe was removed: `crates/ocelli-seg/src/contour_edit.rs` `39702b31160146da28cf5e4721d5b1b38eadc50f8416ce00fd1be68b841d9e72`, `crates/ocelli-seg/tests/contour_edit.rs` `6a824ce4ca6fc07cfc6cea668b97e3cebc600710e174dd35f8ed408d87b16a33`, `crates/ocelli-seg/src/lib.rs` `480c9649acb71ad21fbc5044640bd4f679d1ef4c330479914df6b3170db1096a`, `CHANGELOG.md` `9e4764dfcc94b41188a9c26d97b4ab2fea33c3e63cee4d6f47dd5dc6f37e8cc8`, `docs/lld/public-api.md` `8006715809c3749f5de7810dc87e1dc22ea62b9c8761cd4d99bfef24a810e77b`, `docs/lld/README.md` `01413789f91dd82b59d5e7089731ae52a8957a2304ed6a776c72d12071aa106c`. The first two match the digests supplied for this pass.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the contour edit, the tests, or the LLD lines. Reviewers `01a19d6c-ad1a-76d3-8e5f-e307d479764c`, `01a19d56-7774-7e02-8ca3-4ddea41ce3fa`, `01a19d46-f6d6-7080-a3cd-fbdb19b10e6d`, `01a19d37-53e6-7ac0-8aef-0f5953c4195b`, and `01a19d2a-4ab3-74f3-93b5-f282472ae39a` were not resumed. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-085-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan's row text. They count tool classes and the three representations. They do not define an edit of a labelmap from a polygon. `docs/hld/03-architecture-and-crates.md` line 40 names `ocelli-seg` and does not give this function. The even-odd rule, the closed-edge rule, and the integer sample point are the plan. They are not an HLD formula and they are not the F-064 contour.

AC1 through AC8 are the eight permanent tests. The hand counts below were computed from the plan grids, not copied from a test failure.

On a width 5 height 3 grid, the rectangle `(1, 0), (3, 0), (3, 2), (1, 2)` has eight boundary samples and one interior sample at `(2, 1)`. The top and bottom edges are horizontal, so they add no crossing. `(2, 1)` has one crossing, on the right edge at x = 3. The left edge at x = 1 is not strictly to the right. The count is odd, so the sample is inside. `(0, 1)` has two crossings and is outside. `(4, 1)` has none and is outside. Covered is 9 and changed is 9. Painting an outer column, or dropping column x = 3, would not match the fifteen indices. `(2, 2)` lies on the bottom edge and has no crossing, so it is inside only through the boundary test.

AC2 on a 3 by 3 grid uses the triangle `(2, 0), (2, 2), (0, 1)`. The right edge contains `(2, 0)`, `(2, 1)` and `(2, 2)`. `(0, 1)` is a vertex. `(1, 1)` is not on an edge. Its cross products against the three edges are 2, 1 and 1. The ray crosses only the right edge. `(1, 0)` crosses the right edge and the closing edge, so it is outside. `(0, 0)` and `(0, 2)` and `(1, 2)` are outside. The written grid is `0 0 5`, then `5 5 5`, then `0 0 5`. Covered is 5.

The removed probe, not a permanent test, then separated cases the eight tests do not lock. Expectations came from the same rule, checked with an independent Python model of the crossings before the Rust probe ran.

Segment 0 on a grid of 7s erased the nine inside samples and left both outer columns at 7. A second erase covered 9 and changed 0. `segment_at` read 0 at `(2, 1)` and 7 at `(0, 1)`. A grid whose column x = 2 already held 4 covered 9 and changed 6. Width and height stayed 5 and 3.

A 1 by 2 grid and the polygon `(0, 0), (2, 0), (1e308, 0.5), (-1e308, 1.5)` returned `Frame`. Both stored 9s stayed. `(0, 0)` is the first vertex, so an in-place write would have changed it before the later sample returned `Frame`. The later sample straddles the overflowing edge.

A diamond `(1, 0), (2, 1), (1, 2), (0, 1)` wrote `[0, 5, 0, 5, 5, 5, 0, 5, 0]`. `(1, 1)` is not on an edge and has one crossing.

Two squares in one ring, `(0, 0), (5, 0), (5, 5), (0, 5), (2, 2), (7, 2), (7, 7), (2, 7)`, left the overlap samples `(3, 3)` and `(4, 4)` at 0. A nonzero winding of 2 would paint them. `(1, 1)`, `(4, 1)`, `(1, 3)`, `(6, 3)` and `(6, 6)` became 6. `(0, 1)` stayed 0.

A concave notch on a 6 by 4 grid covered 21 and changed 21. Row 2 wrote three leading 8s and left the open side at 0. The other three rows were entirely inside.

A rectangle whose top edge uses `-0.0` wrote the same nine samples as AC1. Three collinear points `(0, 0), (2, 0), (4, 0)` painted the five samples on y = 0 and left the next row at 1. A triangle entirely in negative coordinates covered 0 and changed 0 on a grid of 3s.

Width 0 with an empty buffer, a 2 by 2 buffer of length 5, and an empty point list were `Degenerate` and wrote nothing. `+inf` as a vertex was `Frame` and left four 7s. The following triangle `(0, 0), (0, 1), (1, 0)` painted three samples and left `(1, 1)` at 7.

`flood_fill` from `(0, 0)` after the AC1 rectangle replaced only the three left zeros with 8. The right column stayed 0. The painted segment blocked the 4-connected walk. `segment_at` read 0 at `(4, 2)`. `Representation::Contour` still matches the contour variant. The labelmap and surface variants are still present.

Those results reject an even fill, a winding fill of the overlap, a segment 0 refusal, a partial write before a later non-finite intersection, a `-0` top edge that drops the boundary, an unbounded horizontal line, and a flood that crosses the new segment.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test contour_edit` | Product hashes above, before the mutation and before the probe | Executed now | Exit 0. 8 passed, 0 failed, 0 ignored |
| Crossing test changed from `== 1` to `== 0`, then restored | `crates/ocelli-seg/src/contour_edit.rs` line 124. The test file was not edited for this run | Executed now | Same test command, exit 101. 3 passed and 5 failed. AC1 covered left 14 and right 9 at `tests/contour_edit.rs:42`. Repeated close covered left 14 and right 9 at line 117. Triangle covered left 8 and right 5 at line 55. Second edit covered left 14 and right 9 at line 129. Presenter outer pixel left `[10, 20, 30, 40]` and right `[0, 0, 0, 0]` at line 177. The vertex, overflow and short-buffer tests stayed green |
| `review_probe_adjacent`, then removed | Only `tests/contour_edit.rs` was edited. `contour_edit.rs` stayed on `39702b31160146da28cf5e4721d5b1b38eadc50f8416ce00fd1be68b841d9e72` | Executed now | `bin/ocelli.sh test ocelli-seg --test contour_edit review_probe_adjacent` exit 0. 1 passed. 8 filtered out |
| Same test command after both files were restored | Both product digests matched the supplied hashes. `review_probe_adjacent` was absent | Executed now | Exit 0. 8 passed, 0 failed, 0 ignored |
| Python model of the plan rule | Not the crate. IEEE doubles. Same polygons as the probe | Executed now | Exit 0. Diamond, overlap, notch, collinear row, negative triangle, `-0` rectangle and the late `Frame` pair agreed with the probe |
| Python compare of the F-ID lists | `docs/lld/public-api.md` line 3 and `docs/lld/README.md` line 38 | Executed now | Both lists have the same 24 ids, in the same order, ending F-084, F-081, F-082, F-083, F-085. No duplicate |
| `git diff --name-only HEAD` and `git status --short` | Working tree against HEAD, after restore | Executed now | Four tracked paths, plus the two untracked Rust files. No `contour.rs`, `labelmap.rs`, `surface.rs`, `fill.rs`, `threshold.rs`, `brush.rs`, `scissors.rs`, `segment_tools.rs`, shader, `graph.rs`, or `DEVIATIONS.md` |
| `/tmp` `rustc` for `u32::MAX`, the two zero bit patterns, and the AC1 corner cross product | Not a repo path and not cargo. The binary was removed | Executed now | The program printed and exited 0. `4294967295` converts to `f64` and back. `+0` bits `0`. `-0` bits `8000000000000000`. `total_cmp` is `Less`. IEEE equality is true. The corner cross bits are `8000000000000000` |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. `rustc` 1.97.1 and `cargo` 1.97.1. The cargo test commands ran one after another. No wasm-pack ran. No feature gate ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`edit_labelmap_with_contour` is exported with `IndexPoint`, `ContourEditCoverage` and `ContourEditError` from `lib.rs`. `IndexPoint` derives `Debug`, `Clone` and `Copy`. It does not derive `PartialEq`. `ContourEditCoverage` and `ContourEditError` do. The error has `Frame` and `Degenerate` only. There is no TypeScript copy. `crates/ocelli-wasm` and `packages/` do not name the function.

The sample point is `f64::from` of the integer column and row. There is no half-pixel centre. The ring closes with `(index + 1) % count`. A sample on any edge returns inside before the crossing loop. A horizontal edge is skipped by `same_number` on the two y values. A straddling edge has one end at or below the sample y and the other strictly above. `x_hit` uses the line fraction. A non-finite `x_hit` is `Frame`. An `x_hit` that is the same number as the sample x is inside. A strictly greater `x_hit` increments the crossing count. Odd means inside.

`on_segment` uses the edge cross product and the dot against the length squared. A zero-length edge contains only that same point. Equality is `same_number` from `brush.rs`, which treats the two zero bit patterns as one number and otherwise uses `total_cmp`. The AC1 corner `(3, 2)` starts the edge toward `(1, 2)`. The `/tmp` program showed that cross product is `-0`. AC1 still paints that corner, so the zero rule is what includes it. `total_cmp` alone would call `-0` less than `+0`.

Vertices are rejected with `Frame` before the copy when `is_finite` is false. The probe's `+inf` vertex took that path, and a later finite edit on the same grid still painted. `require_stack` on the one grid rejects a zero side and a length that is not `width * height`. The probe covered width 0, a long buffer, and the permanent short buffer. Fewer than three points returns before the copy. The empty list did too.

The scan clones `grid.indices` at line 62 and assigns it at line 89, after every sample is classified. A `?` from `inside_polygon` returns before that assignment. The late overflow probe left both 9s in place. Inside samples become the caller segment, including 0. Outside samples stay. Covered counts the inside samples. Changed counts those whose stored number differed. Both use `saturating_add`. The mixed probe covered 9 and changed 6. The second erase changed 0.

There is no `as` cast in `contour_edit.rs`. The only use of the word "as" is in the module comment. There is no `unsafe`, no `wasm-bindgen`, no `sqrt`, and no glam. The function does not call `present_contour` or `intersect_contour`. It does not write `Representation`. The crate stays `no_std` outside tests. The new module uses `core::slice::from_ref` and the existing `Vec` clone. It does not open a device. Tiers A, B and C stay not applicable, as the plan says, and there is no tier branch that could write a different result.

`point_at` returns `Degenerate` when the index is missing. After the length check, every index is in `0..count` or the modulo of the next index, so that arm does not add a reachable refusal. The offset uses `checked_mul`, `checked_add`, `usize::try_from` and `get_mut`. On the grids this pass executed, those arms did not fire. Every `u32` index is below `2^53`. The `/tmp` program showed `u32::MAX` survives `f64::from` and converts back.

AC1 through AC8 passed on the restored tree. The odd-to-even mutation did not leave the suite green. AC1 is the unconfounded permanent failure for the interior sample. Covered moved from 9 to 14, which is every sample except the one odd interior point. The presenter failure was the outer pixel at bytes 0 through 3. The boundary pixel assert at line 176 had already passed, so the boundary stayed inside while the even outside sample was painted. AC2's covered count moved from 5 to 8. The repeated closing point and the second edit failed on the same covered count of 14. The `Frame` and `Degenerate` tests stayed green because they return before a fill result.

AC8 asks the unchanged `present_labelmap` for fill on and outline off. Pixel column 1 is bytes 4 through 7. Segment 4 is LUT entry 3 from `from_segment_one`. The stated colour is 10, 20, 30, 40, and outline off keeps alpha 40. Pixel 0 is segment 0, which `labelmap.rs` lines 130 to 136 write as zeros rather than leaving the initial 7s. `labelmap.rs` is not in the diff. PS3.3 C.8.20.2 is the background rule only. The even-odd rule is the plan.

`CHANGELOG.md` line 13 says the boundary is included, the ring closes, a non-finite vertex or intersection writes nothing, and the contour representation is unchanged. `docs/lld/public-api.md` line 213 says an integer sample is inside on a closed edge or on an odd ray toward increasing x, a horizontal edge does not count, and fewer than three points writes nothing. The horizontal sentence is the crossing exception. It does not drop a sample that lies on a horizontal edge. AC1's bottom row is that case. Both paragraphs match the code that ran. The new sentences have no em dash and no prose semicolon. The two F-ID lists include F-085 once, in the same place.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the other `ocelli-seg` suites did not run. No GPU adapter was opened. The DICOM PDF was not opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the `checked_mul` refusal on that size was read and not executed. The contour test does compile the library.

Segment 0, the mixed changed count, the late `Frame` after an inside vertex, the diamond, the self-overlap, the notch, the signed-zero edge, the collinear ring, the off-grid triangle, the long buffer, the empty point list, the refusal followed by a valid edit, and the `flood_fill` walk were a temporary probe. They are not permanent tests. The probe was removed. Do not treat this pass as coverage of those cases in the suite. AC1 through AC8 do not by themselves reject a nonzero winding fill, because their polygons are simple and the two rules agree there. The overlap probe is what rejects winding 2. AC8 checks one boundary pixel and one outer pixel. It does not reject a presenter that recolours some other pixel. The presenter file is unchanged, and this pass did not rerun the labelmap suite.

The fill suite and the segment-tool suite were not rerun. The probe called `flood_fill` and `segment_at` on the edited grid only. `contour.rs` was not edited, and this pass did not rerun the contour suite. The representation match in the probe only shows that the three variants are still present.
