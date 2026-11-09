# F-227 review, pass 11

**Reviewed**: library half, plus the plan's viewer rows insofar as they state library values. `git diff b502385 90f02b6` in worktree `ocelli-wt-f-227`, branch `work/f-227-xai`, HEAD `90f02b6` ("F-227, AC11 and the reply example name the window's own strings"), `git write-tree` `7bb9eea240a54aa9f17a83edf684b4e68e444f15`, no unstaged tracked changes. The diff is two lines of `.claude/plans/F-227-design.md`, lines 266 and 329. Files as reviewed: `crates/ocelli-pixel/src/auto_window.rs` sha1 `ddc98990`, `crates/ocelli-pixel/tests/auto_window.rs` sha1 `882b72a2`, both unchanged since pass 7, and the plan sha1 `6be62673`. Untracked `tools/oracle/out` and the pass 1 to 10 reports are not part of the review.
**Author / reviewer**: the two edited lines are the sprint lead's, as the caller's brief says. Review by Claude Opus 5.5 as an independent subagent that wrote none of the reviewed code, plan or amendments. Passes 1 to 10 were written by the same model, so this pass is independent of the correction but not of the findings it answers.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 10 D1, AC11 at line 329.** The row now states "centre and width the `f32` `Display` strings of the library's window for AC3's inputs" and a header "carrying those two strings unchanged", grey 129 at stored 50, `series`.
  - Through the real library, AC3's inputs (`new(2)`, stored 1 to 100 at slope 1, then at slope 2) give centre `98.99988` (`0x42C5FFF0`) and width `193.99976` (`0x4341FFF0`). Those `Display` strings parse back to the same bits, and `VoiTransform::new` built from the parsed strings maps frame B's stored 50 (modality 100) to 128.81459, grey 129. **The row is true and closed.** A literal `99 194` header is no longer stated anywhere in the plan.
  - Consistency with the route's other statements. Item 17 (line 168) says the process returns centre and width "as text", Rust `f32` `Display`, and the page never converts it. Line 266 now says the same with placeholders. Line 303 says "The page sends back exactly the strings it received". AC11's "carrying those two strings unchanged" is the same rule, and AC11's rejected implementation "A route that reads the page's numbers into arithmetic of its own" matches item 17. AC14's header checks (finite, width positive) match item 17's. No contradiction.
