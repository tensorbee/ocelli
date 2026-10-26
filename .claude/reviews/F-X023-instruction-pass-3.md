# F-X023 review, pass 3

**Reviewed**: staged tree `e125281e5935204040b6acd34fe8c3fbb6c15fa6`, the F-X023 instruction material. The index at report time is `d07f566f6423c60499605a546803326383fb44fa`. That later tree adds the F-101 pass 4 review, `crates/ocelli-wasm/src/session.rs` product changes, LLD metadata and benchmark configuration, all outside F-X023. Concurrent unstaged paths appeared during the pass and are disclosed in the limitations. The F-X023 paths are identical between `e125281e` and `d07f566f`.
**Author / reviewer**: implementation and remediation by xAI model and Codex. This pass was invoked by the operator as an independent reviewer and did not author the reviewed material. The earlier pass 2 by an independent reviewer reported 1 defect, 1 smell and 3 nitpicks. The instruction refinement passes 1 to 6 were written by the implementing model and are not an independent whole-preparation approval.
**Result**: 2 defects, 1 smell, 2 nitpicks
**Scope**: F-X023 AC4 to AC7 and the shared canonical instruction remediation. AC1 to AC3 remain covered by `.claude/reviews/F-X023-closure-pass-1.md`. The F-101 boundary test at `e125281e` and the LLD metadata are outside F-X023 product and are disclosed separately below.

## Acceptance challenged

AC4. The prepare command still stops before implementation, verification receipts, commits, pushes, merges, tags and publication. Its allowed-writes set is still closed. The new deviation paragraph and `/design` step 3 now agree that a draft records the need under `## Open questions` and does not write the register. The design template's `## Deviations` line still says a new row is added to `docs/hld/DEVIATIONS.md` in the plan change, so a draft that needs a deviation still has two instructions to choose between. This is S1.

AC5. The numbered acceptance inventory and the composition challenge are unchanged and still present. The generated adapters match the canonical sources.

AC6. The `invalidate` subcommand now distinguishes a removed receipt from an absent one. I executed both branches against a temporary ledger. The distinction is correct. The existing unit test exercises the absent branch but does not assert its message, which is N1.

AC7. The adapters, guard census, prose check, skills gate, backlog gate and deviations gate are green on the reviewed material. The guards gate could not complete on the current tree because a concurrent writer changed the developer's unstaged set while the harness ran. That is an external concurrency result, not a guard weakening, and it is recorded in the evidence table.

