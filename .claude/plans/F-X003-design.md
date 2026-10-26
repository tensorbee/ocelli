# F-X003, CPU stack raster, refinement and tier divergence evidence

**Status**: approved
**Epic ref**: X1.3
**Sprint**: S16
**Estimate**: 6w
**Owner**: xAI initial implementation, Codex completion, independent review deferred by operator

## Normative source, transcribed

HLD section 18: "Implement it once, in ocelli-pixel, and let the shader read
the parameters". Section 26: "No allocation in the render loop." and
"Measure with the benchmark harness before optimising anything."

Spike A7.3b: "Reduced resolution during interaction, full resolution on
settle." and "The settled frame is the one the divergence bound is measured
on." A7.3: "Ocelli's budget is no worse than those two numbers", interactive
and idle incumbent cost. "Ocelli's idle cost must be indistinguishable from
zero". Decision D14 requires a measured divergence bound rather than a
bit-exact reproducibility claim.

HLD section 25.1 holds the monochrome bound: at least 99.9 percent within one
LSB, zero over two, informative-region signed mean within 0.1 display code
when inputs, parameters and geometry agree. Geometry is 1e-6 mm and a quarter
canvas pixel. Colour's perceptual threshold is not numerically specified.
Do not invent it or change an existing tolerance to close this story.

## Problem and scope

Tier C resolves, but RenderGraph::new returns TierCUnavailable and no CPU
stack is drawn. Build a real raster using the shared mapping and LutChain,
not a software GPU session. Add visible refining/settled state and measured
interactive, idle and frame-change costs. CPU MPR and volume rendering are
F-X004 and F-X005 and remain outside this story.

## Consumer map and proposed concrete API

| Consumer | Existing provider | Context to preserve |
|----------|-------------------|---------------------|
| CPU texel selection | F-048/View mapping | CSS/device scale, pixel centers, placement, pan/flip/rotation |
| Monochrome output | LutChain::map_into | Modality, selected VOI and explicit presentation exactly once |
| Preset colours | ColourLut::map_preset_into | Shared Grey/Stops bytes, table-address policy |
| Palette/colour | ocelli-pixel ColorTransform | Stored-keyed palette and decoder-reported layout, no second YBR conversion |
| Browser presentation | wasm-side owning worker | No JS shell pixel API, concrete canvas binding |
| State/events | F-101 protocol | Frame generation and refining state, observable before measurement |
| Divergence | existing comparator/candidate contract | Same synthetic/corpus inputs, parameters and settled geometry |

Propose a concrete `CpuRaster` in `ocelli-render/src/cpu.rs`. This crate
already owns pure ColourLut tables and depends on viewport. It needs no
new reverse dependency, trait, cache or device. The CPU constructor takes a
maximum extent and pre-sizes scratch and RGBA buffers. Its raster operation
accepts `&View`, `&Presentation`, typed stored samples and source/target
extents. Proposed signature:

```rust
pub fn raster_into(
    &mut self,
    view: &View,
    presentation: &Presentation,
    stored: &[Stored],
    source: [u32; 2],
    target: [u32; 2],
    quality: RasterQuality,
    out: &mut [u8],
) -> Result<RasterResult, RasterError>
```

`RasterQuality` distinguishes interacting from settled, and `RasterResult`
names the produced extent and generation. RGBA output length is checked
before writing. The wasm binding owns presentation to the OffscreenCanvas,
not the TS shell. Colour inputs need a distinct resolved entry point through
ColorTransform. Freeze its signature and supported input inventory before
approval, rather than silently treating colour as monochrome or dropping it.

## Approach and alternatives

Reuse F-048's f64 mapping and the same nearest-texel selection established by
the GPU stack. A destination sample center maps once to a source texel.
Out-of-image pixels are opaque black before the LUT, matching the GPU rule.
Call ocelli-pixel for value arithmetic. Preserve the carried LINEAR finding
for analysis rather than clamping it to disguise divergence. Quantization to
RGBA must be explicit and matched to existing GPU output conventions.

