# F-X002, Three-layer GPU-less render testing

**Status**: approved
**Epic ref**: X1.2
**Sprint**: S14
**Estimate**: 3w

## Normative source, transcribed

The HLD has no section that names lavapipe, SwiftShader, or a three-layer test. `docs/spikes/A7-tier-c.md` section A7.2 is the resolved spike this story implements. `docs/spikes/GATES.md` leaves the lavapipe measurement as this story's acceptance criterion.

### `docs/spikes/A7-tier-c.md`, section A7.2

| Layer | Mechanism | Catches | Cost | Runs |
|-------|-----------|---------|------|------|
| **1** | `cargo test`, no adapter | Tier C raster and LUT arithmetic | free | every PR |
| **2** | lavapipe via `force_fallback_adapter` on `ubuntu-latest` | wgpu pipeline construction, bind-group layouts, WGSL compilation, tier-B shader variants | free | every PR |
| **3** | Headless Chrome with SwiftShader | What a GPU-less session actually runs | slow | nightly or manual |

|  |
|----|
| **Layer 2 is not a simulator of the real thing and must not be described as one.** lavapipe is a Vulkan software rasteriser reached through wgpu. A GPU-less browser session runs SwiftShader reached through Chrome's WebGL2. Layer 2 proves the pipeline builds and the shader compiles. Only layer 3 says anything about real GPU-less behaviour. |
| **No layer is an oracle.** The reference is cornerstone3D through the harness, per §11. A software adapter proves code runs. It does not prove a pixel is right. |
| **Still to measure**, and it is F-X002's acceptance criterion rather than a gate question: that lavapipe actually resolves as a fallback adapter under the pinned wgpu version on `ubuntu-latest`. If it does not, layer 2 drops and layers 1 and 3 stand. |

### `docs/hld/08-validation-architecture.md`, section 11, the sentence deviation D-04 departs from

|  |
|----|
| Every pull request renders the corpus in CI. Every field bug becomes a permanent fixture. In production, shadow mode renders both libraries and alerts on divergence — the oracle running against real clinical traffic, and the same corpus a regulatory submission would want to see. |

Deviation D-04 keeps that corpus render off CI. This story does not put it back. Layer 2 is pipeline construction. Layer 3 is a session record. Neither compares a pixel to cornerstone3D.

### Deviation D-07, the tier C rule this session must keep

|  |
|----|
| A third tier, C, CPU. The resolved tier may be Cpu, and every tier-gated feature declares its CPU answer. A feature that cannot run on the resolved tier reports unavailable and never silently produces a different answer. |

### Pinned wgpu 30.0.1

`RequestAdapterOptions` has `force_fallback_adapter`. The wgpu-core backend copies that field into its own request. The probe uses that field and no other way of asking for a software adapter.

### What already exists

Layer 1 is the adapter-free `cargo test` surface, including the tier decision in `caps.rs`. This story does not rebuild it. `tools/tier-b/run.mjs` draws section 18.3 on headless Chromium with ANGLE on Metal. It is a tier B run and it is not this story. The oracle already launches Chromium with `--use-angle=swiftshader` and `--enable-unsafe-swiftshader` and checks that the renderer string is SwiftShader. That launch is the reference harness. It is not layer 3.

## What the specification does not cover

1. **How a dropped layer 2 is recorded.** The spike says the layer drops and the drop is the result. It does not name a file. A green step that says the probe was skipped is the failure this sprint names. The record is `ci/lavapipe-layer.json`. Absence of the file means the probe is required. `{"status":"dropped","reason":"..."}` is the only other accepted document, and the reason must be non-empty. `pass` and `skip` are refused. A dropped record does not run the probe and does not print a pass.

2. **Which string counts as lavapipe.** Mesa names the Vulkan device `llvmpipe`. Both `lavapipe` and `llvmpipe` count, case insensitive, in the adapter name, driver, or driver info. Any other adapter is not layer 2, including a hardware adapter returned by a fallback request. The strings are the ones A7 already lists.

3. **What layer 3 asserts.** The spike says it is what a GPU-less session runs, and that it is not an oracle. The script requires the renderer string to contain `SwiftShader`. Tier C is the success the classification rule already specifies for a software renderer. Tier B with that string is a failure, because that is the misdetection D-07 exists to prevent. The script does not compare pixels and does not read the corpus.

4. **Where the jobs live.** `.github/workflows/ci.yml` is the floor, and its header forbids a GPU job there. Layer 2 is not a floor gate and not `gate gpu`. It is a separate workflow so the floor checker and D-04 stay about the floor. Layer 3 is another workflow, on a schedule and on `workflow_dispatch`, because the spike runs it nightly or by hand.