The independent counterexample for the delivery record is the S16 audit intake paragraph in `docs/sprints/SPRINT_PLAN.md` and the audit intake paragraph in `docs/sprints/CURRENT_SPRINT.md`. The plan says F-X023 is pending its independent instruction review and also says the review is recorded in `.claude/reviews/F-X023-instruction-pass-2.md`. The current sprint says both stories remain pending, the remaining review is unassigned, and none of F-048, F-101 and F-X003 is marked done, while its own status table and `BACKLOG.md` mark F-X022 and F-048 done and the pass 2 review is in the reviewed tree. The two delivery-record paragraphs are stale against the tree.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `git write-tree` at the correction and at report time | index | executed now | `d07f566f6423c60499605a546803326383fb44fa`, which adds the F-101 pass 4 review, session product changes, LLD metadata and benchmark configuration to `e125281e` |
| `git diff b71910494b2474338073d0f3ed122077700f3b14 a6241916bdf70c61724ff42636de0d4c69591274` | remediation delta | executed now | the plan, prepare-sprint, design, adapters, runbook, verify_ledger, LLD, delivery-record metadata and ring comments |
| `git diff a6241916bdf70c61724ff42636de0d4c69591274 e125281e5935204040b6acd34fe8c3fbb6c15fa6` | small delta | executed now | CURRENT_SPRINT count command, LLD public-api, gpu-ownership, README, and the F-101 boundary test |
| `python3 scripts/sync_agent_skills.py --check` | 21 canonical sources | executed now | exit 0, 42 Codex and 19 OpenCode adapter files match |
| `shasum -a 256 .claude/commands/design.md .claude/commands/prepare-sprint.md` | canonical sources | executed now | `b4f490a9c801899a544edb6c0705f1435e635a28502593ea9bad561a4fe7b0e0` and `93cca288288d6bab96d0d0164fb319b98b6978e444eadee8078961a3f58045af`, matching the adapter headers |
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | tree `e125281e` | executed now | exit 0, 16 passed |
| `python3 scripts/gen_sprint_plan.py --check` | allocation and plan | executed now | exit 0, 180 planned F-IDs, 18 milestone summary lines and 72 goal lines match |
| `python3 scripts/guard_census.py` | tree `e125281e` | executed now | exit 0, 892 refusals in 73 files, all claimed |
| `python3 scripts/prose_check.py` | tree `e125281e` | executed now | exit 0, voice rules clean over 467 files |
| `bin/ocelli.sh gate backlog` | tree `e125281e` | executed now | exit 0, 213 F-IDs, 67 done, 72 goal lines match |
| `bin/ocelli.sh gate skills` | tree `e125281e` | executed now | exit 0, 42 Codex and 19 OpenCode adapters, 3 examples and 32 tests |
| `bin/ocelli.sh gate deviations` | tree `e125281e` | executed now | exit 0, 25 deviations, every citation resolves |
| `bin/ocelli.sh gate guards` | tree `e125281e`, concurrent writer active | executed now | exit 1, the harness reported "the harness changed the developer's repository while it ran" because the unstaged set changed during the run. The probe output showed the declared red and green probes. The earlier run on `a6241916` exited 0 |
| Temporary-ledger `cmd_invalidate` calls | `scripts/verify_ledger.py` | executed now | exit 0, "no verification receipt ... nothing to invalidate" then "invalidated verification ..." then "no verification receipt ... nothing to invalidate" |
| `grep` for `EventRing`, `push_presented`, `drain_events`, `sessionBulkSink`, `sessionCommandSink`, `commit_dicomweb_response`, `ComputeError` | producer and consumer sources | executed now | the ring producer is in `crates/ocelli-viewport/src/ring.rs`, the adapter names are exported from `packages/core/src/index.ts`, the Rust and wasm sides have no `commit_dicomweb_response`, and no `From<ComputeError>` exists |
| `.claude/reviews/F-X023-instruction-pass-2.md` | prior receipt | prior receipt | 1 defect, 1 smell and 3 nitpicks on tree `b7191049` |
| `.claude/reviews/F-X023-closure-pass-1.md` | prior receipt | prior receipt | 0 defects, 0 smells, AC1 to AC3 only |
| F-048 and F-X022 AS_BUILT, tracker, CHANGELOG and backlog metadata | tree `e125281e` | reviewed now, not re-executed | review counts match `S16-run.json`, the F-048 test count matches `crates/ocelli-render/tests/view.rs`, the F-X022 tests exist, and the GPU and oracle claims remain prior receipts |

## Defects

### D1, the S16 plan says the instruction review is pending while recording it

**Where**: `docs/sprints/SPRINT_PLAN.md:900` and `docs/sprints/SPRINT_PLAN.md:905`.

**What**: The paragraph says "F-X023 remains pending its independent instruction review and the final sprint verification." Later in the same paragraph it says "independent review is recorded in `.claude/reviews/F-X023-instruction-pass-2.md`." The pass 2 report is in the reviewed tree and records 1 defect, 1 smell and 3 nitpicks. The story is pending a clean independent pass after the pass 2 remediation, not pending the review itself.

**Why it is wrong**: The delivery record must state the current review state. The two sentences contradict each other. The prior pass 2 D1 was a delivery-record misstatement. This remediation replaced "pending implementation" with "pending its independent instruction review" and added the recorded review, so the misstatement survives in a new form. A reader cannot tell whether the review is pending or recorded.

**Evidence**: `sed -n '897,907p' docs/sprints/SPRINT_PLAN.md` shows both sentences. `.claude/reviews/F-X023-instruction-pass-2.md` is present in the reviewed tree and its result line is 1 defect, 1 smell and 3 nitpicks. `bin/ocelli.sh gate backlog` exits 0 because no gate reads this paragraph.

**Scope and regression**: The S16 plan and the operator's view of the story. The smallest meaningful fix is "F-X023 remains pending a clean independent instruction pass after the pass 2 remediation and the final sprint verification." Present on `e125281e`.

### D2, the CURRENT_SPRINT audit paragraph is stale

**Where**: `docs/sprints/CURRENT_SPRINT.md:217`, `docs/sprints/CURRENT_SPRINT.md:222` and `docs/sprints/CURRENT_SPRINT.md:224-225`.

