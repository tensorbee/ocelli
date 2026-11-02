# Current sprint, S34

**Milestone**: M10, manipulation and utility tools.
**Branch**: `sprint/s34`
**Opened**: 2026-11-02
**Goal**: Slice intersection, segmentation intersection, an overlay grid, volume cropping, a sculptor, an annotation eraser and a MIP jump.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-089 | E14.3 | SliceIntersection, SegmentationIntersection, OverlayGrid | Rust+TS | 4w | done |
| F-092 | E14.6 | VolumeCropping and VolumeCroppingControl | Rust+TS | 3w | done |
| F-093 | E14.7 | Sculptor, AnnotationEraser, MIPJumpToClick | Rust+TS | 3w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S34 / {print $2, $9}'
```

## What this sprint is

S33 closed at tag `s33`. The merge is `5c07ef9`. F-087, F-088, F-090 and
F-091 are done. Closing S33 did not complete M10 and did not publish. The
workspace version stays 0.1.0.

This sprint owns the last three manipulation stories. Appendix B counts
25 manipulation and utility classes and does not define their formulas.
F-088 already draws a crosshair from a projected point. F-062 already
stores at most six patient clip planes. F-076 already measures a polygon
and a spline. These stories do not replace that arithmetic. They name
the shell helpers that sit beside it. The DICOM SR content tree stays
F-094.

Closing this sprint completes M10 and does not publish.

## What is carried in

- **M10 is not complete at this open.** M10 is S33 to S34. S33 is closed
  at tag `s33`. The merge is `5c07ef9`. The tag object is `b1c32477` and
  peels to that merge. The sprint tip is `8fe1642`, tree `f9d348a6c699`.
  F-087 is `0fd34ab`. F-088 is `c12948d`. F-090 is `18d17e6`. F-091 is
  `9e10257`. No row in that close has a measured duration. No ratio is
  invented. The comparison report is unchanged.
- **M9 is complete at tag `s32`, and that close did not publish.** The
  merge is `7ae7559`. The tag object is `1a255447` and peels to that
  merge.
- **M8 is complete at tag `s29` and was not published.** A finished M8
  would name 0.8.0. M1 through M9 were not published. The workspace
  version is still 0.1.0. `/release` runs
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
  The ceiling is 2,258,755. The S33 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** The S33 tracker rows read `not measured`.
  There is no measured total and no ratio. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A second clip test beside F-062, a second polygon area beside F-076, or
a canvas segment stored as a patient plane.** F-062 already keeps the
half-space `dot(point - origin, normal) >= 0` and refuses a seventh
plane. A shell that repeats that test can drift from the shader. F-089
clips caller canvas segments to the viewport and does not project. F-093
moves a vertex and names a mark. It does not erase the SR tree and it
does not scan a frame.

## What done means

- **F-089** clips one slice segment and a list of segmentation segments
  to the viewport, and returns the interior lines of an overlay grid. It
  does not project and it does not call the crosshair module.
- **F-092** accepts at most six caller planes and drags one origin along
  the caller normal. It does not call `set_clip_planes` and it does not
  retest the kept side.
- **F-093** moves one polygon vertex by the pixel delta, names the nearest
  mark inside a radius, and returns the earlier index of the maximum
  caller sample. It does not read pixels and it does not edit the
  annotation layer.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-089 depends on F-088. F-092 depends on F-062. F-093 depends on F-076.
Those rows are `done`. F-094 owns the SR type and is not a dependency of
this sprint. F-X003 stays `in-progress` and is not a dependency.

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

S33 closed at tag `s33`. The merge is `5c07ef9`. Closing S33 did not
complete M10 and did not publish. The S34 allocation rows and the S34
plan table name F-089, F-092 and F-093. F-X003 stays in progress.
F-X011 remains pending.
