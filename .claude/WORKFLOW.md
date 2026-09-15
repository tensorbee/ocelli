# The Ocelli development workflow

This document is the canonical reference for how work lands in this codebase.
It is opinionated and load-bearing. **When in doubt, this file wins over any
other document except `docs/hld/`**, which is normative on what to build.
This file is normative on how.

It works identically under Claude Code and Codex. The `.claude` directory name
is historical, not an ownership marker: plans, reviews, commands and handoffs
are agent-neutral tracked artefacts. Codex loads `AGENTS.md` and the generated
adapters under `.agents/skills/`.

## The one thing this project is actually about

**The dangerous defect in medical imaging is not the crash, it is the pixel
that is quietly wrong** (`docs/hld/12-workspace-and-build.md`, Part II
preamble). Everything below is shaped by that. A green test suite is not
evidence, a plausible screenshot is not evidence, and an agent's confidence is
not evidence. The oracle is evidence, and a hand-computed fixture citing a
DICOM section is evidence.

The HLD's own worked example: at the centre of a soft-tissue CT window, VOI
LINEAR and LINEAR_EXACT differ by 0.32 of 255. **That is invisible to a human
comparing screenshots and immediately visible to a pixel diff.** It is the
whole argument for building the oracle before the code it validates.

## Normal sprint automation

```text
1. /run-sprint                     Design every unfinished F-ID first
2. answer consolidated questions   One decision round, then autonomy
3. autonomous execution            Safe parallel waves, or serial at width 1
4. consolidated verification       Impacted crates once, plus triggered gates
5. sprint-review loop              Fix and repeat until substance is clean
6. /close-sprint SNN --next SMM    Merge, tag, next sprint
```

`/run-sprint` stops after a clean sprint review. It never merges to `main`,
never creates a tag and never publishes. **It does not stop before that.**
Ending a turn with sprint work outstanding and no blocking question is a defect
in the run, not a checkpoint. An operator typing "keep going" means the rule
was broken.

Readiness is evidence about one exact tree, not a durable green flag. Each
whole-sprint review records its counts and `git write-tree`. Each sprint-profile
verification records the same identity. `close-preflight` accepts only a clean
latest review and a passing latest verification for the current HEAD tree, with
no staged, unstaged or untracked changes. Any remediation therefore requires a
new verification and a new whole-sprint review.

A story that cannot finish for an external or dependency reason is `carried`,
not `completed`. The close preflight accepts that state only when the story has
a non-empty reason under `## Carried forward from SNN` in
`CURRENT_SPRINT.md`. Carried stories do not require a feature review because no
implementation is being claimed.

## Atomic feature rhythm

```text
1. /design F-XXX             Write the design plan against docs/hld/
2. /start-feature F-XXX      Mark in-progress, create test stubs
3. /implement-feature F-XXX  Code + tests
4. /microscope F-XXX         REPEAT until zero defects and zero smells
5. /verify                   The gate
6. /complete-feature F-XXX   Ledgers, LLD, commit with the provenance trailer
```

## Where things live