**What**: The paragraph says "Both stories remain pending", "That remaining review is unassigned" and "None of the three is marked done". The same file's status table at lines 12 to 16 marks F-048 done and F-X022 done, and `docs/sprints/BACKLOG.md` marks both done. The pass 2 review is recorded in the reviewed tree. The status table marks F-X023 in-progress while `BACKLOG.md` marks it pending. The paragraph is dated 2026-10-18 but uses present tense and sits in the same file as the current status table, so it reads as current.

**Why it is wrong**: `.claude/WORKFLOW.md` makes `docs/sprints/` the delivery record. A current-state paragraph that says completed stories are pending and a recorded review is unassigned misstates the sprint. The remediation changed the status table and added the F-X022 and F-X023 bullets but did not update this paragraph. The mismatch survives a green backlog gate because the gate reads `BACKLOG.md` and `SPRINT_PLAN.md`, not this paragraph.

**Evidence**: `sed -n '211,226p' docs/sprints/CURRENT_SPRINT.md` shows the paragraph. `grep -n "F-048\|F-X022\|F-X023" docs/sprints/BACKLOG.md` shows F-048 done, F-X022 done and F-X023 pending. The status table shows F-048 done and F-X022 done. `bin/ocelli.sh gate backlog` exits 0 on the same tree.

**Scope and regression**: The current sprint record, the operator's view and the review assignment. The smallest meaningful fix is to update the paragraph to present state, with F-X022 and F-048 complete, F-X023 in progress pending a clean instruction pass and final verification, and the pass 2 review recorded. If the paragraph is intended as a historical record, change the tense to past and state the date boundary. Present on `e125281e`.

## Smells

### S1, the design template still tells a draft to write the deviation register

**Where**: `.claude/commands/design.md:202`, with `.claude/commands/design.md:48-52` and `.claude/commands/prepare-sprint.md:24-28`.

**What**: The plan template's `## Deviations` line says "A new one is added to `docs/hld/DEVIATIONS.md` here." Step 3 and the prepare command say a draft records the need under `## Open questions`, does not write a `D-NN` row, and the row is written when the plan is approved. The template does not distinguish draft from approved, so a draft plan that needs a new deviation has two instructions. Following the template writes the register during preparation and breaches the prepare command's allowed-writes set. Following step 3 leaves the template line unmet.

**Why it is wrong**: The prior S1 was the same contradiction between the allowed-writes set and the design step. The remediation fixed step 3 and the prepare command but left the template. The next agent must choose which instruction to break. The specific trigger is a draft design that needs a new deviation. The hazard is latent because no draft currently cites an undeclared `D-NN`.

**Evidence**: `sed -n '200,203p' .claude/commands/design.md` shows the template line. `sed -n '48,52p' .claude/commands/design.md` and `sed -n '24,28p' .claude/commands/prepare-sprint.md` show the draft rule. `bin/ocelli.sh gate deviations` exits 0 today because no draft cites an undeclared `D-NN`.

**Scope and regression**: Any draft design that needs a new deviation. The smallest meaningful fix is to change the template line to "D-NN rows for approved deviations, or 'none'. A draft names a needed deviation under Open questions and the row is added to docs/hld/DEVIATIONS.md when the plan is approved." Present on `e125281e`.

## Nitpicks

N1. `scripts/tests/test_sprint_workflow.py:94-97` exercises the new "nothing to invalidate" branch but redirects stdout and asserts only exit 0. The message distinction can regress silently. The smallest fix is to capture stdout and assert both messages.

N2. `docs/runbooks/guard-verification.md:194` says the census default "checks the table unless `--render-runbook` is passed". `scripts/guard_census.py:228` also skips the table check when the census reports problems, so that is a second exception. The gate is red in that case, so the claim holds for a green gate. The smallest fix is "checks the table when the census finds no problems, unless `--render-runbook` is passed."

## Verified clean

The prepare command and `/design` step 3 now agree on a draft's deviation handling. A draft records the need under `## Open questions`, does not write a `D-NN` row, and the row is written at approval. The generated adapters match the canonical sources. `sync_agent_skills.py --check` exits 0 with 42 Codex and 19 OpenCode adapter files for 21 canonical sources, and the design.md and prepare-sprint.md SHA-256 values match the adapter headers.

The prior N1 is confirmed not a defect. `python3 scripts/gen_sprint_plan.py --check` exits 0 with 72 goal lines. `rendered_goals` prints the S16 line by joining the allocation story titles with ", ", preserving their capitalisation. Changing the capitalisation would fail the check with "order or punctuation has drifted", so the capitalisation is a generator style choice and not a plan defect.

