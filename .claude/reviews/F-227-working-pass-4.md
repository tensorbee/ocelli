# F-227 review, pass 4

**Reviewed**: library half only. `git diff e952721` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `e952721be40ae29d932d814e3558a535233a752a` (the pass 3 checkpoint). The round is staged: `git write-tree` `3ecd21e131f873bceda29a82fdff0bbbb5c40f13`, no unstaged tracked changes. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `041ac7db`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `66a619a7`, `.claude/plans/F-227-design.md` sha1 `3cbbc98b`, `crates/ocelli-pixel/src/lib.rs` sha1 `d90ec5ff` (unchanged). The plan's "Lead amendments after review pass 3" is treated as authoritative, with the pass 2 bullet's tier sentence as the lead corrected it. Untracked `tools/oracle/out` and the pass 1 to 3 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the round is by a Claude implementer, attributed by the caller's brief. The amendments and the corrected tier sentence are the sprint lead's. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 3 were written by the same model, so this pass is independent of the remediation but not of the findings it answers.
**Result**: 3 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 3 D1, the hunted seed, is closed.** `REFUSAL_SEEDS` is `1` to `8`. Replaying the test's own generators against checkpoint `e952721`'s `auto_window.rs` in a scratch copy reproduces the comment's before counts seed by seed (uniform 1, 1, 1, 1, 0, 3, 1, 1, then integer times slope 1, 1, 1, 5, 1, 0, 1, 2, then CT-like 1, 0, 0, 1, 1, 0, 0, 1, then PET 0) and both named first refusals (63.38857 with 64.07005, and 46549 with 46377 at slope 1.411621). Seeds that refused on the old code were not chosen to pass on it.
- **Zero refusals on my own seeds.** Twelve seeds the implementer did not use (`0x5EED0001`, `0x5EED0002`, `0x123456789ABC`, `0xDEADBEEF`, `0xC0FFEE00`, `0x0BADF00D`, 9, 10, 11, 12, `u64::MAX`, `0x8000000000000001`), 250,000 pairs per seed per pair distribution with the test's exact generators, and the full 2,000-slice PET series plus its pooled window per seed. 9,024,012 windows: **0 refused, 0 failing**. Each accepted window was checked through the real `VoiTransform` and through a separate C.11.2.1.3.2 evaluation written in the probe. The 1st percentile drew exactly `0.0` and the 99th exactly `255.0`. The centre was bit for bit `low + width / 2`. The width was the span or `span - unit * 2^k` for some `k` up to 47. The largest attempt index was 11 on the pair distributions and 0 on PET.
- **Edge cases, all accepted and all exact.** Pairs straddling every power of two from `2^-20` to `2^30` with log-spread offsets, in positive, negative and mixed-sign form, plus adjacent and next-but-one floats around each boundary: 3,061,224 frames, max attempt 23. Adjacent float pairs anywhere in the range, both signs: 1,992,107 frames, max attempt 1. Arbitrary finite bit-pattern pairs: 1,984,468, with 56 `NonFiniteWindow` where `high - low` overflows. Subnormal pairs: 23,322. Pairs near `f32::MAX`: 10,000, with 2,000 `NonFiniteWindow` for `-MAX, MAX`. Not one percentile window was refused anywhere.
- **Constants.** Every 257th bit pattern of both signs, one counted sample: 11,391,534 accepted, every one centred on the value bit for bit and drawing exactly `127.5`. The amendment's four values give widths 2, 3, 3 and 65 at attempts 23, 24, 24 and 29, as the fixture says. **Every constant with magnitude at or above 1.4074e14, about `2^47`, is refused**: 5,255,120 of the sweep, and 1e15, 1e20 and 1e30 of both signs. D2 covers this.
- **Background.** Every 257th bit pattern of both signs, each left out by its own padding: 16,646,656, none refused, every one drawing exactly `0.0`, with `low` the value, `high` equal to `center + width / 2` and the centre `value + width / 2`, all compared bit for bit. The one probe mismatch was stored `-0.0`, whose modality value is `+0.0`, which is a probe artefact. No case needed more than attempt 0. `±f32::MAX` and `±1e30` are accepted at width 1.
- **Fixtures, re-derived by hand and executed in the probe.** Straddling 64: span `0x3F8C5DA0`, unit `2^-23`. Attempts 0 to 4 leave the centre at `0x428078A3` and the upper edge on the 99th, which draws 254.99957 to 254.99979. Attempts 5 and 6 drop the lower edge to `0x427EBFCE` and give 0.00088915 and 0.00044078. Attempt 7 gives width `0x3F8C5D20`, centre `0x428078A2`, edges `0x427EBFD0` and `0x4281915C`, and exactly 0 and 255. Even mantissa: the unit is `2^-47`, attempts 0 to 22 keep the lower edge at `0x3F7FFFFF` with 127.5, and attempt 23 is width `2^-24` with the lower edge tying to the even `1.0`. Every sentence of both comments holds. So do the review example (lower and upper edges round onto the percentiles themselves, ramp exactly 255), the 0.001 pair (pass 3 D3.1 corrected: both edges tie to `0x3A831270`, 99th draws 0), and the four constant derivations, including the parity of `0x4BFFFFFF` and `0xCB800001`.
- **Comment claims in `search_window`, executed.** The unit is a power of two. A change that overflows gives a non-finite or negative width and ends the search. Narrowing ends once the change reaches `first`, so it never reaches 48 attempts (the edge sweep peaks at 23). "A 99th percentile on the upper edge itself, the usual case" holds: 60.1% of accepted uniform windows have the 99th bit for bit on the upper edge, and 0.8% have it strictly below while the ramp still gives exactly 255.
- **The tier sentence, judged against real code and a real adapter.** `display_index` (`ocelli-render/src/colour.rs:482`) and the tier C store `byte_from_channel` (`cpu.rs:1055`) both send a display value within a few f32 units of 255 to 255 and one near 0 to 0, because the threshold is a half. Tier C draws through `ocelli-pixel` itself, so it is exact. Tier A was then run: `voi.wgsl`'s `lut_chain` on Apple M4 Max, Metal, through the F-041 harness shape, on 4,510 accepted percentile windows. Where the GPU's stage 1 equals the CPU's, stage 2's error is relative to the quotient near 0.5 and cannot reach a half grey. The GPU's stage 1 is not the CPU's, though. D3 covers this.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | staged tree above | now | 0 |
| `cargo clippy -p ocelli-pixel --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-pixel` | same | now | 0, 96 tests over 10 binaries, `auto_window` 27 passed |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| `python3 scripts/guard_census.py` | same | now | 0, 1102 refusals in 78 files, all claimed |
| probe crate `target/f227-p4/probe` (`dists`, `edges`, `constants`, `background`, `fixtures`, `fma`), path dependency on the worktree crates | real `auto_window`, `AutoWindowPool`, `AutoWindow::transform`, `VoiTransform::apply` | now | the sweeps above |
| GPU probe `target/f227-p4/gpu`, `VOI_WGSL` plus F-041's compute harness, `VoiParams::from_chain`, `wgpu =30.0.1` | Apple M4 Max, Metal backend, tier A | now | D3 |
| before counts, the test's `no_window_is_refused_on_the_review_distributions` run with `--nocapture` against `git show e952721:crates/ocelli-pixel/src/auto_window.rs` in a scratch copy | checkpoint source | now | matches the comment and `red_run.txt` exactly |
| 17 source mutations in a scratch copy of `ocelli-core` and `ocelli-pixel`, each applied by exact-anchor replace with the anchor count asserted 1, original restored and sha1 `041ac7db` re-checked after the run | `auto_window.rs` sha1 `041ac7db` | now | table below |

