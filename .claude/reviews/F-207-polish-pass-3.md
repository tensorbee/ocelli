# F-207 polish review, pass 3

**Reviewed**: working tree on `sprint/s46`, HEAD `d48c46853660672230d4540e07f8865742e27856`, unstaged paths only. The `.claude/plans/*-design.md` drafts are out of scope. Blobs pinned at the start of the pass and re-hashed at the end. **Nothing changed during the pass.**

| Path | Blob |
|------|------|
| `.agents/skills/complete-feature/SKILL.md` | `959d50ded74e1cf712ed43c99302bcd4229ec1e4` |
| `.agents/skills/design/SKILL.md` | `1e1bef2bff94826ca1df600865afc16297f364ef` |
| `.claude/commands/complete-feature.md` | `91958bc46e111f5b803ef1a2e43c5381ad7874e5` |
| `.claude/commands/design.md` | `17173167e95f66b0284e7de33ac1cf745b55bcaa` |
| `.opencode/commands/complete-feature.md` | `94f3ab37b9faa35a2759eb786d5052ac2991b903` |
| `.opencode/commands/design.md` | `7b94e3ec6ba57d7fc2ff83b19157f5e82d2bdf28` |
| `README.md` | `fe221a5ad8b8404db9f104092a3e42101087d3fd` |
| `docs/sprints/BACKLOG.md` | `e03b8b3c3be2568b6e62604157e249b8360d0bb9` |
| `docs/sprints/CURRENT_SPRINT.md` | `712e9c54d227dec402b7c744315180eda4aacb2f` |
| `docs/sprints/SPRINT_PLAN.md` | `4c4853f95412210e449f0f39a9cb73ce7cec9896` |
| `docs/sprints/allocation.json` | `3000937b17eb5f138859053e7ec919e01a13c5f4` |
| `examples/folder-viewer/src/more.html` | `6d72fcc98f8defe9f89d0315500682e7fb310862` |
| `examples/folder-viewer/src/page.html` | `fc7d8b9652157cdd1a11a7b36a29fb5ae7d8e144` |
| `examples/folder-viewer/src/serve.rs` | `a8cf6884c24b0dfd03114ebb538b865b650c5d8e` |
| `examples/folder-viewer/src/use.html` | `98f6fa07a557da1f77b37cbd04034883e4e56e30` |
| `examples/folder-viewer/tests/preview.rs` | `acf0bfc2188ebb878914edc611fa1dd8f1019fa8` |
| `scripts/gen_sprint_plan.py` | `845e7b7a9130d5de3e03a4936afd8f5e2c57dc79` |
| `scripts/tests/test_gen_sprint_plan.py` | `876cb605ffd483a9aa22f7e319b4889108f7d017` |

The viewer at `http://127.0.0.1:8742/` was not touched. A throwaway build of the pinned tree, run with `OCELLI_PORT=0` and stopped by PID, served `/`, `/more` and `/use` with exactly the pinned page, more and use blobs.

**Author / reviewer**: the reviewed material was written by other participants. The reviewer is a Claude subagent that wrote none of it and did not author pass 1 or pass 2. No subagents were used. The only file edited is this report.

**Result**: 1 defect, 3 smells, 6 nitpicks

## Acceptance challenged

Pass-2 findings, each replayed with one adjacent case.

