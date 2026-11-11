# F-203, More feature audit, pass 1

Reviewer: Codex. Date: 2026-11-08. Read-only audit at tree
`4a3c0132b8040f5393b038366b7202404d5a2bd9`.
This is a planning and status audit of all 17 More entries. It does not approve
an implementation, assign a sprint, or declare any pending story complete.
No product source, canonical delivery ledger, model-owned benchmark file or
index was changed. No browser, GPU, viewer or timing run was started.

## What the marks mean today

The More introduction explicitly defines Implemented as library code with
tests, rather than an image-page control. Its sections often accurately say
that the folder page or the React example cannot use that library code yet.
The index nevertheless compresses three different questions into one word:
library availability, image-page integration and availability on a GPU-less
host. A reader looking for a working viewer can reasonably read that word as
a promise of a control.

One direct contradiction needs correction now: Preferences is marked Not yet,
which this page defines as no code, while its own paragraph and page.html
implement four persisted choices. That entry is Partial, with precisely those
four settings available. MPR's Implemented mark is accurate under the stated
library definition, but does not mean MPR can be opened in this folder viewer.
The JPEG XL paragraph's reference to the preview drawing monochrome frames
is an outdated description of the supported scope after F-239. The decoder
also reports RGB output and the page now consumes three-channel previews.

More should distinguish usable on Images, library only, and unavailable on
this tier. It should name the actual viewer being discussed when contrasting
the folder page and `npm run dev` React example. Merely relabelling all
library-only features Not yet would lose true information. The remaining UI
work below should be planned under the existing F-IDs, rather than hidden by
changing labels.

## All 17 entries

