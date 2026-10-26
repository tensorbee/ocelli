# F-X023 instruction review, pass 2

**Reviewed**: staged tree `b71910494b2474338073d0f3ed122077700f3b14`, union `git diff 22800f8c1b916818784c63b44dde1760c3512f04 b71910494b2474338073d0f3ed122077700f3b14`. The index matched that tree at the start of the pass. It moved to `cd140b90ccffd8c54503753b2ba3fced3136d46c` during the pass when a concurrent writer staged completion metadata and the F-X003 carry-forward, and then to `cbd7f51bb737e4bdc050256de06b2e78b853da9c` with LLD and ring updates. The reviewed tree is `b7191049`. The later indexes are disclosed separately below.
**Author / reviewer**: implementation and instruction preparation by xAI model and Codex. This pass was invoked by the operator as an independent reviewer and did not author the reviewed material. The earlier closure pass 1 by Codex approved AC1 to AC3 only. The instruction refinement passes 1 to 6 were written by the implementing xAI model and are not an independent whole-preparation approval.
**Result**: 1 defect, 1 smell, 3 nitpicks
**Scope**: AC4 to AC7 and the shared canonical instruction preparation. AC1 to AC3 remain covered by `.claude/reviews/F-X023-closure-pass-1.md`. This pass does not approve the concurrent completion metadata in the new index.

## Acceptance challenged

AC4. The command text was read end to end. It stops before product implementation, verification receipts, commits, pushes, merges, tags and publication. It lists allowed writes, dependencies, decision readiness and the handoff. The plausible wrong behavior, a prepare command that quietly invokes `/run-sprint`, starts a feature or marks a story in-progress, is rejected by the text at `.claude/commands/prepare-sprint.md:7-9` and `:33`. The generated OpenCode adapter names the same canonical file and matches its SHA.

AC5. `design.md` now carries the numbered acceptance inventory with the wrong-implementation column, and `microscope.md` carries the composition challenge that requires an independently derived case across affected consumers. The plausible wrong behavior, a design template that only has a `Tests` table, or a review that only checks a shared helper, is rejected by the template and the review text. The generated adapters match the canonical sources.

AC6. The `invalidate` subcommand was read and its unit test was executed. The test proves that invalidating one tree makes `assert` fail for that tree while another tree's receipt remains. `verify.md` requires invalidation before a verification attempt and a recorded sprint failure on failure. `complete-feature.md` stages completion metadata, runs the affected checks and the required profile on that final staged state, then records. Source identity, selected tests and GPU or candidate parity are kept as separate qualifications.

AC7. The adapter check, guard census, runbook check, prose check, skills gate, backlog gate, CI gate, deviations gate, content gate, unsafe gate and the new probes were executed. The adapters match, the runbook table is current and the new probes go red for their declared reasons. No parallel tracker and no copied normative specification were introduced.

