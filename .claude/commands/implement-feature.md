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
  `packages/core/src/bulk.ts`.** ESLint refuses it. The failure mode is silent.
- **Do not read dwv or Horos**, and do not fetch a URL belonging to either.

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
| a shader, a pipeline or a tier decision | `bin/ocelli.sh gate wasm` and exercise BOTH tiers |
| pixel or geometry arithmetic | `bin/ocelli.sh gate oracle` |
| `packages/` or `examples/` | `npm run lint && npm run typecheck` |
| `docs/hld/` or the spreadsheet | `bin/ocelli.sh gate docs` |
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

## Finish

Hand to `/microscope F-XXX --working`, and repeat that loop until a pass
reports zero defects and zero smells. Then `/verify`, then
`/complete-feature F-XXX`.
