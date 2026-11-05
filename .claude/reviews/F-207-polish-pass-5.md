# F-207 polish review, pass 5

**Reviewed**: working tree on `sprint/s46`, HEAD `d48c46853660672230d4540e07f8865742e27856`, unstaged and untracked paths. The `.claude/plans/*-design.md` drafts are out of scope, except for one check of `.claude/plans/F-208-design.md` for the carried S1 row. Blobs were pinned at the start of the pass and re-hashed at the end.

| Path | Blob at start | Blob at end |
|------|---------------|-------------|
| `README.md` | `e715db97b000d1a39c006e0263d0c1a8e47e07e0` | same |
| `.claude/WORKFLOW.md` | `b2c04c918dc98d89c40e106caea80f5e8d99a4df` | same |
| `.claude/commands/design.md` | `f1cae0505f4a00c775856ee33dd156ddb06ca38d` | same |
| `.claude/commands/run-sprint.md` | `26a8d360ba2a29c3815b451168f513fa6e14af95` | same |
| `docs/sprints/SPRINT_PLAN.md` | `01aa10f9360fb4d23a3114647dc61a568d868721` | same |
| `docs/sprints/CURRENT_SPRINT.md` | `d706493c1ed1310d2aab82bee72d808a02e7f2e9` | `a696fbc1184a39a62e83d577b74a515cc2f5392b`, see below |
| `scripts/gen_sprint_plan.py` | `2b5ac0eaf2df1bcd356f11344c3a0c2d0aa04f43` | same |
| `scripts/tests/test_gen_sprint_plan.py` | `f0128144b94a0827e1650f4a5548fd9105c6b30d` | same |
| `examples/folder-viewer/src/page.html` | `2efbf346ef17ff0a137a3ff646e3b739348cf8d2` | same |
| `examples/folder-viewer/src/more.html` | `c8aceba3f0c3513a8eb09f632c9defd6173d69a9` | same |
| `examples/folder-viewer/src/use.html` | `6cdc7af5eefc9f0f47d825818b6f9636274738ca` | same |
| `examples/folder-viewer/src/serve.rs` | `553f5a09c68c6676bdc8f80dc950fa4a1ed5bdf1` | same |
| `examples/folder-viewer/tests/preview.rs` | `acf0bfc2188ebb878914edc611fa1dd8f1019fa8` | same |
| `bin/ocelli.sh` | `448a286054b2673eb34a7435db3ed7a006539ad8` | same |
| `.claude/plans/F-208-design.md` (checked only for AC14) | `0914d7c680bdfd58d9823e0bd3524d492e070179` | same |
| `docs/ocelli-slice-svg/` (8 files, untracked) | combined `shasum` of `shasum *` is `cbfa9eae4fb5e10fe840fc759b54150ec602e59e` at the end. No change was observed during the pass | |

**Changed during the pass, by another participant**: `docs/sprints/CURRENT_SPRINT.md`. The only change is at `:38`, where "the Grok Build CLI" now reads "an external coding agent". The carried-findings section reviewed below is byte-identical in both blobs. The changed sentence was not reviewed beyond that.

The viewer at `http://127.0.0.1:8742/` was driven read-only (one page load and one `evaluate`) and was not restarted. No throwaway server was started. The Playwright snapshot wrote `.playwright-mcp/page-2026-11-05T03-47-24-571Z.yml` into the repository root, a git-ignored path (`.gitignore:117`). The reviewer deleted that file and the directory it created. No other file outside this report was written.

**Author / reviewer**: the reviewed material was written by other participants, the lead and the operator (artwork). The reviewer is a fresh Claude subagent that authored none of it and none of passes 1 to 4. No subagents were used.

**Result**: 2 defects, 0 smells, 7 nitpicks. Carried and not counted: pass-4 S1, now AC14 of F-208.

Pass line: F-207 polish, pass 5, 2 defects, 0 smells, 7 nitpicks.

## Acceptance challenged

Pass-4 findings, each replayed with an adjacent case.

