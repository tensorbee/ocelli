# Sprint Tracker

Velocity log. One row per completed F-ID, written by `/complete-feature`
step 3. `scripts/backlog_check.py` refuses a `done` backlog row with no row
here, because a story marked done with no completion record is a status and
not a delivery.

## What the estimate column means, and what it does not

The `Est` column is the spreadsheet's engineer-week figure, unmodified. It was
made for a team and it is what the HLD's 397 and 352 totals are built from, so
changing it would break the only cross-check the plan has.

`Days actual` is the real measurement and it is the one that should drive
re-planning. Record it honestly, including when it is much smaller than the
estimate. Under agent-assisted delivery the ratio is the interesting number and
a plausible-looking figure written to make a row tidy destroys it.

**Never write an estimated value into `Days actual`.** Where a duration was not
measured, write `not measured`. A plausible-looking `<1d` written to fill a
cell is invented evidence, and it is worse than a gap, because a gap is
visibly a gap and an invented figure is not.

## Capacity calibration, and why there is no port ratio yet

The plan recalculates after S03 and S06 from the `Days actual` column. Both
points are now recorded in `SPRINT_PLAN.md`. S03 produced no measurement, and
S06's only story is also `not measured` for the reason in its row. The three
measured rows are all foundations and oracle work from S01 and S02, so their
ratio is descriptive evidence and not a forecast for the M2 port. The table
below remains the authority, and this command prints the unmeasured set:

```bash
grep -E '^\| F-' docs/sprints/SPRINT_TRACKER.md | grep -c 'not measured'
```

The S06 close leaves the measurement rule unchanged. Run a port story alone
when its duration is wanted and attribute review and verification separately.
Otherwise record `not measured` rather than inventing a capacity input.

