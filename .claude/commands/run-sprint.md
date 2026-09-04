---
description: Run an entire sprint autonomously, design through consolidated verification and the review loop, pausing only for one consolidated design-question round. Everything except /close-sprint.
---

# /run-sprint [--max-review-passes N] [--max-workers N]

Run the whole active sprint. Default to three sprint-review passes and all
safely available workers.

## Rule zero, do not stop

**DO NOT END YOUR TURN WHILE SPRINT WORK REMAINS.** This is first because it is
the rule that gets broken.

Not stopping points, none of them:

- a story finishing, or `/complete-feature` committing
- a review pass coming back clean
- writing a progress summary or a status table
- a long gate finishing

**The test is mechanical: the last thing in a turn must be a tool call that
advances the sprint, never prose.** If you are writing a paragraph and no
further tool call follows it, you are stopping, whatever the paragraph says.

The breakage has a signature. A story closes, or a gate comes back green, and
the natural next act feels like a well-organised report on what just happened.
It is not. The report is a side effect, never the deliverable. Write it if it
helps, then keep going in the same turn: open the next file, run the next
check, start the next story.

Two consequences. Marking a story `in-progress` and then reporting is a stop,
not progress. And **finishing a story is the loudest false summit in the run**,
because the work feels complete when one of six rows moved.

The run ends in exactly three ways: the sprint reaches `ready_to_close`, the
review loop hits `--max-review-passes` with actionable findings and the sprint
is marked blocked, or one of the three unplanned-pause conditions genuinely
fires. Nothing else terminates it.

## The autonomy contract

**Every phase runs without asking, except one.** The single planned pause is
the consolidated design-question round in step 2. After it is answered, carry
the sprint through implementation, review, integration, consolidated
verification, ledgers and the review loop without further prompting.

Unplanned pauses are limited to a genuinely NEW decision the design round could
not have anticipated, a blocking external requirement, or an action outside
this command's remit. **Reporting progress is not pausing.**

**Launching implementation workers is authorised by this command.** Invoking
`/run-sprint` IS the request for parallel subagents in isolated worktrees. Do
not stop to ask whether to launch them.

`python3 scripts/sprint_workflow.py` is the resumable state authority. Reuse an
existing `.claude/scratch/SNN-run.json` rather than restarting.

## 1. Initialise

1. Read `CLAUDE.md`, `AGENTS.md`, `.claude/WORKFLOW.md`, `CURRENT_SPRINT.md`.
2. Confirm the canonical worktree is on `sprint/sNN`.
3. Refuse unrelated uncommitted changes.
4. `python3 scripts/sprint_workflow.py init --resume --max-review-passes {N}`.
5. List every story not `done`, its dependencies and state.
6. Load the domain skills the sprint's stories need. A sprint touching DICOM
   parsing, the LUT chain or geometry loads `dicom-expert`. A sprint touching
   the corpus or fixtures loads `dicom-tooling`.

## 2. Design every story first

1. `/design F-ID --draft` for **every** unfinished story. Do not implement yet.
2. Record ambiguities in `## Open questions` without interrupting the batch.
3. Compare all drafts and detect: dependency order, files and ledgers that
   cannot be edited concurrently, and stories touching the same HLD section.
4. **Ask one consolidated set of questions.** Group a shared decision once and
   name every affected story.
5. Apply each answer to every affected plan and set `**Status**: approved`.
6. If no material questions exist, approve without pausing.
7. Commit all approved plans together as `SNN, approve sprint designs`. This
   restores a clean worktree and gives every worker the same planning base.
   Do not push.

Do not start implementation while any design is `draft`.

## 3. Build waves

Put stories in the same wave only when they are dependency-independent and
conflict-free. Exclusive resources:

- the same source or test file
- the same shader or pipeline
- `CURRENT_SPRINT.md`, `BACKLOG.md`, `SPRINT_TRACKER.md`, `AS_BUILT.md`,
  `CHANGELOG.md`
- the same LLD section, when the edits are semantic
- **the GPU**, for any story running the oracle or a browser tier test

That last one is specific to this project. Two workers running WebGPU tests
concurrently on one machine contend for the device and produce timeouts that
read exactly like rendering failures. Serialise the oracle.

### Serial mode, chosen automatically

**A wave of one story runs serial, in the canonical worktree, with no claim, no
worker branch, no worktree, no handoff and no integration step.** So does every
wave when the resolved worker count is 1. There is no flag. Detect it and say
which mode each wave is in when reporting the plan.

Serial mode is the **simpler** path, not a degraded one.

## 4. Implement

PARALLEL wave:

```text
/claim-feature F-ID {agent} <worktree>       canonical worktree
/implement-feature F-ID                       worker worktree
/microscope F-ID --working                    REPEAT until 0 defects, 0 smells
/complete-feature F-ID --prepare
/integrate-feature F-ID work/f-xxx-<agent> --batch
```

SERIAL wave:

```text
/implement-feature F-ID                       canonical worktree
/microscope F-ID --working                    REPEAT until 0 defects, 0 smells
/complete-feature F-ID                        commits to the sprint branch
```

Report one line per review pass: F-ID, pass number, defect, smell and nitpick
counts. A loop that is not converging must be visible while it is happening.

**While a review runs, keep working.** Do read-only groundwork for a later
story: transcribe its HLD section, check its plan's factual claims against the
tree, measure the real write set. Do not idle.

A worker failure blocks only that story and its dependants. Continue other
waves.

## 5. Consolidated verification

After every branch is integrated:

```bash
bin/ocelli.sh gate --sprint
```

**Run the whole set, not the diff-selected subset.** The per-story checks were
scoped to one story's changes. This run is the first time the sprint's changes
are seen together, and the interactions are the point.

The sprint profile differs from `--all` in one state only. On S01, while F-010
is still pending in S02, the absent oracle is a named skip because S01 builds
the corpus the oracle consumes and contains no port code. `--all` remains
strict, and every later sprint must run the oracle.

Record it: `python3 scripts/verify_ledger.py record --profile sprint --gates
<what ran> --corpus <result>`.

## 6. The review loop

`/microscope` over the whole sprint diff, remediate, repeat, up to
`--max-review-passes`. **End the loop on substance, not on a count**, and keep
prose churn out of it: a remediation that corrects one sentence and adds three
explaining it ships three new claims for the next pass to falsify.

## 7. Finish

Push the sprint branch once, when verification and the review loop are clean.
Report the sprint state, every story's status, the gate results and the review
history. Then stop, and tell the operator that `/close-sprint SNN --next SMM`
is theirs to run.