| Artefact | Location | Written by | Lifetime |
|----------|----------|-----------|----------|
| Specification | `docs/hld/*.md` | Hand, through a reviewed design plan | Permanent |
| Deviations from it | `docs/hld/DEVIATIONS.md` | Hand, via a design plan | Permanent |
| Backlog status | `docs/sprints/BACKLOG.md` | `/complete-feature`, `/sync-status` | Live |
| Sprint roadmap | `docs/sprints/SPRINT_PLAN.md` | Hand-curated | Live |
| Sprint allocation | `docs/sprints/allocation.json` | Hand, with backlog and plan checks | Live |
| Active sprint | `docs/sprints/CURRENT_SPRINT.md` | `/sync-sprint SNN` | Per sprint |
| Velocity log | `docs/sprints/SPRINT_TRACKER.md` | `/complete-feature` | Append |
| Completion log | `docs/sprints/AS_BUILT.md` | `/complete-feature` | Append-only |
| Release notes | `CHANGELOG.md` | `/release-notes` | Append |
| Design plans | `.claude/plans/F-XXX-design.md` | `/design` | Tracked |
| Review findings | `.claude/reviews/F-XXX-<aspect>-pass-N.md` | `/microscope` | Tracked |
| Sprint run state | `.claude/scratch/SNN-run.json` | `scripts/sprint_workflow.py` | Gitignored |
| In-flight notes | `.claude/scratch/F-XXX-progress.md` | Hand, between sessions | Gitignored |
| Verify evidence | `.claude/verify-ledger.json` | `/verify` | Gitignored |
| Parallel handoff | `.claude/handoffs/F-XXX-ready.md` | `/complete-feature --prepare` | Temporary, tracked |
| Living architecture | `docs/lld/*.md` | `/complete-feature` step 9 | Evolves |
| Corpus manifest | `corpus/manifest.tsv` | `scripts/corpus_check.py --add` | Live |
| Codex adapters | `.agents/skills/` | `scripts/sync_agent_skills.py` | Generated, tracked |

## The gate

`/verify` is the completion gate. `bin/ocelli.sh gate --list` shows what each
gate covers. Four profiles:

- **`--floor`**, everything needing no GPU and no corpus. This is what CI runs.
- **`--sprint`**, everything in `--all`. It carried one bootstrap exception,
  which was named here, implemented in `bin/ocelli.sh` as `s01_pre_oracle`, and
  applied only on S01 while F-010 remained pending: the absent oracle was a
  named skip, because S01 built the corpus the oracle needs and contained no
  port code. **F-010 built the oracle, so the exception is gone.** The function
  was removed rather than left with a condition that can no longer be true,
  because a dead exception is a live misreading. The two profiles are now the
  same set of gates.
- **`--all`**, the floor plus every gate outside it. This is what a human runs
  before a release.
- **named gates**, the inner loop.

**No profile's gate list is written out here**, and that is deliberate.
`bin/ocelli.sh gate --list` prints every gate with its GPU column, and the
`--floor` exclusion list in `bin/ocelli.sh` is compared for set equality against
`scripts/ci_floor_check.py`'s `NOT_IN_FLOOR` by the `ci` gate, so the two
mechanisms cannot drift. A third copy in this file can, and the line above said
"the floor plus `corpus` and `oracle`" from S02 until S11 added a second GPU
gate, at which point it was wrong without anything noticing.

**There are now two gates that need a GPU**, and `oracle` is no longer the only
one. `gpu` was added by F-037 and runs every `#[ignore]`d test in
`ocelli-render`. That is the whole definition of the set, and it is
deliberately not glossed as "the ones needing an adapter": most of them do, and
at least one is ignored because its MUTATION only dies where adapters exist
rather than because it needs one to run. It exists because
`cargo test --workspace` is the `test` gate and carries `needs_gpu = no`, so an
`#[ignore]`d test ran in **no profile at all**, including `--sprint`. A story
whose evidence is a GPU comparison would otherwise have closed green with that
evidence checked by nothing.

**One of the tests it sweeps up asserts nothing**, and the gate's own comment
says so rather than letting the count read as coverage:
`probe::tests::measures_a_fill_rate_on_this_machine` prints a fill rate for a
human recording a band in `ci/tier-thresholds.json`. It is not nothing, because
it drives `resolve` end to end on a real adapter and a panic there fails the
gate, and it is not evidence of correctness either.

The gate runs `--test-threads=1`, because the adapter is an exclusive resource.
`docs/sprints/CURRENT_SPRINT.md` says two workers running device tests
concurrently on one machine contend for the adapter and produce timeouts that
read exactly like rendering failures, and a gate opening several devices at once
is the same contention inside one process.

`gpu` is excluded from `--floor` under deviation **D-04** for exactly the reason
`oracle` is, and it is an ADDITION to the set CI does not run rather than a
weakening of anything CI does. A machine with no adapter resolves tier C, and
the tests there report that they did not apply and pass, which is D-07's honesty
rule rather than a gate pretending to have run.