Preallocate at viewport creation/resize. An interaction updates a reduced
resolution and invalidates the prior settled generation. On settle, schedule
one full-resolution draw for the latest generation. Stale results cannot
become the displayed frame or enable measurements. No idle repaint loop.
Define refinement state on the F-101 readback/event surface and make it visible
in the shell. Do not implement new per-event callbacks or a JS pixel getter.

Measure the scalar path with the benchmark harness first. Then add wasm
SIMD128 inner loops where measured and validate scalar/SIMD against the same
fixtures. No shared memory, extra threads or new arithmetic formula. A7's
SIMD requirement is not waived by choosing to measure first.

## Acceptance and discriminating tests

1. Adapter-free Rust tests draw synthetic mono frames with nonidentity
   slope/intercept, all three VOI functions, MONOCHROME1 and explicit invert.
   Use the PS3.3 C.11 fixtures, never renderer-generated expected values.
2. CPU texel identity and black border agree with GPU at identity, negative
   coordinates, fit, anisotropic spacing, nonzero measured shift, quarter
   rotation, flip and zoom. Include DPR 1.25 and 2 through the shared mapping.
   Wrong center convention, double spacing or double windowing must fail.
3. Grey/Stops output and stored-keyed palettes share existing concrete owners.
   Decoder-reported colour space controls conversion exactly once. Unsupported
   ICC stays unavailable. Freeze the full colour acceptance inventory below.
4. Interaction produces observable refining state. Settle restores full
   resolution for the latest generation only. Rapid scroll/window changes,
   cache miss, cancellation and out-of-order completion cannot show a neighbour
   or mark stale pixels settled. Measurement stays gated until settled.
5. For matched settled frames, measure tier A against C over every applicable
   corpus view using the existing comparator coverage and tolerance contract.
   Record absent/unmeasured classes honestly and publish the measured bound.
   Reference-only rendering and scalar/SIMD agreement cannot satisfy this row.
6. Use release wasm, plain and SIMD builds. Benchmark session.cpu_interactive,
   session.cpu_idle and cine.frame_change_rate on the same GPU-less workload
   as the incumbent, with physical host/runtime identity and CPU-time divided
   by wall-time. Report interactive/idle separately, include noise floor and
   idle scheduling evidence. No invented budget, threshold or baseline.
7. After sizing, instrument allocations over a render and verify none are
   introduced. A settled idle session draws nothing until invalidated. This
   deterministic control complements elapsed-time and vCPU observations.
8. Register permanent runners for all three F-X003 subjects before completion,
   with baseline provenance and the approved regression threshold policy.
   Do not mark done while a required measurement or parity class is missing.

## Dependencies and safe order

Declared dependencies F-029, F-045 and F-X001 are done. Actual composition also
needs F-048's mapping and F-101's refinement/presented state. Implement F-048,
then F-101's common boundary, then CPU raster and final boundary integration.
Treat these as explicit execution dependencies even though imported scheduling
puts the three stories in one sprint. Keep one writer for View, canvas binding
and the command/event protocol. Serialize GPU and browser measurements.

The full viewer candidate gate is F-X021 in S20 after F-052. This does not
waive this story's tier A/C acceptance. Design a bounded stack candidate
producer using the existing F-012 contract now, or replan with an approved
scope/dependency decision. Do not claim the later producer already exists.

## Checks and evidence stages

During implementation run ocelli-render and viewport check/test/clippy,
existing pixel fixtures, worker browser cases with no adapter, and scalar/SIMD
execution. Run benchmarks before optimization and after the change on the
recorded host. Run actual tier A/C candidate comparisons, then required
integrated sprint gates on the final staged tree and independent review.
Retain separate native, browser, adapter-free, SwiftShader and external parity
receipts with source/configuration/input identities.

## Boundary and tier

- wasm-bindgen: only the ocelli-wasm canvas integration
- Pixels across the boundary: no exposed JS shell pixel buffer
- Render-loop allocation: pre-sized scratch/output at create or resize
- unsafe: no new site outside existing allowlist
- Tier A (WebGPU): reference side of measured settled-frame comparison
- Tier B (WebGL2): existing GPU stack unchanged, no inferred parity claim
- Tier C (CPU): real raster and visible refinement, scalar and SIMD measured

