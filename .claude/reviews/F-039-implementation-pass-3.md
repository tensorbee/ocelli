# F-039 review, pass 3

**Reviewed**: working tree against 9426845, including untracked `crates/ocelli-render/src/extent.rs`, `crates/ocelli-render/tests/surface.rs`, `crates/ocelli-wasm/src/canvas.rs` and `packages/core/offscreen-canvas.mjs`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, the surface struct is documented as the render graph

**Where**: `crates/ocelli-render/src/graph.rs:434`
**What**: The doc comment that used to sit on `RenderGraph` now sits on `AttachedSurface`. Its first sentence says that struct is the render graph. `pub struct RenderGraph` at line 444 has no doc comment.
**Why it is wrong**: The plan says the render graph gains a slot from viewport id to the current surface. The surface is not the graph. A `///` comment documents the next item, so the sentence is a false claim about `AttachedSurface`, and the public graph type lost the sentence that was true of it.
**Evidence**: `git diff HEAD -U12 -- crates/ocelli-render/src/graph.rs` shows the added lines `/// A surface bound to one viewport` inserted into the existing render-graph comment, then `struct AttachedSurface` placed before `pub struct RenderGraph`. The current file reads:

```text
434:/// The render graph: registered viewports, their dirt, and the pipelines
435:/// compiled at creation.
436:/// A surface bound to one viewport. The colour attachment is acquired from
437:/// it inside `tick` and is not stored on [`GpuSlot`].
438|struct AttachedSurface {
444|pub struct RenderGraph {
```

### D2, FormatDiffers says the format was already configured

**Where**: `crates/ocelli-render/src/gpu.rs:80`
**What**: The variant doc says "The format just configured is not the format the graph was compiled for."
**Why it is wrong**: The plan's resize drops the previous target and reconfigures. This story's own method doc at `crates/ocelli-render/src/graph.rs:845` says the error is for the format the surface would use, and that the previous configuration is left in place. `docs/lld/render-graph.md:83` says the same return happens before `configure`. The enum doc says the configuration already happened.
**Evidence**: The only `FormatDiffers` return is `crates/ocelli-render/src/graph.rs:874`, and `configure_viewport` is the next line:

```text
873|        if format != self.format {
874|            return Err(SurfaceError::FormatDiffers);
875|        }
876|        ctx.configure_viewport(&attached.surface, adapter, extent)?;
```

## Smells

None.

## Nitpicks

None. Pass 1's N1, N2 and N3 were not promoted. `FrameOutcome::Noop` still documents the acquire-failure collapse at `crates/ocelli-render/src/graph.rs:172`. `AttachedSurface` width and height are still only written. `resize_bound` at `crates/ocelli-wasm/src/canvas.rs:108` still uses the tier C text when no context is open. None of those changed behaviour.

## Verified clean

Independent pass. The reviewer did not write the code and did not write pass 1 or pass 2. Runtime behaviour was checked against the plan. The seven documentation edits from pass 2 were read in the current files and checked against the tree.

Pass 1 D1 still holds. `docs/lld/build-targets.md:305` names `install_panic_hook`, `panic_record_ptr` and `panic_record_len` as F-005's and keeps the post-trap rule on those three. Line 309 says the two canvas exports are F-039. Rows 298 to 301 are `ocelli_version` and the three panic exports. Rows 302 and 303 are the two canvas exports. The sentence after the table does not call those canvas rows the last three.

Pass 1 D2 still holds in the files named by pass 1, and in the later edits below. `docs/lld/tier-resolution.md:453` says F-039 makes `ocelli-wasm` depend on `ocelli-render` and on `wgpu`. Lines 454 and 455 say the release module exports six functions, contains Naga, and that the budget records 1,999,061 bytes. Lines 455 and 456 say cold start was not remeasured and that 2.3 ms is the F-006 measurement of the earlier module. `docs/lld/benchmarks.md:162` records 1,999,061 bytes for the module that reaches wgpu and Naga. Lines 163 to 165 say 2.3 ms is the F-006 measurement and is not a measurement of the current module. Line 169 says the current release module exports six functions and contains Naga.

