# F-039 review, pass 1

**Reviewed**: working tree on `sprint/s13`, uncommitted on `9426845`. `git diff` of the 13 tracked files plus untracked `crates/ocelli-render/src/extent.rs`, `crates/ocelli-render/tests/surface.rs`, `crates/ocelli-wasm/src/canvas.rs` and `packages/core/offscreen-canvas.mjs`. Nothing staged. `shaders/voi.wgsl` is not in the diff.
**Result**: 2 defects, 1 smell, 3 nitpicks

## Defects

### D1, the export table's following sentence still describes the old last three

**Where**: `docs/lld/build-targets.md:305`
**What**: After the table grew to six exports, the next sentence still says the last three are F-005's and that `docs/lld/errors.md` is where they are specified.
**Why it is wrong**: A claim in the LLD is false. The table at lines 298-303 ends with `panic_record_len`, `attachOffscreenCanvas` and `resizeOffscreenCanvas`. Only the first of those three is F-005. The two canvas exports are this story, and `docs/lld/errors.md` does not specify them. The same sentence then says none of "them" is called after a trap, so the panic-record rule is now attached to the wrong rows.
**Evidence**: The built glue at `crates/ocelli-wasm/pkg/ocelli_wasm.js` exports exactly those six names (`ocelli_version`, `install_panic_hook`, `panic_record_ptr`, `panic_record_len`, `attachOffscreenCanvas`, `resizeOffscreenCanvas`) plus wasm-bindgen's init. The sentence was true when the table had four rows and the last three were the panic exports. This diff added the two canvas rows and left the sentence.

### D2, other LLD still says the wasm module does not reach the renderer

**Where**: `docs/lld/tier-resolution.md:452` and `docs/lld/tier-resolution.md:469`. Also `docs/lld/benchmarks.md:169`.
**What**: `tier-resolution.md` says the measured cost today is zero bytes because `ocelli-wasm` does not depend on `ocelli-render`, that the edge is still not there, and that `ci/wasm-size-budget.json` still records 16,388 bytes. `benchmarks.md` says the module holds four exported functions, no wgpu and no Naga, and that the old size caveat applies unchanged.
**Why it is wrong**: Those are current-state claims, and this working tree makes each one false. `crates/ocelli-wasm/Cargo.toml` depends on `ocelli-render` and on `wgpu`. The budget file's `bytes` is 1999061. The release glue exports six functions and contains Naga. `docs/lld/build-targets.md` was updated to say the 2.3 ms cold start was not remeasured. `benchmarks.md` still describes that figure as a module with no wgpu.
**Evidence**: `python3` read of `ci/wasm-size-budget.json` printed `bytes 1999061`. `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` is 1999061 bytes on disk. The dependency lines are `ocelli-render` and `wgpu` under `[dependencies]` in `crates/ocelli-wasm/Cargo.toml`.

## Smells

### S1, an overflowing finite product becomes 1 instead of saturating

**Where**: `crates/ocelli-render/src/extent.rs:40` and `crates/ocelli-render/src/extent.rs:55`
**What**: `saturating_u32` returns 0 for a non-finite value. `axis_extent` then treats that 0 as an empty positive axis and stores 1. A finite CSS size times a finite ratio can overflow to `+infinity` before the round. That axis becomes 1.
**Why it is wrong**: The public contract is a saturating conversion, and the private comment says a value at or above `2^32` becomes `u32::MAX`. `+infinity` is above that range. The floor is for a product that rounded to 0, not for a product that overflowed. The named rows (100 by 50 at dpr 2, the 0.4 floor, `u32::MAX`) are unaffected. The only backing-store formula still answers 1 for an input pair it accepts.
**Evidence**: A throwaway `rustc` of the same two conditions printed `product_finite=false rounded_finite=false out=1` for `1e308_f64 * 10.0`. The finite path was checked separately. A Python port of the bit extraction matched every integer from 0 through 10000 and the powers around `2^31` and `2^32`, with 0 mismatches. `f64::round` of a finite in-range value is not the bug. The bug is the non-finite arm feeding the empty-axis floor.

## Nitpicks

### N1, `Noop` also means every surface acquire failed

