# F-207 polish review, pass 4

**Reviewed**: working tree on `sprint/s46`, HEAD `d48c46853660672230d4540e07f8865742e27856`, unstaged paths only. The `.claude/plans/*-design.md` drafts are out of scope. Blobs pinned at the start of the pass and re-hashed at the end.

| Path | Blob at start |
|------|---------------|
| `.agents/skills/complete-feature/SKILL.md` | `54e14af079bd41d794599f725b451337b2eb732d` |
| `.agents/skills/design/SKILL.md` | `1e1bef2bff94826ca1df600865afc16297f364ef` |
| `.agents/skills/implement-feature/SKILL.md` | `c1f905bf0890537289a3f6bebf546c15922eb718` |
| `.agents/skills/microscope/SKILL.md` | `c326d2936da7501a80a11b55da5260a1245daf50` |
| `.claude/commands/complete-feature.md` | `30d55f2ca09b5251b415fa59b97704f53ebb062a` |
| `.claude/commands/design.md` | `17173167e95f66b0284e7de33ac1cf745b55bcaa` |
| `.claude/commands/implement-feature.md` | `9b4e4e919c2606c9afece0b616be69a97af182f4` |
| `.claude/commands/microscope.md` | `618e9836b324cd8a94942ef455f48b620eff0967` |
| `.opencode/commands/complete-feature.md` | `eec33cb0370ccda0326a00a03039af2f58857110` |
| `.opencode/commands/design.md` | `7b94e3ec6ba57d7fc2ff83b19157f5e82d2bdf28` |
| `.opencode/commands/implement-feature.md` | `5706959363300ebe0b03b9585dc238e48edb64a8` |
| `.opencode/commands/microscope.md` | `8e46e43c94f0f4bba9274cdaeb8a4ce93b421344` |
| `README.md` | `553a5ee06afd92f389ee01514a90b77b8aac7b64` |
| `docs/sprints/AS_BUILT.md` | `55b52859250f65c54ec20d3308ddf74a602b8282` |
| `docs/sprints/BACKLOG.md` | `e03b8b3c3be2568b6e62604157e249b8360d0bb9` |
| `docs/sprints/CURRENT_SPRINT.md` | `712e9c54d227dec402b7c744315180eda4aacb2f` |
| `docs/sprints/SPRINT_PLAN.md` | `aa626d5fb5f635f625ba49567bf36ee9f1b9b3e9` |
| `docs/sprints/allocation.json` | `3000937b17eb5f138859053e7ec919e01a13c5f4` |
| `examples/folder-viewer/src/more.html` | `6d72fcc98f8defe9f89d0315500682e7fb310862` |
| `examples/folder-viewer/src/page.html` | `fc7d8b9652157cdd1a11a7b36a29fb5ae7d8e144` |
| `examples/folder-viewer/src/serve.rs` | `553f5a09c68c6676bdc8f80dc950fa4a1ed5bdf1` |
| `examples/folder-viewer/src/use.html` | `1d4c0a280264add4bde8f0a6ef46c35d00e31a1e` |
| `examples/folder-viewer/tests/preview.rs` | `acf0bfc2188ebb878914edc611fa1dd8f1019fa8` |
| `scripts/gen_sprint_plan.py` | `697d1028ad33074643d9a8f06946398ceb4f1daa` |
| `scripts/tests/test_gen_sprint_plan.py` | `973ce0522691017aa6ec4f174a79c6da3ab77b80` |

**Changed during the pass, by another participant, and not reviewed here**:

| Path | Blob at end |
|------|-------------|
| `README.md` | `6bdbde001036e7c1ecd032ba9a694bd3c5dc53b1` |
| `examples/folder-viewer/src/more.html` | `c8aceba3f0c3513a8eb09f632c9defd6173d69a9` |
| `examples/folder-viewer/src/page.html` | `2efbf346ef17ff0a137a3ff646e3b739348cf8d2` |
| `examples/folder-viewer/src/use.html` | `71c26d4dac0b64e888a258788dfd61215c97dea9` |
| `docs/ocelli-slice-svg/` | new, untracked |

The change is a logo. There is a `<picture>` at the top of the README, plus a favicon, a `.brand` style and an inline SVG wordmark linked to `/` in all three pages. Every other pinned blob was the same at the end. All findings below cite the **pinned** blobs and their line numbers. Line numbers in the end blobs are shifted by 5 (page) and 7 (README).

