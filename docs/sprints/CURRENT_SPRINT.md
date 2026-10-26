# Current sprint, S16

**Milestone**: M4, the public API, the boundary, and the stack viewport.
**Branch**: `sprint/s16`
**Opened**: 2026-10-18
**Goal**: Distinguish canvas, world and index, open the command channel and
the polled event ring, and raster the stack on the CPU from the one LUT
chain, with the settled frame as the one a tier bound is measured on.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-048 | E7.4 | Coordinate transforms: canvas / world / index | Rust+TS | 3w | done |
| F-101 | E16.2 | wasm boundary: command channel, event ring buffer, state readback | TS | 5w | done |
| F-X003 | X1.3 | CPU raster for the stack viewport, progressive refinement, and the tier A against C bound | Rust+TS | 6w | carried |
| F-X022 | Y1.17 | Preserve selected VOI sequences across manual window changes | Rust | 1w | done |
| F-X023 | Y1.18 | Reconcile closure inventory and harden shared sprint preparation | Build | 2w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together.
The digit-only count misses an `F-X` row, so count this table with the
second command:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep -c '^| F-[0-9X]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S16 / {print $2, $9}'
```

## What this sprint is

S15 closed the first sprint of M4. A stack viewport shows one frame, windows
it, scrolls it in stored order, and places it with measured spacing. F-100
documents the viewport, scene and tool surface as a new API. S16 continues
M4. F-048 keeps canvas points, world points and indices from being
interchangeable. F-101 builds the three channels across the wasm boundary.
F-X003 draws that same stack on the CPU from the LUT chain that already
exists, and the frame a measurement uses is the settled one.

M4 runs from S15 through S17. The plan calls this the Phase 1 credibility
gate, because the three-channel boundary is real and a stack viewport diffs
clean against cornerstone3D. Opening the sprint does not make that diff
exist. The oracle still has no Ocelli renderer to compare a full viewer
against. Decision D7 holds. Closing S15 did not complete M4, and this open
publishes nothing.

## What is carried in

- **S15 left a stack camera, a presentation, stored-order scroll, and a
  measured placement.** Fit, pan, zoom, rotate and flip do not rewrite
  stored samples. Scroll does not display a neighbour. `apply_placement`
  stores displayed spacing and the measured index shift and does not draw.
  It does not read the gantry tilt tag. F-048 and F-X003 extend that view.
  They do not replace it with a second mapping.
- **The command channel was left for this sprint.** F-100 names the public
  surface. `coreAvailable` still returns false. TypeScript does not hold a
  second camera matrix. The command names F-100 wrote stay.
- **Two accepted nitpicks stay.** F-045 did not drive a negative image
  coordinate from the GPU tests. F-046 left `PresentationError::Unavailable`
  in place. Neither is scheduled here.
- **An open finding against `ocelli-pixel`'s own arithmetic**, recorded in
  `docs/lld/pixel-pipeline.md` by F-041 and **not scheduled**. A legal
  `LINEAR` chain returns a value outside its declared output range at one
  input per window, identically on CPU and GPU, because PS3.3 C.11.2.1.2's
  formula in `f32` does not reproduce its own breakpoint. It has no F-ID.
  Clamping the finding so a readback matches is a tolerance change. F-X003
  reuses that chain and does not paper over the finding.
- **Tier B greyscale has been drawn.** F-042 ran the section 18.3 rows on
  this host through headless Chromium WebGL2 and matched `LutChain::map_into`
  within 0.001, format `R32Float`. A tier B colour LUT readback was not run.
  The ignored native `Backends::GL` test found no adapter on this host, and
  that miss is not a tier B pass.
- **The wasm size budget records 1,999,061 bytes.** F-045 recorded a release
  module of 2,012,345 bytes against that baseline, inside the ceiling of
  2,099,014. The budget file was not moved by S15. `bearing_on_gate_A4` in
  `ci/wasm-size-budget.json` says this number does not close A4, and cold
  start was not remeasured. The 2.3 ms figure belongs to the 16,388-byte
  module from F-006.
- **`max_review_passes` is stored but not automatically enforced by the
  backing script.** S11 carried this process gap. The shared instructions make the
  run-sprint host responsible for counting whole-sprint passes and invoking
  `set-phase blocked` at the configured limit. The per-feature microscope
  loop remains uncapped. No automatic script transition is claimed.
- **M3 is complete and its release was not taken. M4 is not complete.** The
  cadence names 0.3.0 for M3. The workspace version is still 0.1.0. M1 was
  never published, and M2's release is still due. `/release` step 5 runs
  `python3 scripts/pin_and_size_check.py --require-redistribution`, and that
  command refuses while `openjph-core` 0.1.0 carries no BSD notice material,
  which is D-22. None of that blocks a story here. All of it blocks
  publication. No release is due at this open.
- **No new measured duration.** Every M3 row in `SPRINT_TRACKER.md` reads
  `not measured`, and the S15 rows do too. The capacity points remain the
  S03 and S06 closes. This open does not invent a ratio and does not
  recalculate capacity.
- **F-X011** remains pending because its evidence needs a second physical
  machine. **ICC** is still unimplemented, the other half of HLD section 18's
  stage-4 row. The multi-component JPEG-LS corpus row exists.
  Multi-component JPEG-LS decoding is still owed.
- **The oracle has no Ocelli renderer to compare a full viewer against.**
  F-049 ran the oracle over the reference half and that run is not a viewer
  comparison. F-X021 writes the candidate record and depends on F-012 and
  F-052. It is not this sprint. Decision D7 holds.

## The defect class this sprint is exposed to

**Mixing canvas, world and index.** HLD section 16 makes those three
different types, because a bare number list mixes them. A canvas pan is not
a patient translation. There is no half-voxel offset. `PixelSpacing::new`
takes row millimetres then column millimetres. `View::set_spacing` takes
column millimetres then row millimetres. F-048 owns the conversions. F-X003
places the same frame on the CPU and must use that mapping. A second formula
is the quietly wrong pixel.

**Pixels crossing the boundary, or an event delivered per callback.**
Decisions D2, D3 and D4. Three channels and nothing else. Typed commands go
down. Raw bytes go down into linear memory. Events come up through a ring
drained once per frame. `wasm-bindgen` stays in `ocelli-wasm`. Decision D1
already refuses a cornerstone-shaped drop-in, and F-100 wrote that surface.
F-101 builds the channels. It does not reshape the cornerstone event
catalogue. That catalogue is F-103.

**A second LUT, or a refining frame presented as settled.** HLD section 18
says the arithmetic exists once, in `ocelli-pixel`. The CPU path reuses that
implementation. It does not reimplement the chain and it does not run the
shader as a second copy. Tier C must not quietly produce a different image.
Reduced resolution applies during a window or level drag or a scroll. Full
resolution returns on settle. The settled frame is what the tier A against C
bound is measured on. An interaction frame is not that frame. The user must
be able to tell the two apart, because a measurement taken from a frame that
is still refining is the defect.

## What done means

- **F-048** converts between canvas, world and index through one mapping.
  The three stay non-interchangeable. A canvas pan is not a patient
  translation. There is no half-voxel offset. Spacing follows the argument
  order of the type that already stores it. Camera and presentation-state
  serialisation are F-050. This story does not upload pixels and does not
  add a second point type as a bare number list.
- **F-101** sends typed commands down, copies raw bytes into linear memory
  down, and drains the event ring once per frame. Pixels do not cross.
  There is no JS callback per event. State readback returns the declared
  state and not a pixel buffer. `wasm-bindgen` appears only in
  `ocelli-wasm`. Framework bindings are F-102. The reshaped event catalogue
  is F-103. Type generation from Rust definitions is F-104.
- **F-X003** rasters the stack viewport on the CPU with the LUT chain in
  `ocelli-pixel`. A window or level drag, or a scroll, may draw a reduced
  resolution, and settle draws the full frame. The settled frame is the one
  the tier A against C bound uses. A refining frame is visibly not settled.
  A tier that cannot draw reports unavailable. Inner loops may use wasm
  SIMD128, and the no-SIMD runtime is measured separately. Interactive cost
  and idle cost are reported separately. CPU MPR is F-X004.
- **F-X022** refuses a manual window on a selected VOI LUT sequence and
  leaves the whole presentation unchanged, including the colormap and the
  evidence. A manual window on a modality sequence preserves it. A named
  preset still refuses a modality sequence. This is the approved
  selected-chain policy and is stricter than PS3.3 C.11.2.1.2.2's
  alternative views. It does not upload samples or colour textures.
- **F-X023** makes `close-preflight` compare scratch membership with the
  allocation and completed states with the backlog, adds `/prepare-sprint`
  as a preparation-only command, and hardens the canonical instructions
  around numbered acceptance, independent composition cases, staged
  metadata and invalidated receipts. Regenerated adapters match their
  canonical sources. No parallel tracker and no copied normative spec.
- **Every story declares all three tier rows**, and "n/a" is a real answer
  that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and `ocelli-render` remains the only crate that may create a
  device.

## Carried forward from S16

- **F-X003** is carried because AC6 requires a representative GPU-less incumbent host, the operator has confirmed none is available, and no local diagnostic is accepted as that measurement. The interactive and idle CPU cost divided by wall time, its noise floor and an accepted budget therefore cannot be taken. AC5's coverage verdict is also still red: the tier A against C comparison is 67 matched, 0 pixel failures and 17 unmeasured with byte-identical frames, and that is recorded honestly, but the global census was seeded from a reference-only run and this subset run measures 14 more views, so the verdict is `coverage-loss`. Resume when a representative host, an incumbent workload and an accepted budget exist.

These are explicit carry-forwards, not completed stories. F-X003's backlog row
stays `in-progress`, and the CPU raster it delivered is not claimed as a
completed acceptance.

## Dependency order and the shared resources

F-048 depends on F-045. F-101 depends on F-100. F-X003 depends on F-029,
F-045 and F-X001. All of those backlog dependencies are `done`. No backlog
cell in this sprint names another story in this sprint, so no story begins
blocked by that column. The approved F-048, F-101 and F-X003 plans still
run serially, because they share `View`, the canvas binding and the command
protocol.

F-050 in S17 depends on F-048. F-102 and F-103 in S17 depend on F-101.
F-X004 depends on F-X003 and is not this sprint.

**One mapping, one LUT chain, and one boundary.** F-048 and F-X003 both
place and draw one stack frame, so they share the canvas, world and index
mapping and the LUT chain in `ocelli-pixel`. F-101 is the only crossing
those two may use. The designs land before any of them is implemented.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is
pinned exactly and agents confidently emit APIs that have not existed for
two years**, so every wgpu call is checked against the pinned version's own
documentation rather than from memory. Treat GPU code that compiles first
try with suspicion.

No patient data enters a prompt, tracked file, fixture, log, error or
commit. The ignored corpus remains behind `corpus/manifest.tsv` and its
generators.

A feature that cannot run on the resolved tier **reports unavailable**. It
never quietly produces a different result, which is HLD section 31's rule
generalised by D-07.

## Audit intake and preparation, 2026-10-18

The S13 to S15 audit is `.claude/reviews/S13-S15-audit-pass-1.md`. F-X022
and F-X023 add 3 estimated engineer-weeks to the original 14 in this sprint.
xAI model implemented the shared VOI-sequence refusal and the closure
inventory and backlog-status checks. F-X022 is complete with a clean
independent pass in `.claude/reviews/F-X022-presentation-pass-1.md`.
F-X023 is complete after four independent passes, with the final sprint
verification pending. Its closure pass in
`.claude/reviews/F-X023-closure-pass-1.md` approved AC1 to AC3 only. The
instruction passes are in `.claude/reviews/F-X023-instruction-pass-2.md`,
`.claude/reviews/F-X023-instruction-pass-3.md` and
`.claude/reviews/F-X023-instruction-pass-4.md`, and pass 4 is clean on the
instruction remediation. Those passes are not a whole-sprint review. The
implementer does not supply the whole-sprint review.
F-048 and F-101 are complete. F-101's remediation closed the pass 4 to pass 7
findings and pass 7 is clean. F-X003 is carried because its representative
GPU-less host was unavailable and its coverage verdict is `coverage-loss`.
Tier A against C and the GPU-less interactive and idle costs stay unmet.

The execution order for those three remains shared coordinate mapping, common
boundary, then CPU raster and final integration. The allocation's absence of
an intra-sprint dependency does not establish that those consumers can be
implemented independently. F-X003 requires actual tier A/C evidence, a bounded
stack candidate producer or explicit replanning of the later F-X021
dependency, and measured GPU-less interactive and idle costs. No
representative CPU-baseline host is available, so that acceptance criterion
remains unmet. No measurement was invented here.

Remote issue and PR refresh returned no open entries. No external action was
taken. The carried LINEAR finding, ICC, second-machine evidence and release
notice blocker keep their existing disposition. No tolerance or baseline moved.

## Codex takeover, 2026-10-19

The operator assigned remaining implementation to Codex and approved minimal
resident-stack initialization in F-101. Independent review is explicitly
deferred. F-048, F-101 and F-X003 remain in progress. F-X022 and F-X023 remain
pending. No feature is completed and S16 remains open.

The scroll benchmark now selects a different resident key through the real
wasm boundary. Release worker benchmarks exercise first frame, pointer batch,
scroll, changed-frame cine and idle scheduling. A CPU collector exists, but
there is still no representative GPU-less incumbent host or accepted budget.
The native tier A/C candidate comparison has zero pixel failures across its
67 measured views and 17 unmeasured views. Its coverage gate is still red.
Cornerstone parity remains red. Details, current checks and the independent
review handoff are in `.claude/reviews/S16-codex-completion-evidence.md`.

The takeover's fresh issue and PR refresh both returned HTTP 401. Current
remote intake is unperformed. The earlier empty lists remain historical.
