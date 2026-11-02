# Current sprint, S26

**Milestone**: M7, tool framework and geometry.
**Branch**: `sprint/s26`
**Opened**: 2026-11-02
**Goal**: Pointer, touch and wheel event normalisation and dispatch, cursor management and tool state machine.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-069 | E11.2 | Pointer, touch and wheel event normalisation and dispatch | Rust+TS | 3w | done |
| F-072 | E11.5 | Cursor management and tool state machine | Rust+TS | 3w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S26 / {print $2, $9}'
```

## What this sprint is

S25 closed at tag `s25`. The merge is `f1cd800`. F-068 and F-070 are done.
M6 stays complete at tag `s24`. This open does not publish. The workspace
version stays 0.1.0.

This sprint owns pointer, touch and wheel normalisation in the shell, and
the cursor plus the tool state events that sit on F-068's registry. F-071
is S27 and is not implemented here. F-073 and later annotation tools are
not this sprint.

F-069 and F-072 both depend on F-068. That row is `done`. They do not
depend on each other in the allocation. They share `packages/core/src/index.ts`
and the sprint ledgers, so the canonical worktree runs them one at a time.
F-069 lands first. F-072 does not import the pointer module.

Closing this sprint does not complete M7. M7 is S25 to S27, 5 stories
and 16 engineer-weeks. F-071 remains.

## What is carried in

- **M7 is open, and this open does not publish.** Tag `s25` points at
  merge `f1cd800`. No row in that close has a measured duration. No ratio
  is invented. A finished M7 would name 0.7.0. M1 through M6 were not
  published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. Neither S26 story
  depends on it.
- **The open LINEAR finding against `ocelli-pixel`'s own arithmetic** stays
  unscheduled. It has no F-ID. Clamping it so a readback matches is a
  tolerance change.
- **The SIGMOID file width stays on the product LUT.** File width 0.5 at
  centre 40 is about 4.586 under PS3.3 C.11.2.1.3.1. The pinned renderer
  presents recovered width 1.5, and the candidate passes that window into
  the same function. The product formula is not changed to settle it.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on
  this host through headless Chromium WebGL2 and matched `LutChain::map_into`
  within 0.001, format `R32Float`. A tier B colour LUT readback was not run.
- **The wasm size budget records 2,151,196 bytes** with tolerance 0.05.
  The ceiling is 2,258,755. The S25 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row this close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A pointer sample that becomes a camera command the core will store as
an absolute value, and a cursor that still names the tool which just lost
the binding.** `set_pan`, `set_zoom` and `set_rotation` replace. `scroll_by`
adds. A dispatcher that packs the pointer's client position as `Pan`, or
that sends one wasm call per move, is the quietly wrong result. A cursor
write that does not follow registration order when two tools are active
on different buttons is the same class in the shell.

## What done means

- **F-069** turns pointer, touch and wheel input into the F-103 shell
  event names and at most one packed command buffer per `takeFrame`.
  It does not call wasm. `Pan`, `Zoom` and `Rotate` are packed from a
  caller basis captured for the gesture. `Scroll` is packed from the
  wheel or from a drag whose command is `Scroll`. The other seven
  commands are not given a payload this story does not have.
- **One contact owns the packed gesture.** A second pointer id does not
  pack a second command.
- **F-072** emits `tool.activated` and `tool.modeChanged` from a wrapper
  around the F-068 registry, and sets one cursor string from the active
  tool. It does not listen for DOM events.
- **Each story declares all three tier rows.** Both are shell state, so
  the rows are n/a. An omitted row is not an answer.

## Dependency order

F-069 depends on F-068. F-072 depends on F-068. F-068 is `done`. F-069
lands before F-072 because the sprint runs serial in the canonical
worktree. F-071 waits for F-069 and is S27. F-X003 stays `in-progress`
and is not a dependency.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is
pinned exactly and agents confidently emit APIs that have not existed for
two years**, so every wgpu call is checked against the pinned version's own
documentation rather than from memory. Treat GPU code that compiles first
try with suspicion.

No patient data enters a prompt, tracked file, fixture, log, error or
commit. The ignored corpus remains behind `corpus/manifest.tsv` and its
generators.

A feature that cannot run on the resolved tier **reports unavailable**. It
never quietly produces a different result, which is HLD section 31's rule
generalised by D-07.

## Audit intake, 2026-11-02

S25 closed at tag `s25`. The merge is `f1cd800`. M7 stays open and
unpublished. The S26 allocation rows and the S26 plan table name F-069
and F-072. F-069 is done. F-072 is done. F-068 is done. F-071 is not this sprint.
F-X003 stays in progress. F-X011 remains pending.
