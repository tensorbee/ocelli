# F-227 review, pass 5

**Reviewed**: library half only. `git diff 2c39345 HEAD` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `369835d7da652beeff4d08f9ffa2da9f239d5546`, `git write-tree` `aa3c56fa7afdab0ca0a8e09ae0578d37ff21f90b`, no unstaged tracked changes. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `fde85137`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `b2b2f625`, `.claude/plans/F-227-design.md` sha1 `492a155c`. The plan's amendments through "Lead rulings on the pass 4 remediation" are treated as authoritative. Untracked `tools/oracle/out` and the pass 1 to 4 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the round is by a Claude implementer, attributed by the caller's brief. The amendments and rulings are the sprint lead's. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or rulings. Passes 1 to 4 were written by the same model, so this pass is independent of the remediation but not of the findings it answers. The implementer's surviving equivalent mutant claim is not recorded anywhere in the tree, so it was judged from the mutants themselves (see N3).
**Result**: 2 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Zero refusals on eight seeds of my own.** Seeds `0xA5A50001`, `0x1001`, `0x77777777`, `0xFEEDFACE`, `0x31415926`, `0x27182818`, `0xBEEF0001`, `0x0F0F0F0F0F`, none of them the test's `1` to `8`. 100,000 pairs per seed per pair distribution with the test's generators (uniform fractional, integer times slope, CT-like at intercept -1024), plus the 2,000-slice PET series and its pooled window per seed: 2,416,008 windows, **0 refused**. For every accepted Percentile window, `low`, `low_moved`, `high_moved` and `high`, with the moved pair recomputed in the probe rather than read from the crate, drew exactly `0.0, 0.0, 255.0, 255.0` through `AutoWindow::transform`. `bad_exact` was 0 in every distribution.
- **The tier A fused rescale, emulated with `f32::mul_add(stored, slope, intercept)`.** On the four distributions: **0 grey flips**. Uniform, integer times slope and PET are exact products or intercept 0, so fused equals unfused. CT-like differs by up to 7,296 units near zero HU, where `stored * slope` cancels the intercept, and 14 display values are no longer exactly 0 or 255, but every one still rounds to grey 0 or 255.
- **Pass 4's example still flips at HEAD.** Stored 2371 and 2372, slope 9.00603e-7, intercept 1854.0933: CPU modality `0x44E7C30D` and `0x44E7C30E`, one unit apart, so `margin_pair` moves nothing. The window is centre `0x44E7C30D`, width 1.2207e-4, CPU draws 0 and 255, and the fused stage 1 gives `0x44E7C30D` for both, **drawing 0 and 0, the 99th percentile black**. The second example, stored 1988 and 1989 at slope 7.6754475e-7 and intercept 1736.4397, still draws **255 and 255**. See D1.
- **The midpoint cap, judged.** An adversarial family with small slopes and large intercepts, stored pairs 1 to 80 apart, 3.2 million draws: flips at d = 1 (32 of 621,789 windows) and d = 2 (29 of 233,876), and **none from d = 3 up**, where the cap gives at least one unit and the fused error in this family is at most one unit. So the cap's margin is exactly enough where it is non-zero in this family, and the flip survives where it is zero, which is d = 1 and d = 2. A second adversarial family, where the product cancels the intercept so the percentiles sit near zero, flips at any d: 15,879 of 800,000 windows, for example stored 56893 at slope 4.8812434e-2 and intercept -2777.1045, CPU -1.8798828e-2 and fused -1.868023e-2, which draws 0.619, grey 1, where the CPU draws 0. There the fused error is up to 956 million units, so no unit margin covers it. See D1.
- **How far the window moves from the true percentile span.** PET: at most 2.0e-6 of the span in width and 4.4e-4 grey anywhere in the window. Ordinary CT windows, with spans of hundreds or thousands of HU at magnitudes below 4096, move by at most 32 units of about 2.4e-4 HU, which is invisible. But the move is fixed in units, not relative to the span, so a narrow span moves a lot: CT-like pairs one stored value apart reach 12.5 to 24 percent of the span. See D2.
- **Constants from 2^47 to 2^48.** Every 97th pattern in `[2^47, 2^48)`: 43,241 accepted of 86,481, which is all 43,240 odd significands plus `2^47` itself (width 8388609, its spacing below being 2^23). `0x57000003` and `0x577FFFFF` are accepted at width `2^24`, the widest attempt. The negative half mirrors it, 43,240 accepted. Every 4099th pattern from `2^48` to the largest finite, both signs: **0 accepted**. Every 997th pattern in `[2^24, 2^47)`: 193,519 of 193,519 accepted. The plan's new sentence, "Every constant of magnitude 2^48 or more is refused ... and some between 2^47 and 2^48 are accepted at the widest attempt", holds as written. `auto_window.rs:319-324` holds too, apart from N2.
- **New fixtures, re-derived by hand.**
  - AC1: 0.5 moves 16 units of 2^-24 to `0x3F000010`, and 49.5 moves 16 units of 2^-18 to `0x4245FFF0`. The width `49 - 2^-14 - 2^-20` is a quarter spacing above `49 - 2^-14` and rounds to `0x4243FFF0`. The centre `25 - 2^-15 + 2^-20` is a half spacing of 2^-19 above `0x41C7FFF0`, a tie that goes to the even `0x41C7FFF0`. The edges are exactly 0.5 and `49.5 - 2^-14`. The moved low's `x - c = -24.5 + 33 * 2^-20` ties to the even `-(24.5 - 2^-15)`, `0x41C3FFF0`, whose quotient by the width is exactly -0.5, so it draws 0.0. The moved high is exactly the upper edge and draws 255.0. "Within 7e-5" holds at 6.1e-5 and 3.1e-5. Every sentence holds.
  - The midpoint-cap fixture: `0x41200000` and `0x4120000A` move `(10 - 1) / 2 = 4` units to `0x41200004` and `0x41200006`. The width is 2 units of 2^-20 = 2^-19 = `0x36000000`, the centre is `0x41200005`, and `0x41200006` is on the ramp at `(2^-20 / 2^-19 + 0.5) * 255 = 255`. The pair 32 units apart moves 15 units. The uncapped crossing claim is confirmed by mutation M9 below. The probe checked `margin_pair` against `min(16, (d - 1) / 2)` for every d from 1 to 99 with no mismatch.
  - The other comments were executed in a probe. The `-2000` fixture's bits pass with the stated derivation. The padding fixture's lower edge is `0x3F800010 = 1 + 2^-19`, as its comment says. The `3, 9` pair is centre 5.9999943 and width 5.999981, so the fixture's "within 2e-5" and "within 6e-6" hold. The test module's "within 2.5e-4" claim for centres and widths was executed by setting `assert_close`'s tolerance to 2.5e-4: no centre, width, low or high assertion failed, and only two 3-decimal display-value literals did.
