# F-042 review, pass 3

**Reviewed**: working tree against b93eef02
**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1, The native draw-failure arm asserts a variant it already excluded

**Where**: crates/ocelli-render/tests/tier_b.rs:116
**What**: The `NotSubmitted` and `Readback` arm asserts `matches!(error, Section183Error::Unavailable)`. That pattern cannot succeed. The test still fails closed, because the assertion message fires. Behaviour of this arm did not change. Carried from pass 2.

## Verified clean

The pass 2 discard is gone. `fill_rate_after_wait` returns `None` only when `fence.is_auto_finish()` is true, at `crates/ocelli-render/src/probe.rs:1019`. There is no poll counter. A completion that reaches this function on any other fence value becomes a `FillRate`, including one the first `device.poll(PollType::Poll)` already observed.

`wgpu::GlFenceBehavior` is the pinned type. `Cargo.toml` pins `wgpu = "=30.0.1"` and `Cargo.lock` resolves `wgpu` 30.0.1. `wgpu-30.0.1/src/lib.rs:288` re-exports `GlFenceBehavior` from wgpu-types. The enum is `wgpu-types-30.0.1/src/backend.rs:962`. `Normal` is the `#[default]` variant at line 965. `is_auto_finish` at line 983 is `matches!(self, Self::AutoFinish)` and is false for `Normal`. No local type or trait defines that method. The instance field is the same enum. `GlBackendOptions.fence_behavior` is line 254, and `Default` is `Normal`.

The HAL reads that field. `wgpu-core-30.0.1/src/instance.rs:120` copies `backend_options` into the HAL descriptor. `wgpu-hal-30.0.1/src/gles/web.rs:119` stores `desc.backend_options.gl`. `gles/device.rs:1726` builds `Fence::new(&self.shared.options)`. `gles/fence.rs:51` stores the submission value and returns when `is_auto_finish()` is true. `new_instance_with_webgpu_detection` in `wgpu-30.0.1/src/util/init.rs:116` forwards the descriptor and does not call `with_env`, so `WGPU_GL_FENCE_BEHAVIOR` does not split the instance from the value this code passes.

Call chain, read in `probe.rs`:

- `resolve` at line 156 and `resolve_adapter` at line 206 call `detect`.
- `detect` at line 279 calls `detect_from` with `GlFenceBehavior::Normal`. The instance comes from `InstanceDescriptor::new_without_display_handle`, whose backend options are `Default`, so the instance fence is `Normal` as well.
- `resolve_native_gl` at line 542 does not assign `fence_behavior` and passes `Normal`.
- `resolve_webgl` assigns `AutoFinish` only inside `#[cfg(target_arch = "wasm32")]` at line 493, then copies `descriptor.backend_options.gl.fence_behavior` at line 495 and passes that copy at line 505. That is the only assignment of `AutoFinish` in the repository.
- `measure_candidates` at line 759, `measure` at line 835, and `run` at line 1003 pass that same value through. `run` does not count polls.

The default path cannot reach the discard. `detect` passes `Normal`, and `is_auto_finish` matches only `AutoFinish`. A first-poll completion on that path is still a `FillRate`. `docs/lld/tier-resolution.md:545` says production `detect` is unchanged. Against `HEAD`, `run` still returns a `FillRate` after the wait for this fence. The sentence may stand. The same section at lines 561 to 567 matches the code. A default-fence completion is recorded. An `AutoFinish` value records nothing. `classify` at `caps.rs:803` maps a missing rate to `SoftwareVerdict::Unknown`, and `DecidedBy::Benchmark` is only the Hardware and Software arms at lines 822 and 823.

`an_autofinish_completion_is_not_a_hardware_fill_rate` asserts the two fences separately. `AutoFinish` must be `None` at line 1050. `Normal` must be `Some` of the same pixels and elapsed time at line 1053. That elapsed time is a finished measurement with no poll threshold. Replacing `if fence.is_auto_finish()` with `if false` made `bin/ocelli.sh test ocelli-render --lib an_autofinish_completion_is_not_a_hardware_fill_rate` exit 101. The panic was `probe.rs:1049`, message `an AutoFinish completion was recorded as a hardware fill rate`. The file was restored from the pre-mutation copy. `cmp` was silent and exit 0. Both hashes were `02fa7adb68f276ebaf9e715b64781032b609731cae9f89b897c2b3e694181448`. The same test then exited 0.

`tools/tier-b/run.mjs:185` exits 1 when `decidedBy` is `Benchmark`, after the drew and tier B check. Any other status or tier has already exited 3 at line 183. Neither path exits 0. The wasm probe outputs are newer than `probe.rs`. `node tools/tier-b/run.mjs` was run once, with no other device test in flight. Exit 0. Status `drew`, tier `B`, format `R32Float`, `decidedBy` `AdapterType`, renderer `ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max, Unspecified Version)`. LINEAR was `[0.0, 127.819565, 255.0, 63.909782]` against CPU `[0.0, 127.81955, 255.0, 63.909775]`. LINEAR_EXACT was `[0.0, 127.5, 255.0, 63.75]` on both sides. Input -160 was `0.0`. The centre gap was `0.319565`. Hand rows in the script are `[0, 127.819, 255, 63.91]` and `[0, 127.5, 255, 63.75]`. The largest hand gap is `0.000565`, inside `0.001`. The script printed `tier B section 18.3 matched within 0.001 format R32Float`.

The fence edit does not falsify the pass 2 sentences that were re-read. `create_surface` in this story is still `probe.rs:497`, before `detect_from`. Both `request_device` sites still pass `adapter.limits()`, at lines 411 and 724. `git status` does not show `voi.wgsl` or `ci/wasm-size-budget.json`. `STORED_FORMAT` remains the preferred format in `graph.rs`, with tier B float substitutes unchanged by this edit. Tier C, the `tier-b-probe` feature, and the absence of `unsafe` in the new discard were not moved. `wasm_bindgen` for the probe export remains in `ocelli-wasm`.
