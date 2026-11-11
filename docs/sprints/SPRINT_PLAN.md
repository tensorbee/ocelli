# Sprint Plan

Sprint-by-sprint roadmap for Ocelli. A sprint is a coherent unit of work with
one goal, not a fixed calendar box. The sprint clock starts at the first
`/start-feature` of that sprint.

**Phase 1 is S01 to S44**, 118 stories and 397 engineer-weeks, feature parity
with cornerstone3D. **S45 to S50 extend Phase 1** with the local and single showcase viewer.
They are outside that imported total. **Phase 1.5 starts with S46 and
continues in S51 to S80**, 39 stories and 352 engineer-weeks, the eight differentiating capabilities of HLD Part
III. Phase 2 and Phase 3 carry F-IDs in `BACKLOG.md` and no sprint,
deliberately.

Those two totals agree with HLD section 38 and the Part III preamble, and they
count the IMPORTED rows only. Every `F-X` story added since the import is also
Phase 1, and so are F-191 to F-201 and F-206 to F-207. The allocation's Phase 1 total is therefore
larger than 397, and both numbers are correct. `docs/sprints/BACKLOG.md`'s Summary section
names the command that prints each. The parity target itself is 5.8.2 and not
the HLD's v5.8.9, which does not exist, and that is deviation **D-11**.

## How sprints were allocated

Stories were initially packed into sprints inside a milestone under two caps,
at most six stories and at most sixteen estimated engineer-weeks. The current showcase waves allow a dependency earlier within the same
sprint only when it is completed before its consumer starts, as
`/start-feature` requires. Source review alone does not meet that prerequisite.
A later sprint dependency is never a scheduling shortcut.

**Some sprints hold one story and that is not a packing failure.** It is the
head or the tail of a dependency chain. S06 holds only F-016, because every
other story in M2 depends on it. Several Phase 1.5 sprints hold one story
because that story alone is ten to fourteen engineer-weeks.

**Sprint effort is not sprint duration.** The engineer-week estimates were made
for a team. Treat them as relative size, and let the sprint clock measure the
real thing.

## Capacity calibration

Recalculated at the S03 and S06 closes, the two points this project committed
to.

**Neither point produced a representative measurement, so there is still no
capacity factor with which to re-plan the port.** Every S03 row reads `not
measured` for its recorded attribution reason. The only S06 row, F-016, also
reads `not measured` because its wall clock includes a design decision, five
implementation review passes, and repeated full feature verification runs.
Writing a plausible duration into either sprint would destroy the evidence
rather than complete it.

The three measured rows remain F-001, F-009, and F-010 from S01 and S02. They
total 9 estimated engineer-weeks and 1.44 measured days, a descriptive ratio
of 0.16 measured days per estimated engineer-week. That ratio describes
foundations and oracle work. It does not describe the M2 port and must not be
used to forecast it. These commands reproduce the measured and unmeasured row
sets, and the two patterns are anchored because a bare `measured` also matches
`not measured`:

```bash
grep -cE '^\| F-.*\| [0-9.]+d measured' docs/sprints/SPRINT_TRACKER.md
grep -cE '^\| F-.*\| not measured' docs/sprints/SPRINT_TRACKER.md
```

**What both close points change about how to measure** is the actionable part.
The estimate column stays as it is. A port story must run alone if its duration
is wanted, and verification or review time must be attributed separately. If
that isolation is not available, the row remains `not measured` and no
capacity claim follows from it.

## What re-planning looks like

Phase 1.5 sizing is provisional by the HLD's own statement, 352 engineer-weeks
"to be re-estimated once Phase 1 evidence exists". The operator authorized the showcase completion waves S47 to S50 on
2026-11-08, deferring later scheduled work by four sprints. These are ordered
work packages, not measured-capacity or calendar completion promises.

The six Appendix A spike gates each carry the authority to stop or reshape the
programme. They are not backlog stories, they are questions, and `/spike`
runs them against `docs/hld/A-spike-gates.md`. Answer them in the first six
weeks, which means during M1 and M2.

## The five Part III hooks inside Phase 1

Each costs a few weeks now and a rewrite later. They are the only reason
Part III work appears in a parity plan. This table was WRITTEN from
`allocation.json` by `scripts/gen_sprint_plan.py`, and it can drift, which this
paragraph used to deny. The generator runs once at bootstrap and after that
this file is hand-curated, so nothing rewrites a row when the allocation moves.
What holds it is `python3 scripts/gen_sprint_plan.py --check`, which asserts
every planned F-ID's sprint and estimate against the allocation. It reads the
`**Goal**:` lines and the `_SNN to SNN, N stories, N engineer-weeks._` lines
too, and it did not until the S03 review's fourth pass, which found both
stale: an S04 goal line still carrying F-X014's old title, and M1's summary
line one week behind its own table. **What it still does not read is every
other sentence in this file**, all of which is hand-curated prose.

| Hook | F-ID | Epic ref | Sprint | Now |
|------|------|----------|--------|-----|
| Chunked residency in the cache | F-036 | E5.6 | S12 | 4w |
| Multiscale level axis on the volume | F-058 | E8.8 | S19 | 3w |
| SR as the native annotation type | F-094 | E15.1 | S35 | 4w |
| `ocelli-compute` crate exists | F-008 | E1.8 | S02 | 2w |
| Stable render hashes from the oracle | F-015 | E2.7 | S04 | 2w |

## Single showcase delivery, S47 to S50

All user-facing features are delivered in one viewer. A separate example,
a code snippet or a library-only panel is not acceptance of a viewer feature.
A capability unavailable on a declared tier remains explicitly unavailable,
including CPU volume rays under D-07 and F-X005. Later AI, whole-slide,
streaming and other scheduled stories must include their viewer workflow,
feature-matrix row and browser evidence as part of their own completion.

F-202 to F-205 retain their identities with explicit narrower ownership.
F-245 to F-265 carry the remaining bounded slices. The close allocation
proposal moves F-237 and F-238 into S46 beside F-236. F-235 and F-241
remain S47 and F-242 remains S48. Other stories previously scheduled S47 onward move four sprints,
without changing their estimates or dependency order. Recovered close
backlog drafts are numbered from F-266. The S46 close filing adds F-266 to F-272 to the backlog and allocation as
correctness and tooling follow-ups scheduled beside these waves. They are
outside those authorized wave estimates, and their inherited estimates
are provisional lead suggestions, not capacity-approved. F-273 is a provisional reference-normalization follow-up slotted into S47 beside the existing wave estimates, with a provisional 3w estimate that is not measured and not capacity-approved, and no implementation claimed. The S46 whole-sprint review follow-ups F-274 to F-280 are filed beside these waves the same way, with provisional estimates that are not capacity-approved.

The wave estimates below are sums of provisional lead engineering estimates.
They are not a team-size claim, observed velocity or calendar duration. The
canonical plans remain drafts for the normal pre-implementation design round.

| Wave | Concrete deliverable | Stories | Estimated engineer-weeks |
|------|----------------------|---------|--------------------------|
| S47 | One viewer, worker decode, one to one and shared preferences, with the existing video and VOI work | 6 | 29 |
| S48 | Metadata, histogram, overlays, interpolation, remaining stack tools and ECG, with MPEG-2 | 8 | 36 |
| S49 | MPR, cursor, slab, fusion and segmentation workflows | 7 | 31 |
| S50 | Volume controls, priors, documents/GSPS read/apply, safe JPEG XL recompression, browser conformance and the complete feature gate | 7 | 30 |