### Everything runs natively, and there is no container path

Ocelli needs wasm32 builds, a real GPU for WebGPU and a browser for the
oracle. A container gives it none of those. `bin/ocelli.sh` is the wrapper.

### CI runs no GPU and no corpus, and what replaces it

This is deviation **D-04**, recorded in `docs/hld/DEVIATIONS.md`, and it is a
real weakening of the HLD's design. HLD section 11 says every pull request
renders the corpus in CI, and decision D7 calls the oracle the reason
generation speed is an advantage rather than a liability.

Three mechanisms carry the load instead, and all three are mechanical:

1. **`/verify` records its result against the TREE hash** in
   `.claude/verify-ledger.json`.
2. **`.githooks/pre-commit` reads that ledger** for the staged tree and appends
   the `Ocelli-Verify` trailer it finds there. A trailer cannot be hand-written,
   because the hook only emits one a real gate run recorded.
3. **CI re-reads the trailer** on the pushed head with
   `scripts/verify_ledger.py check-commit`, asserts the trailer's tree matches
   the commit's tree, and fails on a red or missing corpus record. This costs
   no GPU and cannot be satisfied by intention.

`/release` additionally requires `--require-corpus`, so a release is the one
moment the GPU tier is not optional.

## Voice rules

Enforced by `scripts/prose_check.py` over `.claude/plans/`, `.claude/reviews/`,
`.claude/commands/`, `docs/sprints/`, `docs/lld/`, the root markdown, and
commit messages.

- **No em-dash.** Use a hyphen, a comma, or rewrite the sentence.
- **No prose semicolon.** Use a full stop or a comma.

**`docs/hld/` is exempt.** Those files are cut from the author's document with
nothing reworded, and editing a specification to satisfy a lint is the wrong
way round.

**`CHANGELOG.md` is not covered at all**, and that is worth stating precisely,
because the obvious reading of the sentence above is wrong. It is not that
released sections are exempt and unreleased ones are checked. The file is
absent from the checker's include list entirely, so nothing in it is checked,
`## Unreleased` included. S01 found a stale gate count sitting there for
exactly that reason. If the unreleased section should be covered, that needs
section-aware logic in `scripts/prose_check.py`, and it is a change to this
file rather than a one-line include.

## Source provenance, and the three projects nobody may open

This is `docs/SOURCE-POLICY.md` and it is sharper here than in an ordinary project:

> Translating source into Rust is a translation, which is an exclusive right of
> the copyright holder, so a copyleft licence blocks **reading**, not merely
> depending. Agent-assisted development sharpens this: exposure cannot be shown
> to be absent after the fact.

**dwv**, **Horos** and **Grok** must not be opened by a person or an agent
on this project, and **Grok** must not be depended on either.
`scripts/source_provenance_check.py` enforces all three. Where their ideas are worth
having, take them from the standard. dwv's annotations-as-SR is in DICOM PS3.3
and PS3.16, which is where dwv took it from.

In bounds to read and to depend on: cornerstone3D, dicom-rs, wgpu, VTK, ITK,
elastix, DCMTK, OpenJPEG, CharLS, OpenJPH, BlueLight,
dicom-microscopy-viewer, NiiVue, Neuroglancer.

`scripts/source_provenance_check.py` enforces this over every tracked text
file and every manifest. There is no allowlist.

## No patient data, ever

The corpus lives under ignored `corpus/data` behind a committed manifest
(`corpus/README.md`). `.githooks/pre-commit` refuses a staged DICOM by magic
bytes as well as by suffix, because `anon001` with no extension is a very
normal way to receive one. There is no allowlist and no escape hatch.

A set labelled de-identified can still carry burned-in pixel annotation, which
is why HLD story E22.3 exists. A repository that never contains DICOM cannot
leak one.

## Test taxonomy

Each design plan picks the applicable categories.

