# Changelog

The section headed by a release tag is that release's notes. `/release-notes`
renders it and the published GitHub release body is compared byte for byte
against a fresh render. See `docs/RELEASE.md`.

## Unreleased

Repository bootstrap. Nothing is published.

### Added

- `ocelli-seg` writes a caller segment on slices strictly between slices that already hold it. A nearer slice takes that end's mask. The exact halfway slice takes the union. An endpoint stays. Fewer than two keys writes nothing. An empty stack, a zero side, a bad length or mismatched slices write nothing. The presenter is unchanged.
- `ocelli-seg` paints a caller segment on the samples inside a closed index polygon, including the boundary. The ring closes from the last point to the first. A non-finite vertex or a non-finite intersection writes nothing. The contour representation is unchanged.
- `ocelli-seg` paints a caller segment where a caller value is inside a start and an end, and only inside a rectangle or a circle. The end is included. A value outside the shape is ignored, including a non-finite one. A non-finite value inside the shape writes nothing. The presenter is unchanged.
- `ocelli-seg` flood-fills the 4-connected region of a seed value and paint-fills every sample with that value. A diagonal is not connected. Segment 0 is a legal fill. An outside seed writes nothing. The presenter is unchanged.
- `ocelli-seg` erases to segment 0 inside a rectangle, a circle or a sphere. The circle and the sphere use the existing stamp. A rectangle includes its bounds on one slice. An inverted span or a non-finite value writes nothing. The presenter is unchanged.
- `ocelli-seg` reads one segment number, stores a trimmed label for a segment above 0, and reports the longest horizontal and vertical runs. An equal length keeps the earlier run. Segment 0 is not a label and not a run. A short buffer is not background. The brush and the presenter are unchanged.
- `ocelli-seg` stamps a circle or a sphere of segment numbers into a caller label grid. A sample on the radius is inside. A circle paints one slice. A sphere uses every slice. A negative radius or a non-finite centre writes nothing. Segment 0 erases. The labelmap presenter is unchanged.
- `ocelli-geom` reports an image-plane ultrasound direction and the Euclidean length of the same span. `z` is in the length and not in the angle. A rectangular redaction writes a caller fill into the intersection of a row-major buffer. A missed rectangle sets applied false and leaves the buffer unchanged. A non-finite sample in the intersection writes nothing.
- The shell can build an arrow, a label, a key image and an ETDRS grid as display-list marks. The arrow head is a quarter of the shaft, capped at 12 pixels. The grid uses radii of 0.5, 1.5 and 3 mm. The drawing layer is unchanged. The marks are not an SR content tree.
- `ocelli-geom` finds a 4-connected livewire path on a caller-supplied cost grid and marks the pixels that path visits. The start pixel adds no cost. The mask does not fill an interior. An empty path is not a blank mask. Polygon area and the spline point stay the earlier functions.
- `ocelli-geom` summarises caller samples as a mean, a population standard deviation, a minimum, a maximum and the caller's area. An empty list is not a mean of 0. A histogram uses the caller's bins. The maximum stays in the last bin. SUV is a separate path and is unavailable when the units are not BQML or a required magnitude is missing. There is no default weight.
- `ocelli-geom` measures a polygon area and one point on a uniform Catmull-Rom spline. The area closes the ring. The spline passes through the knots. Freehand length remains the existing length. The functions do not convert spaces and do not compute region statistics.
- `ocelli-geom` measures a rectangle, an ellipse and a circle from caller points. A rectangle ignores `z`. A zero ellipse radius is not an area. A circle uses the Euclidean radius. The probe remains the existing volume sample. The functions do not convert spaces and do not compute region statistics.
- `ocelli-geom` measures a named length, a caller-space height, an acute Cobb angle and a bidirectional pair. Fewer than two points is not a length. The angle tool remains the existing unsigned angle. The functions do not convert spaces.
- An SVG drawing layer writes one group per caller mark and emits the annotation names from the difference between display lists. A hidden mark stays in the SVG. The list is not the SR content tree. The layer does not listen.
- A tool-state wrapper emits `tool.activated` and `tool.modeChanged` after the registry accepts a mode change. The subject comes first. One cursor follows the active tool that owns primary, or the first active tool when nobody does, for both the idle phase and the dragging phase. The wrapper does not listen.
- Pointer, touch and wheel input becomes one packed command buffer per frame. Pan, zoom and rotation come from a caller basis captured at pointer down, because those camera writes replace. A wheel sums its samples and then clamps once. The dispatcher does not call wasm.
- `ocelli-geom` measures a distance, a polyline length, an unsigned angle, a segment projection, a segment hit and a plane projection. The functions take `Pt` and do not convert spaces. A hit includes the tolerance. An exact zero segment or normal is degenerate. Length and the angle use workspace glam because `sqrt` and `atan2` are not in `core`.
- A caller-owned tool registry stores groups, modes and pointer bindings. Active is the only mode that owns a binding. Taking a binding moves the previous owner to passive and clears that owner's bindings. The names come back in registration order. A viewport id belongs to one group. The registry does not listen or pack a command.
- A segmentation contour can be cut by one patient Z plane. One emitted point stays a point. Two or more emitted points are an open contour when the input was not already on that plane. A contour already on the plane keeps its kind. The labelmap edge blend draws the outline and ignores fill. Slab, volume and stack report that blend unavailable. Tier C cuts the contour and does not upload a GPU image.
- A composite segmentation overlay remembers whether it shows a labelmap, a contour or a surface. A new overlay starts as a labelmap. Choosing a different representation drops the uploaded image and does not draw it again until the matching show. A show for a different representation uploads nothing. A stack, a slab or a volume refuses the choice.
- A composite segmentation overlay can copy a surface mesh. A pixel centre is covered when every barycentric weight is at least zero. Vertices are not rounded. A zero-area triangle is skipped. The copy shader is the existing labelmap shader. Tier C presents those bytes and does not upload a GPU image.
- A composite segmentation overlay can copy a contour stroke. The geometric types follow DICOM PS3.3 C.8.8.6. The stroke keeps the segment LUT alpha and does not fill. A point off the plane writes nothing. Tier C presents those bytes and does not upload a GPU image.
- A composite segmentation overlay copies presenter bytes. Index 0 stays transparent. Fill writes the segment RGBA8. Outline is the one-pixel 4-connected boundary and uses alpha 255. A missing colour or both flags off writes nothing. Labelmap edge was left for a later story. Tier C presents those bytes and does not upload a GPU image.
- `bin/ocelli.sh compare produce` writes an Ocelli candidate image for every current oracle view. `compare gate` scores that directory. Colour, palette and YBR stay unscored. A missing image is absent and is not a pass.
- Lossless JPEG-LS RGB with three components and sample interleave decodes to the stored samples. Line interleave, a non-zero NEAR and any other component count stay refused.
- Two volumes that share one `FrameOfReferenceUID` can be drawn on one patient plane. Volume B is in front. An omitted colormap uses the existing window range. A second UID draws nothing. Tier C reports fusion unavailable and writes no CPU image.
- Viewports that share one `FrameOfReferenceUID` keep one patient point. A second UID, including a trailing space, stays put. A missing UID moves nobody. The index is host arithmetic and is the same on every tier.
- A thick slab can be drawn as composite, MIP, MinIP or average. Composite is the centre sample. The other three reduce stored samples and then apply the existing window once. An absent sample leaves the previous pixel in place. Tier C reports the slab unavailable.
- An arbitrary patient plane can be taken from a constructed volume. The sample is the existing level-0 trilinear read. An absent slice stays absent. Display reuses the orthographic upload, and tier C reports that viewport unavailable.
- A volume ray reads a 256-entry opacity table. Flat is 0.5 at every display code, which keeps the previous one-step integral. Ramp entry i is i/255. The address is the display value after the existing window. A table of the wrong length, a non-finite entry, or an entry outside 0 to 1 is refused and the previous table stays. Changing the table does not upload bricks. Tier C reports the view unavailable.
- A volume ray can shade each sample from its stored neighbours. The default is unlit, so the previous one-step integral stays. Ambient 0 and diffuse 1 with the light along the gradient keeps that colour. The reversed gradient makes the colour 0 and leaves the opacity. A non-finite weight or a weight outside 0 to 1 is refused and the previous weights stay. Changing the weights does not upload bricks. Tier C reports the view unavailable.
- Tier C can present an axial, sagittal, coronal or oblique plane through the CPU raster. The samples are the existing reslice. An absent sample or a raster refusal writes nothing. Tier A and tier B keep the existing upload. There is no CPU volume image. SIMD timing is not measured.
- VOLUME_3D stays unavailable on tier C. Admission returns before a sample is read and writes no colour buffer. Tier A and tier B keep the existing fragment ray. There is no CPU volume image.
- A volume ray can clip samples to at most six patient planes. The kept side includes the plane. A discarded sample adds no colour and no alpha, and the brick stays bound. No planes leaves the previous integral. More than six planes, a non-finite component, or a zero-length normal is refused and the previous planes stay. Changing the planes does not upload bricks. Tier C reports the view unavailable.
- A VOLUME_3D view ray-casts resident bricks on tier A and tier B. The window is the existing VOI uniform. A ready set that does not fit beside the residency texture is not drawn as a subset. Tier C reports the view unavailable.
- An orthographic plane can be taken from a constructed volume. Axial, sagittal and coronal follow that volume's axes. `reslice` returns the stored samples. An absent slice stays absent. `show_ortho` uploads a complete plane through the existing stack window and leaves the last complete plane in place when a sample is missing. Tier C reports the viewport unavailable.
- A volume can be shown while slices are still arriving. `cut_ready_brick`
  copies only a brick whose slices have all landed. `sync_volume` uploads
  that brick once, and again when the bytes change. An absent slice is not
  a texture of zeros. `present_slice_f32` returns a landed slice and refuses
  one that has not.
