# F-227 review, pass 12

**Reviewed**: library half. `git diff 90f02b6 767dd54` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `767dd54` ("F-227, item 9 no longer states the two-value window"), `git write-tree` `5a43bc78d8f4b0deabe78bc0a9a53422c2136398`, no unstaged tracked changes. The diff is one line of `.claude/plans/F-227-design.md`, line 157. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, and the plan sha1 `f0cc136c`. Untracked `tools/oracle/out` and the pass 1 to 11 reports are not part of the review.
**Author / reviewer**: the edited line is the sprint lead's, as the caller's brief says. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 11 were written by the same model, so this pass is independent of the correction but not of the findings it answers.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 11 D1, line 157.** The bullet now reads "Two counted values: ranks 1 and 2." For `n = 2`, `2.div_ceil(100) = 1` and `(99 * 2).div_ceil(100) = 2`, source lines 398 to 399. Through the real `auto_window`, stored 3 and 9 give counted 2, low 3, high 9 (Percentile, centre 5.9999943, width 5.999981), 1.0 and its `next_up` give counted 2, low and high the two values (Constant), 5.0 and two units up give the two values (Constant), and 5.0 and three units up give the two values (Percentile). The ranks pick the two values under the moved pair and under the close-pair rule alike. **The sentence is true and D1 is closed.** It now states no window, so neither rule can falsify it.
- **The other item 9 bullets, lines 155 to 160, against the code and tests**, each checked against the moved-pair window (pass 4 rulings) and the close-pair constant window with its outward check (pass 5 rulings, extended to constants by the pass 6 amendment):
  - *Constant, line 155.* `center = low` holds: the Constant search keeps the centre the constant (`|_| constant`, source line 472), and the probe gives centre equal to the value at every magnitude tried. Grey 128 holds on tier C: the exact-middle check gives display 127.5. **`width = 1` is false from 2047.9982 up**, D1.
  - *Background, line 156.* Its "Exact while `|m_max| < 2^22`" and the width it rests on are withdrawn by name by pass 1 bullet 3, as passes 9 and 11 recorded. `m_max` matches `left_out_max`, the largest modality value over left-out sampled values, and the check `maps_to_minimum` is on it. Not reopened.
  - *Signed, line 158.* No signedness branch exists in `add_frame` or `finish`. AC8's signed frame is the test. True.
  - *Non-finite, line 159.* `ModalityTransform::new` refuses a non-finite slope or intercept and a LUT descriptor refuses non-finite entries (`lut.rs:82`, `lut.rs:233`), so a finite stored value gives a non-finite modality value only by overflow. The check runs on every sampled value before exclusion (source line 264, test `a_left_out_sample_is_refused_when_not_finite`). No caller of `auto_window` or `AutoWindowPool` exists outside `ocelli-pixel`, so no Float Pixel Data reaches it. True.
  - *Refusals, line 160.* `new(0)` gives `FrameCount`, an empty slice `EmptyFrame`, a frame past the declared count `TooManyFrames`, `finish` with none added `EmptyPool`, all asserted in `auto_window_pool_refusals`. True.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff 90f02b6 767dd54`, `git diff --stat 90f02b6 HEAD`, `git write-tree`, `shasum` | HEAD `767dd54` | now, start and end | one line of the plan. Tree `5a43bc78` at both. Source and test sha1 unchanged from pass 7 |
| probe crate `target/pass12-probe`, path dependencies on `ocelli-pixel` and `ocelli-core`, `CARGO_TARGET_DIR` the worktree's `target`, release build, deleted with its build outputs after the run | the real `auto_window` and `AutoWindow::transform` | now | constants of 4 samples, slope 1: 1023, 1024 and 2047 width 1. 2047.9998, 2048 and 3000 width 2. 5000 width 5. 65535 width 33. Every one centre the value and display 127.5. Walking every f32 from 1 up, the first constant whose width is not 1 is 2047.9982 (`0x44FFFFF1`), width 1.0625. The two-value cases listed above |
| `cargo test -p ocelli-pixel` | tree `5a43bc78` | now | every binary ok, 0 failed. `auto_window` 32 passed in 5.83 s |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `tests/auto_window.rs:1639`, `a_constant_frame_draws_the_middle_grey_at_large_magnitudes` | unchanged test | now, inside the run above | asserts widths 8193, 16385, 16385, 32769, 16385 and 524289 for six constants, bitwise |

## Defects

### D1, item 9 says a constant window has `width = 1`, which the outward check makes false from about 2048

**Where**: `.claude/plans/F-227-design.md:155`, item 9: "Constant counted values, `high - low` not greater than zero: `center = low`, `width = 1`, grey 128."
**What**: since the pass 6 amendment a constant takes the close pair's outward check, its value moved 16 f32 units each way drawing within half a grey of the middle. A point D from the centre draws `255 * D / w` from the middle, so width 1 holds only while 16 units are under `1 / 510`. From 2048 a unit is `2^-8 / 16`, 16 units are `2^-8`, and the search widens. Through the real `auto_window`, a constant 2048 or 3000 gets width 2, 5000 width 5 and 65535 width 33. The first widened constant is 2047.9982 (`0x44FFFFF1`). The test that owns the case asserts widths 8193 to 524289 for its six values. The centre and grey half of the sentence are true and the width half is false.
**Why it is wrong**: microscope section 3, a false claim in prose, the same unscoped exact claim in the same item as pass 11 D1. The amendments that bear on it do not withdraw it the way pass 1 bullet 3 withdraws the background sentence by name. Pass 2 adjusts the constant window only "when f32 rounding at large values would put the value on an edge", pass 3 speaks of constants "above 2^24", and pass 6 leaves the magnitudes to the source. A chronological reader concludes the width is 1 below about 2^24, where it is not from 2048. Constant modality values of thousands are ordinary: CT bone and metal in HU, unscaled MR, PET in Bq/ml.
**Evidence**: the probe rows above, printed with `{}` through the real library on HEAD, and the green `a_constant_frame_draws_the_middle_grey_at_large_magnitudes`.
**Scope and regression**: prose only. Present since the pass 6 amendment for constants from 2048, and from 2^23 since the pass 2 check, which the test comment's pre-pass-6 widths (1.0000001 at 8388610) show. Passes 9 to 11 swept the Constant line and checked its grey, not its width. No code or test is affected and nothing is red. A viewer test or the LLD subsection written from item 9 would assert width 1 for a constant 3000 frame and go red on the correct library. The smallest fix is deletion: drop "`width = 1`" and leave `center = low`, grey 128. AC8's "16 of stored 7: Constant, centre 7, width 1" and AC10's one stored 50 hold, since both are below 2047.9982, and need no change. "This keeps the pinned one-sample fixtures" holds for the same reason.

## Smells

None.

## Nitpicks

- **N1** line 155's label "Constant counted values" is narrower than its condition. `high - low` not greater than zero is equal percentiles, and `constant_basis_is_equal_percentiles_not_constant_values` takes 200 of stored 7 and one 1000 into this window, as the `AutoWindowBasis::Constant` doc comment says. The condition the bullet states is the right one, so no reader computes a wrong window, but the label invites one to expect a uniform frame. If line 155 is edited for D1, "Equal percentiles" would match the code.
- **Carried**: pass 11 N1 (pass 10 N2), pass 9 N1 to N3 and the pass 7 nitpicks pass 9 carried, unchanged because their text and code are unchanged.

## Verified clean

- Pass 11 D1 closed. Line 157 is true for every two-value case probed, Percentile and Constant.
- Item 9's signed, non-finite and refusal bullets, against source and tests, as listed under Acceptance challenged. The background bullet's withdrawal by pass 1 bullet 3 confirmed present at line 437.
- Item 9's Constant bullet, apart from D1: `center = low` and grey 128 at every magnitude probed.
- Not repeated: pass 11's sweep of the rest of the plan (items 1 to 21, the reproducer, the API block, the AC table, the amendment sections), because only line 157 changed since. Items 8 and 9 and the amendment sections at lines 431 to 487 were re-read for this pass's question only.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` pass.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half (not written), the oracle, real bytes. No mutation was run, because no code changed since pass 7's mutation table and the finding is prose.
- Next owner: the lead corrects line 155 (D1), prose only, and a fresh independent pass 13 reviews it. This is a bounded review of the library half and item 9, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` `5a43bc78` and the three sha1s were re-checked at the end. The probe lived under the worktree's ignored `target/` and was deleted with its build outputs.
