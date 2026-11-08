# F-258, Rendered exports, document workflows and planned GSPS read/apply in the showcase

**Status**: draft
**Epic ref**: E38.53
**Sprint**: S50
**Estimate**: 5w

Scheduling and one-viewer scope were authorized by the operator on 2026-11-08.
The estimate is a provisional lead engineering estimate, not measured capacity
or a calendar promise. This draft is intake for the normal pre-implementation
design round, not authorization to skip its API and numerical review.

## Normative source, transcribed

`docs/hld/05-rendering.md`, section7:

```text
**Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.
```

`docs/hld/23-performance-rules.md`, section26:

> No allocation in the render loop.
> One queue.submit() per frame.
> Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.
> Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.
> Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.

`docs/hld/B-parity-surface.md`, AppendixB, relevant capability families:

> | Viewport types | 12 | STACK, ORTHOGRAPHIC, PERSPECTIVE, VOLUME_3D, PLANAR, VIDEO, WHOLE_SLIDE, ECG and \_NEXT variants |
> | Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
> | Blend modes | 5 | Composite, MIP, MinIP, average, labelmap edge projection |
> | Segmentation representations | 3 | Labelmap, contour, surface |
> | Adapters | 4 | SR TID 1500, SEG, RTSTRUCT, parametric map |

D-11 pins actual parity to cornerstone3D 5.8.2 rather than the nonexistent
version in the imported appendix. D-07 and F-X005 permit honest CPU volume
unavailability. User-facing delivery is in the single showcase viewer.

`docs/hld/13-core-types.md`, section16, exact coordinate contract:

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

`docs/hld/15-lut-chain.md`, section18.2, exact existing VOI formulas,
and section18.3, its four required reference rows. Consumers reuse the one
LutChain and never recompute these in JavaScript.

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

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

## What the specification does not cover

The HLD specifies the library boundary and capability families, not the
complete control placement, browser fixture inventory or final module split.
This draft selects the scheduled scope below. Detailed signatures, new command
codes and any new numerical rule must be transcribed from the applicable HLD
and DICOM/ICC standards before implementation, never inferred from a demo.
No tolerance or new deviation is approved by this planning change.

## Problem and reproducer

At source tree `4a3c0132b8040f5393b038366b7202404d5a2bd9`, open the
folder viewer and attempt this story's first acceptance action below.
The existing library capability is absent from, incomplete in, or split across
the user-facing applications. The More audit identifies the exact gap.
F-205 export remainder. Existing F-215 folder marks and document writers are reused, not duplicated.

## Scope, API and compatibility

Concrete existing providers: `existing TID1500/TID2010/SecondaryCapture APIs and annotation state`.
Reuse them and the current source, viewport, annotation and cache identities.
Expose the actual result in the same canonical viewer that opens local data
and DICOMweb sources. A separate demo, a text-only panel, a formula re-created
in JavaScript or a screen control disconnected from the provider is refused
as an implementation of this story. Existing S46 workflows remain reachable.
The final module and public signature selection is part of design completion,
not an invented signature asserted to exist today.

## Consumer and provider map

| Consumer / entry point | Provider | Policy and visible output |
|------------------------|----------|---------------------------|
| Canonical viewer action | existing TID1500/TID2010/SecondaryCapture APIs and annotation state | Selected source and viewport produce the actual result |
| Existing local folder and browser worker modes | Current source/cache/render owners | Same action vocabulary, explicit capability limits |
| More matrix and permanent workflow test | Actual reachable feature inventory | Qualified status and reproducible evidence |

## State transitions and invariants

| Start and event | Committed result | Refusal and preserved state |
|-----------------|------------------|-----------------------------|
| Valid source, action selected | Publish only the current source/viewport generation | Keep prior usable view on refusal |
| Source, frame or mode changes while work is pending | Cancel or discard stale result before any visible commit | No old result or document leaks into the new source |
| Repeated action, release, retry | Same declared semantics with owned resources released once | No duplicate download, handlers or stale actor |

## Ownership, commit and recovery

The current viewer owns DOM, selected tool and source identity. The core owns
pixel arithmetic, geometry, decoder output and the render surface. Buffers are
prepared outside the render loop. Pending work carries source and viewport
selection generations, not a reused address. Successful validation and draw
or completed document construction is the visible commit point. A refusal
before it preserves the last usable image, configuration and selection.
Tests must inject failure after staging and before publication and then retry.

## Numbered acceptance inventory

