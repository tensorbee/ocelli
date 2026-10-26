---
description: Implement one F-ID from its approved design plan, with the focused checks and the mandatory risk check for what it touched.
---

# /implement-feature F-XXX

Implement the story from `.claude/plans/F-XXX-design.md`. The plan is already
approved. **Do not redesign it.**

## The plan is authoritative on decisions and verifiable on facts

Implement the plan's DECISIONS without relitigating them. Treat its FACTUAL
claims about the tree as checkable, because plans are written from a reading of
the code and readings go stale.

Where the tree contradicts the plan: implement everything else, adapt with the
plan's intent, and **report the contradiction precisely**. Never silently
substitute a different approach. A plan that said a function had no callers and
was wrong is a fact to report, not a reason to redesign in place.

## 1. Transcribe before writing

If the plan's `## Normative source, transcribed` section is thin, stop and
fill it from `docs/hld/` before writing code. This is not bureaucracy. The
formulas in this project differ from their near neighbours by a half and a one,
and the difference is invisible in a screenshot.

## 2. Order of work

**Fixtures first, then the code they judge.** Not the other way round.

HLD 27.2 R2: an agent asked to test a function will assert what it does, not
what it should do. A test written after the implementation, by the same
process that wrote the implementation, asserts the implementation is itself.
The four rows of the section 18.3 table exist to be typed in from the
specification before the shader is written, and the same discipline applies to
every arithmetic function in the project.

So:

1. Write the fixture, with its expected values taken from the DICOM section or
   hand-computed and shown working in a comment.
2. Watch it fail.
3. Write the code.
4. Watch it pass.
5. **Mutate one constant in the code, re-run, confirm the test goes red**, and
   revert. A test that passes both ways is not a test. HLD 27.3 asks a human
   to check exactly this, so do not make them find it.

## Acceptance execution record

Before editing, list the plan's acceptance rows with their tests and stages.
Keep the current trigger, expected result and old-source identity beside each
regression. Observe semantic RED before the fix. A compiler rejecting an API
that does not exist yet proves the scaffold is absent, not the intended bug.
After the fix retain GREEN for the same case and inputs, then run the planned
mutation/negative control. Keep failures and environment limitations visible.

Follow the plan's consumer map through every affected entry point. A shared
constructor does not guarantee that callers apply equivalent refusal policy,
units, defaults or context. Test combinations chosen by the design, including
unchanged consumers and lifecycle transitions, before broad verification.
Do not add a Cartesian product or assertion that merely repeats the code.

If a dependency or criterion proves unavailable, implement independent work
and record the exact unmet criterion and owner. Do not mark a whole story done
because its easy subset passes. Proposed patches from an audit are starting
material to inspect and reproduce, not accepted implementation evidence.

Record each acceptance row as evidenced, failed, implementation missing or
external observation unavailable, with its next action and owner. Missing
canvas wiring, a candidate producer or a runner body is implementation work.
Do not describe it solely as a missing host or adapter. A runner that always
throws proves refusal handling, not readiness to measure on an equipped host.

For each missing implementation, name the owning function or module, the
smallest next deliverable and its discriminating check in the existing
acceptance record. Separately name any missing collector configuration and
external host. At handoff, choose the next dependency-ready implementation
step within the authorized scope. A repair-only assignment may stop at its
declared boundary, but its handoff must distinguish that boundary from a
technical blocker. Do not repeatedly hand back "keep open" as the next action
for work that can be implemented. Do not start unapproved scope to fill a gap.

For a new wasm boundary, execute the generated module with the public shell
caller and its adapter. Test alloc returning 0 before any write, release and
reallocation at the same address, malformed and duplicate calls, and memory
growth. Host-only sessions remain a separate evidence column. For rendering,
test the first successful settled draw followed by a successful command, new
source and resize. Verify updated pixels and event identity as applicable.

For each repaired lifecycle, replay the original reproducer, its valid
control and the next failure boundary through the public caller. Include an
identical repeated request and rejected configuration. Inspect the external
sink as well as cached state. A bitmap setter can clear visible content even
when the stored dimensions do not change. Preserve pixels, ownership and
generation until the declared commit point, or retain a correct retry path.