The independent counterexample for the delivery record is the S16 audit intake paragraph. `docs/sprints/SPRINT_PLAN.md:899` says the two repairs remain pending for xAI model implementation. `docs/sprints/CURRENT_SPRINT.md:216-217` says xAI model implemented them. The union contains the F-X022 VOI refusal and the F-X023 closure inventory and backlog-status checks. The plan is stale against the current tree.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` before and after the pass | index, tree `b7191049` then `cd140b90` | now | 0, the index matched `b7191049` at the start and moved to `cd140b90` during the pass |
| `python3 -m unittest discover -s scripts/tests -p test_sprint_workflow.py` | tree `b7191049`, 16 tests | now | 0, 16 passed |
| `python3 scripts/sync_agent_skills.py --check` | tree `b7191049`, 21 canonical sources | now | 0, 42 Codex and 19 OpenCode adapter files match |
| `python3 scripts/guard_census.py` | tree `b7191049` | now | 0, 892 refusal sites in 73 files, all claimed |
| `python3 scripts/guard_census.py --check-runbook` | tree `b7191049`, generated runbook table | now | 0, the table matches the catalogue |
| `python3 scripts/guard_probe.py --self-test` | tree `b7191049`, harness | now | 0, 10 properties |
| `sprint-lifecycle.close-missing-inventory`, `close-extra-inventory`, `close-pending-backlog` | tree `b7191049`, disposable repositories | now | 0, all three red for their declared reasons |
| `sprint-lifecycle.close-current-evidence`, `close-recorded-carry` | tree `b7191049`, disposable repositories | now | 0, both accept controls green |
| `sprint-lifecycle.close-unrecorded-carry`, `close-tree-changed` | tree `b7191049`, disposable repositories | now | 0, both red for their declared reasons |
| `skills.stale-adapter`, `skills.hand-edited-opencode-command`, `skills.orphan-opencode-command` | tree `b7191049`, disposable repositories | now | 0, all three red for their declared reasons |
| `python3 scripts/prose_check.py` | tree `b7191049`, 465 files | now | 0, voice rules clean |
| `bin/ocelli.sh gate backlog` | tree `b7191049`, 213 F-IDs | now | 0, 213 F-IDs, 65 done, 180 planned, 18 milestone summaries and 72 goal lines agree |
| `bin/ocelli.sh gate skills` | tree `b7191049`, adapters and marked examples | now | 0, 42 Codex and 19 OpenCode adapters, 3 examples, 32 tests |
| `bin/ocelli.sh gate ci` | tree `b7191049`, `.github/workflows/ci.yml` | now | 0, all 26 floor gates are invoked by CI |
| `bin/ocelli.sh gate unsafe` | tree `b7191049`, full index | now | 0, 155 files checked, 2 permitted, 5 tests passed |
| `bin/ocelli.sh gate deviations prose content` | tree `b7191049` | now | 0, 25 deviations, 465 prose files, no patient data or build artefact |
| `grep` over `SPRINT_PLAN.md`, `CURRENT_SPRINT.md`, `BACKLOG.md` and the union | tree `b7191049`, delivery record | now | 0, the plan says pending implementation, the current sprint says implemented, the backlog shows F-048, F-101 and F-X003 in progress |
| `.claude/reviews/F-X023-closure-pass-1.md` | tree `74de1e69`, closure delta | prior receipt | 0 defects, 0 smells, AC1 to AC3 only |
| `.claude/reviews/S16-instruction-refinement-pass-1..6.md` | earlier instruction text | prior receipt | partial reviews by the implementing model, not a whole independent approval |
| Implementer integrated sprint gate receipt | earlier tree, not `b7191049` | prior receipt | not rerun here |
| F-X022 presentation pass 1 and F-X023 closure pass 1 native tests | their recorded trees | prior receipt | not rerun here |

## Defects

### D1, the S16 audit intake paragraph reports the repairs as unimplemented

**Where**: `docs/sprints/SPRINT_PLAN.md:899` and `docs/sprints/SPRINT_PLAN.md:902`.

**What**: The paragraph says "Both repairs remain pending for xAI model implementation" and "Remaining S16 product work is prepare-only under this request." The tree contains the F-X022 selected VOI-sequence refusal and the F-X023 closure inventory and backlog-status checks. `docs/sprints/CURRENT_SPRINT.md:216-217` says xAI model implemented them and both stories remain pending review. `docs/sprints/BACKLOG.md` shows F-048, F-101 and F-X003 in progress. The union adds the F-X022 and F-X023 implementation and the F-048, F-101 and F-X003 product code.

**Why it is wrong**: `.claude/WORKFLOW.md` makes `docs/sprints/` the delivery record. A plan that says a repair is pending implementation while the implementation is present misstates the sprint state. The microscope defect class is a claim in prose that is false. The backlog gate does not check this paragraph, so the claim survives a green gate.

**Evidence**: `grep -n "xAI model implemented" docs/sprints/CURRENT_SPRINT.md` gives lines 216 and 217. `grep -n "in-progress" docs/sprints/BACKLOG.md` gives F-101 and F-X003. The union adds `crates/ocelli-render/src/cpu.rs`, `crates/ocelli-viewport/src/commands.rs`, `crates/ocelli-wasm/src/session.rs` and the F-X022 and F-X023 implementation. `bin/ocelli.sh gate backlog` exits 0 on the same tree.

**Scope and regression**: The S16 plan, the operator's view of the sprint and the completion handoff. The smallest meaningful regression is to change the paragraph to "Both repairs are implemented and remain pending independent review and final verification. The remaining S16 product work is in progress." Present on the current tree. Uncertainty: the paragraph is dated 2026-10-18 and may be read as a historical record of the audit intake request. The words "remain" and "Remaining" are present tense, so it reads as current. If the author intended a historical record, the tense should be past and the sentence should say the repairs were implemented later.

## Smells

### S1, the prepare command's allowed-writes list excludes a deviation row that design requires

**Where**: `.claude/commands/prepare-sprint.md:18-21` and `.claude/commands/design.md:48-50`.

**What**: `prepare-sprint.md` says "Allowed writes are the existing sprint intake records, `.claude/plans/` and `.claude/scratch/F-XXX-progress.md` handoffs." Its step 3 invokes `/design F-ID --draft`, whose step 3 says a plan that needs a new deviation "gets a `D-NN` row there in the same change". A draft plan that needs a new deviation has no allowed write for the row. If the agent writes the row, it breaches the allowed-writes list. If it does not, the plan either cites an undeclared `D-NN` and `scripts/deviation_check.py` refuses it, or the deviation stays an open question and the design step is not followed.

**Why it is wrong**: AC4 says the command lists writes. The list is a closed set, and the invoked design command requires a fourth write. The two canonical instructions are not reconciled. The next agent must choose which instruction to break, which is a maintenance hazard with a specific trigger.

**Evidence**: The text is at `prepare-sprint.md` lines 18 to 21 and `design.md` lines 48 to 50. `bin/ocelli.sh gate deviations` exits 0 on the current tree because no draft plan cites an undeclared `D-NN` today. The hazard is latent.

**Scope and regression**: Any story whose draft design needs a new deviation. The smallest meaningful regression is to add `docs/hld/DEVIATIONS.md` to the allowed writes, or add a sentence to `prepare-sprint.md` that a needed deviation is recorded as an open question and its `D-NN` row is written when the plan is approved, not during preparation, and align design step 3 to that wording. Present on the current tree. Uncertainty: the design command's "same change" may be intended to mean the approval change, not the draft. The text does not say so.

## Nitpicks

N1. `docs/sprints/SPRINT_PLAN.md:300`, the S16 goal line capitalizes story fragments after commas ("..., Preserve selected VOI sequences..., Reconcile closure inventory..."). The backlog gate accepts the line, so this is a style preference.

N2. `scripts/verify_ledger.py:1028` prints "invalidated verification for tree ..." even when the tree had no entry. The command is idempotent, but the message claims an invalidation that did not happen. A reader cannot tell whether an entry was removed.

N3. `docs/runbooks/guard-verification.md:192` says "the `guards` gate carries the `--check`." The flag is `--check-runbook`, and the gate relies on the default census path rather than the flag. The check does run, so the claim is true in effect, but the flag name is wrong.

## Verified clean

AC4. `prepare-sprint.md` explicitly stops before product implementation, verification receipts and Git publication. It lists allowed writes, dependencies, decision readiness and the handoff. Nothing in it authorises an implementation step. The generated `.opencode/commands/prepare-sprint.md` names the canonical file and matches its SHA.

AC5. `design.md` requires the numbered acceptance inventory and the wrong-implementation column. `microscope.md` requires an independent case across affected consumers. The generated adapters match the canonical sources. The two reference skills are not generated as OpenCode commands, which is consistent with the sync script's stated rule.

AC6. `verify_ledger.py invalidate` removes only the selected tree's receipt, and the unit test proves that `assert` fails after invalidation while another tree remains. `verify.md` invalidates before a verification attempt and records a failed sprint verification on failure. `complete-feature.md` stages completion metadata, runs the affected checks and the required profile on the final staged state, then records. Source identity, selected tests and GPU or candidate parity are separate qualifications. A required target check that cannot execute remains unmet.

AC7. The adapters match. No parallel tracker and no copied normative specification were introduced. The guard census reports 892 refusal sites in 73 files, all claimed. The runbook table is current. The prose check is clean over 465 files. The skills gate runs 3 marked examples and 32 tests. The backlog gate agrees on 213 F-IDs. The CI gate finds all 26 floor gates invoked. The deviations gate resolves 25 deviations. The content gate finds no patient data or build artefact. The new inventory, backlog and OpenCode probes go red for their declared reasons, and the current-evidence and recorded-carry accept controls stay green. The unsafe gate runs its five temporary-repository controls and passes.

The close-preflight inventory and backlog-status checks are covered by the earlier closure pass and by the probes re-run here. The earlier scoped review's AC1 to AC3 findings are not reopened.

## Limitations and handoff

The index moved to `cd140b90` during the pass and then to `cbd7f51` by the end of it. The new indexes add F-048 and F-X022 completion metadata, an F-X003 carry-forward, LLD updates and `packages/core/src/ring.ts` changes. Those paths are outside the supplied union and were not reviewed here. The receipts in this report apply to `b7191049` only. The new index needs its own verification and independent review before completion.

The actual HEAD at the start of the pass was `2d5d101`, not the `22800f8` named in the review request. The union base is `22800f8`. The `2d5d101` commit, which generated the OpenCode commands, is included in the union.

This pass did not run `bin/ocelli.sh gate --sprint`, the oracle, GPU tests or the full product suite. The GPU and candidate-parity evidence is not re-executed. The product implementation of F-048, F-101, F-X003 and F-X022 is covered by other passes and remains outside this instruction review.

D1 requires a delivery-record correction and a fresh pass. S1 requires a decision on the deviation write and a fresh pass. AC4 to AC7 are therefore not approved by this pass. AC1 to AC3 remain approved by the earlier closure pass. F-X023 remains open.

The delivery record still says the independent review is unassigned. The parent session should record the reviewer assignment and the outcome of this pass after remediating D1 and S1, then verify the new staged tree before recording any receipt.
