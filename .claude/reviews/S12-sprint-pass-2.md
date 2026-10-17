# S12 whole-sprint review, pass 2

**Reviewed**: staged tree `9d0aaab65cd2e175284bc5ffdffba60bf800b758` on `sprint/s12`. HEAD is `0ba66c7`. The sprint diff is `git diff 4976c929b8a7c0c081f6aeb2216dd5414571377c`. The remediation is `git diff HEAD`, staged, and the worktree matches that index for those paths.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the frame sentences still say there is no frame

**Where**: `CLAUDE.md:226-227` and `docs/sprints/AS_BUILT.md:3157-3158`

**What**: The remediation kept "nothing renders a frame" and "there is no frame", then added a sentence that F-038 schedules a frame and acquires no surface. `CLAUDE.md` says:

> **Nothing renders a frame yet**, so decision D7 still holds. The oracle has no Ocelli renderer to compare against. F-038 schedules a frame and acquires no surface.

The F-041 entry in `AS_BUILT.md` says the oracle still has no renderer to compare against, "because there is no frame", and then "F-038 schedules a frame and acquires no surface."

**Why it is wrong**: `RenderGraph::tick` renders. It begins a render pass on the caller-supplied target, draws, and submits once. The crate header says a viewport renders into a caller-supplied texture view. `the_section_18_3_rows_render_through_the_graph` reads those pixels back. No `wgpu::Surface` is acquired anywhere under `crates/ocelli-render/src`, so this is not a presented frame and the oracle still has nothing to judge. Decision D7 stays open for that reason. "Nothing renders a frame yet" and "because there is no frame" are the false halves. The surface sentences beside them are true, and they use "frame" for the thing the first halves deny. `crates/ocelli-render/src/graph.rs` already states the split that these two edits missed.

**Evidence**:

```text
$ rg -n "Nothing renders a frame yet|there is no frame|schedules a frame" \
    CLAUDE.md docs/sprints/AS_BUILT.md
CLAUDE.md:226:  **Nothing renders a frame yet**, so decision D7 still holds. The oracle has
CLAUDE.md:227:  no Ocelli renderer to compare against. F-038 schedules a frame and acquires
docs/sprints/AS_BUILT.md:3157:still has no Ocelli renderer to compare against, because there is no frame. What
docs/sprints/AS_BUILT.md:3158:it closes is the arithmetic half. F-038 schedules a frame and acquires no

$ rg -n "create_surface|wgpu::Surface" crates/ocelli-render/src
(no matches)

$ rg -n "queue\(\).submit" crates/ocelli-render/src/graph.rs
811:        ctx.queue().submit(std::iter::once(encoder.finish()));
```

`rg` on the source tree exited 1 for the surface search, which is the empty result, and 0 for the other two. The readback is `crates/ocelli-render/tests/graph.rs:258-260`, asserting red samples `[0, 128, 255, 64]` after `tick`. The header that already draws the right line is `crates/ocelli-render/src/graph.rs:7-10`. The GPU test was not re-run. The adapter is exclusive and a sprint gate is already recorded for this tree.

## Smells

None.

## Nitpicks

None.

## Verified clean

Checked on this tree. Pass 1 was not treated as evidence.

**Tree.** `git write-tree` at the start and after the spacing mutation was restored returned `9d0aaab65cd2e175284bc5ffdffba60bf800b758`. `git diff -- crates/ocelli-volume/src/lib.rs` was empty after the restore.

**D1 from pass 1, recovery.** `Recovered` is `caps: Caps` and `graph: Option<GraphResources>` at `crates/ocelli-render/src/gpu.rs:111-116`. `GraphResources` is `pipelines_restored: u32` and `cache_generation: u64`. `register_graph_resources` stores that count and keeps the generation. `recover` copies the registration, adds one to the generation, assigns the new context, and returns `Recovered { caps, graph }`. That assignment drops the old device handle. It does not drop the graph's pipelines. `install_recovered` recompiles pipelines and sets every per-viewport slot to `None` at `crates/ocelli-render/src/graph.rs:688-713`. The coexistence comment at `gpu.rs:299-305` says the copied registration is the count and the generation, the caller reinstalls the pipelines, and the old device handle is dropped at the assignment. It does not say the pipelines are dropped there. `docs/lld/gpu-ownership.md:285-288` no longer says there are no resources, and it no longer says `Recovered` carries only `Caps`. It does not count section 22 clauses. The HLD bullet is one sentence in `docs/hld/19-render-graph.md:40`. No `Canvas` type exists. F-039 and F-040 are still pending in `docs/sprints/BACKLOG.md`. The sentence that they extend the struct is about those unlanded stories.

