# Low-level design

Living current-state documents, one per area, updated by `/complete-feature`
step 9 from the design plan's `## LLD impact` list.

**These describe what the code does today, not how it got there.** A changelog
section in an LLD file is a defect: the history is in `AS_BUILT.md` and in
`git log`, and mixing the two produces a document nobody trusts as either.

Each file carries a `**F-IDs that contributed:**` line and a
`**Last updated:**` date, and so does the F-IDs column of the index below.
**Nothing maintains any of the three mechanically.** This paragraph used to say
they were maintained mechanically at completion, which was a claim about a
check that does not exist: `/complete-feature` step 9 asks the agent closing the
story to write them, no gate reads them, and by the S03 review's seventh pass
the index row for `corpus.md` had drifted from the file's own line, the one
mismatch of eleven rows. If you edit an LLD file, edit its header line and its
row here in the same hand.

| File | Area | F-IDs |
|------|------|-------|
| [core-types.md](core-types.md) | `crates/ocelli-core`, the coordinate and value spaces | F-001, F-005, F-018 |
| [geometry.md](geometry.md) | `crates/ocelli-geom`, hit-testing, projection, measurement, ultrasound direction and rectangular redaction | F-070, F-073, F-074, F-075, F-076, F-077, F-079, F-195 |
| [calibration.md](calibration.md) | Spacing provenance, SUVlbm, SUVbsa, length ratio and contour volume | F-195 |
| [dicom-ingest.md](dicom-ingest.md) | Part 10 parsing, DICOMweb source responses, lossless metadata providers, multiframe projection, frame indexing, transfer-syntax dispatch and refusal boundaries | F-016, F-017, F-019, F-020, F-021, F-025 |
| [frame-open.md](frame-open.md) | Chain inputs for one Part 10 frame, padding fill, lossy and burned-in flags, private creators, and the example-viewer helper | F-191 |
| [series-open.md](series-open.md) | Display groups, projected order, temporal slots, cine timing, and the example-viewer series panel | F-192 |
| [overlays.md](overlays.md) | Overlay planes, display shutters, and the example-viewer overlay panel | F-193 |
| [pixel-pipeline.md](pixel-pipeline.md) | Image-plane evidence, stored-value extraction, and all four stages of DICOM PS3.3 C.11: modality, VOI, presentation and colour | F-018, F-029, F-030, F-041, F-044, F-046 |
| [dicomweb.md](dicomweb.md) | TypeScript DICOMweb transport and the pure Rust response contract | F-021, F-101, F-198 |
| [nifti-ingest.md](nifti-ingest.md) | NIfTI-1.1 header, affine, coordinate conversion, and payload validation | F-022 |
| [codecs.md](codecs.md) | Explicit decoder registration, capability, exact Transfer Syntax UID dispatch, native normalization, RLE, JPEG and JPEG 2000 decode | F-023, F-024, F-025, F-026, F-027, F-028, F-030, F-X021 |
| [corpus.md](corpus.md) | Golden corpus layout, generation and verification | F-009, F-013, F-014, F-016, F-026, F-030, F-X006, F-X007, F-X012, F-X013 |
| [build-targets.md](build-targets.md) | The wasm pipeline, the size budget, the cross-target proof and the isolation invariant | F-002, F-004, F-005, F-007, F-008, F-016, F-017, F-021, F-022, F-023, F-026, F-039, F-050, F-101, F-X003, F-X008, F-110, F-198 |
| [gpu-ownership.md](gpu-ownership.md) | One device, one queue, one owner. The section 31 contract | F-004, F-005, F-008, F-037, F-038, F-039, F-041, F-042, F-X001, F-101, F-110 |
| [render-graph.md](render-graph.md) | `crates/ocelli-render` graph and frame scheduler: passes, dirt, one submit per frame, stored-sample upload, brick upload, progressive sync of ready bricks, the composite volume ray, the opacity table, the gradient shade, the patient clip planes, the thick-slab pipelines, the composite labelmap, contour and surface overlay, the labelmap edge-projection blend, contour intersection, the per-viewport representation, and a fusion view inside the existing tick | F-038, F-039, F-040, F-042, F-043, F-044, F-045, F-048, F-X003, F-057, F-059, F-060, F-061, F-062, F-063, F-064, F-065, F-066, F-067, F-054, F-056, F-194 |
| [viewport.md](viewport.md) | Stack camera, presentation state, scroll, measured placement, typed canvas and world mapping, command batches, the matrix `set_view` uploads, the snapshot record, the CPU stack raster, orthographic plane upload, the tier C present of those samples, thick-slab upload, and slice sync inside one frame of reference | F-045, F-046, F-047, F-048, F-049, F-050, F-052, F-053, F-101, F-X003, F-X022, F-054, F-055, F-X004 |
| [tier-resolution.md](tier-resolution.md) | How a session resolves tier A, B or C, the fill-rate probe, the operator override, and the two GPU-less runs | F-004, F-037, F-039, F-042, F-050, F-X001, F-X002, F-X016, F-110 |
| [feature-availability.md](feature-availability.md) | The three-state contract every tier-gated feature declares | F-037, F-041, F-X001, F-X003, F-056, F-X004, F-X005, F-063, F-064, F-065, F-066, F-067 |
| [errors.md](errors.md) | The error model, the code registry, the panic record and structured logging | F-005, F-039, F-X001, F-X017, F-101 |
| [typescript-packaging.md](typescript-packaging.md) | What the npm packages publish, and what proves it | F-003, F-004, F-005, F-021, F-X017, F-035, F-100, F-101, F-104, F-050, F-103, F-102, F-X003, F-198 |
| [public-api.md](public-api.md) | Viewport, scene and tool types, the tool registry, pointer dispatch, cursor and tool state, the SVG annotation drawing layer, length, height, Cobb, bidirectional, rectangle, ellipse, circle, polygon area, spline measurement, ROI statistics, livewire, arrow, label, key image and ETDRS marks, ultrasound direction and rectangular redaction, the circle and sphere brush, rectangle circle and sphere scissors, flood fill and paint fill, a rectangle and circle start-end threshold, a labelmap edit from a closed polygon, between-slice labelmap interpolation, segment select, label and bidirectional runs, window and stack drags, crosshair and reference lines, rotation names and an orientation quadrant, a magnify circle, a scale length and an orientation marker, a slice clip, a segmentation clip and an interior grid, a volume crop of at most six planes and a crop drag, a sculpted vertex, the nearest annotation id and a MIP sample index, a DICOM SR content tree and its provenance children, a frame-of-reference annotation list with undo and redo, a SEG frame round trip the shell does not export, an RTSTRUCT contour import the shell does not export, a measurement document that refuses 4000 content items, a TID 1500 document that round-trips one NUM, a TID 2010 key object and an 8-bit secondary capture, the copied command and state wire, the event names, the React binding, a neighbour element that stays when Ocelli is released, and three kind flags that turn on in stack, MPR, then volume order, and a caller count of bytes that differ by more than an integer limit, and a decode pool of one to eight workers that stays on stable Rust, and a browser row certified only from an observation, and a release cold-start sum for the render or decode artefact, and a keyboard action for pan, zoom and stack scroll, and the next migration kind in stack, MPR, then volume order, and a stable claim that stays false while D-22 is open, and a provenance class for one commit message | F-100, F-101, F-104, F-050, F-103, F-102, F-X003, F-068, F-069, F-072, F-071, F-073, F-074, F-075, F-076, F-077, F-078, F-079, F-080, F-084, F-081, F-082, F-083, F-085, F-086, F-087, F-088, F-090, F-091, F-089, F-092, F-093, F-094, F-095, F-098, F-099, F-096, F-097, F-196, F-105, F-108, F-106, F-107, F-109, F-112, F-114, F-111, F-115, F-116, F-117, F-118 |
| [key-object.md](key-object.md) | TID 2010 key object and 8-bit secondary capture | F-196 |
| [events.md](events.md) | The event catalogue, the delivery classes and the 5.8.2 mapping | F-032, F-034, F-101, F-103 |
| [migration.md](migration.md) | The element seam, the listener rename, a neighbour that stays, kind flags in stack then MPR then volume order, a caller count of differing bytes, and the concepts that stay | F-108, F-106, F-107, F-109 |
| [playbook.md](playbook.md) | The migration order stack, then MPR, then volume, and the null for a skip | F-116 |
| [release-policy.md](release-policy.md) | The stable claim, the deprecation window, and the refusal to publish | F-117 |
| [oracle.md](oracle.md) | The differential harness's reference half, cornerstone3D under headless Chromium | F-010, F-012, F-013, F-014, F-X006, F-X007, F-X008, F-X009, F-X012, F-X013, F-X003, F-026, F-X021 |
| [comparator.md](comparator.md) | The harness's judging half: the tolerance predicate, the verdict vocabulary, the attribution ladder and the census | F-011, F-012, F-013, F-015, F-X012, F-X003, F-103, F-X021 |
| [benchmarks.md](benchmarks.md) | The benchmark harness of HLD section 26, its subject registry, the host class and the `bench` gate | F-006, F-023, F-024, F-026, F-038, F-039, F-050, F-101, F-X003, F-X014, F-111 |
| [guards.md](guards.md) | The guard harness: discovery, the catalogue, the sandbox, the probe runner, the census and executable skill examples | F-037, F-X008, F-X009, F-X010, F-X014, F-X015, F-X018, F-X019, F-X020, F-X023, F-110 |
| [cache.md](cache.md) | `crates/ocelli-cache`: the budgeted LRU, brick addressing, what `Budgeted::bytes` promises, section 8's three tiers, and a load profile from dimensions | F-031, F-032, F-033, F-034, F-035, F-036, F-043, F-047, F-101, F-113 |
| [volumes.md](volumes.md) | `crates/ocelli-volume`: grouping, projected spacing, construction from stored samples, the contiguous store, the volume cache, the brick grid, ready bricks, `resident`, a ready-brick cut, a present slice as f32, the level axis, orthographic reslice, oblique reslice, thick-slab sampling, the index of one patient point on a held plane, world-point sampling on a shared fusion plane, the tier C present of those same samples through `CpuRaster`, and a tilted stack that keeps its lateral step | F-033, F-036, F-043, F-051, F-052, F-053, F-057, F-058, F-059, F-054, F-055, F-056, F-X004, F-197 |