## Goals per sprint

### M1, Foundations and the differential oracle

The workspace builds to wasm and to native, and the oracle renders the corpus through cornerstone3D before any port code exists.

_S01 to S05, 31 stories, 68 engineer-weeks._

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

**Goal**: Runtime capability detection & tiering (WebGPU / WebGL2 / SIMD / threads), Error model, panic-to-JS mapping, structured logging, Benchmark harness: decode, first frame, interaction latency, Pixel-diff comparator with per-modality tolerance policy, Answer Appendix A gates A1 (HTJ2K) and A2 (JPEG-LS) against our own decoders, Oracle volume and MPR reference renders, so the spacing rows are asked something, A standing test for every repository guard, not a mutation run once at authoring time.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-004 | E1.4 | Runtime capability detection & tiering (WebGPU / WebGL2 / SIMD / threads) | Build | 2w |
| F-005 | E1.5 | Error model, panic-to-JS mapping, structured logging | Build | 2w |
| F-006 | E1.6 | Benchmark harness: decode, first frame, interaction latency | Build | 2w |
| F-011 | E2.3 | Pixel-diff comparator with per-modality tolerance policy | Test | 3w |
| F-X006 | Y1.1 | Answer Appendix A gates A1 (HTJ2K) and A2 (JPEG-LS) against our own decoders | Test | 3w |
| F-X007 | Y1.2 | Oracle volume and MPR reference renders, so the spacing rows are asked something | Test | 3w |
| F-X009 | Y1.4 | A standing test for every repository guard, not a mutation run once at authoring time | Build | 3w |

#### Sprint S04

**Goal**: Metadata diff harness (LUT values, geometry, spacing), Stable render-hash emission from the comparator, Tier C, software-adapter detection, and the feature-availability contract, One parity-target version string, and a licence in the published wasm package, CI floor equivalence, and the identical --sprint and --all gate profiles, Cross-machine reference determinism, and what the oracle claims about it, The reference's own SIGMOID width divergence, and what D14's bound says about it, Price the HTJ2K decoder route after gate A1 failed, Close the two guard holes F-X009 declared, and watch the refusals its census leaves to nothing, Execute the skills' worked examples, because the skills gate asserts nothing about their numbers, Try the next adapter when the best candidate cannot open a device, and record what was attempted, Close the measured escapes from the wasm linear memory view ban, which needs type-aware linting, Make close-preflight see the sprint review, and key its verification on the tree it is about, Decide whether a CI step that is not guaranteed to run counts as CI running the gate, Stop gen_sprint_plan.py's write mode silently overwriting a hand-curated SPRINT_PLAN.md.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-013 | E2.5 | Metadata diff harness (LUT values, geometry, spacing) | Test | 2w |
| F-015 | E2.7 | Stable render-hash emission from the comparator | Test | 2w |
| F-X001 | X1.1 | Tier C, software-adapter detection, and the feature-availability contract | Rust | 4w |
| F-X008 | Y1.3 | One parity-target version string, and a licence in the published wasm package | Build | 1w |
| F-X010 | Y1.5 | CI floor equivalence, and the identical --sprint and --all gate profiles | Build | 2w |
| F-X011 | Y1.6 | Cross-machine reference determinism, and what the oracle claims about it | Test | 2w |
| F-X012 | Y1.7 | The reference's own SIGMOID width divergence, and what D14's bound says about it | Test | 2w |
| F-X013 | Y1.8 | Price the HTJ2K decoder route after gate A1 failed | Test | 3w |
| F-X014 | Y1.9 | Close the two guard holes F-X009 declared, and watch the refusals its census leaves to nothing | Build | 2w |
| F-X015 | Y1.10 | Execute the skills' worked examples, because the skills gate asserts nothing about their numbers | Build | 1w |
| F-X016 | Y1.11 | Try the next adapter when the best candidate cannot open a device, and record what was attempted | Rust | 1w |
| F-X017 | Y1.12 | Close the measured escapes from the wasm linear memory view ban, which needs type-aware linting | Build | 2w |
| F-X018 | Y1.13 | Make close-preflight see the sprint review, and key its verification on the tree it is about | Build | 1w |
| F-X019 | Y1.14 | Decide whether a CI step that is not guaranteed to run counts as CI running the gate | Build | 1w |
| F-X020 | Y1.15 | Stop gen_sprint_plan.py's write mode silently overwriting a hand-curated SPRINT_PLAN.md | Build | 1w |

#### Sprint S05

**Goal**: Candidate comparison gate contract and verification plumbing, Quirk-capture workflow: every field bug becomes a fixture.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-012 | E2.4 | Candidate comparison gate contract and verification plumbing | Test | 3w |
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

_S15 to S17, 14 stories, 47 engineer-weeks._

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

**Goal**: Coordinate transforms: canvas / world / index, wasm boundary: command channel, event ring buffer, state readback, CPU raster for the stack viewport, progressive refinement, and the tier A against C bound, Preserve selected VOI sequences across manual window changes, Reconcile closure inventory and harden shared sprint preparation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-048 | E7.4 | Coordinate transforms: canvas / world / index | Rust+TS | 3w |
| F-101 | E16.2 | wasm boundary: command channel, event ring buffer, state readback | TS | 5w |
| F-X003 | X1.3 | CPU raster for the stack viewport, progressive refinement, and the tier A against C bound | Rust+TS | 6w |
| F-X022 | Y1.17 | Preserve selected VOI sequences across manual window changes | Rust | 1w |
| F-X023 | Y1.18 | Reconcile closure inventory and harden shared sprint preparation | Build | 2w |

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

_S18 to S21, 15 stories, 53 engineer-weeks._

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

**Goal**: Oblique / arbitrary-plane reslicing, Thick slab with composite, MIP, MinIP and average blend modes, Slice-position sync and frame-of-reference handling, Multi-volume fusion with per-volume opacity and colormap (PET-CT), Activate the full-corpus candidate-render gate against Ocelli output.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-053 | E8.3 | Oblique / arbitrary-plane reslicing | Rust | 4w |
| F-054 | E8.4 | Thick slab with composite, MIP, MinIP and average blend modes | Rust | 4w |
| F-055 | E8.5 | Slice-position sync and frame-of-reference handling | Rust | 3w |
| F-056 | E8.6 | Multi-volume fusion with per-volume opacity and colormap (PET-CT) | Rust | 4w |
| F-X021 | Y1.16 | Activate the full-corpus candidate-render gate against Ocelli output | Test | 3w |

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

### M19, Series open in the core and the viewer

The series-open gaps that Phase 1 left in the core, then the example viewer calling that core.

_S42 to S44, 11 stories, 43 engineer-weeks._

#### Sprint S42

