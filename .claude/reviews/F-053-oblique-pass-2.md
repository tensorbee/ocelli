# F-053 review, pass 2

**Reviewed**: uncommitted working tree on `sprint/s20`, base `3f9954df100c5ff1a2e6982fa9b9873f4560772c`. Nothing is staged. `git status --short` lists `M crates/ocelli-volume/src/lib.rs`, `M crates/ocelli-volume/src/ortho.rs`, `?? crates/ocelli-volume/tests/oblique.rs`, `?? crates/ocelli-render/tests/oblique.rs`, and `?? .claude/reviews/F-053-oblique-pass-1.md`. No other product path is modified. `docs/lld/volumes.md` is not in the diff.
**Author / reviewer**: implementation by grok. This pass is a separate reviewer subagent that did not write the new test. Independence is limited to that split.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The only product change since pass 1 is `unequal_spacing_selects_each_axis` in `crates/ocelli-volume/tests/oblique.rs`, plus the `spaced_volume` helper that test calls. `crates/ocelli-volume/src/ortho.rs` hashes to `81af0de565a78e2c0bd6c42e3d53ad27b9e823ba67ca054047757d11e030570b`, the same bytes pass 1 recorded after restoring the forced `j = 0.0`. The three index lines are still

```rust
let i = self.direction.col(0).dot(delta) / self.spacing[0];
let j = self.direction.col(1).dot(delta) / self.spacing[1];
let k = self.direction.col(2).dot(delta) / self.spacing[2];
```

The new fixture was recomputed without calling the Rust sampler. Spacing `(2, 4, 8)`, origin `(10, 20, 30)`, identity direction. Index `(0.25, 0.5, 0.75)` is patient `(10.5, 22, 36)`, because `10 + 2 * 0.25`, `20 + 4 * 0.5` and `30 + 8 * 0.75` are those coordinates. The fractional parts are `fx = 0.25`, `fy = 0.5` and `fz = 0.75`. The low and high weights are therefore `0.75/0.25`, `0.5/0.5` and `0.25/0.75`. Over a denominator of 32 the eight weights are `3, 1, 3, 1, 9, 3, 9, 3`. They sum to 32. The matching numerator on bytes `10, 30, 70, 90, 110, 130, 150, 170` is `3600`. `3600 / 32 = 112.5`. That value is exact in `f32`. Its bits are `1122041856`.

The same spacing and origin with columns `(0, 1, 0)`, `(0, 0, 1)` and `(1, 0, 0)` put index `(0, 0, 0.5)` at patient `(14, 20, 30)`. Half of the 8 mm third-axis step is 4 mm, and the third column is X. The inverse dots are `0`, `0` and `4 / 8`. The corners are 10 and 110, so the sample is 60. The bits of 60 are `1114636288`. A dropped `spacing[2]` divisor on that pixel is index `4 / 1 = 4`, which is outside the grid.

Replacing all three divisors with `/ 1.0` makes the identity pixel the index `(0.5, 2, 6)`. Axes of length 2 do not contain 2 or 6, so the sample is `Absent`. `unequal_spacing_selects_each_axis` panicked in `present_bits` at `crates/ocelli-volume/tests/oblique.rs:291`, called from the 112.5 assertion at line 108. The text was `entered unreachable code: the fixture sample has landed`. The other seven oblique tests stayed green. Exit 101.

That run does not by itself show a wrong in-range blend, because two axes have already left the grid. Replacing only the first divisor does. Index `(0.5, 0.5, 0.75)` stays inside and the eight products sum to 117.5, whose bits are `1122697216`. The same assertion then failed with left `1122697216` and right `1122041856`. Exit 101. Replacing only the second divisor, or only the third, again took the `Absent` path at `present_bits`. Exit 101 on each. The second and third single-divisor runs were filtered to this one test, so they do not repeat the seven-green observation. The all-three run does.

Dividing the patient delta components by spacing, with no column dots, still yields 112.5 on the identity direction, because those columns are the axes. The permuted pixel becomes index `(2, 0, 0)` and is `Absent`. The panic was `present_bits` called from the 60 assertion at line 132. Exit 101. The second half is what locks the columns. The first half is what locks a non-zero divisor on each axis. On the permuted pixel the first two dots are zero, so those two divisors are not visible there.

After every mutation, `ortho.rs` was restored. The hash matched the pre-mutation hash. `bin/ocelli.sh test ocelli-volume --test oblique` then exited 0 with 8 passed. `git diff` does not contain `/ 1.0` or a component divide.

