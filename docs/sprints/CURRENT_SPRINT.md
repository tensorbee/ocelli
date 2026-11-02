# Current sprint, S28

**Milestone**: M8, annotation tools.
**Branch**: `sprint/s28`
**Opened**: 2026-11-02
**Goal**: Length, Height, Angle, CobbAngle, Bidirectional, Probe, DragProbe, RectangleROI, EllipticalROI, CircleROI, PlanarFreehandROI, SplineROI and spline mathematics.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-073 | E12.1 | Length, Height, Angle, CobbAngle, Bidirectional | Rust+TS | 5w | done |
| F-074 | E12.2 | Probe, DragProbe, RectangleROI, EllipticalROI, CircleROI | Rust+TS | 5w | done |
| F-076 | E12.4 | PlanarFreehandROI, SplineROI and spline mathematics | Rust+TS | 5w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S28 / {print $2, $9}'
```

## What this sprint is

S27 closed at tag `s27`. The merge is `27c227e`. F-071 is done. M7 is
complete and this open does not publish. The workspace version stays 0.1.0.

This sprint owns the first annotation tools. Length, angle and the ROI
shapes measure with `ocelli-geom`. The SVG layer from F-071 draws the
result. The tools do not own the DICOM SR content tree. That type is F-094.
F-075 and F-077 are S29 and are not this sprint.

F-073, F-074 and F-076 depend on F-070. That row is `done`. The canonical
worktree runs one story at a time. There is no worker branch.

Closing this sprint does not complete M8. M8 is S28 to S29, 7 stories and
30 engineer-weeks. Completing a later M8 does not publish from this open.
A release is not taken from this open.

## What is carried in

- **M7 is complete at tag `s27`, and this open does not publish.** The
  merge is `27c227e`. No row in that close has a measured duration. No
  ratio is invented. A finished M7 would name 0.7.0. M1 through M7 were
  not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. This sprint does not
  depend on it.
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
  The ceiling is 2,258,755. The S27 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S27 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A length, an angle or an area that is quietly wrong.** These tools look
right in a drawing and disagree with the geometry. Canvas pixels and world
points are different types. A measurement that mixes them reports a number
in the wrong space. A degenerate segment, a zero normal or a collapsed ROI
must be refused. It must not return a plausible zero. The SR content tree
stays F-094. A tool that stores its own annotation model would be the type
decision D13 already gave to that later story.

## What done means

- **F-073** measures length, height, an unsigned angle, a Cobb angle and a
  bidirectional pair from the points the caller already holds. The formulas
  are the approved plan. A degenerate input is refused.
- **F-074** places a probe point and the rectangle, ellipse and circle
  shapes the approved plan names. The ROI statistics engine is F-075 and
  is not this sprint.
- **F-076** measures a planar freehand length and a spline that the approved
  plan defines. The spline mathematics live in one place.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-073, F-074 and F-076 depend on F-070. F-070 is `done`. F-071 is `done`
and is the drawing layer these tools can call. F-075 depends on F-074 and
is S29. F-077 depends on F-076 and is S29. F-094 owns the SR type and is
not a dependency of this sprint. F-X003 stays `in-progress` and is not a
dependency.

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

S27 closed at tag `s27`. The merge is `27c227e`. M7 is complete and this
open does not publish. The S28 allocation rows and the S28 plan table name
F-073, F-074 and F-076. F-070 is done. F-071 is done. F-X003 stays in
progress. F-X011 remains pending.