## Approach

### Layer 1

No new raster and no new arithmetic. The existing adapter-free tests stay the layer. This story does not mark them ignored and does not move them behind a feature.

### Layer 2

`fallback_is_lavapipe` in `ocelli-render`'s `caps` module is a pure predicate over the three adapter strings. `llvmpipe` and `lavapipe` match. `Apple`, an empty string, and `SwiftShader` do not. SwiftShader is layer 3's string and must not satisfy layer 2.

`crates/ocelli-render/examples/lavapipe_probe.rs` builds an instance, requests an adapter with `force_fallback_adapter: true`, and prints one JSON object with the name, driver, driver info, device type and backend. Exit 0 when the predicate matches. Exit 1 when an adapter comes back and the predicate does not. Exit 2 when no adapter comes back. The example is not a `#[ignore]` test. `gate gpu` does not run it. A missing adapter must not look like a pass inside `cargo test`.

`.github/workflows/lavapipe.yml` runs on the same pull-request and push events as the floor workflow, on `ubuntu-latest`. It installs `mesa-vulkan-drivers`, points Vulkan at the lavapipe ICD, and runs the example. Before that, `scripts/lavapipe_layer.py` reads `ci/lavapipe-layer.json`. A missing file continues into the probe. A dropped file with a reason prints `layer 2 dropped: <reason>` and exits 0 without the probe. Any other document exits 1. The workflow log's last line is either `lavapipe resolved` or the drop line. This machine is not `ubuntu-latest`. A local run that finds no lavapipe is exit 2 and is not recorded as a pass. The first `ubuntu-latest` run is the measurement the spike asked for.

### Layer 3

`tools/swiftshader/run.mjs` uses the oracle's Playwright Chromium and the oracle's two SwiftShader flags. It does not import the comparator and it does not open the corpus. It builds `ocelli-wasm` with the existing `tier-b-probe` feature, which already returns the renderer string, the tier, and a demotion. The page calls that export.

Exit 0 when the renderer string contains `SwiftShader` and the tier is C. Exit 1 when the string contains `SwiftShader` and the tier is not C. Exit 3 when the string does not, or the page crashes, or the browser does not finish. The script's own log says it is not the oracle and not lavapipe.

`.github/workflows/swiftshader.yml` runs on a nightly schedule and on `workflow_dispatch`. It does not run on a pull request. It is not a gate in `bin/ocelli.sh`. Putting it in `--sprint` would let a missing browser become a skip inside the gate, which is the result this story refuses.

The implementer runs the script once on this machine and writes the renderer string and the tier into `docs/lld/tier-resolution.md`. A failure to launch is recorded there as a failure, not as a pass.

## Boundary and tier

- wasm-bindgen: the layer 3 page uses the existing `ocelli-wasm` probe export. No new `wasm_bindgen` attribute is added outside that crate.
- Pixels across the boundary: the probe export already returns a JSON report for a test page. This story adds no product path that copies pixels out.
- Render-loop allocation: none. These are test launches, not a frame loop.
- unsafe: none
- Tier A (WebGPU): n/a. The story does not draw a tier A frame. Layer 2 may compile a pipeline that tier A also uses, and a compile failure is a red probe.
- Tier B (WebGL2): layer 3 is the browser session. It is a pass only when that session resolves tier C. A tier B result under SwiftShader is a recorded failure.
- Tier C (CPU): layer 1 stays the adapter-free tests. Layer 3's expected resolution is tier C. The story does not implement a new CPU raster.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | `lavapipe` and `llvmpipe` match. `SwiftShader`, `Apple`, and an empty string do not. | `ocelli-render` unit test, no GPU |
| unit | A missing layer file means the probe is required. A dropped file with a reason is the drop. An empty reason, `skip`, and `pass` are refused. | `scripts` unit test, no GPU |
| browser | Headless Chrome with the SwiftShader flags reports a SwiftShader string and tier C, or the script exits non-zero. Not a pixel comparison. | `tools/swiftshader/run.mjs`, run once and recorded |

Layer 2's adapter resolution has no local fixture. `ubuntu-latest` is the fixture, and the workflow is the run.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column.

## Deviations

None. D-04 and D-07 are cited and not amended. The corpus stays off CI.

## LLD impact

`docs/lld/tier-resolution.md`, for the two new runs, the drop file, and the layer 3 record from this machine.

## Open questions

None. The spike states the three layers, the lavapipe acceptance, and the drop. The sprint states that a named skip is not that drop.
