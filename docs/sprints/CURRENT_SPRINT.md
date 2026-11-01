# Current sprint, S23

**Milestone**: M6, segmentation rendering.
**Branch**: `sprint/s23`
**Opened**: 2026-11-01
**Goal**: Contour representation, surface representation, and the segmentation state model with a per-viewport representation registry.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-064 | E10.2 | Contour representation | Rust | 4w | done |
| F-065 | E10.3 | Surface representation | Rust | 3w | pending |
| F-066 | E10.4 | Segmentation state model and per-viewport representation registry | Rust | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S23 / {print $2, $9}'
```

## What this sprint is

S22 closed at tag `s22`. The merge is `33ccb34`. M6 started in S22 and is
not complete. This sprint adds the contour, the surface, and the registry
that says which representation a viewport shows. The S23 table in
`docs/sprints/SPRINT_PLAN.md` and the S23 rows in
`docs/sprints/allocation.json` name F-064, F-065 and F-066.

`ocelli-seg` already presents the labelmap. Index 0 stays transparent.
Fill writes the segment colour and the outline is the one-pixel
4-connected boundary. Contour and surface are not that presenter. The
registry does not invent a second colour path. Labelmap edge projection
stays the uncompiled blend. That blend is F-067 in S24.

F-064, F-065 and F-066 each depend on F-063. That backlog row is `done`.
They do not depend on each other. F-067 depends on F-064 and stays in S24.

This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. None of F-064, F-065
  or F-066 depends on it.
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
  F-050 measured the release module on 2026-10-27 and re-baselined
  `ci/wasm-size-budget.json` from 1,999,061 bytes. `bearing_on_gate_A4` says
  this number does not close A4, and cold start was not remeasured.
  `--accept-size` is a design-plan declaration, not a way to make a gate
  green. The S22 release module measured 2,170,992 bytes and stayed inside
  the ceiling 2,258,755. The budget file was not edited.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The shared instructions make the run-sprint host
  responsible for counting whole-sprint passes and invoking `set-phase
  blocked` at the configured limit. The per-feature microscope loop remains
  uncapped. No automatic script transition is claimed.
- **M6 is not complete, and this open does not publish.** S22 closed F-063.
  M6 is S22 to S24, 5 stories and 16 engineer-weeks. `docs/RELEASE.md` maps
  one minor to each milestone from M2 to M12, so a finished M6 would name
  0.6.0. M1 through M5 were not published. The workspace version is still
  0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block this sprint.
- **No new measured duration.** The F-063 row in `SPRINT_TRACKER.md` reads
  `not measured`, as does every earlier row this close inspected. There is
  no measured total and no ratio. The capacity points remain the S03 and
  S06 closes. This open does not invent a ratio and does not recalculate
  capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A contour that is not the segment boundary, a surface that does not bound
the labelled voxels, or a viewport that draws a representation it was not
given.** HLD section 4 names three representations. HLD section 31 says a
segmentation mask should never round-trip through JavaScript to be drawn.
Painting the contour or the surface with the labelmap fill shader is the
wrong representation. The registry is the record of which one a viewport
shows. A tier that cannot draw one reports unavailable. It does not alias
that representation onto the composite labelmap.

## What done means

- **F-064** stores a contour for a segment. It is not the labelmap fill and
  it is not the one-pixel outline.
- **F-065** stores a surface for a segment. It is not a relabel of the
  contour and it is not the labelmap grid.
- **F-066** records, per viewport, which representation is shown. It does
  not choose a colour and it does not upload pixels.
- **Labelmap edge projection stays uncompiled.** That blend is F-067.
- **Each story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The image LUT arithmetic stays in `ocelli-pixel`. The labelmap
  presenter stays the F-063 path.

## Dependency order

F-064, F-065 and F-066 each depend on F-063. That backlog row is `done`.
No backlog cell in this sprint names another story in this sprint. F-067
depends on F-064 and is S24. F-X003 stays `in-progress` and is not a
dependency of this sprint.

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

S22 closed at tag `s22` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 2 clean at zero defects and zero smells. The gate
tree is `eb4825e5c3c3`. The merge is `33ccb34`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
re-run at this open. The S23 allocation rows and the S23 plan table name
F-064, F-065 and F-066. F-064 is done. F-065 and F-066 stay pending. F-063 is done. F-X003
stays in progress.