- **The moved-pair reading.** The ruling's "about 98 percent of windows refused" under the literal reading was executed as mutation M10: on seed 1 it refused 24,595, 24,559 and 24,220 of 25,000 in the three pair distributions (98.4, 98.2 and 96.9 percent) and none of the PET series. The reading is the right one, and the claim holds for the pair distributions.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | tree `aa3c56fa` | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, 98 tests over 10 binaries, `auto_window` 29 passed |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0, 1102 refusals in 78 files, all claimed |
| probe crate `target/f227-p5/probe`, path dependency on the worktree's `ocelli-pixel` and `ocelli-core` | real `auto_window`, `AutoWindowPool`, `AutoWindow::transform`, `VoiTransform::apply`, `ModalityTransform::apply` | now | the distribution, fused, constant, fixture and shift results above |
| probe `p2` against a mutated scratch copy, 2.97 million accepted windows over cross-zero, mixed-magnitude, near-adjacent bit-pattern and uniform pairs | scratch copy of `auto_window.rs` under M4, M7 and unmutated | now | 0 windows whose `low` or `high` is not exact under any of the three, and 54.6 percent with the moved 99th on the upper edge |
| 16 source mutations in a scratch copy of `ocelli-core` and `ocelli-pixel`, applied by exact-anchor replace with the count asserted 1, original restored and compared after the run | `auto_window.rs` sha1 `fde85137` | now | table below |

| Mutation | Result |
|---|---|
| M1 `PERCENTILE_MARGIN = 0` | red, 7 tests |
| M2 `PERCENTILE_MARGIN = 15` | red, 6 tests |
| M3 `PERCENTILE_MARGIN = 17` | red, 5 tests |
| M4 the check at `low` removed | **survived** (N3) |
| M5 the check at `low_moved` removed | red, `a_pair_straddling_64_is_accepted_on_the_eighth_attempt` |
| M6 the check at `high_moved` removed | red, 4 tests |
| M7 the check at `high` removed | **survived** (N3) |
| M8 cap `up > down` instead of `up >= down` | red, `the_margin_moves_are_capped_at_the_midpoint` |
| M9 no cap | red, 3 tests |
| M10 the literal reading, first width the span and centre `low + width / 2` | red, 10 tests, about 98 percent pair refusals |
| M11 moved first width, unmoved centre rule | red, 10 tests |
| M12 unmoved first width, moved centre rule | red, 6 tests, 3,589 uniform refusals on seed 1 |
| M13 only the low side moves | red, 9 tests |
| M14 an exhausted search returns its last window | red, `a_constant_beyond_the_search_reach_is_refused` (pass 4 D1 closed) |
| M15 `SEARCH_ATTEMPTS = 47` | red, the same test |
| M16 `SEARCH_ATTEMPTS = 49` | red, the same test |

