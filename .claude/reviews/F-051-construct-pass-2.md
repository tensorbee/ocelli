# F-051 review, pass 2

**Reviewed**: uncommitted work on `sprint/s18` at `a404a5e92ef6d2a6166901ca85dcccad422e002b`. Not a committed range. Pass 2 looked at the remediation since pass 1. That is untracked `crates/ocelli-volume/tests/construct.rs` test `construct_spacing_follows_a_four_millimetre_gap`, the AC5 and AC10 rows in unstaged `.claude/plans/F-051-design.md`, and the Tests table row for AC10. `crates/ocelli-volume/src/lib.rs` is still the pass 1 `construct` body. The forced `volume.spacing[2] = 3.0` line is absent. Mutations in this pass were restored. `git diff --stat` for `lib.rs` is still 37 added lines.
**Author / reviewer**: The parent-session implementation agent wrote the remediation. This pass is a different agent and is not a self-review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

Pass 1 D1. Every earlier success fixture expected `spacing[2] == 3`, so assigning `3.0` after `assemble_from` stayed green. AC10 now uses arrivals at z = 10, 2 and 6. Projected positions are 2, 6 and 10. Both gaps are 4 mm. The median of an even pair is their mean, so `spacing[2]` is 4. Row spacing 0.7 mm and column spacing 1.3 mm are `PixelSpacing` order, and the volume triple is column, row, gap, so `[1.3, 0.7, 4.0]`. Origin is the lowest Image Position Patient, `(0, 0, 2)`, not a half step. Samples arrive as `[9,8,7,6]`, `[1,2,3,4]`, `[5,6,7,8]` and must land as `[1,2,3,4]`, `[5,6,7,8]`, `[9,8,7,6]`.

Replayed the original trigger. `volume.spacing[2] = 3.0` immediately after `assemble_from` made `construct_spacing_follows_a_four_millimetre_gap` fail at the spacing zip, exit 101. The line was removed and is not in `lib.rs`.

Adjacent failure. Replacing each placed buffer with its first byte repeated made the same test fail. Slice 0 was `[1,1,1,1]` against `[1,2,3,4]`, exit 101. That closes pass 1 N1 for this entry point. The flat AC1 bytes remain flat, and this fixture is what rejects the splat.

Pass 1 N2. The AC5 comment and plan cell now say the mean is 4 and the worst index stays 2, so the median field is what rejects a mean. That matches gaps 3, 3, 7, 3. Deviations from 4 are 1, 1, 3, 1.

The clean suite after restore passed all 10 construct tests, including AC1 through AC9.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test construct -- construct_spacing_follows_a_four_millimetre_gap` | unmutated tree, z = 10, 2, 6, spacing inputs 0.7 and 1.3 | executed now | exit 0, 1 passed, 9 filtered |
| Same command after `volume.spacing[2] = 3.0` following `assemble_from` | `lib.rs` construct, then restored | executed now | exit 101. Failed `construct.rs:224` on zip against `[1.3, 0.7, 4.0]` |
| Same command after a first-byte splat in `place` | `lib.rs` construct, then restored | executed now | exit 101. Slice 0 `[1,1,1,1]` against `[1,2,3,4]` at `construct.rs:217` |
| `grep` for `spacing[2] = 3.0` in `crates/ocelli-volume/src/lib.rs` after restore | restored `lib.rs` | executed now | no match. `cmp` against the pre-mutation copy matched |
| `bin/ocelli.sh test ocelli-volume --test construct` | restored tree | executed now | exit 0, 10 passed, 0 failed |

## Defects

None. Pass 1 D1 is closed by AC10. Forcing `spacing[2]` to 3 now fails the new test. No new defect.

## Smells

None.

## Nitpicks

None. Pass 1 N1 is closed by the distinct bytes in AC10. Pass 1 N2 is closed by the corrected AC5 sentence in the test and in the plan.

## Verified clean

- AC10's expected slices, origin and spacing were recomputed from PS3.3 C.7.6.2.1.1 before accepting the green run. Axial normal `(0, 0, 1)` projects z directly. Lowest z is 2. Gaps are 4 and 4. `PixelSpacing::new` in the fixture helper takes row then column, and `assemble_from` stores column millimetres, then row millimetres, then the median gap.
- `volume.spacing` is compared with the literal `[1.3, 0.7, 4.0]`. `levels` length is 1, and that level's spacing must match `volume.spacing` within `1e-12`. Forcing the volume field to 3 fails the literal. A lone change to the level field would fail the equality. The force-3 mutation did not need a second assignment to go red.
- `construct` still calls `assemble_from`, clones `order`, and `place`s `evidence.frame().samples()` at the sorted index. There is no spacing assignment after assembly. `place` is still the only store write. The restored source contains `volume.place(index, evidence.frame().samples())?`.
- The new comments state the 4 mm gap, the distinct bytes, and the corrected mean. They have no em-dash and no prose semicolon. The plan edits are table rows. AC10 names the same positions, samples, origin and spacing as the test.
- Pass 1 findings that this pass did not reopen remain as recorded there. `construct` still drops a failed `place`, still refuses shear before a volume is returned, and the `SliceOutOfRange` arms are still the lint-legal total match rather than a tested refusal. `lib.rs` was not edited by the remediation.

## Limitations and handoff

This pass did not re-run the full `ocelli-volume` suite, clippy, wasm, the oracle, or a GPU. The `construct` body is unchanged since pass 1, which ran the crate suite and clippy on that body. No adapter was opened.

AC10 does not read `volume.order`. The slice bytes still require projected placement. A wipe of `order` after a correct `place` would stay green. That was a pass 1 limitation, not a new finding. Mixed orientation, inconsistent dimensions and unsupported kinds are still covered on `assemble` through the same `assemble_from` error path, not by a second `construct` test.

Sprint status files are unchanged from pass 1 and were not re-judged. This pass does not run `gate --sprint` and does not close the sprint. The story's construct behaviour and the pass 1 defect are clean on this tree.
