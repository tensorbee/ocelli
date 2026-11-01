# F-064 review, pass 1

**Reviewed**: working tree on `sprint/s23` at `bc27515a3833fb729f4ef19d0fc701f2af714b05` (`S23, approve sprint designs`). The F-064 implementation is uncommitted. Nothing was staged. Nothing was committed. `git rev-parse HEAD` was the same at the start and at the end.

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
?? crates/ocelli-render/src/contour.rs
?? crates/ocelli-render/tests/contour.rs
?? crates/ocelli-seg/src/contour.rs
?? crates/ocelli-seg/tests/contour.rs
```

`git diff --stat` at the start was 8 files, 84 insertions and 7 deletions. Untracked contour sources are not in that stat. Product sha256 values were recorded before the temporary edits and matched again after every restore. `crates/ocelli-seg/src/contour.rs` is `00118fd03fd67014bae1e0beada840e3f8629c6ed9c9a228732dd10823e57ab1`. `crates/ocelli-seg/tests/contour.rs` is `e880404a32fe7085d617ed501aa506b83469c3382ad1003ddb8f66c1e833df36`. `crates/ocelli-render/tests/contour.rs` is `867272945c1c2357cc990455e142ffa79428310e47adea11f64ff7230b897cbc`.

**Author / reviewer**: the author of the contour code is grok in this session. This pass is an independent reviewer and did not write that implementation.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC5 were checked against the approved plan, not against the comments. AC4's exact halves match `f64::round`. The float just below `0.5` does not. That is D1.

Hand trace of inclusive Bresenham from `(0,0)` to `(2,2)`. `adx = 2`, `ady = 2`, `err = 0`. Plot `(0,0)`, then `e2 = 0` steps both axes to `(1,1)`. The same step reaches `(2,2)` and stops. On a width of 3 the indexes are 0, 4 and 8. Indexes 1 and 3 stay clear. The suite asserts that. A bounding-box stroke would paint them.

`f64::round(1.5)` is 2 and `f64::round(0.5)` is 1. A point at `(1.5, 0.5)` on a 3 by 2 grid is index `1 * 3 + 2 = 5`. The suite asserts that pixel and only that pixel. `f64::round(-0.5)` is -1. The refusal point `(-0.5, 0)` is outside and returns `Empty` with the sentinel kept. Truncation toward zero would have painted column 0 and the test would have failed.

The closed square border keeps alpha 128. Interior `(1,1)` is index 5 and stays transparent. `present_labelmap` of a solid 4 by 3 of segment 1, with fill and outline, paints that interior at alpha 128. The suite asserts the two pixels differ. Adding `plot(raster, 1, 1)` on the closed path made that assertion fail. See Evidence.

Independent calls through the restored `present_contour`, added under `/tmp` only and then removed, also passed these cases before the D1 assertion stopped the test:

- Segment 0 returns `MissingColour` and leaves `0xA5` in place. Index 0 uses `colour`'s `checked_sub(1)` and is not segment 1.
- Spacing `-1` returns `Frame` and leaves the sentinel. Spacing `0` in the suite is the same error for a different reason, because `0/0` is not finite. Negative spacing is the check that still draws if the `<= 0` test is dropped.
- Origin `z` bits `0.0` and a point `z` of `-0.0` return `OutOfPlane`. That is the `to_bits` rule, not an epsilon.
- Open polyline `(0,0) (2,0) (2,2)` paints the two sides and does not paint `(1,1)`, `(0,1)`, `(0,2)` or `(1,2)`. Closing it would paint the diagonal through `(1,1)`.
- Line `(-1,0)` to `(2,0)` on a width of 4 paints columns 0, 1 and 2 and skips the outside end. Column 3 stays clear.
- Spacing `2` and point `x = 3` paint column `round(1.5) = 2`.
- Closed-square interior `(2,1)` is index 6 and stays clear. The committed test checks `(1,1)` only.
- Steep line `(0,0)` to `(1,2)` paints `(0,0)`, `(0,1)` and `(1,2)` only.
- Tie line `(0,0)` to `(2,4)`, where `ady = 2 * adx`, paints `(0,0) (0,1) (1,2) (1,3) (2,4)`. The implementation uses strict `e2 > -ady` and `e2 < adx`. The plan says Bresenham and does not specify the tie. This pass does not treat the strict form as a defect.

A length refusal that wrote `0` into the first output byte before returning made `a_contour_refusal_writes_nothing` fail the `0xA5` check. The restored presenter does not write on that path.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git status --short` | `sprint/s23` working tree, start and end | Executed now | HEAD `bc27515a3833fb729f4ef19d0fc701f2af714b05` both times. End status adds only this review file. |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored presenter, before mutation | Executed now | Exit 0. 6 passed, 0 failed. |
| Closed path also `plot(raster, 1, 1)` | same presenter, one added plot after the closing stroke | Executed now, then restored | Exit 101. Only `closed_square_does_not_fill` failed, at `crates/ocelli-seg/tests/contour.rs:96`. Left `Some([10, 20, 30, 128])`. Right `Some([0, 0, 0, 0])`. 5 passed, 1 failed. |
| Length path wrote `0` into `out[0]` before `ContourError::Length` | restored after the fill mutation | Executed now, then restored | Exit 101. `a_contour_refusal_writes_nothing` failed at `crates/ocelli-seg/tests/contour.rs:199` because a sentinel was no longer `0xA5`. |
| `bin/ocelli.sh test ocelli-seg --test contour` | restored `contour.rs` sha256 `00118fd03fd67014bae1e0beada840e3f8629c6ed9c9a228732dd10823e57ab1` | Executed now | Exit 0. 6 passed, 0 failed. |
| Throwaway `reviewer_independent_probe` through real `present_contour` | temporary test, removed, test sha256 restored | Executed now | Exit 101. Earlier asserts in that test passed. The last assert failed. Pixel 0 of a 2 by 1 point at `f64::from_bits(0.5f64.to_bits() - 1)` was `Some([0, 0, 0, 0])`, not the LUT texel. `paint` had returned `Ok`, so the other pixel was painted. |
| `/tmp/f064_round` versus `f64::round` | verbatim `round_half_away`, `trunc_toward_zero` and `magnitude_u32`. `rounded_i32` differs only by the error type name. Diffed against `contour.rs`. | Executed now | `0.5`, `-0.5`, `1.5`, `-1.5` and `0.0` match in bits and as `i32`. `0.49999999999999994` is column 1 versus 0. `-0.49999999999999994` is column -1 versus 0. `-0.0` is `+0.0` versus `-0.0` and both columns are 0. `2^52 + 1` rounds to `2^52 + 2` versus itself, and both fail `rounded_i32`. |
| `bin/ocelli.sh test ocelli-seg --test labelmap` | F-063 presenter plus `colour_bytes` | Executed now | Exit 0. 7 passed, 0 failed. |
| `bin/ocelli.sh test ocelli-render --test contour -- --ignored --test-threads=1` | uncommitted graph and contour upload | Executed now | Exit 0. 1 passed. A one-line stderr marker after the first readback printed `GPU_CONTOUR_REACHED_READBACK`, then the rest of the test passed, including recovery. The marker was removed and the test sha256 matched the pre-review bytes. This is an adapter run, not an early return. |
| `bin/ocelli.sh clippy ocelli-seg` and `bin/ocelli.sh clippy ocelli-render` | restored tree, before the mutations | Executed now | Exit 0 and exit 0. |

