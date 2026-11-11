# F-227 review, pass 3

**Reviewed**: library half only. HEAD `4458d5fec5b2150ba06b5eb03cf8a192100e155e`, branch `work/f-227-xai`, worktree `ocelli-wt-f-227`. Staged tree `e50fe64f4d1f1340e47c4775ca479c7b05e39113` (pass 1's tree) plus unstaged changes. Working files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `87de3aa6`, `crates/ocelli-pixel/src/lib.rs` sha1 `d90ec5ff` (unchanged since pass 1), `crates/ocelli-pixel/tests/auto_window.rs` sha1 `adbc314c`, `.claude/plans/F-227-design.md` sha1 `6f4ed300`, whose "Lead amendments after review pass 2" this pass treats as authoritative. Untracked `tools/oracle/out` ignored as instructed. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: remediation by GLM-5.3 in opencode, attributed only by its own report at the end of `glm-F-227-fix2.log` in the session scratchpad, after an earlier attempt a watchdog stopped. The amendment is the sprint lead's. Review by Claude Opus 5.5 as an independent subagent, which wrote none of the reviewed code, the plan or the amendment. Passes 1 and 2 were written by the same model, so this pass is independent of the remediation but not of the findings it answers, and D3 below corrects one sentence pass 2 itself got wrong.
**Result**: 3 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **No leftover half-finished edit.** The diff touches only the two `auto_window` files beyond pass 1's tree. No duplicated comment block, no `TODO`, no stray probe code, and the percentile path no longer calls any widening helper. `lib.rs` is unchanged.
- **The check exists once.** `display_bits` (`auto_window.rs:441-453`) builds `VoiTransform::new(.., LinearExact, 0, 255)` and calls `apply`. It has no edge or ramp formula of its own. `lut.rs:651-660` and `voi.wgsl:72-76` write the same edge expressions, `c - w / 2` and `c + w / 2` with `<=` and `>`.
- **Every returned percentile window passes, swept through the real `auto_window`.** 2,000,000 two-sample frames and 200,000 frames of 2 to 401 samples per distribution, with percentiles ranked independently in the probe. Every accepted window drew the 1st percentile exactly `0.0` and the 99th exactly `255.0` through `VoiTransform`. Its centre was bit for bit `low + width / 2` (`low + span / 2` at step 0), and its width was 0 to 64 bit steps below `high - low`. No window broke one of those rules. Step 64 itself is reached and accepted. The refusal counts are D1.

| Distribution | Frames | Refused | First passing step of each refusal |
|---|---|---|---|
| uniform fractional [0, 100), slope 1, pairs | 1,999,999 | 78 | all 65 |
| same, 2 to 401 samples | 200,000 | 0 | |
| stored integer times slope in (0.001, 10), pairs | 2,000,000 | 120 | 65 to 257 |
| same, 2 to 401 samples | 200,000 | 1 | 257 |
| CT-like, slope [0.5, 2), intercept -1024, pairs | 2,000,000 | 12 | all 128 |
| same, 2 to 401 samples | 200,000 | 0 | |
| PET-like, the test's frame, 2,000 slopes `0.01 + k * 0.0137` | 2,000 | 0 | |
| PET-like, three independent skewed frames, the same 2,000 slopes and 20,000 random slopes each | 88,000 | 0 | |
| PET-like series, 16 of 2,000 slices pooled | 1 | 0 | |

  For each refusal the probe searched up to 4,096 steps itself. None had a passing width within 64 steps, so the tree never refuses a window its own budget could reach, and every refusal has a passing width within 257 steps.
- **Constant, swept.** One counted sample per frame, slope 1, every 4,099th bit pattern plus every 3rd value with magnitude in `[2^22, 2^26)` of both signs, 11.8 million accepted. Every accepted window was Constant, centred on the value bit for bit, and drew exactly `127.5`. Stored 16777216 and 8388610 both draw exactly `127.5` at width `1.0000001`, which numpy `float32` agrees with independently. The refusals are D2.
- **Background, swept.** Each value was one stored sample left out by its own padding, every 4,099th bit pattern plus every 13th value with magnitude in `[2^16, 2^30)` of both signs, 19,111,488 values. **None was refused, every one drew exactly `0.0`**, `low` was the value and `high` was `center + width / 2` bit for bit. No case needed more than one widening step. 14,412 cases needed one, including 8388608 (centre `8388609`) and 4194303.75 (centre `4194304.5`), both reproduced in numpy.
- **Pass 2's D1 example, replayed.** Stored `6.9351015, 36.288307` is accepted at one narrowing step. Width `29.353205` (`0x41ead35d`) and centre `21.611704` (`0x41ace4c5`) give exactly `0.0` and `255.0`, and numpy agrees. The unnarrowed window draws `254.99998`, as pass 2 said.
- **D2 replayed.** Stored 16777216 and 8388610 now draw the middle grey (above). Stored 16777218, 33554432, -16777216 and 1e9 are still refused, which is D2 of this pass.
- **D3 replayed.** The "current code instead returns ... grey 0" sentence is gone. The M22 comment now says the frame would be accepted, and M22 applied this pass went red on `a_left_out_sample_is_refused_when_not_finite`. The sentence that replaced the first one is false, which is D3 of this pass.
- **The fixture comments' bit claims, executed.** Second-step pair: the lower edge is `0x41CDC576` at step 0 and `0x41CDC577` at step 1, the upper edge ties at `0x420D3D13` and then falls to `0x420D3D12` at step 2, and the centre is `30.51553`. Budget pair: the upper edge is `0x4281915E` at step 0 with `254.99956`, the lower edge is `0x427EBFCE` at step 64, the window passes at step 65 with width `0x3F8C5D5F`, and the probe confirms 65 is the first passing step. 0.001 pair: width `0x2EFFFFFF` with centre and both edges `0x3A83126F`. 3.0 pair: accepted unnarrowed at width `2^-22`. All of these hold except the two sentences in D3.
- **GLM's three residual examples** have their first passing step at 65, 65 and 128, all beyond the budget. Its report says the residual pairs lie "65 to 255 steps down", but the probe found two at 257.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | working files above | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, 96 tests over 10 binaries, `auto_window` 27 passed |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0, 1102 refusals in 78 files, all claimed |
| probe crate `f227p3` (`replay`, `dists`, `seeds`, `pet`, `constant`, `background`), path dependency on the worktree crates, target `target/f227-p3/tgt` (deleted) | real `auto_window`, `AutoWindowPool`, `AutoWindow::transform`, `VoiTransform::apply` | now | the sweeps above, D1, D2, D3 |
| numpy `float32` (`.venv`) | PS3.3 C.11.2.1.3.2's formula only | now | D1 example, 0.001 and 1.0 pairs, the constants 16777216, 8388610, 16777218 and -16777216, both background triggers. All agree with the tree, and with D2 and D3 |
| 17 source mutations in a copy of `ocelli-core` and `ocelli-pixel` under `target/f227-p3/mut`, each by exact-anchor replace (anchor count asserted 1), restored and sha1 re-checked, baseline green after | `auto_window.rs` sha1 `87de3aa6` | now | table below |
| GLM's before counts (widening refused 826,475, 836,181, 814,842 and 205) | GLM report and test comment | prior receipt, not re-run, because the widening code is gone | not checked |

| Mutation | Result |
|---|---|
| S1-M1 `NARROWING_STEPS = 1` | red: `a_pair_is_accepted_on_the_second_narrowing_step`, distribution test |
| S1-M2 `NARROWING_STEPS = 100_000` | red: `a_pair_the_budget_cannot_reach_is_refused` |
| S1-M3 narrowing by two bit steps | red: four tests |
| S1-M4 percentile check, high side only | red: `an_even_mantissa_adjacent_pair_is_refused`, budget pair |
| S1-M5 Background `high = center + width` | red: `the_window_check_draws_the_4194303_75_maximum_black` |
| S1-M6 narrowed width returned with the unnarrowed centre | red: review example, adjacent pairs |
| S1-M7 widened background centre returned with width 1 | red: both background triggers |
| O1 `CHECK_MIDDLE = 127.0` | red: five tests |
| O2 narrowed centre `(low + width) - width / 2` | red: five tests |
| O3 always narrow at least one step | red: `auto_window_pool_refusals` only, on the exact transform equality |
| O4 constant widening budget 1 | **survived**, an equivalent mutant (D2) |
| O5 background widening budget 1 | **survived**, an equivalent mutant (D2) |
| O6 budget exhaustion returns the last pair instead of refusing | red: budget pair, even-mantissa pair |
| O7 constant check removed | red: `a_constant_frame_draws_the_middle_grey_at_large_magnitudes` |
| M22 finiteness tested only for counted samples | red: `a_left_out_sample_is_refused_when_not_finite` |
| M25 `NonFiniteWindow` check removed | red: `a_non_finite_window_is_refused` |

A further mutation, the constant check evaluated at `center + width / 2 - 0.5`, was discarded. It is numerically identical wherever the constant widening runs, so it says nothing about the tests.

## Defects

### D1, the review distributions still refuse, and the test that says they do not passes only on a hunted seed

**Where**: `crates/ocelli-pixel/tests/auto_window.rs:729-824` (`no_window_is_refused_on_the_review_distributions`), `auto_window.rs:430` (`NARROWING_STEPS`), and the amendment's third bullet.
**What**: the amendment requires "a test runs the check over the review's distributions ... and asserts no refusal on them. Any refusal left is reported with its inputs." The tree refuses 78, 120 and 12 of 2,000,000 pairs on the three pair distributions (table above), and each refused window has a passing width 65 to 257 steps down. The mechanism is a pair that straddles a power of two, or sits in a coarser binade than its width. The edges' quantum is then `2^6` or more times the width's own bit step, so the edges move only once in 64 or more steps. The test asserts zero on 20,000 frames per block, but the probe ran the test's exact generators on the sixteen seeds `0xF227_x000` to `0xF227_x00F` of each block:
- uniform: 9 refusals over 16 seeds, 7 seeds zero
- integer times slope: 27 refusals, **only `0xF227_100D` zero**
- CT-like: 1 refusal, 15 seeds zero

All three blocks use the suffix `D`, and the implementer's log (lines 1368 to 1556) records choosing "a seed with 0 after-refusals in all three pair blocks". The test comment says "none of these distributions may refuse" and "at these seeds the count is zero", and it records only the widening counts. The narrowing residual and its inputs appear in GLM's chat report, not in the tree. That report's claim that the comment "records the 2,000,000-scale counts" holds only for the before counts.
**Why it is wrong**: microscope sections 2 and 3. A test whose zero holds by seed selection does not measure the distribution, and its doc sentence is false about the distribution it names. The underlying conflict is in the amendment. A 64-step budget and "no refusal" cannot both hold on these distributions. That is safe, because nothing is quietly wrong, but about one pair in 17,000 to 170,000 is refused although a valid window exists.
**Scope and regression**: every `AutoWindow` consumer, which today is the tests. A refused series window falls back to per-image windows under plan item 12. Owners: the lead first, who either raises or reshapes the budget (for example, stepping by the edges' quantum, or a budget that covers `ulp(max(|low|, |high|)) / ulp(width)`) or accepts and records the residual rate. The implementer then makes the test honest. Smallest regression: `auto_window(&[Stored(63.687313), Stored(64.78392)], slope 1)` is accepted, or the test states the measured residual rate and a non-hunted sample size, and the residual inputs are recorded in the tree.

### D2, the constant search cannot adjust any value above 2^24, so four of pass 2's six D2 triggers still have no window

**Where**: `crates/ocelli-pixel/src/auto_window.rs:393-401`, `:476-494` (`widen_center_fixed`), `:432-434` (`WIDENING_STEPS`), and the doc at `:297-304`. The test is `tests/auto_window.rs:873-876, 901-905`.
**What**: the amendment says "adjusting the window when f32 rounding at large values would put the value on an edge, and refused otherwise". The constant window is accepted when the lower edge `v - w / 2` rounds below `v`. With `w` starting at 1 and growing by `2^-23` at most 64 times, `w / 2` stays near 0.5, so the edge rounds back onto `v` whenever the spacing below `v` is 2 or more. That is every `|v| > 2^24`, and `v = -2^24`. The sweep refused every value tried in those binades (2,798,249 per dense binade, and every sampled value up to `2^127` of both signs). Stored 16777218, 33554432, -16777216 and 1e9 return `UnverifiableWindow`, although width 4.5 (or `2 * spacing + 1`) with the centre on the value draws exactly `127.5` through `VoiTransform`, and numpy agrees. The rescue works only in the binade `[2^23, 2^24]`, and only at step 1. Steps 2 to 64 never succeed for either widening search: the background sweep's largest step is 1, and mutations O4 and O5 survive because they are equivalent.
**Why it is wrong**: the amendment names large values as the case to adjust, and the implemented adjustment is a no-op there. The test pins the refusal of 16777218 as "the pinned result", so the gap becomes the specified behaviour. `WIDENING_STEPS = 64` reads as a meaningful budget but cannot change any outcome (microscope section 4).
**Scope and regression**: rare in practice, because it needs a constant-percentile frame with modality values of at least `2^24`, for example float or 32-bit parametric data. Nothing is quietly wrong, because the frame is refused and not drawn wrong. Smallest regression: four stored 16777218, 33554432, -16777216 and 1e9 each draw exactly `127.5` with the centre on the value. Otherwise the lead amends the bullet to say that constants above `2^24` are refused, and the budget comment states that one step is the most that can help.

### D3, four comment sentences are false

1. `tests/auto_window.rs:571-575`. "the centre 0.001 + half an ulp rounds UP onto the 99th percentile. A value equal to the centre maps to half the output range ... grey 128." The lower edge `c - w / 2` is also the tie, so it rounds to the even significand `0x3A831270`, which is the 99th percentile. The 99th therefore takes the `x <= c - w/2` branch and draws `0.0`, grey 0, on both the probe's `VoiTransform` and numpy. Pass 2 said "the ramp there is 127.5", which was wrong, and pass 2's D3 then asked for the correct "grey 0" sentence to be deleted. This sentence repeats pass 2's error.
2. `tests/auto_window.rs:712-714`. "the lower edge is below 1.0 until width / 2 < 2^-25, that is width < 2^-24". At width exactly `2^-24`, `1 - 2^-25` ties to the even `1.0`, and the probe finds the first passing step is exactly `2^23` = 8388608, which is width `2^-24`, not below it. The conclusion, far beyond the budget, stands.
3. `auto_window.rs:459-460`. "This is the background search: widening moves the lower edge down". In real arithmetic the lower edge is fixed at the value, because the centre is `low + w / 2`. In f32 the rescue moves it **up**, from 8388607.5 to 8388608 and from 4194303.5 to 4194304, as the fixtures' own comments at `tests:496-500, 532-534` say.
4. `auto_window.rs:303-304`. The background window is "widened the same way around that value". The preceding sentence describes the constant search, which keeps the centre on the value. The background search keeps the centre at `value + width / 2`, which `widen_until` is and `widen_center_fixed` is not.

**Why it is wrong**: microscope section 3. These comments are the hand computation that HLD 27.2 R2 relies on. **Smallest fix**: delete or correct each sentence. Each number above was executed.

## Smells

None. The equivalent widening budget is carried in D2, and the hunted seed is carried in D1.

## Nitpicks

- **N1** `tests/auto_window.rs:805-806, 811` add three `as` casts (`r as i32`, `c as i32`, `(k * k) as usize`). Each is lossless (`r, c < 128`, `k * k <= 65025`), but the file uses `try_from` everywhere else, and pass 2's "no `as` cast in either file" is no longer true.
- **N2** `narrow_until`'s `width == 0.0` guard (`auto_window.rs:512`) is redundant. `display_bits` rejects a zero width, and the next iteration's `bits == 0` returns `None`. The subnormal pairs (bits 1 and 2, 1 and 3) are accepted either way.
- **N3** Unconfirmed concern. `finish`'s doc and the amendment say the check is the same on every tier because it lands on the edge branches. That holds when `w / 2.0` is exact on the GPU. WGSL's accuracy table allows `x / y` 2.5 ULP, so this rests on drivers emitting an exact halving, which pass 2 N1 took as given. No GPU was exercised here.
- **N4** Pass 1 N1 to N5 and pass 2 N2 and N3 are carried unchanged, including the 4 MiB reserve for every one-frame pool.

## Verified clean

- Pass 2 S1 is closed. All seven mutations named there are red, the background width and `high` are asserted by bits, a percentile fixture needs step 2, a fixture fails on the low side only, and a refusal is pinned at step 65.
- No returned window fails its own check: 6.6 million percentile frames, 11.8 million constants and 19.1 million backgrounds.
- `UnverifiableWindow`'s doc, `finish`'s error list, `AutoWindow::low` and `high` for each basis, and `narrow_until`'s "the upper edge falls as the width falls" match the code.
- No consumer outside `ocelli-pixel` names `auto_window`, `AutoWindow` or `UnverifiableWindow`. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched.
- No `unsafe`, and no `as` cast in `src/auto_window.rs`.
- The tree is unchanged by this review. `git write-tree` is still `e50fe64f`, the working-file sha1s are as listed, `git status` shows the same set, and the scratch directory `target/f227-p3` was deleted.

## Limitations and handoff

- Not exercised: the viewer half, any GPU tier, the oracle, and real operator PET bytes. The PET results use synthetic frames.
- The before counts under widening are GLM's receipts and were not re-run.
- Next owner: the lead decides D1's budget against its target and D2's constant rule. The implementer remediates D1's test and record, D2 as decided, and D3. A fresh independent pass 4 then replays these sweeps. This is a bounded review of the library half, not acceptance of F-227.
