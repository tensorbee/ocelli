# F-X023 review, pass 4

**Reviewed**: staged tree `edc78a490226e1eb251551e8ab91f1f0e9489ecd`, confirmed by `git write-tree`, over the F-X023 instruction remediation delta from `e125281e5935204040b6acd34fe8c3fbb6c15fa6`. The F-101 product, benchmark and LLD paths in the same delta are disclosed in the limitations and are not reviewed here. One unstaged modification to `.claude/reviews/F-X023-instruction-pass-3.md` appeared during the pass and is disclosed in the limitations.
**Author / reviewer**: implementation and remediation attributed to xAI model and Codex in the pass 3 receipt. This pass was invoked by the operator as an independent reviewer and did not author the reviewed material. The pass 2 and pass 3 reviews were written by other independent reviewers. The instruction refinement passes 1 to 6 were written by the implementing model and are not an independent whole-preparation approval.
**Result**: 0 defects, 0 smells, 0 nitpicks
**Scope**: F-X023 AC4 to AC7 and the shared canonical instruction remediation. AC1 to AC3 remain covered by `.claude/reviews/F-X023-closure-pass-1.md`.

## Acceptance challenged

AC4. The prepare command still stops before product implementation, verification receipts, commits, pushes, merges, tags and publication. Its allowed-writes set is unchanged. The remediation closes the last contradiction. `/design` step 3 at `.claude/commands/design.md:48-52`, the `## Deviations` template at `.claude/commands/design.md:200-204` and `.claude/commands/prepare-sprint.md:24-28` now give one rule. A draft records a needed deviation under `## Open questions` and does not write a `D-NN` row. The row is written to `docs/hld/DEVIATIONS.md` when the plan is approved. The plausible wrong behavior, a draft that writes the register during preparation and breaches the allowed-writes set, is rejected by all three sources. `gate deviations` exits 0 with 25 deviations and every citation resolving.

AC5. The numbered acceptance inventory at `.claude/commands/design.md:144-154` and the composition challenge at `.claude/commands/microscope.md:132-182` are unchanged and present. The generated adapters match the canonical sources. The plausible wrong behavior, a design template with only a `Tests` table or a review that checks only a shared helper, is rejected by the retained text.

AC6. The invalidation test now distinguishes a removed receipt from an absent one. I removed the message distinction in a temporary runner and the test failed at `scripts/tests/test_sprint_workflow.py:101` with `'no verification receipt for tree old-tree' not found in 'invalidated verification for tree old-tree\n'`. The distinction is load-bearing. The final sprint verification for the reviewed tree is absent. `python3 scripts/verify_ledger.py assert` exits 1 with "no verification recorded for the staged tree edc78a490226". The delivery record says that verification is pending, so the paragraph and the ledger agree.

AC7. The adapters, guard census, prose check, skills gate, backlog gate, deviations gate and content gate are green on the reviewed tree. `gate guards` and `gate --sprint` were not run because another independent reviewer is mutating `crates/ocelli-wasm/src/session.rs` in the same working directory. That leaves the full AC7 gate set unproven on this tree and is recorded in the limitations.

