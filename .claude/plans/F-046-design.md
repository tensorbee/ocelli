# F-046, Window/level, VOI presets, invert, colormap

**Status**: approved
**Epic ref**: E7.2
**Sprint**: S15
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/15-lut-chain.md`, section 18, the stage table and the instruction

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere.
```

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.2, the three functions, character by character

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

The lower comparison is `<=`. The upper comparison is `>`. LINEAR uses `c - 0.5` and `w - 1`. LINEAR_EXACT uses neither.

### Section 18.3, the four rows, and deviation D-13

Soft-tissue CT window, centre 40, width 400, output range 0 to 255.

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is <=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

D-13: the `LINEAR_EXACT` value `1.594` at input `-160` is wrong. Section 18.2 clamps at `x <= c - w/2`, which is `-160`, so the value is `ymin`, and the formula body is `0` there anyway. `1.594` is the value at `x = -157.5`. Fixtures use `0`. Rows 2, 3 and 4 stand. The `f32` literals already locked in `crates/ocelli-pixel/tests/voi.rs` are the ones this story asserts:

| Input | LINEAR | LINEAR_EXACT |
|----|----|----|
| -160 | 0.0 | 0.0 |
| 40 | 127.819_55 | 127.5 |
| 240 | 255.0 | 255.0 |
| -60 | 63.909_775 | 63.75 |

This story does not recompute them and does not clamp the carried LINEAR finding in `docs/lld/pixel-pipeline.md`.

### Section 18.4, the uniform a preset must fill and not replace

```wgsl
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
```

```text
A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2.
```

### Section 18.1, the sequence rule this story must not break

```text
If a Modality LUT Sequence is present it wins over slope and intercept.
```

The same precedence is section 18's stage 2 row: a VOI LUT Sequence wins over window centre and width. `VoiParams::from_chain` already returns `VoiSequenceNotExpressible` and `ModalitySequenceNotExpressible`. A preset does not replace either sequence with the linear numbers it overrode.

## What the specification does not cover

1. **The HLD names no preset library.** F-044 recorded that and shipped two colour tables, `grey` and `stops`. This story's window presets are three named pairs, written here, not copied from another viewer. Each one calls `VoiTransform::new`. None of them has a formula.

   | Name | Centre | Width | Function |
   |----|----|----|----|
   | `soft_tissue` | 40 | 400 | `VoiFunction::Linear` |
   | `lung` | -600 | 1500 | `VoiFunction::Linear` |
   | `bone` | 300 | 1500 | `VoiFunction::Linear` |

   Output range stays the range the chain already has. `soft_tissue` is the section 18.3 window. `lung` and `bone` are plan-defined numbers with no clinical-standard claim. A test asserts the centre, the width and `Linear`, and asserts `apply` of the four rows above against `soft_tissue`.

2. **Invert is the existing presentation shape.** It sets `PresentationLutEvidence::Shape(Inverse)` or `Shape(Identity)` and rebuilds `LutChain::new`. It does not XOR with `MONOCHROME1`. An explicit shape already wins over the photometric default inside `PresentationTransform::new`, so the two cannot compose. A `MONOCHROME1` frame with explicit `IDENTITY` is not inverted.

3. **Colormap is `PresetId::Grey` or `PresetId::Stops`.** No third table. The viewport crate holds a `Colormap` enum with those two variants and does not depend on `ocelli-render`. One match in `ocelli-render` maps `Colormap` to `PresetId` and calls `ColourLut::preset`. A palette photometric interpretation stays `ColourLutError::Unavailable`. ICC stays unimplemented.

4. **A preset on a sequence-driven chain is unavailable.** If `VoiTransform::window()` is `None`, applying a preset returns `PresentationError::Sequence` and does not build a windowed replacement. The same refusal applies when stage 1 is a modality sequence, because the rebuilt chain would drop it.