- The volume keeps a multiscale level axis. Level 0 stays full resolution.
  `append_level` stores a caller-supplied coarser level on the same
  allocation, and a missing level is an error. This story does not build
  the pyramid.
- A React viewport binding. `createViewportPump` submits one packed command
  buffer on a frame that has queued commands, drains the ring once, copies
  state once and reports that copy only when a field changed. `OcelliViewport` takes a session prop and renders
  a host element. A missing session is the absent state. The binding does not
  draw.
- A named event catalogue for the six ring kinds and the shell events, with
  one row per cornerstone3D 5.8.2 event. `decodeNamedEvent` reads the copied
  payload. Scroll, camera and window stay state. Unavailable rows name the
  reason.
- A versioned JSON snapshot of one viewport's camera inputs, display area and
  presentation. `snapshot` and `restore_snapshot` round-trip that record.
  Scroll, flags and generation stay put, and the 96-byte readback is unchanged.
- The boundary's TypeScript declarations are generated from the Rust layout.
  `packages/core/src/generated.ts` is written by
  `crates/ocelli-viewport/tests/typegen.rs` and checked byte for byte, so a
  Rust command id, payload length, enum byte or state offset cannot drift
  from the shell.
- The Cargo workspace and the crates of HLD section 4's crate table, which
  `ls crates | wc -l` counts. Section 15.1's layout block is not that table and
  omits `ocelli-compute`. `wasm-bindgen` is confined to `ocelli-wasm` and
  enforced by `ci/check-bindgen-isolation.sh`.
