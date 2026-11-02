# Current sprint, S33

**Milestone**: M10, manipulation and utility tools.
**Branch**: `sprint/s33`
**Opened**: 2026-11-02
**Goal**: Pan, zoom, window, stack scroll, crosshairs, reference lines, rotation and the magnify overlay.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-087 | E14.1 | Pan, Zoom, WindowLevel, WindowLevelRegion, StackScroll | Rust+TS | 4w | pending |
| F-088 | E14.2 | Crosshairs, WorldCrosshair, ReferenceLines, ReferenceCursors | Rust+TS | 5w | pending |
| F-090 | E14.4 | TrackballRotate, VolumeRotate, PlanarRotate, OrientationController | Rust+TS | 4w | pending |
| F-091 | E14.5 | Magnify, AdvancedMagnify, ScaleOverlay, OrientationMarker | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S33 / {print $2, $9}'
```

## What this sprint is

S32 closed at tag `s32`. The merge is `7ae7559`. F-086 is done. M9 is
complete. This open does not publish. The workspace version stays 0.1.0.

This sprint owns four manipulation stories. Appendix B counts 25
manipulation and utility classes and does not define their formulas. F-069
already owns pan, zoom and rotate from a caller basis. These stories do
not replace that arithmetic. They name the tools that call it, and the
overlays those tools draw. The DICOM SR content tree stays F-094.

Closing this sprint does not complete M10. M10 is S33 to S34. A release
is not taken from this open.

## What is carried in

- **M9 is complete at tag `s32`, and this open does not publish.** The
  merge is `7ae7559`. The tag object is `1a255447` and peels to that
  merge. The sprint tip is `329a872`, tree `296969729f10`. F-080 through
  F-086 stay as committed. No row in that close has a measured duration.
  No ratio is invented. The comparison report is unchanged.
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
  The ceiling is 2,258,755. The S32 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** The S32 tracker row reads `not measured`.
  There is no measured total and no ratio. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A second pan, zoom or rotate formula beside F-069, or a crosshair that
mixes a canvas point with a patient point.** F-069 already adds pan from
the basis and scales zoom and rotate by 256. A tool that writes the camera
again with different arithmetic is the defect. A reference line that treats
a canvas pixel as a patient coordinate is the other.

## What done means

- **F-087** names pan, zoom, window, region window and stack scroll on the
  existing registry. Pan, zoom and the wheel stay the F-069 formulas.
- **F-088** draws crosshairs and reference marks from one patient point.
  Canvas points and patient points stay distinct types.
- **F-090** names trackball, volume and planar rotation. The rotate delta
  stays the F-069 formula.
- **F-091** names magnify, scale and the orientation marker. The drawing
  layer from F-071 is not rewritten.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-087 depends on F-068. F-088 depends on F-055. F-090 depends on F-059.
F-091 depends on F-071. Those rows are `done`. F-089 waits for F-088 and
is S34. F-094 owns the SR type and is not a dependency of this sprint.
F-X003 stays `in-progress` and is not a dependency.

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

S32 closed at tag `s32`. The merge is `7ae7559`. M9 is complete and this
open does not publish. The S33 allocation rows and the S33 plan table name
F-087, F-088, F-090 and F-091. F-X003 stays in progress. F-X011 remains pending.
