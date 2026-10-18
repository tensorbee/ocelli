# F-044, Colour LUT / transfer function upload and preset library

**Status**: approved
**Epic ref**: E6.8
**Sprint**: S13
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/15-lut-chain.md`, section 18, opening and stage table

|  |
|----|
| This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.4, the uniform this story must not replace

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

|  |
|----|
| A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2. |

### Deviation D-23, the stage 4 correction

|  |
|----|
| No arm of stage 4 takes a `Display` input. Palette colour maps the **stored** value through the three palette LUTs. The `RGB` and every `YBR_*` route map decoded samples that never entered the chain at all. |

### PS3.3 C.7.6.3.1.5, as already implemented

`PaletteColorLut` is three `LutDescriptor`s. The descriptor's second value is the first stored pixel value mapped. Inputs below it clamp to the first entry. A first value of 0 means 65536 entries. This story does not reimplement that lookup. It uploads the descriptor's already-resolved outputs.

### The carried LINEAR finding

`docs/lld/pixel-pipeline.md` records that a legal `LINEAR` chain can return a value outside its declared output range, on CPU and GPU, because the `f32` evaluation of C.11.2.1.2 does not reproduce its own breakpoint. This story does not clamp that output to hide it.

## What the specification does not cover

1. **The HLD names no preset table.** F-046 is window, level, VOI presets, invert and colormap, in S15. F-060 is the volume transfer-function editor, in S21. This story uploads a display-keyed colour table and a stored-keyed palette. The library is two tables whose bytes are written in the plan, not copied from another viewer. `grey` maps a display value `d` in `0..255` to RGB `(d, d, d)`. `stops` is the three texels `(0, 0, 0)`, `(255, 0, 0)`, `(255, 255, 255)` at indices 0, 128 and 255, with the intervening entries left black. Those bytes are the fixture. No clinical name is attached.

2. **ICC is not this story.** The stage-4 row names it. `docs/lld/pixel-pipeline.md` already says it is unimplemented. An ICC profile reports `Unavailable`. It is not passed through as device RGB.

3. **Two keys, and mixing them is the defect.** A palette texture is indexed by the stored sample, per D-23. A preset texture is indexed by the display value the VOI shader already produced. A preset does not run modality, VOI, photometric interpretation, or a second YBR conversion. The section 18.4 uniform stays the LUT decision, including `invert`.

4. **Index clamping is not the LINEAR fix.** A 256-entry table cannot address a display value outside `0..255`. The index uses the table endpoints for an out-of-range display value. That clamp is a property of the table address, it is tested as such, and it is not applied to the value `lut_chain` returns before the lookup. The finding remains visible on the greyscale path, which does not go through the table.

5. **Section 20's staging pool is not built.** The upload is `write_texture` of a table that fits in a few hundred bytes, once, at selection. A window change does not upload it again. F-040's image upload counter stays put.

## Approach

`ColourLut` in `ocelli-render` holds either `Preset(PresetId)` or `Palette`. The GPU object is an `Rgba8Unorm` 2D texture of height 1, created at selection, filled with `write_texture`. `grey` and `stops` are the only `PresetId` values.

The fragment shader keeps `lut_chain`. When no colour LUT is bound, the output stays the current greyscale normalisation. When a preset is bound, the display value selects a texel and the greyscale normalisation is not also applied on top. When a palette is bound, the stored sample selects the texel and `lut_chain` is not run. Binding a palette for a monochrome photometric interpretation is `Unavailable`, matching `ColorTransform`'s refusal. Binding a preset for a palette image is `Unavailable`.

The palette texels are produced by `PaletteColorLut`'s existing lookup, once, into a buffer the upload then copies. The shader does not contain a second descriptor walk.

`set_voi` still writes thirty-two bytes and does not touch the LUT texture. A counter `lut_uploads` makes that observable. Selecting a preset increments it by one. Changing the window does not.

Tier C applies the same table bytes through a CPU function that indexes them after `LutChain::map_into` for a preset, and through `ColorTransform::map_into` for a palette. The table bytes are built in one place and shared. There is no second palette formula.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The LUT texture is created when the preset or palette is selected.
- unsafe: none
- Tier A (WebGPU): full for preset and palette upload
- Tier B (WebGL2): full. `Rgba8Unorm` is a WebGL2 sampled format. The same table bytes are uploaded. A tier B result that differs from tier A by more than the existing `0.001` display tolerance, after the 8-bit quantisation both sides share, is a failure.
- Tier C (CPU): full, using the same table bytes and the existing `ocelli-pixel` palette path

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Palette first mapped input 10. Stored 9 yields the first entry. Stored 10 yields that entry's RGB. PS3.3 C.7.6.3.1.5. Compared to `PaletteColorLut`, not re-derived. | `ocelli-pixel` values uploaded and read back, ignored GPU test, plus a CPU test with no GPU |
| fixture | `grey` at display 40 is RGB 40. `stops` at 128 is `(255, 0, 0)` and at 0 is black. | CPU table test and GPU readback |
| unit | A window change leaves `lut_uploads` unchanged and leaves the stored-image upload counter unchanged. | GPU test |
| unit | An out-of-range display index hits the endpoint texel, and the greyscale path still emits the unclamped LINEAR value the finding describes. | CPU test for the index, existing shader test for the finding |
| unit | ICC input is `Unavailable`. A palette bound on a monochrome frame is `Unavailable`. | unit test, no GPU |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column.

## Deviations

None. D-23 is applied as written. ICC stays unimplemented and reports unavailable, which is the absence `docs/lld/pixel-pipeline.md` already records, not a new departure.

## LLD impact

`docs/lld/pixel-pipeline.md`, for the upload and the two keys. `docs/lld/render-graph.md`, for the LUT texture and the counter. `docs/lld/README.md`, because the index rows for those two files mirror their headers.

## Open questions

None. The HLD names no clinical preset, and this plan does not invent one. F-046 and F-060 keep the later surfaces.