**Goal**: Frame open from a Part 10 object: chain inputs including Modality and VOI LUT Sequences, colour selection, pixel padding PS3.3 C.7.5.1.1.2, Lossy Image Compression and Burned In Annotation flags, private creators, Series assembly: order by projected position, split mixed series, temporal axis, cine timing with loop and bounce, Overlay planes PS3.3 C.9.2 and display shutters PS3.3 C.7.6.11 and C.7.6.15, Stack display interpolation, nearest and linear, declared on tiers A, B and C.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-191 | E38.1 | Frame open from a Part 10 object: chain inputs including Modality and VOI LUT Sequences, colour selection, pixel padding PS3.3 C.7.5.1.1.2, Lossy Image Compression and Burned In Annotation flags, private creators | Rust | 5w |
| F-192 | E38.2 | Series assembly: order by projected position, split mixed series, temporal axis, cine timing with loop and bounce | Rust | 5w |
| F-193 | E38.3 | Overlay planes PS3.3 C.9.2 and display shutters PS3.3 C.7.6.11 and C.7.6.15 | Rust | 3w |
| F-194 | E38.4 | Stack display interpolation, nearest and linear, declared on tiers A, B and C | Rust | 3w |

#### Sprint S43

**Goal**: Calibration provenance: spacing, imager spacing with ERMF, ultrasound regions, then manual. SUV by lean body mass and body surface area, length ratio, volume from contours, Key Object Selection read and write (TID 2010) and Secondary Capture write, Volume from a gantry-tilted stack, Example viewer loads one wasm worker, publishes one stored sample, reports the resolved tier, and posts one WADO-RS envelope.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-195 | E38.5 | Calibration provenance: spacing, imager spacing with ERMF, ultrasound regions, then manual. SUV by lean body mass and body surface area, length ratio, volume from contours | Rust | 4w |
| F-196 | E38.6 | Key Object Selection read and write (TID 2010) and Secondary Capture write | TS | 4w |
| F-197 | E38.7 | Volume from a gantry-tilted stack | Rust | 3w |
| F-198 | E38.8 | Example viewer loads one wasm worker, publishes one stored sample, reports the resolved tier, and posts one WADO-RS envelope | TS | 4w |

#### Sprint S44

**Goal**: Example layout grid, stack drag, full screen, reference lines and reference cursor, Example presets, window drag, region window, rotate, flip, marker, magnifier, colour map, scale length, crop planes and error sentence, Example measurement document, annotation list, undo, TID 1500, key image mark, shortcut and touch.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-199 | E38.9 | Example layout grid, stack drag, full screen, reference lines and reference cursor | TS | 4w |
| F-200 | E38.10 | Example presets, window drag, region window, rotate, flip, marker, magnifier, colour map, scale length, crop planes and error sentence | TS | 4w |
| F-201 | E38.11 | Example measurement document, annotation list, undo, TID 1500, key image mark, shortcut and touch | TS | 4w |

### M20, Local folder viewer

The local folder viewer opens a synthetic study or a folder, and the page shows the image tools, the marks, and how to call the library.

_S45 to S48, 52 stories, 156 engineer-weeks._

F-208 to F-234, F-236 to F-240, F-243 and F-244 run in S46 beside
F-119. Their rows appear in S46 under M14, whose first sprint is S46.
F-235 and F-241 remain S47, and F-242 remains S48. Those exception rows appear under M21 beside the new showcase work.
The S46 close filing adds F-266 to F-269 and F-272 in S47 and F-270 and
F-271 in S48, whose rows appear in the S47 and S48 tables under M21 the
same way.
The S46 whole-sprint review filing adds F-278 in S47 and F-274, F-275,
F-277, F-279 and F-280 in S48, whose rows appear under M21 the same way, and
F-276 of M14 in S50, the sprint before F-120 that depends on it. The generator writes each milestone heading once, before its first sprint.

#### Sprint S45

**Goal**: Folder viewer synthetic study, spacing, edge letters and preset window, Folder viewer layout, gestures, marks and image controls.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-206 | E38.16 | Folder viewer synthetic study, spacing, edge letters and preset window | Rust | 5w |
| F-207 | E38.17 | Folder viewer layout, gestures, marks and image controls | TS | 8w |

### M14, Out-of-core volume streaming

Bounded GPU residency on unbounded data. The first of the three checkable claims in HLD C.7.

_S46 to S53, 8 stories, 52 engineer-weeks._

#### Sprint S46

