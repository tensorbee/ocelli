# F-083 review, pass 1

**Reviewed**: branch `sprint/s31`, HEAD `88360221832c46b2e499ae0b44f1172f8ad46e3e` (`F-082, flood fill and paint fill`). `git write-tree` is `9ed3dd1b2477b01cc8e035c7d96e1e285fb03123`, the same tree as `HEAD^{tree}`, because the F-083 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256 after the mutation was restored and the probe was removed: `crates/ocelli-seg/src/threshold.rs` `1dba6d6dd930ec14701cb66e16ca0eea3c9acab6ec1101ee745785904e988bf9`, `crates/ocelli-seg/tests/threshold.rs` `e050699394cf3ec89a8d235f0388196cb2d7e895e8d69f57630c79efcf7a8fc2`, `crates/ocelli-seg/src/brush.rs` `d6f4026fab1968339d901a646f479a279b948a6a31b584102ab8711759db447c`, `crates/ocelli-seg/src/lib.rs` `223e8e7faf19142efbf98a79ba0ccaf97466423ff740242453982fd97ef36740`, `CHANGELOG.md` `5ac9d7114c01c861ee2841ce309fc32f991a794c771043b717cdec66abfe2a10`, `docs/lld/public-api.md` `77d5f6c1854561018fe1679d9d2f460474ceeb916a4363981cd43f6b7bcade50`, `docs/lld/README.md` `9f33a053d4db697826d50f49fb7bda2cc1a35400ac80cd3bcdd19257454fe9ef`. The first three match the digests supplied for this pass.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the threshold, the tests, the brush extraction, or the LLD lines. Reviewers `01a19d56-7774-7e02-8ca3-4ddea41ce3fa`, `01a19d46-f6d6-7080-a3cd-fbdb19b10e6d`, `01a19d37-53e6-7ac0-8aef-0f5953c4195b`, and `01a19d2a-4ab3-74f3-93b5-f282472ae39a` were not resumed. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-083-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan's row text. They count tool classes and the three representations. They do not define a threshold. `docs/hld/03-architecture-and-crates.md` line 40 names `ocelli-seg` and does not give a threshold function. A search of `docs/hld/` found no start-end paint. The inclusive value span is the plan.

AC1 through AC9 are the nine permanent tests. The hand counts below were computed from the plan grids, not copied from a test failure.

On values 0 through 8, a rectangle from `(0, 0)` through `(2, 2)` with start 3 and end 5 paints offsets 3, 4 and 5 only. Covered is 3 and changed is 3. Painting 2 or 6, or leaving 4 alone, would not match `[0, 0, 0, 7, 7, 7, 0, 0, 0]`. A circle at `(1, 1)` with radius 1 has squared distance 0 at the centre, 1 on the four edges and 2 on the four corners. `total_cmp` `is_le` against radius squared 1 keeps the plus and drops the corners. Every plus value is inside 0 through 8, so the shape test is what leaves the corners at 0. Covered is 5. A strict disk would leave the edges at 0.

The removed probe, not a permanent test, then separated the filters. A circle with start 4 and end 4, on a grid of 9s, wrote only the centre and left the plus edges at 9. Covered was 1 and changed was 1. A full-grid rectangle with start 3 and end 5, on a grid of 2s, wrote `[2, 2, 2, 7, 7, 7, 2, 2, 2]`. Out-of-span labels stayed 2. A row-only rectangle with a span that contains every value wrote the first row and left the in-span samples on later rows at 1. A rectangle from `(0.5, 0.5)` through `(1.5, 1.5)` wrote only offset 4. Flooring the minimum would also write offset 0. A span from 0 through 4 wrote offsets 0 through 4 and left value 5 at 9. The sample equal to the end sits at `(1, 1)`, inside the rectangle rather than only on its maximum edge. A circle with start 5 and end 5 wrote only offset 5, which is an edge of the plus, so the high end and `is_le` both have to hold.

`-0.0` as a value with a `+0` span, and `+0` as a value with a `-0` span, each painted the one zero sample. A span from `+0` to `-0` is not inverted and painted only the `-0` sample. A rectangle whose maximum x is `-0` and whose minimum x is `+0` painted column 0 only. Radius 0 and radius `-0` at `(1, 1)` each painted only the centre. Radius 0 at `(1.5, 1.5)` returned covered 0 and changed 0. Segment 0 erased three stored 5s. A grid that already held segment 7 at offset 3 covered 3 and changed 2.