- **Pass 10 N1, line 266.** The reply example is now `{"center":"<c>","width":"<w>",...}` with `<c>` and `<w>` defined as the window's `Display` strings. No integer pair remains to copy into a test. **Closed.** The example's `"sampled":16` and `"counted":262144` are unscoped template values, not centre, width or header values, and claim no case.
- **The sweep for literal centre, width or header values the moved pair makes false**, over the whole plan: the story row and backlog row, the transcribed sources, items 1 to 21, the reproducer, the scope and API block, the folder viewer bullets, the consumer, state and ownership tables, AC1 to AC18 and the RED and mutation paragraph, the approach, the tests table, boundary and tier, LLD impact, user-facing documentation impact, and the amendment sections. One sentence is false, D1. Everything else either holds at its written precision as pass 10's probe table showed (AC1 to AC9, AC13), states today's or a hypothetical wrong rule (the reproducer's centre 250 and width 500, the rejected-implementation cells), is explicitly superseded (item 8's formula, item 9's "Exact while"), or states no centre or width (AC10, AC12, AC14 to AC18, the documentation sections).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git diff b502385 90f02b6`, `git diff --stat b502385 HEAD`, `git write-tree`, `shasum` | HEAD `90f02b6` | now, start and end | two lines of the plan. Tree `7bb9eea2` at both. Source and test sha1 unchanged from pass 7 |
| probe crate `target/pass11-probe`, path dependencies on `ocelli-pixel` and `ocelli-core`, `CARGO_TARGET_DIR` the worktree's `target`, deleted with its build outputs after the run | the real `AutoWindowPool`, `auto_window`, `VoiTransform::new` | now | AC3 `98.99988` `0x42C5FFF0`, `193.99976` `0x4341FFF0`, round trip `true`, grey input 128.81459. Stored 3 and 9: Percentile, low 3, high 9, centre 5.9999943, width 5.999981, edges 3.0000038 and 8.999985. 1.0 and its `next_up`: Constant, centre 1, width 1, edges 0.5 and 1.5 |
| `cargo test -p ocelli-pixel` | tree `7bb9eea2` | now | every binary ok, 0 failed. `auto_window` 32 passed in 5.78 s |
| `python3 scripts/prose_check.py` | same | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, clean over 1708 files |
| pass 10 probe table for AC1 to AC9 and AC13 | unchanged source | prior receipt | stands for unchanged code and unchanged rows |

## Defects

### D1, item 9 says a two-value window "is exactly the two values", which the moved pair and the close-pair rule make false

**Where**: `.claude/plans/F-227-design.md:157`, item 9: "Two counted values: ranks 1 and 2, the window is exactly the two values."
**What**: since the lead rulings on the pass 4 remediation, a Percentile window is computed on the pair moved 16 `f32` units toward each other, so its edges lie inside the two values. Stored 3 and 9 through the real `auto_window` give centre 5.9999943 and width 5.999981, edges 3.0000038 and 8.999985, not 3 and 9. Since the pass 5 amendment, two values fewer than three units apart take the Constant window, so 1.0 and its `next_up` give width 1 and edges 0.5 and 1.5, further still from "the two values". The ranks half of the sentence is true and the window half is false. Before the pass 4 rulings the window was `width = high - low`, centre `low + width / 2`, and the sentence was true.
**Why it is wrong**: microscope section 3, a false claim in prose. It is the same kind of unscoped exact claim as pass 9 D1 (AC9) and pass 10 D1 (AC11). Item 8 carries a pointer saying its formula is superseded, but item 9's bullet carries none, and the test that owns the case says otherwise: `crates/ocelli-pixel/tests/auto_window.rs:480-481`, "the window is the two values each moved 16 f32 units inward by the pass 4 margin". A reader writing a viewer test or the LLD subsection from item 9 would assert edges at the two values and go red on the correct library.
**Evidence**: the probe lines above, printed with `{}` through the real library on HEAD.
**Scope and regression**: prose only, carried since the pass 4 rulings, not found by passes 5 to 10, whose sweeps covered the inventory rows and item 9's Constant and Background lines. No code or test is affected and nothing is red. The smallest fix is to delete "the window is exactly the two values" or to say the window is the two values moved inward by the pass 4 margin, as the test comment does. AC8's "Stored 3 and 9: centre 6, width 6" holds at its written precision and needs no change.

## Smells

None.

## Nitpicks

- **N1** carried, pass 10 N2: the inventory does not say it writes centres and widths to the precision shown. Every row holds at that precision, so this is not blocking, but it is the reading that makes rows like AC8's "centre 6, width 6" true beside the exact claim in D1.
- **Carried**: pass 9 N1 to N3 and the pass 7 nitpicks pass 9 carried, unchanged because their text and code are unchanged.

## Verified clean

- Pass 10 D1 closed. AC11 is true against the library and the round trip, and consistent with item 17, line 266, line 303 and AC14.
- Pass 10 N1 closed. Line 266 states no literal window.
- No literal `99`, `194` or `25`, `49` reply or header string remains in the plan. `grep` for `Series-Window` finds only lines 168, 266, 267, 279 and 329 (AC11), none with a literal value. AC12 and AC14 say "series header" and state no value.
- The sweep listed under Acceptance challenged, one finding.
- `cargo test -p ocelli-pixel`, `prose_check.py` and `source_provenance_check.py` pass.

## Limitations and handoff

- Not exercised: a GPU, tier B, the viewer half (not written, so AC11 was checked through the library on AC3's inputs, not through a Part 10 file and the route), the oracle, real bytes. No mutation was run, because no code changed since pass 7's mutation table.
- Next owner: the lead corrects item 9's two-value bullet (D1), prose only, and a fresh independent pass 12 reviews it. This is a bounded review of the library half and the plan's statements of library values, not acceptance of F-227.
- The tree is unchanged by this review apart from this report. `git write-tree` `7bb9eea2` and the three sha1s were re-checked at the end. The probe lived under the worktree's ignored `target/` and was deleted with its build outputs.