- The npm workspaces `@ocelli/core` and `@ocelli/react`, and the example
  viewer at `examples/viewer-react`.
- Strongly typed canvas, world and voxel-index points, composable transforms,
  and pixel-value newtypes in `ocelli-core`.
- A manifest-backed DICOM corpus with deterministic synthetic fixtures,
  transfer-syntax conformance checks, metadata auditing and digest verification.
- A DICOM Part 10 parser that dispatches from the declared Transfer Syntax UID,
  retains the complete dicom-rs object and selected route, and refuses malformed
  or truncated input without an alternate-syntax fallback.
- A lossless DICOM metadata model and ordered caller-owned provider registry
  that preserve declared VR, empty versus absent values, multiplicity, signed
  widths, nested sequences, VR-aware text semantics, validated binary carriers,
  and stable provider identity.
- A bounded DICOMweb client for QIDO-RS, WADO-RS and WADO-URI, with caller-owned
  authentication, cancellation, strict response validation, lossless DICOM JSON
  metadata projection and encoded frame-part ranges.
- An explicit runtime codec registry with exact Transfer Syntax UID capability
  states, validated DICOM frame descriptions, atomic collision-refusing
  registration, and caller-owned decode output buffers.
- Validated DICOM image-plane and stored-pixel evidence, with one modality and
  VOI pipeline covering LUT precedence and LINEAR, LINEAR_EXACT, and SIGMOID.
