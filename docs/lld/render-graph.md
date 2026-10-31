# Render graph

**F-IDs that contributed:** F-038, F-039, F-040, F-042, F-043, F-044, F-045, F-048, F-X003, F-057, F-059
**Last updated:** 2026-10-30

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
| `Upload` | Extent of the preserving float stored-sample texture owned by a viewport |
| `UploadError` | Format refusal, a wrong sample length, or a viewport with no stored texture |

## The rules that hold

**Pipelines compile at creation.** The composite stack pipeline and the
composite volume ray each have a shader body. A lookup for any other pass,
blend and tier combination reports unavailable rather than aliasing
composite, which is deviation D-07's honesty rule applied to a blend mode.

**Tier B compiles the stack fragment module.** The source is `VOI_WGSL`
followed by the graph fragment shader. The three window functions stay the
F-041 text. `PipelineKey` is still pass, blend and tier. Compute and storage
are a lookup family beside the key, and they have no body on either GPU tier.
`pipeline_family_compiled` is false for those families.
`request_outcome` returns `Unavailable` and does not hand back the fragment
pipeline. `tick` asks for the fragment family.

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

**The camera has one write route.** `set_view` takes the CSS matrix and a
device pixel ratio. It writes the 80-byte `DeviceCamera` into the uniform at
group 0 binding 2 and sets `Dirty.camera`. The uniform is sixteen column-major
CSS floats followed by four scale lanes. Lane `.x` is the device pixel ratio.
The other three lanes are padding. The fragment shader divides the
device-pixel position by that ratio before it multiplies by the CSS matrix.
Registration writes the identity CSS matrix and ratio `1.0` through
`device_camera`. `set_view` replaces those 80 bytes from the same helper. It
does not write the VOI uniform and it does not increment `uploads`,
`lut_uploads` or `brick_uploads`. A viewport that never calls `set_view`
still has that identity camera from registration. A fragment outside the
stored texture is `(0, 0, 0, 1)` and does not run `lut_chain`. `mark_dirty`
still accepts `camera`, because a resize sets that flag and keeps the stored
matrix and ratio. Device-loss recovery drops the uniform and keeps the CSS
floats and the ratio. `rebind` writes them into the new buffer as the same
80-byte `DeviceCamera`. The CSS matrix itself is `View::camera_matrix` in
[viewport.md](viewport.md). The 96-byte viewport state copy keeps its own
64-byte camera field. That readback is not this uniform.

**Stored samples upload with `write_texture`.** `register_stored` asks
`adapter.get_texture_format_features` for `R32Float`, then `Rg32Float`, then
`Rgba32Float`. A format is usable when it has `TEXTURE_BINDING` and
`COPY_DST`. `R32Float` wins on every tier when it is usable. Tier B may take
the next usable float format. Tier A does not substitute. If none qualify the
call is `UploadError::FormatUnavailable`. `Rgba8Unorm` is not a candidate.
A wider format packs each sample into `.r` and writes zeros in the other
channels at upload time, which is not a frame-loop allocation. The `R32Float`
path copies the caller `&[f32]` with `bytemuck::cast_slice` and
`queue.write_texture`. Samples are stored values and are not clamped. The
length must be width times height. A wrong length returns before the copy and
before `uploads` changes. A matching extent reuses the texture. A different
extent replaces it at that call, which is a data change and not a frame-loop
allocation. The tick only binds the view it already holds. Rebind checks the
format the slot remembered.

**`uploads` counts sample uploads, not window changes.** A new sample slice
increments it by one and sets `Dirty.data`. `set_voi` leaves it unchanged.

**`lut_uploads` counts colour-table selections, not window changes.**
`select_colour_lut` creates an `Rgba8Unorm` texture of height 1 and fills it
once with `write_texture`. That increments `lut_uploads` by one. `uploads`
stays the stored-sample counter. `set_voi` still writes thirty-two bytes. It
does not write the colour texture and it does not increment `lut_uploads`.
A device-loss restore writes the remembered table again and does not
increment `lut_uploads`. The placeholder texture created at registration is
not a selection and is not counted.

Two keys, and they are not interchangeable. A preset is indexed by the
display value `lut_chain` already produced. The texel replaces greyscale
normalisation. The section 18.4 uniform stays the LUT decision, including
`invert`. A palette is indexed by the stored sample, per deviation D-23.
Its texels are `ColorTransform::map_into`, copied once. The shader does not
walk the descriptor and does not run `lut_chain` for a palette. When no
colour LUT is bound, mode 0 keeps the greyscale normalisation, and that path
still emits the unclamped LINEAR value.

