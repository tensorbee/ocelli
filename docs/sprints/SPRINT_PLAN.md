# Sprint Plan

Sprint-by-sprint roadmap for Ocelli. A sprint is a coherent unit of work with
one goal, not a fixed calendar box. The sprint clock starts at the first
`/start-feature` of that sprint.

**Phase 1 is S01 to S41**, 118 stories and 397 engineer-weeks, feature parity
with cornerstone3D v5.8.9. **Phase 1.5 is S42 to S72**, 39 stories and 352
engineer-weeks, the eight differentiating capabilities of HLD Part III. Phase 2
and Phase 3 carry F-IDs in `BACKLOG.md` and no sprint, deliberately.

Those two totals are the HLD's own figures, section 38 and the Part III
preamble respectively, reached independently by summing the imported
spreadsheet. They agree exactly, which is the only reason to trust either.

## How sprints were allocated

`scripts/import_backlog_xlsx.py` packs stories into sprints inside a milestone
under two caps, at most six stories and at most sixteen estimated
engineer-weeks, and never places a story in a sprint at or before the sprint
holding something it depends on.

**Some sprints hold one story and that is not a packing failure.** It is the
head or the tail of a dependency chain. S06 holds only F-016, because every
other story in M2 depends on it. Several Phase 1.5 sprints hold one story
because that story alone is ten to fourteen engineer-weeks.

**Sprint effort is not sprint duration.** The engineer-week estimates are the
spreadsheet's, made for a team. They are kept unmodified because they are what
the HLD's totals are built from, and rewriting them would break the only
cross-check available. Treat them as relative size, and let the sprint clock
measure the real thing.

## What re-planning looks like

Phase 1.5 sizing is provisional by the HLD's own statement, 352 engineer-weeks
"to be re-estimated once Phase 1 evidence exists". Do not treat S42 onward as
committed.

The six Appendix A spike gates each carry the authority to stop or reshape the
programme. They are not backlog stories, they are questions, and `/spike`
runs them against `docs/hld/A-spike-gates.md`. Answer them in the first six
weeks, which means during M1 and M2.

## The five Part III hooks inside Phase 1

Each costs a few weeks now and a rewrite later. They are the only reason
Part III work appears in a parity plan. This table is generated from
`allocation.json`, so it cannot drift from the backlog.

| Hook | F-ID | Epic ref | Sprint | Now |
|------|------|----------|--------|-----|
| Chunked residency in the cache | F-036 | E5.6 | S12 | 4w |
| Multiscale level axis on the volume | F-058 | E8.8 | S19 | 3w |
| SR as the native annotation type | F-094 | E15.1 | S35 | 4w |
| `ocelli-compute` crate exists | F-008 | E1.8 | S02 | 2w |
| Stable render hashes from the oracle | F-015 | E2.7 | S04 | 2w |

## Goals per sprint

### M1, Foundations and the differential oracle

The workspace builds to wasm and to native, and the oracle renders the corpus through cornerstone3D before any port code exists.

_S01 to S05, 16 stories, 40 engineer-weeks._

#### Sprint S01

**Goal**: Cargo workspace, crate skeleton, lint/CI baseline, Golden corpus ingest and de-identified fixture store.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-001 | E1.1 | Cargo workspace, crate skeleton, lint/CI baseline | Build | 2w |
| F-009 | E2.1 | Golden corpus ingest and de-identified fixture store | Test | 3w |

#### Sprint S02

**Goal**: wasm-pack build pipeline with a hard size budget gate, TS package scaffold, bundling, npm publish pipeline, Cross-target build proof: native desktop + server binary, ocelli-compute crate skeleton and GPU device-sharing contract, Headless cornerstone3D reference renderer.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-002 | E1.2 | wasm-pack build pipeline with a hard size budget gate | Build | 2w |
| F-003 | E1.3 | TS package scaffold, bundling, npm publish pipeline | Build | 2w |
| F-007 | E1.7 | Cross-target build proof: native desktop + server binary | Build | 2w |
| F-008 | E1.8 | ocelli-compute crate skeleton and GPU device-sharing contract | Build | 2w |
| F-010 | E2.2 | Headless cornerstone3D reference renderer | Test | 4w |

