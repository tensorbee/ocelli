# Current sprint, S15

**Milestone**: M4, the public API, the boundary, and the stack viewport.
**Branch**: `sprint/s15`
**Opened**: 2026-10-18
**Goal**: Show one series in a stack viewport, window it and scroll it, correct
spacing from the measured geometry, and write down the viewport, scene and
tool surface.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-045 | E7.1 | Stack viewport: display, fit, pan, zoom, rotate, flip | Rust+TS | 4w | done |
| F-046 | E7.2 | Window/level, VOI presets, invert, colormap | Rust+TS | 3w | pending |
| F-047 | E7.3 | Stack scroll, prefetch strategy, image sequencing | Rust+TS | 3w | pending |
| F-049 | E7.5 | Calibration, spacing and gantry-tilt correction | Rust+TS | 2w | pending |
| F-100 | E16.1 | API design: viewport, scene and tool surface, documented | TS | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S15 / {print $2, $9}'
```

## What this sprint is

S14 closed M3. Brick upload is the path for every volume, and the two
GPU-less runs exist. S15 is the first sprint of M4. A stack viewport shows
one frame, windows it and scrolls it. The spacing on that view comes from
the geometry F-020 measured. F-100 writes the viewport, scene and tool
surface as a new API.

M4 runs from S15 through S17. The plan calls this the Phase 1 credibility
gate, because the three-channel boundary is real and a stack viewport diffs
clean against cornerstone3D. Opening the sprint does not make that diff
exist. The oracle still has no Ocelli renderer to compare a full viewer
against. Decision D7 holds.

Closing M3 did not publish. The cadence names 0.3.0 for that milestone.
D-22 still refuses the release check. This open publishes nothing.

## What is carried in

- **An open finding against `ocelli-pixel`'s own arithmetic**, recorded in
  `docs/lld/pixel-pipeline.md` by F-041 and **not scheduled**. A legal
  `LINEAR` chain returns a value outside its declared output range at one
  input per window, identically on CPU and GPU, because PS3.3 C.11.2.1.2's
  formula in `f32` does not reproduce its own breakpoint. It has no F-ID.
  F-046 drives the existing window uniform. Clamping the finding so a
  readback matches is a tolerance change.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on
  this host through headless Chromium WebGL2 and matched `LutChain::map_into`
  within 0.001, format `R32Float`. A tier B colour LUT readback was not run.
  That comparison is not one of these five stories. The ignored native
  `Backends::GL` test found no adapter on this host, and that miss is not a
  tier B pass.
- **The wasm size budget records 1,999,061 bytes.** The S14 sprint gate
  measured the product module at 2,011,491 bytes, inside the 5 percent
  ceiling. The budget file was not moved. `bearing_on_gate_A4` in
  `ci/wasm-size-budget.json` says this number does not close A4, and cold
  start was not remeasured. The 2.3 ms figure belongs to the 16,388-byte
  module from F-006.
- **`max_review_passes` is written by `sprint_workflow.py init` and read by
  nothing**, so `/run-sprint`'s "mark the sprint blocked" branch has no
  mechanism behind it. Found by S11's sprint review, outside that sprint's
  diff, and carrying no F-ID. The write is the only hit in
  `scripts/sprint_workflow.py`.
- **M3 is complete and its release was not taken.** The cadence names 0.3.0.
  The workspace version is still 0.1.0. M1 was never published, and M2's
  release is still due. `/release` step 5 runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. None of that blocks a story here. All of it blocks
  publication.
- **M3 added no measured duration.** Every M3 row in `SPRINT_TRACKER.md`
  reads `not measured`. The capacity points remain the S03 and S06 closes,
  and this close does not invent a ratio from them.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. The multi-component JPEG-LS corpus row exists.
  Multi-component JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-X021 writes the candidate record and depends on F-012 and F-052. It is
  not this sprint. Decision D7 holds.

## The defect class this sprint is exposed to

**A view transform that rewrites samples or mixes spaces.** Fit, pan, zoom,
rotate and flip change how the frame is drawn. They do not rewrite stored
samples. HLD section 16 makes canvas points, world points and voxel indices
different types, because cornerstone's `number[]` mixes them. F-048, in S16,
owns those transforms. F-045 must not add a second point type as a bare
number list, and a canvas pan is not a patient-space translation.

**A window applied twice.** The LUT chain already lives in `ocelli-pixel`
and in the thirty-two-byte uniform. F-046 drives that uniform. Windowing a
`Display` value, or uploading the frame or the bricks again when the window
changes, is the quietly wrong pixel. A chain driven by a VOI LUT sequence
stays unavailable rather than being replaced by the linear values the
sequence overrode.

**A prefetched frame shown as the current one.** Scroll selects the frame
on screen. Prefetch fills the image cache F-032 already has. The neighbour
that arrived early is not the frame the index names.

**Shear taken from the gantry tilt tag.** F-020 measures stack shear from
the image positions and keeps calibrated spacing distinct from uncalibrated
spacing. `docs/lld/volumes.md` does not read the gantry tilt tag. F-049
applies that geometry. Reading the tag as the shear, or treating imager
pixel spacing as patient millimetres, is the defect.

**A cornerstone-shaped surface.** Decision D1 is a new API. F-100 documents
the viewport, scene and tool surface. A drop-in shim is the rejected
alternative. The command channel and the event ring are F-101 in S16, and
this sprint does not build them.

## What done means

- **F-045** displays one frame of a stack on the surface F-039 acquired.
  Fit, pan, zoom, rotate and flip change the view and do not rewrite stored
  samples. Coordinate transforms are F-048. Camera and presentation-state
  serialisation are F-050. The CPU raster of this viewport is F-X003. A tier
  that cannot draw the view reports unavailable and does not substitute
  another image.
- **F-046** changes window, level, invert and colormap through the existing
  LUT uniform and the colour table F-044 uploads. A window change writes
  thirty-two bytes and does not upload the frame or the bricks again. The
  chain runs once. A preset does not add a second arithmetic.
- **F-047** scrolls the stack in its stored order. Prefetch uses the image
  cache and does not display a frame the scroll index did not select.
  Volume brick prefetch in HLD section 30 is not this story.
- **F-049** corrects the stack view with the spacing and the measured shear
  F-020 already produced. The gantry tilt tag is not the shear. Calibrated
  and uncalibrated spacing stay distinct. Volume assembly is not rebuilt
  here.
- **F-100** documents the viewport, scene and tool surface as a new API. It
  does not implement the wasm command channel or the event ring.
- **Every story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device.

## Dependency order and the shared resources

F-045 depends on F-039. F-046 depends on F-041. F-047 depends on F-032.
F-049 depends on F-020. F-100 depends on nothing. All four of those
dependencies are `done`. No story in this sprint depends on another story
in this sprint, so no story begins blocked.

F-048 in S16 depends on F-045. F-101 in S16 depends on F-100. F-X003 in S16
depends on F-045. None of those three is in this sprint.

**One viewport, and the GPU.** F-045, F-046, F-047 and F-049 share the
stack viewport's source, its shader bindings and any device test. F-100
names the surface those four must not each invent. The designs land before
any of them is implemented.

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