The composition challenge for the delivery record is the S16 audit intake paragraph at `docs/sprints/SPRINT_PLAN.md:897-907` and the audit intake paragraph at `docs/sprints/CURRENT_SPRINT.md:214-230`. I executed every factual claim against `docs/sprints/BACKLOG.md`, the status table at `docs/sprints/CURRENT_SPRINT.md:12-16`, `.claude/scratch/S16-run.json` and the review files in the tree. The plausible wrong behavior, a paragraph that calls a completed story pending, calls a recorded review unassigned, or claims a story is done while it is in progress, is rejected. F-048 is `done` in BACKLOG and completed in the run state with a clean pass 7. F-X022 is `done` in BACKLOG and completed in the run state with a clean pass 1. F-101 is `in-progress` in both and its pass 4 reports 1 defect and 2 smells, so "pending its independent remediation pass" holds. F-X003 is `carried` in the run state and in the status table, its carry reason is the unavailable GPU-less host, and the `coverage-loss` verdict is recorded at `docs/sprints/CURRENT_SPRINT.md:172`. BACKLOG keeps its row `in-progress`, which the carry-forward section at line 175 states explicitly. F-X023 is not claimed done, pass 2 and pass 3 exist, and the pass 3 result is 2 defects, 1 smell and 2 nitpicks.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` | index | executed now | `edc78a490226e1eb251551e8ab91f1f0e9489ecd` |
| `git status --porcelain` | working tree | executed now | all reviewed paths staged, one unstaged modification to `.claude/reviews/F-X023-instruction-pass-3.md` appeared during the pass, no untracked files |
| `git diff e125281e edc78a49 --name-status` | remediation delta | executed now | 16 files, 8 F-X023 paths, 8 F-101 or LLD paths |
| `python3 scripts/sync_agent_skills.py --check` | 21 canonical sources | executed now | exit 0, 42 Codex and 19 OpenCode adapters match |
| `shasum -a 256 .claude/commands/design.md .claude/commands/prepare-sprint.md` | canonical sources | executed now | `005a2d42814db8f9ac20f7fb9dde66193bb6a9af88390780d7dc4596d61bcb98` and `93cca288288d6bab96d0d0164fb319b98b6978e444eadee8078961a3f58045af`, matching the adapter headers |
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | tree `edc78a49` | executed now | exit 0, 16 passed |
| Mutated `cmd_invalidate` with the message distinction removed, then the same test | temporary runner against `scripts/verify_ledger.py` | executed now | FAIL at `test_sprint_workflow.py:101`, the absent-receipt message was not found |
| `python3 scripts/gen_sprint_plan.py --check` | allocation and plan | executed now | exit 0, 180 planned F-IDs, 18 milestone summary lines and 72 goal lines match |
| `python3 scripts/guard_census.py` | tree `edc78a49` | executed now | exit 0, 892 refusals in 73 files, all claimed |
| `python3 scripts/prose_check.py` | tree `edc78a49` | executed now | exit 0, voice rules clean over 468 files |
| `bin/ocelli.sh gate backlog` | tree `edc78a49` | executed now | exit 0, 213 F-IDs, 67 done, 72 goal lines match |
| `bin/ocelli.sh gate skills` | tree `edc78a49` | executed now | exit 0, 42 Codex and 19 OpenCode adapters, 3 examples and 32 tests |
| `bin/ocelli.sh gate deviations` | tree `edc78a49` | executed now | exit 0, 25 deviations, every citation resolves |
| `bin/ocelli.sh gate content` | tree `edc78a49` | executed now | exit 0, no patient data or build artefacts staged |
| `python3 scripts/verify_ledger.py assert` | tree `edc78a49` | executed now | exit 1, no verification recorded for the staged tree |
| `sed -n` and `grep -n` over the two delivery-record paragraphs | tree `edc78a49` | executed now | claims match BACKLOG, the status table, the run state and the review files |
| `sed -n` over `.claude/reviews/F-X022-presentation-pass-1.md`, `.claude/reviews/F-X023-closure-pass-1.md`, `.claude/reviews/F-048-coordinates-pass-7.md`, `.claude/reviews/F-101-boundary-pass-4.md`, `.claude/reviews/F-X023-instruction-pass-2.md` and `.claude/reviews/F-X023-instruction-pass-3.md` | review receipts | executed now | result lines are 0/0/0, 0/0/0, 0/0/0, 1/2/5, 1/1/3 and 2/1/2 |
| `git diff e125281e edc78a49 -- .github bin ci scripts/guards Cargo.toml` and baseline or tolerance paths | gate and tolerance definitions | executed now | empty |
| `git diff e125281e edc78a49 -- vitest.config.ts` | test configuration | executed now | comments only, the `exclude` entry is unchanged |
| `.claude/reviews/F-048-coordinates-pass-7.md` and the F-048 AS_BUILT and tracker rows | prior receipt | prior receipt | seven passes converged, pass 7 is clean |
| F-X022 AS_BUILT entry | prior receipt | prior receipt | completed 2026-10-26 |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

D1 is closed. `docs/sprints/SPRINT_PLAN.md:897-907` now says F-X022 is complete, F-X023 remains pending a clean independent instruction pass after the pass 2 remediation and the final sprint verification, and the independent passes are recorded in the pass 2 and pass 3 files. Both files exist in the reviewed tree. The pass 2 result is 1 defect, 1 smell and 3 nitpicks, and the pass 3 result is 2 defects, 1 smell and 2 nitpicks. The paragraph does not claim pass 3 was clean and does not claim F-X023 is done. The verification claim is true because `verify_ledger.py assert` exits 1 for the reviewed tree.

D2 is closed. `docs/sprints/CURRENT_SPRINT.md:214-230` now states F-X022 complete with a clean independent pass, F-X023 in progress pending a clean independent instruction pass and the final sprint verification, its closure pass approved AC1 to AC3 only, and the instruction passes recorded in pass 2 and pass 3. It states F-048 complete, F-101 in progress pending its independent remediation pass, and F-X003 carried with its unavailable GPU-less host and `coverage-loss` verdict. The stale sentences "Both stories remain pending", "That remaining review is unassigned" and "None of the three is marked done" are gone. The status table at `docs/sprints/CURRENT_SPRINT.md:12-16` agrees on F-048 done, F-101 in-progress, F-X003 carried, F-X022 done and F-X023 in-progress. `docs/sprints/BACKLOG.md` agrees on F-048 done, F-101 in-progress and F-X022 done. The F-X003 carry reason and verdict are also recorded at `docs/sprints/CURRENT_SPRINT.md:172`.

S1 is closed. `.claude/commands/design.md:48-52` says a draft records a needed deviation under `## Open questions`, does not write the register, and the `D-NN` row is written in the same change that sets the plan to approved. `.claude/commands/design.md:200-204` says the same for the template. `.claude/commands/prepare-sprint.md:24-28` says the same and names the allowed-writes set. `.claude/WORKFLOW.md:134` says deviations are recorded by hand via a design plan, and `.claude/commands/complete-feature.md:21` requires any relied-on deviation to have a row at completion. There is one rule and no remaining contradiction.