**Goal**: Brick decomposition and multiscale pyramid generation, Folder viewer viewports: layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged from its thumbnail onto a cell, and the four-image view kept as a tile mode, Folder viewer linked viewports: scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library, Folder viewer value at the pointer: the modality value and its unit from the modality stage of the LUT chain, HU on CT, returned by the local process, Folder viewer measurements: angle, Cobb angle, bidirectional, arrow and probe, region area with mean, SD, minimum and maximum in modality units, editable handles and a measurement list, Folder viewer four corners at parity: birth date, sex, age, study time, institution, slice location and thickness, kVp and mAs, TR and TE, a scale bar, and edge letters from Patient Orientation on projection images, Folder viewer window presets by modality on number keys, every VOI window in the file listed by its explanation, and a region window from the library, Folder viewer mouse profiles and preferences: the current binding, a scroll-on-left profile and a window-on-left profile, saved in the browser, and cine at Frame Time (0018,1063), Folder viewer marks written as a TID 1500 document and key images as a Key Object Selection through @ocelli/core, saved to a download and loaded back from a folder, Folder viewer performance: each file posted once and held in a bounded cache, decode reused across window changes, measured budgets for open, first image, scroll rate and resident memory, and a release build, Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file, Dependency advisories in shipped packages: adm-zip below 0.6.1 reaches the published @ocelli/core runtime through dcmjs, resolved by a patched override or a proof that no shipped path reaches it, with an audit gate over shipped dependencies, Sprint plan checker refusals that no guard probe triggers: the four refusals in gen_sprint_plan.py the S46 guard review found unwatched, each given a fixture that turns it red, Shipped binary notices: the full licence text of every crate compiled into the wasm module and the native binaries, generated from the lockfile and checked by a gate, Published package contents: the @ocelli/core tarball stops shipping compiled test files, with a packages gate refusal for any test file in a published tarball, Pyramid means skip non-finite F32 samples: a coarser level averages only the finite samples of its box by their overlap weight, a box with no finite sample stays NaN, and negative zero is written as positive zero, Comparative benchmark against cornerstone3D 5.8.2: the folder viewer and a bare cornerstone3D page on the same studies, browser and machine, with every process's memory counted, a pixel check before any timing, and a generated summary, Frame identity: the decoded tier answers a lookup only when the frame's content id, from SOP Instance UID, transfer syntax and encoded bytes, and its description and plane match, DicomwebClient takes a documented cache mode defaulting to no-cache, and the folder viewer keys page caches by the picked File, lets the same folder be chosen again, and refuses to show a file that changed on disk, DICOM conformance report: ocelli-dicom checks each value against the VR rules of PS3.5 section 6.2 and Type 1, 1C and 2 presence for seven PS3.3 modules on CR, CT, MR and PET, a finding never carries a value or stops display, @ocelli/core reads the report, and the folder viewer shows a Checks badge and list, Release by series and study: ImageCache::release_series and BrickCache::release_volume over one Lru range removal, a cache.released ring event decoded by @ocelli/core, and the folder viewer forgetting the previous folder's held files and frames on the local process when another folder or the synthetic study is opened or the page is left, Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is, Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn, JPEG XL decoding: the three JPEG XL transfer syntaxes through jxl-oxide, lossless decoded exactly, JPEG recompression rebuilt and decoded by the existing JPEG path, unsupported features refused as unavailable, with synthetic corpus rows and declared oracle rows, Public options refuse unknown keys: every options and request object in @ocelli/core refuses an unknown key at run time with a coded OcelliError naming it, DicomwebError and the bulk, command and instance errors share that class, and loadCore rejects with the initialiser's cause instead of resolving false, Session frame cache: the wasm session keeps committed stack frames in a caller-budgeted ImageCache checked by ContentId, reports eviction, pressure and release through the ring, and exports a residency query, a show-from-cache call, and releaseSeries and releaseStudy in @ocelli/core, Unsafe guard soundness: unsafe_code forbidden by rustc through the workspace lint table on every target of every workspace member, and scripts/unsafe_allowlist_check.py rebuilt on a left-to-right Rust tokenizer with a planted unsafe test for each of the five triggers the F-229 review found, SIMD wasm builds compile SIMD code: the SIMD codec verification variants pass +simd128 to every crate through a target-scoped RUSTFLAGS, a native arm step refuses a SIMD variant whose dependency rlibs match the plain build, and D-22's F-X013 claim is rechecked on that build, Bounded sequence depth in parse_part10: the structural preflight refuses sequence nesting deeper than a named constant with a typed ParseError before dicom-rs recurses, with tests at the limit and one past on a 2 MiB thread and a fuzz-shaped regression, acceptTransferredFrame honours stackIndex and stackEpoch: a transferred frame carrying both keys commits through the selected-frame export and is refused for a stale index or epoch, with tests at the unit and built wasm levels, The wasm module ships outside the @ocelli/core tarball: docs/RELEASE.md preflight item 6 and the published list name where the module and its licence files ship and how a release proves the module is the one its HEAD builds, with the packages gate checking that place, Native build without wgpu's Metal backend: the shipped native build compiles no objc2 crate, proven by the resolved dependency graph for every Apple target, while development builds keep Metal for the tier A gate, Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, multi-component JPEG 2000 and HTJ2K decoded with their component transforms, window and invert per channel as cornerstone3D 5.8.2 applies them, region statistics per channel, colour above 8 bits scaled by one library rule, an ICC profile named as not applied, and every colour shape this build cannot draw refused by name, Matrix and tone-curve ICC profiles applied in ocelli-pixel stage 4 for colour frames, lookup-table profiles refused by name until F-145, JPEG-LS decoded per ITU-T T.87: the vendored ritk-codecs scan decoder raises the run index only for a full run segment, a scan that runs out of data or leaves unread bytes before EOI is refused, an LSE preset this decoder does not apply is refused by name, default thresholds clamp as C.2.4.1.1.1 gives them, with CharLS-encoded trigger fixtures and banded corpus rows under .80 and .81, Encapsulated Pixel Data with an empty Basic Offset Table opens per PS3.5 A.4: dicom-object 0.10.0 vendored with its collector patched so the zero-length first Item is the table and every later Item a Fragment, at the top level and inside an Icon Image Sequence, for every encapsulated syntax the registry decodes, single-frame and multi-frame, with frames that span Fragments delimited by their codestream start and end markers where A.4 lets a frame span Fragments, a present Extended Offset Table applied through EncapsulatedFrameIndex::from_extended, and an empty-table corpus row per decodable syntax.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-119 | E30.1 | Brick decomposition and multiscale pyramid generation | Rust | 10w |
| F-208 | E38.18 | Folder viewer viewports: layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged from its thumbnail onto a cell, and the four-image view kept as a tile mode | TS | 6w |
| F-209 | E38.19 | Folder viewer linked viewports: scroll by position in a shared frame of reference, synced window and zoom, and reference lines from the library | TS | 4w |
| F-210 | E38.20 | Folder viewer value at the pointer: the modality value and its unit from the modality stage of the LUT chain, HU on CT, returned by the local process | Rust | 3w |
| F-211 | E38.21 | Folder viewer measurements: angle, Cobb angle, bidirectional, arrow and probe, region area with mean, SD, minimum and maximum in modality units, editable handles and a measurement list | All | 6w |
| F-212 | E38.22 | Folder viewer four corners at parity: birth date, sex, age, study time, institution, slice location and thickness, kVp and mAs, TR and TE, a scale bar, and edge letters from Patient Orientation on projection images | All | 4w |
| F-213 | E38.23 | Folder viewer window presets by modality on number keys, every VOI window in the file listed by its explanation, and a region window from the library | All | 3w |
| F-214 | E38.24 | Folder viewer mouse profiles and preferences: the current binding, a scroll-on-left profile and a window-on-left profile, saved in the browser, and cine at Frame Time (0018,1063) | TS | 2w |
| F-215 | E38.25 | Folder viewer marks written as a TID 1500 document and key images as a Key Object Selection through @ocelli/core, saved to a download and loaded back from a folder | TS | 4w |
| F-216 | E38.26 | Folder viewer performance: each file posted once and held in a bounded cache, decode reused across window changes, measured budgets for open, first image, scroll rate and resident memory, and a release build | All | 4w |
| F-217 | E38.27 | Licence hygiene: no copyleft dependency in any tracked manifest, a permissive JPEG reference decoder for the corpus, licence allowlist gates for Rust, npm and Python, and a third-party notices file | All | 2w |
| F-218 | E38.28 | Dependency advisories in shipped packages: adm-zip below 0.6.1 reaches the published @ocelli/core runtime through dcmjs, resolved by a patched override or a proof that no shipped path reaches it, with an audit gate over shipped dependencies | TS | 1w |
| F-219 | E38.29 | Sprint plan checker refusals that no guard probe triggers: the four refusals in gen_sprint_plan.py the S46 guard review found unwatched, each given a fixture that turns it red | Test | 1w |
| F-220 | E38.30 | Shipped binary notices: the full licence text of every crate compiled into the wasm module and the native binaries, generated from the lockfile and checked by a gate | Build | 2w |
| F-221 | E38.31 | Published package contents: the @ocelli/core tarball stops shipping compiled test files, with a packages gate refusal for any test file in a published tarball | TS | 1w |
| F-222 | E30.7 | Pyramid means skip non-finite F32 samples: a coarser level averages only the finite samples of its box by their overlap weight, a box with no finite sample stays NaN, and negative zero is written as positive zero | Rust | 1w |
| F-223 | E38.32 | Comparative benchmark against cornerstone3D 5.8.2: the folder viewer and a bare cornerstone3D page on the same studies, browser and machine, with every process's memory counted, a pixel check before any timing, and a generated summary | All | 3w |
| F-224 | E5.7 | Frame identity: the decoded tier answers a lookup only when the frame's content id, from SOP Instance UID, transfer syntax and encoded bytes, and its description and plane match, DicomwebClient takes a documented cache mode defaulting to no-cache, and the folder viewer keys page caches by the picked File, lets the same folder be chosen again, and refuses to show a file that changed on disk | All | 3w |
| F-225 | E3.8 | DICOM conformance report: ocelli-dicom checks each value against the VR rules of PS3.5 section 6.2 and Type 1, 1C and 2 presence for seven PS3.3 modules on CR, CT, MR and PET, a finding never carries a value or stops display, @ocelli/core reads the report, and the folder viewer shows a Checks badge and list | All | 4w |
| F-226 | E5.8 | Release by series and study: ImageCache::release_series and BrickCache::release_volume over one Lru range removal, a cache.released ring event decoded by @ocelli/core, and the folder viewer forgetting the previous folder's held files and frames on the local process when another folder or the synthetic study is opened or the page is left | All | 3w |
| F-227 | E38.33 | Automatic window per series when the file has none: one window from the 1st to 99th nearest-rank percentile of modality values pooled over up to 16 frames spread through the series, each frame through its own rescale, leaving out stored zero and the Pixel Padding Value range, computed once in ocelli-pixel, labelled Auto window in the folder viewer, and every window in the file still used as it is | All | 3w |
| F-228 | E38.34 | Content Security Policy: the minimal policy an embedding page needs for the Ocelli worker and wasm path, documented and proven by a browser probe of the built example viewer with minimality probes, and the folder viewer served under a strict hash policy with compressed frames decoded and drawn | All | 2w |
| F-229 | E4.10 | JPEG XL decoding: the three JPEG XL transfer syntaxes through jxl-oxide, lossless decoded exactly, JPEG recompression rebuilt and decoded by the existing JPEG path, unsupported features refused as unavailable, with synthetic corpus rows and declared oracle rows | Rust | 3w |
| F-230 | E16.6 | Public options refuse unknown keys: every options and request object in @ocelli/core refuses an unknown key at run time with a coded OcelliError naming it, DicomwebError and the bulk, command and instance errors share that class, and loadCore rejects with the initialiser's cause instead of resolving false | TS | 2w |
| F-231 | E5.9 | Session frame cache: the wasm session keeps committed stack frames in a caller-budgeted ImageCache checked by ContentId, reports eviction, pressure and release through the ring, and exports a residency query, a show-from-cache call, and releaseSeries and releaseStudy in @ocelli/core | Rust+TS | 4w |
| F-232 | E38.35 | Unsafe guard soundness: unsafe_code forbidden by rustc through the workspace lint table on every target of every workspace member, and scripts/unsafe_allowlist_check.py rebuilt on a left-to-right Rust tokenizer with a planted unsafe test for each of the five triggers the F-229 review found | Build | 2w |
| F-233 | E38.36 | SIMD wasm builds compile SIMD code: the SIMD codec verification variants pass +simd128 to every crate through a target-scoped RUSTFLAGS, a native arm step refuses a SIMD variant whose dependency rlibs match the plain build, and D-22's F-X013 claim is rechecked on that build | Build | 1w |
| F-234 | E3.9 | Bounded sequence depth in parse_part10: the structural preflight refuses sequence nesting deeper than a named constant with a typed ParseError before dicom-rs recurses, with tests at the limit and one past on a 2 MiB thread and a fuzz-shaped regression | Rust | 1w |
| F-236 | E16.7 | acceptTransferredFrame honours stackIndex and stackEpoch: a transferred frame carrying both keys commits through the selected-frame export and is refused for a stale index or epoch, with tests at the unit and built wasm levels | TS | 1w |
| F-237 | E19.6 | The wasm module ships outside the @ocelli/core tarball: docs/RELEASE.md preflight item 6 and the published list name where the module and its licence files ship and how a release proves the module is the one its HEAD builds, with the packages gate checking that place | Build | 1w |
| F-238 | E19.7 | Native build without wgpu's Metal backend: the shipped native build compiles no objc2 crate, proven by the resolved dependency graph for every Apple target, while development builds keep Metal for the tier A gate | Build | 2w |
| F-239 | E38.37 | Folder viewer colour frames: RGB, YBR_FULL, YBR_FULL_422, YBR_ICT, YBR_RCT and PALETTE COLOR with Planar Configuration drawn through ocelli-pixel's ColorTransform on the decoder's reported output space, multi-component JPEG 2000 and HTJ2K decoded with their component transforms, window and invert per channel as cornerstone3D 5.8.2 applies them, region statistics per channel, colour above 8 bits scaled by one library rule, an ICC profile named as not applied, and every colour shape this build cannot draw refused by name | All | 12w |
| F-240 | E38.38 | Matrix and tone-curve ICC profiles applied in ocelli-pixel stage 4 for colour frames, lookup-table profiles refused by name until F-145 | All | 5w |
| F-243 | E4.13 | JPEG-LS decoded per ITU-T T.87: the vendored ritk-codecs scan decoder raises the run index only for a full run segment, a scan that runs out of data or leaves unread bytes before EOI is refused, an LSE preset this decoder does not apply is refused by name, default thresholds clamp as C.2.4.1.1.1 gives them, with CharLS-encoded trigger fixtures and banded corpus rows under .80 and .81 | Rust | 3w |
| F-244 | E3.10 | Encapsulated Pixel Data with an empty Basic Offset Table opens per PS3.5 A.4: dicom-object 0.10.0 vendored with its collector patched so the zero-length first Item is the table and every later Item a Fragment, at the top level and inside an Icon Image Sequence, for every encapsulated syntax the registry decodes, single-frame and multi-frame, with frames that span Fragments delimited by their codestream start and end markers where A.4 lets a frame span Fragments, a present Extended Offset Table applied through EncapsulatedFrameIndex::from_extended, and an empty-table corpus row per decodable syntax | Rust | 2w |

