# F-227 review, pass 2

**Reviewed**: library half only. HEAD `4458d5fec5b2150ba06b5eb03cf8a192100e155e`, branch `work/f-227-xai`, worktree `ocelli-wt-f-227`. Staged tree `e50fe64f4d1f1340e47c4775ca479c7b05e39113` (pass 1's tree) plus unstaged changes. Working files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `27623bb0`, `crates/ocelli-pixel/src/lib.rs` sha1 `d90ec5ff` (unchanged since pass 1), `crates/ocelli-pixel/tests/auto_window.rs` sha1 `c2388431`, `.claude/plans/F-227-design.md` sha1 `76eceef0` (the lead's "Lead amendments after review pass 1", which this pass treats as authoritative). Untracked `tools/oracle/out` ignored as instructed. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: remediation by GLM-5.3 in opencode, attributed only by its own report at the end of `glm-F-227-fix1.log` in the session scratchpad. The amendment is the sprint lead's. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed code, none of the plan and none of the amendment. It also wrote pass 1, so it is independent of the remediation but not of the findings the remediation answers.
**Result**: 3 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **The amendment's check is real and single-sourced.** `display_bits` (`auto_window.rs:417-429`) builds `VoiTransform::new(None, &[c], &[w], 0, LinearExact, 0, 255)` and calls `apply`. It has no edge or ramp formula of its own, so the LUT chain still exists once (`lut.rs:651-660`). The widening (`:436-449`) adds one to the width's bits, recomputes `low + width / 2` each step and stops after 64. That matches the amendment's "keeping the centre the midpoint, at most 64 steps".
- **The three pass 1 D1 triggers, re-derived independently in numpy `float32`.** All-padding stored 32768, slope 256: `m_max = 8388608`, the unwidened centre ties to `8388608`, and one widening gives centre `8388609`, lower edge `8388608` and grey 0. Maximum `4194303.75`: the unwidened grey is 63.75, and one widening gives centre `4194304.5`, lower edge `4194304` and grey 0. Adjacent pair `0.001` and its successor: the significand 8589935 is odd, the centre ties up onto the 99th percentile, the ramp there is 127.5, and the tree refuses with `UnverifiableWindow`. All three agree with the tree and with the new fixture comments.
- **Background, swept through the real `AutoWindowPool::finish`.** Every finite `f32` of magnitude in `[2^16, 2^30)` of both signs was run, plus every 4096th bit pattern elsewhere, 235,868,158 values in all. Each was one stored sample left out by its own padding, slope 1, and the returned window's transform was applied to the value. **None was returned without drawing grey 0, and none was refused.** Outside the dense range no tie arises: below `2^16` the half is exact, and above `2^30` the half is below half an ulp.
- **Percentile pairs, swept through the real `finish`.** Two counted samples per frame, 2,000,000 per distribution. **No returned window failed its own check.** Every accepted window drew the 1st percentile exactly `0.0` and the 99th exactly `255.0` through `VoiTransform`. Its centre's bits equalled `low + width / 2`, and its width was within 64 bit steps above `high - low`. The refusal rate is the subject of D1.
- **GLM's claim that "no width of any size maps it" for adjacent percentiles is false.** It holds only for widths at or above the span. With the midpoint rule unchanged, half the span gives a centre `low + w/2` that rounds onto `low`. The lower edge is then `low`, the upper edge is below `high`, and the window draws `low` 0 and `high` 255. This held for all 963,246 adjacent normal pairs sampled, including `0.001`. The in-tree test comment confines itself to widening, and there it is accurate. The report's general claim is not. D1 carries the consequence.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | working files above | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, 90 tests over 10 binaries, `auto_window` 21 passed |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0 |
| numpy 2.x `float32` (`.venv`), the D1 triggers, the PET example, the Constant edge | DICOM C.11.2.1.3.2 formula only | now | agrees with the tree, D1 and D2 below |
| probe crate `f227probe` with bins `bg`, `pairs`, `detail`, `pet`, `narrow`, `adj` and `m22`, a path dependency on the worktree crate, scratch target under `target/f227-scratch` (deleted) | real `AutoWindowPool`, `auto_window`, `AutoWindow::transform`, `VoiTransform::apply` | now | sweeps above, D1, D2, D3 |
| 19 source mutations, `cargo test -p ocelli-pixel --test auto_window`, each applied by exact-anchor replace (anchor count asserted 1) and restored from a byte copy with the sha1 re-checked | `auto_window.rs` sha1 `27623bb0` | now | table below |
| GLM's red runs and its five mutations | GLM report | prior receipt, the five re-run here | M6, M13, M14, M22 and M25 all red now |

