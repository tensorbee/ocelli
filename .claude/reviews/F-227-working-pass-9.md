# F-227 review, pass 9

**Reviewed**: library half only. `git diff 7ad814d 4da5d53` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `4da5d53` ("F-227, plan sentences from review pass 8 corrected"), `git write-tree` `759644955b50d68131c8f3cfbdde3793b8c525d1`, no unstaged tracked changes. The diff touches only `.claude/plans/F-227-design.md`, 3 insertions and 3 deletions at lines 153, 443 and 474. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, and `.claude/plans/F-227-design.md` sha1 `39cf4c2d`. Untracked `tools/oracle/out` and the pass 1 to 8 reports are not part of the review. The viewer half, AC10 to AC18, is out of scope.
**Author / reviewer**: the three edited sentences are the sprint lead's, as the caller's brief says. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 8 were written by the same model, so this pass is independent of the correction but not of the findings it answers.
**Result**: 1 defect, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 8 D1, line 474.** The new text is: "Its window is the constant window centred on the midpoint of the pair, checked as the pass 5 rulings below state."
  - The unconditional "so every tier draws the middle grey" is gone. No other sentence in the plan concludes "every tier" for a grey. Line 383's "the window is the same on every tier" is about the centre and width, which `VoiParams::from_chain` reads, and is true. Line 460 conditions on the margin.
  - Against `finish`: a close pair (`span > 0.0 && high <= low.next_up().next_up()`) gets `(low + span / 2.0, 1.0, Constant)`, the f32 midpoint at width 1, then the widening search with the centre held. Its check `maps_to_middle` requires the centre to draw exactly `CHECK_MIDDLE` and `low_out`, `low`, `high`, `high_out` to draw strictly within 0.5 of it. That is the pass 5 rulings' check, together with pass 2's exact-centre check, which the ruling's "checking the centre alone" presupposes. **True.**
  - Probe through the real `auto_window`: stored 1000 and `next_up(1000)`, slope 1. Result `Constant`, centre 1000.0 (the tie rounds to the even significand), width 1.0.