### M21, One showcase viewer and usable feature completion

One viewer exposes every developed user-facing library feature through real workflows, with explicit tier limits and measured or source-qualified cornerstone3D parity. S47 to S50 are the next four implementation waves, not a measured calendar forecast.

_S47 to S50, 25 stories, 104 engineer-weeks._

#### Sprint S47

**Goal**: Worker decode and owned frame render in the single showcase viewer, One stored pixel per device pixel in the single showcase viewer, Validated shared preferences in the single showcase viewer, VOI shader stage 1 matches LutChain on every tier: the shader's rescale gives the bits of ocelli_pixel::modality on a backend that fuses multiply and add, proven by a tier A fixture from the F-227 reviews that fails today, DICOM video in the folder viewer and @ocelli/core: the MPEG-2, MPEG-4 AVC/H.264 and HEVC/H.265 transfer syntaxes known to the codec registry, the stream demuxed in Rust, YBR_PARTIAL_420 converted once in ocelli-pixel, each frame shown and stepped exactly by the stream's own sample table with window, invert, readout and statistics on the decoded frame, better than cornerstone3D 5.8.2 on measured axes, and every stream this build cannot decode refused by name, One showcase viewer for local, DICOMweb and future user-facing workflows, Built example viewer loads its core from its own output: a vite build with the wasm package present gives a dist whose worker reaches the bundled wasm chunk on any static host, with no absolute build-machine path left in any emitted file, proven by a headless Chromium run of the built dist on a plain static server that reads core ready, and the dev server's fs.allow decoded so a checkout path with a space, unicode or a percent sign serves the package, Folder viewer corners on enhanced multiframe CT and MR: slice thickness, kVp, mAs, TR and TE read from the shared and per-frame functional groups for the shown frame, FD values formatted by one rule with a fixture citing its PS3.3 macro, the top-level attribute first where both exist, and a field that is absent from both left empty rather than guessed, Rustdoc links resolve: no public item's documentation links to a private item and no intra-doc link is unresolved in any workspace crate, the private and unresolved positions a fresh workspace doc run at implementation enumerates fixed in place, and a workspace cargo doc check with those two rustdoc lints denied proposed through the normal workflow, Vendored ritk-codecs JPEG-LS comments audited against ITU-T T.87: every comment in vendor/ritk-codecs-0.6.0/src/jpeg_ls that states a fact about the code or the standard is checked against the code below it and the clause it names, each false one corrected or removed with the vendored pins refreshed, the four F-243 pass 8 listed first, and the cap on k recorded as resting on the in_coded_range refusal, Oracle tests write nothing into the checkout: every produce.rs test that runs the producer when the reference run or the corpus is absent points its reference subset, candidate run and dispositions file at a per-process temporary directory, so cargo test never creates or overwrites tools/oracle/compare-out, with a test that fails if a test run changes that directory, Reference scalar normalization for the equipped comparison: implement a reviewed scalar and upload route that preserves full-range signed values, then use that same declared route for pixel precheck and the complete timed protocol without changing the native pixel arithmetic or acceptance thresholds, Comparison page states what it is: the generator of docs/benchmarks/cornerstone-comparison.md writes that the figures are a synthetic controlled observation, with its date and host conditions, says the equipped acceptance is open, and writes each unavailable-cell bullet once.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-278 | E38.68 | Comparison page states what it is: the generator of docs/benchmarks/cornerstone-comparison.md writes that the figures are a synthetic controlled observation, with its date and host conditions, says the equipped acceptance is open, and writes each unavailable-cell bullet once | TS | 1w |
| F-202 | E38.12 | Worker decode and owned frame render in the single showcase viewer | All | 6w |
| F-204 | E38.14 | One stored pixel per device pixel in the single showcase viewer | All | 2w |
| F-205 | E38.15 | Validated shared preferences in the single showcase viewer | All | 3w |
| F-235 | E6.9 | VOI shader stage 1 matches LutChain on every tier: the shader's rescale gives the bits of ocelli_pixel::modality on a backend that fuses multiply and add, proven by a tier A fixture from the F-227 reviews that fails today | Rust | 2w |
| F-241 | E4.11 | DICOM video in the folder viewer and @ocelli/core: the MPEG-2, MPEG-4 AVC/H.264 and HEVC/H.265 transfer syntaxes known to the codec registry, the stream demuxed in Rust, YBR_PARTIAL_420 converted once in ocelli-pixel, each frame shown and stepped exactly by the stream's own sample table with window, invert, readout and statistics on the decoded frame, better than cornerstone3D 5.8.2 on measured axes, and every stream this build cannot decode refused by name | All | 12w |
| F-245 | E38.40 | One showcase viewer for local, DICOMweb and future user-facing workflows | All | 4w |
| F-266 | E38.61 | Built example viewer loads its core from its own output: a vite build with the wasm package present gives a dist whose worker reaches the bundled wasm chunk on any static host, with no absolute build-machine path left in any emitted file, proven by a headless Chromium run of the built dist on a plain static server that reads core ready, and the dev server's fs.allow decoded so a checkout path with a space, unicode or a percent sign serves the package | TS | 1w |
| F-267 | E38.62 | Folder viewer corners on enhanced multiframe CT and MR: slice thickness, kVp, mAs, TR and TE read from the shared and per-frame functional groups for the shown frame, FD values formatted by one rule with a fixture citing its PS3.3 macro, the top-level attribute first where both exist, and a field that is absent from both left empty rather than guessed | Rust | 2w |
| F-268 | E19.8 | Rustdoc links resolve: no public item's documentation links to a private item and no intra-doc link is unresolved in any workspace crate, the private and unresolved positions a fresh workspace doc run at implementation enumerates fixed in place, and a workspace cargo doc check with those two rustdoc lints denied proposed through the normal workflow | Build | 1w |
| F-269 | E4.14 | Vendored ritk-codecs JPEG-LS comments audited against ITU-T T.87: every comment in vendor/ritk-codecs-0.6.0/src/jpeg_ls that states a fact about the code or the standard is checked against the code below it and the clause it names, each false one corrected or removed with the vendored pins refreshed, the four F-243 pass 8 listed first, and the cap on k recorded as resting on the in_coded_range refusal | Rust | 1w |
| F-272 | E2.8 | Oracle tests write nothing into the checkout: every produce.rs test that runs the producer when the reference run or the corpus is absent points its reference subset, candidate run and dispositions file at a per-process temporary directory, so cargo test never creates or overwrites tools/oracle/compare-out, with a test that fails if a test run changes that directory | Test | 1w |
| F-273 | E38.64 | Reference scalar normalization for the equipped comparison: implement a reviewed scalar and upload route that preserves full-range signed values, then use that same declared route for pixel precheck and the complete timed protocol without changing the native pixel arithmetic or acceptance thresholds | All | 3w |

