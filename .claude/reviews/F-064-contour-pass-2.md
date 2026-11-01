# F-064 review, pass 2

**Reviewed**: working tree on `sprint/s23` at `bc27515a3833fb729f4ef19d0fc701f2af714b05` (`S23, approve sprint designs`). The F-064 implementation is uncommitted. Nothing was staged. Nothing was committed. `git rev-parse HEAD` was the same at the start and at the end. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

Start `git status --short`:

```
 M Cargo.lock
 M crates/ocelli-render/src/graph.rs
 M crates/ocelli-render/src/lib.rs
 M crates/ocelli-seg/Cargo.toml
 M crates/ocelli-seg/src/labelmap.rs
 M crates/ocelli-seg/src/lib.rs
 M docs/sprints/BACKLOG.md
 M docs/sprints/CURRENT_SPRINT.md
?? .claude/reviews/F-064-contour-pass-1.md
?? crates/ocelli-render/src/contour.rs
?? crates/ocelli-render/tests/contour.rs
?? crates/ocelli-seg/src/contour.rs
?? crates/ocelli-seg/tests/contour.rs
```

That start capture does not list `.claude/reviews/F-064-contour-pass-1.md`. The file was already on disk and was read before the status command. The end status lists that file and this pass as untracked.

`git diff --stat` at the start was 8 files, 84 insertions and 7 deletions. Untracked contour sources are not in that stat. The same stat was printed after the temporary `DELTA` edit was restored. `crates/ocelli-seg/src/contour.rs` is `e54a9cb66bf462fa699a10c02404903e916d23138406be056a1646f6b88f8275` before the mutation and after the restore. `cmp` against the pre-mutation copy was silent. `crates/ocelli-seg/tests/contour.rs` is `5e5683835e476dccd3a4a0916e5a62745b9403bc63c50028fe30dcddd7685df8` and was not edited. Pass 1 recorded different hashes for both files, so this pass is looking at the repaired tree.

**Author / reviewer**: this reviewer did not write the contour code, the rounding repair, or pass 1.
**Result**: 0 defects, 0 smells, 0 nitpicks

Pass 1 D1 is closed. No new defect was found.

## Acceptance challenged

AC4 is the rounding claim. The approved plan sets column `c = round((x - origin.x) / spacing_x_mm)` and says `round` is `f64::round`, so `0.5` becomes 1 and `-0.5` becomes -1. The HLD does not define the raster. A `no_std` substitute is acceptable only where the pixel column matches.

The repaired `round_half_away` adds `0.5 - 0.25 * f64::EPSILON`, not a full half. Both sign branches use that `DELTA`. `project` still calls it and then `rounded_i32`.

Independent calls through the real `present_contour`, from `/tmp/f064_probe`, on a 2 by 1 grid with spacing 1 and origin 0:

- `x = f64::from_bits(0.5f64.to_bits() - 1)`, `y = 0`, `z = 0` returns `Ok`. Pixel 0 is `(10, 20, 30, 128)`. Pixel 1 is transparent. This is the pass 1 trigger. Pass 1 saw `Ok` with column 0 transparent and column 1 painted.
- `x = f64::from_bits((-0.5f64).to_bits() - 1)` returns `Ok` with the same pixels. Pass 1 had projected that neighbour to column -1 and had not sent it through the presenter.
- Exact `x = 0.5` returns `Ok`. Pixel 0 is transparent. Pixel 1 is the LUT texel.
- Exact `x = -0.5` returns `Empty`. Every byte stays `0xA5`.

The in-tree locks agree. `a_float_just_below_one_half_stays_on_zero` covers the first two points. `half_rounds_away_from_zero` covers `(1.5, 0.5)` landing on index 5. `a_contour_refusal_writes_nothing` covers exact `-0.5`.

A verbatim copy of `round_half_away`, `trunc_toward_zero`, and `rounded_i32` was compared with `f64::round` in `/tmp/f064_round`. Pixel columns inside `i32` match on the named values and on the boundary scan. The only float-bit difference is the sign of zero, and both columns are 0.

