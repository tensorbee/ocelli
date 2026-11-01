# Current sprint, S24

**Milestone**: M6, segmentation rendering.
**Branch**: `sprint/s24`
**Opened**: 2026-11-01
**Goal**: Segmentation intersection and labelmap edge-projection blend.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-067 | E10.5 | Segmentation intersection and labelmap edge-projection blend | Rust | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S24 / {print $2, $9}'
```

## What this sprint is

S23 closed at tag `s23`. The merge is `8f2d179`. M6 started in S22.
F-067 is done. M6 stays open until this sprint closes. This sprint
intersects a segmentation with the displayed plane and compiles the
labelmap edge-projection blend. Appendix B names that blend as the fifth
mode. The HLD does not give its formula. The design plan records the
local rule.

`BlendMode::LabelmapEdgeProjection` compiles for the segmentation overlay
on the existing labelmap copy shader. Slab, volume and stack have no body
for that blend. A lookup for those keys reports unavailable and does not
substitute composite. Contour, surface and the registry are already done.
They are not this blend.

F-067 depends on F-064. That backlog row is `done`. F-063, F-065 and
F-066 are done and are not dependencies of this row.

This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. F-067 does not depend
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
  The ceiling is 2,258,755. The S23 sprint gate measured the release module
  at 2,171,060 bytes. The budget file was not edited. `--accept-size` is a
  design-plan declaration, not a way to make a gate green.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** The operator required the review loop to continue until
  a pass reports zero defects and zero smells. The stored number is not a
  stop. The per-feature microscope loop remains uncapped.
- **M6 is not complete, and this open does not publish.** S23 closed
  F-064, F-065 and F-066. F-067 is done. M6 stays open until this sprint
  closes. M6 is S22 to
  S24, 5 stories and 16 engineer-weeks. `docs/RELEASE.md` maps one minor
  to each milestone from M2 to M12, so a finished M6 would name 0.6.0.
  M1 through M5 were not published. The workspace version is still 0.1.0.
  `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block this sprint.
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

**An edge projection that quietly draws the composite blend, or an
intersection that paints a pixel the contour does not cross.** HLD
section 31 says a feature that cannot run reports unavailable. It does
not alias onto another blend. The intersection is a patient-space contour
against the displayed plane. Mixing a canvas pixel with a patient
millimetre is the silent wrong pixel.

## What done means

- **F-067** states the intersection rule and the edge-projection rule in
  the approved design plan, then implements both. The composite labelmap,
  the contour stroke and the surface cover stay as S23 left them.
- **The edge-projection blend does not substitute composite.** A tier that
  cannot run it reports unavailable.
- **One `queue.submit` per frame** stays one. This story does not add a
  second submit.
- **Tier C** reports unavailable or shares the one presenter. It does not
  grow a second raster.
- **Each story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.

## Dependency order

F-067 depends on F-064. That backlog row is `done`. No other story is
allocated to S24. F-X003 stays `in-progress` and is not a dependency of
this sprint.

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

S23 closed at tag `s23` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 1 clean at zero defects and zero smells. The gate
tree is `97f98dab6687`. The merge is `8f2d179`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
re-run at this open. The S24 allocation row and the S24 plan table name
F-067. F-067 is done. F-064, F-065 and F-066 are done. F-063 is done.
F-X003 stays in progress.