#### Sprint S48

**Goal**: MPEG-2 video decoded in Rust: the four MPEG2 MP@ML and MP@HL transfer syntaxes decoded by a decoder written here from ITU-T H.262 with no dependency, in the program stream and bare PES as well as F-241's containers, so a local MPEG-2 file plays in the folder viewer and @ocelli/core where cornerstone3D 5.8.2 needs a transcoding server, with hand-computed fixtures for inverse quantisation and prediction, IDCT accuracy measured, size and speed recorded, and every MPEG-2 stream this decoder cannot decode refused by name, Nearest and linear display interpolation controls in the showcase, DICOM overlay planes and display shutters drawn in the showcase, Image and region histograms in the showcase, Searchable recursive metadata browser in the showcase, Metadata comparison and copy in the showcase, Remaining annotation and manipulation tools in the showcase, Bounded ECG waveform reading and display in the single showcase viewer, JPEG 2000 irreversible decode within a measured bound of OpenJPEG: the vendored ritk-codecs dequantisation and 9/7 reconstruction compared per channel against OpenJPEG on rate-truncated 8, 12 and 16-bit monochrome and colour .91 codestreams, the cause of the difference of thousands of codes at 16 bits found and fixed in the vendored decoder against ISO/IEC 15444-1 Annex E and F, and the remaining difference recorded as a D14 divergence, ICC PCS to sRGB inverse measured both ways: a D50 to linear sRGB matrix derived from the IEC 61966-2-1 primaries with Bradford adaptation compared with the printed-note inverse F-240 ships, both against Little CMS's transicc on every AC12 row, and the derived matrix adopted only if it is closer to Little CMS on every row, otherwise the measurement recorded and the printed matrix kept, Window link sends the numeric window: a linked viewport receives the source viewport's window centre and width, as cornerstone3D 5.8.2's VOI synchronizer sends them, instead of the index of a window in the file, so linked series whose files carry different windows show the same values, Pooled series window after a file changes: when F-224 refuses a changed file in a series whose window F-227 pooled, the folder viewer applies one designed rule to that window, and the window is never recomputed from the refused file, Per-image auto window memory bounded: the full-frame allocation the folder viewer's local process makes for a per-image automatic window, outside the counted resident term, measured at the largest frame the viewer admits and then either counted in that term or bounded by a named limit with a test at the limit, S46 review hygiene: a linked region window drops its region label, the region window tool says by name that it does not apply to a colour frame, the palette ICC arm looks its profile up once, and ContentId refuses a sample shape that disagrees with the frame description, Viewer window has one owner: every folder viewer window setter, the pump, a preset, a drag, Reset, a linked copy, the region window and the series window, goes through one window change entry point that owns the latest choice, so a late reply never overwrites a newer window, with a real browser control per setter that holds the late request.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-274 | E38.65 | Window link sends the numeric window: a linked viewport receives the source viewport's window centre and width, as cornerstone3D 5.8.2's VOI synchronizer sends them, instead of the index of a window in the file, so linked series whose files carry different windows show the same values | TS | 2w |
| F-275 | E38.66 | Pooled series window after a file changes: when F-224 refuses a changed file in a series whose window F-227 pooled, the folder viewer applies one designed rule to that window, and the window is never recomputed from the refused file | All | 1w |
| F-277 | E38.67 | Per-image auto window memory bounded: the full-frame allocation the folder viewer's local process makes for a per-image automatic window, outside the counted resident term, measured at the largest frame the viewer admits and then either counted in that term or bounded by a named limit with a test at the limit | Rust | 1w |
| F-279 | E38.69 | S46 review hygiene: a linked region window drops its region label, the region window tool says by name that it does not apply to a colour frame, the palette ICC arm looks its profile up once, and ContentId refuses a sample shape that disagrees with the frame description | All | 1w |
| F-280 | E38.70 | Viewer window has one owner: every folder viewer window setter, the pump, a preset, a drag, Reset, a linked copy, the region window and the series window, goes through one window change entry point that owns the latest choice, so a late reply never overwrites a newer window, with a real browser control per setter that holds the late request | All | 1w |
| F-242 | E4.12 | MPEG-2 video decoded in Rust: the four MPEG2 MP@ML and MP@HL transfer syntaxes decoded by a decoder written here from ITU-T H.262 with no dependency, in the program stream and bare PES as well as F-241's containers, so a local MPEG-2 file plays in the folder viewer and @ocelli/core where cornerstone3D 5.8.2 needs a transcoding server, with hand-computed fixtures for inverse quantisation and prediction, IDCT accuracy measured, size and speed recorded, and every MPEG-2 stream this decoder cannot decode refused by name | All | 8w |
| F-246 | E38.41 | Nearest and linear display interpolation controls in the showcase | All | 3w |
| F-247 | E38.42 | DICOM overlay planes and display shutters drawn in the showcase | All | 4w |
| F-248 | E38.43 | Image and region histograms in the showcase | All | 3w |
| F-249 | E38.44 | Searchable recursive metadata browser in the showcase | All | 4w |
| F-250 | E38.45 | Metadata comparison and copy in the showcase | All | 2w |
| F-263 | E38.58 | Remaining annotation and manipulation tools in the showcase | All | 6w |
| F-265 | E38.60 | Bounded ECG waveform reading and display in the single showcase viewer | All | 6w |
| F-270 | E4.15 | JPEG 2000 irreversible decode within a measured bound of OpenJPEG: the vendored ritk-codecs dequantisation and 9/7 reconstruction compared per channel against OpenJPEG on rate-truncated 8, 12 and 16-bit monochrome and colour .91 codestreams, the cause of the difference of thousands of codes at 16 bits found and fixed in the vendored decoder against ISO/IEC 15444-1 Annex E and F, and the remaining difference recorded as a D14 divergence | Rust | 3w |
| F-271 | E38.63 | ICC PCS to sRGB inverse measured both ways: a D50 to linear sRGB matrix derived from the IEC 61966-2-1 primaries with Bradford adaptation compared with the printed-note inverse F-240 ships, both against Little CMS's transicc on every AC12 row, and the derived matrix adopted only if it is closer to Little CMS on every row, otherwise the measurement recorded and the printed matrix kept | Rust | 2w |