- **D1, en-dash.** In 2x2 the lower-left corner read `Images 1 to 4 / 12`. Adjacent: ArrowDown through the series gave `Images 9 to 12 / 12`, `Images 10 to 12 / 12`, `Images 11 to 12 / 12` and then `Image 12 / 12`. No U+2013 or U+2014 in any of the three pages' rendered `innerText`, nor in any source file in the diff. **Closed.**
- **D2, MPR example.** `more.html:242` is `Volume::construct(candidate)?`, whose doc says "The returned volume is complete" (`crates/ocelli-volume/src/lib.rs:584-595`). The `Absent` comment is now "a slice not landed or outside, never stored zero", which matches `ortho.rs:64` and the out-of-range returns at `ortho.rs:214-244`. **Closed.**
- **D3, build commands.** `use.html:260-263`. `bin/ocelli.sh wasm` is "release wasm-pack build, no size check", which matches `bin/ocelli.sh:504-513`. `gate wasm` runs `decode_graph_check`, both packs and `pin_and_size_check --with-size` (`:195-200`). `npm run build` is the root `--workspaces --if-present` script, with `tsc --build` in both packages and `vite build` in `examples/viewer-react`. **Closed**, with N2 on the sentence above the block.
- **D4, compare produce.** `CpuRaster` is used in `tools/oracle/src/candidate.rs` and `produce_current.rs`, and `reslice_nearest` in `produce.rs` and `produce_current.rs`. **Closed.**
- **D5, drawing.** "the constant `"unavailable"` today". **Closed.**
- **D6, attachSideBySide.** "pairs ... and enables the seam". **Closed.**
- **D7, overlayGrid.** Now names `intersection.ts`. `crosshairLines` and `referenceCursor` are in `crosshair.ts:38,57`, `referenceLines` is an alias at `crosshair.ts:54`, and `overlayGrid` is at `intersection.ts:138`. **Closed.**
- **D8, kind object.** "a `kind` object whose `type` is". **Closed.**
- **D9, panic exports.** `install_panic_hook`, `panic_record_ptr` and `panic_record_len` carry `#[wasm_bindgen]` in `crates/ocelli-wasm/src/lib.rs`. **Closed.**
- **D10, changelog.** "says". **Closed.**
- **D11, milestone summaries.** The counts are fixed. M20 now reads `S45 to S46, 12 stories, 51 engineer-weeks` and M14 reads `S46 to S49, 6 stories, 49 engineer-weeks`. Recounted by each story's own field: M20 is S45 2 plus S46 10 (F-208 to F-217) with 13 + 38 = 51w, and M14 is F-119 plus five later stories. HEAD's generator run on the new test's data gives `[('S1','S1',1,3),('S2','S3',3,14)]`, which matches the author's figure. The new test run against HEAD's generator is red with `AttributeError: module 'gen_sprint_plan' has no attribute 'milestone_summary'`. **Counts closed. The placement half of D11 is still open, see S2.**
- **D12, F-216 note.** "Every window drag step, and every slice not in the page cache, posts the whole file again". This matches pass 2's observation of the page cache. **Closed.**
- **D13, index columns.** `.index` is `grid-template-columns: 1fr`. Headless Chromium at 1400 px counts 1 column. **Closed.**
- **S1, aspect pins.** The author's tile pin does catch its mutation. **The single-view swap that pass 2 named still passes the suite.** See S1. This is the third pass to carry the finding.

New material.