- **D1, design template.** `design.md:211-214` now says "`/implement-feature` section 7 will update before review". `implement-feature.md:188` is `## 7. User-facing documentation`, and it says the edits come before review. `sync_agent_skills.py --check` passes. **Closed.**
- **D2, M20 note.** The sentence changed, but the new reason is still false as a general rule. See D2.
- **D3, test comment.** `test_gen_sprint_plan.py:94-97` now says "M2 three stories and 14 weeks". That is HEAD's `('S2','S3',3,14)` on that fixture (F-002 5w, F-003 2w, F-004 7w). **Closed.**
- **D4, `use.html` gate wasm.** `use.html:262` now reads "checks the decode split, builds both packs and then checks the recorded size budget". `bin/ocelli.sh:195-200` runs `decode_graph_check.py`, then `wasm`, then `wasm-decode`, then `pin_and_size_check.py --with-size`. **Closed.**
- **S2, `--check` keyed by heading.** Replayed both pass-4 scenarios through `review5/probe.py` against the working-tree generator.
  - Shared S2 span with unequal numbers (4w and 5w). The generator's own render passes `check` with exit 0, so the false refusal is gone. Deleting M3's line returns 1 and names M3.
  - Equal numbers. Deleting either line returns 1 and names the right milestone, so the missed deletion is gone.
  - Adjacent cases, all refused with an actionable message. A duplicated line gives "M1 carries 2 summary lines". If a heading is deleted and its line kept, the line is attributed to the previous milestone, which then carries 2, and the deleted one is reported absent. An extra `### M9` with a line gives "the allocation plans no story for it". A summary line under a non-milestone `### Notes` heading is counted under the previous milestone and refused as a duplicate. A trailing space makes a line not match and is reported absent.
  - **Closed.** The duplicate and extra-key branches have no test, see N1.
- **S3, `render` derives headings from `rendered_milestones`.** `render` (`:132-144`) now iterates over `summaries = rendered_milestones(data)` and writes each heading at `first == sprint`. That is one derivation, and the docstring (`:216-222`) now says so truthfully. Mutation results:
  - Making `render` keep only each sprint's first-story milestone (`if first != sprint or key != stories[0]["milestone"]`) turns `test_render_writes_a_heading_for_every_milestone` red. Live `--check` stays green, as expected, because the live plan is hand-curated.
  - The heading sequence of `render(load())` is identical to the live `SPRINT_PLAN.md` (96 headings each).
  - **Closed.**
- **Nitpick follow-ups.**
  - README `:186-190` now says "`0` also means chosen at startup". That matches `serve.rs:91-101`, which parses the trimmed value as `u16`, and `0` binds an ephemeral port. Pass 4 observed this, and `serve.rs` is the same blob.
  - `run-sprint.md:105-106` adds the README exclusive-resource bullet, and its reference to section 7 resolves.
  - `WORKFLOW.md:123` now reads "Code, tests and the user-facing documents".
  - All three closed. The non-UTF-8 residue of pass-4 N1 remains, see N4.
