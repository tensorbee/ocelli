# F-235, VOI shader stage 1 matches LutChain on every tier

**Status**: draft
**Epic ref**: E6.9
**Sprint**: S47
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md`: "VOI shader stage 1 matches LutChain on every tier: the shader's rescale gives the bits of ocelli_pixel::modality on a backend that fuses multiply and add, proven by a tier A fixture from the F-227 reviews that fails today". Layer Rust, depends on F-227, milestone M20.

`allocation.json` notes: "Found by F-227's fourth and fifth reviews on Apple M4 Max under Metal. Answered by the operator on 2026-11-06: a story in S47 with a tier A fixture that fails today."

## Normative source, transcribed

### HLD 18, `docs/hld/15-lut-chain.md`

"This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters - do not let a second copy of this logic appear anywhere."

18.1:

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

18.4:

```wgsl
// ocelli-render/shaders/voi.wgsl
struct VoiParams {
    center : f32,
    width : f32,
    slope : f32,
    intercept : f32,
    ymin : f32,
    ymax : f32,
    fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
    invert : u32,
};
@group(0) @binding(0) var<uniform> voi : VoiParams;
```

"*A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2.*"

### The tier rule

`docs/hld/26-differentiating-capabilities.md` section 31, verbatim:

|  |
|----|
| A kernel with no fallback marks its feature unavailable; it never silently produces a different answer. |

`CLAUDE.md` generalises it: "A feature that cannot run on the resolved tier **reports unavailable**. It never quietly produces a different result".

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"the shader's `stored * slope + intercept` in `crates/ocelli-render/shaders/voi.wgsl` compiles to a fused multiply-add, so tier A can differ from `LutChain` by up to eight f32 units at the top of the range, not the two F-041 recorded. ... F-227's fifth review found a second case: where `stored * slope` nearly cancels the intercept, the fused rounding error reaches hundreds of millions of units and flips greys at any window width, in 15,879 of 800,000 adversarial draws. ... **Answered by the operator on 2026-11-06:** a story in S47 makes the shader's rescale match `LutChain` on every tier, with a tier A fixture that fails today."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e` and in `../ocelli-wt-f-227` at `892fda6`.

1. `voi.wgsl:122`, stage 1: `let m = stored * voi.slope + voi.intercept;`. `lut_chain(stored : f32)` is the one stage 1 in WGSL, and the stack fragment in `graph.rs`, `raycast.wgsl`, `slab.wgsl` and `fusion.wgsl` all call it.
2. `crates/ocelli-pixel/src/lut.rs:202-204`: `pub fn modality(stored: Stored, slope: f32, intercept: f32) -> Modality { Modality(stored.0 * slope + intercept) }`, a separate multiply and add in f32. `grep mul_add` finds nothing in `ocelli-pixel`.
3. Stage 2's LINEAR and LINEAR_EXACT end in `((x - cp) / wp + 0.5) * (ymax - ymin) + ymin`, the same multiply then add shape, in the shader (`voi.wgsl` 56 to 76) and in `apply_window` (`lut.rs` 766 to 806).
4. naga 30.0.1, the pinned version, parses `fma` only as an explicit builtin and has no contraction, `precise` or fast-math control in its MSL or SPIR-V back ends. wgpu-hal 30.0.1's Metal device sets `setPreserveInvariance(true)` on `MTLCompileOptions` and sets no math mode.
5. `crates/ocelli-render/tests/voi_shader.rs`: `the_shader_agrees_with_ocelli_pixel_over_a_sweep` (line 531) asserts `SWEEP_TOLERANCE: f32 = 1e-4` with identity rescale, slope 1 and intercept 0 (`chain()` at 102). `stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu` (777) uses slope 2 and intercept -1024. Every test is `#[ignore]` and runs through `bin/ocelli.sh gate gpu`. A machine resolving tier C returns early and passes.
6. Bricks upload in their stored integer format, `R8Uint`, `R16Sint`, `R16Uint`, or `R32Float` for F32 (`brick_upload.rs` 79 to 82).
7. F-227 pass 4 line 75, measured on Apple M4 Max under Metal: "stored 2371 and 2372 with slope 9.00603e-7 and intercept 1854.0933 give CPU modality values `1854.0953` and `1854.0955`, adjacent floats. The accepted window has centre `1854.0953` and width `1.22e-4`, one ulp. The CPU draws 0 and 255, and **the GPU draws 0 and 0, the 99th percentile black**." Line 74: "Every GPU value below equals the probe's `f32::mul_add` emulation bit for bit."
8. F-227 pass 5 line 12: "stored 56893 at slope 4.8812434e-2 and intercept -2777.1045, CPU -1.8798828e-2 and fused -1.868023e-2, which draws 0.619, grey 1, where the CPU draws 0." That value was emulated with `f32::mul_add`, which pass 4 showed bit identical to Metal for its inputs.
9. F-227 carries `const PERCENTILE_MARGIN: usize = 16;` in `crates/ocelli-pixel/src/auto_window.rs:520` on its branch, documented as covering "a tier whose stage 1 rounds differently by fewer units than the move".

