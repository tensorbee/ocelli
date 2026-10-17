# F-039, OffscreenCanvas in a render worker, resize and DPR handling

**Status**: approved
**Epic ref**: E6.3
**Sprint**: S13
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/07-concurrency-and-typescript.md`, sections 9 and 10

|  |
|----|
| **Start single-threaded per worker.** N decode workers, each holding its own WebAssembly instance; one render worker owning the GPUDevice and every OffscreenCanvas; the main thread doing DOM, events and tool UI. No SharedArrayBuffer, no wasm-bindgen-rayon, no nightly toolchain. |

|  |
|----|
| DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver |

### `docs/hld/05-rendering.md`, section 7

|  |
|----|
| **One submit per frame.** The render graph tracks dirty viewports and issues a single submission across all of them, driven by requestAnimationFrame inside the render worker. |

### `docs/hld/19-render-graph.md`, section 22

|  |
|----|
| **One queue.submit() per frame** across all viewports, from the render worker's requestAnimationFrame. |
| **Device loss is a real state, not an error path.** Handle device_lost, rebuild the device and all resources, and restore viewport state from the shell's copy — the same recovery path §23 needs for panics. |

### `docs/hld/DEVIATIONS.md`, D-12, the purpose paragraph

|  |
|----|
| D2's PURPOSE is untouched: CLAUDE.md says one bindgen crate "is what makes the desktop and server targets new entry points rather than rewrites", and wgpu abstracts the target itself, so `ocelli-render` carries no browser binding in its source and compiles for native unchanged. What D2 forbids is OUR code binding to the browser outside one crate. |

### Allocation note

|  |
|----|
| One GPUDevice serves N viewports — no context pool |

### `ci/wasm-size-budget.json`, `bearing_on_gate_A4`

The recorded text says the number that bears on gate A4 arrives with F-039, and that F-039 is the first story with a reason for `ocelli-wasm` to reach `ocelli-render`.

## What the specification does not cover

1. **Which crate names `OffscreenCanvas`.** wgpu 30.0.1's safe constructor is `Instance::create_surface(SurfaceTarget::OffscreenCanvas(web_sys::OffscreenCanvas))`, read from the pinned crate at `src/api/surface.rs`. Naming `web_sys` in `ocelli-render` would put a browser binding in that crate, which D-12's purpose forbids. `ci/check-device-ownership.sh` fails any crate but `ocelli-render` whose source contains the substring `create_surface`. The split this plan uses is the one that satisfies both. `ocelli-wasm` names the canvas. `ocelli-render` owns a method `attach_target` whose body is the only `create_surface` call. `ocelli-wasm` does not contain that substring.

2. **`GpuContext` holds no `Instance` today.** A surface is created from the instance that produced the adapter. `probe` already builds that instance. This story stores it on `GpuContext` and threads it through `GpuContext::new`. No second instance is constructed for a surface.

3. **Device pixels.** The HLD does not give the formula. This plan uses `round(css * devicePixelRatio)` per axis, in `f64`, half away from zero via `f64::round`, then saturating cast to `u32`. A positive CSS size whose product rounds to 0 becomes 1, so the backing store cannot be empty while the element is visible. Non-finite inputs and a `devicePixelRatio` that is not strictly positive are refusals.

4. **Native tests have no canvas.** The extent function and the "replace the previous target" rule are tested with an offscreen `wgpu::Texture` the graph already accepts as a caller-supplied view. The browser proof is a Playwright page, the same Chromium the oracle already installs. It transfers an `OffscreenCanvas` into a worker and checks `width` and `height` against the extent function.

5. **One wasm module for every worker.** Decision D2 leaves one bindgen crate. This story adds `ocelli-render` as a dependency of `ocelli-wasm` unconditionally. Decode workers load that module and do not call the surface exports. The size cost is the A4 number this story was reserved to measure. The new budget is a release measurement via the size gate's accept path, written with the cause. The plan does not guess the byte count.

6. **Resize drops the previous target.** A resize reconfigures the surface, or reallocates the stand-in texture, and drops the previous view before the next frame. The graph's `rebind` path from F-038 is how the new view is installed. Device loss already drops per-viewport GPU objects. A recovered context gets new surfaces from the shell's canvases, which the shell still holds.

## Approach

`device_extent(css_width, css_height, dpr) -> Result<(u32, u32), ExtentError>` lives in `ocelli-render` and touches no GPU. It is the only definition of backing-store size.

`GpuContext` gains the `Instance`. `attach_target` takes `impl Into<wgpu::SurfaceTarget<'_>>` and calls `instance.create_surface`. `configure_viewport` takes the surface, the adapter capabilities, and the extent, and configures it. The previous configuration's texture is not kept. `SurfaceConfiguration::format` is the first format `surface.get_capabilities` reports that can be a render attachment. The plan does not hard-code `Rgba8Unorm` or `Bgra8Unorm`.

The render graph gains a slot from viewport id to the current surface texture view. `tick` still submits once. When a viewport has a surface, that tick's colour attachment is `get_current_texture`, and `present` runs after the single submit, not per viewport. A viewport with no surface keeps F-038's caller-supplied view.

On tier C, `attach_target` is not called, because tier C opens no device. The graph's existing `TierCUnavailable` stands. The extent function still runs, so a CPU session can size a buffer later without a second formula.

`ocelli-wasm` exports `attachOffscreenCanvas(viewport, canvas, cssWidth, cssHeight, dpr)`. The export builds `SurfaceTarget::OffscreenCanvas` and calls `attach_target`. `web-sys` is a dependency of `ocelli-wasm` only.

A resize export takes the same four numbers, computes a new extent, and replaces the backing store. The test that matters is `dpr != 1`. CSS size 100 by 50 at `dpr` 2 is a backing store of 200 by 100. Using the CSS size as the backing store fails that test.

One `GpuContext` serves every viewport. The test opens one context, attaches two targets, and asserts a single device. There is no context pool and no second `request_device`.

## Boundary and tier

- wasm-bindgen: `ocelli-wasm` only. The canvas type is named there.
- Pixels across the boundary: no
- Render-loop allocation: the surface texture is replaced on resize, which is not a per-frame allocation. Frame submission allocates nothing beyond what F-038 already does.
- unsafe: none. The safe `SurfaceTarget::OffscreenCanvas` path is used. `create_surface_unsafe` is not called.
- Tier A (WebGPU): full
- Tier B (WebGL2): full, same canvas path, format taken from surface capabilities
- Tier C (CPU): unavailable. No device and no surface. Extent math still runs.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Extent at `dpr` 1, 2 and 1.25, the empty-product floor of 1, and refusal of a non-positive `dpr`. | `ocelli-render` unit test, no GPU |
| unit | A resize drops the previous texture id and the next frame draws the new extent. | `ocelli-render` test, ignored, needs an adapter |
| unit | Two viewports, one device, one submit. | the same ignored test |
| integration | A worker's `OffscreenCanvas` width and height equal `device_extent`. | Playwright page under `packages/core`, using the oracle's Chromium |

No new pixel arithmetic. The ignored test draws the graph's existing clear so a stale extent is visible as the wrong readback size.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column.

## Deviations

None. D-12 is followed by keeping the browser type out of `ocelli-render`. D-14's `webgl` feature is already on that crate and is not changed.

## LLD impact

`docs/lld/gpu-ownership.md`, `docs/lld/render-graph.md`, `docs/lld/build-targets.md`. The size budget file is updated from the measurement, not from a guess.

## Open questions

None. The crate split is fixed by D-12 and by `ci/check-device-ownership.sh`. The size move is the one `ci/wasm-size-budget.json` already assigns to this story.