- **S1 carry, three conditions.**
  1. **The F-208 acceptance row exists.** `.claude/plans/F-208-design.md:186` is AC14, and the plan is `**Status**: approved` (`:3`). It measures width over height `128*0.5 : 96*0.7 = 64 : 67.2 = 0.952381` within 1e-3 for the single view at 1 by 1, the four tiles and the four 2 by 2 cells. The evidence is headless Chromium reading `getBoundingClientRect()` in a committed script under a new `viewer-browser` gate. It names the `aspectX`/`aspectY` assignment swap as the mutation control and states the expected wrong ratio, 1.866667, which the reviewer recomputed as `89.6/48`. The arithmetic follows PS3.3 C.7.6.2.1.1: the first Pixel Spacing value is the row spacing (vertical) and the second is the column spacing (horizontal). AC14 is a superset of what pass 4 asked for. **Met.**
  2. **The CURRENT_SPRINT line exists.** `CURRENT_SPRINT.md:41-52`, `## Review findings carried into a story`, names the finding, the target story and the mutation, says it is not waived, and says F-208 does not complete while the row is open. Its factual claims were checked. The only aspect checks are the string pins at `preview.rs:217-233`. The swap site is `page.html:1711-1712`. **Met.** One limitation: `close-preflight` reads only `## Carried forward from SNN` (`sprint_workflow.py:99`), not this section, so the sprint review is the only reader. The line itself does not claim otherwise. See N7.
  3. **The operator decision, by delegation.** The operator said on 2026-11-05 "we cannot compromise the accuracy, quality and robustness so you make a call", and the lead made the call. **The reviewer's judgement is that this counts as an operator decision for the disposition, with one qualification.**
     - Neither `WORKFLOW.md` nor `microscope.md` forbids an operator from delegating a call, and the instruction was explicitly to make one.
     - The rule pass 4 cited is that an author cannot **waive** their own blocking finding (`WORKFLOW.md:73-75`, "The author does not supply their own independent approval"). The disposition does not waive anything. It moves the blocker into an approved acceptance row whose control is a mutation, and makes F-208's completion depend on it. That makes the carried check stricter than the original pin, not weaker.
     - The qualification is that the delegation is not a release from review. `microscope.md` still has no clean exit while a smell is open. What it does have is an "already-carried findings" category (`microscope.md` composition section: "Separate reproduced defects, plausible concerns, already-carried findings and external failures"). This pass therefore records S1 as **carried**, outside the counts, and not as closed.
     - The record paraphrases the operator ("to make the quality call") rather than quoting the words. See N7.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `python3 -m unittest scripts.tests.test_gen_sprint_plan` | pinned tree | now | 0, 9 tests OK |
| `python3 scripts/gen_sprint_plan.py --check` | pinned tree | now | 0, 203 F-IDs, 20 milestone lines, 76 goal lines |
| `review5/mutate.sh m1`, `render` kept to the first-story milestone | scratch copy of generator, tests and data | now | 1 failure, `test_render_writes_a_heading_for_every_milestone`. Live `--check` OK |
| `mutate.sh m2`, `len(lines) > 1` branch disabled | same | now | 9 OK, live OK. Untested branch, N1 |
| `mutate.sh m3`, extra-key loop disabled | same | now | 9 OK, live OK. Untested branch, N1 |
| `mutate.sh m4`, compare `lines[-1]` instead of `lines[0]` | same | now | 9 OK. Equivalent unless duplicated, and duplicates are refused anyway |
| `mutate.sh m5`, heading regex `^#+ (M\w+), ` | same | now | 9 OK. No `#### M` heading exists, so this is harmless |
| `review5/probe.py`, pass-4 scenarios plus 7 adjacent edits | working-tree generator | now | as listed under S2 |
| `render(load())` headings against the live plan | working-tree generator, live allocation | now | identical, 96 and 96 |
| `cargo test -q -p ocelli-folder-viewer` | page `2efbf34`, more `c8aceba`, use `6cdc7af`, hashed before and after the run | now | 0, 31 passed |
| `cargo clippy -q -p ocelli-folder-viewer --all-targets -- -D warnings` | same | now | 0 |
| `cargo fmt --all --check` | same | now | 0 |
| `backlog_check.py`, `prose_check.py`, `deviation_check.py`, `source_provenance_check.py` | pinned tree | now | all 0. 240 F-IDs, 152 done. 794 files. 27 deviations. 1605 files |
| `sync_agent_skills.py --check` | pinned tree | now | 0, 42 Codex and 19 OpenCode adapters match 21 sources |
| `html.parser` on `page.html:7` | `2efbf34` | now | `href` is `data:image/svg+xml,%3Csvg xmlns=`, followed by 20 stray attributes |
| Chromium via Playwright on the live `:8742/`, served bytes identical to `2efbf34` (`git hash-object` of the `curl` body) | live viewer | now | `link[rel=icon]` href truncated as above, `new Image()` fires `error`. `/favicon.ico` is 404. Brand computed colour `rgb(230, 230, 230)`, svg fill the same. Logo box 66.8 by 20. Focused link outline `solid 2px rgb(255,255,255)` offset 4px. One `h1` |
| Inline logo body against `ocelli-slice-logo-dark.svg` | three pages, source SVG | now | byte-equal after the `<title>` in all three (6138 bytes) |
| SVG scan for `script`, `foreignObject`, `on*=`, `href`, `xlink`, `url(`, `data:`, `style`, `image`, `use` | 6 SVGs | now | none. Elements are only `svg`, `title`, `g`, `path`, plus `rect` and `text` in the preview |
| `staged_content_check` functions applied to the 8 files | untracked folder | now | no DICOM, no artefact path, none over 2 MB, no derived-output prefix |
| `prose_check.in_scope` and `scan` on `README.txt` | untracked | now | out of scope (not `.md`). Scanned anyway, 0 problems |
| aspect swap mutation | pass-3 receipt on `fc7d8b9` | prior | `page.html` differs from `fc7d8b9` only at the favicon, `.brand` CSS and the `h1` line, so the receipt carries |