#### Sprint S03

**Goal**: Runtime capability detection & tiering (WebGPU / WebGL2 / SIMD / threads), Error model, panic-to-JS mapping, structured logging, Benchmark harness: decode, first frame, interaction latency, Pixel-diff comparator with per-modality tolerance policy.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-004 | E1.4 | Runtime capability detection & tiering (WebGPU / WebGL2 / SIMD / threads) | Build | 2w |
| F-005 | E1.5 | Error model, panic-to-JS mapping, structured logging | Build | 2w |
| F-006 | E1.6 | Benchmark harness: decode, first frame, interaction latency | Build | 2w |
| F-011 | E2.3 | Pixel-diff comparator with per-modality tolerance policy | Test | 3w |

#### Sprint S04

**Goal**: CI gate: every PR renders the full corpus, Metadata diff harness (LUT values, geometry, spacing), Stable render-hash emission from the comparator, Tier C, software-adapter detection, and the feature-availability contract.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-012 | E2.4 | CI gate: every PR renders the full corpus | Test | 3w |
| F-013 | E2.5 | Metadata diff harness (LUT values, geometry, spacing) | Test | 2w |
| F-015 | E2.7 | Stable render-hash emission from the comparator | Test | 2w |
| F-X001 | X1.1 | Tier C, software-adapter detection, and the feature-availability contract | Rust | 4w |

#### Sprint S05

**Goal**: Quirk-capture workflow: every field bug becomes a fixture.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-014 | E2.6 | Quirk-capture workflow: every field bug becomes a fixture | Test | 3w |

### M2, DICOM ingest and the pixel pipeline

A frame parses, decodes and passes the hand-computed LUT fixtures of HLD section 18.3.

_S06 to S10, 15 stories, 48 engineer-weeks._

#### Sprint S06

**Goal**: ocelli-dicom: parse and transfer-syntax dispatch over dicom-rs.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-016 | E3.1 | ocelli-dicom: parse and transfer-syntax dispatch over dicom-rs | Rust | 3w |

#### Sprint S07

**Goal**: Metadata model and provider registry, DICOMweb client: WADO-RS, WADO-URI, QIDO-RS, NIfTI volume ingest, Codec dispatch layer and capability registry.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-017 | E3.2 | Metadata model and provider registry | Rust | 4w |
| F-021 | E3.6 | DICOMweb client: WADO-RS, WADO-URI, QIDO-RS | Rust | 3w |
| F-022 | E3.7 | NIfTI volume ingest | Rust | 2w |
| F-023 | E4.1 | Codec dispatch layer and capability registry | Rust | 2w |

#### Sprint S08

**Goal**: Image plane, pixel, modality-LUT and VOI-LUT modules, Multiframe and enhanced SOP class handling, JPEG baseline / extended / lossless via jpeg-decoder, RLE, deflate, raw little- and big-endian, JPEG 2000 via openjp2, validated against the corpus.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-018 | E3.3 | Image plane, pixel, modality-LUT and VOI-LUT modules | Rust | 3w |
| F-019 | E3.4 | Multiframe and enhanced SOP class handling | Rust | 4w |
| F-024 | E4.2 | JPEG baseline / extended / lossless via jpeg-decoder | Rust | 3w |
| F-025 | E4.3 | RLE, deflate, raw little- and big-endian | Rust | 2w |
| F-026 | E4.4 | JPEG 2000 via openjp2, validated against the corpus | Rust | 4w |

#### Sprint S09

**Goal**: Per-frame functional groups, gantry tilt, spacing calibration, HTJ2K: spike, then integrate or bridge to openjph wasm, JPEG-LS: decide CharLS bridge vs pure Rust, then integrate, LUT chain: modality, VOI (linear / exact / sigmoid), presentation, invert.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-020 | E3.5 | Per-frame functional groups, gantry tilt, spacing calibration | Rust | 3w |
| F-027 | E4.5 | HTJ2K: spike, then integrate or bridge to openjph wasm | Rust | 5w |
| F-028 | E4.6 | JPEG-LS: decide CharLS bridge vs pure Rust, then integrate | Rust | 5w |
| F-029 | E4.7 | LUT chain: modality, VOI (linear / exact / sigmoid), presentation, invert | Rust | 3w |