- The presentation stage of the LUT chain, completing DICOM PS3.3 C.11's first
  three stages in one composed pipeline. Inversion is resolved exactly once from
  Photometric Interpretation and Presentation LUT Shape, an explicit shape
  overrides rather than composes, and a declared Presentation LUT Sequence
  reports unsupported rather than falling back.
- The colour stage of the LUT chain, completing DICOM PS3.3 C.11. Palette
  colour indexes the stored value through the three lookup tables, honouring the
  descriptor's first mapped input and its zero-means-65,536 entry count. RGB,
  YBR_FULL, YBR_FULL_422 and YBR_PARTIAL_422 are interpreted, with the inverse
  matrices derived from PS3.3 C.7.6.3.1.2's own equations. YBR_ICT and YBR_RCT
  report unsupported, because the JPEG 2000 codec owns those transforms. The
  colour conversion is applied exactly once, decided against the decoder's own
  reported output space, so a frame a codec already converted is not converted
  again. Planar Configuration is honoured for native Pixel Data and ignored for
  encapsulated. A 4:2:2 frame is read as two stored samples per pixel rather
  than three.
- Enhanced multiframe projection with checked frame counts, retained shared
  and per-frame functional-group provenance, and bounded frame indexing.
- Derived per-frame image-plane geometry, with calibrated against uncalibrated
  spacing kept distinct, gantry tilt measured from geometry rather than read
  from its nominal tag, and non-uniform inter-frame spacing refused rather than
  averaged.
- JPEG Baseline, Extended, Lossless, and Lossless SV1 decoding with exact UID
  registration, atomic caller-buffer output, and decoded colour evidence.
- JPEG 2000 Part 1 decoding for lossless Transfer Syntax `.90` and general
  Transfer Syntax `.91`, with exact stored-domain validation, atomic output,
  native and WebAssembly execution, and a measured release benchmark.
- JPEG-LS decoding for lossless Transfer Syntax `.80` and near-lossless `.81`,
  with the two UIDs held apart by the codestream's own `NEAR` parameter, the
  stored-domain round trip proven over the whole 16-bit range, multi-component
  frames refused rather than mis-decoded, and a measured release benchmark.
- HTJ2K decoding for Transfer Syntaxes `.201`, `.202` and `.203`, with the CAP
  marker required so a JPEG 2000 Part 1 codestream cannot be read as HTJ2K, the
  irreversible syntax published as a pinned measured divergence rather than a
  bound, and identical output proven on native, plain WebAssembly and SIMD
  WebAssembly. **It cannot be redistributed yet**: the decoder's package carries
  no licence notice, and the release path refuses while that is true.
- Native DICOM Pixel Data normalization for little-endian and retired
  big-endian transfer syntaxes, allocation-free RLE Lossless decoding, checked
  native multiframe extraction, and typed Pixel Data VR and decoded sample
  layout evidence. Deflated Explicit VR Little Endian remains a whole-data-set
  ingest route rather than a frame decoder.
- A safe in-memory NIfTI-1.1 ingest path for little-endian single-file volumes,
  with checked payload bounds, retained scaling and affine declarations, and
  selected qform or sform geometry converted from RAS to LPS coordinates.
- A quirk-capture registry and checker that bind a synthetic generator recipe,
  manifest row, independently computed expectation, regression test and active
  mutation evidence into one reviewed record.
- The authoritative Markdown specification under `docs/hld/`, sanitized during
  bootstrap so no external source-document bundle is needed by the workflow.
- `docs/sprints/`, with 190 F-IDs imported from the backlog spreadsheet and
  `F-X` stories added since. This line deliberately does not repeat the totals.
  `python3 scripts/backlog_check.py` prints how many F-IDs there are and how
  many are done, and `python3 scripts/gen_sprint_plan.py --check` prints how
  many carry a sprint. The allocation spans 72 sprints and 18 milestones, which
  `docs/sprints/BACKLOG.md`'s summary section names the command for.
- The gate set behind `bin/ocelli.sh gate`, and a CI floor that runs every one
  of them that needs no GPU and no corpus. `bin/ocelli.sh gate --list` is the
  list, and this line deliberately does not repeat the count, because a number
  written here is a second list that goes stale the first time a gate is
  added.
- The workflow: `.claude/WORKFLOW.md`, eighteen commands, and generated Codex
  adapters under `.agents/skills/`.