The viewer at `http://127.0.0.1:8742/` was not touched. Throwaway copies of the built binary ran only with `OCELLI_PORT=0`, `" 0 "` or values that never bind, and were stopped by PID.

**Author / reviewer**: the reviewed material was written by other participants. The reviewer is a Claude subagent that wrote none of it and did not author passes 1 to 3. No subagents were used. The only file edited is this report.

**Result**: 4 defects, 3 smells, 5 nitpicks

Pass line: F-207 polish, pass 4, 4 defects, 3 smells, 5 nitpicks.

## Acceptance challenged

Pass-3 findings, each replayed with one adjacent case.

- **D1, step 10 claims a review that cannot happen.** The edits now happen in `/implement-feature` section 7 (`implement-feature.md:188-199`), before `## Finish` hands to `/microscope`. `complete-feature.md:83-100` edits nothing. It confirms, fills the AS_BUILT field, and sends any late edit back through section 7 and a fresh pass. `microscope.md:81-86` names the three documents. Adjacent case: the template that feeds both steps, `design.md:210-214`, still says the documents are what "`/complete-feature` step 10 will update". See D1 below. **The ordering defect is closed. The template sentence left behind is a new defect.**
- **S3, AS_BUILT field.** `AS_BUILT.md:28-29` adds `**User-facing documentation.**` after `**LLD updated.**`, and `:8` now says step 1. `scripts/backlog_check.py:115-123` is the only script that reads AS_BUILT, and it checks only that a `done` F-ID has an entry. No checker enforces a field list, so the new field cannot break one, and nothing enforces it either. `backlog_check.py` exits 0. **Closed.**
- **S2, milestone placement and disappearance.** Disappearance is fixed. `render` (`gen_sprint_plan.py:132-146`) and `rendered_milestones` (`:216-230`) now walk every milestone in each sprint through `sprint_milestones`. HEAD's generator, run against the new test file in a scratch copy, is red on both new tests. One is `AttributeError ... milestone_summary`. The other is `[('S1','S2',3,9)] != [('S1','S2',2,5),('S1','S1',1,4)]`, which matches the author's figure. `render(load())` from HEAD and from the working tree were diffed in the scratchpad. **Only the two summary lines differ**, M20 `S45 to S45, 2, 13` becomes `S45 to S46, 12, 51` and M14 `16, 87` becomes `6, 49`. No live milestone was hidden before, and HEAD's allocation renders byte-identically under both generators. Placement is handled by a hand note instead of the generator, which suits a hand-curated file, but the note's reason is wrong (D2). Adjacent cases found two new gaps, S2 and S3.
- **N1, `OCELLI_PORT`.** The behaviour was observed on a throwaway binary. `""`, `" "`, `abc`, `70000` and `-1` all exit 1 with `OCELLI_PORT is not a port number`. `" 0 "` and `0` bind an ephemeral port. README `:179-182` and `serve.rs:64-74` now say that. **Closed**, with N1 below.
- **N2, size-budget sentence.** No longer contradicts "no size check". The replacement gets the order wrong, see D4.
- **S1, the single-view aspect, carried.** `page.html` and `preview.rs` hold exactly pass 3's blobs (`fc7d8b9`, `acf0bfc`), so pass 3's receipt applies to identical content. The consumer swap at `page.html:1706-1707` passed the suite with 31 green. Still open, see S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `python3 scripts/sync_agent_skills.py --check` | pinned tree | now | 0, 42 Codex and 19 OpenCode adapters match 21 sources |
| `python3 -m unittest scripts.tests.test_gen_sprint_plan` | pinned tree | now | 0, 6 tests OK |
| same tests against `git show HEAD:scripts/gen_sprint_plan.py` (`review4/head`) | HEAD generator, working-tree tests and data | now | red: 1 error (`milestone_summary` absent), 2 failures (the never-lead list, and `check` against the working-tree plan) |
| `python3 scripts/gen_sprint_plan.py --check` | pinned tree | now | 0, 203 F-IDs, 20 milestone lines, 76 goal lines |
| `python3 scripts/backlog_check.py` | pinned tree | now | 0, `240 F-IDs, 152 done` |
| `python3 scripts/prose_check.py` | pinned tree | now | 0, 794 files |
| `python3 scripts/deviation_check.py` | pinned tree | now | 0, 27 deviations |
| `cargo test -q -p ocelli-folder-viewer` | tree at compile time, see Limitations | now | 0, 31 passed |
| `cargo clippy -q -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | 0 |
| `cargo fmt --all --check` | same | now | 0 |
| `review4/renderdiff.py`, `render(load())` under HEAD and working-tree generators | live allocation, both generators | now | 2 differing lines, as quoted above. HEAD allocation renders identically |
| `review4/collide.py`, two milestones whose only stories share sprint S2 | working-tree generator | now | S2: own render fails `--check`. With equal numbers, a deleted line passes |
| `render` heading loop mutated to `sprint_milestones(stories)[:1]` in `review4/mut` | working-tree generator plus one mutation | now | unittest 6 OK, live `--check` OK. A rendered never-lead fixture fails `check`. See S3 |
| lead-story probe, M1 or M2 first in shared S2 | working-tree generator | now | headings identical either way. See D2 |
| throwaway binary, `OCELLI_PORT` in `""`, `" "`, `abc`, `70000`, `-1`, `" 0 "`, `0` | `target/debug/ocelli-folder-viewer` built from the tree | now | as quoted under N1 |
| aspect consumer mutation | `page.html` `fc7d8b9`, `preview.rs` `acf0bfc` | pass-3 receipt, identical blobs | 31 passed with `aspectX = aspect.y; aspectY = aspect.x;` |
| `sed -n 192,202p bin/ocelli.sh` | HEAD script | now | `gate wasm` runs `decode_graph_check.py` first, then the two packs, then `pin_and_size_check.py --with-size` |

## Defects

### D1, the plan template still names `/complete-feature` step 10 as the place the documents are updated
**Where**: `.claude/commands/design.md:211-214`
**What**: "The places in README.md, ... more.html that /complete-feature step 10 will update". After this remediation step 10 "edits none of them" (`complete-feature.md:89`), and `/implement-feature` section 7 does the update.
**Why it is wrong**: it is a false sentence in normative guidance, and it is the sentence every new plan copies. A designer or implementer reading the plan section will defer the edits to completion, and that is the exact pass-3 D1 ordering this remediation removed.
**Evidence**: read at the cited lines. `grep -n "step 10 will" .claude/commands/design.md` gives `:213`. The Codex and OpenCode adapters carry only the source hash, so the fix is one source edit plus `sync_agent_skills.py`.
**Scope and regression**: every plan drafted from S46 on. Smallest fix: "...that `/implement-feature` section 7 will update before review, or "no change" with the reason."

### D2, the M20 note gives the wrong reason for where the rows sit
**Where**: `docs/sprints/SPRINT_PLAN.md:655-656`
**What**: "Their rows are in the S46 table under M14, because F-119 leads that sprint." Under the new generator the lead story does not decide this. Each milestone's heading is written once, before its first sprint. S46 is M14's first sprint and M20's second, so `### M14` precedes the S46 table whichever story leads.
**Why it is wrong**: it is a false causal claim (`microscope.md` section 3). It describes HEAD's first-story walk, which this same change removed. A reader who re-sorts S46 to "fix" the placement will see nothing move.
**Evidence**: lead-story probe. With M1 or M2 first in a shared S2, the heading sequence is `['#### Sprint S1', '### M2, M2', '#### Sprint S2']` both times.
**Scope and regression**: prose only. Smallest fix: "..., because the plan writes each milestone heading once, before its first sprint, and S46 is M14's first."

