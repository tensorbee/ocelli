# Current sprint, S21

**Milestone**: M5, volume, MPR and 3D rendering.
**Branch**: `sprint/s21`
**Opened**: 2026-11-01
**Goal**: Transfer-function editing surface and preset library, Lighting, shading and gradient opacity, Cropping planes and volume clipping, CPU MPR and oblique reslicing, with SIMD inner loops, The CPU volume-rendering decision, and honest unavailability.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-060 | E9.2 | Transfer-function editing surface and preset library | Rust | 3w | done |
| F-061 | E9.3 | Lighting, shading and gradient opacity | Rust | 3w | done |
| F-062 | E9.4 | Cropping planes and volume clipping | Rust | 4w | done |
| F-X004 | X1.4 | CPU MPR and oblique reslicing, with SIMD inner loops | Rust | 5w | pending |
| F-X005 | X1.5 | The CPU volume-rendering decision, and honest unavailability | Rust+TS | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S21 / {print $2, $9}'
```

## What this sprint is

S20 closed at tag `s20`. The merge is `685f55d`. M5 continues. This sprint
gives the volume ray cast a transfer function and a preset library, shades
that ray, clips it to caller planes, and decides what the CPU tier may draw
for MPR and for the volume view. The S21 table in
`docs/sprints/SPRINT_PLAN.md` and the S21 rows in
`docs/sprints/allocation.json` both name F-060, F-061, F-062, F-X004 and
F-X005.

F-059 ray-casts ready bricks. F-060 replaces its constant 0.5 opacity with
a 256-entry table. F-061 multiplies that colour by a fragment shade and
leaves the default unlit. F-052 samples the axial, sagittal and coronal planes of
the constructed volume. This sprint consumes those views. It does not
reopen volume construction, the LUT chain, fusion, or the candidate gate.

F-X004 and F-X005 also depend on F-X003. That backlog row is still
`in-progress`, carried for lack of a representative GPU-less host. Those
two stories do not start while that dependency is open. F-060, F-061 and
F-062 depend only on F-059, which is `done`.

This open publishes nothing.

## What is carried in

- **F-X003 is carried out of S16 and is not a story in this sprint.** AC6
  needs a representative GPU-less incumbent host, the operator has confirmed
  none is available, and no local diagnostic is accepted as that
  measurement. Its backlog row stays `in-progress`. F-X004 and F-X005 name
  it as a dependency and wait.
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
- **M5 is not complete, and this open does not publish.** S20 closed F-053,
  F-054, F-055, F-056 and F-X021. `docs/RELEASE.md` maps one minor to each
  milestone from M2 to M12, so a finished M5 would name 0.5.0. S21 remains.
  M1, M2, M3 and M4 were not published. The workspace version is still
  0.1.0. `/release` runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. That blocks publication. It does not block this sprint.
- **No new measured duration.** The S20 rows in `SPRINT_TRACKER.md` read
  `not measured`, as does every earlier M5 and M4 row. There is no measured
  total and no ratio. The capacity points remain the S03 and S06 closes.
  This open does not invent a ratio and does not recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. Lossless JPEG-LS RGB at 8 bits, three components and sample
  interleave decodes. Line interleave, a non-zero NEAR and any other
  component count stay refused.

## The defect class this sprint is exposed to

**An opacity that is not the named preset, a shade that rewrites the
sample, a clip that is not the requested plane, or a CPU image that
quietly differs from the GPU one.** HLD section 7 and section 19. The
volume has one origin, one spacing and one direction. F-059 holds opacity
at 0.5 until a transfer function exists. A preset that maps to a different
curve is the wrong function. Changing the VOI window and calling it the
transfer function is the same defect.

Lighting and gradient opacity are optical. They are not a second window
and they do not rewrite the stored sample. A crop cuts the ray in the
patient frame. Dropping a whole brick, or clipping in canvas space, is
the wrong crop.

Section 7 makes a 3D-texture ray-cast legal on tier B. Deviation D-07 says
a feature that cannot run on the resolved tier reports unavailable. F-X004
has to sample the same plane F-052 already takes. F-X005 must not draw a
slow CPU volume that differs from the tier A ray cast. Unavailable is a
legitimate answer for that view.

## What done means

- **F-060** replaces the constant 0.5 opacity with the caller's transfer
  function and a named preset. The preset is that curve. It is not a VOI
  window under another name.
- **F-061** shades the ray and applies gradient opacity. The stored sample
  stays the sample. A mode that cannot run on the resolved tier reports
  unavailable.
- **F-062** clips the ray to caller planes in the patient frame. A brick
  that merely intersects the plane is not discarded whole.
- **F-X004** reslices on the CPU, including an oblique plane, with the SIMD
  inner loop and a measured no-SIMD path. The sample is the plane F-052
  already defines. It does not start while F-X003 is open.
- **F-X005** decides the CPU volume path. A quiet second image is not an
  acceptable decision. Unavailable, said to the user, is. It does not start
  while F-X003 is open.
- **Each story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device. The LUT arithmetic stays in `ocelli-pixel`.

## Dependency order

F-060, F-061 and F-062 depend on F-059. That backlog row is `done`.
F-X004 depends on F-052 and F-X003. F-052 is `done`. F-X003 is
`in-progress`. F-X005 depends on F-059 and F-X003. No backlog cell in this
sprint names another story in this sprint. F-X004 and F-X005 begin blocked.

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

S20 closed at tag `s20` with 31 of 31 gates green, the corpus passing, and
whole-sprint review pass 2 clean at zero defects and zero smells. The gate
tree is `93bf532df411`. The merge is `685f55d`. `scripts/backlog_check.py`,
`scripts/gen_sprint_plan.py --check` and `scripts/deviation_check.py` are
green at this open. The S21 allocation rows and the S21 plan table name the
same F-IDs. No new audit was run at this open.

F-060, F-061, F-062 and F-X005 are done. F-X004 stays pending while F-X003 is in progress.