## Parity surface covered

Only observations actually completed. Tier A/C bound is required, no
bit-exact claim. Colour thresholds remain unqualified until approved.

## Deviations and LLD impact

None approved. Any output/tolerance decision requires normal design review.
Update render-graph, viewport, feature-availability, public-api,
TypeScript packaging, benchmarks and build-targets with mirrored index rows.
Add a focused CPU LLD only if the implementation requires a separate area.

## What the specification does not cover

Section 18 says the LUT chain exists once. It does not specify a CPU canvas
type, a colour perceptual threshold, or a numeric CPU-cost budget. A7 asks
for a comparison with an incumbent on a GPU-less host. No such host is
available. That row stays open. The operator answered the colour scope, the
bounded producer and the refinement wire on 2026-10-18.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Settled raster | Interaction quality | Reduced extent, refining bit 3 set, kind 6 carries the flag | The previous settled generation is not labelled settled |
| Refining | Settle for the latest generation | Full extent, refining clear, one presented event | A stale completion does not display and does not clear refining for the newer generation |
| Any quality | Sample outside the image | Opaque black `(0,0,0,255)` before the LUT | LINEAR is not clamped |
| Monochrome stored samples | `LutChain::map_into` | Display through the existing chain once | A second window in the raster |
| Palette or RGB that `ColorTransform` supports | Decoder-reported space | RGB through that transform once | ICC and any other unsupported space return unavailable and draw nothing in their place |
| Unsupported colour | ICC or codec-owned space | `RasterError::Unavailable` | No invented perceptual score |

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Synthetic mono frames with slope and intercept, all three VOI functions, MONOCHROME1 and explicit invert match the PS3.3 C.11 fixtures | `cpu.rs` tests, expected values copied from the pixel fixtures | Expected values read back from the raster | implementation |
| AC2 | Identity, negative coordinates, fit, anisotropic spacing, measured shift, quarter turn, flip, zoom and dpr 1.25 and 2 select the same texel as the shared mapping. Outside samples are opaque black | same tests | A half-texel patient shift or a second dpr multiply | implementation |
| AC3 | Grey and Stops use the existing preset tables. Palette and supported RGB use `ColorTransform`. ICC is unavailable. Colour pixels are not scored | colour fixture | A mono average standing in for RGB, or a new threshold | implementation |
| AC4 | Interaction sets refining. Settle draws the latest generation only. A stale result cannot become settled | generation fixture and kind 6 payload | An idle repaint or a JS pixel getter | implementation |
| AC5 | One settled stack frame per applicable row the raster can draw is handed to the existing F-012 comparator. Tier A is the current GPU stack when an adapter opens. Tier C is this raster | candidate producer | A claim that F-X021 already exists, or a passed row with no frame | integrated when an adapter opens. Otherwise the producer is built and the GPU side stays unrun |
| AC6 | Interactive and idle cost against a GPU-less incumbent | not observable here | An invented budget, threshold or host | external, unmet. Owner is a future representative CPU-baseline host |
| AC7 | After sizing, a settled draw allocates nothing. Idle draws nothing until invalidation | storage counter in the raster test, and a counting-allocator probe over one settled draw | A `Vec` grown inside the pixel loop | implementation. The probe is temporary, recorded and deleted, as F-031 did |
| AC8 | The three F-X003 bench subjects have runners. They do not record a sample on this machine | bench registry | A completed acceptance row with no measurement | implementation of the runner. The measurement stays open |

## Colour and the bounded producer

Draw monochrome through `LutChain`. Draw Grey and Stops through the existing
preset tables. Draw palette or RGB through `ColorTransform` when the decoder
reports a supported space. ICC and any other unsupported space report
unavailable. Do not score colour pixels against an invented threshold.
Mono tier A against C remains required when both frames exist.

This story feeds the existing F-012 comparator one settled stack frame per
applicable row the raster can draw. Tier A is the current GPU stack. Tier C
is this raster. That is not F-X021 and it is not a CPU-cost budget.