- **Pass 8 D2, line 443.** The clause "keeping the centre the midpoint `low + width / 2`" is deleted rather than marked. The bullet now states no centre. Pass 3's "keeping the centre rule of each window kind" (line 451) now resolves only to the current rules: `low_moved + width / 2` (pass 4 rulings), the constant held (`|_| constant`), and `value + width / 2` for Background. `grep -n "low + width" ` over the plan finds only item 8, which is now marked. **Closed.**
- **The new sentence at line 153**, "The lead rulings on the pass 4 remediation supersede this formula with the moved pair." For pairs at least three units apart, `finish` searches from `high_moved - low_moved` with centre `low_moved + width / 2.0`. Probe on AC1: centre `0x41C7FFF0` (24.99997), width `0x4243FFF0` (48.99994), against the item 8 formula's 25. **True** for every pair that reaches the moved pair. Close pairs never reach it, see N1.
- **The whole plan, swept once more** for a sentence a later amendment or ruling contradicts. The one found is D1. The rest is listed under Verified clean.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff 7ad814d 4da5d53`, `git diff --stat 7ad814d HEAD`, `git write-tree`, `shasum` | HEAD `4da5d53` | now | one file, the plan. Tree `75964495`, source and test sha1 unchanged from pass 7 and pass 8 |
| `cargo test -p ocelli-pixel`, `CARGO_TARGET_DIR` the worktree's `target` | tree `75964495` | now | every binary `ok`, `auto_window` 32 passed in 5.85 s, 0 failed anywhere |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| probe crate under the worktree's `target/`, path dependency on `ocelli-pixel` and `ocelli-core`, deleted after the run | the real `auto_window`, `AutoWindow::transform`, `VoiTransform::new` | now | AC1 window as above. `transform(0.0, 255.0) == VoiTransform::new(None, &[25.0], &[49.0], 0, LinearExact, 0.0, 255.0)` is **false**. The close pair at 1000 as above |
| pass 7 and 8 mutations, scans and sweeps | unchanged source | prior receipts | stand for unchanged code |

## Defects

### D1, AC9's transform equality is false on HEAD and nothing marks it superseded

**Where**: `.claude/plans/F-227-design.md:327`, AC9: "`AutoWindow::transform(0.0, 255.0)` of AC1 equals `VoiTransform::new(None, &[25.0], &[49.0], 0, VoiFunction::LinearExact, 0.0, 255.0)`". Line 315 says of the inventory "Every value is hand-computed from the formulas above".
**What**: since the pass 4 rulings, AC1's window is computed on the moved pair, centre `0x41C7FFF0` and width `0x4243FFF0`, not 25 and 49. The equality the AC states is an exact one, and it does not hold. The AC9 test itself was changed to assert the moved bits (`tests/auto_window.rs:1903-1910`, `f32::from_bits(0x41C7_FFF0)` and `0x4243_FFF0`), so the plan's acceptance row and the test that claims it now say different things. The test file's header (lines 19 to 28) records that the integer centres and widths of AC1 to AC8 hold to within `assert_close`'s 0.001, which makes those rows readable as values under a tolerance. AC9 is not one of them, because it states equality of two transforms. No amendment or ruling names AC9, and the new pointer at line 153 supersedes item 8's formula without reaching the inventory computed from it.
**Why it is wrong**: microscope section 3, a false claim in prose. The acceptance inventory is the contract `/complete-feature` maps tests to, so a reader checking AC9 against its test finds a mismatch, or a reader implementing from the plan writes the equality the plan states and goes red.
**Evidence**: the probe line `AC9 equals (25,49): false`, with the AC1 window printed as `24.99997 0x41c7fff0`, `48.99994 0x4243fff0`.
**Scope and regression**: prose only, carried since the pass 4 rulings (`low_moved + width / 2`). Passes 5 to 8 treated the body and inventory as covered by the amendments' general authority, and pass 8 said so of item 8. The lead's new explicit marker at item 8 is what makes the unmarked inventory row stand out. The smallest fix is to restate AC9 as equality with `VoiTransform::new` of AC1's own centre and width, or with the two bit patterns the test asserts. One sentence at line 315 noting that the inventory's centres and widths are the item 8 values, which the moved pair changes by at most about 5e-4 below 256, would cover AC1 to AC8 too. No code or test change is needed, the test is already right.

## Smells

None.

## Nitpicks

- **N1** `.claude/plans/F-227-design.md:153`. The new pointer names only the pass 4 rulings. For percentiles 1 or 2 units apart, which `high - low` greater than zero puts under item 8 rather than item 9's Constant line, the replacement is the pass 5 amendment's constant window, and following the pointer gives `margin_pair` with no move and so item 8's formula back. The pass 5 amendment is later and unambiguous, so no reader in chronological order is misled. The derivation after the pointer ("`x <= c - w/2` is `x <= low` ... with no half and no one") describes the superseded formula and is not scoped as such. Pass 1's bullet 3 withdraws exactness sentences in general, so it is not false as written.
- **N2** carried, pass 8 N1, line 453: "The magnitudes where refusal begins are superseded" refers to a clause no longer in the text.
- **N3** carried, pass 8 N2, line 468: "adjacent pairs none" is literally true of `min(16, (d - 1) / 2)`, but since pass 5 adjacent pairs never reach `margin_pair` and get the 16-unit outward check, which the bullet does not point to.
- **Carried**: pass 7 N1 to N4 and the nitpicks pass 7 carried, unchanged because the code is unchanged.

## Verified clean

- Pass 8 D1 closed and the replacement sentence true against `maps_to_middle` and a probe.
- Pass 8 D2 closed by deletion. No other percentile centre rule other than the marked item 8 remains.
- Line 153's new sentence true for every Percentile window, by source and probe.
- The sweep, every amendment and ruling section (lines 431 to 487) and the body items they touch:
  - Item 9's "Exact while `|m_max| < 2^22`" is withdrawn explicitly by pass 1 bullet 3. Its Constant line "grey 128" holds on tier C because the exact-middle check gives display 127.5, which `floor(y + 0.5)` draws as 128.
  - Pass 1's widening bullet is marked superseded by pass 2. Its background bullet matches `maps_to_minimum` over the pool's largest left-out value.
  - Pass 2's one-step rule is marked superseded by pass 3, its tier sentence by pass 4. Its constant bullet's exact middle is `display_bits(.., constant) == CHECK_MIDDLE`.
  - Pass 3's geometric search matches `search_window`, 48 attempts. Its 2^48 sentence was swept by pass 8.
  - Pass 4's four-point check matches `maps_extremes`. 1e30 is tested (`tests/auto_window.rs:1152`).
  - Pass 4 rulings match `margin_pair` and the `search_window(high_moved - low_moved, Narrow, |w| low_moved + w / 2.0, ..)` call.
  - Pass 5's close-pair rule matches the `close` branch. The 2371 and 2372 example is a test (`tests/auto_window.rs:702-750`).
  - Pass 5 rulings and pass 6 match `outward_pair` and `near_middle`, and line 481's 16-unit condition was measured by pass 8.
  - Line 383's tier A sentence is about the centre and width, not the grey, and is true.
  - The API block at line 252 lacks `UnverifiableWindow`, which pass 1 announces as "a new named error" and pass 3 names, so the amendment supersedes it.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` all pass.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half, the oracle, real bytes. No new mutation was run, because no code changed since pass 7's mutation table.
- Next owner: the lead restates AC9 (D1), prose only, and a fresh independent pass 10 reviews it. This is a bounded review of the library half, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` and the three sha1s were re-checked at the end. The probe lived under the worktree's ignored `target/` and was deleted.