An out-of-range display index uses the table endpoints. That clamp is not
applied to the value `lut_chain` returns before the lookup. Palette on a
monochrome photometric interpretation is unavailable. A preset on a palette
image is unavailable. ICC is unavailable and is not passed through as device
RGB. `Rgba8Unorm` here is the colour table. It is still not a stored-sample
format.

Tier A, on this host's default adapter, read `grey` at display 40 back as
RGB 40, 40, 40 and the `stops` texels at 0, 128 and 255. Stored samples 9
and 10 read back the first palette entry and stored 11 read back the second.
`set_voi` left both `lut_uploads` and `uploads` unchanged across those
windows. With no colour LUT bound, the carried LINEAR finding read back
above 255, within `0.001` of `297.5000305175781`. That check is the existing
shader tolerance on an `Rgba32Float` value. It is not an 8-bit comparison
with tier B. A tier B colour LUT readback was not run, so the plan's tier A
against tier B failure condition was not measured. The ignored native GL
test was run and found no adapter. That miss is not a tier B colour pass.
The section 18.3 tier B run below is the greyscale path only.

**Recovery drops per-viewport objects.** A bind group or buffer from a dead
device cannot be bound on its replacement, so `install_recovered` drops
every per-viewport slot, adopts the recovered tier, and marks every viewport
fully dirty. The caller re-supplies a caller-owned source through `rebind`.
A viewport that owns a stored texture is recreated through `rebind_stored`
at the extent the registry still holds, and that call does not increment
`uploads`. Held frame textures are dropped before the surfaces they came
from. The shell still holds the canvases and supplies them again through
`bind_surface`. The registry keeps each pass, its last uniform values, the stored
extent and the colour table bytes, so nothing but GPU objects is rebuilt.
The kept camera is the CSS matrix and the device pixel ratio. `rebind`
writes those into a new 80-byte `DeviceCamera` uniform.

**A bound surface is the colour attachment for that tick.** `bind_surface`
records the surface and its extent and sets the camera flag. It does not call
`rebind` and it does not increment `uploads`. `tick` calls
`get_current_texture` for each dirty surface viewport that can encode.
Success and Suboptimal are drawn. Any other status leaves the slot empty,
keeps the dirt, and does not draw the caller-supplied placeholder. `present`
runs after the single submit, once per acquired texture, in registration
order. A viewport with no surface keeps the caller-supplied view from F-038.
If nothing was encoded, held textures are dropped without `present` and
nothing is submitted.

**Resize does not go through `rebind`.** `reconfigure_surface` drops any held
frame texture, then configures the surface at the new extent when the format
still matches the graph. A format that would differ returns
`SurfaceError::FormatDiffers` before `configure`, so the previous
configuration stays. `rebind` and `rebind_stored` are not called. A
stored-sample texture stays where it is and `uploads` does not change. The
camera flag is set so the next tick draws the new extent. `rebind` on a
viewport that owns stored samples still returns
`GraphError::StoredNeedsRebind`, and `rebind_stored` still recreates that
texture without incrementing `uploads`.

Native tests have no canvas. They replace a caller-supplied offscreen texture
through `rebind` and `rebind_stored`. That is the path a viewport with no
surface already has. A real surface texture cannot be stored across frames,
so the browser path acquires and presents inside `tick`.

**Tier C opens no surface.** `RenderGraph::new` still returns
`TierCUnavailable` on CPU. `attach_target` is not called. `device_extent`
still runs with no device. It is CSS size times device pixel ratio, rounded
half away from zero per axis, then saturated to `u32`. A strictly positive
CSS axis that rounds to 0 becomes 1. Non-finite inputs, a ratio that is not
strictly positive, and a positive ratio whose `f32` narrowing is not positive
are refused. That last condition keeps the CPU extent and the GPU camera
uniform on one policy. The function touches no GPU. F-X003 does
not make this graph draw. `CpuRaster` is a separate scalar path. It is not a
software device and it does not call `RenderGraph::new`.

**Bricks upload at true extent.** `upload_brick` copies one `Brick` into a
`TextureDimension::D3` texture. The format is `R8Uint` for `U8`, `R16Uint`
for `U16`, `R16Sint` for `I16`, and `R32Float` for `F32`. `Unorm` is not a
candidate. A format other than that mapping is `FormatUnavailable`, and no
second format is tried. Tier C returns `Unavailable` before a format is named
and before a texture is created, and the host brick is given back. An axis of
0 is `EmptyExtent`. An axis longer than `max_texture_dimension_3d`, or a row
whose byte count does not fit in `u32`, is `ExtentUnavailable`. The pinned
tier B floor for that limit is 256. A brick edge of 128 fits. A longer volume
axis is more bricks. `bytes_per_row` is the brick width times the sample
width. The texture extent is the brick extent. An edge brick is not padded
to a cube. The accepted buffer length is the voxel count times the sample
width. Any other length is `WrongLength` and returns before `create_texture`.