Pass 1 S1 still holds. `crates/ocelli-render/src/extent.rs:41` returns `u32::MAX` when the rounded product is non-finite and positive, before the empty-axis floor at line 47. `device_extent(1e308, 1.0, 10.0)` is `Ok((u32::MAX, 10))`. `device_extent(1e308, 10.0, 1.0)` stays on the finite saturation path. There is no `as` cast. `bin/ocelli.sh test ocelli-render extent::tests -- --test-threads=8` exited 0. 6 passed, including `an_overflowing_product_saturates_to_u32_max`, `extent_at_dpr_one_two_and_one_and_a_quarter` and `a_positive_css_size_whose_product_rounds_to_zero_becomes_one`. The kept rows are 100 by 50 at dpr 2 equals 200 by 100, dpr 1.25 equals 125 by 63, and 0.4 by 10 equals 1 by 10.

The dpr multiply is what the 200 by 100 row locks. It was changed from `f64::round(css * dpr)` to `f64::round(css)` and the same test command for `extent_at_dpr_one_two_and_one_and_a_quarter` exited 101. The failure was `left: Ok((100, 50))` and `right: Ok((200, 100))` at `extent.rs:100`. The file was restored. Its sha256 before the edit and after the restore was `90a69eae65c565652c31001759864def9edbe73a991d37015bfd0a85c6e60f96`, and the round line is `f64::round(css * dpr)` again.

Pass 2 D1 is fixed. The current-state sentences sit above the heading "What this story deliberately does not do". The bullet under that heading says F-004 does not wire tier resolution into `ocelli-wasm`, and that F-039 later added the edge described above. `crates/ocelli-wasm/src/canvas.rs:200` calls `resolve_adapter` on the attach path.

Pass 2 D2 is fixed. `docs/lld/errors.md:303` is titled "Panic-path exports, and three of them are called exactly once". The four-row table is the panic path. Lines 312 to 315 name `attachOffscreenCanvas` and `resizeOffscreenCanvas` as F-039 and say they are not part of that path.

Pass 2 D3 is fixed. `tools/bench/subjects.json` and the header plus the caveat string in `tools/bench/src/runners/wasm_cold_start.mjs` say the release module exports six functions, contains Naga, is recorded at 1,999,061 bytes, that cold start of this module was not remeasured, and that A4 stays open. `git diff HEAD -- ci/bench-baseline.json` is empty. That file still has `value` 2.3, `story` F-006 and artefact `bytes` 16388. The 2.3 ms figure is not presented as a measurement of the 1,999,061 byte module.

Pass 2 D4 is fixed. The `ocelli-core` comment in `crates/ocelli-wasm/Cargo.toml` says that dependency is F-005's panic record and is not the move the budget file now records. The `ocelli-render` comment points at the recorded size cost.

Pass 2 D5 is fixed. The D-14 Why cell says the measured cost today is 1,999,061 bytes because F-039 made `ocelli-wasm` depend on `ocelli-render` and on `wgpu`. It no longer says the cost is zero or that the budget moves only when the render path is wired in later. The webgl feature list and the decision stay. `crates/ocelli-render/Cargo.toml` still has `features = ["webgl"]` and is not in the diff. The pinned wgpu 30.0.1 manifest still has `webgl` at line 117, and its default features are still `std, parking_lot, dx12, metal, gles, vulkan, wgsl, webgpu`. `git diff HEAD -- docs/hld/DEVIATIONS.md` is that one cell. No new deviation row.

Pass 2 D6 is fixed. `docs/sprints/CURRENT_SPRINT.md:52` records 1,999,061 bytes and says `bearing_on_gate_A4` says this is the first time the module reaches wgpu and Naga and that the number does not close A4. That matches the field in `ci/wasm-size-budget.json`. The rest of the page's diff is that bullet. The new sentences match the budget file and the three dependencies in `crates/ocelli-wasm/Cargo.toml`. The attach path calls into the renderer.

Pass 2 D7 is fixed. `CHANGELOG.md` records the first measurement as 14,104 bytes and 16,388 after F-005. It records 1,999,061 for the render edge, from 16,388. It says the 183x and 488x figures described the 16,388 byte module. `3000000 / 16388` is 183.06 and `8000000 / 16388` is 488.16. `3000000 / 1999061` is 1.5007, so "about 1.5 times under the 3 MB low end" matches. A4 stays open because cold start was not remeasured. No cold-start duration was invented for this module.

