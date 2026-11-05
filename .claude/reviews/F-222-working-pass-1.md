# F-222 review, pass 1

**Reviewed**: base `c775931`, uncommitted working diff of `crates/ocelli-volume/src/levels.rs` and `crates/ocelli-volume/tests/pyramid.rs` (`git diff` sha256 prefix `7ae8d6ce1d9bee8a`, 239 insertions and 54 deletions). The untracked `tools/oracle/out` symlink is excluded. Worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-222`, branch `work/f-222-xai`.
**Author / reviewer**: an external coding agent implemented the diff. The reviewer is a Claude subagent that did not author the design or the code. The reviewer did not verify the author's identity from anything other than the caller's statement.
**Result**: 2 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **AC1**: worked out independently. 1 to 7 sum to 28, with finite weight 7, so 4.0 = `0x4080_0000`. Dividing by the full weight (mutation M3) goes red.
- **AC2**: `+inf, 1.0` gives `0x3f80_0000`, and `-inf, -3, 6, +inf` gives 3/2 = `0x3fc0_0000`. Testing `!is_nan` in place of `is_finite` (M1) goes red.
- **AC3**: every input word in the payload box is a NaN (`0x7fff_ffff` has an all-ones exponent and a non-zero fraction). All three infinity pairs give `0x7fc0_0000`. Changing the constant's bits (M2) goes red. Writing the narrowed input NaN or infinity (M11) goes red.
- **AC4**, worked out from IEEE 754 and `narrow_f32`'s code, not from the comment:
  - `0x8000_0001` is `-2^-149`. Paired with `+0.0`, the f64 sum is exact and the f64 mean is `-2^-150`, exact. The exponent field is 873, so the target is -23 and the shift is 53. Shifting a significand of `2^52` by 53 places gives main 0 and a remainder exactly at the half point. Main is even, so the result rounds down to `0x8000_0000`.
  - The control `0x8000_0001` twice has mean `-2^-149`, a shift of 52, main 1 and remainder 0, so `0x8000_0001`.
  - The first three pairs have f64 sum `+0.0` under round-to-nearest-even (`+0 + -0 = +0`, `3 - 3 = +0`), so they pass with or without the rule. Only the fourth case discriminates, which matches the plan.
  - Disabling the clearing (M4) goes red. Normalising in f64 instead of after narrowing (M8) goes red, with left `0x8000_0000`. `abs` (M9) breaks the control and goes red.
- **AC5**: with D=5 and d=3, the intervals are child `[5c, 5c+5)` and parent `[3p, 3p+3)`. The child weights are (3, 2), (1, 3, 1) and (2, 3). The finite means are 10, 130/4 = 32.5 and 40. 32.5 is `2^5 * 1.015625`, which is `0x4202_0000`. 10 is `0x4120_0000` and 40 is `0x4220_0000`. Counting samples instead of adding up weights (M7) goes red.
- **AC6**: going from 4 to 2, each child has weights (2, 2), giving 1.0 and 16/4 = 4.0. Going from 4 to 1 gives 9/3 = `0x4040_0000`. Reading the source from the last level instead of level 0 (M10) turns this test and F-119's `fixture_every_level_is_a_downsample_of_level_zero` red.
- **AC7 and AC8**: when every sample is finite, `weight` is the exact integer sum of the factors, which is `Dx*Dy*Dz`. That is below `2^53`, and each axis weight is bounded to u32 by `weight_f64`. The sum is accumulated in the same order and with the same expression as before. The integer path is untouched. `fixture_float_mean` and the integer fixtures are unchanged in the diff, and they are green.
- **AC9**: the coordinate half is the same text as before under the new name, and `level_voxel_to_level0` is not in the diff.
- **AC10**: `non_finite_floats_keep_the_current_bits` and "decision is open" have no matches under `crates/`. `docs/lld/volumes.md:152` still says "is an open operator question", which the plan correctly defers to `/complete-feature`.
- **Zero weight with `any` set**: no meaningful wrong behaviour is possible. The two ends of `parent_span` have positive overlap by construction, and a 0/0 would still pass through `narrow_f32`'s NaN branch.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | working tree | now | 0 |
| `cargo clippy -p ocelli-volume --all-targets -- -D warnings` | working tree, own target dir | now | 0 |
| `cargo test -p ocelli-volume` | working tree | now | 0. pyramid has 34 passed, every other suite is green |
| `python3 scripts/prose_check.py` | working tree | now | 0, clean over 829 files |
| `python3 scripts/source_provenance_check.py` | working tree | now | 0, clean over 1648 files |
| `python3 scripts/guard_census.py` | working tree | now | 0, "0 watched by nothing" |
| Mutations M1 to M11 on `levels.rs`, `cargo test -p ocelli-volume --test pyramid` | each mutant applied to a byte copy of the reviewed file, then restored | now | Red: M1, M2, M3, M4, M5, M7, M8, M9, M10, M11. Survived: M6 (nitpick N1). The restored file is byte-identical to the copy, and `git diff` is byte-identical to the starting diff |

The mutation list:

| ID | Mutation | Tests that went red |
|----|----------|---------------------|
| M1 | `!is_nan` | AC2, AC3, AC5 |
| M2 | NaN constant `0x7fc0_0001` | AC3 |
| M3 | divide by `Dx*Dy*Dz` | AC1, AC2, AC5, AC6 |
| M4 | clearing never matches | AC4 |
| M5 | `any` never set | 7 tests, including `fixture_float_mean` |
| M6 | `!any` branch removed | none, it survived |
| M7 | weight +1 per sample | AC5, AC6, `fixture_float_mean` |
| M8 | f64 `+ 0.0` normalisation, no post-narrow clearing | AC4 |
| M9 | `abs` | AC4 control, `fixture_float_mean` |
| M10 | source from the last level | AC6 and the F-119 level-0 fixture |
| M11 | empty box narrows an input word | AC3 |

## Defects

### D1, AC6's doc comment credits the operator's answer with a rule it does not contain

**Where**: `crates/ocelli-volume/tests/pyramid.rs:1288`

**What**: "The operator's answer of 2026-11-05 re-weights each level from level 0." The plan quotes the operator's answer verbatim, and it says nothing about which level is the source. "Every level is computed from level 0" is F-119 decision 2, and F-222 decision 9 keeps it.

**Why it is wrong**: the microscope's severity table says a false claim in prose is a defect. CLAUDE.md requires a fixture to cite its real source, and this sentence names the wrong one.

**Evidence**: compare the plan's "Normative source, transcribed" section (the answer, and F-119 decision 2) with line 1288.

**Scope and regression**: this is a test comment only, and the behaviour is correct. The smallest fix is to cite F-119 decision 2, kept by F-222 decision 9, for the level-0 sentence.

### D2, AC9's doc comment describes binary32 samples that the test does not have

**Where**: `crates/ocelli-volume/tests/pyramid.rs:1315` to `1316`

**What**: the doc comment of `non_finite_centres_follow_ieee_arithmetic` begins "Samples are IEEE 754 binary32 words, DICOM PS3.5 section 6.2 VR OF, in the Float Pixel Data layout of PS3.3 C.7.6.24." Both volumes in that test are `VoxelKind::U8`, built by `bare(..., VoxelKind::U8)` and by `filled(..., VoxelKind::U8, &repeat_u8(8, 1))`. The non-finite values are f64 coordinates, not stored samples. The boilerplate citation from AC1 to AC6 was copied onto a test where it is false.

**Why it is wrong**: a false claim in prose is a defect. It also blurs the very distinction AC9 exists to draw, decision 8, that the sample rule does not apply to coordinates.

**Evidence**: read lines 1323 to 1337.

**Scope and regression**: this is a test comment only. The smallest fix is to delete the sentence, or to replace it with one saying the inputs are f64 level-voxel coordinates under F-119 decision 12.

## Smells

None.

## Nitpicks

- **N1**: the `!any` early return is an equivalent mutant today (M6 survived). With no finite sample, `sum / weight` is 0/0, and `narrow_f32` already writes `0x7fc0_0000` for any NaN. AC3 therefore cannot tell the flag from `narrow_f32`'s NaN branch. The flag remains a correct defence if `narrow_f32`, which the oblique sampler shares, ever changes. A comment saying so, or a unit test on `mean_f32` against a stub, would make that dependence visible. This does not block.
- **N2**: `0x8000_0000` in `mean_f32` is an unnamed literal next to the named `CANONICAL_NAN_BITS`. A `NEGATIVE_ZERO_BITS` constant, or `narrowed == 0.0` written through `to_bits()` with `0x8000_0000` named, would read more evenly.
- **N3**: AC3's doc comment, "`[2, 1, 1]` of `+inf, +inf`, of `-inf, -inf`, and of `+inf, -inf` are the same empty box", reads as though the three volumes were one box. "Each is a box with no finite sample" says what is meant.

## Verified clean

- **Arithmetic**:
  - There is no `as` cast anywhere in the diff.
  - `factor` is the product of three u32-bounded integers as f64, unchanged from F-119.
  - The finite weight is exact for decision 5's reason.
  - Finiteness is tested on the `f32` from `read_f32`, before `f64::from`, so a signalling NaN is never widened.
  - The empty-box NaN comes from the named constant, positive, with the quiet bit as the only fraction bit, and never from arithmetic or an input word.
  - The negative-zero clearing tests the bits after `narrow_f32` and changes only `0x8000_0000`.
  - The f64 mean can never be `-0.0` before narrowing, because the sum starts at `+0.0` and exact cancellation gives `+0.0` under round-to-nearest-even. So `narrow_f32` is the only source of a negative zero, as the comment on line 413 says.
- **No float comparison**: the `any` flag is a bool and the zero test is on bits, so `clippy::float_cmp` is satisfied.
- **Doc comments on `downsample_level` and `generate_pyramid`**: they match the code. `generate_pyramid` reaches `mean_f32` through `self.downsample_level(dims)`.
- **Prose claims in the `levels.rs` comments**: lines 380 to 381, 400, 409 and 413 are true against the code.
- **Regressions**:
  - `mean_i128`, `round_ties_away`, `fold_parents`, `axis_weight`, `narrow_f32` and `level_voxel_to_level0` are untouched.
  - `fixture_float_mean`, including `lost_units`, is unchanged and green.
  - The old pin is replaced with the name the plan gives.
  - The new `f32_bits_at` helper uses checked arithmetic and bounded `get`.
- **User docs**: `README.md`, `use.html` and `more.html` contain no pyramid, non-finite or negative-zero sentence, so the diff leaves nothing in them stale.

## Limitations and handoff

- Only native aarch64-darwin was executed. Byte identity on wasm32 and on x86-64 is argued from the code (AC11), not observed, and `bin/ocelli.sh native` was not run.
- The semantic RED against the pre-change `mean_f32` was not replayed. Mutations M3 and M5, plus the plan's arithmetic, stand in for it.
- AC10's LLD sentence at `docs/lld/volumes.md:152` remains for `/complete-feature`.
- Next action: the implementer fixes D1 and D2, and a fresh independent pass 2 reviews the changed lines.