### D3, the new test's comment misstates what HEAD computed
**Where**: `scripts/tests/test_gen_sprint_plan.py:93-95`
**What**: "gave M1 one story over S1 and M2 two stories". On that fixture HEAD returns `[('S1','S1',1,3),('S2','S3',3,14)]`, so M2 got **three** stories and 14 weeks, which is the drift the test exists to show.
**Why it is wrong**: a false factual sentence in a doc comment (`microscope.md` section 3). Pass 3 recorded the same tuple.
**Evidence**: HEAD generator in `review4/head`, `rendered_milestones` on the fixture.
**Scope and regression**: comment only. Smallest fix: "M2 three stories and 14 weeks".

### D4, `use.html` puts the decode-split check after the build
**Where**: `examples/folder-viewer/src/use.html:259` (pinned blob)
**What**: "`bin/ocelli.sh gate wasm` builds it and then checks the decode split and the recorded size budget." `bin/ocelli.sh:195-200` runs `decode_graph_check.py` **first**, which is a dependency-graph check that needs no build. Then come both packs (`wasm` and `wasm-decode`), and then `pin_and_size_check.py --with-size`. The script's own comment says the graph check is first "so a decode build that still links Naga fails before either release link".
**Why it is wrong**: `microscope.md:81-86`, added by this remediation, makes a false sentence in `use.html` a defect. The order is load-bearing by the script's own account.
**Evidence**: `sed -n 192,202p bin/ocelli.sh`, and `scripts/decode_graph_check.py:1-5`.
**Scope and regression**: the Use book only. The comment in the code block (`:261`) is fine. Smallest fix: "checks the decode split, builds both packs and then checks the recorded size budget."