**D2 from pass 1, the gpu gate.** The arm is still `cargo test -p ocelli-render -- --ignored --test-threads=1`, the same command as in `4976c929`. The comment no longer enumerates a set and names no test file. `docs/lld/gpu-ownership.md:304-306` says the gate is every `#[ignore]`d test in the crate and points at the list command. `cargo test -p ocelli-render -- --ignored --list` exited 0. The list includes `tests/graph.rs` and `tests/graph_voi.rs` as well as the device, probe, and `voi_shader` rows. No count is written here as something the comment should contain.

**D3 value types, the half that is true.** `impl Budgeted` for production values is `DecodedFrame` at `crates/ocelli-cache/src/image.rs:90`, `Brick` at `crates/ocelli-cache/src/brick.rs:344`, and `Volume` at `crates/ocelli-volume/src/lib.rs:554`. `GpuTexture` in `crates/ocelli-cache/tests/budget.rs` is a test double. `CLAUDE.md:220-222` names the three production types and says an encoded byte run and a GPU texture are not among them. That part matches. The frame sentence in the same bullet is D1 above.

**S1.** `docs/lld/render-graph.md:3` is F-038 and the README row is F-038. `docs/lld/gpu-ownership.md:3` and its README row are both F-004, F-005, F-008, F-037, F-038, F-041, F-X001. A comparison of every `docs/lld/*.md` header with its README row found no mismatch. `oracle.md` wraps its header onto the next line and the joined IDs match the row.

**N1.** `docs/lld/benchmarks.md:3-4` now names F-038 and says 2026-10-17, matching the README row. The `render.first_frame` row was not changed by the remediation. It is still unavailable and names F-101, with F-037, F-038, F-039 and F-040 before that. `tools/bench/subjects.json` sets `subject_story` to F-101 and says F-038 presents no frame.

**`voi_shader.rs` module docs.** The edit no longer says the oracle verdict is F-038 and F-040. It says the verdict arrives when there is a renderer to render one, and that F-038 schedules a frame and acquires no surface. That does not call the tick a presented oracle frame. The tier B sentence is unchanged. Tier B is still unexercised, which `docs/lld/render-graph.md:51-52` says, with F-042 named as the first test.

**F-041 "What this closes", apart from D1.** The entry still says the shader does not close decision D7 and that the oracle has no Ocelli renderer to compare against. Both of those are true. The false causal clause is D1. The arithmetic-half sentences around it were not the new claim.

**Cache kinds, budgets, brick events.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, in `crates/ocelli-cache/src/image.rs:130-141`. `ImageCache::insert` builds one `CacheEvent` per `Admission` field and does not collapse them. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, at `crates/ocelli-cache/src/brick.rs:368-369`. `TierCounters::record` counts `evicted` and `refused` and does not read `displaced`, so a displacement does not increment `evicted`. `ImageCache` and `VolumeCache` are separate `Lru` values on `CacheTier::Decoded`, with separate budgets. `BrickCache` is `CacheTier::Gpu`. `docs/lld/cache.md` says the same.

**F-033 spacing.** Inter-slice spacing is the median of `position_vector().dot(slice_normal())` at `crates/ocelli-volume/src/lib.rs:425-493`. The crate does not read `SpacingBetweenSlices` or `SliceThickness`. Those names appear only in comments. `PixelSpacing::new` takes index 0 as `row_mm`. The volume stores `[column_mm, row_mm, median]` beside direction columns row, column, normal. `row_spacing_multiplies_the_column_direction_cosine` expects `[1.0, 0.5, 2.0]` for input `[0.5, 1.0]`, so `PixelSpacing[0]` is the column cosine's spacing. One mutation, `position_vector().dot(normal)` replaced with `position_vector().z`, and `an_oblique_series_sorts_by_projection_and_not_by_ipp_z` failed at `assembly.rs:352` because assembly refused the volume. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` exited 101. The line was edited back. `git diff` on that file is empty.

**Decision D7.** No Ocelli output is handed to the oracle. `RenderGraph::new` acquires no surface. D7 stays open. The defect above is the wording that denies the offscreen render, not a claim that the oracle can judge one.

**Tiers.** The five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full, and assembly does not branch on tier. F-036 is full on all three for host arithmetic, with no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. `RenderGraph::new` returns `TierCUnavailable` when the tier is `Cpu`. An uncompiled blend reports unavailable and does not alias composite. None of those paths substitutes a different numeric result.

**D-25.** The sprint diff adds one deviation row. `Volume::present` is `SlicePresence` over `Vec<u64>`. `rg bitvec` over `Cargo.toml` files finds nothing.

**Status column.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the column is hand-typed and that the backlog is the authority. The five cells still say pending. The backlog rows say done. Not filed.

**Not claimed done.** The sprint still says the M2 release, deviation D-22, F-X011, ICC and the unscheduled LINEAR range were carried in and not taken.

**Layout and submit.** A volume slice starts at `n * dims[0] * dims[1] * bytes_per_voxel`. `BrickGrid::byte_offset` is `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`. `tick` calls `queue().submit` once.
