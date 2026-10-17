# S12 whole-sprint review, pass 4

**Reviewed**: staged tree `67a5e5abe1c610af67c73486b9a29886adf84c65` on `sprint/s12`. HEAD is `0ba66c7`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since HEAD is `git diff HEAD`.
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, the benchmark split still does not match the registry

**Where**: `docs/lld/benchmarks.md:16-22` and the table at `docs/lld/benchmarks.md:162-176`, and `tools/bench/subjects.json:92`

**What**: The opening says two of eleven subjects have a runner and a recorded number, eight are blocked on a story that has not landed because there is no worker and no boundary, and `tier.startup_microbenchmark` has no runner. The table calls itself the state of every subject and still marks HTJ2K and JPEG-LS unavailable on F-027 and F-028. It omits JPEG 2000. It says no runner has been written for the fill-rate subject. The `render.first_frame` cell this sprint wrote is blocked on F-101 and, before that, on F-037, F-038, F-039 and F-040. The subject note says the same chain is what blocks the row, then says F-038 has landed and the story is now F-101.

**Why it is wrong**: The same file says `bin/ocelli.sh bench --list` is the authority on the split. That list has twelve subjects, not eleven. Six have a pending `subject_story`: three name F-101 and three name F-X003. None names the render-worker story, F-039. F-X003 is CPU raster, not the absence of a worker or a boundary. F-027 and F-028 are `done` in `docs/sprints/BACKLOG.md`. Runners exist for both, and for JPEG 2000 and for `tier.startup_microbenchmark`. `ci/bench-baseline.json` records numbers for `wasm.cold_start`, `decode.frame`, `decode.transfer_syntax.htj2k`, `decode.transfer_syntax.jpeg2000` and `decode.transfer_syntax.jpegls`. That is five recorded subjects, not two, and the fill-rate runner is not absent. F-037 and F-038 are `done`. A row is not blocked on a story that has landed. Removing "no renderer" from the because-clause left a cause that does not fit the six pending rows, and it left the table the sprint retitled as current on 2026-10-17.

**Evidence**:

```text
$ node tools/bench/run.mjs --list
  decode.transfer_syntax.htj2k     F-027 (done)
  decode.transfer_syntax.jpeg2000  F-026 (done)
  decode.transfer_syntax.jpegls    F-028 (done)
  render.first_frame               F-101 (pending)
  interaction.window_level_drag    F-101 (pending)
  interaction.scroll               F-101 (pending)
  session.cpu_interactive          F-X003 (pending)
  session.cpu_idle                 F-X003 (pending)
  cine.frame_change_rate           F-X003 (pending)
  tier.startup_microbenchmark      F-004 (done)
```

Twelve subjects are printed. Pending stories are the six above. `ls tools/bench/src/runners` includes `decode_transfer_syntax_htj2k.mjs`, `decode_transfer_syntax_jpeg2000.mjs`, `decode_transfer_syntax_jpegls.mjs` and `tier_startup_microbenchmark.mjs`. The runner's own comment is "The permanent runner for F-004's existing fill-rate instrument." Baseline subject keys are the five names above. Backlog rows for F-026, F-027, F-028, F-037 and F-038 say `done`. F-039, F-040, F-101 and F-X003 say `pending`. The bench was not executed. The GPU is exclusive and a sprint gate is already recorded for this tree. The contradiction is between the tracked registry, the backlog and the baseline.

### D2, the oracle still says no volume builder exists

**Where**: `tools/oracle/src/volume.mjs:22-24`, `tools/oracle/volume-truth.json:11`, `docs/lld/oracle.md:905` and `docs/lld/oracle.md:977-978`

**What**: The volume module, its truth file and the oracle LLD say no volume builder exists, so the reference decides nothing about a non-uniform series. The LLD also says the normative status of the spacing rule matters to the future volume builder.

**Why it is wrong**: F-033 landed `Volume::assemble`. It sorts on the projected position, takes the median gap, and returns `VolumeError::NonUniformSpacing` when a gap departs from that median by more than 0.5 percent. That is a volume builder, and it is the refusal the oracle text says has not been decided. The backlog row for F-033 is `done`. F-051, volume construction for a later reslice, is still `pending` and is not what this sentence is waiting on. These three files are outside the sprint diff. F-033 is what made the present-tense claim false. The separate claim that no HLD section states the 0.5 percent threshold was not re-opened. A grep of `docs/hld/` for a non-uniform spacing policy still does not find one.