#### Sprint S10

**Goal**: Palette colour, planar configuration, photometric interpretation, YBR.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-030 | E4.8 | Palette colour, planar configuration, photometric interpretation, YBR | Rust | 2w |

### M3, Cache and the render core

One budgeted cache, one wgpu device, the LUT chain running as a shader stage on both capability tiers.

_S11 to S14, 15 stories, 51 engineer-weeks._

#### Sprint S11

**Goal**: ocelli-cache: budgeted LRU across encoded, decoded and GPU tiers, ocelli-render: device init, capability tiering, device-lost recovery, WGSL LUT-chain shader.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-031 | E5.1 | ocelli-cache: budgeted LRU across encoded, decoded and GPU tiers | Rust | 4w |
| F-037 | E6.1 | ocelli-render: device init, capability tiering, device-lost recovery | Rust | 4w |
| F-041 | E6.5 | WGSL LUT-chain shader | Rust | 4w |

#### Sprint S12

**Goal**: Image cache with eviction events surfaced to JS, Volume cache and progressive volume assembly, Memory-pressure telemetry and JS-visible budget controls, Chunked residency model and brick addressing in the cache, Render graph and frame scheduler with dirty tracking.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-032 | E5.2 | Image cache with eviction events surfaced to JS | Rust | 2w |
| F-033 | E5.3 | Volume cache and progressive volume assembly | Rust | 4w |
| F-034 | E5.4 | Memory-pressure telemetry and JS-visible budget controls | Rust | 2w |
| F-036 | E5.6 | Chunked residency model and brick addressing in the cache | Rust | 4w |
| F-038 | E6.2 | Render graph and frame scheduler with dirty tracking | Rust | 4w |

#### Sprint S13

**Goal**: Zero-copy handoff strategy from decode worker to renderer, OffscreenCanvas in a render worker, resize and DPR handling, Texture upload path: decode straight to write_texture, WebGL2 fallback and tier-B shader variants, Colour LUT / transfer function upload and preset library.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-035 | E5.5 | Zero-copy handoff strategy from decode worker to renderer | Rust | 2w |
| F-039 | E6.3 | OffscreenCanvas in a render worker; resize and DPR handling | Rust | 3w |
| F-040 | E6.4 | Texture upload path: decode straight to write_texture | Rust | 4w |
| F-042 | E6.6 | WebGL2 fallback and tier-B shader variants | Rust | 5w |
| F-044 | E6.8 | Colour LUT / transfer function upload and preset library | Rust | 2w |

#### Sprint S14

**Goal**: Chunked brick residency as the default upload path, Three-layer GPU-less render testing: lavapipe in CI and Chrome with SwiftShader nightly.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-043 | E6.7 | Chunked brick residency as the default upload path | Rust | 4w |
| F-X002 | X1.2 | Three-layer GPU-less render testing: lavapipe in CI and Chrome with SwiftShader nightly | Build | 3w |

### M4, Public API, the boundary, and the stack viewport

The three-channel boundary is real and a stack viewport diffs clean against cornerstone3D. This is the Phase 1 credibility gate.

_S15 to S17, 12 stories, 44 engineer-weeks._

#### Sprint S15

**Goal**: Stack viewport: display, fit, pan, zoom, rotate, flip, Window/level, VOI presets, invert, colormap, Stack scroll, prefetch strategy, image sequencing, Calibration, spacing and gantry-tilt correction, API design: viewport, scene and tool surface, documented.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-045 | E7.1 | Stack viewport: display, fit, pan, zoom, rotate, flip | Rust+TS | 4w |
| F-046 | E7.2 | Window/level, VOI presets, invert, colormap | Rust+TS | 3w |
| F-047 | E7.3 | Stack scroll, prefetch strategy, image sequencing | Rust+TS | 3w |
| F-049 | E7.5 | Calibration, spacing and gantry-tilt correction | Rust+TS | 2w |
| F-100 | E16.1 | API design: viewport, scene and tool surface, documented | TS | 4w |

#### Sprint S16

