# Current sprint, S19

**Milestone**: M5, volume, MPR and 3D rendering.
**Branch**: `sprint/s19`
**Opened**: 2026-10-27
**Goal**: Orthographic viewport: axial, sagittal, coronal, Progressive volume rendering while the series still loads, Multiscale pyramid hooks in the volume model, VOLUME_3D viewport with ray-cast direct volume rendering.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-052 | E8.2 | Orthographic viewport: axial, sagittal, coronal | Rust | 4w | pending |
| F-057 | E8.7 | Progressive volume rendering while the series still loads | Rust | 3w | pending |
| F-058 | E8.8 | Multiscale pyramid hooks in the volume model | Rust | 3w | pending |
| F-059 | E9.1 | VOLUME_3D viewport with ray-cast direct volume rendering | Rust | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S19 / {print $2, $9}'
```

## What this sprint is

S18 closed at tag `s18`. The merge is `ad2cdd0`. M5 continues. This sprint
reslices a constructed volume on the axial, sagittal and coronal planes,
draws while slices are still arriving, leaves a multiscale level axis on the
volume, and ray-casts a VOLUME_3D view. The S19 table in
`docs/sprints/SPRINT_PLAN.md` and the S19 rows in
`docs/sprints/allocation.json` both name F-052, F-057, F-058 and F-059.
Oblique reslice, thick slab, fusion and the full-corpus candidate gate stay
on later sprints.

F-051 fills one candidate. `place` writes stored samples at sorted position
`n`, and `order[n]` is the arrival index. `assemble` stays cleared. This
sprint consumes that volume. It does not reopen construction geometry and it
does not reopen the LUT chain.

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
- **M5 is not complete, and this open does not publish.** S18 closed F-051.
  `docs/RELEASE.md` maps one minor to each milestone from M2 to M12, so a
  finished M5 would name 0.5.0. S19, S20 and S21 remain. M1, M2, M3 and M4
  were not published. The workspace version is still 0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block these four
  stories.
- **No new measured duration.** F-051 in `SPRINT_TRACKER.md` reads
  `not measured`, as does every M4 row. There is no measured total and no
  ratio. The capacity points remain the S03 and S06 closes. This open does
  not invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Multi-component JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-X021 writes the candidate record and depends on F-012 and F-052. It is
  S20. Decision D7 holds.

## The defect class this sprint is exposed to

**A plane or a ray that is not the constructed volume.** HLD section 7 and
section 19. Axial, sagittal and coronal are the volume's own axes, from the
direction built when the volume was constructed. A sagittal sample that steps
along the row cosine instead of the column cosine is the swap. A brick whose
slices have not all landed must stay out of the upload, because the missing
slices would become zeros. A ray uses the same origin, spacing and direction
as `Volume::construct`. Spacing Between Slices is not that spacing.

HLD section 7 makes a 3D-texture ray-cast legal on tier B. Deviation D-07
says a feature that cannot run on the resolved tier reports unavailable.
F-X005 owns the CPU ray-cast decision and is S21. This sprint must not draw
a different image on the CPU and call it the same view.

## What done means

- **F-052** shows axial, sagittal and coronal from the constructed volume's
  direction, origin and spacing. The three planes are that volume. They are
  not a second patient frame and not a stack of stored frames sorted by
  Instance Number.
- **F-057** draws while the series is incomplete. A slice that has not landed
  contributes no samples. The view does not paint those voxels as zeros and
  call the volume complete.
- **F-058** leaves the multiscale level axis on the volume. The hook is the
  deliverable. A later consumer of the pyramid is not this story.
- **F-059** ray-casts a VOLUME_3D view from the constructed volume. Tier A
  and tier B each have an answer. Tier C reports unavailable unless F-X005
  has a measured path, and that story is not in this sprint. A CPU image
  that differs from the GPU image is not an answer.
- **Each story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- Oblique reslice, thick slab, slice sync, fusion and the candidate-render
  gate stay out. Those are F-053, F-054, F-055, F-056 and F-X021.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The LUT arithmetic stays in `ocelli-pixel`.

## Dependency order

F-052, F-057, F-058 and F-059 depend on F-051. That backlog row is `done`.
No backlog cell in this sprint names another story in this sprint.

F-053, F-054, F-055 and F-056 depend on F-052 and are S20. F-X021 depends on
F-012 and F-052 and is S20. F-060 depends on F-059 and is S21. F-X004 depends
on F-052 and F-X003 and is S21. F-X005 is S21.

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

S18 closed at tag `s18` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 2 clean at zero defects and zero smells. The gate
tree is `189248116673`. The merge is `ad2cdd0`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
green at this open. The S19 allocation rows and the S19 plan table name the
same F-IDs. No new audit was run at this open.

F-052, F-057, F-058 and F-059 are pending.