**Evidence**: `crates/ocelli-volume/src/lib.rs:398` is `Volume::assemble`. Lines 485-490 return `NonUniformSpacing` with the median, the worst gap and the index. `docs/sprints/BACKLOG.md:159` is F-033 `done`. `docs/sprints/BACKLOG.md:198` is F-051 `pending`. `docs/sprints/AS_BUILT.md:3352` records the median refusal. The oracle lines were not in `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`.

### D3, AS_BUILT and the GPU LLD still say there is no texture, buffer, pipeline or viewport

**Where**: `docs/sprints/AS_BUILT.md:3042-3043`, `docs/sprints/AS_BUILT.md:3165`, and `docs/lld/gpu-ownership.md:271`

**What**: The F-037 close says this workspace has no texture, buffer, pipeline or viewport type to rebuild. The F-041 close, in the section this remediation edited, says there is no texture type yet for a stored value to arrive through. The recovery section of `docs/lld/gpu-ownership.md` says this workspace has no texture type for either device to hold.

**Why it is wrong**: F-038 added those types. `RenderGraph` stores `pipelines: Vec<wgpu::RenderPipeline>` and a `GpuSlot` of `wgpu::TextureView`, `wgpu::Buffer` and two bind groups, one slot per viewport. `install_recovered` recompiles the pipelines and sets every per-viewport slot to `None`. The fragment entry appended to `VOI_WGSL` declares `texture_2d<f32>` and reads it with `textureLoad`. `docs/lld/README.md` says an LLD describes what the code does today. The F-038 entry in the same AS_BUILT file already describes the pipelines, the viewport slots and the caller-supplied texture view. The three sentences above still deny those types. `Recovered` carrying a pipeline count rather than the pipeline objects does not make the types absent.

**Evidence**:

```text
$ rg -n "pipelines: Vec|struct GpuSlot|texture_2d" crates/ocelli-render/src/graph.rs
214:@group(1) @binding(0) var stored_tex : texture_2d<f32>;
245:struct GpuSlot {
311:    pipelines: Vec<wgpu::RenderPipeline>,
```

`install_recovered` is `crates/ocelli-render/src/graph.rs:688-713`. The F-037 and F-041 sentences are present tense in `docs/sprints/AS_BUILT.md`, which this sprint edited. The GPU LLD sentence is in a file this sprint edited and was not part of that edit.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Passes 1 to 3 were not treated as evidence. The pass 3 sentences that the remediation rewrote were re-read.

**Tree.** `git write-tree` at the start, and again after the spacing line was restored, returned `67a5e5abe1c610af67c73486b9a29886adf84c65`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty.

**Pass 3 D1, the oracle frame.** `rg` for `until F-040`, `until that lands`, `stays open for F-039`, `no port code exists`, `no frame exists` and `there is no Ocelli renderer` prints nothing. `crates/ocelli-render/src/graph.rs:8-10` and `docs/lld/render-graph.md:49-50` say the oracle still has no Ocelli frame to judge and do not name F-040, F-039 or F-X021 as the close. The F-038 entry says decision D7 stays open and does not name those stories. That absence is accepted because the comparator still has no candidate side. `rg` for a candidate writer under `crates/ocelli-render` finds nothing. F-040 in `docs/sprints/BACKLOG.md:166` is texture upload and depends on F-037. F-X021 at `docs/sprints/BACKLOG.md:212` is the full-corpus candidate-render gate and depends on F-012 and F-052. The F-012 entry still says real Ocelli renderer activation is F-X021 after F-052. `rg` for `create_surface` and `wgpu::Surface` under `crates/ocelli-render/src` finds nothing. `RenderGraph::tick` calls `queue().submit` once, at `crates/ocelli-render/src/graph.rs:811`. The other `queue().submit` is the recovery test probe in `gpu.rs`, not a second submit on the frame path.

**Pass 3 D2, the three edited paragraphs.** `tools/oracle/src/lib.rs:18-24` and `docs/lld/comparator.md:21-28` no longer say there is no Ocelli renderer and no longer say no port code exists. They say the comparator has no second side of the corpus. The three directory fillers are hand-built fixtures, identity over the reference, and the mutation catalogue. None of those is `RenderGraph`. "When the port lands" is the candidate-directory writer, which is still absent. `docs/lld/comparator.md:1297` says no Ocelli producer exists and names F-X021. That producer is the candidate record, not the graph. The remaining "no Ocelli renderer to compare against" in `CLAUDE.md`, the F-041 close and `docs/sprints/CURRENT_SPRINT.md` sits beside a sentence that the graph renders into a caller-supplied texture view and acquires no surface. Those sentences do not say the oracle can judge a frame. They are not the unqualified denial pass 3 filed. The false benchmark conjunct is D1, not a leftover "no renderer".

