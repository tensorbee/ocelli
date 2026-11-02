# Current sprint, S31

**Milestone**: M9, segmentation tools.
**Branch**: `sprint/s31`
**Opened**: 2026-11-02
**Goal**: Rectangle, Circle and Sphere scissors, PaintFill and flood fill, RectangleROI and CircleROI start-end threshold tools, LabelmapEditWithContour.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-081 | E13.2 | Rectangle, Circle and Sphere scissors | Rust+TS | 4w | done |
| F-082 | E13.3 | PaintFill and flood fill | Rust+TS | 3w | done |
| F-083 | E13.4 | RectangleROI and CircleROI start-end threshold tools | Rust+TS | 4w | done |
| F-085 | E13.6 | LabelmapEditWithContour | Rust+TS | 3w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S31 / {print $2, $9}'
```

## What this sprint is

S30 closed at tag `s30`. The merge is `1cff82a`. F-080 and F-084 are done.
M9 is not complete. This open does not publish. The workspace version stays
0.1.0.

This sprint owns scissors, flood fill, paint fill, a start-end threshold
inside a rectangle or a circle, and a labelmap edit from a closed polygon.
Each tool writes a segment number into a caller label grid. None of them
draws. The existing labelmap presenter still draws. The tools do not own
the DICOM SR content tree. That type is F-094.

F-081, F-082, F-083 and F-085 depend on F-080. That row is `done`. The
canonical worktree runs one story at a time. There is no worker branch.
F-086 stays in S32.

Closing this sprint does not complete M9. M9 is S30 to S32, 7 stories and
24 engineer-weeks. A release is not taken from this open.

## What is carried in

- **M8 is complete at tag `s29`, and this open does not publish.** The
  merge is `26f003c`. No row in that close has a measured duration. No
  ratio is invented. A finished M8 would name 0.8.0. M1 through M8 were
  not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and
  that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **S30 is closed at tag `s30` and did not complete M9.** The merge is
  `1cff82a`. F-080 and F-084 are done. The brush and the segment readings
  stay as committed.
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
  The ceiling is 2,258,755. The S30 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S30 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A write that is quietly one sample off, or a refusal that still writes.**
A sample on a scissor edge or on a polygon edge belongs inside. A sample
past that edge stays as it was. A flood does not cross a different segment
and does not use a diagonal. A threshold paints only where the value is
inside the caller's start and end. A non-finite value inside the shape
writes nothing. Segment 0 remains a legal erase and is still background
for the presenter.

## What done means

- **F-081** erases to segment 0 inside a rectangle, a circle or a sphere.
  The circle and the sphere use the existing stamp. The rectangle uses
  inclusive index bounds on one slice.
- **F-082** flood-fills the 4-connected region of the seed value, and
  paint-fills each sample that still holds that value. The caller
  chooses the written segment.
- **F-083** paints a caller segment where a caller value buffer is between
  a start and an end, and only inside a rectangle or a circle.
- **F-085** paints a caller segment on the samples inside a closed polygon,
  including the boundary. It does not edit the contour representation.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-081, F-082, F-083 and F-085 depend on F-080. That row is `done`. F-086
depends on F-080 and stays in S32. F-094 owns the SR type and is not a
dependency of this sprint. F-X003 stays `in-progress` and is not a
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

S30 closed at tag `s30`. The merge is `1cff82a`. M9 is not complete and
this open does not publish. The S31 allocation rows and the S31 plan table
name F-081, F-082, F-083 and F-085. F-080 is done. F-086 stays in S32.
F-X003 stays in progress. F-X011 remains pending.
