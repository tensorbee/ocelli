# Changelog

The section headed by a release tag is that release's notes. `/release-notes`
renders it and the published GitHub release body is compared byte for byte
against a fresh render. See `docs/RELEASE.md`.

## Unreleased

Repository bootstrap. Nothing is published.

### Added

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
  16,388 after F-005 added the panic hook, with the delta and its cause
  attributed in that file. Both are baselines for regression detection and
  neither is an answer to Appendix A gate A4, whose estimate is a little over
  two orders of magnitude larger, 183x at its low end and 488x at its high one.
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
  for a subject whose story has not landed is refused by the `bench` gate
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
- The session's one long-lived GPU device, opened from the adapter the tier
  resolved on and with the adapter's own limits rather than the WebGPU
  defaults, so a downlevel adapter can open one at all. Device loss is an
  observed state rather than a later call failing: an unspecified loss is
  rebuilt on the same adapter without re-resolving the tier, and a deliberate
  destroy is refused rather than rebuilt behind the caller. A session that
  resolves to the CPU tier holds no device by construction.
- The LUT chain's WGSL shader, evaluating DICOM PS3.3 C.11's first three stages
  from a thirty-two-byte uniform, so a window or level change writes those
  bytes and nothing else. The shader makes no LUT decision, because
  the uniform carries a resolved inversion flag, one selected window pair and
  rescale values, and carries no photometric interpretation, no window
  multiplicity and no lookup-table sequence from which any of them could be
  re-derived. A chain driven by a Modality or VOI LUT Sequence cannot be
  expressed in that uniform and reports unavailable rather than substituting
  the values the sequence overrode.