**Where**: `crates/ocelli-render/src/graph.rs:172` and `crates/ocelli-render/src/graph.rs:1386`
**What**: If every dirty viewport has a surface and `get_current_texture` yields no drawable texture, `tick` returns `FrameOutcome::Noop`, the same value as a frame with nothing dirty. Dirt stays. A lost or outdated surface retries forever and the caller cannot tell that from idle.
**Why it is a nitpick**: The variant comment documents the collapse, and the plan does not require a separate outcome. A shell that stops on `Noop` would stick. The frame loop is not in this story.

### N2, the stored surface extent is never read

**Where**: `crates/ocelli-render/src/graph.rs:438`
**What**: `AttachedSurface.width` and `height` are written in `bind_surface` and `reconfigure_surface` and never read. The draw uses the current surface texture, whose size comes from `configure`. The fields cannot disagree with a frame because nothing consults them.
**Why it is a nitpick**: Behaviour matches the plan. `configure` is what sizes the swapchain. The fields are dead record-keeping. `cargo clippy -p ocelli-render --all-targets -- -D warnings` does not flag them.

### N3, a resize with no context always says tier C

**Where**: `crates/ocelli-wasm/src/canvas.rs:108`
**What**: `resize_bound` returns "tier C opens no device, so no surface was attached" when `context` is missing, including a resize that arrives before `attach` has finished opening a tier A device.
**Why it is a nitpick**: The successful path does not do this. After a context exists, resize reconfigures. The false text is only the not-yet-open path, and it does not draw.

## Verified clean

Independent pass. The reviewer did not write the code. The per-axis floor matches the plan and is not a wrong backing store. The plan's formula is `round(css * devicePixelRatio)` per axis, then a saturating conversion, and "becomes 1" is that axis. The purpose is that a visible element does not get an empty backing store. CSS 0.4 by 10 at dpr 1 is visible on the long axis. `(1, 10)` keeps the axis that rounded to 10. Folding both axes to `(1, 1)` would discard a correct 10. `(0.4, 0.4)` is `(1, 1)`. Non-finite inputs return `ExtentError::NonFinite` before the ratio check. `dpr <= 0.0` covers `-0.0` and returns `NonPositiveRatio`. CSS 100 by 50 at dpr 2 is `(200, 100)`. CSS 100 by 50 at dpr 1.25 is `(125, 63)` because `f64::round` is half away from zero. The multiply is restored. `crates/ocelli-render/src/extent.rs:36` is `f64::round(css * dpr)`. There is no `css * 1.0`.

`bin/ocelli.sh test ocelli-render extent::tests -- --test-threads=8` exited 0. 5 passed. The finite saturating path matches integers, including `u32::MAX` and `2^32`, which becomes `u32::MAX`. The overflow hole is S1, not these rows.

One instance. `detect` builds one instance with `new_instance_with_webgpu_detection` and stores it on `ResolvedAdapter`. Tier C requested as `Tier::Cpu` returns before that call, with `instance: None`. `open` passes `self.instance.clone()` into `GpuContext::new`. `wgpu::Instance` is a cloned handle, and `opened_from_this_instance` compares the two. `attach_target` calls `self.instance.create_surface` and does not construct another instance. `ResolvedAdapter::open` still sets `required_limits: self.adapter.limits()` at `crates/ocelli-render/src/probe.rs:396`. It does not use `Limits::default()`.

`create_surface` appears in `crates/ocelli-render/src/gpu.rs` only. A search of `crates/ocelli-wasm` found no `create_surface` substring. A search of `crates/ocelli-render` found no `web_sys` identifier. `ci/check-device-ownership.sh` exited 0. `create_surface_unsafe` is not called. No new `unsafe`. `wasm_bindgen` in `crates/**/*.rs` is only under `crates/ocelli-wasm`. The new extent and canvas sources have no `as` cast and no `unwrap` or `expect`. `bin/ocelli.sh clippy ocelli-render` exited 0.