| Feature | Actual library and folder-viewer state | Tier limits and work remaining | Existing delivery home |
|---|---|---|---|
| Overlay planes and shutters | `ocelli-dicom/src/overlay.rs:329` reads planes, bit locations, shutter shapes and presentation value. Images has no reader consumer, overlay control or shutter render route. Its HTML `.overlay` elements are text corners, not DICOM planes. | Compose the selected frame's overlay coordinates and shutter mask with the actual image transform, expose visibility and refuse unsupported shapes honestly. This can use CPU arithmetic, with matching pixel and geometry fixtures. | F-193 reader is done. F-204 explicitly owns a shutter beyond the example panel. Include plane drawing in the follow-up scope rather than treating the reader as viewer completion. |
| Interpolation | `ocelli-render/src/interpolation.rs` has nearest and linear stack sampling on A/B/C. The folder canvas is styled `image-rendering: pixelated` and has no interpolation selector. Volume trilinear reslicing exists separately. | Add a real in-plane mode control and propagate it to the consumer. Distinguish display interpolation from a new between-stack-slices blending mode. Orthographic presentation deliberately refuses a second linear interpolant. | F-194 covers the library. No explicit pending row was found for the folder selector or the proposed between-slice stack blend. Name these in an approved F-203/F-204 amendment or file a follow-up before scheduling. |
| Reference line and cursor | `ocelli-viewport::link::reference_segment` computes plane intersections and the folder Link route draws reference lines. Core crosshair/reference-cursor helpers consume an already projected point. The folder's crosshair/readout is local to a viewport, not a patient-point cursor across viewports. | Keep current reference lines. Add a shared patient-point cursor with frame-of-reference checks and proper per-plane projection. It must not move an unrelated frame of reference. | F-088/F-199 library and demo helpers, F-209 folder lines, F-203 remaining frame-of-reference cursor UI. |
| Several series | Folder layouts 1 by 1 to 3 by 3, active cell, series placement, linked scroll by patient position, same-kind window and zoom links are implemented. This includes colour isolation from F-239. Prior-study discovery and loading are absent. The React example still labels its own linked controls unavailable. | Reuse the existing folder controls. Add prior selection and loading through the established DICOMweb surface, with study and frame-of-reference identity retained. Do not promise automatic anatomical alignment across different frame UIDs. | F-208/F-209 are folder work. F-203 retains prior studies and the missing React integration. F-021 provides the DICOMweb foundation. |
| MPR | `ocelli-volume/src/ortho.rs` reslices orthographic and oblique planes. `ocelli-render/src/ortho.rs` presents A/B planes and has `present_cpu_plane` at line108. Folder Images has no volume assembly or MPR viewport. | Connect real series assembly to volume admission, orientation selection, reslice, camera and frame lifecycle. Show axial/coronal/sagittal and oblique planes from actual data, handle incomplete or refused geometry visibly, and test patient-position and measurement coordinates. CPU MPR is already a library option. | F-051/F-052/F-053/F-X004 are library work. F-203 is the existing UI home. |
| Slab | `ocelli-volume/src/slab.rs` and render blend modes exist. No folder slab viewport, thickness or reduction control exists. | Wire thickness and MIP/MinIP/average to the real reducer. A/B support is library work already done. Tier C has no CPU reducer and must report unavailable unless a separately designed implementation is approved. | F-054 done at library level. F-203 UI remains. |
| Volume rendering | RenderGraph volume ray, opacity, shade and clip APIs exist. No folder volume viewport or controls exist. | Add a capable rendering path and real transfer-function, camera, shading and clipping controls. The current local-process RGB canvas preview is a stack consumer, not a volume renderer. Tier C's `volume_ray_admission` refuses before sampling. That is an approved limit, not an unimplemented mandatory CPU ray. | F-057/F-059/F-060/F-061/F-062 done at library level, F-X005 records honest CPU unavailability. F-203 UI remains. |
| Fusion | Volume-plane sampling and render fusion exist, including frame UID checks, per-volume opacity and colormap. Folder Images has no fusion source pairing or render path. | Select two volumes, retain patient-space registration rules, show opacity/colormap controls and refuse frame mismatch. A/B require the declared float sample/target capability. Tier C deliberately refuses. Registration across different frames is separate scope. | F-056 library, F-203 UI. F-179/F-180 are later registration work and must not be silently folded into this UI task. |
| Histogram | `ocelli-geom/src/stats.rs:245` fills caller-owned bins. Folder has region statistics, but no histogram view or control. | Draw a real histogram over a named image or ROI and value domain, with tested bin edges, padding policy and labels. Plain CPU counting is possible. HLD GPU histogram compute is a separate tier-A path and is not needed merely to expose the existing CPU function. | F-075 library, F-204 UI. F-128 is later GPU statistics, not a prerequisite for the CPU chart. |
| Automatic window | `AutoWindowPool`, `series_sample` and `auto_window` are present. Folder code pools series frames and labels Auto window, with per-image fallback. A file's declared VOI remains authoritative. | This is already a usable folder feature. Remaining sprint evidence and story closure are distinct from implementation. The GPU library consumer can use the transform, but no GPU consumer calls the pool yet. Colour has its separate initial colour window. | F-227 in progress in the frozen backlog. Do not relabel its actual folder behaviour absent merely because sprint acceptance is open. |
| Metadata browser | `MetadataSet` and typed recursive values exist. Folder Headers shows a tag/value table. No full searchable recursive metadata browser, comparison or copy workflow is present. React explicitly labels its browser unavailable. | Keep Headers usable, then add recursive sequence navigation, search, selection/copy and a two-instance comparison with clear value handling. A flat diagnostic table is a real partial implementation, not the whole pending browser. Rendering tiers do not apply. | F-205 owns compare/copy browser UI. F-225 findings are a separate report, not a replacement for this browser. |
| Conformance report | The reader and `ocelli-dicom/src/conformance.rs:489` exist. Folder shows Headers count and Checks findings through the local process, including rendered RGB and refused files. | Usable on the folder page. Keep the explicit checked-module limits and no-value finding policy. Browser-only generation still needs a parser link in wasm or another declared transport. No pixel-render tier distinction is relevant. | F-225 in progress at the audit tree. Browser decode groundwork is F-202, but not authority to widen F-225's checked IOD scope. |
| One to one | Explicit zoom factors exist, but no folder one-stored-pixel-to-one-device-pixel control or computation exists. Fit, zoom and magnification are different behaviours. | Add the actual physical-device-pixel mapping, respecting DPR, pixel aspect, rotation and current transform. Verify a known checkerboard at different DPRs. | F-204. |
| Preferences | Folder `readPreference`/`writePreference` catch localStorage failures and persist mouse profile, cine rate and the two side-column visibility settings. There is no general preferences module, preset store, keymap editor or saved hanging layout. | Correct Not yet to Partial now. Extend a validated preference schema, defaults, migration and failure behaviour under the later approved scope. Do not confuse viewport fault-recovery snapshots with saved preferences. | F-214 covers current folder preferences. F-205 covers example preferences, while F-185 is a broader later workstation feature. |
| JPEG XL and transfer syntaxes | `.110` and `.112` have real decoders, including RGB output. `.111` is known and unavailable under D-28 because reconstruction reaches the pinned dependency's panic. The registry has no per-frame deflate decoder because deflate is a whole-data-set operation. The parser explicitly enables it under D-18. | Update the stale monochrome scope sentence. Do not call parser-owned deflate a missing pixel decoder. A safe `.111` implementation needs a separately reviewed dependency/fix and synthetic reconstruction regressions before enabling it. Current unsupported shapes must continue to refuse by name. | F-229 done with its documented exception, F-239 colour, F-243 JPEG-LS, F-244 frame boundaries. No separately allocated `.111` repair row was found. Plan that follow-up explicitly rather than pretending F-229 enables it today. |
| Worker decode and wasm | Core has bounded fetch/transfer, bulk ticket, worker pool and session frame-cache APIs. The React worker publishes a stored sample as a path proof. It does not decode the posted WADO envelope into a renderable frame. Folder decoding happens in its local native process. | Complete WADO envelope parse/decode, owned frame identity, transfer/admission and actual frame render in the worker, then exercise selection epochs, eviction and release. Preserve worker/wasm CSP proofs. This is a real library integration gap, not fixed by the folder preview server. | F-198 groundwork, F-202 pending and unscheduled. F-224/F-226/F-231/F-236 are relevant existing identity/release/cache contracts. |
| Documents | Core implements TID1500 and TID1410 groups, annotation persistence/undo, TID2010 key images and Secondary Capture read/write. Folder Save marks writes and loads SR/KOS through these APIs. It does not expose every document/export API as an image-page workflow. | Keep existing marks save/load usable and folder-scoped. Complete further export actions such as a real rendered Secondary Capture, with provenance, selected image identity and reload/interop controls. Tier labels do not apply to serializers. | F-097/F-196/F-201 library/demo and F-215 folder SR/KOS. F-205 owns exports beyond the existing document panel. |