**Goal**: Coordinate transforms: canvas / world / index, wasm boundary: command channel, event ring buffer, state readback, CPU raster for the stack viewport, progressive refinement, and the tier A against C bound.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-048 | E7.4 | Coordinate transforms: canvas / world / index | Rust+TS | 3w |
| F-101 | E16.2 | wasm boundary: command channel, event ring buffer, state readback | TS | 5w |
| F-X003 | X1.3 | CPU raster for the stack viewport, progressive refinement, and the tier A against C bound | Rust+TS | 6w |

#### Sprint S17

**Goal**: Camera, display-area and presentation-state serialisation, Framework bindings (React first), Event model: the ~103 cornerstone events, re-shaped and documented, TypeScript type generation from Rust definitions.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-050 | E7.6 | Camera, display-area and presentation-state serialisation | Rust+TS | 3w |
| F-102 | E16.3 | Framework bindings (React first) | TS | 3w |
| F-103 | E16.4 | Event model: the ~103 cornerstone events, re-shaped and documented | TS | 4w |
| F-104 | E16.5 | TypeScript type generation from Rust definitions | TS | 4w |

### M5, Volume, MPR and 3D rendering

Volumes assemble progressively, reslice obliquely and ray-cast on both tiers.

_S18 to S21, 14 stories, 50 engineer-weeks._

#### Sprint S18

**Goal**: Volume construction from a stack: geometry, orientation, spacing.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-051 | E8.1 | Volume construction from a stack: geometry, orientation, spacing | Rust | 4w |

#### Sprint S19

**Goal**: Orthographic viewport: axial, sagittal, coronal, Progressive volume rendering while the series still loads, Multiscale pyramid hooks in the volume model, VOLUME_3D viewport with ray-cast direct volume rendering.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-052 | E8.2 | Orthographic viewport: axial, sagittal, coronal | Rust | 4w |
| F-057 | E8.7 | Progressive volume rendering while the series still loads | Rust | 3w |
| F-058 | E8.8 | Multiscale pyramid hooks in the volume model | Rust | 3w |
| F-059 | E9.1 | VOLUME_3D viewport with ray-cast direct volume rendering | Rust | 4w |

#### Sprint S20

**Goal**: Oblique / arbitrary-plane reslicing, Thick slab with composite, MIP, MinIP and average blend modes, Slice-position sync and frame-of-reference handling, Multi-volume fusion with per-volume opacity and colormap (PET-CT).

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-053 | E8.3 | Oblique / arbitrary-plane reslicing | Rust | 4w |
| F-054 | E8.4 | Thick slab with composite, MIP, MinIP and average blend modes | Rust | 4w |
| F-055 | E8.5 | Slice-position sync and frame-of-reference handling | Rust | 3w |
| F-056 | E8.6 | Multi-volume fusion with per-volume opacity and colormap (PET-CT) | Rust | 4w |

#### Sprint S21

**Goal**: Transfer-function editing surface and preset library, Lighting, shading and gradient opacity, Cropping planes and volume clipping, CPU MPR and oblique reslicing, with SIMD inner loops, The CPU volume-rendering decision, and honest unavailability.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-060 | E9.2 | Transfer-function editing surface and preset library | Rust | 3w |
| F-061 | E9.3 | Lighting, shading and gradient opacity | Rust | 3w |
| F-062 | E9.4 | Cropping planes and volume clipping | Rust | 4w |
| F-X004 | X1.4 | CPU MPR and oblique reslicing, with SIMD inner loops | Rust | 5w |
| F-X005 | X1.5 | The CPU volume-rendering decision, and honest unavailability | Rust+TS | 2w |

### M6, Segmentation rendering

All three representations render: labelmap, contour, surface.

_S22 to S24, 5 stories, 16 engineer-weeks._

#### Sprint S22

**Goal**: Labelmap representation: colour LUT, fill and outline rendering.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-063 | E10.1 | Labelmap representation: colour LUT, fill and outline rendering | Rust | 4w |

#### Sprint S23

**Goal**: Contour representation, Surface representation, Segmentation state model and per-viewport representation registry.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-064 | E10.2 | Contour representation | Rust | 4w |
| F-065 | E10.3 | Surface representation | Rust | 3w |
| F-066 | E10.4 | Segmentation state model and per-viewport representation registry | Rust | 3w |

#### Sprint S24

