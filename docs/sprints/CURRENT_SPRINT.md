# Current sprint, S14

**Milestone**: M3, the cache and the renderer.
**Branch**: `sprint/s14`
**Opened**: 2026-10-18
**Goal**: Upload every volume as bricks, and add the two GPU-less render
layers that a pull request and a nightly browser can run.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-043 | E6.7 | Chunked brick residency as the default upload path | Rust | 4w | done |
| F-X002 | X1.2 | Three-layer GPU-less render testing: lavapipe in CI and Chrome with SwiftShader nightly | Build | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S14 / {print $2, $9}'
```

## What this sprint is

S13 acquired an OffscreenCanvas when one is bound and uploaded a stored frame
with `write_texture`. A viewport with no surface still draws into a
caller-supplied texture view. S14 is the last sprint of M3. F-043 makes the
brick the unit that upload sends, for every volume. F-X002 adds lavapipe on
`ubuntu-latest` and headless Chrome with SwiftShader. Layer 1, the tier C
raster, is already a cargo test and is not rebuilt here.

M3 runs from S11 through S14. Closing this sprint is the milestone boundary.
Opening it is not, and a release is a milestone boundary. This open publishes
nothing.

## What is carried in

- **An open finding against `ocelli-pixel`'s own arithmetic**, recorded in
  `docs/lld/pixel-pipeline.md` by F-041 and **not scheduled**. A legal `LINEAR`
  chain returns a value outside its declared output range at one input per
  window, identically on CPU and GPU, because PS3.3 C.11.2.1.2's formula in
  `f32` does not reproduce its own breakpoint. It has no F-ID, because creating
  one moves the backlog's identity and the sprint plan's arithmetic and is the
  operator's decision. F-043 uploads stored samples. The window stays the
  thirty-two-byte uniform. A story that consumes `Display` values should know
  the range is not a guarantee, and clamping the finding to make a readback
  match is a tolerance change.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on this
  host through headless Chromium WebGL2 and matched `LutChain::map_into` within
  0.001, format `R32Float`. A tier B colour LUT readback was not run. That
  comparison is not F-X002. The ignored native `Backends::GL` test found no
  adapter on this host, and that miss is not a tier B pass.
- **The wasm size budget records 1,999,061 bytes.** The S13 sprint gate
  measured the product module at 2,010,440 bytes, inside the 5 percent ceiling.
  The budget file was not moved. `bearing_on_gate_A4` in
  `ci/wasm-size-budget.json` says this number does not close A4, and cold start
  was not remeasured. The 2.3 ms figure belongs to the 16,388-byte module from
  F-006.
- **`max_review_passes` is written by `sprint_workflow.py init` and read by
  nothing**, so `/run-sprint`'s "mark the sprint blocked" branch has no
  mechanism behind it. Found by S11's sprint review, outside that sprint's
  diff, and carrying no F-ID. The write is the only hit in
  `scripts/sprint_workflow.py`.
- **M2's release is due and cannot be taken.** `/release` step 5 runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. M1's `0.1.0` was never published. S13 did not finish M3, so
  closing it did not make an M3 release due. Neither blocks a story here. Both
  block publication.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. The multi-component JPEG-LS corpus row exists. Multi-component
  JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-X021 writes the candidate record and depends on F-012 and F-052. It is not
  this sprint. F-X002's three layers are test layers, and none of them is that
  comparison. Decision D7 holds.

## The defect class this sprint is exposed to

**A size check that puts the whole volume back.** Decision D11 says chunked
residency is the path, and the rejected alternative is bricks only above a
threshold. F-036 already has no whole-volume path and no size threshold in the
cache. F-043 is where an upload can put one back. A small series that uploads
whole, with bricks reserved for the series that already hurt, makes bounded
GPU memory a fallback. The product claim is the opposite.

**An edge brick padded to a cube.** F-036 keeps the true extent and prices
`Brick::bytes` as the host buffer length. An upload that pads the edge brick
to 128 or to 64, and a shader that then samples the pad, draws voxels that
were not in the volume. That is the quietly wrong pixel.

**A window change that uploads the bricks again.** The samples are the brick.
The window is thirty-two bytes, which is the rule F-040 already holds for a
stored frame. Re-uploading bricks on a window or level change, or uploading
values the shader will window again, is the same defect on the path every
volume now takes.

**Lavapipe described as the GPU-less session.** F-X002. Layer 2 is lavapipe
through wgpu on `ubuntu-latest`. Layer 3 is SwiftShader through headless
Chrome WebGL2, nightly or manual. A pipeline that constructs under lavapipe
says nothing about what the browser session draws. Neither layer is the
oracle. The reference remains cornerstone3D. If lavapipe does not resolve as
a fallback adapter under the pinned wgpu, layer 2 drops and layers 1 and 3
stand. Recording that drop as a pass is the failure.

## What done means

- **F-043** uploads bricks from the `BrickGrid` F-036 built, for every volume,
  including one that fits. There is no second cache and no size threshold.
  Phase 1 uploads every brick. Frustum selection, level of detail, the
  indirection texture and prefetch are HLD section 30 and are not this story.
  An edge brick uploads its true extent. A window change writes the VOI
  uniform and does not upload the bricks again. Tier C reports unavailable and
  does not upload. An extent or format the resolved tier cannot sample reports
  unavailable and does not substitute another image. Tier B's 3D texture floor
  is 256. A brick edge of 128 fits that floor, and a longer volume axis is
  more bricks.
- **F-X002** builds layers 2 and 3. Layer 1 stays the adapter-free cargo test.
  Layer 2's acceptance is that lavapipe resolves as a fallback adapter under
  the pinned wgpu on `ubuntu-latest`, or that the layer is dropped and the
  drop is the recorded result. A named skip printed as a pass is not that
  result. Layer 3 is the run that speaks for a GPU-less browser session.
  None of the three layers is the oracle.
- **Every story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device.

## Dependency order and the shared resources

F-043's declared dependency is F-040. F-X002's declared dependency is F-037.
Both are `done`. No story in this sprint depends on another story in this
sprint, so no story begins blocked. The brick grid those uploads read is
F-036, also `done`.

**Two exclusive resources, and they are not the same one.**

**`crates/ocelli-render`** is where F-043's upload lands. F-X002 is a CI job
and a browser harness. If that harness edits device creation in
`ocelli-render`, it joins F-043's queue.

**The GPU**, for a device test and for layer 3's Chrome. Two workers running
device tests concurrently on one machine contend for the adapter and produce
timeouts that read exactly like rendering failures. The `gpu` gate runs
`--test-threads=1` for the same reason inside one process. Layer 2 is
`ubuntu-latest` and is not a result this machine can substitute.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is pinned
exactly and agents confidently emit APIs that have not existed for two years**,
so every wgpu call is checked against the pinned version's own documentation
rather than from memory. Treat GPU code that compiles first try with suspicion.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

A feature that cannot run on the resolved tier **reports unavailable**. It never
quietly produces a different result, which is HLD section 31's rule generalised
by D-07.