## Cornerstone3D 5.8.2 comparison

Installed `@cornerstonejs/core`, `tools` and `dicom-image-loader` each report
5.8.2 under `tools/oracle/node_modules`. The audit read those permissive
installed sources, not another viewer's code. It inspected the exported
Stack/Volume/3D viewport and generic planar paths, interpolation and display
area controls, synchronizers, Crosshairs/ReferenceLines/ReferenceCursors tools,
and the DICOM worker decoder dispatch. Source hashes are recorded in
`/private/tmp/ocelli-s46-codex/more-feature-audit/source-identity.json`.

The pinned library supplies MPR/orientation, slab blend, 3D volume and fusion
APIs. Those are meaningful integration gaps for Ocelli's image page even
though corresponding Ocelli Rust APIs already exist. It supplies interpolation,
reference/crosshair tools, and `Viewport.setDisplayAreaScale`, whose scale1
uses canvas height and image spacing and selects nearest sampling. Merely
having Ocelli zoom factors therefore does not establish one-to-one parity.
The DICOM image loader executes a real decodeTask through its worker manager,
so F-202 is a concrete gap against its browser loading path.

Do not generalize the legacy `BaseVolumeViewport` CPU rejection into a claim
that pinned Cornerstone has no CPU MPR. The same installed 5.8.2 package has
`GenericViewport/Planar/CpuVolumeSliceRenderPath.js`, a CPU volume sampler and
a default resolver selecting that path. Its legacy restriction and generic
capability must be distinguished. No claim of complete CPU slab or fusion
parity is made by this source-only audit.