The prior N2 is closed. `verify_ledger.py invalidate` now prints "invalidated verification for tree ..." when an entry was removed and "no verification receipt for tree ..., nothing to invalidate" when it was not. I executed both branches against a temporary ledger. The new branch is correct.

The prior N3 is closed. The runbook now names `--render-runbook` correctly and the `guards` gate runs the census default.

The ring-producer comments are correct. `EventRing` and `push_presented` are in `crates/ocelli-viewport/src/ring.rs`, `crates/ocelli-viewport/tests/ring.rs` asserts 16 and 48 with literals, `drain_events` returns a copied `Vec<u8>`, and `sessionBulkSink` and `sessionCommandSink` are exported from `packages/core/src/index.ts`. The `bulk.ts` and `ring.ts` comments name the real producer and adapters.

No gate was disabled or weakened in the remediation. `git diff b71910494b2474338073d0f3ed122077700f3b14 a6241916bdf70c61724ff42636de0d4c69591274 -- .github bin ci scripts/guards scripts/sprint_workflow.py scripts/tests scripts/unsafe_allowlist_check.py` is empty. The delta from `a6241916` to `e125281e` touches `CURRENT_SPRINT.md`, three LLD files and an F-101 boundary test, not a gate.

The `bin/ocelli.sh gate backlog`, `gate skills` and `gate deviations` receipts are green on the reviewed material. The guard census reports 892 refusals in 73 files, all claimed. The prose check is clean over 467 files. The `gate guards` failure is the harness's concurrent-write invariant, not a guard result, and is separated in the limitations.

The F-048 and F-X022 AS_BUILT, tracker, CHANGELOG and backlog metadata are internally consistent with `S16-run.json` for the review counts and statuses. The F-048 test count matches `crates/ocelli-render/tests/view.rs`, and the F-X022 tests exist in `crates/ocelli-viewport/tests/presentation.rs`. The GPU, oracle and product claims are prior receipts and were not re-executed.

## LLD metadata, reviewed separately

The LLD delta from `b7191049` to `e125281e` is not only headers and dates. `docs/lld/errors.md`, `docs/lld/dicomweb.md`, `docs/lld/feature-availability.md`, `docs/lld/public-api.md` and `docs/lld/gpu-ownership.md` rewrite state claims. I checked the executable claims. `EventRing` and `push_presented` exist in `crates/ocelli-viewport/src/ring.rs`. `crates/ocelli-viewport/tests/ring.rs` uses literal 16 and 48. `drain_events` returns `Vec<u8>`. `commit_dicomweb_response` is absent from Rust and wasm exports and remains a TypeScript sink interface. No `From<ComputeError>` exists. The F-ID headers and README rows agree. I did not re-execute the GPU or oracle claims. No F-X023 instruction defect was found there. The parent's description "headers and dates" understates the content changes in those files.

The F-101 boundary test added at `e125281e` holds a copied state and drained events across a later memory growth. That is F-101 product and is outside this F-X023 instruction review. I did not review it as F-X023 material.

## Limitations and handoff

The reviewed tree is `e125281e`. The index at report time is `d07f566f`, which adds `.claude/reviews/F-101-boundary-pass-4.md`, `crates/ocelli-wasm/src/session.rs` product changes, `docs/lld/build-targets.md`, `docs/lld/typescript-packaging.md`, `docs/lld/README.md`, `packages/core/src/bulk.test.ts`, `tools/bench/tests/run_test.mjs` and `vitest.config.ts`, outside F-X023. Concurrent unstaged paths appeared during the pass. They are the same F-101 product and LLD metadata paths. The F-X023 paths are unchanged between `e125281e` and `d07f566f`.

`bin/ocelli.sh gate guards` exited 1 on the current tree because the harness detected the developer's unstaged set changing while it ran. The probe output showed the declared red and green probes, but the harness's invariant failed on the concurrent writes. The earlier run on `a6241916` exited 0. This is an external concurrency failure, not a guard weakening. The guards gate on the final stable tree remains unproven.

I did not run `bin/ocelli.sh gate --sprint`, the oracle, GPU tests or the product suite. The F-048 and F-X022 GPU, oracle and product claims are prior receipts and are not re-executed here.

AC4 to AC7 are not approved. D1 and D2 are delivery-record defects. S1 is a latent instruction contradiction. F-X023 remains open. The parent session should remediate D1, D2 and S1, then verify the new staged tree and request another independent pass. The guards gate must be re-run on a stable tree before any completion receipt is recorded.