| ID | Trigger and expected behaviour | Permanent observation | Wrong implementation rejected | Stage |
|----|--------------------------------|-----------------------|-------------------------------|-------|
| AC1 | Reach existing SR/KOS save/load and a new rendered Secondary Capture export through the same selected-image workflow | Named real browser workflow `f-258-primary` in the canonical viewer | Text, stub or detached demo counted as usable | integrated browser |
| AC2 | Open each emitted synthetic document independently and check references, frame number, rendered pixels and declared provenance | Independent hand/spec or pinned oracle fixture through the actual consumer | Shared wrong formula or a label-only control | implementation and integrated |
| AC3 | Cancellation or writer refusal downloads nothing, and source changes during save cannot publish an old image as a new one | Negative and recovery browser case `f-258-refusal-retry` | Partial commit, silent substitution or stale result | integrated browser |
| AC4 | Repeat the action, change viewport, source and frame while pending, release and reopen | Lifecycle case `f-258-ownership` with actual surface/document identity | A settled cache or late reply bypasses validation | integrated browser |
| AC5 | Existing S46 mono, colour, ICC, links, marks, presets and conformance still work | Current permanent suites on the final stacked source | New control breaks another source or viewport kind | integration |
| AC6 | A, B and C each produce the declared result or precise unavailable reason | Actual capable tier observation and declared forced-refusal control | Missing tier branch or counterfeit CPU fallback | external where hardware required |
| AC7 | More status and comparison row name the usable workflow, limits and evidence | Single-viewer feature inventory and source-pinned matrix | Library-only status passed off as viewer availability | integration |
| AC8 | Changed hot path retains the required release performance evidence | Existing named benchmark subjects before/after and raw observations | Debug timing, unrun measurement or changed tolerance claimed green | external if hot path changed |

| AC9 | Read and apply the approved synthetic GSPS subset to the referenced monochrome image/frame in the same viewer | Independently derived displayed area, VOI/presentation and annotation observations plus wrong-reference controls | Treating the image PresentationTransform as if it had read a GSPS document, or applying another instance's state | implementation and integrated browser |

A case-insensitive source audit found no GSPS document reader or apply API at
the input tree. This is new planned read/apply work, not an existing feature.
The detailed design must enumerate the PS3.3 GSPS modules and supported
attributes, their precedence, references and refusal policy before coding.
An unsupported module is named as not applied and cannot silently change the
image through a partially applied state. Full coded-concept GSPS write remains
F-139 in S60. Existing SR/KOS/SC and F-264 SEG/RTSTRUCT workflows share the
same document/export entry point. No existing serializer is invented.

## Approach and dependencies

Reuse `existing TID1500/TID2010/SecondaryCapture APIs and annotation state`. First write the independent synthetic fixture and
browser action/refusal controls, then connect the smallest real consumer,
then challenge a lifecycle combination the initial test did not cover.
Dependencies: F-245, F-215, F-196, F-205. Same-sprint dependencies must be done before the consumer starts, as
`/start-feature` requires. A source review or partial checkpoint does not
satisfy that precondition. No shared page, renderer,
provider or ledger file has concurrent writers. Browser and GPU observations
use one allocated resource slot. Root coordinates complete source unions.

## Verification and evidence reuse

Use focused unit and named browser selectors while implementing. Run the
required feature profile only on the final staged integration tree, then
record exact source, binary, selected tests, direct exits and observations.
A final plan, metadata, status or provider change invalidates prior evidence.
Use `viewer.open`, `viewer.first_image`, `viewer.window_step`,
`viewer.scroll_rate` and `viewer.resident_memory` when their paths change.
Add an instrument before claiming a new performance axis. A missing host or
benchmark is open evidence, not acceptance. The sprint profile runs once at
sprint close under the current workflow.

## Boundary and tier

- wasm-bindgen: only `ocelli-wasm` if this slice needs an export.
- Pixels across the wasm boundary: no. The worker owns its real surface.
- Render-loop allocation: none. Reuse pre-sized provider and viewport buffers.
- unsafe: none outside the existing two allowed files, and none planned here.
- Tier A (WebGPU): Use the existing provider on this tier.
- Tier B (WebGL2): Use the existing provider on this tier.
- Tier C (CPU): Use the existing CPU provider or report its precise unsupported mode. Attribute-only, persistence and document operations are rendering-tier independent.

## Open questions for implementation design

Complete the concrete module/API and numerical transcription against the
applicable standards before code begins. Establish the named fixture and
actual consumer selector, and resolve any provider gap found by the inventory
without silently expanding another story's completion claim. These are draft
design tasks within this assigned sprint, not an unscheduled backlog escape.
The scope, one-viewer requirement and sprint allocation are already decided.