## What the specification does not cover

1. How the shader obtains `modality`'s bits when the backend compiler may contract. The HLD gives the formula and the uniform and is silent on contraction. Options under Open questions.
2. Whether stage 2's final multiply and add (fact 3) is in scope. This draft proposes yes, because it is the same defect class on the same code path, and a fix to stage 1 alone leaves a one-unit difference wherever `ymin` is not zero.
3. What happens to F-227's 16-unit margin (fact 9) once tier A matches. This draft proposes keeping it until this story's fixture is green on tier A, then reviewing it in this story.

## Problem and reproducer

On Apple M4 Max under Metal, through the F-041 harness shape, the shader draws fact 7's two stored values the same grey where `LutChain` draws 0 and 255. No test in the tree uses a rescale whose product is inexact (fact 5), so `gate gpu` is green.

## Scope, API and compatibility

In scope: stage 1 and, under decision 2, stage 2's linear tail in `voi.wgsl`, the upload path if the chosen option needs it, the F-041 test file, and F-227's margin review.

Non-goals: SIGMOID's `exp`, whose precision the sweep tolerance already declares. Tier C, which runs `LutChain` itself.

Compatibility: `VoiParams` stays HLD 18.4's eight fields and 32 bytes, and a window drag still writes only that uniform.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| stack viewport fragment | `lut_chain` in `voi.wgsl` | tier A and B | display value per texel |
| raycast, slab, fusion | `lut_chain` | tier A and B | display value per sample |
| tier C raster | `LutChain::map_into` | CPU | the reference value |
| F-227 auto window | `PERCENTILE_MARGIN` | all tiers | the accepted window |

## State transitions and invariants

The invariant this story establishes: for every stored value and rescale, `lut_chain(stored)` on tiers A and B returns the f32 `LutChain::apply` returns on tier C, bit for bit for LINEAR and LINEAR_EXACT. A window drag still changes only the uniform.

## Ownership, commit and recovery

Not applicable unless option O1 is chosen, in which case the modality table is owned by the viewport's render state, rebuilt when the rescale changes, and a failed upload leaves the previous frame on screen.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Fact 7's pair. Stored 2371 and 2372, slope 9.00603e-7, intercept 1854.0933, LINEAR_EXACT centre 1854.0953 width 1.22e-4, output 0 to 255. Tier A draws 0 and 255, equal to `LutChain` | `stage_one_matches_lut_chain_across_an_inexact_product`, tier A, `gate gpu` | Today's shader, which draws 0 and 0 | Implementation on the M4 Max host |
| AC2 | Fact 8's cancellation case. Stored 56893, slope 4.8812434e-2, intercept -2777.1045. Tier A's modality bits equal `ocelli_pixel::modality`'s | `stage_one_matches_lut_chain_where_the_product_cancels_the_intercept` | A fix that narrows the error without removing it | Implementation on the M4 Max host |
| AC3 | AC1 and AC2 fail on today's shader on the M4 Max | old-source RED recorded | A fixture that a fused stage 1 passes | Implementation, recorded |
| AC4 | Sweep with inexact rescales. The F-041 sweep gains rescales whose products are inexact, CT-like slope 1.2525758 intercept -1024 among them, and asserts bit equality of stage 1 and the sweep tolerance of stage 2 | extended `the_shader_agrees_with_ocelli_pixel_over_a_sweep` | A sweep that keeps slope 1 | Implementation |
| AC5 | Tier B. AC1 and AC2 hold through the WebGL2 downlevel path where the host has one, or are recorded unavailable with the reason | tier B run or a recorded unavailable | Claiming tier B from tier A | Implementation, recorded |
| AC6 | Window drag unchanged. A drag writes the 32-byte uniform only, no texture or buffer upload | existing `voi_params` tests plus an upload counter | An option that re-uploads per drag | Implementation |
| AC7 | F-227's margin reviewed with AC1 green, and kept or changed with the reason recorded | review | Leaving a constant whose reason is gone, unexamined | Implementation |