## Defects

### D1, the margin leaves pass 4's own tier A flip in place, and the plan still says the grey holds on every tier

**Where**: `.claude/plans/F-227-design.md:459-460`, the pass 4 amendment's sentences "A tier whose rescale rounds differently within 16 units therefore still draws black and white" and "What holds on every tier is the grey within the 16 unit margin". The ruling at `:468` then sets the margin to zero for adjacent pairs and does not say what that does to a tier. The code is `auto_window.rs:470-483` (`margin_pair`) against `crates/ocelli-render/shaders/voi.wgsl`'s contracted stage 1.
**What**: the pass 4 amendment was written for the pair at stored 2371 and 2372, slope 9.00603e-7, intercept 1854.0933. That pair is one unit apart, so under the cap it moves zero units, and at HEAD it is accepted with the same one-unit window as before. Under the fused stage 1 both stored values give `0x44E7C30D`, and the 99th percentile draws grey 0. The pass 4 counter-example, 1988 and 1989, still draws grey 255 at both. In an adversarial small-slope family of 3.2 million draws, 61 windows flip, all at d = 1 or 2, and none from d = 3 up. A cancellation family flips at any d (15,879 of 800,000), because the fused error there is up to 956 million units and outside any unit margin. The code's own doc (`:295-299`) says adjacent percentiles have no margin, and that sentence is true. The plan's two sentences are false for every pair under three units apart, the amendment's own trigger among them.
**Why it is wrong**: microscope sections 3 and 5, and CLAUDE.md's tier rule that a feature never quietly produces a different result on a tier. When the tier maps the two percentiles to one f32, no monotone window can draw them differently. So no search can repair d = 1, and a window here cannot make the claim true. The authoritative text claims a tier property the accepted windows do not have.
**Evidence**: probe `target/f227-p5/probe` (`main` and `bin/hist`), above. The GPU was not re-run in this pass, and the emulation is the one pass 4 showed bit-identical to Metal on these inputs.
**Scope and regression**: no consumer draws an automatic window on tier A yet. The frames that trigger it have a 98 percent percentile span of one or two f32 units, which takes a large intercept with a tiny slope, or a frame that is nearly constant after rescale. Owner: the lead. Either the sentences name the limit, that pairs fewer than three units apart, and any rescale that cancels the intercept, still draw one grey on a tier whose stage 1 is contracted, or the story refuses such windows and the viewer half handles the refusal. The stage 1 contraction stays with the F-041 owner as pass 4 recorded. The smallest regression in this story is a fixture pinning that the 2371 and 2372 pair is accepted with zero margin, plus a plan sentence that says so.

### D2, the ruling's two bounds on the moved window are false

**Where**: `.claude/plans/F-227-design.md:467`, "so each percentile lies at least 16 units outside its edge. The window changes by at most 2.5e-4 of its span."
**What**:
- **"At most 2.5e-4 of its span."** The move is 32 units of the values, whatever the span. Stored 40000 and 40001 at slope 1 and intercept -1024 are integer HU 38976 and 38977, and the accepted width is 0.875, **12.5 percent** narrower than the span. A value a quarter of the way up draws 54.6 where the span window draws 63.75. At 10.0 and 10.0 plus 40 units the width is 8 units, **80 percent** narrower. Over my seeds the CT-like distribution, which has no pair under 33 units, reached 23.9 percent and the integer-times-slope distribution 20.7 percent (stored 30000 and 30001 at slope 0.001: 6.1 percent). The 2.5e-4 figure is the test module's absolute bound for AC fixtures below 256 in magnitude, which is true there and was executed. As a fraction of the span it does not hold.
- **"Each percentile lies at least 16 units outside its edge."** The edges are rounded, so the lower edge can land one unit below the moved 1st percentile. In 71,704 of 1,000,000 uniform windows with the full 16-unit move it does, which leaves the 1st percentile 15 units outside its edge. For example, low `0x41406791`, moved low `0x414067A1`, lower edge `0x414067A0`. The grey is still safe there, because the check demands an exact 0.0 at the moved value on the ramp. The source doc says exactly that at `:305-308`, so the source and the plan disagree. Under the cap the figure is also smaller for d below 33, as the next bullet says.
**Why it is wrong**: microscope section 3. These are authoritative sentences a later reader will treat as the window's accuracy contract. The first understates the change by up to three orders of magnitude, for exactly the narrow windows where the change is visible.
**Evidence**: `target/f227-p5/probe` `bin/ex` and `bin/edge`, above.
**Scope and regression**: no ordinary CT or PET window is visibly affected. The PET windows move by at most 2.0e-6 of the span and 4.4e-4 grey, and a CT window spanning hundreds of HU moves by about 1e-5 grey. The affected frames are those whose 1st and 99th percentiles are a few stored values apart. Owner: the lead. Either delete the two clauses, which is the smallest fix, or state the bound in units, for example "each edge moves by at most 16 f32 units of the percentile it bounds, plus the edge's rounding". No code change is required.

