---
description: Prepare the active sprint through intake and detailed draft designs, then stop before implementation, verification receipts, commits or publication.
---

# /prepare-sprint

Prepare only. This command does not invoke `/run-sprint`, start a feature,
implement product code, commit, push, merge, tag or publish. It does not change
`/complete-feature --prepare`, which remains the worker completion handoff.

## Inputs and allowed writes

Read AGENTS, CLAUDE, WORKFLOW, current sprint, backlog, allocation, roadmap,
tracker, AS_BUILT, in-flight notes and the relevant domain skills. Confirm the
branch, HEAD and worktree/index ownership. Preserve another writer's changes.
Inspect actual dependency and consumer code, HLD and declared deviations.

Allowed writes are the existing sprint intake records, `.claude/plans/` and
`.claude/scratch/F-XXX-progress.md` handoffs. Do not mark implementation
in-progress or done, approve a draft with unresolved material questions, write
verification receipts or change the product, tests, baselines or tolerances.
Record existing implementation work separately from new prepared work.

A draft that needs a new deviation records the need under the plan's
`## Open questions` and does not write a `D-NN` row. The row is written to
`docs/hld/DEVIATIONS.md` when the plan is approved, because a deviation is a
design decision and not a preparation artefact. This keeps the allowed-writes
set closed without contradicting `/design` step 3.

## Intake and designs

1. Resolve every unfinished F-ID from backlog, reconcile the active story set
   with allocation and CURRENT_SPRINT, and preserve parked work's reason.
2. Refresh relevant issues and PRs read-only when available. Pin issue body
   content and PR head, map every acceptance criterion to an F-ID, regression
   and evidence. Partial fixes stay partial. Record access failures honestly.
3. Write `/design F-ID --draft` for every unfinished story. Preserve existing
   approved decisions, and include the detailed acceptance and consumer map.
   Do not run the implementation or completion commands after design.
4. Identify dependencies absent from the imported `depends_eids` field by
   also reading `depends_fids`, declared additions and actual source usage.
   Name shared files, normative sections, ledgers, boundary mappings and the
   GPU. A known implementation gap is a dependency, not an assumed capability.
5. Produce a safe order. Serial is valid. Parallel work requires the executing
   workflow's authorization, isolated ownership and serialized GPU access.
6. State capacity changes as estimates, never measured duration. Leave material
   decisions in each plan's Open questions and consolidate questions once.
   Readiness is observable acceptance and settled decisions, not plan length.

## Handoff and stopping point

For every story record plan status, remaining decisions, exact tests, evidence
owner/stage, dependencies, write set and the next supported command. Name the
chosen implementer if the operator selected one. Name an independent reviewer
only if available, otherwise record that assignment is required.

For each unmet criterion, distinguish an unanswered design decision from an
unavailable execution resource. Record the owner's next action and the exact
condition for resuming. Keep the operator's existing answers, including that
no representative host is available. Do not invent a budget or promote a
draft to approved by renaming an open question as a future task.

Stop after preparation with a concrete handoff. `/run-sprint --max-workers 1`
is a later implementation entry point if authorized. It is never executed as
part of preparation. `/microscope F-ID --working` and whole-sprint review must
be performed by an actual independent reviewer. An unavailable reviewer leaves
a handoff, never a self-review labeled independent. `/close-sprint` remains a
separate operator action.