Cornerstone is a library, not a bundled radiology workstation. Prior-study
search, persistent preferences, a full metadata browser and export workflows
need host application work on either side. No parity defect is inferred from
the absence of a standalone Cornerstone UI for those rows. Its metadata
providers do not constitute a DICOM conformance report. Its worker switch has
no JPEG XL UID `.110`, `.111` or `.112`, so Ocelli's two supported JPEG XL
syntaxes are additional capability. The per-image default VOI calculation
also differs from Ocelli's labelled percentile pool. These differences do not
establish that Ocelli is generally more capable or faster.

Official documentation corroborates the broad API categories:
[Cornerstone overview](https://www.cornerstonejs.org/docs/getting-started/overview/),
[viewports](https://www.cornerstonejs.org/docs/concepts/cornerstone-core/viewports/)
and [Crosshairs](https://www.cornerstonejs.org/docs/api/tools/classes/crosshairstool/).
The installed 5.8.2 sources, rather than evolving documentation, bind the
version-specific observations above. This is capability source inspection,
not runtime or pixel parity proof. F-223 remains the controlled comparative
measurement, and each newly exposed UI feature needs its own behaviour and
image checks before a parity claim.

## Proposed order for an approved implementation plan

1. Correct visible status now, especially Preferences and library-only versus
   image-page wording. Keep existing usable folder controls named accurately.
   This does not schedule the feature implementations.
2. Give F-202 its own reviewed implementation plan, completing the worker
   decode/render and frame-ownership path rather than reusing the synthetic
   sample proof as if it were an image viewer. This unlocks browser-side
   consumers of the real rendering library.
3. Expand F-203 into reviewable slices: patient-space cursor and priors,
   then orthographic MPR, then oblique controls, then slab, volume and fusion
   on capable tiers. Reuse folder linkage already implemented under F-209.
   Each slice must expose real data and a specific unavailable reason where
   capabilities are absent. Library existence alone cannot close it.
4. Plan F-204's one-to-one, histogram and overlay/shutter integration as
   independent usable controls. Assign the otherwise untracked folder
   interpolation selector and any between-slice stack blend explicitly before
   implementation. Existing APIs and a future feature are not the same row.
5. Plan F-205's metadata compare/copy, expanded document exports and general
   preferences. Reuse SR/KOS and the four saved settings instead of rebuilding
   or relabelling them absent.
6. Treat `.111` safe reconstruction and broader unsupported codec shapes as
   separately scoped codec work. Retain documented refusals until reviewed
   implementations and their corpus/interop evidence exist.

This is an ordering proposal, not a sprint allocation or an estimate. F-202
to F-205 are pending Phase1 rows with no sprint in the frozen backlog. Their
current 4-week entries cover broad bundles and do not establish a credible
completion date for all the work above. No new F-ID or second backlog has
been created. The next authorized planning step is to amend those existing
plans/rows with explicit slice acceptance and estimates before allocation.

## Evidence limits

The audit used immutable `git show`/`git grep` source, direct installed-file
reads and package versions. Source identity is recorded separately. It read
More, folder source, React unavailable controls, HLD rendering and D-07/D-18/
D-28, and existing backlog mappings. Several initial path-discovery queries
returned no file or no match, then the actual viewer-react paths were read.
No failed read was presented as a passing check. No feature tests were run
because this task changed no implementation and source capability is not a
substitute for runtime proof.

## Subsequent operator instruction and scheduled disposition

After this source audit, the operator required one viewer for all user-facing
capabilities and scheduled completion in the next few sprints. The authorized
M21 allocation now assigns F-202 to F-205 and F-245 to F-264 to S47 through
S50, with draft canonical designs and explicit browser acceptance. This
supersedes the earlier unscheduled planning recommendation, not the recorded
source observations at 4a3c013. Later scheduled work moves four sprints and
must include its real viewer workflow. Full GSPS write remains F-139 at S60,
while newly planned GSPS read/apply is explicitly F-258 in S50. The absence of
a GSPS reader at the audited source is not silently called existing support.
