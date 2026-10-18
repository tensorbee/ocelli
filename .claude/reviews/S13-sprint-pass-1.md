# S13 whole-sprint review, pass 1

**Reviewed**: git diff 439c44287cfe13b2a0d67acb69aa159afbe48cc1..HEAD
**Result**: 5 defects, 0 smells, 0 nitpicks

## Defects

### D1, four edited LLD headers do not match the index

**Where**: `docs/lld/cache.md:3` against `docs/lld/README.md:40`. `docs/lld/gpu-ownership.md:3` against `docs/lld/README.md:30`. `docs/lld/tier-resolution.md:3` against `docs/lld/README.md:32`. `docs/lld/typescript-packaging.md:3` against `docs/lld/README.md:35`.
**What**: Each of those files lists an S13 F-ID in `F-IDs that contributed` that its index row omits. Cache and TypeScript packaging add F-035. GPU ownership and tier resolution add F-042.
**Why it is wrong**: `docs/lld/README.md` lines 10 to 18 say an edit to an LLD file updates that header and the index row in the same hand. This sprint edited all four files and the index. A mismatch is a false list of who contributed.
**Evidence**: The cache header is `F-031, F-032, F-033, F-034, F-035, F-036` and the index row stops at F-036 without F-035. The GPU ownership header includes F-042 and the index row stops at F-041 before F-X001. The tier resolution header is `F-004, F-037, F-039, F-042, F-X001, F-X016` and the index row has no F-042. The TypeScript packaging header ends in F-035 and the index row ends at F-X017. The same diff does keep pixel pipeline, render graph, build targets, benchmarks and errors aligned with their index rows.

### D2, the wasm export surface is still described as the pre-canvas module

**Where**: `docs/lld/build-targets.md:88` and `crates/ocelli-wasm/src/lib.rs:42`.
**What**: Build targets says the module exports four functions, and that the three beside `ocelli_version` are the panic record. The version function's own comment says it is the module's entire exported surface until F-101.
**Why it is wrong**: Both sentences are present tense in files this sprint edited. F-039 added `attachOffscreenCanvas` and `resizeOffscreenCanvas` on the product module. The same build-targets file, at lines 292 to 303, lists those two plus the four older exports and calls the count six. The earlier paragraph was left behind.
**Evidence**: `crates/ocelli-wasm/src/canvas.rs` declares `#[wasm_bindgen(js_name = attachOffscreenCanvas)]` at line 66 and `resizeOffscreenCanvas` at line 89. `tierBSection183` at line 371 is behind `feature = "tier-b-probe"` and is not one of the six. `packages/core/src/index.ts` still has `coreAvailable` returning false, so that half of the version comment is unchanged.

### D3, D-14 calls the budget number today's measured module size

**Where**: `docs/hld/DEVIATIONS.md:31`.
**What**: The Why cell says the measured cost today is 1,999,061 bytes, because F-039 made `ocelli-wasm` depend on `ocelli-render` and on `wgpu`.
**Why it is wrong**: 1,999,061 is the recorded budget. It is not the size of the module this sprint last measured. The F-044 wasm gate measured the product module at 2,010,440 bytes and left the budget file alone. That later size is inside the 5 percent ceiling of 2,099,014. Calling the budget figure today's measurement overwrites that later result.
**Evidence**: `ci/wasm-size-budget.json` still has `bytes` 1999061 and tolerance 0.05. `.claude/scratch/F-044-progress.md` records `bin/ocelli.sh gate wasm` exiting 0, product module 2,010,440, baseline 1,999,061, ceiling 2,099,014, and says the budget file was not edited. This pass did not rebuild wasm. `CHANGELOG.md` records 1,999,061 as the budget figure for the render edge. That changelog sentence is the budget, and this finding does not contradict it.

### D4, the F-040 tier row still says tier B has no wider format

**Where**: `docs/sprints/AS_BUILT.md:3467`.
**What**: Tier coverage says tier B is full when the device can sample `R32Float` and otherwise unavailable, and that this story does not substitute a wider format.
**Why it is wrong**: That is a present description of the upload path. Today's `register_stored` asks for `R32Float`, then `Rg32Float`, then `Rgba32Float`. Tier B returns the first of those that has `TEXTURE_BINDING` and `COPY_DST`. Tier A still returns only `R32Float`. The same entry's deviations paragraph does point at the F-042 entry for the later rule. The tier row itself still says the substitute does not exist.
**Evidence**: `Upload::preserving_format` at `crates/ocelli-render/src/graph.rs:282` returns a non-`R32Float` candidate only when `tier == Tier::B`. `register_stored` at line 922 calls `selected_stored_format`, which calls that function. The unit test `preserving_format_keeps_r32_and_refuses_a_tier_a_substitute` expects `Rg32Float` on tier B when `R32Float` is missing, and `FormatUnavailable` for the same features on tier A. The preferred constant `STORED_FORMAT` remains `R32Float`.