The wasm binding owns the CPU canvas in `ocelli-wasm`. Pixels are not copied
to the shell. Refinement uses F-101 flag bit 3 and ring kind 6. The shell
reads copied state and the drained ring.

SIMD128 waits for a benchmark that shows it is worth adding. No such
measurement exists here, so the first raster is scalar. That is not a waiver
of A7. It is the "measure first" rule with the measurement still missing.

## Verification and evidence reuse

Run `ocelli-render` and `ocelli-viewport` check, test and clippy. Pixel
fixtures stay the expected values. `bin/ocelli.sh gate oracle` runs when the
producer writes a candidate the harness can see. A missing GPU adapter does
not become a tier A pass. A missing CPU-baseline host does not become an
interactive or idle pass. No tolerance and no wasm size budget moves unless
`bin/ocelli.sh wasm` measures a real release growth, and that growth is
reported rather than used to edit the budget file.

## Open questions

None that block the implementation. One acceptance row stays open on purpose.

CPU cost: the operator confirmed on 2026-10-18 that no representative
GPU-less host or incumbent baseline is available. AC6 remains unmet. Do not
fabricate numbers or mark the story complete.

Colour: draw supported colour and leave it unmeasured.

Producer: the bounded stack producer above. F-X021 stays in S20.

Refinement wire: flag bit 3 and ring kind 6, shared with the approved F-101
plan.

## Repair, 2026-10-19

A successful command, a new committed source, and a successful extent or
scale change invalidate a settled CPU frame. An empty or refused batch does
not. A failed resize does not. The initial open, whose device extent was
zero, does not bump generation.

Colour scratch is reserved with `reserve_source` before the draw. The draw
does not allocate that scratch and does not clone the presentation. Kind 6
is `publish_cpu_frame`, after a successful canvas blit on the wasm path.
`publishCpuCanvas` does not publish when the blit fails. There is no pixel
getter.

`tools/oracle/src/candidate.rs` calls `compare_view` with the CPU raster's
settled RGBA for stored 40, LINEAR 40/400. That is one synthetic `mono16`
frame. It is not a corpus pass and not a tier A observation.

The three tier C bench runners execute a workload and then refuse a host
figure. They record no vCPU fraction and no changed-frame rate.

## Repair, 2026-10-19, worker publication

The owned worker canvas uses `OffscreenCanvasRenderingContext2D`. The
context and one `ImageData` are retained at bind and replaced when the
device extent changes. A drawn frame copies borrowed session bytes into that
image. `putImageData` uses the retained image.

A written draw stays refining until the blit succeeds.
`accept_presentation` clears refining and records settlement only after
`putImageData` returns. A failed blit leaves the frame retryable. The next
publication redraws the current generation and emits one kind 6 event. A
command between the failure and the retry publishes the newer generation.

AC5 stays open. The synthetic `compare_view` call is one frame. AC6 stays
open. The host runners name a missing CPU-time and wall-time collector and
a missing representative host. They do not record a fraction. Browser colour
and interactive reduced resolution were not wired through this canvas
binding. The binding called `draw_committed_mono` only. The colour and
reduced-resolution section below is the entry that removes that limit.

## Repair, 2026-10-19, resize commit point

`resizeCpuCanvas` asks the session to accept or refuse the size before it
changes the bitmap. A refused extent, including a zero axis, leaves the
canvas dimensions, the visible frame and the retained publication where they
were. An identical device extent does not assign canvas width or height.
That assignment clears the displayed pixels even when the numbers are the
same, so a repeated resize must not classify a cleared bitmap as settled.

A device extent that actually changes still replaces the retained image
after the raster has invalidated settlement. The next publication draws that
frame and emits kind 6. The same-size path keeps the retained image.

## Repair, 2026-10-19, colour and reduced resolution