**Goal**: Segmentation intersection and labelmap edge-projection blend.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-067 | E10.5 | Segmentation intersection and labelmap edge-projection blend | Rust | 2w |

### M7, Tool framework and geometry

Interaction state in TypeScript, hit-testing and measurement mathematics in Rust.

_S25 to S27, 5 stories, 16 engineer-weeks._

#### Sprint S25

**Goal**: Tool lifecycle, tool groups, bindings and modes, ocelli-geom: hit-testing, projection and measurement mathematics.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-068 | E11.1 | Tool lifecycle, tool groups, bindings and modes | Rust+TS | 3w |
| F-070 | E11.3 | ocelli-geom: hit-testing, projection and measurement mathematics | Rust+TS | 4w |

#### Sprint S26

**Goal**: Pointer, touch and wheel event normalisation and dispatch, Cursor management and tool state machine.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-069 | E11.2 | Pointer, touch and wheel event normalisation and dispatch | Rust+TS | 3w |
| F-072 | E11.5 | Cursor management and tool state machine | Rust+TS | 3w |

#### Sprint S27

**Goal**: SVG annotation drawing layer.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-071 | E11.4 | SVG annotation drawing layer | Rust+TS | 3w |

### M8, Annotation tools

The 26 annotation classes and the ROI statistics engine.

_S28 to S29, 7 stories, 30 engineer-weeks._

#### Sprint S28

**Goal**: Length, Height, Angle, CobbAngle, Bidirectional, Probe, DragProbe, RectangleROI, EllipticalROI, CircleROI, PlanarFreehandROI, SplineROI and spline mathematics.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-073 | E12.1 | Length, Height, Angle, CobbAngle, Bidirectional | Rust+TS | 5w |
| F-074 | E12.2 | Probe, DragProbe, RectangleROI, EllipticalROI, CircleROI | Rust+TS | 5w |
| F-076 | E12.4 | PlanarFreehandROI, SplineROI and spline mathematics | Rust+TS | 5w |

#### Sprint S29

**Goal**: ROI statistics engine: mean, SD, min, max, area, SUV, histogram, LivewireContour and LivewireContourSegmentation, ArrowAnnotate, Label, KeyImage, ETDRSGrid, UltrasoundDirectional, VideoRedaction.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-075 | E12.3 | ROI statistics engine: mean, SD, min, max, area, SUV, histogram | Rust+TS | 4w |
| F-077 | E12.5 | LivewireContour and LivewireContourSegmentation | Rust+TS | 5w |
| F-078 | E12.6 | ArrowAnnotate, Label, KeyImage, ETDRSGrid | Rust+TS | 3w |
| F-079 | E12.7 | UltrasoundDirectional, VideoRedaction | Rust+TS | 3w |

### M9, Segmentation tools

The 12 segmentation classes.

_S30 to S32, 7 stories, 24 engineer-weeks._

#### Sprint S30

**Goal**: Brush with sphere and circle brush geometry, SegmentSelect, SegmentLabel, SegmentBidirectional.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-080 | E13.1 | Brush with sphere and circle brush geometry | Rust+TS | 4w |
| F-084 | E13.5 | SegmentSelect, SegmentLabel, SegmentBidirectional | Rust+TS | 3w |

#### Sprint S31

**Goal**: Rectangle, Circle and Sphere scissors, PaintFill and flood fill, RectangleROI and CircleROI start-end threshold tools, LabelmapEditWithContour.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-081 | E13.2 | Rectangle, Circle and Sphere scissors | Rust+TS | 4w |
| F-082 | E13.3 | PaintFill and flood fill | Rust+TS | 3w |
| F-083 | E13.4 | RectangleROI and CircleROI start-end threshold tools | Rust+TS | 4w |
| F-085 | E13.6 | LabelmapEditWithContour | Rust+TS | 3w |

#### Sprint S32

**Goal**: Between-slice labelmap interpolation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-086 | E13.7 | Between-slice labelmap interpolation | Rust+TS | 3w |

### M10, Manipulation and utility tools

The 25 manipulation and utility classes.

_S33 to S34, 7 stories, 26 engineer-weeks._

#### Sprint S33