## Smells

None.

## Nitpicks

- **N1** `auto_window.rs:507-508`, `Search::Narrow`: "the Percentile window, whose upper edge is `low + width` in real arithmetic". In this file `low` names the 1st percentile, and the upper edge is now `low_moved + width`. The sentence's conclusion, that the edge falls as the width falls, still holds.
- **N2** `auto_window.rs:320-322`: "A Constant whose spacing below it is wider than that, every magnitude from 2^48 up". At exactly 2^48 the spacing below is 2^24, not wider, so 2^48 falls under the next sentence's tie rule. It is refused there because its significand is even, which the sweep confirms. "Above 2^48" is the accurate phrase.
- **N3** The equivalent mutants. M4 and M7, which remove the check at the unmoved `low` or `high`, survive the suite. Over 2.97 million accepted windows, chosen to stress the case (cross-zero, values that differ widely in magnitude, near-adjacent bit patterns), no window under either mutant has an inexact `low` or `high`. The reason is monotonicity: once 16 units falls below the resolution of `x - centre`, the moved and unmoved values evaluate identically, and above that resolution the moved value is strictly on the ramp side. I accept the claim as equivalent in practice. It is not proved, and it is not recorded in the tree. A sentence at `:403-408` saying the two outer checks are kept as a defence would let the next reviewer tell an intended survivor from a gap.
- **Carried**: pass 4 N1, the pass 2 amendment bullet still ending "one f32 step at a time ... at most 64 steps", explicitly superseded. Pass 1 N1 to N5 and pass 2 N2 and N3 are unchanged. Pass 4 N2 is closed, because both background comments now name the first attempt's unit.

## Verified clean

- Pass 4 D1 is closed. `a_constant_beyond_the_search_reach_is_refused` asserts `UnverifiableWindow` for 1e30, -1e30, -2^47 and `0x57800001`, and it goes red under M14, M15 and M16. Pass 4 D2 is closed. The plan's constant sentence was executed over sweeps of `[2^24, 2^47)`, `[2^47, 2^48)` of both signs and every magnitude from `2^48` up. Pass 4 D3's sentence is superseded, and its replacement is D1 above.
- `margin_pair` matches `min(16, (d - 1) / 2)` for d from 1 to 99. Its moved pair is strictly ordered, so `high_moved - low_moved` is positive and no wider than the finite span, as the comment at `:400-401` says.
- `finish`'s doc at `:287-324` matches the code, apart from N2. "A moved 99th on the upper edge itself, the usual case" holds at 54.6 percent. "A moved 1st above a lower edge that rounded below it, draws through the ramp" occurs in 7.2 percent of uniform windows and is exact in every one.
- The LUT chain exists once. `display_bits` builds `VoiTransform::new(.., LinearExact, 0, 255)` and calls `apply`. `margin_pair` is `next_up` and `next_down` only, with no window or ramp formula. Neither file contains an `as` cast, `unsafe` or `#[allow]`.
- Every one of the 16 mutations except M4 and M7 is red, including each margin constant either side of 16, each of the two moved checks, both cap mutations and every half of the moved-pair reading.
- No consumer outside `ocelli-pixel` names `auto_window` or `AutoWindow`. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched by the diff and describe nothing it changed.

## Limitations and handoff

- Not exercised: a real GPU in this pass. The fused stage 1 was emulated with `f32::mul_add`, which pass 4 showed bit-identical to Metal on M4 Max for these inputs. Tier B, the viewer half, the oracle and real PET or CT bytes were not exercised either.
- The implementer's equivalent mutant claim was not available as text. N3 judges the two surviving mutants directly.
- Next owners: the lead rewrites or bounds the plan sentences in D1 and D2 and decides whether d = 1 and d = 2 windows are accepted with their tier limit named or refused. The stage 1 contraction stays with the F-041 owner. A fresh independent pass 6 then reviews the changes. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report: `git write-tree` is still `aa3c56fa`, the three file sha1s are as listed, and the scratch directory `target/f227-p5` was deleted at the end.