- **README folder-viewer section.** Step 5 was checked against `page.html:561-611`: the groups and their order, the `[` and `]` toggles at both ends of the top bar, and Scroll as the initially active tool. Step 7 was checked against `personName` and `dicomDate`. Step 3's 40 px wheel threshold is at `page.html:2042`. Step 6's button mapping is at `:2103-2104`. Step 4's FPS range 1 to 30 starting at 10 is at `:651`. The right column is WINDOW, MARKS, MOUSE and KEYS (`:657-710`). The `OCELLI_PORT` paragraph was run, see N1. No other stale sentence was found in lines 169-242.
- **Workflow guidance.** See D1 and S3.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo test -q -p ocelli-folder-viewer` | pinned tree | now | 0, 31 passed |
| `cargo clippy -q -p ocelli-folder-viewer --all-targets -- -D warnings` | pinned tree | now | 0 |
| `cargo fmt --all --check` | pinned tree | now | 0 |
| `python3 -m unittest scripts.tests.test_gen_sprint_plan` | pinned tree | now | 0, OK |
| same test file against `git show HEAD:scripts/gen_sprint_plan.py` in the scratchpad | HEAD generator | now | red, `AttributeError ... milestone_summary` (two other errors come from the scratch copy having no allocation file) |
| `python3 scripts/backlog_check.py` | pinned tree | now | 0, `OK: 240 F-IDs, 152 done` |
| `python3 scripts/gen_sprint_plan.py --check` | pinned tree | now | 0, `OK: 203 planned F-IDs ... 20 milestone summary line(s)` |
| `python3 scripts/prose_check.py` | pinned tree | now | 0, 794 files. It does not scan HTML |
| `python3 scripts/source_provenance_check.py` | pinned tree | now | 0, 1605 files |
| `python3 scripts/sync_agent_skills.py --check` | pinned tree | now | 0, 42 Codex and 19 OpenCode adapters match 21 sources. Both new SHA-256 lines match their canonical sources |
| `python3 scripts/deviation_check.py` | pinned tree | now | 0, 27 deviations |
| `review3/prose.py`, an HTML parser over visible text outside `pre`, `code`, `style` and `script`, plus `title`, `aria-label`, `alt` and `placeholder` | pinned more, use and page | now | 0 em-dash, 0 en-dash, 0 semicolons. `<canvas` appears 0 times in more and use and 6 times in page, which is expected |
| mutation runs in a scratchpad copy of the workspace (`review3/ws`, separate target dir) | pinned tree plus one mutation each, reverted and re-hashed | now | see S1 |
| throwaway binary with `OCELLI_PORT` set to `0`, `""`, `" "` and `8742`, and unset | pinned `serve.rs` | now | `0` and unset give an ephemeral `http://127.0.0.1:<port>/`. `""` and `" "` give `OCELLI_PORT is not a port number`. `8742` gives `Address already in use (os error 48)` |
| `review3/p3.js` and `p3b.js`, Playwright Chromium headless at 1400x900, DPR 1, against the throwaway server | pinned page | now | view ratio 0.95237 against 64/67.2 = 0.95238. Corner sequence as quoted. No page errors |
| `review3/headwalk.py` | pinned and HEAD generators | now | see S2 |

## Defects

### D1, step 10 claims a review that the workflow order makes impossible
**Where**: `.claude/commands/complete-feature.md:104-106`, against precondition 1 at `:14` and `.claude/WORKFLOW.md:118-127`
**What**: "The microscope pass that clears the story reviews these three documents against the tree as part of its prose claims." Precondition 1 requires that clean pass to exist **before** `/complete-feature` runs, and step 10 writes the README, `use.html` and `more.html` edits **inside** `/complete-feature`, after that pass. So the pass that cleared the story cannot have seen them. Nothing after step 10 reviews them: step 7 re-verifies and step 8 commits. `.claude/commands/microscope.md` is what a reviewer reads, and its section 3 (`:70-79`) names plans, reviews, AS_BUILT entries and doc comments, but not these three documents.
**Why it is wrong**: it is a false sentence in normative guidance. `microscope.md:15-17` says "Every remediation is itself unreviewed work until a pass has seen it". As written, user-facing prose edits are committed with no reviewer, while the rule text says a reviewer checked them.
**Evidence**: read at the cited lines. `git grep -n "use.html\|more.html" .claude/commands .claude/WORKFLOW.md` finds them only in `complete-feature.md` and `design.md`.
**Scope and regression**: affects every story from S46 on. None has closed under the new text yet. Smallest fix: do the documentation update in `/implement-feature`, before `/microscope`, and leave step 10 as a check that it was done. Or require a fresh `/microscope` pass over the step-10 edits before step 7. In either case, add the three documents to `microscope.md` section 3.

## Smells