`ocelli-wasm` depending on `wgpu` directly does not break D-12, bindgen isolation, or device ownership. D-12's purpose is that `ocelli-render` carries no browser binding. `OffscreenCanvas` is named only in `crates/ocelli-wasm/src/canvas.rs`. `web-sys` is a direct dependency only in that crate's wasm32 table. `SurfaceTarget::OffscreenCanvas` is a wgpu type, so the wasm crate has to name `wgpu` or the type cannot be built outside `ocelli-render`. The device is still opened by `ResolvedAdapter::open` inside `ocelli-render`. The wasm source does not contain `request_device` or `create_surface`. The bindgen guard skips `ocelli-wasm` and fails any other crate that declares `wasm-bindgen` or names `wasm_bindgen`. This change does neither.

Tick submits once. `encode_frame` calls `queue.submit` once, then `queue.present` on each acquired `SurfaceTexture` after that submit, in slot order. A viewport with no surface uses `slot.target`. A viewport whose surface did not yield a texture is skipped and is not drawn into the placeholder. `Queue::present` in wgpu 30.0.1 is the present after submit. Its internal barrier submit is inside wgpu's present, which the plan requires to run after the one application submit.

F-040 behaviour held on the texture path. `reconfigure_surface` does not call `rebind` or `rebind_stored` and does not increment `uploads`. `rebind` still returns `StoredNeedsRebind` when `state.stored` is set. `shaders/voi.wgsl` is not in the diff. The size-budget `tolerance` is still `0.05`. No pixel tolerance moved.

`bin/ocelli.sh test ocelli-render --test surface -- --ignored --test-threads=1` exited 0, 1 passed, 0.52s. The same command with `--nocapture` exited 0 in 0.35s and did not print "this machine resolved tier C", so the test did not take the early return. The device opened and the readback assertions ran. That test uses caller-supplied textures and `rebind`. It does not call `attach_target`, `get_current_texture` or `present`. The plan says native tests have no canvas and that the replace-the-target rule is proved with an offscreen texture. The integration row is the bitmap size. Neither test executes `attach_target`. That is the split the plan wrote down. The surface acquire path is still unexecuted.

Tier C does not attach. `attach_offscreen_canvas` calls `sized` before `ensure_device`. `sized` is `device_extent` plus `set_width` and `set_height`. On tier C, `ensure_device` sets `unavailable` and returns the tier C error before `attach_bound`, so `attach_target` is not called. `RenderGraph::new` still returns `GraphError::TierCUnavailable` when `caps.tier` is `Tier::Cpu`. Extent math does not need a device. The wasm path does not draw a substitute image.

`node packages/core/offscreen-canvas.mjs` exited 0 and printed:

```text
{"attached":{"width":200,"height":100,"dpr":2,"error":"tier C opens no device, so no surface was attached"},"resized":{"width":125,"height":63,"dpr":1.25,"error":"tier C opens no device, so no surface was attached"}}
```

That satisfies the plan's integration row. The row says a worker `OffscreenCanvas` width and height equal `device_extent`, in the oracle Chromium. The worker does not assign `canvas.width` itself. The wasm export does, from `device_extent`, before the tier C return. The assertions lock 200 by 100 and then 125 by 63, and they reject the CSS size 100 by 50. It does not prove a surface was created. The error string is the evidence that SwiftShader opened no device and `attach_target` was not called. The plan put surface creation on the native ignored test's texture stand-in, not on this page.

`eslint.config.js` adds a `languageOptions.globals` block for `packages/core/offscreen-canvas.mjs` because the file is node and the `page.evaluate` callback is browser. It does not set `ocelli/no-wasm-memory-view` or `no-restricted-syntax` to off, and it does not change `ALLOWED_TO_DISABLE`. That list is still `packages/core/src/bulk.ts`, `packages/core/src/panic.ts` and `packages/core/src/*.test.ts`. The typed ban is on `**/*.{ts,tsx}` only, same as the other mjs scripts. The new file does not construct a view over `WebAssembly.Memory`. A new file is not granted that allowance.

The size number was measured. The pkg wasm is 1999061 bytes. The budget `bytes` is 1999061, `from` 16388, `delta` 1982673, `delta_fraction` 120.9832. `1982673 / 16388` is 120.98322, which rounds to the recorded four places. Tolerance stayed 0.05. The cause text says the parent commit was not rebuilt and that cold start was not remeasured. The `to` figure matches the file on disk. It was not invented, and the tolerance was not widened to hide the jump.

No patient data in the new sources. The new prose that was checked has no em dash. The defects above are false sentences, not punctuation.