- The wasm build pipeline. `bin/ocelli.sh wasm` produces
  `crates/ocelli-wasm/pkg` through `wasm-pack` under HLD section 15.2's release
  profile, and the `wasm` gate measures that artefact against a recorded size
  budget in `ci/wasm-size-budget.json`. First measurement 14,104 bytes, and
  16,388 after F-005 added the panic hook. The file now records 1,999,061 bytes
  for the render edge, from 16,388. The 183x and 488x figures described the
  16,388 byte module. This module is about 1.5 times under the 3 MB low end of
  the Appendix A gate A4 estimate. A4 stays open because cold start was not
  remeasured.
- The cross-target build proof, `bin/ocelli.sh native` and the `native` gate.
  It links the `ocelli-desktop` and `ocelli-server` entry points, builds every
  shared crate for both wasm32 and the host, and compares resolved features
  across the two targets against a declared baseline. `ocelli-native` is now a
  compile error under wasm32 rather than a crate that merely should not be
  there.
- The GPU device-sharing contract of HLD section 31. `ocelli-render` owns
  `GpuContext`, holding one device, one queue and the resolved `Caps`, and is
  the only crate permitted to create a device. `ocelli-compute` borrows it
  through `ComputeCtx` and declares the `Kernel` trait. Enforced by the types,
  by compile-fail cases that need no GPU, and by the `device` gate.
- The npm packaging pipeline and the `packages` gate. It proves what a
  consumer receives rather than what compiles: the tarball carries every path
  its exports map advertises and both licence files, a project outside the
  workspace installs and imports it under `bundler` and `node16` resolution,
  and `npm publish --dry-run` exercises the registry path without publishing.
  `@ocelli/core` and `@ocelli/react` now ship a README and their licences.
- The differential oracle's reference half. `bin/ocelli.sh oracle` renders
  every applicable corpus row through cornerstone3D 5.8.2 in headless Chromium
  on SwiftShader and writes reference pixels plus a metadata sidecar, or a
  precise failure at one of four named boundaries. 90 of 92 rows render
  deterministically, and the two that do not are recorded with their reason in
  `tools/oracle/unsupported.json`.
- The oracle's volume and reformat pass. Four series directories declared in
  `tools/oracle/volume-params.json` are attempted. Three are assembled into
  cornerstone3D volumes and rendered as three orthogonal reformats each. The
  fourth is refused at its declared geometry boundary. Twelve reformats are
  declared and nine are written, which `tools/oracle/out/run.json` records in
  its run-level `boundaries` object. Its `volumes` key is the four per-subject
  records, whose reformat counters are attempted rather than achieved. They
  render on their own page opened only
  after the stack page has closed, so the existing stack frames are provably
  untouched. Series geometry is measured from the files themselves through
  PS3.3 C.7.6.2.1.1 rather than from any cornerstone3D module, which is what
  makes the reference's own through-plane spacing something the harness can
  contradict. It does: the reference derives spacing from the endpoints alone
  and so renders two deliberately non-uniform synthetic series identically, and
  a real MR series with gaps running 5 to 50 mm gets a uniform 10 mm grid with
  no warning. `tools/oracle/volume-truth.json` asserts both, so the day the
  reference stops averaging, the assertion goes red and names the reason.
- The differential oracle's comparator half, which is what makes it an oracle.
  `bin/ocelli.sh gate oracle` is now `oracle && compare`: it renders the corpus
  through cornerstone3D and then diffs it, returning one record per view
  against HLD 25.1 with the tolerance class resolved from the manifest's
  category tokens rather than from the modality. Class-two views publish their
  measurement and claim no verdict, because 25.1 states no threshold for them
  and a `pass` against a bound nobody wrote is exactly what decision D14
  forbids. 25.1's maximum-difference rule passes a whole-frame swap between VOI
  `LINEAR` and `LINEAR_EXACT` everywhere, which is this project's own headline
  defect, so a signed-mean bias bound was added to 25.1 by operator decision and
  is evaluated over the informative region rather than the image rectangle. A
  mutation catalogue is replayed on every oracle gate, and a mutation that goes
  undetected fails the gate.
