# Current sprint, S12

**Milestone**: M3, the cache and the renderer.
**Branch**: `sprint/s12`
**Opened**: 2026-09-15
**Goal**: Turn S11's three components into things a session can use. The cache
gets its consumers, its events and its brick addressing, and the renderer gets
the graph that schedules a frame.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-032 | E5.2 | Image cache with eviction events surfaced to JS | Rust | 2w | pending |
| F-033 | E5.3 | Volume cache and progressive volume assembly | Rust | 4w | pending |
| F-034 | E5.4 | Memory-pressure telemetry and JS-visible budget controls | Rust | 2w | pending |
| F-036 | E5.6 | Chunked residency model and brick addressing in the cache | Rust | 4w | pending |
| F-038 | E6.2 | Render graph and frame scheduler with dirty tracking | Rust | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S12 / {print $2, $9}'
```

## What this sprint is

**S11 built three components and connected none of them.** It created the first
long-lived `wgpu::Device`, a budgeted LRU holding no value type, and a shader
with no frame to run in. S12 is where each acquires a consumer: F-032 and F-033
give the cache the tiers it was built for, F-036 gives it the brick addressing
HLD section 38 calls a Phase 1 hook, F-034 makes its pressure visible to the
shell, and F-038 gives the renderer the graph that decides when to draw.

**A frame still does not reach a canvas at the close of this sprint.** F-039's
OffscreenCanvas and F-040's texture upload are S13, and they are what put pixels
on a surface. What changes here is that the pieces stop being independent.

## What is carried in

- **An open finding against `ocelli-pixel`'s own arithmetic**, recorded in
  `docs/lld/pixel-pipeline.md` by F-041 and **not scheduled**. A legal `LINEAR`
  chain returns a value outside its declared output range at one input per
  window, identically on CPU and GPU, because PS3.3 C.11.2.1.2's formula in
  `f32` does not reproduce its own breakpoint. It has no F-ID, because creating
  one moves the backlog's identity and the sprint plan's arithmetic and is the
  operator's decision. **Nothing in S12 touches that arithmetic**, and a story
  that begins consuming `Display` values should know the range is not a
  guarantee.
- **Tier B has still never been exercised**, by anything, in any sprint. F-037
  and F-041 both wrote for it and neither could try it. F-042 is the WebGL2
  story and it is S13. F-038's pipelines are the next code written for a tier
  nobody has run on.
- **The wasm size budget has not moved**, and still records 16,388 bytes.
  `ocelli-wasm` reaches `ocelli-core` and nothing else. Appendix A gate A4's
  measurement arrives with F-039 in S13, which `ci/wasm-size-budget.json` now
  says rather than naming S11.
- **`max_review_passes` is written by `sprint_workflow.py init` and read by
  nothing**, so `/run-sprint`'s "mark the sprint blocked" branch has no
  mechanism behind it. Found by S11's sprint review, outside that sprint's diff,
  and carrying no F-ID.
- **M2's release is still due and still cannot be taken.** `/release` step 5
  refuses while `openjph-core` 0.1.0 carries no BSD notice material, which is
  D-22, and M1's `0.1.0` was never published. Neither blocks a story here, both
  block publication, and the gap widens every milestone.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. **Multi-component JPEG-LS decoding** is still owed, with a corpus
  row measuring the refusal.
- **The oracle still has no Ocelli renderer to compare against**, and F-038 does
  not close that either. A render graph with no canvas and no texture upload
  produces no frame, so decision D7 holds until F-040.

## The defect class this sprint is exposed to

**S11 could be wrong in a way only hardware catches. S12 can be wrong in a way
only a second study catches.**

**Slice spacing taken from a tag.** This is the one to fear, and F-033 is where
it lives. `SpacingBetweenSlices` is frequently absent and frequently wrong, and
`SliceThickness` is the reconstructed slab and may overlap or gap. **The ground
truth is the difference between consecutive `ImagePositionPatient` values
projected onto the slice normal**, and nothing else. Reading the tag on an
overlapping-reconstruction CT compresses the volume along z by a constant
factor, which makes every sagittal and coronal reformat wrong while the axial
view looks perfect, and no measurement tool flags it. Sorting by
`InstanceNumber` or `SliceLocation` rather than by projected position is the
same defect one step earlier.

**Non-uniform spacing averaged rather than refused.** Dose-modulated and
multi-slab acquisitions really do produce variable gaps. A volume model that
assumes uniform spacing must detect and refuse, not silently average. Gantry
tilt makes the volume a sheared parallelepiped rather than a box, which F-020
already established must be **detected from the geometry** rather than read from
`GantryDetectorTilt`.

**A series is not a volume.** It can hold two orientations, a localiser mixed
with axials, and duplicate positions. F-033 groups by `FrameOfReferenceUID` and
`ImageOrientationPatient` before it assembles anything, or it builds one volume
out of two.

**A JS callback per eviction.** Decision **D4** is that events are polled from a
ring once per frame and never delivered as a callback per event, and F-032 and
F-034 are the first stories with a real event to emit. An eviction handler that
crosses the boundary per eviction is the hot-path crossing D4 exists to remove,
and it will look correct because it works.

**And pixels still do not cross the boundary**, which is decision D3. A cache
surfacing eviction events surfaces the event and not the frame.

**Chunked residency is the default path, not a fallback.** Deviation **D-11**
says so in terms. F-036 written as an optimisation that engages above a
threshold is the architecture the HLD explicitly refused, and section 7's own
figure is why: a 512 by 512 by 600 sixteen-bit series is roughly 300 MB against
a guaranteed maximum buffer of 256 MiB, so the chunked path is the ordinary one.

**One submit per frame, across all viewports.** HLD section 22, and F-038 is the
first code that can break it. So is "pipelines compile at init, never
mid-frame", and so is section 20's "no allocation in the render loop", which
until now has been a rule with no render loop to apply to. **F-038 is where
those three stop being aspirations.**

**Device loss rebuilds resources, and F-037 deliberately did not.** Section 22
asks for the device and all resources. F-037 rebuilt the device and returned
`Recovered` carrying only `Caps`, because no resource type existed. F-038 is the
first story that creates resources a rebuild has to restore, so it extends
`Recovered` rather than changing `recover`'s signature.

## What done means

- **F-032** surfaces evictions the cache already reports. `Lru::insert` returns
  `Admission` with `evicted`, `displaced` and `refused` distinguished, which is
  deviation D-24 and exists precisely so these become three different events.
  Collapsing them at the boundary throws away what F-031 was built to preserve.
- **F-033** derives spacing from projected `ImagePositionPatient`, refuses
  non-uniform spacing rather than averaging it, and groups by frame of reference
  and orientation before assembling. **Every one of those is a fixture with
  hand-computed values citing its PS3.3 section**, and the spacing fixture uses
  a series whose tag and whose projected spacing disagree, because one where
  they agree cannot tell the two implementations apart.
- **F-034** reports pressure per tier, which is what `CacheTier` and `Pressure`
  are for, and crosses the boundary through the ring rather than a callback.
- **F-036** makes bricking the ordinary path and says so in its design plan, per
  D-11. A threshold above which it engages is a deviation and needs a row.
- **F-038** issues one `queue.submit()` per frame across all viewports, compiles
  every pipeline at init, allocates nothing in the render loop, and extends
  `Recovered` with the resources a device rebuild must restore.
- **Every one of the five declares all three tier rows**, and "n/a" is a real
  answer that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a device.

## Dependency order and the shared resources

F-032, F-033, F-034 and F-036 all depend on F-031. F-038 depends on F-037. All
of those are `done`, so no story begins blocked and there is no order between
the five.

**Two exclusive resources, and they are not the same one.**

**`crates/ocelli-cache`** is written by four of the five stories. They cannot
share a wave without conflicting on the same source files, whatever the worker
count says. F-038 touches `ocelli-render` and can run beside any one of them.

**The GPU**, for F-038 alone this sprint. The rule S11 established holds: two
workers running device tests concurrently on one machine contend for the adapter
and produce timeouts that read exactly like rendering failures. The `gpu` gate
runs `--test-threads=1` for the same reason inside one process.

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
