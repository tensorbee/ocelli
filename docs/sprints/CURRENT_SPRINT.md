# Current sprint, S11

**Milestone**: M3, the cache and the renderer.
**Branch**: `sprint/s11`
**Opened**: 2026-09-14
**Goal**: Open M3 by standing up the two things every rendered frame needs, a
budgeted cache and a real GPU device, and then writing the LUT chain's shader
against the arithmetic `ocelli-pixel` already owns.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-031 | E5.1 | `ocelli-cache`: budgeted LRU across encoded, decoded and GPU tiers | Rust | 4w | pending |
| F-037 | E6.1 | `ocelli-render`: device init, capability tiering, device-lost recovery | Rust | 4w | done |
| F-041 | E6.5 | WGSL LUT-chain shader | Rust | 4w | done |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S11 / {print $2, $9}'
```

## What this sprint is

**This is the sprint the GPU stops being a plan.** Ten sprints built ingest,
codecs and the pixel pipeline, and every one of them ran on the CPU. S11 creates
the first long-lived `wgpu::Device`, gives the cache a budget to respect, and
writes the first shader. Nothing renders a corpus frame end to end at the close
of this sprint, and that is later business: F-038's render graph in S12, then
F-040's texture upload path and F-039's OffscreenCanvas in S13, are what turn
these three components into a frame.

The three stories are close to independent. F-031 touches `ocelli-cache` and no
GPU at all. F-037 touches `ocelli-render`'s device path. F-041 writes WGSL and
its host-side uniform. They share no source file, so they can run as a parallel
wave, **except for one resource**: any test that touches a real device
contends for it. See the serialisation note below.

## What is carried in

- **M2 closed at S10 and a release is due by the table in `docs/RELEASE.md`**,
  which maps M2 to `0.2.0`. **It cannot be taken.** `/release` step 5 refuses
  while `openjph-core` 0.1.0 carries no BSD notice material, which is D-22, and
  M1's `0.1.0` was never published either, so the namespace reservation the
  release table treats as already done is still outstanding. Neither blocks any
  story here. Both block publication, and the gap widens every milestone.
- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is unfinished M1 evidence and is
  not a dependency of anything in this sprint.
- **ICC is not implemented**, the other half of HLD section 18's stage-4 row.
  F-030 built palette and the colour transforms and left ICC, which is
  whole-slide colour management and has no corpus row carrying a profile.
- **Multi-component JPEG-LS decoding is still owed.** F-030 added the corpus
  row Appendix A gate A2 asked for, and the adapter still refuses such a frame
  cleanly. The row measures the refusal. Decoding one is a codec story.
- **The oracle still has no Ocelli renderer to compare against.** That is
  decision D7 holding rather than a gap, and **F-041 is the first story that
  moves toward closing it**, though it does not close it.

## The defect class this sprint is exposed to

**Every previous sprint could be wrong in a way a fixture catches. This one
can be wrong in a way only a second machine catches.**

**A second copy of the LUT arithmetic, living in WGSL.** This is the one the
HLD names directly, in section 18: implement it once in `ocelli-pixel` and let
the shader read the parameters. F-041 writes a shader whose whole job is to
apply `LINEAR`, `LINEAR_EXACT` and `SIGMOID`, and the obvious way to write it
is to type the three formulas into WGSL. That is the forbidden second copy, and
it is worse than an ordinary duplicate because it diverges only on hardware and
only under a pixel diff. Section 18.4's `VoiParams` struct is the specified
shape: the shader reads `center`, `width`, `slope`, `intercept`, `ymin`, `ymax`,
`fn_kind` and `invert`, and `LutChain::inverts` already resolves that last flag
exactly once on the CPU. **A shader that recomputes inversion from Photometric
Interpretation is the double-inversion defect F-029 spent a story preventing.**

**Tier B is a declared tier and nothing has ever run on it.** HLD section 7
gives tier B as WebGL2 through wgpu's downlevel profile, fragment shaders only,
no compute, no storage buffers, and a 3D-texture floor of 256 against tier A's
2048. D-14 added wgpu's `webgl` feature to `ocelli-render` precisely so tier B
can resolve in a browser at all, and that feature has cost zero bytes so far
because nothing reaches it. F-037 and F-041 are where a feature written for
tier A and never tried on tier B starts to look finished while being
unavailable on half the declared matrix.

**And tier C is CPU, which is deviation D-07 and not the HLD's.** `ocelli-pixel`
is tier C's authoritative path, so the correct tier C answer for a shader story
is that the arithmetic already exists and is not reimplemented. An omitted row
and a deliberate "no CPU path" read identically six months later.

**Device loss is a real state, not an error path nobody reaches.** A browser
drops a WebGPU device on a driver reset, a tab backgrounded too long, or an
OOM, and the specified behaviour is to recover rather than to fail the session.
An implementation that treats loss as unreachable will be correct on every
machine anyone develops on.

**The cache's budget is a promise about bytes, and `Budgeted::bytes` is where
that promise is kept or quietly broken.** HLD section 20 gives the trait and
the `Lru::insert` signature returning evicted entries so the caller can emit
events. A GPU texture whose `bytes` reports its decoded source size rather than
its allocated size makes the budget a number that does not describe memory.

## What done means

- **F-031** implements HLD section 20's `Budgeted` and `Lru<K, V>` with
  `insert` returning the evicted entries rather than dropping them, because
  F-032 surfaces those as JS events and an eviction nobody can observe is not
  one the shell can react to. Three tiers with distinct pressure, per HLD
  section 8: encoded bytes transient, decoded frames in a caller-sized LRU,
  GPU textures their own tier, because evicting a texture and evicting a frame
  have very different costs. **No allocation in the render loop**, and the
  budget is asserted in bytes against hand-computed entry sizes rather than
  against what the implementation reports.
- **F-037** creates the long-lived device. `GpuContext` already exists from
  F-008 and `resolve` already decides the tier from F-004 and F-X001, so this
  story wires the decision to a real adapter request and adds the recovery
  path. **`ocelli-render` remains the only crate permitted to create a
  `wgpu::Device`**, which `ci/check-device-ownership.sh` asserts and which this
  story must not weaken. Device loss is recovered from and the recovery is
  observable, not inferred.
- **F-041** writes the WGSL and the host-side uniform of HLD section 18.4,
  transcribed rather than reinvented, and **adds no arithmetic that
  `ocelli-pixel` does not already own**. Its evidence is that the shader's
  output agrees with `LutChain::map_into` over the section 18.3 fixture inputs,
  which is a comparison against this repository's own validated CPU path and is
  therefore weaker than an oracle verdict and stronger than a screenshot. Say
  which it is in the design plan.
- **Every one of the three declares all three tier rows**, and "n/a" is a real
  answer that an omitted row is not.
- `wasm-bindgen` remains confined to `ocelli-wasm`, pixels do not cross the
  boundary, and no story adds a second `queue.submit()` per frame.

## The size budget does NOT move this sprint, and this section said it would

**This heading read "The size budget starts moving this sprint, and that is
expected" when the sprint opened, and the S11 design round decided otherwise.**
The paragraph below it quoted `ci/wasm-size-budget.json` saying the number
bearing on Appendix A gate A4 "arrives with the render path from S11, not here",
and F-037 corrected that field to name **F-039, E6.3 in S13**. The quotation is
left here in its corrected form rather than deleted, because a sprint plan that
silently stops predicting something it predicted is worse than one that records
the change.

**The reason is that no S11 story reaches `ocelli-render` from `ocelli-wasm`.**
`crates/ocelli-wasm/Cargo.toml` names `ocelli-core` and nothing else. F-031 is
`ocelli-cache`, F-037 is the device inside `ocelli-render`, and F-041 is a
shader inside `ocelli-render`, and none of the three adds that dependency edge.
Adding it would put an entry point nothing calls into the shipped module purely
to move a number, which is the same answer F-004 was given in the S03 design
round. D-14 is unchanged and still correct from the other side: wgpu's `webgl`
feature costs zero bytes today only because `ocelli-wasm` does not reach
`ocelli-render`.

So **`ci/wasm-size-budget.json` is expected to still record 16,388 bytes at the
close of S11**, and a story re-baselining it is a finding rather than the plan.

**The recorded budget and the measured artefact already differ, and that is not
this sprint's doing.** The `wasm` gate measures 16,455 bytes against the
recorded 16,388, a drift of 67 bytes that is inside the file's 5 per cent
tolerance and so passes. It reproduces at this sprint's base commit `3b00890`,
so it predates S11 and no S11 story caused it. It is recorded here because the
sentence above is about the RECORDED number and a reader checking it against a
build would otherwise find a discrepancy with nothing explaining it.

The route stays documented for the sprint that does take it.
`scripts/pin_and_size_check.py` refuses a silent re-baseline and
`python3 scripts/pin_and_size_check.py --accept-size` is the explicit one, which
the budget file's own `note` field already names. The design plan that takes it
says what grew and why, and fills the `rebaselined` block the way F-005 did,
with a measurement against a rebuild of the base commit so the delta is the
story's and not drift. Gate A4 estimates 3 to 8 MB uncompressed with Naga
dominating and records itself as unmeasured. **A measurement that lands inside
that estimate is evidence and a measurement that lands outside it is a
finding**, and either is worth more than the estimate. Neither is a reason to
widen the tolerance.

## Dependency order and the one shared resource

F-031 depends on F-001, F-037 on F-004, F-041 on F-029. All three are `done`,
so no story begins blocked and there is no order between them.

**The GPU is an exclusive resource and the wave plan must serialise it.** Two
workers running device tests concurrently on one machine contend for the
adapter and produce timeouts that read exactly like rendering failures. F-031
needs no device and can run beside either of the others. F-037 and F-041 both
do, so they do not run concurrently with each other whatever the worker count
says.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation. **`wgpu` is pinned
exactly and agents confidently emit APIs that have not existed for two years**,
so every wgpu call in this sprint is checked against the pinned version's own
documentation rather than from memory. Treat GPU code that compiles first try
with suspicion.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

A feature that cannot run on the resolved tier **reports unavailable**. It never
quietly produces a different result, which is HLD section 31's rule generalised
by D-07.