- Runtime tier resolution. `Caps` now has a detection procedure that resolves
  tier A, B or C from an adapter enumeration, a startup fill-rate measurement,
  the reported adapter type and the renderer string, in that order of trust,
  with an operator override through `OCELLI_TIER`. A software rasteriser
  presents a conforming WebGL2 context, so the benchmark decides and the
  strings are only a hint. `ocelli-render` now takes wgpu's `webgl` feature,
  without which tier B could not resolve in a browser at all.
- A stable error model. `ocelli-core` carries a versioned `u16` error code and
  a 32-byte record that fits the event ring's payload exactly, and
  `@ocelli/core` exports the decoder, the human-readable messages and the panic
  reader. A Rust panic is written to a fixed location in linear memory and read
  back by the shell after the trap without calling into the module, because
  `wasm32-unknown-unknown` is `panic = "abort"` in every profile and a trapped
  instance must not be reused.
- The benchmark harness, `tools/bench`, which is an instrument rather than a
  report. A tracked subject registry lists the things this project will ever
  measure, each with its normative definition, its unit, its tier dimensions
  and the F-ID of the story that will give it a subject. The driver resolves
  each subject at run time to `measured`, to `unavailable` naming the blocking
  story, or to `incomparable` on a host-class mismatch, and a recorded number
  for a pending, archived or superseded story is refused by the `bench` gate
  rather than left to discipline. **Most subjects had nothing to measure when
  this landed and the harness says so**, which is decision D7 holding rather
  than a shortfall. No proxy workload was substituted, no stub was timed and no
  number was invented. `bin/ocelli.sh bench --list` is the authority on the
  split, because it reads the backlog and a count written here goes stale the
  first time a story lands. **The HLD states no performance target of any
  kind**, which was searched rather than assumed, so every subject's definition
  is fixed now from the specification and a later story adds a runner into a
  slot with no latitude to redefine the measurement into something easier.
- Answers to Appendix A gates A1 and A2, in `docs/spikes/`. **JPEG-LS resolves
  to a single pure-Rust decoder on every target.** **HTJ2K is not registered**:
  `openjp2` does not link for `wasm32-unknown-unknown` and traps on every
  codestream when forced to. The priced replacement is an exact pin of
  `openjph-core` 0.1.0, subject to its recorded provenance and activation
  gates. It reproduces both lossless rows on native and wasm. Its irreversible
  row agrees across those targets and differs from OpenJPH 0.31.0 by one at 41
  of 6,144 samples. No Ocelli decoder is activated yet.
- A standing probe harness for the repository's guards, and **not for every
  guard**, which is what this line claimed for four sentences before qualifying
  itself until the S03 review's seventh pass. `python3 scripts/guard_census.py`
  prints the bucket watched by nothing. **It is empty today**, and this line
  said it was not until S11's sprint review ran the command.
  `bin/ocelli.sh gate guards`
  drives each probed refusal into its rejected state in a disposable repository
  and requires it to fire, and a probe whose guard exits zero is a failure of
  the harness rather than a pass. The census refuses in both directions, so a
  refusal no entry claims and an entry claiming no refusal both fail. It sorts
  every discovered refusal into four buckets, and the fourth is the refusals
  watched by nothing. **That bucket is empty today**, and the number in it is a
  ratchet that may only go down, so a refusal added without a watcher fails the
  floor and the count cannot climb back. `python3 scripts/guard_census.py` prints the buckets and names each
  uncovered entry, each declared limit and each open hole with its owner. Those
  are entry-level buckets summed over refusal sites rather than a count of
  refusals driven red, which the census says on the line that prints them. Holes
  in existing guards are declared rather than hidden, and a declared hole whose
  probe starts passing also fails, so the record cannot go stale in either
  direction. Two of the four declared in S03 were closed there. S04 closed the
  remaining two and removed their declarations, which is that ratchet working.
  `docs/lld/guards.md` is the design.
- Metadata and geometry comparison beside the pixel diff. The comparator now
  checks resolved LUT inputs, functional-group scope, presentation inversion,
  pixel spacing, image orientation, projected positions and volume geometry
  against independently generated truth. A stable aggregate render hash binds
  every view identifier, kind, dimensions and RGBA byte. The oracle gate
  verifies the reference and candidate aggregates agree, and mutations prove
  that metadata-only and hash-only corruption make the run red.
