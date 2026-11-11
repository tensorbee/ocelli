# F-227 review, pass 8

**Reviewed**: library half only. `git diff 8865b33 HEAD` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `7ad814d` ("F-227, two plan sentences corrected after review pass 7"), `git write-tree` `c0b7b2a51a5f9bc8aa0807ee06575eb171f88bdd`, no unstaged tracked changes. The diff touches only `.claude/plans/F-227-design.md`, 2 insertions and 2 deletions at lines 453 and 481. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, and `.claude/plans/F-227-design.md` sha1 `450d943d`. Untracked `tools/oracle/out` and the pass 1 to 7 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the two corrected sentences are the sprint lead's, as the caller's brief says. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 7 were written by the same model, so this pass is independent of the correction but not of the findings it answers.
**Result**: 2 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The diff is the plan only.** `git diff --stat 8865b33 HEAD` lists one file, `.claude/plans/F-227-design.md`. The source and test sha1s match pass 7's, so pass 7's code evidence and mutation table stand for this tree.
- **Pass 7 D1's correction, line 453.** The new text is: "Every constant of magnitude 2^48 or more is refused with `UnverifiableWindow`, which is safe. The magnitudes where refusal begins are superseded by the pass 6 amendment, which leaves them to measurement in the source."
  - The first sentence was re-measured over the part pass 7 did not sample. Every 997th f32 bit pattern from `0x57800000` (2^48) to `0x7F7FFFFF` (`f32::MAX`), both signs, plus both ends, ran as a four-sample constant frame at slope 1 and intercept 0. That is 1,346,220 frames, and **every one returned `Err(UnverifiableWindow)`**.
  - The second sentence is true. The pass 6 amendment says "The magnitudes where refusal begins are measured by the implementation and recorded in the source and tests, not stated here", and they are recorded at `auto_window.rs:327-348`. Pass 7 confirmed those numbers exhaustively.
  - The withdrawn clause about constants between 2^47 and 2^48 is gone, and no other sentence in the plan states it.
- **Pass 7 D2's correction, line 481.** The new text is: "within half a grey of 127.5 on any tier whose rescale lands within 16 f32 units of the CPU value. A tier whose rescale lands further away, the fused cancelling rescale recorded for the shader's owner, is not covered."
  - *Pass 7's example, re-run through the real `auto_window`.* Stored 52636 four times, slope 2.490162 (f32 2.490161895751953) and intercept -131071.08 (f32 -131071.078125). CPU value 1.078125, `f32::mul_add` value 1.0834198, which is 44,416 units away. The window is Constant, centre 1.078125, width 1. The CPU value draws 127.5 and the fused value draws 128.85017, grey 129. An exact rational recomputation in Python gives the same numbers: product 131072.16154 rounds to 131072.15625, fused 1.0834197998046875, display 128.8501739501953. The case the sentence excludes is real.
  - *The 16-unit condition, measured directly rather than through sampled fused values.* For every accepted window, every f32 within 16 units of each CPU percentile was evaluated, 33 points per percentile, through `VoiTransform`. There are five families, 200,000 windows each and 1,000,000 in all, with 0 refused:
    - F1 constants from 2^15 to 2^34.
    - F1b constants at slope 0.5 to 3.5 and intercept -1024 to 1025.
    - F3 cancelling constants, with the intercept within 2 of minus the product.
    - P close pairs 1 or 2 units apart, built as a two-frame pool whose second frame is exact.
    - P3 cancelling close pairs.

    **0 of 46,200,000 neighbour points drew outside (127, 128).** The `mul_add` value was within 16 units 1,001,405 times, and **0** of those drew outside (127, 128).
  - *The excluded case.* Beyond 16 units, the cancelling families F3 and P3 had 398,560 fused values, up to about 1.0e9 units away. 95,240 of them drew outside (127, 128) and 25,103 drew outside grey 127 or 128. The non-cancelling families had only 35 beyond 16 units, at most 459 units out, and none drew outside (127, 128).
  - The f32 ramp at `lut.rs:659` is a chain of monotone f32 operations, so checking the outward endpoints bounds every value between them. That matches the measurement.
  - **Both corrected sentences are true on HEAD.**