Pass 1's other checked behaviour stays closed because `ortho.rs` is those same bytes. The mix test still compares 95 with 70. The named-plane test is still present. The seven fixtures that use spacing `(1, 1, 1)` still pass when the divisors are deleted, which is the pass 1 observation, and the new test is what now goes red. `docs/lld/volumes.md` is still the orthographic section only. That was a handoff limitation, not a counted finding, and this pass did not edit it.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| Hand weights on bytes 10, 30, 70, 90, 110, 130, 150, 170 | No Rust sampler. Spacing `(2, 4, 8)`, origin `(10, 20, 30)` | Executed now | `3600 / 32 = 112.5`. Patient `(10.5, 22, 36)` and `(14, 20, 30)`. Sample 60. Dropped first divisor is 117.5. `f32` bits 1122041856, 1114636288 and 1122697216 |
| `bin/ocelli.sh test ocelli-volume --test oblique` | Restored tree before any mutation. Eight host tests | Executed now. Author receipt was a prior exit 0 | Exit 0. 8 passed |
| All three divisors replaced by `/ 1.0`, then the volume oblique test | `ortho.rs` index solve. Restored after the run | Executed now. Author receipt was exit 101, `Absent` at the 112.5 assertion, seven green | Exit 101. Panic at `tests/oblique.rs:291`, called from line 108. Text `the fixture sample has landed`. 7 passed, 1 failed |
| Only `spacing[0]` replaced by `/ 1.0`, then `unequal_spacing_selects_each_axis` | Same file, one divisor. Restored after the run | Executed now | Exit 101. Assertion at line 108. Left bits 1122697216. Right bits 1122041856 |
| Only `spacing[1]`, then only `spacing[2]`, each against that one test | Same file. Restored after each run | Executed now | Exit 101 both times. `Absent` panic in `present_bits`. The other seven were filtered out |
| Column dots replaced by `delta.x`, `delta.y` and `delta.z` over the same spacing | Same three lines. Restored after the run | Executed now | Exit 101. Panic at line 291, called from the 60 assertion at line 132 |
| `bin/ocelli.sh test ocelli-volume --test oblique` | Restored `ortho.rs` hash `81af0de565a78e2c0bd6c42e3d53ad27b9e823ba67ca054047757d11e030570b` | Executed now. Author receipt was exit 0, 8 passed | Exit 0. 8 passed. Test file hash `ddf3bc86807a4927624f07279eb1c2e9323e152d30693687af7d0a5fb318bd2a` |
| Environment | arm64, rustc 1.97.1 `(8bab26f4f 2026-07-14)` | Executed now | Recorded |

Clippy, `gate unsafe`, the rest of `ocelli-volume`, and `ocelli-render --test oblique` were not re-run. Render sources are not in the diff. The render test file is the untracked file pass 1 already reviewed.

## Defects

None.

Pass 1's open limitation is closed. Deleting the three spacing divisors no longer leaves `tests/oblique.rs` green. The new test fails, and each divisor fails on its own. The 112.5 bit compare is live for the axis that stays inside the grid. The 60 compare is live for a solve that divides patient components and skips the columns.

## Smells

None.

## Nitpicks

None.

## Verified clean

`voxel_world` of `(0.25, 0.5, 0.75)` and of `(0, 0, 0.5)` is asserted with `f64` bits against the patient points above. The plane origin is that same literal, not a value read back from `voxel_world`, so the forward multiply and the inverse divide are separate checks. Pixel `(0, 0)` does not step the plane row or column. The sample location is the volume inverse.

The identity pixel uses every corner. None of the eight weights is zero. 112.5 is not 95, not 70, not 120, and not a stored corner. A first-divisor miss is 117.5, not 112.5. The permuted pixel is the average of 10 and 110. Ignoring the columns on that pixel leaves the grid instead of returning 60.

`spaced_volume` writes the same triple into `Volume.spacing` and the level-0 spacing record. `reslice_oblique` divides `self.spacing`. The two fields do not diverge in this fixture, and the mutation that fails is the divisor on `self.spacing`.

The earlier seven tests are unchanged. `half_voxel_oblique_is_the_trilinear_mix` still rejects a y = 0 snap. `named_planes_stay_ortho`, `a_non_unit_row_is_not_renormalised` and `a_bad_plane_allocates_nothing` are still in the file. `lib.rs` still only exports `ObliquePlane` and adds `VolumeError::ObliquePlane`. No new trait, generic, `unsafe`, `unwrap`, `expect` or `as` cast was added by this pass, because the production files were not edited.

## Limitations and handoff

The mutations were reverted. `ortho.rs` matches the pass 1 restored hash. The new test file was not edited by this pass.

`docs/lld/volumes.md` still describes only `Volume::reslice` in the orthographic section. Pass 1 already recorded that gap. It is not a defect in the new test.

Deleting `spacing[2]` fails on the identity pixel, whose index becomes 6, so the run does not reach the permuted pixel where the comment predicts index 4. The comment is still the right geometry for that second pixel. The red run that shows the second assertion is the no-dot solve, not the dropped divisor.

The single-divisor runs of axes 1 and 2 did not capture a backtrace, so their caller line is inferred from the `Absent` panic and from source order. The all-three run and the no-dot run have stacks. `columns` times `rows` overflow, a counting allocator, a non-orthonormal direction, and a GPU adapter were not exercised again. `show_ortho` was not called.

The only file this pass adds is `.claude/reviews/F-053-oblique-pass-2.md`.