- Recorded reference divergences for SIGMOID and volume spacing. The SIGMOID
  case preserves cornerstone3D 5.8.2's use of LINEAR's half-width term and
  shows exactly where it differs from PS3.3. The volume truth distinguishes
  the reference's averaged spacing from the individual projected gaps without
  inventing a uniformity threshold for real data.
- A complete tier-C feature-availability contract. Adapter candidates are
  ranked once, every device-open attempt is recorded, and failure of the best
  candidate falls through to the next compatible adapter before resolving to
  CPU. Package metadata uses one cornerstone3D parity target, and the published
  wasm package carries the required licence material.
- Stronger repository workflow guards. CI coverage now rejects gates hidden in
  tolerated shell conditions, the wasm linear-memory-view lint follows aliases
  and destructuring, marked skill examples execute under the skills gate, and
  the sprint-plan writer refuses to overwrite its hand-curated output without
  an explicit force flag.
- Sprint closure evidence bound to one Git tree. Whole-sprint review and
  verification records must both name the clean HEAD tree. Explicitly carried
  stories remain distinct from completed work and need a tracked reason in
  `CURRENT_SPRINT.md` before close preflight accepts them.
- A budgeted LRU across the encoded, decoded and GPU tiers. `Budgeted::bytes`
  reports what an entry costs the resource it is budgeted against rather than
  what it was made from, insertion reports evictions, a replaced value and a
  refused entry as three distinct outcomes so a caller can surface three
  different events, and the budget is asserted in bytes against hand-computed
  entry sizes.
- The decoded image cache, whose inserts report an eviction, a replacement and
  a refusal as three distinct events, each encodable into the fixed 48-byte
  event envelope the shell will drain once per frame.
- Volume assembly from decoded frames. Slices group by frame of reference and
  orientation, then sort by `ImagePositionPatient` projected on the slice
  normal, and the inter-slice spacing is the median of those gaps. Non-uniform
  spacing, a sheared stack and a multi-sample layout are refused rather than
  averaged or placed into a store they cannot fill. Each volume keeps one
  contiguous allocation, a per-slice presence set, and its own decoded-tier
  budget so it cannot evict frames.
- Memory-pressure bands and a budget control on each cache tier. A reading
  is Low, Elevated or Critical from integer comparisons at one half and nine
  tenths of that tier's budget. A budget of zero is Low when the tier is
  empty and Critical when it holds bytes, and setting the budget to zero
  empties the tier. Shrinking evicts least recently used first. Band
  crossings and budget changes encode into the same 48-byte event envelope
  as an eviction. Nothing reaches JavaScript yet.
- The session's one long-lived GPU device, opened from the adapter the tier
  resolved on and with the adapter's own limits rather than the WebGPU
  defaults, so a downlevel adapter can open one at all. Device loss is an
  observed state rather than a later call failing: an unspecified loss is
  rebuilt on the same adapter without re-resolving the tier, and a deliberate
  destroy is refused rather than rebuilt behind the caller. A session that
  resolves to the CPU tier holds no device by construction.
- A render graph and frame scheduler that registers viewports, compiles its
  pipelines at creation, tracks camera, window, slice, segmentation and data
  dirt per viewport, and submits one command encoder per frame across all of
  them. A blend mode with no shader reports itself unavailable instead of
  rendering the wrong one, and a rebuilt device restores the graph's
  pipelines while the caller re-supplies its viewport textures.
- A decoded frame leaves one wasm instance as a fresh ArrayBuffer. The
  worker transfer is the zero-copy hop. The bulk channel copies once out
  and once in, because a view retained across a memory growth detaches.
- Stored samples upload with `write_texture`. A window change writes the
  thirty-two-byte VOI uniform and does not upload the samples again.
- An OffscreenCanvas in the render worker, sized in device pixels as CSS
  size times device pixel ratio, rounded per axis. Resize reconfigures that
  surface and does not upload pixels again. A session on the CPU tier reports
  the surface unavailable and does not draw a substitute.
- Tier B draws the composite stack through WebGL2. The fragment module is
  the existing window shader, and a compute or storage request reports
  unavailable. On this host a headless Chromium WebGL2 context matched the
  CPU LUT chain within 0.001. The readback fence is not used as a fill rate.
- A colour lookup and a two-table preset library. `grey` maps a display
  value to the same value on each channel. `stops` is black, red and white
  at indices 0, 128 and 255. A palette is indexed by the stored sample.
  A window change writes the thirty-two-byte VOI uniform and does not
  upload the table again. Palette on a monochrome image, a preset on a
  palette image, and an ICC profile report unavailable.