| F-ID | Title | Sprint | Est | Days actual | Completed |
|------|-------|--------|-----|-------------|-----------|
| F-001 | Cargo workspace, crate skeleton, lint/CI baseline | S01 | 2w | 0.37d measured, 2h58m wall clock from the design commit to the F-ID commit, across four review passes | 2026-09-04 |
| F-009 | Golden corpus ingest and de-identified fixture store | S01 | 3w | 0.76d measured, 6h03m wall clock from the design commit to the F-ID commit, across seven review passes | 2026-09-04 |
| F-002 | wasm-pack build pipeline with a hard size budget gate | S02 | 2w | not measured, wall clock from the S02 design commit to this F-ID commit spans concurrent work on F-010 and is not attributable to this story | 2026-09-04 |
| F-007 | Cross-target build proof: native desktop + server binary | S02 | 2w | not measured, same reason as F-002, this lane ran beside the F-010 worker | 2026-09-05 |
| F-008 | ocelli-compute crate skeleton and GPU device-sharing contract | S02 | 2w | not measured, three review passes, and it also corrected F-007's feature guard | 2026-09-05 |
| F-003 | TS package scaffold, bundling, npm publish pipeline | S02 | 2w | not measured, two review passes | 2026-09-05 |
| F-010 | Headless cornerstone3D reference renderer | S02 | 4w | 0.31d measured, 7h29m wall clock from the design commit to the worker commit, across thirteen review passes and a strategy change, then integrated and reviewed once more | 2026-09-05 |
| F-004 | Runtime capability detection and tiering | S03 | 2w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-005 | Error model, panic-to-JS mapping, structured logging | S03 | 2w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-X006 | Answer Appendix A gates A1 and A2 against our own decoders | S03 | 3w | not measured, three of the sprint's stories ran concurrently in worker worktrees and their implementing agents terminated on a session rate limit, so wall clock covers an interruption and is not attributable | 2026-09-05 |
| F-006 | Benchmark harness: decode, first frame, interaction latency | S03 | 2w | not measured, it ran concurrently with F-X007 in a worker worktree and the wall clock covers the other story's oracle runs | 2026-09-05 |
| F-X007 | Oracle volume and MPR reference renders | S03 | 3w | not measured, it ran concurrently with F-006 and its wall clock includes several full oracle runs plus one remediation round after the integrator's review | 2026-09-05 |
| F-011 | Pixel-diff comparator with per-modality tolerance policy | S03 | 3w | not measured, it ran serial in the canonical worktree and its wall clock includes several full oracle runs | 2026-09-05 |
| F-X009 | A standing test for every repository guard | S03 | 3w | not measured, it ran concurrently with the sprint review's remediation and its wall clock includes waiting on that | 2026-09-05 |
| F-X017 | Close the measured escapes from the wasm linear memory view ban | S04 | 2w | not measured, work overlapped F-013 verification and included repeated full floor runs | 2026-09-06 |
| F-X018 | Bind sprint review and verification evidence to the current tree | S04 | 1w | not measured, work ran serially during a whole-sprint execution with repeated floor probe and gate runs | 2026-09-06 |
| F-X019 | Require CI gates to run or make their step fail | S04 | 1w | not measured, work ran serially during a whole-sprint execution and included exhaustive bash cross-checks plus full floor runs | 2026-09-06 |
| F-013 | Metadata diff harness for LUT, geometry and spacing | S04 | 2w | not measured, worker implementation was replayed and followed by repeated oracle and full-gate runs | 2026-09-06 |
| F-015 | Stable render-hash emission from the comparator | S04 | 2w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X001 | Tier C and feature-availability contract | S04 | 4w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X008 | One parity target and published wasm licences | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X010 | Structural CI floor equivalence | S04 | 2w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X012 | Measured SIGMOID reference divergence | S04 | 2w | not measured, worker ran concurrently and used the serial oracle resource | 2026-09-06 |
| F-X013 | Price the HTJ2K decoder route | S04 | 3w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X014 | Close declared guard holes | S04 | 2w | not measured, worker ran concurrently and included repeated mutation profiles | 2026-09-06 |
| F-X015 | Execute marked skill examples | S04 | 1w | not measured, worker ran concurrently and included repeated mutation profiles | 2026-09-06 |
| F-X016 | Adapter fallback after device-open failure | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-X020 | Protect the hand-curated sprint plan | S04 | 1w | not measured, worker ran concurrently with other S04 stories | 2026-09-06 |
| F-012 | Candidate comparison gate contract and verification plumbing | S05 | 3w | not measured, the wall clock includes sprint replanning, controlled corpus-scale comparisons and repeated full floor runs | 2026-09-06 |
| F-014 | Quirk-capture workflow: every field bug becomes a fixture | S05 | 3w | not measured, the wall clock includes three review passes, repeated full floor and oracle runs, and a verification-discovered oracle repair | 2026-09-06 |
| F-016 | ocelli-dicom: parse and transfer-syntax dispatch over dicom-rs | S06 | 3w | not measured, the wall clock includes a design decision, five implementation review passes and repeated full feature verification runs | 2026-09-07 |
| F-021 | DICOMweb client: WADO-RS, WADO-URI, QIDO-RS | S07 | 3w | not measured, serial implementation followed earlier concurrent sprint work and included three F-021 review passes, an additional F-017 seam review and repeated verification, so no isolated duration was captured | 2026-09-09 |
| F-022 | NIfTI volume ingest | S07 | 2w | not measured, serial implementation included source preflight, two review passes, controlled mutations and repeated staged verification, so no isolated duration was captured | 2026-09-09 |
| F-017 | Metadata model and provider registry | S07 | 4w | not measured, parallel implementation was followed by two initial review passes, integration, a later F-017 sequence seam required by F-021 and two dedicated seam review passes, so no isolated duration was captured | 2026-09-09 |
| F-023 | Codec dispatch layer and capability registry | S07 | 2w | not measured, parallel implementation and three review passes overlapped the first S07 wave and integration verification, so no isolated duration was captured | 2026-09-09 |
| F-025 | RLE, deflate, raw little- and big-endian | S08 | 2w | not measured, serial implementation included specification preflight, six review passes, controlled mutations and repeated staged verification, so no isolated duration was captured | 2026-09-10 |
| F-026 | JPEG 2000 via ritk-codecs, validated against the corpus | S08 | 4w | not measured, serial implementation included dependency source preflight, six review passes, controlled benchmark calibration and repeated full verification, so no isolated duration was captured | 2026-09-10 |
| F-018 | Image plane, pixel, modality-LUT and VOI-LUT modules | S08 | 3w | not measured, parallel implementation included four review passes, controlled arithmetic mutations and repeated staged verification, so no isolated duration was captured | 2026-09-10 |
| F-019 | Multiframe and enhanced SOP class handling | S08 | 4w | not measured, parallel implementation included two review passes, controlled frame-index mutations and integration verification, so no isolated duration was captured | 2026-09-10 |
| F-024 | JPEG baseline / extended / lossless via jpeg-decoder | S08 | 3w | not measured, parallel implementation included six review passes, dependency adaptation, controlled codec mutations and repeated benchmark and staged verification, so no isolated duration was captured | 2026-09-10 |
| F-029 | LUT chain: modality, VOI (linear / exact / sigmoid), presentation, invert | S09 | 3w | not measured, serial implementation included specification preflight, two review passes, five controlled mutations and staged feature verification, so no isolated duration was captured | 2026-09-13 |
| F-020 | Per-frame functional groups, gantry tilt, spacing calibration | S09 | 3w | not measured, serial implementation included specification preflight, two review passes, eight controlled mutations and staged feature verification, so no isolated duration was captured | 2026-09-13 |
| F-028 | JPEG-LS: decide CharLS bridge vs pure Rust, then integrate | S09 | 5w | not measured, serial implementation included dependency source preflight, fixture generation, two review passes, eleven controlled mutations and staged feature verification, so no isolated duration was captured | 2026-09-13 |
| F-027 | HTJ2K: spike, then integrate or bridge to openjph wasm | S09 | 5w | not measured, serial implementation included independent provenance re-verification, vendoring, an unsafe audit, two review passes, nine controlled mutations, four new guard probes and staged feature verification, so no isolated duration was captured | 2026-09-13 |
| F-030 | Palette colour, planar configuration, photometric interpretation, YBR | S10 | 2w | not measured, serial implementation included specification preflight from PS3.3 rather than the HLD, three review passes, eleven controlled mutations, three new corpus rows through a new single-case generator path and staged feature verification, so no isolated duration was captured | 2026-09-14 |
| F-037 | ocelli-render: device init, capability tiering, device-lost recovery | S11 | 4w | not measured, serial implementation in the canonical worktree included transcribing the pinned wgpu 30.0.1 device-lost API from the vendored crate rather than from memory, six independent review passes converging 10/3, 4/1, 2/1, 1/0, 1/0, 0/0, nine controlled mutations of which one is a compile error and two are declared residues that stay green, a new GPU gate committed separately as a workflow change, and staged feature verification, so no isolated duration was captured | 2026-09-14 |
| F-041 | WGSL LUT-chain shader | S11 | 4w | not measured, serial implementation in the canonical worktree included computing every fixture value in exact rational arithmetic before writing the shader, eight independent review passes converging 5/4, 3/2, 1/1, 1/0, 1/0, 0/2, 1/0, 0/0, nineteen controlled mutations, and staged feature verification, so no isolated duration was captured. The four arithmetic and coverage findings all arrived in the first four passes and the last four turned up none | 2026-09-15 |
| F-031 | ocelli-cache: budgeted LRU across encoded, decoded and GPU tiers | S11 | 4w | not measured, implemented in a parallel worker worktree with ten independent review passes converging 2/3, 4/1, 1/1, 3/1, 2/1, 5/2, 1/0, 0/1, 1/0, 0/0, the six mandated mutations plus roughly 140 more, and the findings that left the suite green are listed in the F-031 close, and staged feature verification, so no isolated duration was captured | 2026-09-15 |
| F-032 | Image cache with eviction events surfaced to JS | S12 | 2w | not measured, serial implementation in the canonical worktree included three independent review passes converging 3/1, 0/1, 0/0, controlled mutations for kind numbers, payload order and bytes, and staged feature verification, so no isolated duration was captured | 2026-10-17 |
| F-038 | Render graph and frame scheduler with dirty tracking | S12 | 4w | not measured, serial implementation in the canonical worktree included four independent review passes converging 4/6, 1/2, 0/1, 0/0, GPU gate runs on a real adapter throughout, and staged feature verification, so no isolated duration was captured | 2026-10-17 |
| F-033 | Volume cache and progressive volume assembly | S12 | 4w | not measured, serial implementation in the canonical worktree included three independent review passes converging 4/6, 0/1, 0/0, an oblique projected-spacing fixture, and staged feature verification, so no isolated duration was captured | 2026-10-17 |
| F-034 | Memory-pressure telemetry and JS-visible budget controls | S12 | 2w | not measured, serial implementation in the canonical worktree included four independent review passes converging 1/0, 1/0, 1/0, 0/0, controlled mutations of the zero-budget drain, the band fractions and the counters, and staged feature verification, so no isolated duration was captured | 2026-10-17 |
| F-036 | Chunked residency model and brick addressing in the cache | S12 | 4w | not measured, serial implementation in the canonical worktree included three independent review passes converging 2/1, 0/1, 0/0, controlled mutations of the address products, the residency walk and the empty-grid check, and staged feature verification, so no isolated duration was captured | 2026-10-17 |
| F-039 | OffscreenCanvas in a render worker, resize and DPR handling | S13 | 3w | not measured, serial implementation in the canonical worktree included four independent review passes converging 2/1, 7/0, 2/0, 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-042 | WebGL2 fallback and tier-B shader variants | S13 | 5w | not measured, serial implementation in the canonical worktree included three independent review passes converging 4/0, 1/0, 0/0, a WebGL2 draw on Chromium, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-044 | Colour LUT / transfer function upload and preset library | S13 | 2w | not measured, serial implementation in the canonical worktree included two independent review passes converging 3/0, 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-035 | Zero-copy handoff strategy from decode worker to renderer | S13 | 2w | not measured, parallel worker implementation included one independent review pass at 0 defects and 0 smells with one nitpick, and feature verification on the worker tree with corpus absent, so no isolated duration was captured | 2026-10-18 |
| F-040 | Texture upload path, decode straight to write_texture | S13 | 4w | not measured, parallel worker implementation included one independent review pass at 0 defects and 0 smells, and feature verification on the worker tree with corpus absent, so no isolated duration was captured | 2026-10-18 |
| F-043 | Chunked brick residency as the default upload path | S14 | 4w | not measured, serial implementation in the canonical worktree included three independent review passes converging 4/0, 1/0, 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-X002 | Three-layer GPU-less render testing: lavapipe in CI and Chrome with SwiftShader nightly | S14 | 3w | not measured, serial implementation in the canonical worktree included three independent review passes converging 1/0, 0/0, 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-100 | API design: viewport, scene and tool surface, documented | S15 | 4w | not measured, serial implementation in the canonical worktree included three independent review passes converging 2/3, 0/0, 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-045 | Stack viewport: display, fit, pan, zoom, rotate, flip | S15 | 4w | not measured, serial implementation in the canonical worktree included one independent review pass at 0 defects, 0 smells and 1 nitpick, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-046 | Window/level, VOI presets, invert, colormap | S15 | 3w | not measured, serial implementation in the canonical worktree included two independent review passes converging 1/0, 0/0, one nitpick on each pass, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-047 | Stack scroll, prefetch strategy, image sequencing | S15 | 3w | not measured, serial implementation in the canonical worktree included one independent review pass at 0 defects, 0 smells and 0 nitpicks, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-049 | Calibration, spacing and gantry-tilt correction | S15 | 2w | not measured, serial implementation in the canonical worktree included two independent review passes converging 0/0, 0/0, one nitpick on the first pass, and staged feature verification, so no isolated duration was captured | 2026-10-18 |
| F-048 | Coordinate transforms: canvas / world and index | S16 | 3w | not measured, serial implementation in the canonical worktree included seven independent review passes converging 1/0, 1/0, 0/0, 4/3, 2/1, 1/0, 0/0, a real-adapter DPR observation through the gpu gate and staged feature verification, so no isolated duration was captured | 2026-10-26 |
| F-X022 | Preserve selected VOI sequences across manual window changes | S16 | 1w | not measured, the repair was reviewed in one independent pass at 0 defects and 0 smells with isolated mutation evidence, and the sprint profile ran on the final staged union, so no isolated duration was captured | 2026-10-26 |
| F-X023 | Reconcile closure inventory and harden shared sprint preparation | S16 | 2w | not measured, serial implementation in the canonical worktree included four independent review passes converging 0/0, 1/1, 2/1, 0/0, with the remediation re-gated on the final staged union, so no isolated duration was captured | 2026-10-26 |
| F-101 | wasm boundary: command channel, event ring buffer, state readback | S16 | 5w | not measured, serial implementation in the canonical worktree included seven independent review passes converging 3/0, 4/0, 0/0, 1/2, 1/0, 1/0, 0/0, the explicit dev wasm boundary suite and the release browser worker instrument suite on the final staged union, so no isolated duration was captured | 2026-10-26 |
| F-104 | TypeScript type generation from Rust definitions | S17 | 4w | not measured, serial implementation in the canonical worktree included five review passes converging 2/1, 1/1, 0/0, 0/3, 0/0, a controlled stride mutation of the wasm drain wrapper, a swapped presented-extent expectation, and staged feature verification, so no isolated duration was captured | 2026-10-27 |
| F-050 | Camera, display-area and presentation-state serialisation | S17 | 3w | not measured, serial implementation in the canonical worktree included one review pass at 0 defects and 0 smells in the session the operator directed to review, a controlled mutation of the unfitted fit scale, the release measurement of 2,151,196 bytes, and staged feature verification, so no isolated duration was captured | 2026-10-27 |
| F-103 | Event model, the cornerstone events re-shaped and documented | S17 | 4w | not measured, serial implementation in the canonical worktree included two review passes in the session the operator directed to review, a controlled shift of the cache-key series offset, a floor probe that drops one catalogue row, and staged feature verification, so no isolated duration was captured | 2026-10-27 |
| F-102 | Framework bindings (React first) | S17 | 3w | not measured, serial implementation in the canonical worktree included one review pass in the session the operator directed to review, a controlled change of the batch threshold, and staged feature verification, so no isolated duration was captured | 2026-10-27 |
| F-051 | Volume construction from a stack: geometry, orientation, spacing | S18 | 4w | not measured, serial implementation in the canonical worktree included two review passes converging 1/0 and 0/0 and the sprint profile on the final staged union, so no isolated duration was captured | 2026-10-27 |
| F-058 | Multiscale pyramid hooks in the volume model | S19 | 3w | not measured, serial implementation in the canonical worktree included two independent review passes converging 1/0 and 0/0, and staged feature verification, so no isolated duration was captured | 2026-10-27 |