The on-disk artefact matches the recorded size. `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` is 1,999,061 bytes and contains the bytes `naga-30.0.1`. `1999061 / 1048576` prints as 1.91 with the `.2f` MiB format in `scripts/pin_and_size_check.py`. The budget `bytes` is 1999061, `from` 16388, `to` 1999061, `delta` 1982673 and `delta_fraction` 120.9832. `1982673 / 16388` rounds to 120.9832 at four places. Tolerance is still 0.05. The cause says the parent commit was not rebuilt, and that sentence is about the 16,388 baseline. `extent.rs` is newer than the wasm file. No sentence claims the recorded measurement includes the overflow fix. This pass did not rebuild wasm and did not run the ignored GPU test.

`crates/ocelli-wasm/pkg/ocelli_wasm.js` exports `attachOffscreenCanvas`, `install_panic_hook`, `ocelli_version`, `panic_record_len`, `panic_record_ptr` and `resizeOffscreenCanvas`. Those are the six names in the Rust source. `panic_probe_trigger` stays behind the `panic-probe` feature.

`ResolvedAdapter::open` still sets `required_limits: self.adapter.limits()` at `crates/ocelli-render/src/probe.rs:396`. A tier C request returns in `detect` before `new_instance_with_webgpu_detection`, with `instance: None`. `attach_offscreen_canvas` calls `sized` before `ensure_device`. `sized` is `device_extent` plus `set_width` and `set_height`. On no adapter, `ensure_device` returns the tier C error and `attach_target` is not called. `RenderGraph::new` still returns `TierCUnavailable` when the tier is CPU.

`encode_frame` calls `queue.submit` once, then `queue.present` on each acquired texture. A viewport with no surface uses the caller-supplied view. A surface that does not yield a texture is skipped and is not drawn into that view. `reconfigure_surface` does not call `rebind` or `rebind_stored` and does not increment `uploads`. `install_recovered` drops frame textures before surfaces. `shaders/voi.wgsl` is not in the diff.

`create_surface` is called only at `crates/ocelli-render/src/gpu.rs:304`. A search of `crates/ocelli-wasm` found no `create_surface` substring. A search of `crates/ocelli-render` found no `web_sys` identifier. The new extent and canvas sources have no `unsafe` block. `wasm_bindgen` in crate sources is only under `crates/ocelli-wasm`. `OffscreenCanvas` is named in `crates/ocelli-wasm/src/canvas.rs`. Direct `web-sys` is only in that crate's wasm32 table, at `=0.3.104`, which is the version wgpu 30.0.1 resolves. `js-sys` is `=0.3.104` and `wasm-bindgen-futures` is `=0.4.77` beside `wasm-bindgen` 0.2.127 in `Cargo.lock`. The attach export is async. The direct `wgpu` dependency is the `SurfaceTarget::OffscreenCanvas` constructor. The device is still opened by `ResolvedAdapter::open`.

`packages/core/offscreen-canvas.mjs` asserts bitmap size 200 by 100 at dpr 2 and 125 by 63 at dpr 1.25, and rejects the CSS size 100 by 50. It does not treat an error string as failure. The eslint change names globals for that file and does not edit the wasm-memory allowance list. The file does not build a view over `WebAssembly.Memory`. This pass did not launch the browser.

The `ocelli_version` doc comment at `crates/ocelli-wasm/src/lib.rs:43` still says that function is the whole exported surface until F-101. The diff does not touch that sentence. It was already false after F-005. This pass does not charge it to F-039. The same choice applies to the pre-existing compute-error sentence in `docs/lld/gpu-ownership.md` that says nothing crosses the boundary until F-101. This diff does not refresh it.

`docs/sprints/AS_BUILT.md`, `ci/bench-baseline.json`, `docs/spikes/A2-jpeg-ls.md` and the `from` baseline wording in `ci/wasm-size-budget.json` were not charged. They are history or they name the previous baseline rather than the current module.
