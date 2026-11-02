# Current sprint, S27

**Milestone**: M7, tool framework and geometry.
**Branch**: `sprint/s27`
**Opened**: 2026-11-02
**Goal**: SVG annotation drawing layer.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-071 | E11.4 | SVG annotation drawing layer | Rust+TS | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S27 / {print $2, $9}'
```

## What this sprint is

S26 closed at tag `s26`. The merge is `84402db`. F-069 and F-072 are done.
M6 stays complete at tag `s24`. This open does not publish. The workspace
version stays 0.1.0.

This sprint owns the SVG annotation drawing layer in the shell. It draws
a caller-supplied display list into one SVG element and emits the F-103
annotation names. It does not own the DICOM SR content tree. That type is
F-094. F-073 and the later annotation tools are not this sprint.

F-071 depends on F-069. That row is `done`. The canonical worktree runs
this story alone.

Closing this sprint completes M7. M7 is S25 to S27, 5 stories and 16
engineer-weeks. Completing M7 does not publish. A finished M7 would name
0.7.0. A release is not taken from this open.

## What is carried in

- **M7 closes with this sprint, and this open does not publish.** Tag
  `s26` points at merge `84402db`. No row in that close has a measured
  duration. No ratio is invented. A finished M7 would name 0.7.0. M1
  through M6 were not published. The workspace version is still 0.1.0.
  `/release` runs `python3 scripts/pin_and_size_check.py --require-redistribution`,
  and that command refuses while `openjph-core` 0.1.0 carries no BSD notice
  material, which is D-22. That blocks publication. It does not block
  this sprint.
- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. This story does not
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
  The ceiling is 2,258,755. The S26 sprint gate passed that budget. The
  budget file was not edited. `--accept-size` is a design-plan
  declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **No new measured duration.** Every `SPRINT_TRACKER.md` row the S26 close
  inspected reads `not measured`. There is no measured total and no ratio.
  The capacity points remain the S03 and S06 closes. This open does not
  invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**A display list that quietly becomes a second annotation model, or an SVG
write that drops a hidden mark.** The HLD says the drawing layer renders
from the annotation type and does not own it. The SR content tree is F-094.
A mark stored here as if it were that type would be the model D13 refuses.
A hidden mark removed from the SVG would come back later as a new id. Points
are canvas pixels the caller already projected. Treating them as world
points would move the drawing.

## What done means

- **F-071** writes one SVG group per caller mark and emits the F-103
  annotation names from the difference between the last accepted list and
  the next one. It does not listen. It does not call wasm. It does not
  define an SR content item.
- **A refused list changes nothing.** The previous SVG and the previous
  events stay as they were.
- **Each story declares all three tier rows.** This layer is DOM SVG, so
  the rows are n/a. An omitted row is not an answer.

## Dependency order

F-071 depends on F-069. F-069 is `done`. F-094 owns the SR type and is not
a dependency of this story. F-X003 stays `in-progress` and is not a
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

S26 closed at tag `s26`. The merge is `84402db`. M7 stays open until this
sprint's story is done, and this open does not publish. The S27 allocation
row and the S27 plan table name F-071. F-069 is done. F-072 is done. F-068
is done. F-070 is done. F-X003 stays in progress. F-X011 remains pending.
