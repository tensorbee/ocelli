# Current sprint, S18

**Milestone**: M5, volume, MPR and 3D rendering.
**Branch**: `sprint/s18`
**Opened**: 2026-10-27
**Goal**: Volume construction from a stack: geometry, orientation, spacing.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-051 | E8.1 | Volume construction from a stack: geometry, orientation, spacing | Rust | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S18 / {print $2, $9}'
```

## What this sprint is

S17 closed M4 at tag `s17`. The merge is `7117d14`. M5 starts here. This
sprint constructs a volume from a stack, and the work is the geometry, the
orientation and the spacing. The S18 table in `docs/sprints/SPRINT_PLAN.md`
and the S18 row in `docs/sprints/allocation.json` both name F-051, epic E8.1.
Orthographic MPR, progressive rendering, the pyramid, ray casting and the
candidate-render gate stay on later sprints.

F-033 already owns the volume cache and `Volume::assemble`. The as-built note
on that story says F-051 is the first caller of `place`. This sprint is that
caller. It does not reopen the LUT chain.

The oracle still has no Ocelli renderer to compare a full viewer against.
Decision D7 holds. This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not scheduled here.** AC6 needs a
  representative GPU-less incumbent host, the operator has confirmed none is
  available, and no local diagnostic is accepted as that measurement. AC5's
  coverage verdict is `coverage-loss` with 67 matched views, 0 pixel failures
  and 17 unmeasured. Its backlog row stays `in-progress`. Resume when a
  representative host, an incumbent workload and an accepted budget exist.
- **The open LINEAR finding against `ocelli-pixel`'s own arithmetic** stays
  unscheduled. It has no F-ID. Clamping it so a readback matches is a
  tolerance change.
- **The SIGMOID reference-width diagnosis** stays open in
  `docs/lld/oracle.md`. The rendered reference codes fit width 0.5 while the
  installed transfer-function path statically yields width 1.5. An oracle run
  is needed to settle it. Do not change DICOM arithmetic or the parity policy
  to settle it.
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
- **M4 is complete. A release is the cadence's next step, and this open does
  not take it.** `docs/RELEASE.md` maps one minor to each milestone from M2
  to M12, so M4 names 0.4.0. M1, M2 and M3 were not published. The workspace
  version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block F-051.
- **No new measured duration.** Every M4 row in `SPRINT_TRACKER.md` reads
  `not measured`. There is no measured total and no ratio. The capacity
  points remain the S03 and S06 closes. This open does not invent a ratio
  and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Multi-component JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-X021 writes the candidate record and depends on F-012 and F-052. It is
  not this sprint. Decision D7 holds.

## The defect class this sprint is exposed to

**Spacing or slice order taken from a tag.** HLD section 19, in
`docs/hld/16-volume-representation.md`, and DICOM PS3.3 C.7.6.2.1.1.
`PixelSpacing` is row then column. `ImagePositionPatient` is the centre of
voxel zero. The inter-slice step is that position projected on the slice
normal, the row cosine crossed with the column cosine. Shear is the part of
the step that is not parallel to the normal, measured from the positions.
HLD section 19 stores `spacing` as three millimetre values and `origin` as
the `ImagePositionPatient` of the first slice.

The failure is a volume whose spacing is Spacing Between Slices, Slice
Thickness or Slice Location, whose order is Instance Number, or whose shear
is Gantry/Detector Tilt. Those tags are evidence. They are not the geometry.
Swapping the two `PixelSpacing` values is the same class of defect.

## What done means

- **F-051** constructs a volume from a stack. Geometry, orientation and
  spacing come from the image plane under PS3.3 C.7.6.2.1.1 and HLD section
  19. The origin is the `ImagePositionPatient` of the lowest projected slice,
  copied exactly, and that point is the centre of voxel zero. Spacing index 0
  steps along the row and index 1 along the column. A sheared stack is not
  stored as an axis-aligned box. The story is the first caller of
  `Volume::place`. It does not treat Spacing Between Slices, Slice Thickness,
  Slice Location, Instance Number or the gantry tilt tag as the answer.
- **F-051 declares all three tier rows**, and "n/a" is a real answer that an
  omitted row is not.
- Orthographic reslice, progressive upload, the pyramid, ray casting and the
  full-corpus candidate gate stay out. Those are F-052, F-057, F-058, F-059
  and F-X021.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The LUT arithmetic stays in `ocelli-pixel`.

## Dependency order

F-051 depends on F-033. That backlog row is `done`. No backlog cell in this
sprint names another story in this sprint.

F-052, F-057, F-058 and F-059 depend on F-051 and are S19. F-X021 depends on
F-012 and F-052 and is S20. F-X004 depends on F-052 and F-X003 and is S21.

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

## Audit intake, 2026-10-27

S17 closed at tag `s17` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 2 clean at zero defects and zero smells. The gate
tree is `6fc40a9abcad`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
green at this open. The S18 allocation row and the S18 plan table name the
same F-ID. No new audit was run at this open.

F-051 is done. Its design plan is approved in `.claude/plans/F-051-design.md`.