N1 is closed. `scripts/tests/test_sprint_workflow.py:91-102` captures stdout, asserts `invalidated verification for tree old-tree` after the removal, resets the buffer, and asserts both `no verification receipt for tree old-tree` and `nothing to invalidate` after the absent branch. The mutation that removed the distinction failed the test at line 101, so the assertion is load-bearing rather than decorative.

N2 is closed. `docs/runbooks/guard-verification.md:193-194` now says the census default checks the table when the census finds no problems, unless `--render-runbook` is passed. `scripts/guard_census.py:228` is `if args.check_runbook or not problems:`, so the default skips the table check when the census reports problems, and `--render-runbook` returns at line 218 before the check. The `guards` gate runs `python3 scripts/guard_census.py` with no flags at `bin/ocelli.sh:224`, so the sentence describes the executed path. The `--record` mode also returns before the table check, and the sentence is scoped to the census default rather than to every flag.

The regenerated adapters match the canonical sources. `.agents/skills/design/SKILL.md:10` and `.opencode/commands/design.md:9` carry `005a2d42...`, which is the SHA-256 of `.claude/commands/design.md`. The prepare-sprint adapters carry `93cca288...`, which is the SHA-256 of `.claude/commands/prepare-sprint.md`. `sync_agent_skills.py --check` exits 0 over 42 Codex and 19 OpenCode adapter files for 21 canonical sources.

No gate was disabled or weakened and no tolerance or baseline changed. The delta contains no path under `.github`, `bin`, `ci`, `scripts/guards` or `Cargo.toml`, and no baseline or tolerance file. The one test-configuration change, `vitest.config.ts`, is comments only and leaves the `exclude` entry in place. The invalidation test change adds assertions rather than removing them.

The guard census reports 892 refusals in 73 files, all claimed, and the prose check is clean over 468 files. The backlog gate reports 213 F-IDs with 67 done, the skills gate reports 42 Codex and 19 OpenCode adapters with 3 examples and 32 tests, and the deviations gate reports 25 deviations with every citation resolving.

## Limitations and handoff

The reviewed tree is `edc78a49`. A concurrent writer modified `.claude/reviews/F-X023-instruction-pass-3.md` in the working tree during the pass. The staged version is the one in the reviewed tree. The unstaged edit changes two sentences in the pass 3 report and is not part of this review. The reviewed tree and the index still hash to `edc78a49`.

The delta also contains F-101 product and LLD material outside F-X023. Those paths are `crates/ocelli-wasm/src/session.rs`, `packages/core/src/bulk.test.ts`, `tools/bench/tests/run_test.mjs`, `vitest.config.ts`, `docs/lld/README.md`, `docs/lld/build-targets.md`, `docs/lld/typescript-packaging.md` and `.claude/reviews/F-101-boundary-pass-4.md`. They are disclosed and not reviewed here. The F-101 pass 4 review reports 1 defect, 2 smells and 5 nitpicks on tree `118d12e9`, so F-101 is not complete and its remediation pass remains open.

`.claude/scratch/S16-run.json` records F-X023 as `reviewed` while its last review pass 3 reports 2 defects and 1 smell, and `docs/sprints/BACKLOG.md` records it as `pending` while the status table and the paragraph record it as in progress. That three-way label mismatch pre-exists the remediation and is unchanged by it. It does not make the paragraph false because the story is not complete. It should be reconciled before the sprint closes.

`bin/ocelli.sh gate guards`, `gate --sprint`, the oracle and GPU tests were not run because another independent reviewer is mutating `crates/ocelli-wasm/src/session.rs` in the same working directory. The final sprint verification for `edc78a49` is not recorded. AC6's integrated evidence and AC7's full gate set therefore remain outstanding. AC4 to AC7 are not approved as complete by this pass, which reviews only the instruction remediation.

F-X023 remains open. The parent session should reconcile the F-X023 status labels, run the remaining gates and the sprint verification on a stable tree, and request another independent pass on that tree. This pass is a clean bounded review of the instruction remediation, not a completion receipt.
