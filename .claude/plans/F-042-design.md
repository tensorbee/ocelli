# F-042, WebGL2 fallback and tier-B shader variants

**Status**: approved
**Epic ref**: E6.6
**Sprint**: S13
**Estimate**: 5w

## Normative source, transcribed

### `docs/hld/05-rendering.md`, section 7

|  |
|----|
| **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup. |
| **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail. |
| **Blend modes are shader variants** — composite, MIP, MinIP, average — selected by specialisation constant rather than branching per fragment. |

### `docs/hld/11-decision-log.md`, decision D6

|  |
|----|
| WebGPU primary, WebGL2 a declared tier. Two shader paths; compute degrades, never fails. |

### `docs/hld/19-render-graph.md`, section 22

|  |
|----|
| **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame. |

### `docs/hld/15-lut-chain.md`, section 18.2, character for character

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

### Section 18.3, and deviation D-13

The HLD table gives `LINEAR_EXACT(-160) = 1.594`. D-13 records that the formula yields `0.000`, because the comparison is `<=` and `-160` equals `c - w/2`. Rows at 40, 240 and -60 stand, including the 0.32 gap at the window centre. This story asserts the corrected row. Tolerance stays `0.001`.

### What already exists, and what it does not prove

`voi.wgsl` has no `@compute` and no `var<storage>`. `graph.rs` samples with `textureLoad`. `voi::tests::the_shader_uses_nothing_tier_b_lacks` is a text search. `tests/voi_shader.rs` says that composing a render pipeline on the machine's resolved tier, which has been tier A, does not prove tier B. F-X002 is the later story that brings lavapipe and SwiftShader. This story is the one that runs the variant.

## What the specification does not cover

1. **Native GLES is not this machine's tier B.** wgpu-hal 30.0.1's `gles` module says it is designed for Linux and Android with EGL. The pinned crate's `gles` feature is the native backend. The `webgl` feature, already enabled by D-14, is the browser one. Tier B on this Mac is headless Chromium's WebGL2, not a native GL adapter. A native `Backends::GL` probe that finds nothing records that and does not count as the run.

2. **Section 7's specialisation constant and section 22's pipeline key.** F-038 already recorded these as one rule at two levels. This story does not reopen blend modes. Mip, MinIp and Average stay unavailable until F-054. The variant this story adds is the tier axis. Tier B compiles the fragment module. A request for a compute or storage pipeline on tier B returns `Unavailable` and does not alias the fragment pipeline.

3. **`R32Float` on WebGL2.** F-040 refuses the upload when the format cannot be sampled. If the WebGL2 device cannot sample `R32Float`, this story compiles a fragment variant whose texture type matches a format that preserves the section 18.3 values exactly as `f32` after `textureLoad`. `Rgba8Unorm` is not that format. If no such format exists, tier B rendering of the stack pass reports unavailable. It does not draw a different number.

4. **The browser harness is not a new gate.** `bin/ocelli.sh gate --sprint` already runs `gpu` and `oracle`. The tier B proof is an ignored `ocelli-render` test when a GL adapter exists, plus a Playwright page, built like `tools/oracle`, that loads the wasm module and forces the WebGL backend. It is invoked from the story's verification notes and from the `gpu` profile only if it can live as an ignored cargo test. Where it cannot, the Playwright script is what the story runs before completion, and the sprint gate's existing `oracle` invocation is not rewritten to hide a skip.

5. **F-X002 is not pulled forward.** SwiftShader-versus-lavapipe stays that story's acceptance text. This story uses the WebGL2 context Chromium already gives the oracle.

## Approach

Pipeline compilation stays at graph creation. The key keeps its tier field. For tier B the compiled module is the fragment variant. `pipeline_compiled` returns false for a compute key on tier B, and `tick` reports `FrameOutcome::Unavailable` rather than binding the fragment pipeline under a compute key.

The fragment variant contains the three section 18.2 functions by including `VOI_WGSL` unchanged. It does not re-type them. It adds no second slope, no second invert, and no clamp of the LINEAR output. The carried finding in `docs/lld/pixel-pipeline.md` stays untouched.

The WebGL2 run uploads the four section 18.3 stored values, draws, and compares to `LutChain::map_into` within `0.001`. The same draw on the machine's tier A adapter, when one exists, must agree with the same chain. A tier B result that compiles and differs from the CPU chain by more than the tolerance fails the test. That is the D6 failure mode.

Limits passed to `request_device` stay `adapter.limits()`, which F-037 documented as the line only a downlevel adapter can police. The WebGL2 device is opened with those limits. `Limits::default()` is not used.

Tier C is unchanged. The graph still returns `TierCUnavailable` and runs no shader.

## Boundary and tier

- wasm-bindgen: the Playwright harness loads `ocelli-wasm`. Any new export is in that crate only. If F-039 has not yet added the `ocelli-render` dependency, this story adds it, because the harness cannot reach the shader otherwise. The size budget moves once, with the cause recorded, and is not guessed here.
- Pixels across the boundary: the harness reads back bytes for the assertion. That readback is the test, not a product path, and it is the same shape as the existing GPU tests.
- Render-loop allocation: none beyond F-038
- unsafe: none
- Tier A (WebGPU): full, existing fragment pipeline, same numbers
- Tier B (WebGL2): full for the composite stack fragment variant when a preserving sampled format exists. Compute and storage report unavailable. Other blend modes stay unavailable.
- Tier C (CPU): unavailable for this shader. The CPU answer remains `LutChain::map_into`.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Section 18.3 on a WebGL2 device matches `LutChain::map_into` within `0.001`, including `LINEAR_EXACT(-160) = 0.000` per D-13. PS3.3 C.11.2.1.2 and C.11.2.1.3.2. | Playwright WebGL2 run, and the ignored GL test when an adapter exists |
| unit | A compute pipeline key on tier B is `Unavailable` and is not the fragment pipeline. | `ocelli-render` test, no GPU, over the key table |
| unit | The tier B module text still has no `@compute` and no `var<storage>`. | beside `voi::tests` |
| fixture | The same four rows on tier A still match. A divergent variant fails. | existing `voi_shader` ignored test, unchanged tolerance |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column. The three VOI functions are already F-041's rows.

## Deviations

None.

## LLD impact

`docs/lld/tier-resolution.md`, `docs/lld/render-graph.md`, and `docs/lld/gpu-ownership.md`. The text that says tier B has never been exercised is replaced with what this story actually ran, including a native GL miss if that is the measurement. The ownership page also names the second `create_surface`, which is `resolve_webgl`.

## Open questions

None. Native GLES is out of reach on this host by the pinned hal's own statement. WebGL2 in Chromium is the tier the HLD names.