Mutations. The first five are pass 1's survivors. R1 to R13 are this pass's own mutations of the new check.

| Mutation | Result |
|----------|--------|
| M6 padding bounds in declared order | red: `padding_bounds_are_honoured_in_either_declared_order` |
| M13 refused frame commits its left-out maximum | red: `a_refused_frame_leaves_the_left_out_maximum_unchanged` |
| M14 stored zero without the sign mask | red: `stored_negative_zero_is_counted_as_stored_zero` |
| M22 finiteness tested only for counted samples | red: `a_left_out_sample_is_refused_when_not_finite` and two more |
| M25 `NonFiniteWindow` check removed | red: `a_non_finite_window_is_refused` |
| R1 percentile check removed | red: `the_window_check_verifies_two_adjacent_percentile_values` |
| R2 background check removed | red: both background-trigger tests |
| R3 `WIDENING_STEPS = 1` | **survived** |
| R4 `WIDENING_STEPS = 100_000` | **survived** |
| R5 widening keeps centre `low + 0.5`, not the midpoint | red: both background-trigger tests |
| R6 percentile check tests the low side only | red: adjacent-pair test |
| R7 percentile check tests the high side only | **survived** |
| R8 check evaluated as LINEAR | red: adjacent-pair test |
| R9 widening by two bit steps | **survived** |
| R10 Background `high` field `center + width` | **survived** |
| R11 background returns the unwidened width 1 with the widened centre | **survived** |
| R12 percentile returns the widened width with the unwidened centre | **survived** |
| R13 background accepts anything but the output maximum | red: both background-trigger tests |

## Defects

### D1, the check refuses a large share of ordinary fractional windows, PET included, because widening cannot reach the 99th percentile

**Where**: `crates/ocelli-pixel/src/auto_window.rs:373-386` (percentile check), `:436-449` (`widen_until`), the doc at `:287-299`, and the plan amendment's first bullet (`.claude/plans/F-227-design.md:435`).
**What**: the percentile window is accepted only if `apply(high)` is exactly `255.0`. When `low + span / 2` rounds, `high` lands on the ramp at `t = (high - c) / w + 0.5` just below 1. The two-value frame `6.9351015, 36.288307`, slope 1, gives width `29.353207`, centre `21.611706` and upper edge `36.288307` (numpy and the tree agree). There `t = 0.99999994` and `y = 254.99998`, which `display_index` draws as grey 255, yet the tree returns `Err(UnverifiableWindow)`. Widening cannot help. In real arithmetic `c - w/2 = low` for every `w` and `t(high) = span / w`, which falls as `w` grows, so the 99th percentile only moves further from 255. Across 4,000,000 mixed probes, widening rescued 30,018 windows and 1,362,438 were refused. In 989,304 of those refusals the unwidened window already drew grey 0 and grey 255.
- Refusal rates by distribution: uniform fractional `[0, 100)` 41.2 percent, `[0, 1)` 20.5 percent, stored integer times a slope in `(0.001, 10)` 41.8 percent, CT-like float rescale with intercept -1024 15.9 percent, integers below `2^23` 0 percent, 1 ulp apart 49.6 percent, 3 ulp apart 100 percent.
- On a synthetic 128 by 128 PET-like frame (stored 0 outside a disc, skewed counts inside), 2,000 per-image slopes from 0.01 in steps of 0.0137 with intercept 0 refuse 190 times. The first refusal is at slope 0.0374. These are the operator's survey series, slope per image and no window, which the story exists for.
- The remedy direction exists and keeps the midpoint rule. Narrowing the width one bit step at a time, with centre `low + w/2`, satisfied 989,150 of 989,262 tree refusals within 64 steps. For the adjacent pair it is the half-span window above, so GLM's "no width of any size" is false (Acceptance challenged).
**Why it is wrong**: PS3.3 C.11.2.1.3.2 puts `high` at `ymax` through `x > c + w/2`. Below the span the upper edge falls under `high`, and above it `high` stays on the ramp, so only narrowing approaches that branch. The amendment asked for a window drawn correctly, but its "widened" direction makes the check a gate that refuses windows rendering already draws correctly. Under plan items 12 and 16, a refused series window falls back to per-image windows, and a refused per-image window is the viewer's `window` refusal. About one PET slice in ten would then not be drawn. That is safe, because nothing is quietly wrong, but it defeats AC1's purpose and is not what the amendment intended.
**Evidence**: probe bins `pairs`, `detail`, `pet` and `narrow` through the real crate. The numpy reproduction of the example is in the session.
**Scope and regression**: every `AutoWindow` consumer, which today is the tests and soon the viewer half. This is a design defect first, because the code follows the amendment's literal "widened", so the lead amends the direction (narrow, or search both ways) and the implementer follows. Smallest regression: `auto_window(&[Stored(6.9351015), Stored(36.288307)], slope 1)` returns a window that draws the first value at `0.0` and the second at `255.0`. Also pin `0.001` and its successor, which narrowing satisfies.