### D5, CURRENT_SPRINT still describes the graph and the handoff from before this work

**Where**: `docs/sprints/CURRENT_SPRINT.md:27` and `docs/sprints/CURRENT_SPRINT.md:89`.
**What**: One sentence says the graph draws into a caller-supplied texture view and acquires no surface. Another says the handoff is a pointer and a length.
**Why it is wrong**: Both are present tense, and this sprint made both false. The status column still says pending. That column is hand-typed and is not charged. These two sentences are not that column. F-039 binds a surface and `tick` acquires its current texture. F-035 copies a decoded range into a fresh `ArrayBuffer` because two wasm instances cannot share a pointer. The pixels are not a retained view over wasm memory. They are also not a pointer and a length.
**Evidence**: `RenderGraph::bind_surface` records the surface at `crates/ocelli-render/src/graph.rs:1068`. `acquire_dirty_surfaces` calls `get_current_texture` at line 1679, and `encode_frame` presents after the single submit at line 1784. `exportDecodedFrame` in `packages/core/src/bulk.ts` allocates an `ArrayBuffer`, copies into it, and returns that buffer. `.claude/plans/F-035-design.md` says decision D5 forbids `SharedArrayBuffer`, so a decoded frame cannot move by pointer. The backlog status for F-035, F-039, F-040, F-042 and F-044 is `done`. That part of the sprint file's own authority note is still true.

## Smells

None.

## Nitpicks

None.

## Verified clean

`python3 scripts/prose_check.py` exited 0 over 382 tracked files. This review was not in that set. The sprint diff adds no `unsafe` and no numeric `as` cast. New conversions go through `try_from` or the bit walk in `saturating_u32`. `wasm_bindgen` added by the diff is only in `crates/ocelli-wasm/src/canvas.rs`. `request_device` remains two calls, both in `crates/ocelli-render/src/probe.rs`. `tick` submits once, at `graph.rs:1780`. `present` is after that submit and is not a second submit. `RenderGraph::new` still returns `TierCUnavailable` on tier C. `wgpu` in the workspace manifest is `=30.0.1`. No new trait, generic parameter or `Box<dyn>` was added. `shaders/voi.wgsl` is not in the sprint diff. `voi_linear` still uses `c - 0.5` and `w - 1.0`, with `<=` on the low side and `>` on the high side. `voi_linear_exact` uses neither adjustment. Colour and upload tolerances stay `0.001`. The CPU colour test still requires the unbound LINEAR finding to stay above 255 and to keep bits `FINDING_BITS` after indexing. Mode 2 returns before `lut_chain`. Mode 1 indexes the display value and does not also normalise. ICC, a palette on monochrome, and a preset on palette colour return `Unavailable`.

The stops mutation was applied and then restored. In `preset_bytes`, index 128 was changed from `[255, 0, 0]` to `[0, 255, 0]`. `bin/ocelli.sh test ocelli-render --test colour -- --test-threads=1 grey_at_display_40_is_rgb_40_and_stops_are_the_three_texels` exited 101. The assertion at `crates/ocelli-render/tests/colour.rs:179` showed left `[0, 255, 0, 255]` and right `[255, 0, 0, 255]`. `git checkout -- crates/ocelli-render/src/colour.rs` restored the file. `git diff -- crates/ocelli-render/src/colour.rs` was empty. The same test then exited 0. The constant at `colour.rs:232` is `[255, 0, 0]` again.

The carried story notes were re-read and not raised. The wide-`usize` arm of `check_buffer` returns a length error when `bytes` does not fit `usize`. It does not accept a short buffer. `FrameOutcome::Noop` after a failed acquire drops held textures and does not present. `AttachedSurface` width and height are stored and unread. `resize_bound` uses the tier C string when no context is open. The native GL match arm in `crates/ocelli-render/tests/tier_b.rs` cannot succeed and fails by its assertion message. None of those four changes a pixel result on the path that does run.

No tier B colour draw was required or run. `docs/lld/render-graph.md` and the F-044 as-built entry both say that readback was not run. The render-graph section 18.3 paragraph is the greyscale WebGL2 run, with the recorded LINEAR and LINEAR_EXACT tuples, and it does not call that run a colour LUT comparison. The ignored colour tests were not repeated. Story review pass 2 already ran them on tier A. `ci/bench-baseline.json` was not charged. Cold start stays the F-006 figure of 2.3 ms in `docs/lld/benchmarks.md`, which says that figure is not a measurement of the current module. `ocelli-render` does not name `web_sys`. `ocelli-wasm` does not call `create_surface`.