**Goal**: Pan, Zoom, WindowLevel, WindowLevelRegion, StackScroll, Crosshairs, WorldCrosshair, ReferenceLines, ReferenceCursors, TrackballRotate, VolumeRotate, PlanarRotate, OrientationController, Magnify, AdvancedMagnify, ScaleOverlay, OrientationMarker.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-087 | E14.1 | Pan, Zoom, WindowLevel, WindowLevelRegion, StackScroll | Rust+TS | 4w |
| F-088 | E14.2 | Crosshairs, WorldCrosshair, ReferenceLines, ReferenceCursors | Rust+TS | 5w |
| F-090 | E14.4 | TrackballRotate, VolumeRotate, PlanarRotate, OrientationController | Rust+TS | 4w |
| F-091 | E14.5 | Magnify, AdvancedMagnify, ScaleOverlay, OrientationMarker | Rust+TS | 3w |

#### Sprint S34

**Goal**: SliceIntersection, SegmentationIntersection, OverlayGrid, VolumeCropping and VolumeCroppingControl, Sculptor, AnnotationEraser, MIPJumpToClick.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-089 | E14.3 | SliceIntersection, SegmentationIntersection, OverlayGrid | Rust+TS | 4w |
| F-092 | E14.6 | VolumeCropping and VolumeCroppingControl | Rust+TS | 3w |
| F-093 | E14.7 | Sculptor, AnnotationEraser, MIPJumpToClick | Rust+TS | 3w |

### M11, Annotation state and DICOM interop

SR is the native annotation type, and SEG, RTSTRUCT and TID 1500 round trip.

_S35 to S36, 6 stories, 22 engineer-weeks._

#### Sprint S35

**Goal**: DICOM SR as the native in-memory annotation model, Annotation state manager, frame-of-reference indexing, undo/redo, DICOM SEG round-trip, RTSTRUCT import to contour representation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-094 | E15.1 | DICOM SR as the native in-memory annotation model | Rust+TS | 4w |
| F-095 | E15.2 | Annotation state manager, frame-of-reference indexing, undo/redo | Rust+TS | 4w |
| F-098 | E15.5 | DICOM SEG round-trip | Rust+TS | 4w |
| F-099 | E15.6 | RTSTRUCT import to contour representation | Rust+TS | 4w |

#### Sprint S36

**Goal**: Measurement persistence and serialisation format, DICOM SR TID 1500 round-trip.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-096 | E15.3 | Measurement persistence and serialisation format | Rust+TS | 3w |
| F-097 | E15.4 | DICOM SR TID 1500 round-trip | Rust+TS | 3w |

### M12, Migration and rollout

Both libraries coexist, viewport by viewport, with shadow mode before each cutover.

_S37 to S39, 5 stories, 20 engineer-weeks._

#### Sprint S37

**Goal**: Integration seam: DOM-element contract identical to cornerstone, Migration guide and codemods for the existing product.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-105 | E17.1 | Integration seam: DOM-element contract identical to cornerstone | TS | 3w |
| F-108 | E17.4 | Migration guide and codemods for the existing product | TS | 4w |

#### Sprint S38

**Goal**: Side-by-side runtime: both libraries alive in one application.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-106 | E17.2 | Side-by-side runtime: both libraries alive in one application | TS | 4w |

#### Sprint S39

**Goal**: Per-viewport strangler migration behind feature flags, Shadow mode: render both in production, alert on divergence.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-107 | E17.3 | Per-viewport strangler migration behind feature flags | TS | 4w |
| F-109 | E17.5 | Shadow mode: render both in production, alert on divergence | TS | 5w |

### M13, Performance, hardening and release

Binary size and cold start inside budget, the browser matrix certified, semver and provenance policy in force.

_S40 to S41, 9 stories, 30 engineer-weeks._

#### Sprint S40

**Goal**: Binary size budget: Naga trimming, wasm-opt, split builds, Worker pool tuning and the threading escalation decision, Memory profiling under 4D and large-volume load, Browser matrix validation and WebGL2 tier certification.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-110 | E18.1 | Binary size budget: Naga trimming, wasm-opt, split builds | Build | 4w |
| F-112 | E18.3 | Worker pool tuning and the threading escalation decision | Build | 4w |
| F-113 | E18.4 | Memory profiling under 4D and large-volume load | Build | 3w |
| F-114 | E19.1 | Browser matrix validation and WebGL2 tier certification | All | 4w |