### S1, carried from pass 1 and pass 2: the single-view aspect can be swapped with the suite green
**Where**: `examples/folder-viewer/tests/preview.rs:217-233`, against `examples/folder-viewer/src/page.html:1705-1707`
**Mechanism**: the three new pins guard `pictureW` and `pictureH` (`:1820-1821`) and the tile ratio (`:1774`). They do not guard the assignment `aspectX = aspect.x; aspectY = aspect.y;` at `:1706-1707`, which is the exact consumer swap pass 2 named. Swapping it stretches the single view's x by the row spacing. The image then shows at 1.4 times the true width over height for `0.7\0.5`, and marks follow the wrong geometry.
**Evidence**: in a scratchpad copy of the workspace:
- Baseline: `the_page_offers_a_folder_picker` passes.
- Tile mutation, `String(aspect.y / aspect.x)` to `String(aspect.x / aspect.y)`: **red**, `the display aspect is not applied where it is used: node.dataset.aspect = String(aspect.y / aspect.x);`. The author's claim holds for this pin.
- Consumer mutation, `aspectX = aspect.y; aspectY = aspect.x;`: **31 passed, green**. This is the residual gap. Substring pins cannot close it, because every consumer of `aspectX` (`:1806`, `:2294`, `:2304`, `:2364`, `:2564`, `:2599`) would need its own pin.
- The copy was restored and re-hashed afterwards.

**This finding has now survived three passes.** Per `microscope.md:25-26`, the operator should be told.
**Smallest fix**: the behavioural check pass 1 and pass 2 asked for. Open the synthetic study in headless Chromium and assert that the `#view` and first `#tiles canvas` width-to-height ratio is 64/67.2 within 1e-3. `review3/p3.js` measures this today (0.95237). If a browser test is ruled out of this crate's suite, the operator should record that waiver explicitly rather than adding more pins.

### S2, the residual of pass-2 D11: S46's M20 stories still sit under the M14 heading, and a milestone that never leads a sprint vanishes
**Where**: `scripts/gen_sprint_plan.py:134-145` (`render`) and `:225-229` (`rendered_milestones`), and `docs/sprints/SPRINT_PLAN.md:649-688`
**Mechanism**: a heading is still emitted only when a sprint's **first** story changes milestone. `milestone_summary` fixes the numbers but not where they are placed:
- **Placement.** `### M20` (`:649`) says `S45 to S46, 12 stories` but holds only the S45 table. Its ten S46 stories, F-208 to F-217, are in the S46 table under `### M14, Out-of-core volume streaming` (`:664-686`), whose line says 6 stories above 16 rows. Pass 2's D11 "What" named exactly this placement.
- **Disappearance.** A milestone whose stories never lead a sprint gets no heading and no summary line, and `--check` stays green, because `rendered_milestones` uses the same first-story walk. `headwalk.py` with `S1: M1, M2(5w)` and `S2: M1` gives `[('S1','S2',2,5)]` on the new generator. M2 is simply absent. The docstring at `:239-241` ("adds each story to its own milestone") does not hold for that milestone.

No milestone in the live allocation is hidden today (`never lead: set()`), so this is latent.
**Smallest fix**: emit a heading or a one-line "shared with M20 (F-208 to F-217)" note for every milestone that has stories in a sprint. Make `--check` assert one summary line per milestone in `data["milestones"]` that has stories. Add the `headwalk.py` two-milestone fixture to `MilestoneSummary`.

### S3, step 10's AS_BUILT record has no field in the documented AS_BUILT format
**Where**: `.claude/commands/complete-feature.md:100-102`, against `:33-34` (step 1, "in the documented format") and `docs/sprints/AS_BUILT.md:10-30`
**Mechanism**: the entry format has `**LLD updated.**` but no field for user-facing documentation. Step 10 asks for `User-facing documentation: no change` in colon form, which is unlike the format's `**Field.**` style. It asks for a record only when nothing changed, so a story that did edit the documents records nothing about which places it edited. An author following step 1 literally writes no such line, and `scripts/backlog_check.py:115-123` checks only that an entry exists. The result will be AS_BUILT entries in two shapes, with the "an empty answer is not an answer" rule unenforceable, because there is no slot for the answer.
**Smallest fix**: add `**User-facing documentation.** {places updated in README.md, use.html and more.html, or "no change" with the reason}` to the AS_BUILT entry format, after `**LLD updated.**`, and make step 10 say "fills the `User-facing documentation` field". The format section is not a prior entry, so the append-only rule does not forbid this.