#### Sprint S49

**Goal**: Orthographic and oblique MPR in the single showcase viewer, Labelmap, contour and surface segmentation displays in the showcase, Patient-space linked cursor in the showcase, MIP, MinIP and average slab controls in the showcase, Patient-space multi-volume fusion in the showcase, Fractional between-slice stack sampling in the showcase, Segmentation editing and DICOM interchange in the showcase.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-203 | E38.13 | Orthographic and oblique MPR in the single showcase viewer | All | 6w |
| F-251 | E38.46 | Labelmap, contour and surface segmentation displays in the showcase | All | 5w |
| F-252 | E38.47 | Patient-space linked cursor in the showcase | All | 3w |
| F-253 | E38.48 | MIP, MinIP and average slab controls in the showcase | All | 4w |
| F-254 | E38.49 | Patient-space multi-volume fusion in the showcase | All | 5w |
| F-261 | E38.56 | Fractional between-slice stack sampling in the showcase | All | 3w |
| F-264 | E38.59 | Segmentation editing and DICOM interchange in the showcase | All | 5w |

#### Sprint S50

**Goal**: Actual 3D volume viewport in the showcase, Volume transfer functions, shading and clipping in the showcase, Prior-study search and side-by-side reading in the showcase, Rendered exports, document workflows and planned GSPS read/apply in the showcase, Safe JPEG XL JPEG Recompression decoding and showcase workflow, Browser-owned DICOM conformance checks in the showcase, One-viewer feature inventory, parity matrix and standing workflow gate, Pyramid means across differing rescales and padding: a coarser ocelli-volume level either refuses a box whose slices carry different Rescale Slope or Intercept or averages each slice through its own rescale, chosen at design, and samples in the Pixel Padding Value range never enter a level mean, with hand-computed fixtures citing PS3.3 C.11.1 and C.7.5.1.1.2.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-276 | E30.8 | Pyramid means across differing rescales and padding: a coarser ocelli-volume level either refuses a box whose slices carry different Rescale Slope or Intercept or averages each slice through its own rescale, chosen at design, and samples in the Pixel Padding Value range never enter a level mean, with hand-computed fixtures citing PS3.3 C.11.1 and C.7.5.1.1.2 | Rust | 2w |
| F-255 | E38.50 | Actual 3D volume viewport in the showcase | All | 6w |
| F-256 | E38.51 | Volume transfer functions, shading and clipping in the showcase | All | 3w |
| F-257 | E38.52 | Prior-study search and side-by-side reading in the showcase | All | 4w |
| F-258 | E38.53 | Rendered exports, document workflows and planned GSPS read/apply in the showcase | All | 5w |
| F-259 | E38.54 | Safe JPEG XL JPEG Recompression decoding and showcase workflow | All | 5w |
| F-260 | E38.55 | Browser-owned DICOM conformance checks in the showcase | All | 4w |
| F-262 | E38.57 | One-viewer feature inventory, parity matrix and standing workflow gate | All | 3w |

#### Sprint S51

**Goal**: Frustum and LOD chunk selection.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-120 | E30.2 | Frustum and LOD chunk selection | Rust | 8w |

#### Sprint S52

**Goal**: Adaptive per-LOD sample count with opacity correction, Residency budget, prefetch and eviction policy.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-121 | E30.3 | Adaptive per-LOD sample count with opacity correction | Rust | 8w |
| F-122 | E30.4 | Residency budget, prefetch and eviction policy | Rust | 8w |

#### Sprint S53

**Goal**: Chunk-cost indicator surface, 4D and time-series as a first-class chunked axis.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-123 | E30.5 | Chunk-cost indicator surface | Rust | 3w |
| F-124 | E30.6 | 4D and time-series as a first-class chunked axis | Rust | 12w |

### M15, The WebGPU compute subsystem