#### Sprint S41

**Goal**: Cold-start and first-frame optimisation, Accessibility and keyboard interaction parity, Documentation, examples and the migration playbook, Release engineering: semver, changelog, deprecation policy, Provenance recording for agent-generated code.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-111 | E18.2 | Cold-start and first-frame optimisation | Build | 3w |
| F-115 | E19.2 | Accessibility and keyboard interaction parity | All | 3w |
| F-116 | E19.3 | Documentation, examples and the migration playbook | All | 4w |
| F-117 | E19.4 | Release engineering: semver, changelog, deprecation policy | All | 2w |
| F-118 | E19.5 | Provenance recording for agent-generated code | All | 3w |

### M14, Out-of-core volume streaming

Bounded GPU residency on unbounded data. The first of the three checkable claims in HLD C.7.

_S42 to S45, 6 stories, 49 engineer-weeks._

#### Sprint S42

**Goal**: Brick decomposition and multiscale pyramid generation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w |

#### Sprint S43

**Goal**: Frustum and LOD chunk selection.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-120 | E30.2 | Frustum and LOD chunk selection | Rust | 8w |

#### Sprint S44

**Goal**: Adaptive per-LOD sample count with opacity correction, Residency budget, prefetch and eviction policy.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-121 | E30.3 | Adaptive per-LOD sample count with opacity correction | Rust | 8w |
| F-122 | E30.4 | Residency budget, prefetch and eviction policy | Rust | 8w |

#### Sprint S45

**Goal**: Chunk-cost indicator surface, 4D and time-series as a first-class chunked axis.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-123 | E30.5 | Chunk-cost indicator surface | Rust | 3w |
| F-124 | E30.6 | 4D and time-series as a first-class chunked axis | Rust | 12w |

### M15, The WebGPU compute subsystem

Kernels sharing the renderer's device, every tier-A kernel carrying a declared fallback.

_S46 to S51, 7 stories, 68 engineer-weeks._

#### Sprint S46

**Goal**: ocelli-compute: dispatch, buffer pools, device sharing with the renderer.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-125 | E31.1 | ocelli-compute: dispatch, buffer pools, device sharing with the renderer | Rust | 10w |

#### Sprint S47

**Goal**: Region growing and interactive segmentation kernels.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-126 | E31.2 | Region growing and interactive segmentation kernels | Rust | 12w |

#### Sprint S48

**Goal**: Filtering: denoise, smooth, sharpen, gradient.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-127 | E31.3 | Filtering: denoise, smooth, sharpen, gradient | Rust | 10w |

#### Sprint S49

**Goal**: Per-frame histogram and ROI statistics on GPU, Tier-B fallback path for every kernel.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-128 | E31.4 | Per-frame histogram and ROI statistics on GPU | Rust | 8w |
| F-131 | E31.7 | Tier-B fallback path for every kernel | Rust | 8w |

#### Sprint S50

**Goal**: Full-resolution MPR and oblique resampling on compute.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-129 | E31.5 | Full-resolution MPR and oblique resampling on compute | Rust | 10w |

#### Sprint S51

**Goal**: Registration refinement kernels.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-130 | E31.6 | Registration refinement kernels | Rust | 10w |

### M16, Prompted segmentation and standards-native annotations

SAM2-class prompting against GPU-resident tensors, and GSPS write with coded concepts.

_S52 to S56, 8 stories, 57 engineer-weeks._

#### Sprint S52

**Goal**: ONNX Runtime Web on WebGPU with GPU-resident tensors, GSPS write.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-132 | E32.1 | ONNX Runtime Web on WebGPU with GPU-resident tensors | Rust+TS | 10w |
| F-139 | E34.1 | GSPS write | Rust | 6w |

#### Sprint S53

**Goal**: Zero-copy bridge from volume textures to model input, SNOMED CT and UCUM coded concept library.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-133 | E32.2 | Zero-copy bridge from volume textures to model input | Rust+TS | 8w |
| F-140 | E34.2 | SNOMED CT and UCUM coded concept library | Rust | 6w |

#### Sprint S54