A `NaN` start, a `NaN` minimum, a `NaN` radius, `+inf` at offset 4, and a span from `-inf` to `+inf` were `Frame` and left every stored index in place. A later finite call on the same grid still painted. A `NaN` at the circle centre, with a span of 0 through 1, was `Frame` even though that span would not contain a finite 4 and a positive `NaN` is outside the span under `total_cmp`. A `NaN` at offset 8, after earlier samples were inside the span, was `Frame` and the grid stayed all 1s. A `NaN` on a circle corner stayed outside and the plus was painted. A negative radius with a `NaN` start was `Degenerate`, because the shape is checked before the value span, and the following rectangle still painted the three middle samples. An inverted y span was `Degenerate`, and the following one-cell rectangle still painted offset 4.

A value buffer of length 10, an index buffer of length 10, and an index buffer of length 8 were `Degenerate` and kept their stored numbers. Height 0 kept an empty buffer. A corner circle at `(0, 0)` with radius 1 covered 3 and matched `stamp_brush` on the same centre, radius, slice 0 and segment. The AC2 plus matched `stamp_brush` with covered 5 and the same indices. A one-cell grid painted its only sample.

Those results reject a circle that ignores the value span, a rectangle that floors its bounds, an exclusive end, a partial write before a later non-finite sample, a zero-size refusal that appends an index, and a threshold disk that differs from the F-080 stamp on these two circles.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test threshold` | Product hashes above, before the mutation and before the probe | Executed now | Exit 0. 9 passed, 0 failed, 0 ignored |
| `number_in_span` low side without `same_number`, then restored | `crates/ocelli-seg/src/brush.rs` line 202. High side left as `same_number` or `is_lt`. Disk and ball comparisons left as `is_le` | Executed now | Same test command, exit 101. 5 passed and 4 failed. AC1 covered left 2 and right 3 at `tests/threshold.rs:67`. Exact value covered left 0 and right 1 at `tests/threshold.rs:187`. Outside non-finite covered left 0 and right 3 at `tests/threshold.rs:100`. Second paint covered left 2 and right 3 at `tests/threshold.rs:171` |
| SHA-256 of the seven product paths after restore | Same paths as the Reviewed line | Executed now | Each digest matches. The low side is `same_number` or `is_gt` again. `review_probe_adjacent` is absent |
| First temporary probe binary | Added only in `tests/threshold.rs`. The local name `grid` shadowed the helper | Executed now | Compile exit 101, `E0618` at `tests/threshold.rs:264`. No test ran. This is a harness mistake, not a product failure |
| Corrected `review_probe_adjacent`, then removed | `threshold.rs` and `brush.rs` stayed on their restored digests. The test file was restored to `e050699394cf3ec89a8d235f0388196cb2d7e895e8d69f57630c79efcf7a8fc2` | Executed now | `bin/ocelli.sh test ocelli-seg --test threshold review_probe_adjacent` exit 0. 1 passed. 9 filtered out |
| Python compare of the F-ID list on `docs/lld/public-api.md` line 3 and the F-ID cell of `docs/lld/README.md` line 38 | Working tree | Executed now | Both lists have the same 23 ids, in the same order, ending F-080, F-084, F-081, F-082, F-083. No duplicate |
| `git diff --name-only HEAD` | Working tree against HEAD, after restore | Executed now | Five tracked paths. No `fill.rs`, `scissors.rs`, `labelmap.rs`, `contour.rs`, `surface.rs`, `segment_tools.rs`, shader, `graph.rs`, or `DEVIATIONS.md` |
| `/tmp` `rustc` for `u32::MAX` and the two zero bit patterns | Not a repo path and not cargo. The binary was removed | Executed now | Exit 0. `4294967295` converts to `f64` and back. `+0` bits `0`. `-0` bits `8000000000000000`. `total_cmp` is `Greater`. IEEE equality is true |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. The cargo test commands ran one after another. No wasm-pack ran. No feature gate ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`threshold_roi` is exported with `ThresholdRoi`, `ThresholdShape`, `ThresholdCoverage` and `ThresholdError` from `lib.rs`. `ThresholdShape` and `ThresholdRoi` do not derive `PartialEq`. `ThresholdCoverage` and `ThresholdError` do. `ThresholdError` has `Frame` and `Degenerate` only. There is no sphere variant and no TypeScript copy. `crates/ocelli-wasm` and `packages/` do not name `threshold_roi`.

The rectangle uses `index_in_span` on x and y. There is no z. The circle calls `disk_contains` and does not copy the squared-distance expression, call `sqrt`, or use glam. `index_in_span` is `number_in_span(f64::from(index), min, max)`. The diff in `brush.rs` is that extraction. `disk_contains` at line 145 and `ball_contains` at line 162 stay `total_cmp` `is_le`. `stamp_brush` is not otherwise edited. The removed probe's two circles matched `stamp_brush` sample for sample.

`number_in_span` includes an endpoint by `same_number` or by `total_cmp`. `same_number` treats the two zero bit patterns as one number. The removed probe painted both signs. `span_inverted` is false when the ends are the same number, so `+0` above `-0` is not `Degenerate`. A negative radius uses `strictly_negative`, which rejects a negative number and keeps `-0`. Non-finite bounds, centre, radius, start and end return `Frame` before a write. A non-finite value is read only after `inside` is true, and it returns `Frame` before the span test. A non-finite value outside the shape is not read.

`paint` clones `grid.indices` only after the finite checks, the inverted-span check and `require_stack`. `grid.indices` is assigned at line 183, after the loop. A `Frame` or `Degenerate` return before that line drops the clone. The late `NaN` probe left the stored 1s in place. Counts use `saturating_add`. Covered counts samples inside the shape and inside the span. Changed counts those whose stored segment differed. The mixed probe covered 3 and changed 2. The second permanent paint covers 3 and changes 0.

The offset is `y * width + x` through `checked_mul` and `checked_add`, then `usize::try_from` and `get`. There is no `as` cast in `threshold.rs`. The only use of the word "as" is in the module comment. There is no `unsafe` and no `wasm-bindgen`. The crate stays `no_std` outside tests. `threshold.rs` uses `core::slice::from_ref` and the existing `Vec` clone. It does not open a device. Tiers A, B and C stay not applicable, as the plan says, and there is no tier branch that could write a different result.

`require_stack` on the one grid rejects a zero side and a length that is not `width * height`. The value-length test is the live half of the later check. After `require_stack` succeeds, `grid.indices.len()` is already that count, so the index half of the same check does not add a new refusal. The short, long and mismatched buffers in the permanent test and the probe returned `Degenerate` without a panic.

Every `u32` is below `2^53`. The `/tmp` program showed `u32::MAX` survives `f64::from` and converts back. The rectangle comparison therefore does not round an index.

AC1 through AC9 passed on the unmutated tree. AC9 asks the unchanged `present_labelmap` for fill on and outline off. Pixel index 1 is bytes 4 through 7. Segment 4 is LUT entry 3 from `from_segment_one`. The stated colour is 10, 20, 30, 40, and outline off keeps alpha 40. Pixel 0 is segment 0, which `labelmap.rs` lines 130 to 136 write as zeros rather than leaving the initial 7s. `labelmap.rs` is not in the diff. PS3.3 C.8.20.2 is the background rule only. The value span is the plan. Segment 0 is a legal erase, and the probe wrote it.

The low-end mutation did not leave the suite green. AC8 is the unconfounded permanent failure. Its only sample is at `(1, 1)`, which remains inside a rectangle from 0 through 2 when equal minima are excluded, and the value 4 is then outside an exclusive start. AC1 also went red, but value 3 sits on column 0, so that one failure also loses the rectangle minimum. The circle fixture and the presenter fixture stayed green because their painted samples are not equal to the excluded minimum. The permanent AC2 span contains every plus value, so the circle value filter is locked by AC1's shared `paint` path and by the removed probe, not by AC2 alone.

`CHANGELOG.md` line 13 says the end is included, an outside value is ignored, and a non-finite inside value writes nothing. `docs/lld/public-api.md` line 211 says at or above `start` and at or below `end`, both zero bit patterns match, and an inverted span or a negative radius writes nothing. Both match the code that ran. The new sentences have no em dash and no prose semicolon. The two F-ID lists include F-083 once, in the same place.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the other `ocelli-seg` suites did not run. No GPU adapter was opened. The DICOM PDF was not opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the `checked_mul` refusal on that size was read and not executed. The threshold test does compile the library, including the extracted helper.

The circle value filter, the fractional rectangle, the signed zeros, radius 0, segment 0, the mixed changed count, the late non-finite refusal, the refusal followed by a valid paint, and the stamp comparison were a temporary probe. They are not permanent tests. The probe was removed. Do not treat this pass as coverage of those cases in the suite. AC2 does not by itself reject a circle that ignores the value span, because every plus value is inside 0 through 8. AC9 checks one edge and one corner. It does not reject a presenter that recolours some other pixel. The presenter file is unchanged, and this pass did not rerun the labelmap suite.

The brush suite was not rerun. The extraction keeps the same two comparisons, and the probe's two `stamp_brush` calls matched the threshold masks. That is not the whole brush suite.

The product bytes were restored to the hashes in the Reviewed line before this review file was added. This pass adds only `.claude/reviews/F-083-threshold-pass-1.md`. That file is not in the `git write-tree` above. Do not attach this result to a later tree.

F-083 pass 1: D=0 S=0 N=0
