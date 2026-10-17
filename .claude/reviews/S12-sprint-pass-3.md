# S12 whole-sprint review, pass 3

**Reviewed**: staged tree `1f2ab25d70e4210028224fdc6bd5255e66c49e0c` on `sprint/s12`. HEAD is `0ba66c7`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation since pass 2 is `git diff 9d0aaab65cd2e175284bc5ffdffba60bf800b758`, which changes `CLAUDE.md`, the F-041 close in `docs/sprints/AS_BUILT.md`, and the carried bullet in `docs/sprints/CURRENT_SPRINT.md`.
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, the oracle is said to receive a frame at F-040

**Where**: `crates/ocelli-render/src/graph.rs:8-10`, `docs/lld/render-graph.md:49-50`, and `docs/sprints/AS_BUILT.md:3336`

**What**: The crate header says a viewport renders into a caller-supplied texture view, from a caller-supplied stored-value texture, so the oracle still has no Ocelli frame to judge until F-040. `docs/lld/render-graph.md` says no texture is uploaded, which is F-040, and no frame exists for the oracle to judge until that lands. The F-038 entry says decision D7 stays open for F-039 and F-040.

**Why it is wrong**: The render half is true. `RenderGraph::tick` draws into the caller-supplied target and submits once, and `the_section_18_3_rows_render_through_the_graph` reads those pixels back. No `wgpu::Surface` is acquired under `crates/ocelli-render/src`. The oracle still has no candidate side. That absence does not end at F-040. F-040 is "Texture upload path: decode straight to write_texture" and depends on F-037, not on this graph. The F-012 entry records real Ocelli renderer activation as F-X021 after F-052. F-X021 is "Activate the full-corpus candidate-render gate against Ocelli output" and depends on F-012 and F-052. Upload does not write a candidate directory, and a canvas does not either. "Until F-040" and "until that lands" name the wrong story. "Stays open for F-039 and F-040" makes the same attribution.

**Evidence**:

```text
$ rg -n "create_surface|wgpu::Surface" crates/ocelli-render/src
(no matches)

$ rg -n "queue\(\).submit" crates/ocelli-render/src/graph.rs
811:        ctx.queue().submit(std::iter::once(encoder.finish()));
```

The readback is `crates/ocelli-render/tests/graph.rs:258-260`, asserting red samples `[0, 128, 255, 64]` after `tick`. F-040 is `docs/sprints/BACKLOG.md:166`. F-X021 is `docs/sprints/BACKLOG.md:212`. The activation sentence is `docs/sprints/AS_BUILT.md:1896-1897`. The GPU test was not re-run. The adapter is exclusive and a sprint gate is already recorded for this tree.

The pass 2 sentences that denied the render are gone. `CLAUDE.md:226-228`, `docs/sprints/AS_BUILT.md:3156-3159` and `docs/sprints/CURRENT_SPRINT.md:70-72` say the graph renders into a caller-supplied texture view, acquires no surface, and that the oracle still has no Ocelli renderer to compare against. Those three do not say the oracle can judge a frame, and they do not name F-040 as the close.

### D2, several current-state sentences still say there is no renderer

**Where**: `tools/oracle/src/lib.rs:19-20`, `docs/lld/comparator.md:22-23`, and `docs/lld/benchmarks.md:17-18`

**What**: The comparator module and its LLD say there is no Ocelli renderer, so there is no Ocelli frame. `docs/lld/benchmarks.md`, which this sprint edited to name F-038, still says eight subjects are blocked because there is no renderer, no worker and no boundary.

**Why it is wrong**: F-038 landed a renderer. `RenderGraph::tick` renders. "There is no Ocelli renderer" is false. The clause beside it, that the comparator has no second side of the corpus, is still true, and it is not this defect. "There is no renderer" in the benchmark file is the false conjunct. No worker and no boundary remain true, and `render.first_frame` still names F-101. The neighbouring "No port code exists" was already false before this sprint and is not filed.

**Evidence**: `rg -n "fn tick" crates/ocelli-render/src/graph.rs` prints the method at line 722. `docs/lld/benchmarks.md` is in the sprint diff. The header now lists F-038. Lines 17-18 were not changed. `tools/oracle/src/lib.rs` and `docs/lld/comparator.md` are outside that diff. F-038 is what made the renderer sentence false. `docs/lld/oracle.md:13-18` still waits on an Ocelli output for the reference half to compare, which is the candidate directory and is still absent.

### D3, the F-036 fixture sentence attaches the brick origin's offset to the voxel

**Where**: `docs/sprints/AS_BUILT.md:3389`

**What**: The entry says the 512 by 512 by 600 sixteen-bit fixture places voxel 200, 130, 599 in brick 3, 2, 9 at byte offset 302121344.

**Why it is wrong**: Brick 3, 2, 9 is the right brick. 302121344 is the byte offset of that brick's origin, voxel 192, 128, 576, not of voxel 200, 130, 599. Section 19's layout is `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`. For the voxel that is `((599 * 512 + 130) * 512 + 200) * 2`, which is 314182032. For the origin it is `((576 * 512 + 128) * 512 + 192) * 2`, which is 302121344. The two tests assert those facts separately. The sentence joins them into one address.

