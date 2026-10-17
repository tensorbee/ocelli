# S12 whole-sprint review, pass 1

**Reviewed**: `git diff 4976c929b8a7...HEAD` on sprint/s12, HEAD `0ba66c7`, tree `139e3cf756bc`. 56 files, about 8900 insertions. Commits `3fe48d3`, `c827431`, `773d338`, `c2efd13`, `4629015`, `5bed3e6`, `0ba66c7`.
**Scope**: whole-sprint interactions. The five per-story loops were not re-run.
**Result**: 3 defects, 1 smell, 1 nitpick

## Defects

### D1, `gpu-ownership.md` still says recovery carries only caps and that no resources exist

**Where**: `docs/lld/gpu-ownership.md:285-289`, and the same claim in `crates/ocelli-render/src/gpu.rs:299-301`

**What**: F-038 updated the type table in this file. `Recovered` is "caps, and from F-038 the graph resources it restored", and `GraphResources` is "pipelines restored and the graph cache generation". The recovery section, which that edit did not touch, still says:

> There are no resources and no viewports in this workspace, so `Recovered` carries the rebuilt device's `Caps` and nothing else. It is a struct rather than a bare `Caps` so F-038's render graph, F-039's canvas and F-040's textures extend it instead of changing `recover`'s signature.

`GpuContext::recover` repeats the premise in the coexistence comment: the old device "owns no resource this workspace has a type for".

**Why it is wrong**: F-038 is in this tree. `Recovered` has `graph: Option<GraphResources>`. `RenderGraph` registers viewports and compiles a pipeline, a uniform buffer and bind groups. The paragraph is the future tense about work under review, and it contradicts the type table in the same file. A reader who stops at the recovery section will believe F-037's residue is still the whole story.

**Evidence**: the sprint diff for this file changes the header, the `Recovered` row and the new `GraphResources` row. It does not change lines 285-289. Against the code:

```
$ rg -n "pub struct Recovered|pub graph:|pub struct GraphResources|There are no resources" \
    crates/ocelli-render/src/gpu.rs docs/lld/gpu-ownership.md
crates/ocelli-render/src/gpu.rs:84:pub struct GraphResources {
crates/ocelli-render/src/gpu.rs:111:pub struct Recovered {
crates/ocelli-render/src/gpu.rs:116:    pub graph: Option<GraphResources>,
docs/lld/gpu-ownership.md:286:There are no resources and no viewports in this workspace, so `Recovered`
```

`rg` exited 0. The coexistence comment is `crates/ocelli-render/src/gpu.rs:300-301`.

### D2, the gpu-gate gloss still describes the set from before F-038

**Where**: `bin/ocelli.sh:303-305`, and the same enumeration in `docs/lld/gpu-ownership.md:304-307`

**What**: the gate comment says the definition is every `#[ignore]`d test in `ocelli-render`, then:

> What the set is today, with F-031, F-037 and F-041 all landed: the device lifecycle, the tier C short-circuit test, the fill-rate instrument noted below, and F-041's LUT-shader comparison against `ocelli-pixel`.

Two paragraphs later it says F-038, F-040 and F-042 add to the crate, as if F-038 had not landed. The LLD repeats the four-item list as "every `#[ignore]`d test in the crate".

**Why it is wrong**: F-038 landed in this sprint and added ignored tests in `tests/graph.rs` and `tests/graph_voi.rs`. The comment warns that a gloss over a growing set goes stale the first time a test joins it for a new reason, and then keeps a gloss that F-038 made stale. The definitional sentence is still true. The enumeration is not. This is the same failure S11 pass 1 filed against an earlier wording of this comment.

**Evidence**:

```
$ cargo test -p ocelli-render -- --ignored --list | grep -c ': test$'
25
```

`cargo` exited 0 (`pipestatus[1]`). The attribute lines are 5 in `crates/ocelli-render/tests/graph.rs` and 2 in `crates/ocelli-render/tests/graph_voi.rs`. Those 7 are the difference from the 18 this comment's history still talks about. Neither file is in the gloss. `git diff --stat 4976c929...HEAD` does not list `bin/ocelli.sh`, so F-038 did not update the comment it made false.

### D3, `CLAUDE.md` still says the cache holds no value type

**Where**: `CLAUDE.md:218-227`

**What**: the "What exists" list says the budgeted LRU "**holds no value type**" and that a decoded frame and a GPU texture "are S12 and S13 stories". The next bullet says nothing renders a frame yet, which is still true of the oracle, and then says "F-038 and F-040 are what make a frame out of these three."

**Why it is wrong**: F-032 added `DecodedFrame` (`impl Budgeted` at `crates/ocelli-cache/src/image.rs:90`). F-036 added `Brick` (`crates/ocelli-cache/src/brick.rs:344`). F-033 added `Volume` (`crates/ocelli-volume/src/lib.rs:554`). "Holds no value type" is the opposite of the tree, not a missing bullet. The frame clause is the forecast this sprint retired. `tools/bench/subjects.json`, edited here, says F-038 presents no frame and names F-101. `docs/lld/render-graph.md` says decision D7 stays open. The list this file keeps is the one its own later paragraph says a sprint review has to correct, because no story in the sprint touched the file.