- **Every amendment and ruling section, read end to end against the code**, lines 431 to 487. The result is two false sentences, D1 and D2 below, and the rest holds as follows:
  - **Pass 1.** The widening bullet is marked superseded by pass 2. The background bullet matches `maps_to_minimum`. 8388608 and 4194303.75 are in the tests.
  - **Pass 2.** The exact minimum and maximum at the unmoved percentiles is still a necessary condition (`maps_extremes`). The one-step rule is marked superseded by pass 3 and the tier sentence by pass 4. The constant bullet's exact centre check is `display_bits(.., constant) == CHECK_MIDDLE`. The PET series test exists.
  - **Pass 3.** The geometric search matches `search_window`: one unit, doubling, 48 attempts. The seeds rule matches the fixed-seed test. The constant fixtures 16777218, 33554432, -16777216 and 1e9 are present.
  - **Pass 4.** The margin is conditioned on "within the margin" and "at least 33 units apart". 1e30 is tested. Pass 7's M9 showed the last-window mutation goes red.
  - **Pass 4 rulings.** The moved-pair search and the midpoint cap match `margin_pair`.
  - **Pass 5 ruling bullet 1** matches `maps_to_middle`.
  - **Pass 6.** Its conditional "a few units" sentence is true within 16 units, and its refusal and measurement sentences are true.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff --stat 8865b33 HEAD`, `git write-tree`, `shasum` | HEAD `7ad814d` | now | one file, the plan. Tree `c0b7b2a5`, source and test sha1 unchanged from pass 7 |
| `cargo test -p ocelli-pixel` (cargo 1.97.1) | tree `c0b7b2a5` | now | 0, `auto_window` 32 passed in 5.94 s, every binary ok |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| probe crate in the session scratchpad, release, path dependency on the worktree's `ocelli-pixel` and `ocelli-core`, its own target directory | the real `auto_window`, `AutoWindowPool`, `ModalityTransform::apply`, `VoiTransform` and `f32::mul_add` | now | the example, the two close-pair cases, five families of 200,000 and the 2^48-and-above sweep, results above and below |
| Python `fractions` and `struct` | pass 7's example | now | matches the probe |
| pass 7 mutations M1 to M9 and its exhaustive scans | unchanged source | prior receipt, pass 7 | stand for unchanged code |

## Defects

### D1, the pass 5 amendment still says every tier draws the middle grey

**Where**: `.claude/plans/F-227-design.md:474`: "Its window is the constant window centred on the midpoint of the pair, which a rounding difference of a few units cannot move off the middle grey, so every tier draws the middle grey."
**What**: this is the twin of pass 7 D2, two bullets above the sentence that was corrected. It concludes "every tier" with no condition, and the line-481 correction now says directly that a tier whose rescale lands more than 16 units away is not covered. So the plan contradicts itself, and the unconditional half is false.
- Executed: a two-frame pool. Frame 1 is stored 52636 four times at slope 2.490162 and intercept -131071.08, CPU value 1.078125. Frame 2 is stored 1 four times at slope `next_up(1.078125)` and intercept 0, exact on any tier.
- The pool gives a Constant window, a close pair with d = 1, centre 1.078125 and width 1, and the CPU values draw 127.5 and 127.50003.
- Frame 1's fused rescale, 1.0834198, draws **128.85017, grey 129**. With d = 2 (frame 2 at two units up) the window is centre 1.0781251 and width 1, and the fused value draws 128.85014.
- Family P3, cancelling close pairs: 47,538 of 200,000 windows have a fused value outside (127, 128), and 12,486 outside grey 127 or 128.

**Why it is wrong**: microscope section 3, and CLAUDE.md's tier rule. A guarantee stated without its condition becomes the contract that the viewer half and F-041's owner read, and here it now contradicts the corrected ruling.
**Evidence**: the probe's `pair d=1` and `pair d=2` lines and family P3.
**Scope and regression**: prose only, carried from the pass 5 corrections (`3d73f11`). Pass 7 corrected the ruling and missed this one, and so did the lead's correction. The smallest fix is to end the sentence at "cannot move off the middle grey", or to give it line 481's condition. No test is needed, because the covered case is pinned and the uncovered case is the shader's.

### D2, the pass 2 centre rule for a percentile window is false today and not marked superseded

**Where**: `.claude/plans/F-227-design.md:443`: "When the computed window fails, the width is narrowed ..., keeping the centre the midpoint `low + width / 2`, at most 64 steps".
**What**: on HEAD, `finish` searches the Percentile window with `center_of = |width| low_moved + width / 2.0` (`auto_window.rs`, the `search_window(high_moved - low_moved, Search::Narrow, ..)` call), where `low_moved` is the 1st percentile moved up to 16 units toward the 99th. For any pair at least 3 units apart, `low_moved != low`, so the centre is not `low + width / 2`. Pass 3 marks only "the one-step rule" of this bullet as superseded and says "keeping the centre rule of each window kind", which reads as reaffirming this centre. The pass 4 ruling at line 467 sets `low_moved + width / 2` but marks only the literal reading of the pass 4 amendment, not this sentence. Two contradictory centre rules stand, and the earlier one is not marked superseded. The plan's own convention elsewhere is to name what it supersedes, as in "The pass 1 bullet that widened is superseded" and "The tier sentence of the pass 2 amendment is superseded".
**Why it is wrong**: microscope section 3. The caller's brief also requires a superseded rule to be marked as such rather than left standing.
**Evidence**: read against the source. Pass 5's M10, the literal `low + width / 2` reading, went red in 10 tests with about 98 percent of pairs refused, so the code demonstrably does not follow this sentence.
**Scope and regression**: prose only, carried since the pass 4 rulings and not flagged by passes 5 to 7. The smallest fix is to add "superseded by the pass 4 rulings' centre" to the pass 2 bullet, or to the pass 4 ruling. Body item 8 at line 153 states the same original centre and is covered by the amendments' general authority, so it needs nothing.

## Smells

None.

## Nitpicks

- **N1** `.claude/plans/F-227-design.md:453`. "The magnitudes where refusal begins are superseded" refers to a clause that is no longer in the text, so a reader without the history sees it superseding nothing visible. "Where refusal begins is left to the pass 6 amendment" would say the same without pointing at deleted text. The sentence is true.
- **N2** `.claude/plans/F-227-design.md:468`. "adjacent pairs none" is literally true of `min(16, (d - 1) / 2)`. Since the pass 5 amendment, though, a pair 1 or 2 units apart never reaches `margin_pair` and gets the 16-unit outward check instead, which this bullet does not point to.
- **Carried**: pass 7 N1 to N4 and the nitpicks pass 7 carried, unchanged because the code is unchanged.

## Verified clean

- Pass 7 D1 is closed: the clause is deleted, the replacement is true, and refusal at 2^48 and above was swept to `f32::MAX`.
- Pass 7 D2 is closed: the condition holds over 46.2 million neighbour points and 1.0 million `mul_add` values within 16 units, and the excluded case reproduces as stated.
- "Recorded for the shader's owner" holds. `docs/sprints/CURRENT_SPRINT.md` on `sprint/s46` (`45c4127`, `73b68ad`, neither an ancestor of this branch) records the fused rescale and the cancelling case for the operator, against `voi.wgsl`.
- Every other amendment and ruling sentence was checked, as listed under Acceptance challenged.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` all exit 0.

## Limitations and handoff

- Not exercised: a real GPU. Stage 1 on tier A was emulated with `f32::mul_add`. A tier's own VOI evaluation, which F-041 records as agreeing to two f32 units, was not emulated, and the CPU `VoiTransform` drew every point. The line-481 guarantee conditions on the rescale only. The display margins pass 7 recorded (at least about 0.002 grey from 127 or 128 in its fixtures) are far wider than two units at 128, but no sweep of the minimum margin was made. Tier B, the viewer half, the oracle and real bytes were not exercised. The 2^48-and-above sweep is a stride-997 sample, not exhaustive.
- Out of scope, noted for the lead: `45c4127`'s sprint entry says "F-227 now accepts a window only with a 16 unit margin, so its own windows hold". D1's pool shows an F-227 Constant window drawing grey 129 on tier A under cancellation. `73b68ad` adds that case without withdrawing that clause.
- Next owners: the lead conditions or truncates line 474 (D1) and marks the pass 2 centre rule superseded (D2). Both are prose only, and a fresh independent pass 9 then reviews them. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` and the three sha1s were re-checked at the end, and the probe lives only in the session scratchpad.
