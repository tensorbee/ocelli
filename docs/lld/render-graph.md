# Render graph

**F-IDs that contributed:** F-038
**Last updated:** 2026-10-17

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
renders from. Validation runs state first and the chain last, in the order
`UnknownViewport`, `PassHasNoUniform`, `ViewportNotRebound`, then the chain's
own answer.

**Recovery drops per-viewport objects.** A bind group or buffer from a dead
device cannot be bound on its replacement, so `install_recovered` drops
every per-viewport slot, adopts the recovered tier, and marks every viewport
fully dirty. The caller re-supplies views through `rebind`. The registry
keeps each pass and its last uniform values, so nothing but GPU objects is
rebuilt.

## What this does not do

No surface is acquired, which is F-039. No texture is uploaded, which is
F-040. The oracle still has no Ocelli frame to judge, so decision D7 stays
open. Tier B is full by construction and has never been
exercised by a gate: F-042 is its first test. Mip, MinIp and Average wait
for F-054, which depends on F-052. LabelmapEdgeProjection waits for F-067.