**Evidence**: `crates/ocelli-cache/tests/bricks.rs:26` asserts `brick_of([200, 130, 599]) == [3, 2, 9]`. Lines 32-40 assert `origin_of([3, 2, 9]) == [192, 128, 576]` and `byte_offset([192, 128, 576], 2) == 302121344`. The same arithmetic in Python prints `voxel 314182032` and `origin 302121344`. The eight-bit clause in the same sentence is the other fixture and is right: extent 116, 44, 2 is `116 * 44 * 2 = 10208` bytes.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Pass 1 and pass 2 were not treated as evidence. The pass 2 findings that the remediation corrected were re-read and are not re-filed.

**Tree.** `git write-tree` at the start, and again after the spacing line was restored, returned `1f2ab25d70e4210028224fdc6bd5255e66c49e0c`. `git diff -- crates/ocelli-volume/src/lib.rs` is empty.

**Recovery.** `Recovered` is `caps: Caps` and `graph: Option<GraphResources>` at `crates/ocelli-render/src/gpu.rs:111-116`. `recover` copies the registration, bumps `cache_generation`, and assigns the new context at line 317, which drops the old device handle. The coexistence comment at lines 299-305 says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old device handle is dropped at the assignment. It does not say the pipelines are dropped there. `install_recovered` replaces the pipelines and sets every per-viewport slot to `None` at `crates/ocelli-render/src/graph.rs:688-713`. `docs/lld/gpu-ownership.md:285-288` says `Recovered` carries caps and, when a graph registered them, `GraphResources`. A canvas and a texture are not types in that struct. F-039 and F-040 are still pending in `docs/sprints/BACKLOG.md`.

**Gpu gate.** The arm is `cargo test -p ocelli-render -- --ignored --test-threads=1` at `bin/ocelli.sh:344`. The comment names no test file and writes no count. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command. `cargo test -p ocelli-render -- --ignored --list` exited 0. The list includes `tests/graph.rs`, `tests/graph_voi.rs`, `tests/device.rs`, `tests/voi_shader.rs`, and the ignored rows in `gpu` and `probe`. No count is written here as something the comment should contain.

**Value types.** `impl Budgeted` for production values is `DecodedFrame` at `crates/ocelli-cache/src/image.rs:90`, `Brick` at `crates/ocelli-cache/src/brick.rs:344`, and `Volume` at `crates/ocelli-volume/src/lib.rs:554`. `Block` and `Shifting` are test doubles in `lru.rs`. `CLAUDE.md:220-222` names the three production types and says an encoded byte run and a GPU texture are not among them. `docs/lld/cache.md:145-151` says the same.

**LLD headers.** A comparison of every `docs/lld/*.md` header with its row in `docs/lld/README.md` found no mismatch. Twenty files, twenty rows. `render-graph.md` is F-038 on both. `gpu-ownership.md`, `cache.md`, `volumes.md` and `benchmarks.md` match their rows, including F-038 where the sprint added it.

**Cache kinds, budgets, brick events.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, at `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` pushes one `CacheEvent` per `Admission` field and does not collapse them. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `crates/ocelli-cache/src/brick.rs:368-369`. `docs/lld/cache.md:228-229` says that. `TierCounters::record` walks `admission.evicted` and increments `refused` when `refused` is set. It does not read `displaced`, so this pass does not claim that a displacement inflates `evicted`. `ImageCache` and `VolumeCache` are separate `Lru` values on `CacheTier::Decoded`, with separate budgets. `BrickCache` is `CacheTier::Gpu`.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness` as values. `PixelSpacing::new` takes index 0 as `row_mm`. The volume stores `[column_mm, row_mm, median]` beside direction columns row, column, normal. `row_spacing_multiplies_the_column_direction_cosine` expects spacing `[1.0, 0.5, 2.0]` for input `[0.5, 1.0]`, and the column step is `(0.0, 0.5, 0.0)`, so `PixelSpacing[0]` scales the column direction. One mutation replaced `position_vector().dot(normal)` with `position_vector().z`. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` failed at `assembly.rs:352` because `volume.is_ok()` was false, and cargo printed `test result: FAILED`. The shell then stopped on a read-only `status` assignment before its own restore. The line was edited back. `git diff` on that file is empty.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full, and assembly does not branch on tier. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend is skipped in `tick` and reported unavailable. It does not draw with the composite pipeline. None of those paths substitutes a different numeric result.

**D-25.** The sprint diff adds one deviation row. `Volume::present` is `SlicePresence` over `words: Vec<u64>` at `crates/ocelli-volume/src/lib.rs:221-222`. `rg bitvec` over `Cargo.toml` files finds nothing.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. The five cells still say pending. Not filed.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC, multi-component JPEG-LS and the unscheduled LINEAR range were carried in and not taken.

**Layout and submit.** A volume slice starts at `n * dims[0] * dims[1] * bytes_per_voxel`. `BrickGrid::byte_offset` is `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`. `tick` calls `queue().submit` once. The exception is D3, which cites the origin's offset as the voxel's.

**Boundary.** `rg wasm_bindgen` and `rg unsafe` over `crates/ocelli-cache`, `crates/ocelli-volume` and `crates/ocelli-render/src/graph.rs` find nothing. `subjects.json` still says F-038 presents no frame, which is the surface it does not acquire, and `subject_story` for `render.first_frame` is F-101.
