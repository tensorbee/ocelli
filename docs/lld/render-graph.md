# Render graph

**F-IDs that contributed:** F-038, F-040
**Last updated:** 2026-10-18

HLD section 22's graph and scheduler, made into `crates/ocelli-render::graph`.

## The types

| Type | What it is |
|------|-----------|
| `Pass` | Section 22's enum: `Stack`, `VolumeRaycast`, `SegOverlay` |
| `PipelineKey` | Pass kind, blend mode and tier. The whole key, tier included |
| `Dirty` | Five flags from section 22 word for word: camera, VOI, slice, segmentation, data |
| `ViewportId` | A newtype, so an index cannot transpose at a call site |
| `RenderGraph` | Registered viewports, their dirt, and the pipelines compiled at creation |
| `FrameOutcome` | `Submitted`, `Noop`, `DeviceLost` or `Unavailable` |
| `Upload` | Extent of the `R32Float` stored-sample texture owned by a viewport |
| `UploadError` | Format refusal, a wrong sample length, or a viewport with no stored texture |

## The rules that hold

**Pipelines compile at creation.** Only the composite stack pipeline has a
shader body. A lookup for any other pass, blend and tier combination reports
unavailable rather than aliasing composite, which is deviation D-07's honesty
rule applied to a blend mode.

**One submission per submitted frame.** A tick encodes each dirty viewport
once into one encoder and submits once, and `submissions` counts them. That
counter is the observable half of the rule: a tick that submitted per
viewport would count its viewports. Dirt clears only after a successful
encode, so an unavailable viewport is reported again on the next tick rather
than hidden behind a submitted mixed frame.

**VOI has one write route.** `set_voi` writes the thirty-two uniform bytes
and sets the VOI flag together, and `mark_dirty` refuses a VOI mark. A window
change therefore cannot schedule a frame without updating the bytes it
renders from. It does not increment `uploads` and it does not write the
stored texture. Validation runs state first and the chain last, in the order
`UnknownViewport`, `PassHasNoUniform`, `ViewportNotRebound`, then the chain's
own answer.

**Stored samples upload with `write_texture`.** `register_stored` asks
`adapter.get_texture_format_features` for `R32Float` and creates that texture
in the viewport's GPU slot, beside the uniform buffer. `TEXTURE_BINDING` or
`COPY_DST` absent is `UploadError::FormatUnavailable`. No other format is
substituted. `upload_stored` copies the caller `&[f32]` with
`bytemuck::cast_slice` and `queue.write_texture`. Samples are stored values
and are not clamped. The length must be width times height. A wrong length
returns before the copy and before `uploads` changes. A matching extent
reuses the texture. A different extent replaces it at that call, which is a
data change and not a frame-loop allocation. The tick only binds the view it
already holds.

**`uploads` counts sample uploads, not window changes.** A new sample slice
increments it by one and sets `Dirty.data`. `set_voi` leaves it unchanged.

**Recovery drops per-viewport objects.** A bind group or buffer from a dead
device cannot be bound on its replacement, so `install_recovered` drops
every per-viewport slot, adopts the recovered tier, and marks every viewport
fully dirty. The caller re-supplies a caller-owned source through `rebind`.
A viewport that owns a stored texture is recreated through `rebind_stored`
at the extent the registry still holds, and that call does not increment
`uploads`. The registry keeps each pass, its last uniform values and the
stored extent, so nothing but GPU objects is rebuilt.

## Tiers

| Tier | Stored upload |
|------|----------------|
| A, WebGPU | Full, `R32Float` |
| B, WebGL2 | Full when the device can sample `R32Float`. Otherwise unavailable, with no substitute format |
| C, CPU | Unavailable. Tier C has no device and no texture. The CPU path remains `LutChain::map_into` |

## What this does not do

No surface is acquired, which is F-039. No staging-buffer pool is built.
`write_texture` takes the caller slice. The oracle still has no Ocelli frame
to judge, so decision D7 stays open. Tier B has never been exercised by a
gate. F-042 is its first test. Mip, MinIp and Average wait for F-054, which
depends on F-052. LabelmapEdgeProjection waits for F-067. The carried LINEAR
out-of-range finding is left where F-041 recorded it.
