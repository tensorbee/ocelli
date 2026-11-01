# Current sprint, S22

**Milestone**: M6, segmentation rendering.
**Branch**: `sprint/s22`
**Opened**: 2026-11-01
**Goal**: Labelmap representation: colour LUT, fill and outline rendering.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-063 | E10.1 | Labelmap representation: colour LUT, fill and outline rendering | Rust | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S22 / {print $2, $9}'
```

## What this sprint is

S21 closed at tag `s21`. The merge is `971376e`. M5 is complete. M6 starts.
This sprint draws a labelmap. The colour comes from that label's own LUT.
Fill covers the labelled voxels. Outline draws their boundary. The S22
table in `docs/sprints/SPRINT_PLAN.md` and the S22 row in
`docs/sprints/allocation.json` both name F-063.

`ocelli-seg` is still the F-001 scaffold. HLD section 4 gives that crate
the segmentation state and the three representations. HLD section 22 names
the overlay pass `SegOverlay`. This sprint fills the labelmap
representation. It does not reopen the image LUT chain, volume
construction, fusion, or the candidate gate.

F-063 depends on F-041. That backlog row is `done`. The image VOI shader
is not the segment colour table. Contour, surface, the representation
registry and labelmap edge projection are later stories. Labelmap edge
stays the uncompiled blend already left in place.

This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. F-063 does not depend
  on it.
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
  green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The shared instructions make the run-sprint host
  responsible for counting whole-sprint passes and invoking `set-phase
  blocked` at the configured limit. The per-feature microscope loop remains
  uncapped. No automatic script transition is claimed.
- **M5 is complete, and this open does not publish.** S21 closed F-060,
  F-061, F-062, F-X004 and F-X005. `docs/RELEASE.md` maps one minor to each
  milestone from M2 to M12, so a finished M5 names 0.5.0. M1, M2, M3 and M4
  were not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block this sprint.
- **No new measured duration.** The M5 rows in `SPRINT_TRACKER.md` read
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

**A label drawn in a colour that is not its LUT entry, a fill that covers
voxels the label does not own, or an outline that is not that label's
boundary.** HLD section 22 and section 31. Section 31 says a segmentation
mask should never round-trip through JavaScript to be drawn. Using the
image VOI window as the segment colour is the wrong table. Drawing the
outline as a second fill, or the fill as only the boundary, is the wrong
representation. A tier that cannot draw the overlay reports unavailable. It
does not alias `SegOverlay` to the composite stack pass.

## What done means

- **F-063** draws the labelmap fill and the labelmap outline from that
  label's colour LUT. The LUT is not the image VOI chain. The outline is
  the boundary of the labelled voxels. The mask stays with the drawer and
  does not cross into JavaScript to be painted.
- **Labelmap edge projection stays uncompiled.** That blend is F-067.
  Contour and surface stay unbuilt. Those are F-064 and F-065.
- **The story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The image LUT arithmetic stays in `ocelli-pixel`.

## Dependency order

F-063 depends on F-041. That backlog row is `done`. No backlog cell in this
sprint names another story in this sprint. F-X003 stays `in-progress` and
is not a dependency of F-063.

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

S21 closed at tag `s21` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 2 clean at zero defects and zero smells. The gate
tree is `adb6adafd1f7`. The merge is `971376e`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
re-run at this open. The S22 allocation row and the S22 plan table name
F-063. F-063 is pending. F-X003 stays in progress.
