# F-227 review, pass 6

**Reviewed**: library half only. `git diff 369835d HEAD` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `df768df8b9e112586e7ccc43857206f8a09a8a5c` (the lead's pass 5 corrections `3d73f11` and the close-pair round `df768df`), `git write-tree` `09d33a95f7d00dae2027ace676b07aa6e43bc14f`, no unstaged tracked changes. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `546679d2`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `fbbfdb94`, `.claude/plans/F-227-design.md` sha1 `706ecba3`. The plan's amendments through "Lead rulings on the pass 5 remediation" are treated as authoritative, and the rulings themselves are judged for soundness as the brief asks. Untracked `tools/oracle/out` and the pass 1 to 5 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the round is by a Claude implementer, and the corrections, amendments and rulings are the sprint lead's, both attributed by the caller's brief. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or rulings. Passes 1 to 5 were written by the same model, so this pass is independent of the remediation but not of the findings it answers. The implementer's "five mutations" are not recorded anywhere in the tree, so the five replayed below are the five this round's code most obviously needs, chosen before running, and three more are mine.
**Result**: 3 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **The close-pair rule on the CPU.** Percentiles d = 1, 2 and 3 f32 units apart, slope 1, 200 random significands per binade for every exponent from 2^-40 to 2^60, both signs: 121,200 frames. Every accepted window satisfies its contract with all four points recomputed in the probe rather than read from the crate. d = 1 and 2 are Constant, the centre draws exactly 127.5, and the 1st, the 99th and each moved 16 units outward draw strictly inside (127, 128). d = 3 is Percentile, and `low`, the moved low, the moved high and `high` draw exactly 0.0, 0.0, 255.0, 255.0. **0 contract violations.** Refusals: none below 2^34, and **every d = 1 and d = 2 pair from 2^34 up refused** (see S1). The same sweep against the pass 5 source refuses nothing and accepts every close pair as a Percentile window.
- **Under the tier A emulation, `f32::mul_add(stored, slope, intercept)`.** Pass 5's small-slope, large-intercept family on two seeds of my own, 1.6 million draws: **0 flips at any d**, including d = 0 (91,279 and 92,057 windows at d = 1, 44,761 and 44,427 at d = 2, 30,534 and 30,948 at d = 3), and no fused value more than 16 units from its CPU value. Pass 4's pair, stored 2371 and 2372 at slope 9.00603e-7 and intercept 1854.0933, is now Constant at width 17/16 and its fused values, both `0x44E7C30D`, draw 127.4707, grey 127. Pass 5's second example, 1988 and 1989 at slope 7.6754475e-7 and intercept 1736.4397, is Constant at width 1.0625 and its fused values draw 127.529305, grey 128. The cancellation family still flips, 260 of 400,000, every one with the fused value about 1,000 units or more from the CPU value, which is outside any margin and which the plan's "within the margin" wording now covers honestly. The 2^23 widening case is fixture-checked below.
- **Zero refusals on eight seeds of my own.** Seeds `0x6A09E667`, `0xBB67AE85`, `0x3C6EF372`, `0xA54FF53A`, `0x510E527F`, `0x9B05688C`, `0x1F83D9AB`, `0x5BE0CD19`, none of them among the test's twelve. 100,000 pairs per seed in each of the three pair distributions with the test's generators, plus the 2,000-slice PET series and its pooled window: 2,416,008 windows, **0 refused, 0 contract violations**, and 23 close-pair Constant windows among them. Under `mul_add` the pair distributions give 0 Percentile windows whose fused percentiles miss grey 0 or 255 and 0 close pairs leaving (127, 128).
- **The three rulings, judged.**
  - *A close pair is checked at both percentiles and outward.* Sound. At 2^23 and 8388610, width 1 draws the centre 127.5 and the percentiles 0.0 and 382.5, executed, so a centre-only check accepts a black and white window. Mutations M3 and M4 confirm that the fixture catches it.
  - *The middle grey means grey 127 or 128.* Sound. 127.5 is the tie of `floor(y + 0.5)`, so any tier error downward gives 127 and any upward gives 128, and no window can make one grey on every tier. The half-grey tolerance is stricter than grey 127 or 128 needs at the checked points (see N2).
  - *A stated limit.* Not sound as written. The limit is real, but it starts at 32768 and not near 1e9. See D1.
- **New fixtures, re-derived by hand.**
  - 10.0, d = 1: the spacing in [8, 16) is 2^-20, and the half span 2^-21 ties onto the even 10.0. The 99th is `0.5 + 2^-20` exact in [0.5, 1), and times 255 it is `127.5 + 31.875 * 2^-17`, which rounds in [64, 128) to `127.5 + 2^-12`. Executed: centre `0x41200000`, width 1, draws 127.5 and 127.500244, outward 127.49611 and 127.504135. Every sentence holds.
  - 10.0, d = 2: the midpoint is `0x41200001` exactly, and the 1st draws `127.5 - 2^-12` = 127.499756, grey 127. Holds.
  - 10.0, d = 3: the margin moves 1 unit each. The centre `L1 + 2^-21` ties onto the even L2, and both edges tie onto L2, so L2 draws ymin and the computed window fails. Below 2^-20 the unit is 2^-44, so the first narrowing is `0x357FFFFF`. Its half is under the tie, so the centre and both edges are L1. Executed: centre `0x41200001`, width `0x357FFFFF`, draws 0, 0, 255, 255. Holds.
  - The pass 4 pair at width 17/16: CPU `0x44E7C30D` and `0x44E7C30E` and fused `0x44E7C30D` for both, executed. The midpoint ties onto the even `0x44E7C30E`. 17 units draw `4335 / 8192 / w = 0.529175 / w`. At k = 19, w = 1.0625, they draw 0.498047 below, 127.00195, executed. The half width 17/32 is 4352 units of 2^-13, so the edges are exact. The 1st draws 127.4707, grey 127. Holds, apart from k = 18 (D3).
  - 2^23 at width 16385: the midpoint is 8388609. At width 1 the edges tie onto 8388608 and 8388610, drawing 0.0 and 382.5, executed. The 1st moved 16 units down crosses 2^23 into spacing 0.5 and lands on 8388600, executed, so 17 units above is the binding side and `w > 4335 * 2 = 8670`. Executed: k = 36, w = 8193, high_out 128.02911, a gap of 0.52911, fails. k = 37, w = 16385, gives 127.48444 and 127.51556 and passes. The half width 8192.5 and the lower edge 8380416.5 are exact. "Would stop at k = 32, w = 513, where one unit draws 0.49708" was executed: 127.00293 and 127.99707, with k = 31 at 126.50778. Every sentence holds.
- **Executed comments and plan sentences this round changed.** The N3 comment at `auto_window.rs:411-418`: the mutants removing the check at the unmoved `low` (P1) and at `high` (P2) still survive. The edge-branch half is implied by comparison, the ramp is a composition of monotone f32 operations, and "measured, not proved" is the honest word. The case it covers is not rare: in 78,937 of 2,999,594 accepted windows the unmoved 1st sits strictly above its lower edge on the ramp and still draws exactly 0.0. The test module header's "two moves together are at most 2^-11 = 4.9e-4 ... within 0.001" holds. The source doc's "one unit for percentiles 3 or 4 units apart" holds by `min(16, (d - 1) / 2)`. "At width 1 that holds for magnitudes below 1024, where 17 units are at most ... 0.26 grey" holds at 17 * 2^-14 * 255 = 0.2646. The `close` predicate's comment holds. The plan's pass 4 amendment edits hold. Two clauses of the corrected pass 4 ruling do not (D2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | tree `09d33a95` | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, 99 tests over 10 binaries, `auto_window` 30 passed in 5.9 s |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0, "0 watched by nothing" |
| probe crate `target/f227-p6/probe`, path dependency on the worktree's `ocelli-pixel` and `ocelli-core` | real `auto_window`, `AutoWindowPool`, `AutoWindow::transform`, `ModalityTransform::apply`, `VoiTransform::new` and `apply` | now | modes `fixtures`, `close`, `fused`, `dist`, `const`, `d0`, `edges`, `edges2`, `edges3`, results above and below |
| the same probe against `git show 369835d:crates/ocelli-pixel/src/auto_window.rs` in a scratch copy | pass 5 source | now | close sweep: 0 refusals at every exponent, every d = 1 and 2 pair a Percentile window |
| 13 source mutations in a scratch copy of `ocelli-core` and `ocelli-pixel`, applied by exact-anchor replace with the count asserted 1, `cargo test --test auto_window`, original restored and compared with `cmp` after the run | `auto_window.rs` sha1 `546679d2` | now | table below |

| Mutation | Result |
|---|---|
| M1 close at one step, `high <= low.next_up()` | red, `percentiles_fewer_than_three_units_apart_take_the_constant_window`, `a_close_pair_above_2_23_is_widened_until_it_draws_the_middle_grey` |
| M2 close at three steps | red, `percentiles_fewer_than_three_units_apart_take_the_constant_window` |
| M3 the outward values dropped from the check | red, the 2^23 test and `the_pass_4_tier_a_pair_draws_the_middle_grey` |
| M4 the close pair checked at the centre only | red, the same two |
| M5 a close pair centred on `low` instead of the midpoint | red, 3 tests |
| M6 (mine) `outward_pair` moves 15 units | red, the 2^23 test and the pass 4 test |
| M7 (mine) `CONSTANT_TOLERANCE = 0.51` | **survived** (N2). 0.52 and 0.4 are red on the pass 4 test |
| M8 (mine) the outward check for equal percentiles too, ruling 3's alternative | red, `a_constant_frame_draws_the_middle_grey_at_large_magnitudes`, `a_constant_beyond_the_search_reach_is_refused` |
| M9 non-strict tolerance, `>=` and `<=` | **survived**, equivalent for grey 127 or 128 (N2) |
| M10 only the high side outward | red, the pass 4 test |
| M11 only the low side outward | red, the 2^23 test |
| P1, P2 the unmoved `low` or `high` check removed | survived, as the N3 comment says |

## Defects

### D1, the stated limit for constant frames starts at 32768, not near 1e9

**Where**: `.claude/plans/F-227-design.md:482`, "A constant frame at very large magnitude, 1e9 for example, is accepted at a width of tens of units, and a tier whose rescale rounds a unit differently there can draw it black or white. No DICOM rescale in practice produces such frames." Read with `:481`, "What is guaranteed for a constant or close-pair window is a display value within half a grey of 127.5".
**What**: a constant frame (d = 0) is checked at its centre alone and is accepted at width 1 for every magnitude below 2^23. A value one f32 unit away then draws `127.5 +/- 255 * spacing`. Executed through `auto_window` and `AutoWindow::transform`:
- Below 2^15 a unit stays within half a grey (16384: 127.998 and 127.251).
- **From 2^15 = 32768** a unit leaves ruling 2's half grey (32768, width 1: next up 128.4961).
- **From 2^16 = 65536** it leaves grey 127 or 128 (65536, width 1: 129.49219, grey 129).
- **From 2^22 = 4194304** it draws black or white **at width 1** (4194304: next up 255.0, and 4194305: 255 and 0).
At 1e9 the sentence's description is accurate: width 65, and the neighbours draw 378.6 and 0. The limit as stated starts four and a half decades too high, and the "tens of units" mechanism is not the one that operates where it starts. A tier case, emulated with `mul_add`: a constant frame of stored 36000 at slope 3.1631906 and intercept 809.33826. The CPU gives 114684.195 and the fused rescale 114684.2, one unit apart. The window is width 1, and the fused value draws 129.49, grey 129. In a family of stored 1 to 65535, slope 0.5 to 3.5 and intercept -1024 to 1024, 208,531 of 1,351,548 constant frames above 32768 leave grey 127 or 128 under the fused rescale. The rule is inconsistent too. The pair {65536, next up}, d = 1, is widened to width 129 and protected to 16 units, while the constant {65536} keeps width 1 (probe mode `d0`).
**Why it is wrong**: microscope section 3, and CLAUDE.md's tier rule. Ruling 3 is the authoritative statement of where the guarantee in ruling 2 stops holding, and it places that point at 1e9. Modality values from 32768 to a few million are inside the range a 16-bit stored value and an ordinary slope produce.
**Evidence**: probe modes `const` and `d0`, above.
**Scope and regression**: present on HEAD. No consumer draws an automatic window on tier A yet. A tier difference here needs a non-zero intercept and an inexact product, because with intercept 0 the fused and unfused results agree. So unsigned CT at slope 1 and PET at intercept 0 do not trigger it, and I cannot establish from the corpus how often anything else does. Owner: the lead. Either restate the limit with the measured onsets, a half grey from 2^15, outside grey 127 or 128 from 2^16, black or white from 2^22 at width 1, or apply `outward_pair` to d = 0 as well. M8 shows that the second option costs refusing constant frames from 2^34 up and rewriting two constant fixtures. The smallest regression is a fixture pinning a constant at 65536 together with whichever behaviour the lead chooses.

### D2, the corrected pass 4 ruling still states two bounds the windows break

**Where**: `.claude/plans/F-227-design.md:467`, in `3d73f11`: "so each percentile lies outside its edge or on it, and draws the output minimum or maximum. The window is narrower than the span by at most 32 f32 units of the percentiles".
**What**:
- **"Each percentile lies outside its edge or on it."** In 78,937 of 2,999,594 accepted Percentile windows, the unmoved 1st percentile lies strictly above the lower edge, on the ramp. These come from four families (uniform, near-adjacent pairs, mixed-magnitude pairs, CT-like): 2,242 of 750,000 in the uniform family, 71,962 in the mixed-magnitude family and 4,733 in CT-like. In 2 windows the 99th lies strictly below the upper edge. Uniform example: low `0x3F7C1A48` = 0.984776, high `0x42B81A12`, centre `0x423A1236`, width `0x42B621CD`, lower edge `0x3F7C1A40`, 8 units below the 1st. CT-like example: low `0xC2E06860` = -112.20386, lower edge `0xC2E06880`, 32 units below it. Every one still draws exactly 0.0 or 255.0, so the second half of the clause holds as a measurement. The first half is false.
- **"At most 32 f32 units."** Measured worst: low `0xC47DE87B` = -1015.6325, high `0x445F098A` = 892.14905, width `0x44EE78F2` = 1907.7795. That is narrower than the span by **exactly 33 units** of 2^-14. The width is in [1024, 2048), where its own unit is two percentile units, so rounding it adds up to one unit beyond the 32 the margin moves. 418,807 windows exceed 32 by some fraction of a unit. Pass 5's D2 proposed "plus the edge's rounding", and the correction left that clause out.
**Why it is wrong**: microscope section 3. This is the lead's correction of pass 5's D2 and it carries the same shape: a bound stated without the rounding term that the measurement shows.
**Evidence**: probe modes `edges`, `edges2` and `edges3` over 3,000,000 draws, and the exact narrowing recomputed in Python from the bits.
**Scope and regression**: no visible effect. The 33rd unit is 6.1e-5 HU, and every percentile inside its edge draws its extreme exactly. Owner: the lead. The smallest fix deletes the two clauses. Otherwise "each percentile draws the output minimum or maximum, through an edge branch or exactly on the ramp" and "by at most 32 units plus the rounding of the width, 33 measured".

### D3, a figure in the pass 4 fixture's derivation is wrong

**Where**: `crates/ocelli-pixel/tests/auto_window.rs:714`, "up to k = 18, w = 1.03125, the 17 units draw 0.51306 below and fail".
**What**: `0.529175 / 1.03125 = 0.513139`, and `VoiTransform::apply` gives 126.98686, 0.5131378 below. Neither is 0.51306. The conclusion, a failure at k = 18, holds.
**Why it is wrong**: HLD 27.2 R2 and microscope section 2. The derivation is the fixture's hand-computed evidence, and a figure nobody computed weakens the claim that the rest were computed.
**Evidence**: probe mode `fixtures`, `k 18 w 1.03125 lo_out draws 126.98686 (127.5 - 0.5131378)`.
**Scope and regression**: comment only. The fix is one number, 0.51314.

## Smells

### S1, close pairs from 2^34 up are refused, a new refusal that no sentence and no test states

**Where**: `auto_window.rs:308-313` and `:430-451`, against `AutoWindowBasis::Constant`'s doc at `:103-107` and the reach sentences at `:315-324` and plan `:453`.
**What**: a close pair's outward points are 17 units from the centre, so the check needs `w > 8670 * spacing`. Widening from width 1 reaches 2^24. Above 2^34 the spacing is 2048 and 17.76 million exceeds 16.78 million, so **every d = 1 and d = 2 pair from 2^34 up is refused**. The probe found all 400 of each d refused per binade from 2^34 to 2^60, and none below. At `369835d` every such pair was accepted as a Percentile window. The doc says "from 1024 up the search may widen" and places the Constant basis's refusal "above 2^48", and that basis now includes close pairs. No test pins the 2^34 boundary.
**Failure mechanism**: the viewer half, and any later reader, will take the documented reach of a Constant window as 2^47 to 2^48 for the basis as a whole. This review would have accepted the round's refusal-free distributions as evidence that the reach was unchanged. A change to the margin or the tolerance moves the boundary with no red test.
**Scope and regression**: no DICOM rescale produces modality values of 1.7e10, and refusal is the safe outcome. The smallest fix is one sentence in the doc giving the close-pair reach and its arithmetic, plus a fixture refusing a d = 1 pair at 2^34 and accepting one at 2^33.

## Nitpicks

- **N1** `auto_window.rs:307-308` and the `Constant` doc: "for 1 unit is a tie and rounds onto whichever has the even significand". Below 2^-125 the span is the least subnormal, and `span / 2.0` rounds to zero before the sum, so the centre is always `low`. Stored `0x3` and `0x4` give the centre `0x3`, executed. Frames at 1e-38 do not occur.
- **N2** M7 (`CONSTANT_TOLERANCE = 0.51`) and M9 (non-strict comparisons) survive. The pass 4 fixture pins the tolerance to [0.4981, 0.5131), since 0.52 and 0.4 are both red. Any tolerance below 1.0 keeps the checked points at grey 127 or 128, so neither survivor breaks the grey guarantee. This is a note on why the exact 0.5 is unpinned, not a gap.
- **N3** The `finish` doc comment was reflowed to 100 columns while the rest of the file wraps near 80. That makes the diff harder to read against pass 5, and rustfmt does not wrap comments, so nothing will reconcile them.
- **Carried**: pass 4 N1, and pass 1 N1 to N5 and pass 2 N2 and N3, unchanged. Pass 5 N1 and N2 are closed. Pass 5 N3 is closed by the comment at `:411-418`.

## Verified clean

- Pass 5 D1 is closed for the case it named. Pairs one or two units apart are Constant windows, and no close pair in the small-slope family flips under `mul_add` at any d. The plan's percentile sentence is now conditional on "within the margin", which the cancellation family respects. Pass 5 D2's first clause is closed: the 2.5e-4 bound is gone.
- `close` matches "1 or 2 next_up steps above low" across zero and across binades. The midpoint for d = 2 is exact, including across a power of two. For d = 1 it is a tie to even above 2^-125.
- `outward_pair` and `near_middle` match their docs. The check is strict, the centre is checked for exactness, and NaN or an invalid transform returns false.
- The LUT chain exists once. `near_middle` goes through `display_bits`, which calls `VoiTransform::apply`, and no window or ramp formula is written in `auto_window.rs`. Neither file contains an `as` cast, `unsafe` or `#[allow]`.
- Every mutation except M7, M9, P1 and P2 is red, including both close thresholds, both one-sided outward checks, the 15-unit outward move and the low-centred midpoint.
- No consumer outside `ocelli-pixel` names `auto_window` or `AutoWindow`. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched by the diff and describe nothing it changed.

## Limitations and handoff

- Not exercised: a real GPU. The fused stage 1 was emulated with `f32::mul_add`, which pass 4 showed bit-identical to Metal for its inputs. Tier B, the viewer half, the oracle and real PET or CT bytes were not exercised either. The implementer's own five mutations were not available as text, so the five here are mine, named before running.
- `target/release/probe`, dated 12:05 and outside the tree, is a leftover from pass 5's probe build in the shared target directory, and pass 5's report says its scratch was deleted. It is ignored by git and was not touched by this pass.
- Next owners: the lead restates or changes ruling 3 (D1) and the pass 4 ruling's two clauses (D2), and the implementer corrects one figure (D3) and states and pins the close-pair reach (S1). A fresh independent pass 7 then reviews the changes. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` is still `09d33a95`, the three file sha1s are as listed, and the scratch directory `target/f227-p6` was deleted at the end.