## Smells

### S1, carried from passes 1, 2 and 3: the single-view aspect can be swapped with the suite green
**Where**: `examples/folder-viewer/src/page.html:1706-1707` (pinned, `1711-1712` in the end blob), against `examples/folder-viewer/tests/preview.rs:217-233`
**Mechanism**: unchanged from pass 3. `aspectX = aspect.y; aspectY = aspect.x;` passes all 31 tests and stretches the single view by the row spacing.
**This is the fourth pass to carry it.** Per `microscope.md:25-26` the operator should already have been told.
**On carrying it into F-208.** That is acceptable as a disposition, but only once three things are recorded. Until they are, this remains an open smell and blocks a clean pass. `microscope.md` has no "carry" exit for a blocking finding ("Zero defects **and** zero smells to pass"). `WORKFLOW.md:53-57` carries **stories**, not findings, and `complete-feature.md:25-29` allows a scope change only "through the normal approved design or replanning process". What must exist:
1. **An operator decision**, recorded here or in the polish handoff, that names this finding (F-207 polish S1), says it moves to F-208, and says the polish lands without it. The author's intention does not count, because the author cannot waive their own blocking finding (`WORKFLOW.md:73-75`).
2. **A numbered acceptance row in F-208's approved plan.** It measures, in a real browser on the synthetic study, the width-to-height ratio of the single view and the first tile against 64/67.2 within 1e-3, for every viewport F-208 creates, and it names the swap above as the mutation that must turn it red. `grep -i aspect .claude/plans/F-208-design.md` finds only the state-model lines (`:56`, `:73`, `:89`) and **no such row today**. F-208 moves `aspectX` and `aspectY` into a per-viewport record, which is where the swap site will be rewritten, so the row belongs there.
3. **A line under the sprint's carried or open items in `CURRENT_SPRINT.md`**, so `close-preflight` and the sprint review see it. Nothing in `CURRENT_SPRINT.md` or `BACKLOG.md` mentions it now.

Also, no completion text for the polish may say the aspect is guarded by the suite.

### S2, `--check` keys summary lines by span, and spans can now collide
**Where**: `scripts/gen_sprint_plan.py:346-369`
**Mechanism**: `found_by_span` is a dict keyed by `(first, last)`. Under HEAD's first-story walk, distinct milestones had disjoint sprint sets and so distinct spans. Now that each story counts under its own milestone, two milestones can share a span, for example two milestones whose only stories sit in S2. Then:
- **False refusal.** With M2 at 4w and M3 at 5w, both `S2 to S2`, `check` fails on the generator's own unedited `render` output: "should read 1 stories and 4 engineer-weeks, and SPRINT_PLAN.md says ... 5".
- **Missed deletion.** With equal numbers, deleting one of the two lines leaves `check` at exit 0. It still prints "3 milestone summary line(s) ... match", though the file holds two. The "one summary line per milestone" assertion pass 3 asked for therefore does not hold in this case.

The live allocation has no shared span (S45 to S46 and S46 to S49 differ), so this is latent.
**Evidence**: `review4/collide.py`.
**Smallest fix**: key each found line by the `### Mx` heading above it, or compare `Counter(actual_lines)` with `Counter(expected_lines)`. Add the shared-S2 fixture as a test that `check(data, render(data))` returns 0 and a deleted line returns 1.

### S3, the `render` half of the fix is untested, and the docstring claims one walk where there are two
**Where**: `scripts/gen_sprint_plan.py:132-146` against `:216-230`, and the docstring at `:219-220`
**Mechanism**: `rendered_milestones` says it is "Built by the same walk `render` uses". They are two separate loops that both call `sprint_milestones`. Both new tests exercise only `rendered_milestones`. Reverting `render` alone to the first-story behaviour (`sprint_milestones(stories)[:1]`) leaves all 6 unit tests and the live `--check` green. Only a never-lead fixture rendered and then checked goes red, and no test does that. A later edit to either loop can drift from the other without anything noticing, which is the pass-2 D11 failure in a new place.
**Evidence**: `review4/mut` mutation run.
**Smallest fix**: have `render` iterate `rendered_milestones`' key order rather than repeating the loop, or add a test that renders the never-lead fixture and asserts `### M2` is present and `check` returns 0. Then reword the docstring to match.