Kernels sharing the renderer's device, every tier-A kernel carrying a declared fallback.

_S54 to S59, 7 stories, 68 engineer-weeks._

#### Sprint S54

**Goal**: ocelli-compute: dispatch, buffer pools, device sharing with the renderer.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-125 | E31.1 | ocelli-compute: dispatch, buffer pools, device sharing with the renderer | Rust | 10w |

#### Sprint S55

**Goal**: Region growing and interactive segmentation kernels.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-126 | E31.2 | Region growing and interactive segmentation kernels | Rust | 12w |

#### Sprint S56

**Goal**: Filtering: denoise, smooth, sharpen, gradient.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-127 | E31.3 | Filtering: denoise, smooth, sharpen, gradient | Rust | 10w |

#### Sprint S57

**Goal**: Per-frame histogram and ROI statistics on GPU, Tier-B fallback path for every kernel.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-128 | E31.4 | Per-frame histogram and ROI statistics on GPU | Rust | 8w |
| F-131 | E31.7 | Tier-B fallback path for every kernel | Rust | 8w |

#### Sprint S58

**Goal**: Full-resolution MPR and oblique resampling on compute.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-129 | E31.5 | Full-resolution MPR and oblique resampling on compute | Rust | 10w |

#### Sprint S59

**Goal**: Registration refinement kernels.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-130 | E31.6 | Registration refinement kernels | Rust | 10w |

### M16, Prompted segmentation and standards-native annotations

SAM2-class prompting against GPU-resident tensors, and GSPS write with coded concepts.

_S60 to S64, 8 stories, 57 engineer-weeks._

#### Sprint S60

**Goal**: ONNX Runtime Web on WebGPU with GPU-resident tensors, GSPS write.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-132 | E32.1 | ONNX Runtime Web on WebGPU with GPU-resident tensors | Rust+TS | 10w |
| F-139 | E34.1 | GSPS write | Rust | 6w |

#### Sprint S61

**Goal**: Zero-copy bridge from volume textures to model input, SNOMED CT and UCUM coded concept library.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-133 | E32.2 | Zero-copy bridge from volume textures to model input | Rust+TS | 8w |
| F-140 | E34.2 | SNOMED CT and UCUM coded concept library | Rust | 6w |

#### Sprint S62

**Goal**: Prompt surface: point, box, and 3D propagation, Bulk annotations tier for very large object counts.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-134 | E32.3 | Prompt surface: point, box, and 3D propagation | Rust+TS | 8w |
| F-141 | E34.3 | Bulk annotations tier for very large object counts | Rust | 8w |

#### Sprint S63

**Goal**: Annotation provenance: tool, version, algorithm, operator.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-142 | E34.4 | Annotation provenance: tool, version, algorithm, operator | Rust | 5w |

#### Sprint S64

**Goal**: Result to DICOM SEG with model and prompt provenance.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-135 | E32.4 | Result to DICOM SEG with model and prompt provenance | Rust+TS | 6w |

### M17, Multi-monitor, attestation and live sessions

Real multi-monitor, a published divergence bound, and a CRDT over state that is already a serialisable struct.

_S65 to S73, 11 stories, 104 engineer-weeks._

#### Sprint S65

**Goal**: Window Management API: enumeration, placement, persistence, Render attestation: presentation state plus output hash.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-136 | E33.1 | Window Management API: enumeration, placement, persistence | TS | 8w |
| F-151 | E36.2 | Render attestation: presentation state plus output hash | Rust | 8w |

#### Sprint S66

**Goal**: Multi-window session state sync across displays, Per-display calibration probe and DICOM Part 14 reporting.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-137 | E33.2 | Multi-window session state sync across displays | TS | 7w |
| F-138 | E33.3 | Per-display calibration probe and DICOM Part 14 reporting | TS | 8w |

#### Sprint S67

**Goal**: Deterministic pipeline audit: pin every source of variance.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-150 | E36.1 | Deterministic pipeline audit: pin every source of variance | Rust | 10w |

#### Sprint S68

**Goal**: Cross-target reproducibility harness: browser, desktop, server.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-152 | E36.3 | Cross-target reproducibility harness: browser, desktop, server | Rust | 10w |

#### Sprint S69

**Goal**: Divergence measurement and reporting.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-153 | E36.4 | Divergence measurement and reporting | Rust | 6w |

#### Sprint S70

**Goal**: CRDT over viewport presentation state.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-154 | E37.1 | CRDT over viewport presentation state | Rust+TS | 12w |

#### Sprint S71

**Goal**: Shared annotation editing with conflict resolution.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-155 | E37.2 | Shared annotation editing with conflict resolution | Rust+TS | 14w |

#### Sprint S72

**Goal**: Presence: cursors, viewport follow, participant list.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-156 | E37.3 | Presence: cursors, viewport follow, participant list | Rust+TS | 10w |

#### Sprint S73

**Goal**: Transport, auth and session lifecycle.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-157 | E37.4 | Transport, auth and session lifecycle | Rust+TS | 11w |

### M18, Whole-slide imaging and the unified scene graph

Radiology and pathology in one scene graph, sharing one annotation coordinate model.

_S74 to S80, 7 stories, 74 engineer-weeks._

#### Sprint S74

**Goal**: Runtime pyramid discovery from VL Whole Slide Microscopy metadata.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-143 | E35.1 | Runtime pyramid discovery from VL Whole Slide Microscopy metadata | Rust | 10w |

#### Sprint S75

**Goal**: Frame-level DICOMweb tile retrieval and tile cache.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-144 | E35.2 | Frame-level DICOMweb tile retrieval and tile cache | Rust | 10w |

#### Sprint S76

**Goal**: ICC colour management pipeline.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-145 | E35.3 | ICC colour management pipeline | Rust | 10w |

#### Sprint S77

**Goal**: Additive multi-channel blending for immunofluorescence.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-146 | E35.4 | Additive multi-channel blending for immunofluorescence | Rust | 10w |

#### Sprint S78

**Goal**: Unified viewport model: WSI and volume in one scene graph.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-147 | E35.5 | Unified viewport model: WSI and volume in one scene graph | Rust | 14w |

#### Sprint S79

**Goal**: Cross-modality annotation correlation.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-148 | E35.6 | Cross-modality annotation correlation | Rust | 12w |

#### Sprint S80

**Goal**: Multi-server data sources within one study.

| F-ID | Epic ref | Story | Layer | Est |
|------|----------|-------|-------|-----|
| F-149 | E35.7 | Multi-server data sources within one study | Rust | 8w |

## S16 audit intake, 2026-10-18

F-X022 and F-X023 add 3 estimated engineer-weeks to S16's existing 14.
These are planning estimates, not measured elapsed time. No existing story
was silently removed or marked done. Both repairs were implemented after this
intake. F-X022 is complete. F-X023 is complete after four independent passes.
The historical findings stay in `.claude/reviews/S13-S15-audit-pass-1.md`.
No open issues or PRs were returned by the read-only remote refresh. The
remaining S16 product work is the final sprint verification and the
whole-sprint review. xAI model was the selected implementation model. The
independent passes are recorded in
`.claude/reviews/F-X023-instruction-pass-2.md`,
`.claude/reviews/F-X023-instruction-pass-3.md` and
`.claude/reviews/F-X023-instruction-pass-4.md`.
