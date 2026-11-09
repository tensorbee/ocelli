# F-227 review, pass 7

**Reviewed**: library half only. `git diff df768df HEAD` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `8865b331c1ee3964ed51b881724dd774366c1b48` (the lead's pass 6 corrections `b41e165` and the round `8865b33`), `git write-tree` `dab326eb14bc7d72fde2c04c83be3e5f3d12a5eb`, no unstaged tracked changes. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, `.claude/plans/F-227-design.md` sha1 `58d31a98`. The plan's amendments through "Lead amendments after review pass 6" are authoritative. Untracked `tools/oracle/out` and the pass 1 to 6 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the round is by a Claude implementer and the amendment and corrections are the sprint lead's, both attributed by the caller's brief. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 6 were written by the same model, so this pass is independent of the remediation but not of the findings it answers. The implementer's three mutations are not recorded anywhere in the tree, so the three replayed below (M1 to M3) are the three this round most obviously needs, named before running, and M4 to M9 are mine.
**Result**: 2 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- **Every boundary the source records, re-measured by exhaustive scan.** My method is not the implementer's bisection. Every f32 from 2^33 (`0x50000000`) to 2^36 (`0x51800000`), both signs, was run as a four-sample constant frame through `auto_window`. The only transition is at `0x50FFFFF1`: accepted up to `0x50FFFFF0` at width `0x4B800000` = 2^24, refused from `0x50FFFFF1`, and the negatives match bit for bit. Every f32 `low` from 2^33 to 2^35 was run as a close pair at d = 1 and d = 2, both positive and mirrored `(-high, -low)`. **d = 1**: first refusal at `0x50800000`. From `0x507FFFFD` the accepted lows are `507FFFFD`, `507FFFFE`, `507FFFFF` and then exactly `50800001`, `03`, `05`, `07`, `09`, `0B`, `0D`, the seven recorded. **d = 2**: first refusal at `0x507FFFFE`, and `0x507FFFFD` is the last accepted. 0 mismatches against the recorded rule over both signs, 0 contract violations. Per k above 2^34 the probe gives centre == 99th for odd k, a low-side distance of 18432 + 1024k (19456 at k = 1, 31744 at k = 13, 33792 at k = 15) and 32768 on the high side, and for even k 34816 on the high side. That matches the source's derivation at `auto_window.rs:339-345`. **Every number recorded at `auto_window.rs:327-348` is confirmed.**
- **Every binade, my own sample.** For each exponent field from 0 (subnormals) to `0xBD` (2^62): the 512 lowest and 512 highest significands plus 1024 random ones, as a constant and as d = 1 and d = 2 pairs, both signs. That is 2,334,720 windows with 0 mismatches against the recorded boundaries and 0 contract violations. "No exception beyond those named here" holds on this sample. Pairs straddling zero: `(-tiny, tiny)` is Constant at centre 0, `(-0, 0)` is Background, and each accepted one passes the contract.
- **Width 1 below 1024.** 2,000,000 random constants and d = 1 or 2 pairs whose outward points are all below 1024 in magnitude: every one is accepted at width exactly 1, as `auto_window.rs:315-318` says.
- **Tier A, emulated with `f32::mul_add(stored, slope, intercept)`.**
  - *F1*, constants from 2^15 to 2^35: 100,000 per binade, random stored 1 to 65535, intercept up to half the target magnitude either sign, 2,000,000 frames. 0 refused. The fused value is at most 2 units from the CPU value, and **0 fused values draw outside (127, 128)**.
  - *F1b*, constants at stored 1 to 65535, slope 0.5 to 3.5 and intercept -1024 to 1025: 1,000,000 frames. 0 fused values within 16 units draw outside (127, 128). The 268 values more than 16 units off (up to 1280 units, by cancellation) all still draw grey 127 or 128.
  - *F2*, close pairs from 2^15 to 2^35, made with a slope of up to 2.5 units: 1,821,764 accepted with 0 fused values outside (127, 128). The 74,979 refused all have |value| at least 1.718e10, above 2^34, as recorded.
  - *F3*, a cancellation family: 252,452 fused values more than 16 units off leave grey 127 or 128 (D2).
  - *The 128.02103 point.* The fused value moved 16 more units lies 17 units from the centre. In F1 such points leave (127, 128) 2,266,940 times and leave grey 127 or 128 0 times, because 32 units draw less than one grey. The fixture labels that point "grey 128" only, so no comment claims half a grey for it. The plan's guarantee concerns the value a tier draws, and that holds within 16 units. **Within half a grey holds for every fused value within 16 units. Grey 127 or 128 holds for every fused value measured within 16 units, and for some beyond.** The only false claim is the unconditional sentence in D2.
- **Zero refusals on eight seeds of my own.** The seeds were `0x428A2F98D728AE22`, `0x7137449123EF65CD`, `0xB5C0FBCFEC4D3B2F`, `0xE9B5DBA58189DBBC`, `0x3956C25BF348B538`, `0x59F111F1B605D019`, `0x923F82A4AF194F9B` and `0xAB1C5ED5DA6D8118`, none among the test's twelve or pass 6's eight. Each ran 100,000 pairs in each of the three pair distributions with the test's generators, plus the 2,000-slice PET series and its pooled window: 2,416,008 windows, **0 refused, 0 contract violations**, and 21 Constant windows among them.
- **Rewritten constant fixtures, re-derived by hand and executed.**
  - 32768: the spacing is 2^-8 above and 2^-9 below, so D = 0.0625 and 0.03125 and w > 31.875. Width 17 gives 255 * 0.0625 / 17 = 0.9375 and fails. Width 33 gives 127.98295 and 127.25852 (executed 127.982956 and 127.258522).
  - 65536 at width 65: 127.99038 and 127.25481. 4194304 at width 4097, with w > 4080 and 2049 too narrow: 127.99793 and 127.25104.
  - 8388610: down two units reach 2^23 and 14 units of 0.5 follow, D = 9. w > 8160, so width 4097 fails and 8193 passes. It draws 127.99799 and 127.21988.
  - 16777216, 16777218 and -16777216 at width 16385, and 33554432 at width 32769: 127.99802 and 127.25099, low 127.23543, 127.00198 and 127.74901, and 127.99803 and 127.25098.
  - 1e9: D = 1024, w > 522240, width 262145 fails and 524289 passes, 127.00195 and 127.99805. Every value matched to the printed digits.
  - `0x50FFFFF0`: v + 2^23 = 2^35 + 8355840 = 2^35 + 2040 * 4096 is exact. The displays are 65025/512 and 65535/512 exactly. `0x50FFFFF1`'s 16 units up span 34816, and at width 2^24 that draws 128.02917, executed.
  - Close-pair fixtures: 2088705, 2084625, 1044225/8192, 2084115 and 2089215 over 16384 recomputed as 2088960 minus 255, 4335, (255 over 8192), 4845 and plus 255. All are exact, and the probe gives the same bits.
  - The tier case: product 113874.86171722412 rounds to 113874.859375. The sum 114684.19763183594 rounds to `0x47DFFE19`. The fused exact 114684.19997406 rounds to `0x47DFFE1A`. One unit draws 0.030649, giving 127.53065, 127.04026 and 128.02103, executed. The pass 6 D3 figure 0.513139 is confirmed: `VoiTransform::apply` gives 126.98686.
- **Executed comments and plan sentences this round changed.**
  - `auto_window.rs:311-325`: the four points, "w > 510 * D" and the width-one-below-1024 claim are all checked above.
  - `:327-330`: 1 + 2^24 ties to 2^24, 255 * 32768 / 2^24 = 0.498 and 255 * 34816 / 2^24 = 0.529.
  - The enum doc at `:107-109` holds.
  - The test module header at `:12-13` holds.
  - The plan's pass 6 amendment holds as written.
  - The rewritten pass 4 ruling, "the check confirms each percentile draws the output minimum or maximum", is what `maps_extremes` checks.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | tree `dab326eb` | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, `auto_window` 32 passed in 5.8 s, every binary ok |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0, "0 watched by nothing" |
| probe crate `target/f227-p7/probe`, path dependency on the worktree's `ocelli-pixel` and `ocelli-core`, release | the real `auto_window`, `AutoWindowPool`, `series_sample`, `AutoWindow::transform`, `ModalityTransform::apply` and `VoiTransform` | now | modes `scan` (32 s, exhaustive), `sweep`, `w1`, `fused`, `dist` and `fixtures`, results above |
| Python `fractions` and `struct` f32 rounding | D2's example | now | CPU 1.078125, fused 1.0834198, display 128.85017 |
| 9 source mutations in a scratch workspace `target/f227-p7/mut` of copies of `ocelli-core` and `ocelli-pixel`, exact-anchor replacement with the anchor count asserted 1 and the new text asserted absent, `cargo test -p ocelli-pixel --test auto_window`, original restored and compared with `cmp` against the tree | `auto_window.rs` sha1 `ddc98990` | now | table below |

| Mutation | Result |
|---|---|
| M1 the outward check for a close pair only, the pre-round code | red: `a_constant_beyond_the_search_reach_is_refused`, `a_constant_frame_draws_the_middle_grey_at_large_magnitudes`, `a_constant_from_2_15_up_draws_the_middle_grey_on_tier_a` |
| M2 `outward_pair` moves 15 units | red, 4 tests including both reach tests |
| M3 `CONSTANT_TOLERANCE = 0.53` | red, 4 tests including both reach tests |
| M4 (mine) `SEARCH_ATTEMPTS = 47`, widest width 1 + 2^23 | red, both reach tests |
| M5 (mine) `CONSTANT_TOLERANCE = 0.49` | red, 5 tests |
| M6 (mine) a constant's low-side outward point dropped | red, `a_constant_beyond_the_search_reach_is_refused`, `a_constant_frame_draws_the_middle_grey_at_large_magnitudes` (the -2^24 row) |
| M7 (mine) `outward_pair` moves 17 units | red, 5 tests |
| M8 (mine) a d = 1 pair always centred on the 1st | red, `a_close_pair_beyond_the_search_reach_is_refused`, `the_pass_4_tier_a_pair_draws_the_middle_grey` |
| M9 (mine) the search returns its last tried window instead of `None`, the pass 4 amendment's required red | red, both reach tests |
| informational: `CONSTANT_TOLERANCE` 0.51 and 0.4981 | survived, as pass 6 N2 (carried) |

## Defects

### D1, the pass 3 amendment still says constants between 2^47 and 2^48 are accepted

**Where**: `.claude/plans/F-227-design.md:453`: "Every constant of magnitude 2^48 or more is refused with `UnverifiableWindow`, which is safe, and some between 2^47 and 2^48 are accepted at the widest attempt."
**What**: on HEAD every constant from `0x50FFFFF1`, about 3.4e10, up is refused. `0x57000001` = 2^47 + 2^24 is the very case that clause described, and it is now refused. `a_constant_beyond_the_search_reach_is_refused` asserts that, and its doc says "Before pass 6 the first was accepted at width 2^24". The probe agrees: `konst(0x57000001)` returns `UnverifiableWindow`, and so does every sampled value of exponent field `0xA2` to `0xBD`. The pass 6 amendment withdrew the pass 5 limit sentence by name and moved the measured boundaries into the source. It did not withdraw or supersede this sentence, which the plan still presents as the rule.
**Why it is wrong**: microscope section 3. It is a factual claim about what the window accepts, and on HEAD it is false. The plan is where the next reader looks for the reach.
**Evidence**: probe mode `fixtures` (`const 0x57000001: Some(UnverifiableWindow)`) and mode `sweep`, with 0 mismatches against "refused iff |v| bits >= 0x50FFFFF1".
**Scope and regression**: prose only, introduced by the round's behaviour change. The smallest fix deletes the clause after "which is safe", or marks the bullet's reach superseded by the pass 6 amendment the way earlier amendments mark what they supersede. Nothing needs a new test, because the reach tests already pin the behaviour.

### D2, the pass 5 ruling guarantees half a grey on every tier, unconditionally

**Where**: `.claude/plans/F-227-design.md:481`: "no window can make one grey on every tier. What is guaranteed for a constant or close-pair window is a display value within half a grey of 127.5."
**What**: the check guarantees that this crate's evaluation draws the value, and every value within 16 units of it, inside (127, 128). It therefore covers a tier whose stage 1 lands within 16 units, and that is measured: 0 of about 15.6 million fused values within 16 units (families F1, F1b and F2) leave (127, 128). A tier rescale that differs by more than 16 units is not covered, and with cancellation it does.
- Executed: stored 52636, four times, slope 2.490162 (f32 2.490161895751953), intercept -131071.08 (f32 -131071.078125).
- The CPU stage rounds the product 131072.16154 to 131072.15625, and the sum is 1.078125. The fused `mul_add` rounds once, to 1.0834198, which is 44,416 units away.
- The window is Constant, centre 1.078125, width 1. The fused value draws 128.85017, **grey 129**.
- In family F3 (stored 1 to 65535, slope 0.5 to 3.5, intercept within 2 of minus the product), 252,452 fused values leave grey 127 or 128. The smallest product among them is about 131072.
The pass 4 amendment for percentiles says "within the margin", and the pass 6 amendment says "a tier whose rescale rounds a few units differently". This sentence carries neither condition, so it reads as a guarantee on every tier, which the sentence before it is about.
**Why it is wrong**: microscope section 3, and CLAUDE.md's tier rule. A guarantee stated without its condition becomes the contract the viewer half and F-041's owner read.
**Evidence**: probe mode `fused`, family F3, and the example recomputed in Python with `fractions` and f32 rounding through `struct`.
**Scope and regression**: carried prose from the pass 5 rulings. Pass 6, by this same model, judged the ruling sound without testing a tier difference beyond the margin, so this is a new finding on unchanged text rather than a regression of this round. No consumer draws an automatic window on tier A yet. The trigger needs a product above about 2^17 cancelled to near zero, which ordinary CT (slope 1, intercept -1024) and PET (intercept 0) do not produce. The smallest fix adds the condition: "for a tier whose stage 1 lands within 16 f32 units of this crate's". No test is needed, because `a_constant_from_2_15_up_draws_the_middle_grey_on_tier_a` already pins the covered case.

## Smells

None.

## Nitpicks

- **N1** `auto_window.rs:540-542`, `CONSTANT_TOLERANCE`, "How far from [`CHECK_MIDDLE`] a close pair's values may draw", and `:545-547`, `outward_pair`, "The 1st and 99th percentiles of a close pair". Since this round both apply to equal percentiles too. Neither sentence is false, but both name only the case the code no longer limits itself to.
- **N2** `tests/auto_window.rs:1118-1120` and `:1190-1191`. "The next widest, 1 + 2^23 = 8388609, allows D < 16448": 8388609 / 510 = 16448.25, so D = 16448 itself passes. The figure 32896.5 is 32896.502. Both are immaterial, because D is a multiple of 1024 or 2048 there.
- **N3** `auto_window.rs:319-320` ends a doc line at "point. The Background window is" after a full 100-column line. That is a ragged wrap that rustfmt will not repair, on top of pass 6 N3's reflow.
- **N4** `a_constant_from_2_15_up_draws_the_middle_grey_on_tier_a` asserts the fused value moved 16 more units, 17 units from the centre, at 128.02103 and grey 128. No tier draws that point for this frame and no contract covers it, so the assertion pins an incidental value. It is correct and labelled grey only. A margin change would turn it red for a reason that is not a defect.
- **Carried**: pass 6 N1 (the tie sentence below 2^-125), N2 (the exact 0.5 is unpinned, and 0.51 and 0.4981 survive again) and N3 (100-column reflow). Pass 4 N1, pass 1 N1 to N5 and pass 2 N2 and N3 are unchanged.

## Verified clean

- **Pass 6 closures.**
  - D1 is closed. A constant takes the outward check. 32768, 65536 and 2^22 are pinned, and so is the review's tier case, now width 65 and fused 127.53065. M1 shows the pre-round code goes red.
  - D2 is closed by deleting both clauses. The replacement sentence is what `maps_extremes` checks.
  - D3 is closed. The figure is 0.513139.
  - S1 is closed. The close-pair reach is stated at `auto_window.rs:339-348`, confirmed exhaustively, and pinned by `a_close_pair_beyond_the_search_reach_is_refused` on both sides of each boundary and both signs.
- The constant check is the close-pair check, with `outward_pair(low, low)`. The four points are the value moved down, the value twice and the value moved up, as the doc says.
- The LUT chain exists once: `near_middle` and `display_bits` call `VoiTransform::new` and `apply`, and no window or ramp arithmetic is written in `auto_window.rs`. Neither file contains an `as` cast, `unsafe` or `#[allow]` (`git grep`). The probe's own casts are in the scratch crate only.
- Every mutation M1 to M9 is red, including the search returning its last window, which the pass 4 amendment requires to be red.
- No consumer outside `ocelli-pixel` names `auto_window` or `AutoWindow`, and `lib.rs` only re-exports. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched by the diff and describe nothing it changed.

## Limitations and handoff

- Not exercised: a real GPU. The fused stage 1 was emulated with `f32::mul_add`, which pass 4 showed bit-identical to Metal for its inputs. A tier's own VOI evaluation was not emulated, and the CPU `VoiTransform` drew every fused value. Tier B, the viewer half, the oracle and real CT or PET bytes were not exercised.
- The boundary scans are exhaustive over [2^33, 2^36) for constants and [2^33, 2^35) for pair lows. Elsewhere they are a sample of 2,048 significands per binade, and that sample is not the implementer's 16384-per-end method.
- Next owners: the lead deletes or supersedes the pass 3 reach clause (D1) and adds the 16-unit condition to the pass 5 guarantee (D2). Both are prose only. A fresh independent pass 8 then reviews the change. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` and the three file sha1s are re-checked at the end, and the scratch directories `target/f227-p7` and `target/mut-target` were deleted.