| Category | Proves | Where |
|----------|--------|-------|
| `unit` | Pure logic, no I/O | `crates/<crate>/src/*.rs` under `#[cfg(test)]` |
| `fixture` | Pixel and geometry arithmetic against hand-computed values citing a DICOM section | `crates/<crate>/tests/` |
| `property` | Round-trips within epsilon, e.g. canvas to world to canvas | `crates/<crate>/tests/` |
| `golden` | The rendered frame matches cornerstone3D over the corpus | `tools/oracle/` |
| `conformance` | Each transfer syntax decodes correctly | Published DICOM test corpora |
| `browser` | The boundary, workers and tiers under a real engine | `tools/oracle/`, Playwright |

**`fixture` is not optional for pixel arithmetic** (HLD 27.2 R3), and R2 says
why the others do not substitute: an agent asked to test a function will assert
what it does, not what it should do. The fixture derives from the specification
or the oracle, never from reading the implementation.

## Tolerance policy

Written down once, in `docs/hld/22-testing-and-tolerance.md` section 25.1, and
held. **A tolerance change is a pull request with a rationale, reviewed like
code.** Tuning tolerance per failure is how a suite stops meaning anything.

## Git workflow

Per-sprint branches off `main`, named `sprint/sNN`. Every F-ID commit lands on
the active sprint branch, never directly on `main`.

Commit message format, written by `/complete-feature`:

```text
F-XXX, {short title}

{one paragraph: what was built, why, and any non-obvious choices}

Tests, {summary}
HLD, {sections implemented}
Deviations, {D-NN rows, or "none"}

Ocelli-Verify: profile=... gates=... corpus=... tree=...
Ocelli-Generated-By: {agent}
```

The two trailers are HLD section 27.2 R6, the provenance requirement. They are
written by the pre-commit hook from the verify ledger, not by hand. **No agent
co-author trailer.**

## Parallel work

The normal mode is one active writer on `sprint/sNN`. When two agents work
concurrently:

1. The integrator keeps the canonical sprint worktree.
2. Workers use `work/<fid-lower>-<agent>` branches in separate worktrees, with
   the F-ID hyphenated exactly as written, so F-094 is `work/f-094-claude`.
3. `/claim-feature` before `/start-feature --claimed`.
4. Workers never edit sprint totals and never push the sprint branch.
5. `/complete-feature --prepare` writes a handoff, `/integrate-feature`
   consumes and removes it.
6. **Integrate with a three-way merge against the worktree's base commit,
   never a hand-picked file list.** A worker branch based before other stories
   landed cannot be merged by listing its files, and the failure is silent.

**A wave of one story runs serial**, in the canonical worktree, with no claim,
worker branch, worktree, handoff or integration step. The claim machinery
exists only to make concurrent work safe. At width one it protects nothing and
carries its own failure modes.

## Escalation triggers

| Signal | Response |
|--------|----------|
| F-ID consistently over 2x its estimate | Split into F-XXXa/b, update the backlog |
| The same review finding survives three passes | Surface to the operator, do not stop the loop |
| Corpus divergence with no recent code change | The corpus or cornerstone3D moved. Check digests before code |
| A tolerance needs widening to pass | Stop. This is a design-plan decision, not a fix |
| A feature works on one tier and differs on another | Stop. It reports unavailable, it does not quietly differ. D-07 |
| An `as` cast is added to make types line up | Stop. HLD 27.3 makes every cast a human review item |
| Sprint has no completion after four weeks | Replan, the scope or the split is wrong |
| A gate is disabled to get a build green | Stop. Disabling a gate is a WORKFLOW.md change |

## When this file changes

1. Note the trigger in the next AS_BUILT entry's "Notes for future sessions".
2. Update this file.
3. Update the affected `.claude/commands/*.md`.
4. Run `python3 scripts/sync_agent_skills.py` and then `--check`.
5. Commit the workflow change **separately** from any feature change, so the
   rationale is visible in `git log`.

Treat changes here like ADRs.