## Defects

### D1, soft round disagrees with f64::round just below one half

**Where**: `crates/ocelli-seg/src/contour.rs:200`
**What**: `round_half_away` adds `0.5` for a non-negative value, or subtracts `0.5` for a negative value, then truncates toward zero. The float just below `0.5`, bits `0x3fdfffffffffffff`, plus `0.5` is `1.0`. The column is 1. `f64::round` of that float is 0. `present_contour` of a `Point` at that `x`, with `y = 0`, `z = 0`, spacing 1 and origin 0, on a 2 by 1 grid returns `Ok` and leaves column 0 transparent. The painted pixel is column 1.
**Why it is wrong**: The approved F-064 plan is the contract. The HLD does not define the raster. The plan sets column `c = round((x - origin.x) / spacing_x_mm)` and says `round` is `f64::round`, so `0.5` becomes 1 and `-0.5` becomes -1. A `no_std` substitute is acceptable only if it matches `f64::round` on that set, including a value just below `0.5`. This value does not match, and the stroke paints the neighbour. The comment at line 198 names the two exact halves and those two do match. The comment is not the formula.
**Evidence**: The `/tmp/f064_round` program printed `pix_got=Ok(1)` and `pix_expect=Ok(0)` for `0.49999999999999994`. The same program printed column `-1` versus `0` for `-0.49999999999999994`. That negative float was not passed through `present_contour`. `project` calls this helper and then `rounded_i32`, and the copied helper matches the source except for the error type name. The real presenter run is the positive case above. Exit 101 on the removed probe. The committed `half_rounds_away_from_zero` test uses exact `1.5` and `0.5`, so it stays green. In the sampled set (`n + 0.5` for `n` from -4000 to 4000, nearby bit patterns, and short walks around powers of two through `2^54`) the only column changes were these two floats. `2^52 + 1` also disagrees with `f64::round`, and both results are outside `i32`, so `rounded_i32` returns `Frame` either way. That witness does not change the presenter result. `-0.0` becomes `+0.0` rather than `-0.0`, and both columns are 0.
**Scope and regression**: `present_contour` and `show_contour`, which uploads those bytes. Present on this working tree. Not a historical commit. Smallest regression is the 2 by 1 point at `f64::from_bits(0.5f64.to_bits() - 1)`. Expect column 0 to hold `(10, 20, 30, 128)` and column 1 to stay transparent. Actual result is the opposite, with `Ok`. The same helper's negative neighbour should paint column 0 and today projects to -1, which `plot` skips.

## Smells

None.