A review handoff names the complete build/test command chain for each target
artifact and when it was rebuilt. Keep explicit suites outside default npm
test in that list. Link each changed acceptance row to its permanent proof
and state whether it covers an internal helper or the public production path.

## 3. Rules that bite in this codebase

- **`as` casts.** `cast_possible_truncation`, `cast_precision_loss` and
  `cast_sign_loss` are denied. Do not `#[allow]` one to move on. Every
  conversion is a deliberate, visible choice, and if a conversion genuinely is
  safe, say why in one line at the site.
- **`unwrap` and `expect` are denied.** A panic poisons the wasm instance
  (HLD section 23). There is no "this cannot fail" in an exported path.
- **No allocation in the render loop.** Pre-size at viewport creation.
- **Decode into caller-provided buffers**, `fn decode(&self, src: &[u8], out:
  &mut [u8])`, never a `Vec` returned per frame.
- **`bytemuck::cast_slice`** for reinterpreting pixel buffers. A hand-written
  transmute is `unsafe` with no upside, and `unsafe` is allow-listed to two
  files that are not this one.
- **Never build a view over wasm linear memory outside
  `packages/core/src/bulk.ts` and `packages/core/src/panic.ts`.** ESLint
  refuses it, through `ALLOWED_TO_DISABLE`, whose second list also exempts the
  tests. The failure mode is silent.
- **Do not read dwv, Horos or Grok**, and do not fetch a URL belonging to
  any of them.
- **A new refusal arrives with its probe.** Every refusal under `scripts/`,
  `ci/`, `.githooks/`, `bin/` and `tools/` is claimed by an entry in
  `scripts/guards/catalogue.py` naming the citation it enforces and how to
  drive it red. `bin/ocelli.sh gate guards` refuses a refusal no entry claims,
  and it is in the floor, so the push that adds one without an entry is the
  push that goes red. This sentence is a courtesy. The census is the mechanism.

## 4. Checks

Run the focused checks for what changed, then the plan's mandatory risk check:

```bash
bin/ocelli.sh check <crate>
bin/ocelli.sh test  <crate>
bin/ocelli.sh clippy <crate>
```

Plus, selected by what the diff touches:

| The diff touches | Also run |
|------------------|----------|
| any crate manifest or dependency | `bin/ocelli.sh gate bindgen pins` |
| any `unsafe`, or a new `.rs` file | `bin/ocelli.sh gate unsafe` |
| a shader, a pipeline or a tier decision | `bin/ocelli.sh gate wasm`, exercise applicable A/B/C paths and verify declared unavailable behavior |
| pixel or geometry arithmetic | `bin/ocelli.sh gate oracle` |
| `packages/` or `examples/` | `npm run lint && npm run typecheck` |
| `docs/hld/` or sprint planning data | `bin/ocelli.sh gate backlog deviations` |
| any tracked prose | `bin/ocelli.sh gate prose` |

**Read every exit code from the command itself, never from the end of a pipe.**
`cmd | tail -2; echo $?` reports `tail`'s status.

## 5. When a gate fails

Fix the code. **Do not:**

- widen a tolerance,
- add an `#[allow]`,
- add a path to the `unsafe` allow-list,
- add a file to the ESLint exception,
- disable a gate,

to make it pass. Each of those is a design-plan decision with a recorded
rationale, and four of the five are also changes to `.claude/WORKFLOW.md`.

If a gate is red before your change, prove it rather than assuming it: check
out the sprint base in a throwaway worktree and run the same gate there,
comparing failures by name.

## 6. Progress notes

Keep `.claude/scratch/F-XXX-progress.md` current: the exact last green command,
current failures, changed areas, next action. Before handing off, that file is
what the next agent reads first.

The handoff names full base/HEAD and staged tree when applicable, staged and
unstaged paths, the ACs implemented and still unmet, and exact evidence paths.
Keep semantic RED, repaired GREEN and mutation results individually with
source/input identities and own exit statuses. State which checks ran now and
which are prior receipts. Name independent-review status separately from
implementation status. Never leave an old "fix not implemented" handoff as
the only current account after implementing the repair. Preserve the earlier
attempt as history and add the current disposition.

## Finish

Hand to `/microscope F-XXX --working`, and repeat that loop until a pass
reports zero defects and zero smells. Then `/verify`, then
`/complete-feature F-XXX`.
