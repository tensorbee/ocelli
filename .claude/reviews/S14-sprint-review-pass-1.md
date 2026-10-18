# S14 review, pass 1

**Reviewed**: f41f2fb4d9578d8dcc433efe9ad1e2ed8cf6741b..30f7e1654f9c88445bb9ffac3dd28a91849a8afc
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, Tier B's 256 is not the limit brick upload enforces

**Where**: `docs/lld/volumes.md` lines 93 to 94.

**What**: The tiers section says tier B's 3D texture limit of 256 is enforced by the brick upload in `ocelli-render`. `brick_plan` returns `ExtentUnavailable` only when an axis is greater than the `max_texture_dimension_3d` argument it was given. `RenderGraph::upload_brick` passes `ctx.caps().max_tex_3d`. `classify` copies `facts.max_tex_3d` for every tier other than CPU and does not clamp that field to 256. `Backend::Gl` is always a tier B candidate, and a GPU backend without compute shaders is a tier B candidate too, so a tier B adapter can report a 3D limit above 256. The same comparison then accepts an axis of 257. The unit test refuses 257 only because `plan_of` passes 256. `docs/lld/render-graph.md` calls 256 the pinned tier B floor. That is the size a 128 edge is guaranteed to fit. It is not a ceiling the upload applies.

**Why it is wrong**: The sentence says the upload refuses every axis past 256 on tier B. The code refuses an axis past the reported device limit. Clamping the reported limit down to the floor is the lie `classify` already refuses.

### D2, The tracker records two F-X002 review passes

**Where**: `docs/sprints/SPRINT_TRACKER.md`, the F-X002 row.

**What**: The row says the story included two independent review passes converging 1/0, 0/0. This diff contains `.claude/reviews/F-X002-layers-pass-1.md`, `F-X002-layers-pass-2.md`, and `F-X002-layers-pass-3.md`. Those results are 1 defect and 0 smells, then 0 and 0, then 0 and 0. The F-X002 entry in `docs/sprints/AS_BUILT.md` says three review passes converged 1/0, 0/0, 0/0.

**Why it is wrong**: The tracker and the as-built entry cannot both be the count of the review files in this diff. The tracker is the false one.

## Smells

None.

## Nitpicks

None.

## Verified clean

F-043. `brick_format` maps `U8` to `R8Uint`, `U16` to `R16Uint`, `I16` to `R16Sint`, and `F32` to `R32Float`. The plan refuses `R8Unorm`, `R16Unorm`, and an `I16` brick presented as `R32Float`. There is no second format. `upload_brick` asks for features of `brick_format` only and copies `brick.samples` with `write_texture`. The texture extent is `brick.extent`. An edge brick is not padded. `BrickGrid` still chooses the edge, 128 for eight-bit and 64 for sixteen-bit and float, and `ready_bricks` lists a complete 8 by 8 by 8 volume as the single brick `[0, 0, 0]`. No new upload path compares the volume to 256 MiB or to any other size threshold. `set_voi` still writes `VoiParams` and does not call `upload_brick`. `brick_uploads` is not incremented there. `install_recovered` calls `BrickResidency::clear` and does not increment the counter. The residency map has no byte budget. The new module has no frustum test, no LOD selection, no indirection texture, and no prefetch. Tier C is the first branch of `brick_plan` and returns `Unavailable` before a texture exists. A missing `TEXTURE_BINDING` or `COPY_DST` is `FormatUnavailable`. `tick` does not upload bricks. `ocelli-cache` still has no wgpu dependency. The carried LINEAR path was not in this diff. `ocelli-pixel` and `shaders/voi.wgsl` are unchanged, and the brick copy does not clamp samples.

F-X002. `examples/lavapipe_probe.rs` calls `request_adapter` with `force_fallback_adapter` and does not call `request_device`. `fallback_is_lavapipe` matches `lavapipe` or `llvmpipe` only. `gate gpu` is `cargo test -p ocelli-render -- --ignored --test-threads=1`, so it does not run the example. `ci/lavapipe-layer.json` is absent. A missing record makes `lavapipe_layer.main` return 10 and print nothing. A drop with a non-empty reason returns 0 and prints `layer 2 dropped:`. `pass`, `skip`, and a blank reason return 1. The workflow runs the probe only after exit 10, and a probe exit other than 0 stays a failure. `tools/swiftshader/run.mjs` exits 0 only when the renderer string contains `SwiftShader` and the tier is `C`. Any other tier with that string exits 1. Anything else exits 3. The script does not import the comparator and does not open the corpus. Layer 3 is the GPU-less browser session. The lavapipe workflow text says layer 2 is not that session and not the oracle. `.github/workflows/ci.yml` is not in the diff. `docs/hld/DEVIATIONS.md` is not in the diff. `classify` and the adapter-free tier C tests were not rebuilt. The SwiftShader negative fixture is the nitpick this review was told not to reopen. The shipped needle is still `llvmpipe`.

Boundary. The diff adds no `unsafe` and no `wasm_bindgen`. The layer 3 page calls the existing `tierBSection183` export. No tolerance constant moved. No DICOM object was added. The eslint block for `tools/swiftshader` sets four readonly globals and sets no rule. `upload_stored` remains the 2D stored path. It is not a second volume upload.

Not re-run here: `bin/ocelli.sh gate --sprint`, the oracle, Chrome, and the lavapipe probe. The task already recorded the sprint gate as green, including the two unsupported corpus rows, and recorded this machine's probe exit 2.
