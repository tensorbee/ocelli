# F-042 review, pass 1

**Reviewed**: working tree against b93eef02
**Result**: 4 defects, 0 smells, 1 nitpick

## Defects

### D1, Tier B is recorded as a fill-rate decision from a clock that does not wait for the GPU

**Where**: crates/ocelli-render/src/probe.rs:484
**What**: `resolve_webgl` sets `GlFenceBehavior::AutoFinish` on the same instance that runs the startup fill-rate probe. On that fence, `signal` stores the completed value at submit. `probe::run` treats `on_submitted_work_done` as completion, so the elapsed time is the wait until the next poll. The full pass shades 16,777,216 fragments. With the recorded floor of 400,000,000 pixels per second and no software ceiling, any non-zero elapsed at or under 41.943 ms is Hardware. `classify` then keeps the GL candidate as tier B with `DecidedBy::Benchmark` and does not consult adapter type or the renderer string. A software WebGL adapter whose submit returns inside that window is `Opened` at tier B. `WebGlProbe::Demoted` is not reached. The comment on that variant says a software rasteriser that presents WebGL2 is the demoted case.
**Why it is wrong**: HLD section 7 names tier B as WebGL2, and the resolver's step 5 is the A7 rule that a measured fill rate is the fact and a renderer string is only a hint. `FillRate.elapsed_nanos` is documented as wall-clock time from submission to completion. wgpu-types 30.0.1 says AutoFinish makes that callback return the next time the device is maintained, not when the GPU finishes. `wgpu-hal` `Fence::signal` stores the value and returns. This is not an operator override. `detect_from` is called with `TierRequest::Auto`. It is not a WebGPU device reported as tier B. The instance backends are `Backends::GL`, and `new_instance_with_webgpu_detection` only removes `BROWSER_WEBGPU` when that bit is already set. The letter B on this machine matches section 7, because the renderer is hardware ANGLE Metal. The step that produced the letter is not a fill rate.
**Evidence**: `node tools/tier-b/run.mjs` exited 0. The JSON was status `drew`, tier `B`, format `R32Float`, decidedBy `Benchmark`, renderer `ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max, Unspecified Version)`. LINEAR was `[0.0, 127.819565, 255.0, 63.909782]` against CPU `[0.0, 127.81955, 255.0, 63.909775]`. LINEAR_EXACT was `[0.0, 127.5, 255.0, 63.75]` on both sides. The immediate store is `wgpu-hal-30.0.1` `src/gles/fence.rs` lines 51 to 53. The Hardware arm is `caps.rs` line 822. The comment at `probe.rs` lines 479 to 481 says the tier falls through to adapter type when elapsed time is zero. This run did not fall through.

### D2, Living docs still say tier B has never been exercised

**Where**: docs/sprints/CURRENT_SPRINT.md:49
**What**: `CURRENT_SPRINT.md` says tier B has still never been exercised, by anything, in any sprint. `docs/lld/gpu-ownership.md:337` says the same thing in the present tense, and that F-042 is the story that still has to do it. The working tree has a WebGL2 draw on tier B.
**Why it is wrong**: Both sentences are current-state claims. A run this tree performed makes them false. Prior `AS_BUILT` entries that said tier B had not been run at F-037 are history and are not part of this finding.
**Evidence**: The probe in D1 returned tier B and drew the section 18.3 rows. `cargo test -p ocelli-render --test tier_b -- --include-ignored --exact native_gl_section_18_3_or_a_recorded_miss --nocapture` printed `native Backends::GL found no adapter. This is not a tier B run.` That miss is a separate fact. It does not keep the "never exercised" sentences true.

### D3, gpu-ownership still says attach_target is the only create_surface

**Where**: docs/lld/gpu-ownership.md:246
**What**: The device-lifecycle table says `GpuContext::attach_target` is the only `create_surface`. Line 405 repeats it. `resolve_webgl` calls `instance.create_surface` at `probe.rs:487` before `request_adapter`, because WebGL2 needs a surface and no device exists yet.
**Why it is wrong**: The sentences describe the crate as it is now. After this story they are false. A green ownership script does not make them true.
**Evidence**: `rg -n create_surface crates --glob '*.rs'` shows the call at `probe.rs:487`, the call at `gpu.rs:308`, and a comment at `gpu.rs:62`. `bash ci/check-device-ownership.sh` printed `OK: only ocelli-render may create a device, and it still owns GpuContext`. The script refuses the call outside `ocelli-render`. Both calls are inside that crate, so the script stays green.

### D4, STORED_FORMAT is documented as the only stored format

**Where**: crates/ocelli-render/src/graph.rs:233
**What**: The doc comment says stored samples are uploaded only as `R32Float`, and that a device which cannot sample it reports `FormatUnavailable`. `register_stored` calls `selected_stored_format`. On tier B that accepts `Rg32Float` and then `Rgba32Float` when `R32Float` lacks `TEXTURE_BINDING` and `COPY_DST`. Tier A still refuses those substitutes. `Rgba8Unorm` is not a candidate.
**Why it is wrong**: The comment describes the upload rule this story replaced. The module header already says tier B may store the wider float formats. The constant comment was left on the old rule.
**Evidence**: `Upload::preserving_format` returns `Rg32Float` for tier B when the first feature set is not sampleable, and `Err(FormatUnavailable)` for tier A on the same features. `cargo test -p ocelli-render --lib -- preserving_format_keeps_r32_and_refuses_a_tier_a_substitute` passed.