## Approach

Depends on the option chosen under Open questions. Each option keeps one stage 1 in `ocelli-pixel` or a bit-equal one in WGSL, and each is proved by AC1 to AC4 on the M4 Max.

## Dependencies, write ownership and resources

Declared: F-227, which writes `ocelli-pixel` and owns the margin constant. Writes `crates/ocelli-render/shaders/voi.wgsl`, `crates/ocelli-render/tests/voi_shader.rs`, and under O1 the upload path in `ocelli-render` and `ocelli-viewport`. Exclusive resource: the M4 Max GPU for `gate gpu`, one GPU story at a time.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate gpu
cargo test -p ocelli-pixel
```

AC1 to AC3 need a real adapter that contracts. The host is the operator's M4 Max, the one the F-227 reviews used. A run on a tier C host passes vacuously and is not evidence (fact 5).

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none on a window drag. Under O1, the table is sized once per rescale change, outside the frame loop
- unsafe: none
- Tier A (WebGPU): full, bit-equal stage 1, which is the story
- Tier B (WebGL2): full if the chosen option runs on the downlevel profile, otherwise reported unavailable for the affected path, never a different answer
- Tier C (CPU): full, unchanged, it is the reference

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2, hand-computed against PS3.3 C.11.1 and the `LutChain` value | `crates/ocelli-render/tests/voi_shader.rs` |
| gpu | AC4, AC5 | same file, `gate gpu` |
| unit | AC6 | `crates/ocelli-render/tests/voi_params.rs` |

## Parity surface covered

To be named at approval from `docs/hld/B-parity-surface.md`, the modality LUT and VOI rows the stack viewport already claims.

## Deviations

None proposed. If option O3 is chosen and a backend cannot be made bit equal, a deviation row would be raised at approval.

## LLD impact

`docs/lld/pixel-pipeline.md` and `docs/lld/render-graph.md`, stage 1's tier agreement and the measured bound replacing "two ULP at 255".

## User-facing documentation impact

`examples/folder-viewer/src/more.html`, if it states the shader's agreement figure. To be checked at approval.

## Open questions

- **Q1, how stage 1 gets `modality`'s bits.** Affects AC1 to AC6. Decision owner: the lead in the S47 design round. Options:
  - **O1, recommended.** A modality table for integer stored types, computed by `ocelli_pixel::modality` for every stored value of the frame's range and read by the shader, and for F32 stored data, stage 1 applied on the CPU before upload into the `R32Float` texture fact 6 already uses, with the uniform's rescale set to identity. Stage 1 then exists once, in `ocelli-pixel`, which is HLD 18's rule. Cost: one table upload per rescale change, none per window drag.
  - **O2.** Keep stage 1 in WGSL and make it immune to contraction, for example by computing the product's exact error with an explicit `fma` and correcting. Needs a measurement that each backend compiler does not contract the correction too, and fact 4 says there is no source-level control.
  - **O3.** Rely on a backend setting. Fact 4 found none in wgpu-hal 30.0.1 or naga 30.0.1.
  Deferral leaves tier A drawing a different grey from tier C under fact 8's inputs.
- **Q2, stage 2's linear tail.** Proposed in scope, decision 2. Decision owner: the lead.
- **Q3, F-227's margin.** Proposed reviewed after AC1 is green, decision 3. Decision owner: the lead.
