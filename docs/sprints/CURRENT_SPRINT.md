# Current sprint, S29

**Milestone**: M8, annotation tools.
**Branch**: `sprint/s29`
**Opened**: 2026-11-02
**Goal**: ROI statistics engine: mean, SD, min, max, area, SUV, histogram, LivewireContour and LivewireContourSegmentation, ArrowAnnotate, Label, KeyImage, ETDRSGrid, UltrasoundDirectional, VideoRedaction.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-075 | E12.3 | ROI statistics engine: mean, SD, min, max, area, SUV, histogram | Rust+TS | 4w | done |
| F-077 | E12.5 | LivewireContour and LivewireContourSegmentation | Rust+TS | 5w | done |
| F-078 | E12.6 | ArrowAnnotate, Label, KeyImage, ETDRSGrid | Rust+TS | 3w | done |
| F-079 | E12.7 | UltrasoundDirectional, VideoRedaction | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S29 / {print $2, $9}'
```

## What this sprint is

S28 closed at tag `s28`. The merge is `f5e9bbc`. F-073, F-074 and F-076 are
done. M8 stays open until this sprint closes. This open does not publish.
The workspace version stays 0.1.0.

This sprint owns ROI statistics, a livewire contour, the arrow, label, key
image and ETDRS marks, and the ultrasound direction and video redaction
tools. Statistics and livewire measure with the geometry crate. The SVG
layer from F-071 draws a mark. The tools do not own the DICOM SR content
tree. That type is F-094.

F-075 depends on F-074. F-077 depends on F-076. F-078 depends on F-071.
F-079 depends on F-070. Those rows are `done`. The canonical worktree runs
one story at a time. There is no worker branch.

Closing this sprint completes M8. M8 is S28 to S29, 7 stories and 30
engineer-weeks. Completing M8 does not publish from this open. A release
is not taken from this open.

## What is carried in

- **M8 stays open at tag `s28`, and this open does not publish.** The
  merge is `f5e9bbc`. No row in that close has a measured duration. No
  ratio is invented. A finished M8 would name 0.8.0. M1 through M7 were
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
  The ceiling is 2,258,755. The S28 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S28 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A statistic or a contour that is quietly wrong.** A mean, a standard
deviation, an SUV or a histogram bin can look plausible and still disagree
with the samples inside the region. An empty region must be refused. It
must not return a plausible zero. A livewire path that ignores the image
gradient is a different contour. The SR content tree stays F-094. A tool
that stores its own annotation model would be the type decision D13 already
gave to that later story.

## What done means

- **F-075** reports the statistics the approved plan names for a region the
  caller already holds. The area already measured by F-074 and F-076 is not
  a second formula. An empty region is refused.
- **F-077** builds a livewire contour from the approved plan. The polygon
  area and the spline point stay the F-076 functions.
- **F-078** draws an arrow, a label, a key image and an ETDRS grid through
  the F-071 layer. It does not create the SR content tree.
- **F-079** reports ultrasound direction and video redaction from the
  approved plan. A redaction that leaves the pixel unchanged says so.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-075 depends on F-074. F-077 depends on F-076. F-078 depends on F-071.
F-079 depends on F-070. Those four rows are `done`. F-094 owns the SR type
and is not a dependency of this sprint. F-X003 stays `in-progress` and is
not a dependency.

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

S28 closed at tag `s28`. The merge is `f5e9bbc`. M8 stays open and this
open does not publish. The S29 allocation rows and the S29 plan table name
F-075, F-077, F-078 and F-079. F-070, F-071, F-074 and F-076 are done.
F-X003 stays in progress. F-X011 remains pending.