## Smells

None.

## Nitpicks

### N1, The native draw-failure arm asserts a variant it already excluded

**Where**: crates/ocelli-render/tests/tier_b.rs:116
**What**: The `NotSubmitted` and `Readback` arm asserts `matches!(error, Section183Error::Unavailable)`. That pattern cannot succeed. The test still fails closed, because the assertion message fires, so a real draw error is not ignored. A `panic` with the same text would say the same thing.

## Verified clean

The WebGL2 script was executed once. `node tools/tier-b/run.mjs` exited 0 with the JSON in D1. Differences from `LutChain::map_into` were about 1.5e-5 and 7e-6, inside 0.001. The hand rows for centre 40 and width 400 are LINEAR `127.819548...` and `63.909774...`, and LINEAR_EXACT `127.5` and `63.75`. Input -160 is `ymin` for both, because `-160` equals `c - w/2` and the comparison is `<=`. That is D-13, and the readback was `0.0`, not `1.594`. The centre gap was `0.319565`. The script also checks the CPU row, so the expectation is not a previous GPU readback pasted in as the only oracle. Playwright `1.62.1` is the oracle package. The Chromium args in the script are `--use-gl=angle --use-angle=metal --ignore-gpu-blocklist --enable-webgl --enable-webgl2`. They are not the oracle SwiftShader flags.

That binary is older than `tier_b.rs` (`04:26:29` against `04:29:23`). `strings` finds `map callback did not run` and `short map` in it, and does not find `short display row`, `readback offset` or `short texel`. Those three are in the current failure paths. `voi.wgsl` and `graph.rs` are both older than the binary, so the window functions in the run are the current shader text. The drew numbers are not a fresh build of the current readback. This pass did not rebuild the probe.

`git diff HEAD -- crates/ocelli-render/shaders/voi.wgsl` is empty. Line 57 is `let cp = c - 0.5`. That is the LINEAR formula, so this pass did not re-mutate it. The tier B module is `VOI_WGSL` followed by the existing fragment entry. `the_tier_b_module_includes_voi_unchanged` passed and counts each window function once. There is no second Rust window function on the tier B path. The CPU row is `LutChain::map_into`. The fragment shader normalises by `ymax - ymin` for an `Rgba32Float` target and the readback multiplies by 255. For this fixture that range is 0 to 255, so the round trip is not a second slope. `select` substitutes 0 only when the range is not positive. It does not clamp LINEAR.

This device reported `R32Float`. The wider-format pack was not executed. The unit test covers the selection rule, including the tier A refusal, and it passed. `Rgba8Unorm` is not in the candidate list.

Compute and storage lookups return `Unavailable` and do not return the fragment pipeline. `a_compute_or_storage_key_on_tier_b_is_unavailable` and `a_compute_or_storage_key_is_not_the_fragment_pipeline` passed. `pipeline_compiled` is still only stack composite, so Mip, MinIp and Average stay unavailable. `tick` asks for `PipelineFamily::Fragment` and submits once. The section 18.3 readback submits again after the frame to copy the target. That second submit is the test readback, not a second submit inside `tick`. The wider-format upload allocates a padded `Vec` at upload time. The `R32Float` path used here copies the caller slice. Tier C still returns `TierCUnavailable` from `RenderGraph::new` before a shader is built, and `read_section_18_3` returns `TierC` before a draw. This pass did not open a CPU session.

Both `request_device` sites pass `adapter.limits()`. A search of the new Rust found no `Limits::default()` used as a value, no new `unsafe`, no `as` cast, no new trait and no `Box<dyn>`. `Box<WebGlOpened>` is a concrete box. `wasm_bindgen` for the new export is in `ocelli-wasm` and is behind `tier-b-probe`. That feature has no `default` entry. Its named user is `tools/tier-b/run.mjs`. `bin/ocelli.sh wasm` does not pass the feature. `strings` on `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` does not find `tierBSection183`.

`wc -c` of that pkg wasm is 1,999,272. The file is timestamped `04:30:09`, which is after every `ocelli-render` and `ocelli-wasm` source in this tree, so it is a current artefact. `git diff HEAD -- ci/wasm-size-budget.json` is empty. The recorded baseline is 1,999,061 and the 5 percent ceiling is 2,099,014. The probe wasm is a different file, 2,049,485 bytes, and it is the one that carries the feature.

The native GL test was run after the browser had exited. It found no adapter and did not draw. The LLD does not treat that miss as the WebGL2 proof. `docs/lld/tier-resolution.md` and `docs/lld/render-graph.md` match the script output, the args, the native miss line, the `R32Float` format and the `adapter.limits()` call. Those two files are not charged.
