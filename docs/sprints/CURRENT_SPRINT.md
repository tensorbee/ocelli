# Current sprint, S25

**Milestone**: M7, tool framework and geometry.
**Branch**: `sprint/s25`
**Opened**: 2026-11-01
**Goal**: Tool lifecycle, tool groups, bindings and modes, ocelli-geom: hit-testing, projection and measurement mathematics.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-068 | E11.1 | Tool lifecycle, tool groups, bindings and modes | Rust+TS | 3w | pending |
| F-070 | E11.3 | ocelli-geom: hit-testing, projection and measurement mathematics | Rust+TS | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S25 / {print $2, $9}'
```

## What this sprint is

S24 closed at tag `s24`. The merge is `71ac28d`. M6 is complete. M6 is
S22 to S24, 5 stories and 16 engineer-weeks. This open does not publish.
The workspace version stays 0.1.0.

This sprint is the first M7 sprint. It owns tool lifecycle state in the
shell and the hit-testing, projection and measurement functions in
`ocelli-geom`. F-069, F-071 and F-072 are later sprints. They are not
implemented here.

F-068 depends on F-100. F-070 depends on F-048. Both of those backlog
rows are `done`. The two S25 stories do not depend on each other.

Closing this sprint does not complete M7. M7 is S25 to S27, 5 stories
and 16 engineer-weeks.

## What is carried in

- **M6 is complete, and this open does not publish.** Tag `s24` points at
  merge `71ac28d`. No row in that close has a measured duration. No ratio
  is invented. A finished M6 would name 0.6.0. M1 through M5 were not
  published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. Neither S25 story
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
  The ceiling is 2,258,755. The S24 sprint gate passed that budget. The
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

**Two tools quietly owning one gesture, and a length that mixes a canvas
pixel with a patient millimetre.** Tool state lives in the shell. The
numbers live in `ocelli-geom` on `Pt<S>`, so a canvas point and a world
point do not type-check as the same argument. A mode change that leaves
the previous owner active is the interaction form of the quietly wrong
result. A constant segment parameter is the measurement form.

## What done means

- **F-068** is a caller-owned registry. A group holds tools, modes and
  pointer bindings, and the viewports attached to that group. It does not
  listen for events and it does not draw.
- **One active owner per pointer binding inside a group.** Taking a binding
  moves the previous owner to passive and clears that owner's bindings.
  The move is in the return value.
- **A refused change leaves the group untouched.**
- **F-070** is the distance, the polyline length, the angle, the segment
  projection, the segment hit and the plane projection. The formulas are
  the local rules in the design plan. The HLD names the crate and does
  not give the formulas.
- **No second copy of that arithmetic in TypeScript.** Camera mapping
  stays the F-048 transforms. This sprint does not add a wasm export.
- **Each story declares all three tier rows.** Both stories are pure
  state or pure arithmetic, so the rows are n/a. An omitted row is not
  an answer.

## Dependency order

F-068 depends on F-100. F-070 depends on F-048. Both are `done`. F-068
lands before F-070 because the sprint runs serial in the canonical
worktree. F-069 and F-072 wait for F-068 and are S26. F-071 waits for
F-069 and is S27. F-X003 stays `in-progress` and is not a dependency.

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

## Audit intake, 2026-11-01

S24 closed at tag `s24`. The merge is `71ac28d`. M6 is complete and
unpublished. The S25 allocation rows and the S25 plan table name F-068
and F-070. Both backlog rows are pending. F-100 and F-048 are done.
F-069, F-071 and F-072 are not this sprint. F-X003 stays in progress.
F-X011 remains pending.