`publishCpuColor` takes one `u32`. Value 1 is interleaved native RGB.
`CpuRaster::raster_rgb_into` calls `ColorTransform::resolve` with samples
per pixel 3, photometric RGB, planar configuration interleaved, native pixel
data, preserved photometric evidence, preserved layout and no palette. The
wasm crate does not take a new pixel dependency for that call. The transform
runs once per publication, before the pixel loop. `reserve_source` runs
before the draw and is not called from inside the loop.

Value 0 and every other value return `RasterError::Unavailable`. The call
does not draw, blit or enqueue kind 6. The previous bitmap stays. The
remembered shade stays.

The remembered shade starts as monochrome. `publishCpuCanvas` still forces
that monochrome path and the full device extent, and remembers monochrome
only after the draw returns Ok. `publishCpuColor(1)` remembers RGB only
after that draw returns Ok. `publishCpuInteraction` uses the shade already
remembered.
Palette, YBR and ICC stay on the native `raster_color_into` fixtures. This
entry does not embed a palette LUT and it does not add a colour threshold.

The reduced extent is `max(1, device / 2)` on each axis. The raster samples
that grid with the existing `canvas_to_index` mapping. Replication onto the
canvas is not a second resampling and not a photometric formula.

The retained `ImageData` stays the bound device size. Interaction does not
assign canvas width or height. That assignment would clear the bitmap, which
the resize section already forbids for an unchanged extent. Kind 6 width and
height are the drawn extent. During interaction that extent is the reduced
grid. On settle it is the full device.

A presentation buffer is allocated beside the draw buffer at bind and when a
real extent change resizes storage. A drawn frame zeroes bytes past its
prefix, then expands the prefix into that buffer. The source column is
`min(drawn - 1, device_index * drawn / device)`, and the same rule applies
to rows. The canvas blits the device-sized buffer. A full-device draw is the
identity case of that rule.

Interaction calls `begin_cpu_interaction`, draws `RasterQuality::Interacting`,
defers presentation, and publishes kind 6 only after `putImageData` returns.
It does not call `accept_cpu_presentation`. Refining stays set and the
generation is not marked settled. A failed blit publishes nothing and leaves
refining set. The next interaction begins again, so the event names the
newer generation. A command between the failure and the retry begins a
generation as well. Colour does not read the monochrome LUT, so SetInvert
does not change the RGB bytes.

Settle is `publishCpuColor(1)` for the current generation at the full device
extent. Accept runs only on that settled path, and only after the blit
returns. A second call is idle. The public settle entry has no generation
argument, so an older generation cannot be selected. A direct raster call
that still names the previous generation remains `RasterError::Stale` and
writes nothing.

The resize section is unchanged. An identical device extent does not assign
canvas width or height. A zero axis is refused before the bitmap changes.

There is no pixel getter. Stored interleaved RGB `10, 20, 30, 40, 50, 60`
on a 2 by 1 source, a 2 by 1 device, an identity view and dpr 1 is the
browser fixture. The unfitted origin is half a CSS pixel. Image coordinate
equals CSS because the raster adds that half back. Full columns sample CSS
0.5 and 1.5, so the bytes stay `10, 20, 30, 255` and `40, 50, 60, 255`.
The one reduced centre samples CSS 1.0, which is source column 1. Both
canvas pixels are therefore `40, 50, 60, 255`. Those bytes are the stored
channels. They are not a monochrome average.

AC5 stays the missing applicable-corpus candidate producer. The three
interaction bench subjects stay unimplemented instruments and record no
sample. AC6 stays the missing collector and the missing representative
GPU-less host. Those three gaps are separate.

## Repair, 2026-10-19, mode preserved on refusal

Source length and rendering mode are cache-validity inputs. The monochrome
raster and the colour raster both accept that length before they return a
settled idle result. A mismatch is `RasterError::Length` and writes nothing.
A stale generation is still reported as stale. A matching length still
returns idle when the settled generation is current.

`CpuShade` changes only after the requested draw returns Ok. That includes
a valid idle of a compatible payload. `select_supported_color` accepts
code 1 and does not remember RGB. `draw_committed_mono` draws as
monochrome and remembers that shade only after Ok. `draw_committed_rgb`
does the same for interleaved native RGB. `publishCpuColor` still refuses
every code other than 1 before the draw. That refusal leaves the shade
unchanged.