Setting `DELTA` to `0.5` made `a_float_just_below_one_half_stays_on_zero` fail at `crates/ocelli-seg/tests/contour.rs:150`. `paint` had already returned `Ok`. Pixel 0 was `Some([0, 0, 0, 0])` against `Some([10, 20, 30, 128])`. On a 2 by 1 point, that is column 1 painted. The new test guards D1. The source was restored before the final suite.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `git status --short`, `git diff --stat` | `sprint/s23` working tree, start and after restore | Executed now | HEAD `bc27515a3833fb729f4ef19d0fc701f2af714b05` both times. Tracked diff stayed 8 files, 84 insertions, 7 deletions. |
| sha256 and `cmp` of `crates/ocelli-seg/src/contour.rs` | pre-mutation copy `/tmp/contour.rs.pre` | Executed now | `e54a9cb66bf462fa699a10c02404903e916d23138406be056a1646f6b88f8275` before the edit and after `cp` restore. `cmp` matched. The test file hash did not change. |
| `/tmp/f064_probe` `present_contour` | real `ocelli-seg` presenter, spacing 1, origin 0, 2 by 1 | Executed now | Exit 0. `probe_failures=0`. Below-half and above `-0.5` paint pixel 0 `(10, 20, 30, 128)` and leave pixel 1 clear. Exact `0.5` paints pixel 1. Exact `-0.5` is `Err(Empty)` and the buffer stays `0xA5`. |
| `bin/ocelli.sh test ocelli-seg --test contour a_float_just_below_one_half_stays_on_zero` | restored helper, before the mutation | Executed now | Exit 0. 1 passed, 6 filtered out. |
| `bin/ocelli.sh test ocelli-seg --test contour half_rounds_away_from_zero` | same tree | Executed now | Exit 0. 1 passed. |
| `bin/ocelli.sh test ocelli-seg --test contour a_contour_refusal_writes_nothing` | same tree, includes exact `-0.5` | Executed now | Exit 0. 1 passed. |
| `/tmp/f064_round` versus `f64::round` | verbatim helper. `DELTA` bits `3fdfffffffffffff`, equal to `next_down(0.5)`. | Executed now | Exit 0. Named columns match, including just below `0.5`, just above `-0.5`, exact halves, just below `1.5` and `2.5`, just above `-1.5` and `-2.5`, `2^52+1`, and `-0.0`. Full-half add of the two D1 floats is column 1 and column -1. Scan of every integer from `-2147483652` through `2147483651`, plus the half and its two neighbours, plus 192 floats around each power of two from `2^0` through `2^53` on both signs, plus 2000000 random patterns. `checked=17181888953`. `column_mismatches=0`. `bits_inside=0`. `bits_outside=0`. `signed_zero=498832`. Scan time 6.043 seconds on 16 threads. |
| `rustup run 1.97.1 rustc --edition 2024 --crate-type lib /tmp/f064_nostd.rs` | `#![no_std]` calling `f64::round` | Executed now | Exit 1. `E0599`, no method named `round` for `f64`. |
| `DELTA` changed to `0.5`, then the below-half test | one line in `contour.rs`. `diff` against the backup showed only that line. | Executed now, then restored | Exit 101. `a_float_just_below_one_half_stays_on_zero` failed at `tests/contour.rs:150`. Left `Some([0, 0, 0, 0])`. Right `Some([10, 20, 30, 128])`. |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored sha256 above | Executed now | Exit 0. 7 passed, 0 failed. |
| `bin/ocelli.sh clippy ocelli-seg` | restored tree | Executed now | Exit 0. |

## Defects

None.

Pass 1 D1 is closed. `round_half_away` at `crates/ocelli-seg/src/contour.rs:201` no longer adds a full half. The float just below `0.5` is column 0. The float just above `-0.5` is column 0. Exact `0.5` is column 1. Exact `-0.5` is column -1, which a grid that starts at 0 rejects as `Empty` without a write. `2^52+1` now matches `f64::round` in bits. Both results are outside `i32`, so `rounded_i32` returns `Frame` either way, as pass 1 already noted for the old disagreement.

The helper and `f64::round` still disagree on the sign of zero. `f64::round(-0.0)` is `-0.0`. The helper returns `+0.0`. The same split appears for values in `(-0.5, 0)`, including the repaired neighbour just above `-0.5`. `rounded_i32` maps both to column 0. The real presenter paints column 0 and returns `Ok`. The comparison reported 498832 such rows and zero rows whose `i32` column differed. That is not a pixel defect. Pass 1 already treated `-0.0` against `+0.0` as column 0 on both sides.

## Smells

None.

## Nitpicks

None.

## Verified clean

The repair under review is `round_half_away` and `a_float_just_below_one_half_stays_on_zero`. `project`, `trunc_toward_zero`, and `rounded_i32` were read. They are the consumers and the truncation path. The new formula is the only rounding change visible against the pass 1 description. The pre-repair untracked bytes are not in git, so this pass checked the current helper against `f64::round` rather than a stored hunk.

`DELTA` is `0.5 - 0.25 * f64::EPSILON`. Its bits equal `next_down(0.5)`, so the added magnitude is just under one half. The negative branch subtracts it. The positive branch adds it. Neither branch adds `0.5`. There is no `as` cast and no float `==` in the helper.

The comment at `contour.rs:198` says `f64::round` is unavailable in this `no_std` crate, that half steps go away from zero, that the added magnitude is just under one half, and that a full half turns the float below `0.5` into `1`. Each of those sentences was executed. The `no_std` compile cannot see `round`. The inherent method lives in `std`. `core::f64::math::round` is unstable. Exact `0.5`, `-0.5`, `1.5`, and `-1.5` match `f64::round`. The full-half copy of the float below `0.5` is column 1, and the mutation made the presenter paint that neighbour. The comment does not say a full half is identical to `f64::round`.

`present_contour` of the four points above is the real caller. Column 0 of the below-half point is the LUT texel. Column 1 is transparent. The negative neighbour matches. Exact `0.5` paints column 1. Exact `-0.5` leaves the destination unchanged.

The boundary scan covers every half that can land in `i32`, the float on either side of that half, and the integer itself, from four integers below `i32::MIN` through four above `i32::MAX`. `bits_inside=0` means that wherever the column was `Ok`, the helper bits matched `f64::round`, except the signed-zero class counted separately. `bits_outside=0` on this sample means the outside-`i32` floats that were checked also matched in bits. `2^52+1` and its negative are in the named cases and match.

The mutation was one constant. The test went red for the original trigger and was not a green-both-ways guard. Restore was a byte copy. The final contour suite was 7 passed. Clippy on `ocelli-seg` passed with the workspace denials.

## Limitations and handoff

Bresenham, tiers, one `queue.submit`, recovery, and the dead `Present -> Extent` arm are pass 1 coverage. They were not re-executed. The GPU contour test was not re-run. This repair does not touch the upload path. `show_contour` was not called. `bin/ocelli.sh wasm` was not run.

The signed-zero split is recorded above and is not a finding. It does not change an `i32` column.

F-065 and F-066 were not started. This pass does not record a sprint review. The rounding repair is clean on the checks above.
