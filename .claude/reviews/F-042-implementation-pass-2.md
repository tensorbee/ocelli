# F-042 review, pass 2

**Reviewed**: working tree against b93eef02
**Result**: 1 defect, 0 smells, 1 nitpick

## Defects

### D1, The first-poll discard applies to every fill-rate run, not only AutoFinish

**Where**: crates/ocelli-render/src/probe.rs:995
**What**: `fill_rate_after_wait` returns `None` when `polls` is below 2. `run` is the only timed submission, and it calls that function for every candidate. `measure` passes `run` into `measure_with`. `measure_candidates` calls `measure`. `detect_from` calls `measure_candidates`. Production `detect` calls `detect_from` with no surface. `resolve` and `resolve_adapter` call `detect`. `resolve_webgl` and `resolve_native_gl` call the same `detect_from`. `GlFenceBehavior::AutoFinish` is set only inside `resolve_webgl`, and only on wasm. A completion that the first `device.poll(Poll)` already observes is dropped on the Metal path as well. `measure_with` then applies `?` to the calibration, so that one `None` drops the whole measurement and the full pass never runs. `classify` sees an unknown benchmark and does not record `DecidedBy::Benchmark`. `docs/lld/tier-resolution.md:545` says production `detect` is unchanged.
**Why it is wrong**: HLD section 7 names the startup measurement as the fact that separates hardware from a software rasteriser. A7 says a measured fill rate is the fact and a renderer string is only a hint. wgpu-types 30.0.1 says `AutoFinish` makes `on_submitted_work_done` return the next time the device is maintained, not when the GPU finishes. That sentence is about the GL fence option. It is not true of the default fence. wgpu-hal Metal `Fence::get_latest` returns the submission value on the poll that sees `MTLCommandBufferStatus::Completed`. wgpu-core `Device::poll` fires the closure in that same poll. Nothing in that path waits for a second poll. Discarding that completion changes the signal `classify` uses for every session that hits it. The LLD sentence is false because `detect` calls the function this story changed.
**Evidence**: Call chain read in `probe.rs`: `run` at line 982, `measure` at line 816, `measure_candidates` at line 741, `detect_from` at line 295, `detect` at line 279, `resolve_webgl` at line 495, `resolve_native_gl` at line 532. AutoFinish is the `#[cfg(target_arch = "wasm32")]` assignment at line 484. The calibration `?` is `measure_with` at line 874. wgpu-hal 30.0.1 `src/metal/mod.rs` lines 1272 to 1283 and `src/gles/fence.rs` lines 51 to 53. wgpu-core 30.0.1 `Device::poll` fires the closures it just collected. `an_autofinish_completion_is_not_a_hardware_fill_rate` passes the integer 1. It does not open a device. Changing `polls < 2` to `polls < 1` made that test panic at `probe.rs:1025` with `an AutoFinish completion was recorded as a hardware fill rate`, exit 101. The constant was restored and the same test passed. That shows the test locks the shared discard. It does not show a real fence needs two polls. `node tools/tier-b/run.mjs` exited 0 with `decidedBy` `AdapterType`, which is the AutoFinish case. The script checks pixels and tier. It does not check `decidedBy`, so a Benchmark regression would still exit 0. Production `detect` was run on its own, before that script, by `bin/ocelli.sh test ocelli-render --lib -- --ignored --nocapture measures_a_fill_rate_on_this_machine`. It recorded tier A, `decided_by` `Benchmark`, `pixels_shaded` 16777216, `elapsed_nanos` 11344250, adapter `Apple M4 Max`. This workload was not a first-poll completion, so this session still has a fill rate. The shared function would drop one that the first poll did observe.

## Smells

None.

## Nitpicks

### N1, The native draw-failure arm asserts a variant it already excluded

**Where**: crates/ocelli-render/tests/tier_b.rs:116
**What**: The `NotSubmitted` and `Readback` arm asserts `matches!(error, Section183Error::Unavailable)`. That pattern cannot succeed. The test still fails closed, because the assertion message fires. Behaviour of this arm did not change.

## Verified clean

`node tools/tier-b/run.mjs` was run once, after the native resolver had exited. Exit 0. JSON was status `drew`, tier `B`, format `R32Float`, decidedBy `AdapterType`, renderer `ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max, Unspecified Version)`. LINEAR was `[0.0, 127.819565, 255.0, 63.909782]` against CPU `[0.0, 127.81955, 255.0, 63.909775]`. LINEAR_EXACT was `[0.0, 127.5, 255.0, 63.75]` on both sides. Input -160 was `0.0`. The centre gap was `0.319565`. The script printed `tier B section 18.3 matched within 0.001`. The hand rows for centre 40 and width 400 are LINEAR about `127.819549` and `63.909774`, and LINEAR_EXACT `127.5` and `63.75`. PS3.3 C.11.2.1.2 and C.11.2.1.3.2. The low sample is `ymin` because the comparison is `<=` and `-160` equals the window edge. That is D-13. The probe wasm is timestamped after `probe.rs`, and this run's `AdapterType` is the new decision, not the pass 1 `Benchmark`.

`docs/sprints/CURRENT_SPRINT.md` lines 49 to 55, `docs/lld/gpu-ownership.md` lines 337 to 344, `docs/lld/render-graph.md` lines 127 to 136, and `docs/lld/tier-resolution.md` lines 566 to 587 name that run: `AdapterType`, `R32Float`, the renderer string, the row values, tolerance 0.001, and the native miss. They do not say this run decided by Benchmark, and they do not call the AutoFinish clock a fill rate. `bin/ocelli.sh test ocelli-render --test tier_b -- --include-ignored --exact native_gl_section_18_3_or_a_recorded_miss --nocapture` printed `native Backends::GL found no adapter. This is not a tier B run.` Prior `docs/sprints/AS_BUILT.md` lines that say tier B had not been run stay historical.

`docs/lld/gpu-ownership.md:246` and line 410 name both `attach_target` and `resolve_webgl`. `rg` shows `create_surface` at `probe.rs:487` and `gpu.rs:308` only. `bash ci/check-device-ownership.sh` printed `OK: only ocelli-render may create a device, and it still owns GpuContext`.

`graph.rs:233` does not say `R32Float` is the only stored format. Tier A still refuses a substitute. Tier B may use `Rg32Float` and then `Rgba32Float` only when `R32Float` lacks `TEXTURE_BINDING` and `COPY_DST`. `Rgba8Unorm` is not a candidate. The constant at line 240 is still `R32Float`. `preserving_format_keeps_r32_and_refuses_a_tier_a_substitute` passed.

`git diff HEAD -- crates/ocelli-render/shaders/voi.wgsl` is empty. Line 57 is `let cp = c - 0.5`. `the_tier_b_module_includes_voi_unchanged` and `the_shader_uses_nothing_tier_b_lacks` passed. Compute and storage tests `a_compute_or_storage_key_is_not_the_fragment_pipeline` and `a_compute_or_storage_key_on_tier_b_is_unavailable` passed. Both `request_device` sites pass `adapter.limits()`, at `probe.rs:403` and line 706. Tier C still returns `TierCUnavailable` from `RenderGraph::new` and `Section183Error::TierC` from `read_section_18_3` before a draw. `tier-b-probe` is not a default feature. Its named user is `tools/tier-b/run.mjs`. `bin/ocelli.sh wasm` does not pass it. `git diff HEAD -- ci/wasm-size-budget.json` is empty. The added Rust has no `unsafe`, no `as` cast, and no new trait. `wasm_bindgen` for the new export is in `ocelli-wasm` behind that feature.
