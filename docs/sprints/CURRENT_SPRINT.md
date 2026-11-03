# Current sprint, S35

**Milestone**: M11, annotation state and DICOM interop.
**Branch**: `sprint/s35`
**Opened**: 2026-11-02
**Goal**: DICOM SR as the native in-memory annotation model, an annotation state manager with frame-of-reference indexing and undo, a DICOM SEG round trip, and an RTSTRUCT import into the contour representation.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-094 | E15.1 | DICOM SR as the native in-memory annotation model | Rust+TS | 4w | done |
| F-095 | E15.2 | Annotation state manager, frame-of-reference indexing, undo/redo | Rust+TS | 4w | done |
| F-098 | E15.5 | DICOM SEG round-trip | Rust+TS | 4w | done |
| F-099 | E15.6 | RTSTRUCT import to contour representation | Rust+TS | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S35 / {print $2, $9}'
```

## What this sprint is

S34 closed at tag `s34`. The merge is `c3f85e5`. F-089, F-092 and F-093
are done. Closing S34 completed M10 and did not publish. The workspace
version stays 0.1.0.

This sprint opens M11. Decision D13 says the annotation type is a DICOM
SR content tree. Decision D10 keeps dcmjs for SR generation. HLD section
34 says the in-memory type is that tree, the drawing layer renders from
it, and provenance names the tool, the engine version, the algorithm
where one was involved, and the operator. The section does not give a
content-item layout. The plans will transcribe that limit and decide the
gap locally. F-070 already owns measurement values. F-066 already owns
the labelmap presenter. F-064 already owns the contour representation.
These stories do not replace that arithmetic.

Closing this sprint does not complete M11 and does not publish. M11 is
S35 to S36.

## What is carried in

- **M10 is complete at tag `s34`, and that close did not publish.** The
  merge is `c3f85e5`. The tag object is `bff12197` and peels to that
  merge. The sprint tip is `1c1b85a`, tree `ae3fbbc21b6a`. F-089 is
  `50d0fc0`. F-092 is `92ddd1e`. F-093 is `9e3d356`. The open is
  `3e1ee1a`. The design is `599ddfc`. No row in that close has a measured
  duration. No ratio is invented. The comparison report is unchanged.
- **M9 is complete at tag `s32`, and that close did not publish.** The
  merge is `7ae7559`. The tag object is `1a255447` and peels to that
  merge.
- **M8 is complete at tag `s29` and was not published.** A finished M8
  would name 0.8.0. M1 through M10 were not published. The workspace
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
  The ceiling is 2,258,755. The S34 sprint gate measured 2,171,131 bytes
  and passed that budget. The budget file was not edited. `--accept-size`
  is a design-plan declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** The S34 tracker rows read `not measured`.
  There is no measured total and no ratio. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A second annotation model beside the SR content tree, a SEG written as
a private bitmap, or an RTSTRUCT contour stored in canvas pixels.** HLD
section 34 says the in-memory type is the content tree and the drawing
layer does not own it. F-066 already presents a labelmap. F-064 already
stores a contour. A second copy of either one can drift from the
presenter. dwv, Horos, and the medical viewer Grok must not be opened.
The SR idea is taken from DICOM PS3.3 and PS3.16.

## What done means

- **F-094** stores an annotation as a DICOM SR content tree. The drawing
  layer is not the owner. Provenance can name the tool, the engine
  version, an algorithm, and the operator. The story does not read a
  GPL viewer.
- **F-095** indexes those annotations by frame of reference and records
  undo and redo. It does not invent a second measurement formula.
- **F-098** reads and writes a DICOM SEG from the existing labelmap. It
  does not replace the F-066 presenter.
- **F-099** reads an RTSTRUCT into the existing contour representation.
  It does not store the contour as canvas pixels.
- **Each story declares all three tier rows.** An omitted row is not an
  answer. A measurement that cannot run on the resolved tier reports
  unavailable.

## Dependency order

F-094 depends on F-070. F-095 depends on F-070. F-098 depends on F-066.
F-099 depends on F-064. Those rows are `done`. F-096 and F-097 are S36
and are not stories in this sprint. F-X003 stays `in-progress` and is
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

S34 closed at tag `s34`. The merge is `c3f85e5`. Closing S34 completed
M10 and did not publish. The S35 allocation rows and the S35 plan table
name F-094, F-095, F-098 and F-099. F-X003 stays in progress.
F-X011 remains pending.