### D2, the Constant window is exempt from the check, and the stated reason is false at large modality values

**Where**: `crates/ocelli-pixel/src/auto_window.rs:291-294` and `:367-369`.
**What**: the doc says "A Constant window is not checked: its single value minus the centre is exactly zero, so the ramp there is exactly half the output range". The ramp is never reached when the lower edge `low - 0.5` rounds onto `low`. That happens for every even value from `2^23`, ties to even, and for every value from `2^24`. Four samples of stored `8388610`, `16777216`, `16777218`, `33554432`, `-16777216` or `1e9`, slope 1, each return `basis: Constant` and draw **grey 0, not 128**. `8388608` draws 127.5. numpy agrees: `float32(8388610) - 0.5 == 8388610`.
**Why it is wrong**: plan item 9 and Q5 fix a constant image at mid grey. The amendment says "No window is returned that the check has not passed", and this window is returned unchecked, by a rule whose justification is the same exact-in-reals argument pass 1 D1 withdrew. Drawing a constant image black is less dangerous than drawing background grey, but the doc sentence is false.
**Evidence**: probe bin `detail`, the `constant` lines.
**Scope and regression**: requires modality values of at least `2^23` whose 1st and 99th percentiles are equal, which is rare. Either verify the Constant window the same way (centre maps to the ramp's middle, or within its own budget), or bound the sentence and state the black result. Smallest regression: four stored `16777216` draw grey 128, or the documented alternative.

### D3, two sentences in the new test comments describe behaviour the tree no longer has

**Where**: `crates/ocelli-pixel/tests/auto_window.rs:520-521` and `:677-679`.
**What**:
1. `the_window_check_verifies_two_adjacent_percentile_values` says "The current code instead returns a window that draws the 99th percentile at grey 0." The current code refuses it, and the test asserts the refusal. The sentence describes the pre-fix tree.
2. `a_left_out_sample_is_refused_when_not_finite` says a pool that checked only counted samples "would carry the infinite value into the left-out maximum and answer with an infinite background centre". I applied that mutation (M22) to the tree and ran an all-left-out frame through it: stored 5, slope 3.0e38, padding 5. The tree answers `Err(UnverifiableWindow)`, because the new background check refuses the infinite centre. It does not return one.
**Why it is wrong**: microscope section 3. These comments are the hand computation HLD 27.2 R2 relies on, and the second one now misstates what the mutation it guards would do.
**Evidence**: probe bin `m22` against the mutated source, which was restored afterwards with the sha1 re-checked.
**Scope and regression**: test prose only. Delete the first sentence. For the second, say that the frame would be accepted, which is what the assertion catches.

## Smells

### S1, the widening's own parameters and returned values are not pinned, and seven mutations of the new check survive

**Where**: `crates/ocelli-pixel/src/auto_window.rs:333-339, 345, 374-385, 410, 439`, and `tests/auto_window.rs:465, 496`.
**What**: R3, R4, R7, R9, R10, R11 and R12 leave the suite green.
- No fixture is a percentile window that widening rescues, so the step count (R3, R4), the step size (R9) and returning the widened pair (R12) are untested on that branch.
- No fixture fails only the low side, so R7 survives.
- Background `high` (R10) is a documented field that nothing asserts.
- The two background fixtures assert `assert_close(window.width, 1.0000001)` with a tolerance of 0.001, which `1.0` also passes. R11 returns the widened centre with the unwidened width, and the test stays green. For these inputs that window still happens to draw black, so the grey assertion cannot see that the returned pair is not the pair the check passed.
**Why it matters**: microscope sections 2 and 4. The amendment names "one f32 step at a time, at most 64 steps", and none of those numbers is pinned. When D1 changes the search direction, nothing here will report a regression in the widening or a returned pair that differs from the checked one.
**Scope and regression**: assert the background width by bits (`1.0000001f32.to_bits()`). Add one percentile fixture accepted only after the search moves the width, with its expected step count hand-derived. Add one fixture that fails only the low side. Assert Background `high`. Each should go red under the corresponding R mutation.

## Nitpicks

- **N1** `display_bits` (`:414-416`) and `finish` (`:289`) say "evaluated in f32 exactly as rendering evaluates it". That is literal for tier C, which is this function. The edge tests are also identical on tiers A and B, because subtraction is correctly rounded and halving is exact in WGSL. The ramp's division is not, and the project's recorded GPU agreement is 2 ULP at 255, so "exactly" is too strong for an interior value accepted at exactly `255.0`. Saying "as `ocelli-pixel` evaluates it" would be true.
- **N2** The check is fixed at `0..255`. The edge-branch results carry over to any range. An interior `t == 1` gives `range + ymin`, which can differ from `ymax` by an ulp for an unusual `ymin`. Every renderer call site in the tree uses `0..255`.
- **N3** Pass 1 N1 to N5 are carried and unchanged. Of these, N3, the 4 MiB reserve for every one-frame pool, matters most for the viewer's thumbnails. It also made the background sweep allocation-bound.

## Verified clean

- **Pass 1 closures replayed.** D1's three triggers (above). D2's three comments now state the true numbers (floor ranks width 98 with centre and low discriminating, interpolation 2.49 and 148.51 with the width discriminating, unsigned CT). D3's `Constant` doc and its outlier fixture (201 samples, ranks 3 and 199, centre 7, the outlier grey 255). D4's `PartialEq` doc is now true: `-0.0 != 0.0` and `NaN == NaN` are pinned. S1's five mutations are all red.
- **No returned window fails its own check.** 235,868,158 background values and 22,000,000 percentile pairs were checked through the real `finish`. Each accepted percentile window keeps `center == low + width / 2` bit for bit, and its width is within 64 bit steps of the span.
- `UnverifiableWindow`'s doc and `finish`'s error list match the code: refusal follows 64 failed steps or a width that overflows. The background path was never refused in the sweep. `AutoWindow::low` and `high` for Background match the code (`low` is the largest left-out value, `high` is `center + width / 2` in the same `f32` operations as the transform's upper edge). `AutoWindowBasis::Background`'s "every sample is `ymin`" holds by monotonicity once the maximum is at or below the lower edge.
- `widen_until`'s `to_bits() + 1` cannot overflow `u32`, because the width is positive and finite, and an infinite step is refused before use. There is still no `as` cast in either file.
- Fixture arithmetic in the new comments, checked in numpy: 0.001's significand 8589935 is odd. `1.0000001` is one bit step above 1. The widened centres `8388609` and `4194304.5` and the lower edges `8388608` and `4194304` are as stated. The `3.0` pair ties to the even `3.0` and is accepted unwidened. The `-0.0`, reversed-padding, refused-frame and `±3e38` fixtures compute as their comments say, apart from the two sentences in D3.
- `lib.rs` is unchanged since pass 1. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched and name nothing new, consistent with the plan's split.
- The tree is unchanged by this review. After the pass, `git write-tree` is still `e50fe64f`, the working-file sha1s are as listed, `git status` shows the same set, and the scratch probe and its target were deleted.

## Limitations and handoff

- Not exercised: the viewer half, AC10 to AC18, any GPU tier, the oracle, and real operator PET bytes. The PET result uses a synthetic frame and synthetic slopes, not the survey files.
- The narrowing result is evidence that a satisfying window exists, not a reviewed design. The search direction, the budget, and whether Constant is checked belong to the lead under the operator's delegation.
- Next owner: the sprint lead amends the search direction (D1) and the Constant rule (D2). The implementer then remediates D1 to D3 and S1, and a fresh independent pass 3 replays this pass's sweeps and the R mutations. This report is a bounded review of the library half, not acceptance of F-227.