**Evidence**: `git diff --stat 4976c929...HEAD` does not list `CLAUDE.md`. `rg -n "impl Budgeted for" crates/ocelli-cache/src crates/ocelli-volume/src/lib.rs` prints `DecodedFrame`, `Brick` and `Volume` plus the crate's test doubles.

## Smells

### S1, the LLD index has no render-graph row, and the gpu-ownership row dropped F-038

**Where**: `docs/lld/README.md` against `docs/lld/render-graph.md:3` and `docs/lld/gpu-ownership.md:3`

`docs/lld/README.md:18` says that editing an LLD file means editing its header and its row here in the same hand, and that nothing maintains either line. F-038 added `docs/lld/render-graph.md` and did not add a row. The same story added F-038 to the `gpu-ownership.md` header and left the index row at F-004, F-005, F-008, F-037, F-041, F-X001. `cache.md` and `volumes.md` were updated in both places, so the rule is applied when the closing story remembers it. A reader of the index cannot find the render graph at all. Nothing mechanical reads the lines, which is why this is a smell and not a second copy of D1.

## Nitpicks

- **N1** `docs/lld/benchmarks.md:3-4` still says last updated 2026-09-10 and does not name F-038. F-038 did change the `render.first_frame` row, and the row text is right: the subject stays unavailable and names F-101. The README row matches the file's own F-ID line, so this is a stale header, not an index disagreement.

## Verified clean

Checked on this tree, not taken from the per-story passes.

**The status warning is still there.** `docs/sprints/CURRENT_SPRINT.md:18-19` says the Status column is hand-typed and that `docs/sprints/BACKLOG.md` is the authority. The five cells still say pending. The backlog rows say done. That is the staleness the warning names, so it is not filed.

**D-25 matches the code.** `Volume::present` is `SlicePresence` over `Vec<u64>`. `rg bitvec` over `Cargo.toml` files finds nothing. The sprint diff adds that one row and no other deviation.

**Cache kinds, budgets and brick events.** Kinds are 1 evicted, 2 displaced, 3 refused, 4 pressure and 5 budget changed, in `crates/ocelli-cache/src/image.rs:130-141`. `BrickCache::insert` returns `Admission` and builds no `CacheEvent`, which is what `docs/lld/cache.md` says. `ImageCache` and `VolumeCache` are separate `Lru` values on `CacheTier::Decoded`. `BrickCache` is `CacheTier::Gpu`. The tier tests pin those three. `set_budget` is on `Lru` and the typed caches do not forward it, which is the split the approved F-032 and F-034 plans wrote. `CacheEvent` is public, so a frame admission can still be encoded without `ImageCache`. A brick eviction has no kind, and the brick story says so rather than pretending kind 1 can carry a `BrickKey`.

**F-033 spacing.** The crate does not read `SpacingBetweenSlices` or `SliceThickness`. Those names appear only in comments. Inter-slice spacing is the median of `position_vector().dot(slice_normal())` gaps, refused past 0.5 percent. In-plane spacing is `column_mm` at index 0, along the row cosine, and `row_mm` at index 1, along the column cosine, so `PixelSpacing[0]` scales the column direction. One mutation, `position_vector().dot(normal)` replaced with `position_vector().z`, and `an_oblique_series_sorts_by_projection_and_not_by_ipp_z` failed at `assembly.rs:352` because assembly refused the volume. `bin/ocelli.sh test ocelli-volume --test assembly an_oblique_series_sorts_by_projection_and_not_by_ipp_z -- --exact` exited 101. The line was reverted by editing it back. `git diff` on that file is empty.

**F-038 is not an oracle frame.** `RenderGraph::tick` submits once. `RenderGraph::new` returns `TierCUnavailable` on tier C. The module header, `docs/lld/render-graph.md`, the F-038 AS_BUILT entry and `CHANGELOG.md` all say no surface is acquired and no texture is uploaded. Decision D7 stays open. No sentence in those four places says S12 rendered a frame the oracle can judge. The false frame clause is the `CLAUDE.md` one in D3.

**Tiers.** All five AS_BUILT entries name A, B and C. F-032 and F-034 are n/a on all three. F-033 is n/a, n/a and full, and the assembly path does not branch on tier. F-036 is full on all three for host arithmetic, with the 128 edge inside the tier B 256 floor and no upload. F-038 is full on A, full by construction and never exercised on B, and unavailable on C. None of those paths substitutes a different numeric result for an unavailable one.

**Layout.** A volume slice starts at `n * dims[0] * dims[1] * bytes_per_voxel`. `BrickGrid::byte_offset` is `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`. `Brick::cut` reads through that offset. The two agree with section 19.

**Not claimed done.** This diff does not say the M2 release, deviation D-22, F-X011, ICC or the unscheduled LINEAR range finding is done. `CURRENT_SPRINT.md` still lists them as carried in and not taken.

**Tree**: the oblique mutation was reverted before this file was written. `pub fn set_budget` is still `crates/ocelli-cache/src/lru.rs:246`. `git write-tree` returns `139e3cf756bc319e1c76bd66039689a4d9470486`.