- The LUT chain's WGSL shader, evaluating DICOM PS3.3 C.11's first three stages
  from a thirty-two-byte uniform, so a window or level change writes those
  bytes and nothing else. The shader makes no LUT decision, because
  the uniform carries a resolved inversion flag, one selected window pair and
  rescale values, and carries no photometric interpretation, no window
  multiplicity and no lookup-table sequence from which any of them could be
  re-derived. A chain driven by a Modality or VOI LUT Sequence cannot be
  expressed in that uniform and reports unavailable rather than substituting
  the values the sequence overrode.
- Chunked residency and brick addressing in the cache. A volume decomposes
  into bricks whose edge is 128 voxels for eight-bit samples and 64 for
  sixteen-bit samples and for four-byte floats. An edge brick keeps its true
  extent, and its cost is the host buffer length. Residency is the set of
  brick keys present on the GPU tier. An address that does not fit is an
  error rather than a wrapped index.
- Brick upload is the default path for every volume, including one that fits
  in a single brick. Each ready brick is a 3D texture at its own extent and
  sample format. Eight-bit samples use `R8Uint`, unsigned sixteen-bit uses
  `R16Uint`, signed sixteen-bit uses `R16Sint`, and float uses `R32Float`.
  An edge brick is not padded to a cube. A window change writes the VOI
  uniform and does not upload the bricks again. Tier C reports unavailable
  and creates no texture. An extent or a format the device cannot sample
  reports unavailable and does not substitute another image.
- Two GPU-less render runs that are not the oracle. Lavapipe is a fallback
  adapter probe on ubuntu-latest, and a missing record runs that probe. The
  only other accepted record is an explicit drop with a reason, which does
  not print a pass. Headless Chrome with SwiftShader is the nightly run, and
  it passes only when the renderer string contains SwiftShader and the tier
  is C.
- Viewport, scene and tool types in `@ocelli/core`. The eleven stack
  commands, the section 17.4 readback fields, and drawing availability of
  `available`, `degraded` or `unavailable` are named here. The wasm command
  channel is still unbuilt.
- A stack camera. Fit, pan, zoom, rotation and flip compose about the image
  centre on a y-down canvas. `set_view` uploads the sixteen column-major
  floats and does not rewrite the window uniform. A fragment outside the
  image is black.
- Window presets, invert and a colormap on the stack viewport. Soft tissue
  is the section 18.3 LINEAR window. Lung and bone are named centre and
  width pairs. A preset does not replace a modality or VOI LUT sequence.
  Invert sets an explicit presentation shape. A window change does not
  change the colormap.
- Stack scroll in stored frame order. Prefetch names the neighbour below
  and the neighbour above. A missing selected frame returns no samples.
  Probing whether a neighbour is resident does not refresh it.
- Measured spacing and index shift on the stack view. The shift is the
  displayed frame placed on the reference image position, in the
  displayed frame's pixel size. A zero spacing contributes no shift.
  Gantry tilt is not read as the shear.
- Typed coordinate adapters between CSS canvas points, DICOM patient points and
  voxel indices. The half texel enters once in index space, a singular display
  map is an error rather than a fallback, and a canvas point is never treated
  as a patient translation. The camera uniform carries the device pixel ratio
  and the fragment shader divides the fragment position once before the CSS
  matrix.
- A manual window change on a selected VOI LUT sequence is refused and leaves
  the whole presentation unchanged, matching the named-preset policy. A
  modality LUT sequence is preserved by a manual window and still refuses a
  named preset.
- A wasm boundary with a typed command channel that applies every command or
  none, a ticketed bulk allocation for frames and stacks, a 256-event ring
  drained as a copy once per frame, and a 96-byte state readback into
  JavaScript-owned storage. No pixel buffer is exposed and there is no
  per-event callback. A reduced interactive draw publishes a device-sized
  presentation buffer by integer cell coverage.
- A close preflight that compares the run-state inventory with the sprint
  allocation, refuses a completed story whose backlog row is not done, and
  reports a recorded carry separately. A canonical prepare-sprint command
  stops before product implementation, verification receipts or publication.
  The unsafe gate audits the complete staged index and the verify ledger
  reports whether a receipt existed before invalidation.