**`brick_uploads` counts brick uploads, not window changes.** A successful
upload increments it by one and replaces any texture already held for that
`BrickKey`. `set_voi` writes the thirty-two uniform bytes and does not
increment `brick_uploads`, `uploads`, or `lut_uploads`. `install_recovered`
drops the brick textures and their stamps and does not increment the counter.
Phase 1 keeps
every uploaded texture. There is no byte-budget eviction of those textures.
Frustum selection, LOD, the indirection texture and prefetch are HLD section
30 and are not built. The host brick is not inserted into `BrickCache` after
a successful copy.

**Progressive upload is a ready brick.** `RenderGraph::sync_volume` walks
`Volume::ready_bricks` and calls `upload_brick` when the level-0 bytes
differ from the stamp. The stamp is those bytes. It is written only after
the upload returns. A repeated sync of equal bytes does not increment
`brick_uploads`. A brick the list omits gets no texture and no zeros.
`install_recovered` drops the stamps with the textures, so the next sync
uploads ready bricks again. Tier C returns `BrickUploadError::Unavailable`
before a cut and before a stamp write. A grid product that does not fit in
`u32` is `ProgressError::Grid`. Coarser levels are not uploaded.
`ProgressReport` returns `present_count` and `dims[2]`. `sync_volume` takes
the same `&ResolvedAdapter` `upload_brick` already requires.

## Volume ray

**The composite volume ray compiles beside the stack.** The key is
`VolumeRaycast`, composite, and the tier. Tier A and tier B use one fragment
module. `VOI_WGSL` is included once. The ray entry has one `texture_3d<u32>`
binding per brick slot and a `texture_2d<u32>` residency texture on the next
binding. There is no compute entry and no storage buffer. MIP and the other
volume blends stay uncompiled. `request_outcome` for those keys is
`Unavailable`.

**The integral is front to back on voxel centres.** A step whose residency
flag is 0 is skipped before the brick is read. Display colour comes from the
existing VOI uniform. Opacity is the constant 0.5 until F-060. Colour and
alpha accumulate `(1 - A) * colour * 0.5` and `(1 - A) * 0.5`. The pipeline
blend is off, so the integral is not blended a second time.

**Every brick in the caller set is bound, or none of this call is.** Room is
the sampled-texture limit minus one, because residency takes a binding. A
limit of zero, a ready count of zero, or a count above that room returns
`SampledTextureLimit`. A count above the compiled slot count returns the same
error before any texture for that call is created. The ray does not draw a
subset. The caller slice is not compared with `Volume::ready_bricks`.

**Tier C returns `TierCUnavailable`.** `RenderGraph::new` and
`volume_ray_admission` both refuse CPU before an image exists. This story
does not return a CPU vector of ray colours. F-X005 owns that path.

**One submit per frame still holds.** The ray bind group is encoded only
when the viewport has been bound. `set_voi` writes the same thirty-two bytes
for a volume viewport. `set_view` stays stack-only.

## Tiers

| Tier | Stored upload |
|------|----------------|
| A, WebGPU | Full, `R32Float` |
| B, WebGL2 | `R32Float` when it can be sampled and copied. Otherwise `Rg32Float` or `Rgba32Float`. If none of the three qualify, unavailable. `Rgba8Unorm` is not used |
| C, CPU | Unavailable as a texture. No device. A preset indexes the same bytes after `LutChain::map_into`. A palette uses `ColorTransform::map_into`. |

## What this does not do

No staging-buffer pool is built.
`write_texture` takes the caller slice. The oracle still has no Ocelli frame
to judge, so decision D7 stays open. F-042 drew the section 18.3 rows on
tier B. Headless Chromium WebGL2, Playwright 1.62.1 from `tools/oracle`, with
ANGLE on Metal and not SwiftShader. The renderer string was
`ANGLE (Apple, ANGLE Metal Renderer: Apple M4 Max, Unspecified Version)`.
The stored format was `R32Float`. LINEAR read back
`[0.0, 127.819565, 255.0, 63.909782]` against `LutChain::map_into`
`[0.0, 127.81955, 255.0, 63.909775]`. LINEAR_EXACT read back
`[0.0, 127.5, 255.0, 63.75]` on both sides. The gap at input 40 stayed.
`LINEAR_EXACT` at input -160 was `0.0`. The script exited 0. Native
`Backends::GL` found no adapter. That miss is not the tier B run. Mip, MinIp
and Average wait for F-054, which depends on F-052. LabelmapEdgeProjection
waits for F-067. The carried LINEAR out-of-range finding is left where F-041
recorded it.
