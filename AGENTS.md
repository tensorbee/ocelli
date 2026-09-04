# Repository guidance for coding agents

One development workflow across Claude Code, Codex and human contributors.
Before changing code or tracked documentation, read `CLAUDE.md` and
`.claude/WORKFLOW.md`. The workflow file wins on process questions.
`docs/hld/` wins on what to build.

## Shared state

- `docs/sprints/CURRENT_SPRINT.md`, `BACKLOG.md`, `SPRINT_PLAN.md`,
  `SPRINT_TRACKER.md` and `AS_BUILT.md` are the shared delivery record.
- `.claude/plans/` and `.claude/reviews/` are agent-neutral tracked artefacts
  despite the directory name.
- `.claude/scratch/F-XXX-progress.md` is the local handoff record. Read it when
  resuming an in-progress F-ID and update it before handing work to another
  agent.
- **Never create a second sprint tracker, backlog, design plan or review record
  for Codex.** One set of files, both hosts.

## Non-negotiable rules

- **No patient data** in prompts, source, fixtures, logs, errors, documentation
  or commits. The corpus is outside git behind `corpus/manifest.tsv`.
- **Do not open dwv or Horos**, and do not depend on Grok. Reading a copyleft
  source and translating it into Rust is a translation, an exclusive right of
  the copyright holder, and agent exposure cannot be disproved after the fact.
  Take the ideas from DICOM PS3.3 and PS3.16.
- **No `unsafe`** outside `crates/ocelli-wasm/src/ring.rs` and
  `crates/ocelli-core/src/cast.rs`.
- **`wasm-bindgen` only in `ocelli-wasm`.**
- **Do not change a tolerance to make a test pass.** That is a design-plan
  decision reviewed like code.
- **Do not disable a gate to make a build green.** That is a change to
  `.claude/WORKFLOW.md`.
- No em-dash and no prose semicolon in enforced Markdown or commit messages.
- **Do not commit or push unless the invoked workflow explicitly includes that
  action.** Never add an agent co-author trailer. The provenance trailers
  `Ocelli-Verify` and `Ocelli-Generated-By` are written by the pre-commit hook
  from the verify ledger, never by hand.

## Commands

```bash
bin/ocelli.sh check <crate>          # cargo check -p <crate> --all-targets
bin/ocelli.sh test  <crate> [args]   # cargo test -p <crate>
bin/ocelli.sh clippy [crate]         # workspace form, or -p <crate>
bin/ocelli.sh fmt
bin/ocelli.sh cargo <anything>       # raw passthrough

bin/ocelli.sh wasm                   # wasm-pack build + size budget
bin/ocelli.sh native                 # the cross-target proof (E1.7)
bin/ocelli.sh oracle                 # differential harness, needs a GPU
bin/ocelli.sh corpus                 # verify the corpus against its manifest

bin/ocelli.sh gate --list            # what each gate covers
bin/ocelli.sh gate --floor           # what CI runs
bin/ocelli.sh gate --all             # everything

npm run lint | typecheck | test | dev
```

**Rules for agents:**

- Default to `bin/ocelli.sh check <crate>` while iterating, not `build`.
- Everything runs natively. There is no container, because Ocelli needs a real
  GPU for WebGPU and a browser for the oracle.
- **Read every exit code from the command itself, never from the end of a
  pipe.** `cmd | tail -2; echo $?` reports `tail`'s status, and that has hidden
  real clippy failures inside runs reported as passing. In zsh, which is the
  shell here, `${PIPESTATUS[0]}` is **empty** and `${pipestatus[1]}` is the one
  you want, so the bash idiom silently reports nothing at all.
- **Stage before you gate, and gate before you record.** Several guards read
  `git ls-files`, and `scripts/verify_ledger.py` keys on `git write-tree`,
  which hashes the **index**. Both are blind to an untracked file. Running them
  over new work that has not been staged checks the previous content and says
  so in the language of success. Measured in S01: staging first moved `unsafe`
  from 14 files to 23, `provenance` from 191 to 205 and `prose` from 44 to 49,
  and the first ledger record certified a tree byte-identical to the base
  commit's. The order is write, stage, gate, record, commit.
- **`git add -N` does not count as staging for any of this.** An intent-to-add
  entry shows in `git status` as `A`, and shows in
  `git diff --cached --name-only` as nothing at all, which is what every
  `--staged` guard reads. So `bin/ocelli.sh gate content` over an `add -N`
  tree reports `OK: no patient data or build artefacts staged` while a DICOM
  sits in it. Measured. **This is not a hole in the hook**, because git itself
  refuses to commit an intent-to-add-only path and `git commit -a` updates the
  index before the hook runs, so nothing lands unchecked. It is a hole in the
  evidence: the guard answered a question about an empty set and said so in the
  language of success. Use `git add` when you want to be checked.
- **Set `OCELLI_AGENT`, or pass `--agent`, before recording a verification.**
  `verify_ledger.py` defaults it to `unknown`, and the commit-msg hook then
  writes `Ocelli-Generated-By: unknown`, which is a provenance trailer that
  records no provenance. HLD 27.2 R6 wants the generator named.
- A release build is the only meaningful one for size. HLD section 15.2's
  profile applies to release only, so a dev-profile measurement against the
  size budget is meaningless.

## Structural rules

The codebase must stay easy to reason about locally. A reader should answer
"what does this do?" from one file.

**The test for any new construct:** does it reduce the number of cases a reader
must consider, or increase the number of places they must look?

Reducing cases is good even when it adds types. `Pt<Canvas>` and `Pt<World>`
being non-interchangeable is the clearest example in this project, and HLD
section 16 says why: cornerstone represents canvas points, world points and
voxel indices all as `number[]`, and mixing them is a silent, common and
expensive bug. Prefer these and add more of them.

Increasing places to look is not:

- No new trait unless two implementers exist **today**.
- No new generic parameter unless instantiated two ways **today**.
- No `Box<dyn>` or `Arc<dyn>` where the concrete type is statically known.
- No wrapper that only forwards.
- No feature flag without a named user.

Two deliberate exceptions, both from the HLD: the `Decoder` trait
(section 21) and the `SeriesSource` and render-target traits (section 13) exist
with one implementer each, because they are the declared extension points that
make Phases 2 and 3 entry points rather than rewrites.

## Performance rules

From `docs/hld/23-performance-rules.md`. These are not general advice, they are
this project's hot path.

- No allocation in the render loop. Pre-size at viewport creation.
- One `queue.submit()` per frame across all viewports.
- Prefer a uniform update to a texture update. Window/level is thirty-two
  bytes, not a re-upload.
- Batch pointer events into one command buffer per animation frame. Never cross
  the boundary per event.
- **Measure before optimising. The intuitions that work in JavaScript do not
  transfer.**

## Skills

Skills under `.agents/skills/` are generated from `.claude/commands/` by
`scripts/sync_agent_skills.py`. Do not edit a generated adapter. Change the
canonical command file and re-run the sync, then `--check`.