| Mutation | Result |
|---|---|
| R1 `SEARCH_ATTEMPTS = 7` | red: straddle 64, even mantissa, constants, distributions |
| R2 `SEARCH_ATTEMPTS = 22` | red: even mantissa, constants |
| R3 one bit step at a time (`change += unit`) | red: straddle 64, even mantissa, constants, distributions |
| R4 percentile check, high side only | red: straddle 64, even mantissa |
| R5 percentile check, low side only | red: four tests |
| R6 Background `high = center + width` | red: `the_window_check_draws_the_4194303_75_maximum_black` |
| R7 narrowed width with the unnarrowed centre | red: three tests |
| R8 found width returned with the computed centre | red: five tests |
| R9 `CHECK_MIDDLE = 127.0` | red: six tests |
| R10 narrowed centre `(low + width) - width / 2` | red: five tests |
| R11 budget exhaustion returns the last tried window instead of refusing | **survived** (D1) |
| R12 constant check removed | red: `a_constant_frame_draws_the_middle_grey_at_large_magnitudes` |
| O1 widening starts at two units | red: three tests |
| O2 narrowing starts at two units | red: three tests |
| O3 tripling instead of doubling | red: four tests |
| O4 constant centre moves with the width | red: constants |
| O5 background centre fixed at `value + 0.5` | red: both background fixtures |