`publishCpuInteraction` checks the remembered payload before
`begin_cpu_interaction`. A length refusal does not bump the generation and
does not set refining. The following valid interaction uses the preserved
route. The test does not reset the mode.

Palette, YBR and ICC stay on the native colour fixtures. This repair does
not add a palette LUT, a colour threshold or a perceptual score. The resize
section is unchanged.

## Repair, 2026-10-19, applicable stack candidate

The bounded producer lives in `tools/oracle/src/produce.rs`. It reads a
reference run directory and the permitted corpus files. It does not add a
shell command and it does not change `ocelli-compare gate`. The existing
command is the harness connection, with `--reference` and `--candidate`
pointing at two different directories the producer writes.

A stack row is selected when the reference rendered it, the photometric
interpretation is `MONOCHROME1` or `MONOCHROME2`, there is one sample per
pixel, and the categories include `mono16`. Colour, palette, YBR, volume
reformats and rows the reference did not render are unsupported. Colour is
not scored. Stored samples come from the corpus file through the existing
parser and codec. The settled frame is `CpuRaster` at the sidecar frame
extent. Spacing, window, slope, intercept and photometric interpretation
come from that sidecar, because those are the parameters the comparator
requires to agree and the ones the reference page applied. Presentation LUT
Shape is not applied. The reference page does not set invert from it.
The candidate applies the requested parallel scale after the physical fit,
unless that scale is already equivalent to the fit within the existing world
bound. The supported pose is a centred focal point and a view-up equal to
the negated column direction. An off-centre focal point, a rotated view-up,
or an oblique gaze is refused before a sidecar is published. A gaze parallel
to the plane normal is supported on both sides. The reference stack reset
places the camera on the negative-normal side, with view-up equal to the
negated column. That side is the unflipped column order. The positive-normal
side reverses the horizontal basis inside the raster. The reference
`flipHorizontal` and `flipVertical` fields are the observed camera readback.
A horizontal flip already moves the camera onto the opposite side and sets
the flag, so the producer applies that pose once and does not reflect again
from the flag. The published flips are those observed fields. The raster
coefficient stays internal. Position, focal point and view-up are published
only after the pose check. A vertical flip also negates view-up, and that
pose is refused. Presentation parameters still come from the sidecar.
Reference pixels are not copied.

Synthetic `compare_settled_linear_grey` remains the positive control.
A frame of stored `0` against the same hand grey remains the wrong candidate
the comparator rejects. Neither control is a corpus row.

Tier A stays unrun when no adapter is opened for this comparison. The three
interaction instruments stay unimplemented. The CPU-time and wall-time
collector stays unconfigured. The representative GPU-less host stays
unavailable. Those are separate from this producer.

## Codex completion implementation, 2026-10-19

This dated section supersedes earlier missing-instrument and missing-producer
statements. Browser session instruments and a CPU-time collector now exist.
The representative GPU-less incumbent host remains unavailable. Normal
acceptance runners refuse without that host. Local diagnostics and release
browser controls do not satisfy AC6 or set a baseline.

The bounded native producer now supports actual tier A using the existing
RenderGraph stack, explicit Grey LUT and native texture readback. CPU and
GPU candidates execute the same matched pose and presentation. Candidate
execution identity is recorded separately from the reference environment.
The full bounded comparison observes 67 pixel passes, zero pixel failures
and 17 unmeasured views. The unchanged coverage gate remains red. External
cornerstone parity remains red. AC5 is not marked satisfied.

Plain and SIMD-enabled execution are separate observations. Enabling
`simd128` is not proof of an optimized raster inner loop or a representative
speedup. No optimization or SIMD performance acceptance is claimed before
measurement. Independent review is deferred by the operator. This document
records implementation and open acceptance, not approval of these changes.

## Repair, 2026-10-25, display-code quantisation

The oracle found a systematic defect that the per-story fixtures could not:
the display value to 8-bit code mapping truncates toward zero, while the
pinned reference rounds to the nearest code. Every monochrome view is
therefore one code dark on roughly half of its informative pixels.