`ContourDrawError::from_labelmap` maps `LabelmapDrawError::Present` to `Extent` at `crates/ocelli-render/src/contour.rs:48`. That arm is dead from `show_contour`. Presenter failures use `ContourDrawError::Present` directly, before `store_overlay`. `labelmap::stage` returns `Extent` only. `store_overlay` adds `NotRebound` if the slot is gone after staging. Neither returns `Present`. The arm does not relabel an error this story can produce. It is exhaustiveness, not a current mislabel. This pass does not call it a smell.

## Nitpicks

None.

## Verified clean

`Contour`, `PlaneFrame` and `ContourGeometricType` store `Pt<World>` and the four PS3.3 C.8.8.6 names. There is no RTSTRUCT parse. `Point` requires one point and paints it. Open kinds require at least two and do not add a closing segment. That was executed on a polyline whose closing diagonal would have added pixels. `ClosedPlanar` requires at least three and the last segment returns to the first. The closed square does not fill. The fill mutation went red and was removed.

`z` uses `to_bits`, including `-0.0` against `0.0`. A mismatch is `OutOfPlane` and the destination is unchanged. Oblique points are not dropped. Non-finite coordinates and non-positive spacing return `Frame` before a write. Spacing `-1` was executed. A pixel outside the grid is skipped. A line that is partly inside still paints the inside pixels. A contour that paints nothing inside returns `Empty` without copying the scratch buffer. A short output returns `OutputLength` with the caller buffer unchanged. The length-write mutation showed that the sentinel test fails if a refusal writes.

Colour comes from `LabelColourLut::colour_bytes`, which is `colour` then `Rgba8::bytes`. Alpha is the LUT alpha. Segment 0 fails the lookup. The stroke does not force alpha 255. The committed open line is `(10, 20, 30, 128)` on columns 0 through 3 and transparent on `(0,1)`.

`show_contour` returns `TierCUnavailable` before it reads the points. Unknown viewport, a non-overlay pass, and a non-composite blend return before `present_contour`. A missing GPU slot is `NotRebound`. `install_recovered` clears every slot. The GPU test observed `NotRebound` before `rebind`, then a second upload whose readback matched `present_contour`. `rebind` builds a slot with `labelmap: None`, so the next show cannot take the equal-bytes return. Equal bytes skip `labelmap::stage` and do not set dirt. The GPU test observed `Dirty::default()`, `FrameOutcome::Noop`, and an unchanged `submissions` counter. A presenter error is `ContourDrawError::Present` and does not call `store_overlay`.

The copy shader is the existing `labelmap.wgsl`. It loads the texel and does not test neighbours. `fragment_pipeline` sets `blend: None`, which is replace. No second shader was added. `encode_frame` has one `queue.submit`, at `graph.rs:3656`. `show_contour` uploads with `write_texture` inside `stage`, which is the F-063 path, and it is not the tick. The CPU scratch in `present_contour` and the byte `Vec` in `show_contour` are outside the tick. Equal bytes still build that CPU buffer, then skip the texture. That matches `show_labelmap`.

`show_labelmap` now calls `store_overlay`. The moved tail matches the committed function. Arguments are still the labelmap width, height and bytes. `present_labelmap` itself is unchanged apart from `colour_bytes`. The labelmap suite passed.

`Cargo.lock` adds only the `ocelli-seg` to `ocelli-core` edge. The new code has no `as` cast, no `unsafe`, no `wasm-bindgen`, no new trait, and no `Box<dyn>`. `z` is not compared with `==`. Clippy on both crates passed with the workspace `float_cmp` and cast denials. `ContourView` groups width and height so `show_contour` stays at seven arguments. `present_contour` still takes width and height separately. That grouping does not change a coordinate. `f64::round` is not stable in `core` on Rust 1.97.1. The inherent method is in `std`, and `core::f64::math::round` is unstable. The `no_std` comment is true. The crate root says the presenter does not import wgpu. The contour module does not.

No product file differed from its pre-review sha256 after the mutations, the GPU marker, and the throwaway test were removed.

## Limitations and handoff

D1 blocks this story. The committed suite does not contain the adjacent float, so a green `contour` test is not a fix.

Tier C on `show_contour` was read and not executed. `RenderGraph::new` refuses tier C before a graph exists. The branch matches `show_labelmap` and returns before the points are read. `install_recovered` can store `recovered.caps.tier` with no extra check. This pass did not build that caps value.

The ignored GPU test did run on an adapter and reached readback. It does not use the D1 coordinate. `bin/ocelli.sh wasm` was not run. No new wasm export was added, and pixels stay in the presenter buffer and the overlay texture. The F-063 GPU labelmap test was not re-run. The shared upload tail did run through `show_contour`.

F-065 and F-066 were not started. Shaders, `voi.wgsl`, `fusion.wgsl`, `cpu.rs`, the oracle producer, tolerances, and the wasm budget were not edited.

The next owner fixes `round_half_away` so the column matches `f64::round` for the float just below `0.5` and the float just above `-0.5`, without changing the exact halves the suite already locks. A fresh independent pass reviews that edit. This pass does not claim sprint readiness.