R1 to R12 replay pass 3's mutation set adapted to `search_window`. Pass 3's M22 and M25 are unchanged code and were not re-run. O1 to O5 are this pass's own.

## Defects

### D1, no test reaches `UnverifiableWindow`, so a search that returns an unchecked window passes the suite

**Where**: `crates/ocelli-pixel/src/auto_window.rs:486-500` (`search_window`'s loop and final `None`), `:341-348` and `:397-399` (the two refusals). The round deleted the three refusal fixtures at the checkpoint's `tests/auto_window.rs:700, 725, 904` and added none. `grep UnverifiableWindow tests/auto_window.rs` now finds nothing.
**What**: mutation R11 makes an exhausted search return its last tried width without the check passing. All 27 tests stay green. Under R11, `auto_window(&[Stored(1e30); 4], identity, None)` returns centre `1e30`, width `16777216`, and the constant draws **exactly 0.0, grey 0**, where the tree refuses. The same happens for `1.5 * 2^47` and `-1e20`. The tree has a reachable refusal (D2), so this is a live path that nothing executes.
**Why it is wrong**: microscope sections 2 and 4. The amendment's rule is that the first width that passes is returned and nothing else. A test suite that passes with that rule removed is not guarding it. The failure it hides is a quietly wrong pixel, a constant frame drawn black.
**Scope and regression**: every `AutoWindow` consumer, today the tests only. Smallest regression: one fixture asserting `Err(AutoWindowError::UnverifiableWindow)` for a constant beyond the search's reach, for example stored `1e30`, or `1.5 * 2^47` if the lead keeps the rule (D2).

### D2, the amendment says a constant above 2^24 draws the middle grey, and every constant from about 2^47 up is refused

**Where**: `.claude/plans/F-227-design.md:453`, the pass 3 amendment's third bullet, against `auto_window.rs:393-395` and `SEARCH_ATTEMPTS`.
**What**: the bullet's heading is unconditional: "A constant frame above 2^24 draws the middle grey." The widening rule the same amendment fixes starts the change at one unit of width 1, `2^-23`, and doubles it at most 48 times, so the largest change is `2^24`, and `1 + 2^24` rounds to `2^24`. A value whose spacing below is `2^24` or more needs a half width of at least half that spacing for the lower edge to round below it, which the rule never reaches. Measured: `1.5 * 2^46` is accepted at width 8388609, and every magnitude from 1.4074326e14 up is refused, 5,255,120 of the every-257th-pattern sweep, including 1e15, 1e20 and 1e30 of both signs. The refusal is safe, and the tree's own doc says "The window is refused when none does", so the code's prose is honest. The plan's is not.
**Why it is wrong**: microscope section 3, a claim in prose that is false, in the authoritative text a later reader will treat as the contract. The heading and the search rule written beside it contradict each other.
**Scope and regression**: constant-percentile frames with modality values of at least about `1.4e14`, which no ordinary modality produces. Nothing is drawn wrong. Owner: the lead. Either bound the heading (for example, "from 2^24 to 2^47, and refused above") or change the constant rule, for example by starting the constant's change at the value's own f32 spacing. The regression is D1's fixture at whichever boundary is chosen.

### D3, the corrected tier sentence is false on tier A, measured on Metal

**Where**: `.claude/plans/F-227-design.md:443`, the sentences beginning "In most accepted windows" and "What holds on every tier is the grey", added in this round. The relevant code is `crates/ocelli-render/shaders/voi.wgsl:122`, stage 1 `stored * voi.slope + voi.intercept`, against `ocelli-pixel/src/lut.rs:200`, the same expression on the CPU.
**What**: the sentence claims that tiers A and B "can differ by a unit or two" and that "the 1st percentile draws black and the 99th white everywhere". On Apple M4 Max under Metal, the shader's stage 1 is contracted to a fused multiply-add. Every GPU value below equals the probe's `f32::mul_add` emulation bit for bit. The consequences:
- Six of 4,510 accepted percentile windows draw both percentiles the same grey. For example, stored 2371 and 2372 with slope 9.00603e-7 and intercept 1854.0933 give CPU modality values `1854.0953` and `1854.0955`, adjacent floats. The accepted window has centre `1854.0953` and width `1.22e-4`, one ulp. The CPU draws 0 and 255, and **the GPU draws 0 and 0, the 99th percentile black**. Stored 1988 and 1989 at slope 7.6754475e-7 and intercept 1736.4397 draw **255 and 255, the 1st percentile white**. All six have a width of about one ulp of the modality values, which takes a small slope and a large intercept.
- On ordinary wide windows the grey holds, but the gap is not "a unit or two". CT-like windows, for example stored 45887 and 52359 at slope 1.2525758 and intercept -1024 with width 8106.7, put the 99th at 254.99988, eight f32 units below 255 at the top of the range, and the 1st up to 3.4 units above 0. F-041's two-unit figure was measured with rescales whose product is exact, slope 1 (`voi_shader.rs:103`) and slope 2 with intercept -1024 (`:783`), where a fused stage 1 changes nothing.
- Stage 2 is not the cause. Where the two stage 1 values agree, the GPU's division error is relative to a quotient near 0.5 and stays far below half a grey for any width.
**Why it is wrong**: microscope sections 3 and 5. The sentence claims a tier property that does not hold, with a citation that does not cover it. "Within two units of 0" also does not describe the 3.4e-5 the GPU gives at the low end, though that value still rounds to 0. Tier B was not exercised, and F-041's harness is tier A only (`voi.wgsl:154`), so the sentence's claim about B rests on no measurement either.
**Scope and regression**: no consumer draws an automatic window on the GPU yet, and the viewer half decides the tier. The divergence itself is F-041's, a stage-1 difference that affects every window, declared ones included. F-227's windows only make it visible as a grey flip when the width is one or two ulps of the values. Owner: the lead for the sentence, which should state that exactness and the greys are established for this crate's evaluation and for tier C, and that tiers A and B differ wherever the shader's stage 1 is contracted, with the narrow-window flip named. The stage-1 contraction and F-041's agreement figure under a non-identity rescale go to the F-041 or F-038 owner as a separate handoff, not to this story. Smallest regression in this story: none in `ocelli-pixel`. The GPU probe's frame above is the trigger to keep for the viewer half.

## Smells

None.

## Nitpicks

- **N1** The pass 2 amendment bullet the lead just edited still ends "narrowed by one f32 step at a time ... at most 64 steps". The pass 3 amendment supersedes it explicitly, but the bullet was touched in this round and left saying the superseded rule.
- **N2** The two background fixture comments (`tests/auto_window.rs:497, 531`) still say "widens the width one step". That is the first attempt, a change of one unit, so the sentence is true, but it is the old search's vocabulary.
- **N3** Carried: pass 1 N1 to N5 and pass 2 N2 and N3, including the 4 MiB reserve for every one-frame pool. Pass 3 N1, the three `as` casts, is closed: neither file has an `as` cast now.

## Verified clean

- Pass 3 D1 is closed (above). Pass 3 D2 is closed for the four named values and open above `2^47` (D2). All four pass 3 D3 sentences are corrected or gone, and each replacement was executed.
- The check exists once. `display_bits` builds `VoiTransform::new(.., LinearExact, 0, 255)` and calls `apply`. `search_window` contains no edge or ramp formula. `CHECK_MIDDLE` is a range constant, not window arithmetic.
- No returned window fails its own check, over 9.0 million distribution windows, 7.1 million edge-case pairs, 11.4 million constants and 16.6 million backgrounds.
- The attempt bound holds everywhere it was measured: at most 23 when narrowing and at most 29 when widening for any accepted case.
- No `unsafe`, no `#[allow]` and no `as` cast in either file. `finish`'s doc, `search_window`'s doc, `Search::Narrow`, `AutoWindow::high` for Background and `UnverifiableWindow`'s doc all match the code.
- No consumer outside `ocelli-pixel` names `auto_window`, `AutoWindow` or `UnverifiableWindow`. `README.md`, `use.html`, `more.html` and `docs/lld` are untouched by the diff.
- Seventeen mutations run, sixteen red. The refusal record in the test prints each seed's count and first inputs.

## Limitations and handoff

- Not exercised: the viewer half, tier B, any GPU other than Apple M4 Max under Metal, the oracle, and real PET bytes. The PET results use the test's synthetic frames.
- The seeds being "written down before the test was first run" is supported by `red_run.txt` and by the old code refusing on them. It cannot be proved from the tree.
- Next owners: the lead decides D2's rule or wording and rewrites D3's sentence. The implementer then adds D1's refusal fixture at the boundary D2 settles. The stage-1 contraction goes to the F-041 or F-038 owner. A fresh independent pass 5 reviews the changes. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report: `git write-tree` is still `3ecd21e1`, the file sha1s are as listed, and the scratch directory `target/f227-p4` was deleted.