Measured against `codex-tier-c-final/candidate` and its matched reference:

- `real__ct_cmb_mml__00000001`, informative signed mean `-0.491`, 36,646
  pixels at `-1`, 37,985 at `0`, maximum absolute difference `1`.
- The same signature on every `LINEAR` monochrome row. Of the 67 pre-repair
  failures, 42 failed only HLD 25.1's signed-mean bound of `0.1`, 24 failed
  both that bound and the 99.9 percent within one LSB rule at a maximum
  absolute difference of `64`, and one, `synthetic__mr_nonsquare_spacing`,
  failed only the max-absolute rule.
- A hand-derived synthetic ramp reproduces it exactly. Stored 1104 with
  rescale `1` and intercept `-1024` is modality `80`. PS3.3 C.11.2.1.2 at
  centre `40` and width `400` gives `153.3835`, and the reference emits
  `153` while the candidate emits `153` there. Stored 1120 gives modality
  `96`, display `163.609`, reference `164`, candidate `163`.
- Tier A and tier C differed by at most one code on 28 of 84 frames before
  the repair, with maximum informative signed mean `0.0037249936353526015`,
  and are byte-identical after it. The defect was shared by both tiers and
  not a tier divergence.

The reference builds its colour table with `Math.floor(value * 255 + 0.5)`
in vtk.js and reads it back through an 8-bit texture, so it rounds. Deviation
D-11 makes the pinned reference the definition of correct until a register
entry says otherwise, and no register entry covers a one-code offset that
25.1 itself rejects. `byte_from_channel` already rounds for the colour path,
so truncation in the grey path is also an internal inconsistency.

The repair rounds to the nearest code, ties up, at both sites that quantise:
`display_index` in `ocelli-render/src/colour.rs` and `table_index` in the
fragment shader in `ocelli-render/src/graph.rs`. No formula, tolerance,
baseline, sampling or applicability changes. The existing fixtures that
assert a truncated byte are corrected to the nearest code and cite PS3.3
C.11.2.1.2 plus HLD 25.1 and D-11, because their expected value now derives
from the reference rather than from the implementation.

AC5's producer comparison is unaffected in kind. The SIGMOID width-half row
remains a separate open diagnosis: the reference page calls cornerstone's
`toLowHighRange`, and the installed `createSigmoidRGBTransferFunction` on the
traced range statically yields an applied width of 1.5, while the recorded
rendered codes fit 0.5. The reference's applied width stays unresolved pending
an oracle run, so that row is an open diagnosis rather than a settled width
comparison. Correction dated 2026-10-26: the earlier sentence in this
paragraph said the page passes the declared width through `toWindowLevel` and
settles the applied width at 1.5, which the recorded codes do not support.

Measured after the repair, tier C against the matched reference is
`39 pass, 28 fail, 17 unmeasured` and still red. The 28 are separated: 24
fail only the max-absolute rule at the fit boundary or on the SIGMOID row,
three MR rows fail only the signed-mean bound at about `0.17`, and
`synthetic__mr_nonsquare_spacing` fails both. The 17 unmeasured views still
carry `divergent-while-unmeasured`. Tier A against tier C is `67 pass, 0
fail, 17 unmeasured`, byte-identical on every produced frame with render
hash `95ac9185be70be670e29a62bdafeec2180f287785bf99658fa3aa50218d45aa4` on
both sides. The A/C run's `coverage-loss` verdict comes only from the shared
census, which was seeded from the reference-only identity run and was not
edited. External cornerstone parity remains a separate standing result.

AC7's permanent counter covers backing-storage reservation. The pixel loop's
allocation freedom is measured by a temporary counting `GlobalAlloc`
integration test over one settled mono draw, as F-031 did. It observed zero
allocations, and pushing a `Vec` inside `rgba_of_preset` made it red with
"settled draw allocated 1". The mutation was reverted and the probe deleted,
so the R5 unsafe scan never sees it. The storage counter remains the
permanent check for reservation growth.
