# F-249, Searchable recursive metadata browser in the showcase

**Status**: draft
**Epic ref**: E38.44
**Sprint**: S48
**Estimate**: 4w

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
F-205 metadata browser remainder, extending existing usable Headers rather than replacing it with a second viewer.

## Scope, API and compatibility

Concrete existing providers: `ocelli-dicom/src/metadata.rs and existing Headers transport`.
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
| Canonical viewer action | ocelli-dicom/src/metadata.rs and existing Headers transport | Selected source and viewport produce the actual result |
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
| AC1 | Browse every synthetic element including nested sequences, VR, multiplicity, keyword and safely rendered values | Named real browser workflow `f-249-primary` in the canonical viewer | Text, stub or detached demo counted as usable | integrated browser |
| AC2 | Search by tag, keyword and text and navigate to the exact matching sequence item with no generated stand-in rows | Independent hand/spec or pinned oracle fixture through the actual consumer | Shared wrong formula or a label-only control | implementation and integrated |
| AC3 | Opening another instance or source invalidates the previous tree, and unsupported values are named rather than dropped | Negative and recovery browser case `f-249-refusal-retry` | Partial commit, silent substitution or stale result | integrated browser |
| AC4 | Repeat the action, change viewport, source and frame while pending, release and reopen | Lifecycle case `f-249-ownership` with actual surface/document identity | A settled cache or late reply bypasses validation | integrated browser |
| AC5 | Existing S46 mono, colour, ICC, links, marks, presets and conformance still work | Current permanent suites on the final stacked source | New control breaks another source or viewport kind | integration |
| AC6 | A, B and C each produce the declared result or precise unavailable reason | Actual capable tier observation and declared forced-refusal control | Missing tier branch or counterfeit CPU fallback | external where hardware required |
| AC7 | More status and comparison row name the usable workflow, limits and evidence | Single-viewer feature inventory and source-pinned matrix | Library-only status passed off as viewer availability | integration |
| AC8 | Changed hot path retains the required release performance evidence | Existing named benchmark subjects before/after and raw observations | Debug timing, unrun measurement or changed tolerance claimed green | external if hot path changed |

## Approach and dependencies

Reuse `ocelli-dicom/src/metadata.rs and existing Headers transport`. First write the independent synthetic fixture and
browser action/refusal controls, then connect the smallest real consumer,
then challenge a lifecycle combination the initial test did not cover.
Dependencies: F-202, F-245, F-225. Same-sprint dependencies must be done before the consumer starts, as
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

## Appended draft refinement, observable controls and lead questions

This draft refinement is retained for the normal pre-implementation design round after independent non-author review. Status stays draft. It maps observable controls to the existing numbered acceptance inventory above and lists remaining lead questions. No existing AC row is rewritten or weakened, and no new policy, rounding, tolerance, equality, refusal, API, dependency, estimate, sprint or status choice is made here.

**Observable controls mapped to existing ACs**

- **AC1.** The same canonical viewer browses the synthetic fixture, including nested sequences, VR, multiplicity, keyword and safely rendered values. The existing Headers transport already traverses SQ into 0-based prefixed flat rows and searches tag, VR, name, value and text content. The current depth limit of 6, the 4096 element-row cap plus its omission marker and the separate 1024-character value cap are observed facts, and the fixture must not have deep or broad elements silently dropped by them.
- **Typed slots.** Typed MetadataSet presentation preserves absent, present-empty and null slots, value multiplicity, ordered sequence items and carriers, with file meta kept separate.
- **AC2.** Search navigates to the exact matching item path, including repeated tags in different items, and shows VR, VM and typed values. DOM metacharacters render safely. Cap and unsupported disclosure is visible rather than silent. No generated stand-in rows, no dictionary meaning invented for unknown private tags and no automatic BulkData URI fetch.
- **AC3.** Opening another instance or source invalidates the previous tree. A-B-A, close, reopen and frame, source and viewport replacement never show A as B. A retained prior tree or image carries honest identity and status. MetadataError unsupported pixel projection can abort an entire set today, so per-row unsupported with the rest of the tree is an open provider gap and design question, not claimed as existing behaviour.
- **AC4.** Repeat, pending source and viewport change, release and reopen use the generation-carrying lifecycle.
- **AC5.** Existing S46 workflows remain reachable on the final stacked source.
- **AC6.** Metadata browsing is renderer-tier independent. Tiers A, B and C each observe the declared result, and Tier C refusal is never required merely because GPU stack drawing is unavailable.
- **AC7.** The More row names the usable workflow, its caps and its evidence.
- **Privacy.** Actual private user values are allowed in the intended local UI only. Evidence uses synthetic fixtures. No real values in logs, prompts, errors or telemetry and no conflation with conformance claims.

**Remaining lead questions, explicitly unresolved**

- Tree and path navigation UX, search scope, pagination, the file meta group and a VM column.
- The per-row unsupported disclosure design for the MetadataError abort case.
- Exact retained-prior-tree presentation and identity labelling.