## Defects

### D1, the favicon is a broken data URI on all three pages
**Where**: `examples/folder-viewer/src/page.html:7`, `more.html:7`, `use.html:7`. The line is identical in all three (md5 `dc8f73d9...`).
**What**: `href="data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" ..."`. The SVG's own attribute values use raw `"`, and the attribute is closed at the first one. The browser's `href` is `data:image/svg+xml,%3Csvg xmlns=`. The rest of the line becomes junk attributes (`http:`, `www.w3.org`, `svg"`, `viewbox`, `d`, ...), and no icon renders.
**Why it is wrong**: the change adds a favicon, and it never displays. HTML attribute tokenisation ends a double-quoted value at the next `"`.
**Evidence**: `html.parser` shows the truncated href. Chromium on the live viewer reports the same href, and an `Image` loaded from it fires `error`. `/favicon.ico` returns 404, so there is no fallback either.
**Scope and regression**: all three pages, current tree. No test touches the `<link>`. Smallest fix: write the inner SVG with single quotes (`xmlns='http://www.w3.org/2000/svg' viewBox='-6 -6 112 112' fill='%23e6e6e6'`, and likewise for each `d` and `transform`), or percent-encode `"` as `%22`. Smallest regression: in `preview.rs`, for each page, extract the `rel="icon"` href and assert it starts with `data:image/svg+xml,`, contains no raw `"` and ends with `%3C/svg%3E`.

### D2, the M20 note's new reason is false as a general rule
**Where**: `docs/sprints/SPRINT_PLAN.md:655-657`
**What**: "because each sprint table is written once, under the first milestone whose span starts at that sprint, and S46 starts M14." The conclusion for S46 is right, because M14 is the only milestone that starts at S46. The stated rule is wrong in both of its other cases:
- **A sprint that starts no milestone.** S47 to S49 start none. Their tables sit under M14, which starts at S46.
- **A sprint that starts two milestones.** `render` (`gen_sprint_plan.py:133-144`) writes every starting milestone's heading and then the table, so the table sits under the **last** of them, not the first.
**Why it is wrong**: it is a false causal claim in prose (`microscope.md` section 3). It is the same class of error as pass-4 D2, and a reader who relies on the rule will mispredict placement.
**Evidence**: the repository's own `test_render_writes_a_heading_for_every_milestone` asserts `### M1` < `### M2` < `#### Sprint S1`, so S1's table follows M2, the second milestone to start there. `probe.py` case B prints `['### M1, M1', '#### Sprint S1', '### M2, M2', '### M3, M3', '#### Sprint S2']`.
**Scope and regression**: prose only. Smallest fix: pass 4's wording, "because the plan writes each milestone heading once, before its first sprint, and S46 is M14's first", or simply delete the "because" clause.

## Smells

None.

## Carried, not counted

- **Pass-4 S1, the aspect swap.** It is carried into F-208 AC14 under the operator's delegated call, as judged above. It still holds: the swap passes all 31 tests (pass-3 receipt, applicable as shown). It closes only when AC14's browser check exists and its mutation control has been observed red.

## Nitpicks