**Goal**: Prompt surface: point, box, and 3D propagation, Bulk annotations tier for very large object counts.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-134 | E32.3 | Prompt surface: point, box, and 3D propagation | Rust+TS | 8w |
| F-141 | E34.3 | Bulk annotations tier for very large object counts | Rust | 8w |

#### Sprint S55

**Goal**: Annotation provenance: tool, version, algorithm, operator.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-142 | E34.4 | Annotation provenance: tool, version, algorithm, operator | Rust | 5w |

#### Sprint S56

**Goal**: Result to DICOM SEG with model and prompt provenance.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-135 | E32.4 | Result to DICOM SEG with model and prompt provenance | Rust+TS | 6w |

### M17, Multi-monitor, attestation and live sessions

Real multi-monitor, a published divergence bound, and a CRDT over state that is already a serialisable struct.

_S57 to S65, 11 stories, 104 engineer-weeks._

#### Sprint S57

**Goal**: Window Management API: enumeration, placement, persistence, Render attestation: presentation state plus output hash.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-136 | E33.1 | Window Management API: enumeration, placement, persistence | TS | 8w |
| F-151 | E36.2 | Render attestation: presentation state plus output hash | Rust | 8w |

#### Sprint S58

**Goal**: Multi-window session state sync across displays, Per-display calibration probe and DICOM Part 14 reporting.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-137 | E33.2 | Multi-window session state sync across displays | TS | 7w |
| F-138 | E33.3 | Per-display calibration probe and DICOM Part 14 reporting | TS | 8w |

#### Sprint S59

**Goal**: Deterministic pipeline audit: pin every source of variance.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-150 | E36.1 | Deterministic pipeline audit: pin every source of variance | Rust | 10w |

#### Sprint S60

**Goal**: Cross-target reproducibility harness: browser, desktop, server.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-152 | E36.3 | Cross-target reproducibility harness: browser, desktop, server | Rust | 10w |

#### Sprint S61

**Goal**: Divergence measurement and reporting.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-153 | E36.4 | Divergence measurement and reporting | Rust | 6w |

#### Sprint S62

**Goal**: CRDT over viewport presentation state.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-154 | E37.1 | CRDT over viewport presentation state | Rust+TS | 12w |

#### Sprint S63

**Goal**: Shared annotation editing with conflict resolution.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-155 | E37.2 | Shared annotation editing with conflict resolution | Rust+TS | 14w |

#### Sprint S64

**Goal**: Presence: cursors, viewport follow, participant list.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-156 | E37.3 | Presence: cursors, viewport follow, participant list | Rust+TS | 10w |

#### Sprint S65

**Goal**: Transport, auth and session lifecycle.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-157 | E37.4 | Transport, auth and session lifecycle | Rust+TS | 11w |

### M18, Whole-slide imaging and the unified scene graph

Radiology and pathology in one scene graph, sharing one annotation coordinate model.

_S66 to S72, 7 stories, 74 engineer-weeks._

#### Sprint S66

**Goal**: Runtime pyramid discovery from VL Whole Slide Microscopy metadata.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-143 | E35.1 | Runtime pyramid discovery from VL Whole Slide Microscopy metadata | Rust | 10w |

#### Sprint S67

**Goal**: Frame-level DICOMweb tile retrieval and tile cache.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-144 | E35.2 | Frame-level DICOMweb tile retrieval and tile cache | Rust | 10w |

#### Sprint S68

**Goal**: ICC colour management pipeline.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-145 | E35.3 | ICC colour management pipeline | Rust | 10w |

#### Sprint S69

**Goal**: Additive multi-channel blending for immunofluorescence.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-146 | E35.4 | Additive multi-channel blending for immunofluorescence | Rust | 10w |

#### Sprint S70

**Goal**: Unified viewport model: WSI and volume in one scene graph.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-147 | E35.5 | Unified viewport model: WSI and volume in one scene graph | Rust | 14w |

#### Sprint S71

**Goal**: Cross-modality annotation correlation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-148 | E35.6 | Cross-modality annotation correlation | Rust | 12w |

#### Sprint S72

**Goal**: Multi-server data sources within one study.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-149 | E35.7 | Multi-server data sources within one study | Rust | 8w |

