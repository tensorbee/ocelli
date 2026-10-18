# Current sprint, S13

**Milestone**: M3, the cache and the renderer.
**Branch**: `sprint/s13`
**Opened**: 2026-10-17
**Goal**: Move a decoded frame from the decode worker onto a surface the render
worker owns, on both GPU tiers, without a pixel becoming a JavaScript object.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-035 | E5.5 | Zero-copy handoff strategy from decode worker to renderer | Rust | 2w | pending |
| F-039 | E6.3 | OffscreenCanvas in a render worker; resize and DPR handling | Rust | 3w | pending |
| F-040 | E6.4 | Texture upload path: decode straight to write_texture | Rust | 4w | pending |
| F-042 | E6.6 | WebGL2 fallback and tier-B shader variants | Rust | 5w | pending |
| F-044 | E6.8 | Colour LUT / transfer function upload and preset library | Rust | 2w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S13 / {print $2, $9}'
```

## What this sprint is

**S12 scheduled a frame and presented none.** The graph draws into a
caller-supplied texture view and acquires no surface. S13 is the handoff and
the first surface: F-035 carries bytes from a decode worker to the renderer,
F-039 puts an OffscreenCanvas in the one render worker and sizes it in device
pixels, F-040 uploads decoded samples with `write_texture`, F-042 is the first
code that has to run on tier B, and F-044 uploads a colour lookup and a preset
library.

M3 runs from S11 through S14. This sprint does not finish it, and a release is
a milestone boundary.

## What is carried in

- **An open finding against `ocelli-pixel`'s own arithmetic**, recorded in
  `docs/lld/pixel-pipeline.md` by F-041 and **not scheduled**. A legal `LINEAR`
  chain returns a value outside its declared output range at one input per
  window, identically on CPU and GPU, because PS3.3 C.11.2.1.2's formula in
  `f32` does not reproduce its own breakpoint. It has no F-ID, because creating
  one moves the backlog's identity and the sprint plan's arithmetic and is the
  operator's decision. S13 is the first sprint that uploads samples and presets
  for display. A story that begins consuming `Display` values should know the
  range is not a guarantee.
- **Tier B has still never been exercised**, by anything, in any sprint. F-042
  is the story that has to. Until it lands, a tier B row that says full is a
  claim about a path nobody has run.
- **The wasm size budget records 1,999,061 bytes.** `ci/wasm-size-budget.json`
  records that size. Its `bearing_on_gate_A4` field says this is the first time
  the module reaches wgpu and Naga and that the number does not close A4.
  `ocelli-wasm` depends on `ocelli-core`, `ocelli-render` and `wgpu`. The
  dependency has a user, the OffscreenCanvas attach path.
- **`max_review_passes` is written by `sprint_workflow.py init` and read by
  nothing**, so `/run-sprint`'s "mark the sprint blocked" branch has no
  mechanism behind it. Found by S11's sprint review, outside that sprint's
  diff, and carrying no F-ID.
- **M2's release is still due and still cannot be taken.** `/release` step 5
  refuses while `openjph-core` 0.1.0 carries no BSD notice material, which is
  D-22, and M1's `0.1.0` was never published. S12 did not finish M3, so closing
  it did not make a release due. Neither blocks a story here. Both block
  publication.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. **Multi-component JPEG-LS decoding** is still owed, with a corpus
  row measuring the refusal.
- **The oracle still has no Ocelli renderer to compare against.** F-040 uploads
  a texture the graph can sample. It does not write the candidate record.
  F-X021 activates that gate and depends on F-012 and F-052. Decision D7 holds.

## The defect class this sprint is exposed to

**S12 could be wrong in a way a second study catches. S13 can be wrong in a
way a screenshot will not show.**

**A view kept across an allocation.** This is the one to fear, and F-035 is
where it lives. HLD section 5 builds the typed-array view immediately before
the write and discards it after. Any wasm memory growth relocates the backing
buffer and detaches every outstanding view. A module-level
`Uint8Array` over `wasm.memory.buffer` then writes a detached buffer, or
throws far from the cause. The handoff is a pointer and a length. The pixels
do not become JavaScript objects, which is decision D3.

**A texture upload standing in for a uniform.** HLD section 18.4's window and
level are thirty-two bytes. F-040 uploads decoded samples, once. Re-uploading
the image on a window change, or uploading values the shader will window
again, is the quietly wrong pixel. At the centre of a soft-tissue CT window,
`LINEAR` and `LINEAR_EXACT` differ by 0.32 of 255. Invisible in a screenshot.
Immediately visible in a pixel diff.

**CSS pixels on a high-density display.** F-039. A canvas whose backing store
is the CSS size, on a display whose `devicePixelRatio` is not 1, looks slightly
soft and reads as acceptable. One `GPUDevice` serves every viewport. There is
no context pool. A resize that leaves the previous target in place draws the
new frame into the old rectangle.

**Tier B returning another number.** F-042. Tier B is fragment shaders only.
No compute and no storage buffers. The 3D texture floor is 256. A variant that
compiles and produces a different value from tier A is a failure of decision
D6, which says compute degrades and never fails. A feature that cannot run on
the resolved tier reports unavailable. It never quietly produces a different
result, which is deviation D-07.

**A colour lookup applied to samples that were already converted.** F-044. The
colour stage runs once, against the decoder's own reported output space. A
preset that runs a conversion again, or that indexes a table with a windowed
value when the table is keyed on the stored value, has the right shape and the
wrong colours. That is the shape of deviation D-23.

## What done means

- **F-035** builds the view at the write and does not retain it. A fixture
  grows wasm memory between two uses. Keeping the view fails that fixture.
  Collapsing the handoff into a copied `Uint8Array` that outlives the call
  fails decision D3.
- **F-039** owns the OffscreenCanvas in the render worker. Resize and
  `devicePixelRatio` both replace the backing store. One device serves the
  viewports.
- **F-040** writes decoded samples with `write_texture`. A window or level
  change updates the uniform and does not re-upload the image. The bytes do
  not cross into JavaScript.
- **F-042** resolves tier B and runs the shader variants that tier can run.
  What the tier cannot run reports unavailable and does not alias the tier A
  pipeline.
- **F-044** uploads a lookup and a preset. It does not re-derive modality, VOI
  or photometric interpretation. The uniform still carries the resolved
  decision.
- **Every story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device.

## Dependency order and the shared resources

F-035 depends on F-033. F-039 and F-040 depend on F-037. F-042 and F-044
depend on F-041. F-033, F-037 and F-041 are `done`, so no story begins blocked.
No story in this sprint depends on another story in this sprint.

**Two exclusive resources, and they are not the same one.**

**`crates/ocelli-render`** is where F-039, F-040, F-042 and F-044 land. They
cannot share a wave without conflicting on the same source files, whatever the
worker count says.

**The GPU**, for those four. The rule S11 established holds: two workers
running device tests concurrently on one machine contend for the adapter and
produce timeouts that read exactly like rendering failures. The `gpu` gate
runs `--test-threads=1` for the same reason inside one process.

**F-035** is the memory-view contract between a decode worker and that
renderer. It can be designed beside the render stories. If its implementation
edits `ocelli-render`, it joins their queue.

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