5. **The viewport crate does not call `set_voi` or `upload_stored`.** It returns the rebuilt `LutChain` and the `Colormap`. The existing `RenderGraph::set_voi` writes the thirty-two bytes and does not increment `uploads`, `lut_uploads` or `brick_uploads`. The existing `select_colour_lut` increments `lut_uploads` once per selection. A later window change goes through `set_voi` only, so the colour texture stays. This story adds no wgpu call.

## Approach

`Presentation` in `crates/ocelli-viewport/src/presentation.rs` holds the current `LutChain` and a `Colormap`. `apply_preset` rebuilds stage 2 through `VoiTransform::new` with the preset's centre, width and `Linear`, and keeps the chain's modality transform and presentation evidence. `set_window` does the same for a caller-supplied centre, width and `VoiFunction`, still through `VoiTransform::new`, and refuses a width outside that function's domain with the error `VoiTransform::new` already returns. `set_invert` rebuilds the chain with an explicit shape. `set_colormap` stores the enum.

`ocelli-viewport` depends on `ocelli-pixel`. It stays `no_std`. `lib.rs` gains `pub mod presentation`.

The TypeScript names `WindowPreset`, `Colormap` and `Invert` are declared once, in F-100. This story does not add a TypeScript file.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Building a chain is the call that handles the command, not the render loop. The render loop writes the uniform the chain already produced.
- unsafe: none
- Tier A (WebGPU): full, through the existing `set_voi` and `select_colour_lut`
- Tier B (WebGL2): full, the same uniforms. F-042 already drew the section 18.3 rows on WebGL2.
- Tier C (CPU): full for the chain, because `LutChain::map_into` is the arithmetic and it has no device. Drawing the result is F-X003 and stays unavailable here. A colormap on tier C indexes the same preset bytes F-044 defined, after `map_into` for `Grey` and `Stops`. It does not add a formula. ICC remains unavailable on every tier.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | `soft_tissue` on modality inputs `-160`, `40`, `240`, `-60` matches `0.0`, `127.819_55`, `255.0`, `63.909_775`. The same chain is not `LINEAR_EXACT`: input `40` is not `127.5`. PS3.3 C.11.2.1.2, D-13 on the `-160` exact row. | `crates/ocelli-viewport/tests/presentation.rs` |
| fixture | `lung` reports centre `-600`, width `1500`, `Linear`. `bone` reports `300`, `1500`, `Linear`. | same |
| unit | `apply_preset` on a chain whose `window()` is `None` returns `Sequence` and leaves the chain in place. | same |
| unit | `MONOCHROME1` with `set_invert(false)` has `inverts() == false`. `MONOCHROME2` with `set_invert(true)` has `inverts() == true`. | same |
| unit | `set_window` with width `0` under `Linear` is `InvalidWindow`. Width `1` under `Linear` is accepted. Width `1` under `LinearExact` is accepted. Width `0` under `LinearExact` is refused. These are `VoiTransform::new`'s domain, not a new one. | same |
| unit | After `set_colormap(Stops)`, a `set_window` does not change the stored `Colormap`. The colour table is not rebuilt by a window change. | same |
| regression | The carried LINEAR finding is not clamped. This story does not edit `lut.rs` or `voi.wgsl`. | review of the diff |

Mutate the `soft_tissue` width from `400` to `401` and confirm the centre-row fixture goes red. Revert.

## Parity surface covered

Appendix B's aggregate row `VOI LUT functions` counts `LINEAR`, `LINEAR_EXACT`, `SIGMOID`. All three already exist. This story drives them through presets and `set_window`. There is no `Covered by` column.

## Deviations

none. D-13 and D-23 are cited and not extended. A new preset table is not a deviation, because the HLD does not specify one.

## LLD impact

- `docs/lld/viewport.md`, a presentation section
- `docs/lld/pixel-pipeline.md`, one paragraph that presets call `VoiTransform::new` and are not a stage

## Open questions

None.