## Nitpicks

- N1. `README.md:179-182` and `serve.rs:66-68` say the port is chosen at startup "unless `OCELLI_PORT` names one". An empty or blank `OCELLI_PORT` names no port, yet it is refused with exit 1 instead of being treated as unset. This is carried from pass 2 N1, and the README now repeats it. Either treat a trimmed-empty value as unset in `chosen_port`, or add "and refuses a value that is not a port number" to both. `chosen_port` still has no unit test.
- N2. `use.html:259` still says the module is built "then measured against the recorded size budget", directly above `bin/ocelli.sh wasm  # ... no size check`. Pass 2 flagged this clause. "with `gate wasm` measuring it against the recorded size budget" would agree with the block below it.
- N3. Step 10 has no rule for a missing `## User-facing documentation impact` section, unlike step 9's `:80-81`. All ten S46 draft plans, F-119 and F-208 to F-216, lack it, since they predate the template. Step 10's "search all three documents" clause mitigates this.
- N4. Step 7 (`complete-feature.md:46`, "After all completion and LLD edits") and `WORKFLOW.md:105` ("After ledger, LLD, status or plan edits") do not name the documentation edits as part of the final staged state. Separately, and already on HEAD, `AS_BUILT.md:8` says it is written by step 2 when it is step 1.
- N5. The `--prepare` section says to commit "only feature-local changes". README, `use.html` and `more.html` are shared by every parallel worker, so step 10 under `--prepare` will conflict at integration, and nothing says who owns those edits.
- N6. `SPRINT_PLAN.md:672`, the generated S46 goal, joins eleven story titles that themselves contain commas and colons into one sentence. This is pre-existing generator behaviour, made visible by the S46 intake.

## Verified clean

- Every pass-2 defect, D1 to D10, D12 and D13, closed with its trigger and one adjacent case, as listed under Acceptance challenged. The D11 counts are closed.
- `milestone_summary` is used by both `render` and `rendered_milestones`. The new test is red against HEAD. The live summary lines were recounted independently from `allocation.json`.
- The `OCELLI_PORT` behaviour for `0`, unset, empty, blank and an occupied port was observed on a throwaway binary.
- README lines 169-242 were checked claim by claim against `page.html` and `serve.rs`.
- More and use visible text has no em-dash, en-dash, prose semicolon or `<canvas`. `/more`'s index is one column at 1400 px.
- The F-217 note claims were checked. `pyproject.toml:12` pins `pylibjpeg-libjpeg==2.4.0`. `package-lock.json` has `lightningcss` 1.33.0, MPL-2.0, under vite 8.2.2. `vendor/openjph-core-0.1.0` has no licence or notice file. No `deny.toml` or licence checker exists under `scripts`, `bin` or `ci`.
- The adapters' Source SHA-256 lines match. No other tracked file enumerates complete-feature's steps or the plan template's sections apart from those reported.

## Limitations and handoff

- Chromium headless at DPR 1 only. The DPR 2, Firefox and Safari paths were not run. The pass-1 and pass-2 browser scripts (D1 file change, D3 width sweep) were not re-run, because `page.html` differs from pass 2's pin only at line 1868.
- `CURRENT_SPRINT.md` and `BACKLOG.md` were diffed against pass 2's pins. Only the F-217 rows changed, and pass 2 N7 ("Grok Build CLI") was not re-assessed.
- No snippet on `/use` or `/more` was compiled.
- Next: the author fixes D1 and S3 in the workflow guidance, S2 in the generator, and S1 with a behavioural test or a recorded operator waiver. **S1 has survived three passes and should go to the operator now.** A fresh independent pass 4 then reviews the changed material.