**Pass 3 D3, the brick offset.** The F-036 sentence now says voxel 200, 130, 599 is in brick 3, 2, 9, and that brick's origin, voxel 192, 128, 576, is at byte offset 302121344. `crates/ocelli-cache/tests/bricks.rs:26` asserts the brick. Lines 32-40 assert the origin and `byte_offset` of that origin. Independent products, not the comment above the assertion: `200 // 64`, `130 // 64` and `599 // 64` are 3, 2 and 9. Times 64 that origin is 192, 128, 576. Section 19's `((z * dy + y) * dx + x) * 2` is 302121344 for the origin and 314182032 for the voxel. `checked_linear` is that product. The eight-bit extent 116 by 44 by 2 is 10208 bytes, which the edge-brick test asserts as `samples.len()` and `bytes()`.

**Recovery.** `Recovered` is `caps: Caps` and `graph: Option<GraphResources>` at `crates/ocelli-render/src/gpu.rs:111-116`. `recover` copies the registration, bumps `cache_generation`, and assigns the new context at line 317, which drops the old device handle. The coexistence comment at lines 299-305 says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old device handle is dropped at the assignment. It does not say the pipelines are dropped there. `install_recovered` replaces the pipelines and sets every per-viewport slot to `None`. `docs/lld/gpu-ownership.md:285-288` says `Recovered` carries caps and, when a graph registered them, `GraphResources`. A canvas and a texture are not types in that struct. The exception is D3, the earlier sentence in the same file that still says there is no texture type.

**Gpu gate.** The arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` at `bin/ocelli.sh:344`. The comment names no test file and writes no count. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command. `cargo test -p ocelli-render -- --ignored --list` exited 0. The list includes `tests/graph.rs`, `tests/graph_voi.rs`, `tests/device.rs`, `tests/voi_shader.rs`, and the ignored rows in `gpu` and `probe`. No count is written here as something the comment should contain.

**Value types.** `impl Budgeted` for production values is `DecodedFrame` at `crates/ocelli-cache/src/image.rs:90`, `Brick` at `crates/ocelli-cache/src/brick.rs:344`, and `Volume` at `crates/ocelli-volume/src/lib.rs:554`. `Block`, `Shifting`, `GpuTexture` and `DecodedSeries` are test doubles. `CLAUDE.md:220-222` names the three production types and says an encoded byte run and a GPU texture are not among them. `docs/lld/cache.md:145-151` says the same.

**LLD headers.** A comparison of every `docs/lld/*.md` F-ID header with its row in `docs/lld/README.md` found no mismatch. Twenty files, twenty rows. `comparator.md` still lists F-011, F-012, F-013, F-015 and F-X012 on both sides. Its date changed. The index does not store the date. `render-graph.md` is F-038 on both. `gpu-ownership.md`, `cache.md`, `volumes.md` and `benchmarks.md` match their rows, including F-038 where the sprint added it.

**Cache kinds, budgets, brick events.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, at `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` pushes one `CacheEvent` per `Admission` field and does not collapse them. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `crates/ocelli-cache/src/brick.rs:368-369`. `docs/lld/cache.md:228-229` says that. `TierCounters::record` walks `admission.evicted` and increments `refused` when `refused` is set. It does not read `displaced`. This pass does not claim that a displacement inflates `evicted`. `ImageCache` and `VolumeCache` are separate `Lru` values on `CacheTier::Decoded`, with separate budgets. `BrickCache` is `CacheTier::Gpu`.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing` input `[0.5, 1.0]` is stored as spacing `[1.0, 0.5, 2.0]`, and `row_spacing_multiplies_the_column_direction_cosine` expects the column step `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column direction. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false, and the test command's status was 101. The line was edited back. `git diff` on that file is empty.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full, and assembly does not branch on tier. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable. It does not draw with the composite pipeline. None of those paths substitutes a different numeric result.

**D-25.** The sprint diff adds one deviation row. `Volume::present` is `SlicePresence` over `words: Vec<u64>` at `crates/ocelli-volume/src/lib.rs:221-222`. `rg bitvec` over `Cargo.toml` files finds nothing.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. The five cells still say pending. Not filed.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` find nothing. The graph does not reference `ImageCache`, `BrickCache` or `VolumeCache`. That split is what F-040 and F-043 are for, and the sprint prose that was checked does not say the graph reads the cache.