## Nitpicks

- N1. `README.md:179-182` says "chosen at startup unless `OCELLI_PORT` is set". `OCELLI_PORT=0` is set and still gives an ephemeral port. A non-UTF-8 value reaches `Err(_) => Ok(0)` at `serve.rs:99`, so it is read as unset, although the next sentence says a non-port value stops the process. `:182` is 87 columns against the file's wrapping. `chosen_port` still has no unit test (carried from pass 2).
- N2. Parallel workers now edit `README.md`, `use.html` and `more.html` inside `/implement-feature`. `run-sprint.md:98-105` lists exclusive resources. `use.html` and `more.html` fall under "the same source or test file", but `README.md` is not listed. Two workers in one wave can edit it. Each `/microscope` sees only its own worktree's README, and `/integrate-feature` either auto-merges the union unseen or stops at a conflict. The whole-sprint review is the only backstop. One bullet, "`README.md`, `use.html`, `more.html` when both stories change user-facing behaviour", would make the wave planner serialise them. `integrate-feature.md:33-44` needs no change, because a prose conflict is already a semantic conflict that gets re-reviewed.
- N3. `WORKFLOW.md:123` still describes step 3 as "/implement-feature  Code + tests". It could read "Code, tests and user-facing docs", so that the rhythm agrees with section 7.
- N4. Step 1 (`complete-feature.md:33-36`) writes the AS_BUILT entry "in the documented format", which now contains the field, and step 10 item 2 says "Fill" it again. That is harmless, but step 10 could say "Confirm step 1 filled".
- N5. The M20 note is hand prose that `render` does not write, so `--force` would drop it and `--check` does not see it. That is consistent with "Prose drift is a human's business" (`gen_sprint_plan.py:8-9`). Recorded so that nobody mistakes it for generated text.

## Verified clean

- The three command files agree on order. Design writes the impact section, implement section 7 edits, microscope section 3 reviews, complete step 10 confirms and refuses late edits. D1 is the exception. `run-sprint.md:123-139`, in both serial and parallel waves, puts `/implement-feature` before `/microscope`, so the new order holds in both modes. `WORKFLOW.md:105-108` staging text and complete step 7 no longer need to name document edits, because none happen at completion. That closes pass-3 N4's first half. `AS_BUILT.md:8` now says step 1, which closes the second half.
- `--prepare` commits the worker branch's reviewed diff, now including the document edits, and the handoff's `Files touched` will list them. N2 covers the residual.
- Pass-3 N3 (a missing plan section) is closed by `implement-feature.md:195-196`.
- No script parses AS_BUILT fields. `backlog_check.py` exits 0.
- The generator fix closes milestone disappearance. HEAD is red on both new tests, live render output changes only in the two summary lines, and HEAD's allocation is unchanged. `--check` asserts one line per milestone except under S2.
- The `SPRINT_PLAN.md` M20 and M14 summaries were recounted by `milestone_summary` on the live allocation: `S45 to S46, 12, 51` and `S46 to S49, 6, 49`.
- `OCELLI_PORT` behaviour for seven values was observed and matches README and `serve.rs` except as noted in N1.
- Adapters: `sync_agent_skills.py --check` passes. The four changed adapter pairs differ from HEAD only in their source hash line.

## Limitations and handoff

- Four pinned files changed during the pass (the logo work listed above). That material is **unreviewed** and needs its own pass. The `cargo test` and `clippy` runs compiled whichever `page.html`, `use.html` and `more.html` were on disk at that moment. They began before the first observed change, but the reviewer cannot prove which blobs they compiled. Re-run both on the final tree.
- S1 was not re-mutated. Pass 3's receipt is cited because `page.html` and `preview.rs` matched its pins byte for byte at the start of this pass.
- No browser was run this pass, and the `/use` and `/more` snippets were not compiled. Pass-3 N6 and pass-2 N7 were not re-assessed.
- `.claude/plans/F-208-design.md` was read with one grep, only to check whether the S1 carry is recorded. It was not reviewed.
- Next: the author fixes D1 to D4 and S2 to S3. The operator either records the S1 disposition (the three items under S1) or the author lands the browser check now. Then a fresh independent pass 5 reviews the changed material together with the logo change.
