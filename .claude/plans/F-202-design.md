# F-202, Worker decode and owned frame render in the single showcase viewer

**Status**: draft
**Epic ref**: E38.12
**Sprint**: S47
**Estimate**: 6w

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

`docs/hld/14-the-boundary-in-code.md`, section17, exact control, bulk
and 48-byte event layout. The current safe wrappers remain authoritative for
implementation, and quoted unsafe is not permission for another unsafe site.

```rust
// wire format, little-endian
// [u16 kind][u16 viewport_id][u32 payload_len][payload ...] repeated
// ocelli-wasm/src/lib.rs
#[wasm_bindgen]
impl Session {
    /// Apply a packed command buffer. Returns the number of commands applied.
    #[wasm_bindgen(js_name = applyCommands)]
    pub fn apply_commands(&mut self, ptr: *const u8, len: usize) -> u32 {
        let buf = unsafe { core::slice::from_raw_parts(ptr, len) };
        self.inner.apply(buf)
    }
}
```

```rust
#[wasm_bindgen]
impl Session {
    /// Reserve `len` bytes and return a pointer into linear memory.
    pub fn alloc(&mut self, len: usize) -> *mut u8 { /* ... */ }
    /// Hand ownership back; `ptr` must be the value returned by `alloc`.
    pub fn commit_frame(&mut self, ptr: *mut u8, len: usize, meta: &FrameMeta) { /* ... */ }
}
// packages/core/src/bulk.ts -- the ONLY correct order
const ptr = session.alloc(bytes.byteLength);
// Build the view AFTER the allocation. Use it immediately. Let it go.
new Uint8Array(wasm.memory.buffer, ptr, bytes.byteLength).set(bytes);
session.commit_frame(ptr, bytes.byteLength, meta);
```

```rust
// ocelli-wasm/src/ring.rs -- single producer (Rust), single consumer (JS)
#[repr(C)]
pub struct RingHeader {
    pub head: u32, // producer writes
    pub tail: u32, // consumer writes
    pub cap: u32, // power of two
    pub dropped: u32, // overflow count; nonzero means JS is not draining
}
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

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
Worker decode, frame cache and render remainder of F-198. Browser acceptance must run in the single showcase, not the sample-only React proof.

## Scope, API and compatibility

Concrete existing providers: `packages/core/src/instance-bytes.ts, pool.ts and the ocelli-wasm session`.
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
| Canonical viewer action | packages/core/src/instance-bytes.ts, pool.ts and the ocelli-wasm session | Selected source and viewport produce the actual result |
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
| AC1 | Use the completed F-245 source chooser to select and fetch a real synthetic WADO-RS envelope, then display its decoded selected frame through the owned worker surface in that same canonical viewer | Named real browser workflow `f-202-primary`, actual QIDO/WADO requests and independently expected literal canvas pixels | Text, a fetched envelope alone, a stale frame or a detached demo counted as a usable remote viewer | integrated browser |
| AC2 | Change stack selection while decode is pending, then release and reopen the series without publishing a stale frame | Independent hand/spec or pinned oracle fixture through the actual consumer | Shared wrong formula or a label-only control | implementation and integrated |
| AC3 | Warm redraw reuses the correct content id, description and plane, and eviction or source mutation forces a fresh decode | Negative and recovery browser case `f-202-refusal-retry` | Partial commit, silent substitution or stale result | integrated browser |
| AC4 | Repeat the action, change viewport, source and frame while pending, release and reopen | Lifecycle case `f-202-ownership` with actual surface/document identity | A settled cache or late reply bypasses validation | integrated browser |
| AC5 | Existing S46 mono, colour, ICC, links, marks, presets and conformance still work | Current permanent suites on the final stacked source | New control breaks another source or viewport kind | integration |
| AC6 | A, B and C each produce the declared result or precise unavailable reason | Actual capable tier observation and declared forced-refusal control | Missing tier branch or counterfeit CPU fallback | external where hardware required |
| AC7 | More status and comparison row name the usable workflow, limits and evidence | Single-viewer feature inventory and source-pinned matrix | Library-only status passed off as viewer availability | integration |
| AC8 | Changed hot path retains the required release performance evidence | Existing named benchmark subjects before/after and raw observations | Debug timing, unrun measurement or changed tolerance claimed green | external if hot path changed |

## S47 provider and consumer sequencing

F-245 must be done before this story starts. Its completed foundation supplies
the canonical shell and real QIDO/WADO source selection and byte fetch, while
preserving the working native image path. Its remote browser mode remains
explicitly partial until this story connects the actual decode and owned
render surface. AC1 closes that remaining end-to-end gap. A reviewed partial
checkpoint does not replace dependency completion, and no second viewer is
introduced to satisfy either acceptance inventory.

## Approach and dependencies

Reuse `packages/core/src/instance-bytes.ts, pool.ts and the ocelli-wasm session`. First write the independent synthetic fixture and
browser action/refusal controls, then connect the smallest real consumer,
then challenge a lifecycle combination the initial test did not cover.
Dependencies: F-245, F-198, F-231, F-236. Same-sprint dependencies must be done before the consumer starts, as
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

## Operator design round, 2026-11-11

This plan stays draft until F-245 is integrated on the sprint branch. It is then designed against the landed one viewer rather than F-245's plan. The story may carry to S48 if F-245 lands late.