- N1. The `len(lines) > 1` branch (`gen_sprint_plan.py:363-366`) and the extra-key loop (`:373-376`) are both reached and correct by probe, but no test covers them. Disabling either leaves 9 tests green. One fixture each, a duplicated line and an `### M9` heading, would pin them.
- N2. `gen_sprint_plan.py:224`, `out: list[tuple[str, str, int, int]]`, is stale. The list now holds 5-tuples, and the return annotation at `:215` is right.
- N3. `more.html:27` and `use.html:27`, `header.top strong { ... }`, are dead CSS. The `<strong>Ocelli</strong>` they styled is now the `.brand` link.
- N4. Carried from pass 4 N1. A non-UTF-8 `OCELLI_PORT` reaches `Err(_) => Ok(0)` (`serve.rs:100`) and is treated as unset, while README `:189-190` says a value that is not a port number stops the process. `chosen_port` still has no unit test.
- N5. On `page.html:568` the `h1`'s accessible name is the link's `aria-label`, "Ocelli, Images", so assistive technology announces the page heading as "Ocelli, Images". `aria-label="Ocelli"` on the link, or the link outside the `h1`, would keep the heading as "Ocelli". It is not a duplicate `h1`: each page has exactly one.
- N6. The README `<picture>` uses `ocelli-slice-logo-light.svg`, fill `#f5f5f3`, and `ocelli-slice-preview.svg` uses `#faf9f6`. Neither has equal RGB channels. The operator's pure-grey rule is stated for viewer chrome, which uses `#e6e6e6` throughout, so this is recorded rather than raised. Separately, both README paths resolve relative to the repository root (`docs/ocelli-slice-svg/...`), but the folder is untracked. Committing README without the folder publishes a broken image.
- N7. `CURRENT_SPRINT.md:47-48` paraphrases the operator ("to make the quality call"). Quoting the sentence verbatim, with its date, would make the delegation checkable later. The section is not read by `close-preflight`, so only the sprint review sees it.

## Verified clean

- **Generator.** `check` is keyed by `### Mx` heading. It refuses a missing line, a duplicate, an orphan heading, an extra milestone and a misplaced line, each with the milestone named. `render` and `rendered_milestones` are now one derivation. The live plan's heading sequence equals the render's. HEAD's allocation behaviour was established in pass 4 and is not re-run.
- **Pass-4 D1, D3, D4, S2, S3 and the three nitpick follow-ups are closed**, as replayed above.
- **Logo artwork.** None of the 6 SVGs has a script, event handler, `foreignObject`, external reference, `data:` URI or `style`. Dark, light and `currentColor` variants share identical path data, and the mark is the logo's first 4 paths. `README.txt`'s claims hold for the logo and mark files: outlined lettering, no raster, no external font, no mask, no script. The preview's two `<text>` labels use a system font stack and no web font, and the README says the previews are for display only.
- **Inline wordmark.** Identical to `ocelli-slice-logo-dark.svg` in all three pages, with the source's `#141414` swapped for `fill="currentColor"`, resolving to `rgb(230,230,230)`, which has equal channels. Accessible name "Ocelli" on the `svg`, link name "Ocelli, Images". Focus-visible outline present. `<canvas` is absent from `more.html` and `use.html`, and the test at `preview.rs:247` passes.
- **Content guard and provenance.** The content guard accepts every file in the folder. `source_provenance_check.py` exits 0.
- **No stale prose about the top bar.** README `:215` "icons at either end of the top bar" is still true, because `toggle-series` is still the first child of the top bar.

## Limitations and handoff

- Only the Images page was loaded in a browser. For More and Use, the favicon line was compared by hash and the brand markup was read, not rendered. Keyboard focus was set with `focus()`, not by Tab traversal.
- The aspect mutation was not re-run. The pass-3 receipt applies by diff.
- GitHub rendering of `<picture>` and `prefers-color-scheme` was not observed, only the relative paths were resolved.
- `.claude/plans/F-208-design.md` was read only around AC14 and its status line. The rest of that plan was not reviewed.
- Next: the author fixes D1 and D2, ideally with N1 and N2. Then a fresh independent pass 6 reviews the changed lines. The polish cannot be called clean until that pass reports 0 and 0. S1 stays carried until F-208 AC14 is observed red under the swap and green without it.
