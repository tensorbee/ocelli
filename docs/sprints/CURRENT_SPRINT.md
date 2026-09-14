# Current sprint, S09

**Milestone**: M2, DICOM ingest and the pixel pipeline.
**Branch**: `sprint/s09`
**Opened**: 2026-09-13
**Goal**: Complete the per-frame geometry and calibration rules F-019 deferred,
resolve the two P0 kill-criterion codecs, HTJ2K and JPEG-LS, and assemble the
full LUT chain through presentation and inversion on top of the modality and
VOI arithmetic F-018 already established.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-020 | E3.5 | Per-frame functional groups, gantry tilt, spacing calibration | Rust | 3w | pending |
| F-027 | E4.5 | HTJ2K: spike, then integrate or bridge to openjph wasm | Rust | 5w | pending |
| F-028 | E4.6 | JPEG-LS: decide CharLS bridge vs pure Rust, then integrate | Rust | 5w | pending |
| F-029 | E4.7 | LUT chain: modality, VOI (linear / exact / sigmoid), presentation, invert | Rust | 3w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S09 / {print $2, $9}'
```

## What this sprint is

S09 finishes M2's ingest and codec surface. F-020 takes the per-frame
functional-group evidence F-019 projected losslessly and turns it into derived
geometry, which is the first story in the programme that computes a coordinate
from a tag rather than retaining one. F-027 and F-028 are the two remaining
Transfer Syntax UIDs the codec registry claims, and both carry a **P0 kill
criterion** in the allocation, so each begins with a decision rather than an
implementation. F-029 completes the LUT chain by adding the presentation and
inversion stages to the modality and VOI arithmetic that already exists.

This sprint does not move LUT arithmetic into a shader and does not create a
rendering API. Palette colour and ICC execution stay out of scope unless
F-029's design plan brings them in explicitly.

## What is carried in

- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is unfinished M1 evidence and it
  is not a dependency of any S09 story.
- S08 closed F-018, F-019, F-024, F-025 and F-026. Every declared S09
  prerequisite is therefore `done`, and no S08 implementation story is carried
  into this sprint.

## Two things S08 changed that the story text predates

**The allocation's note on F-028 says "no credible pure-Rust path", and that is
now measurably out of date.** F-026 vendored `ritk-codecs` 0.6.0 under deviation
**D-20**, and that package already ships `pub mod jpeg_ls`, a native ISO 14495-1
codec whose own module documentation claims both lossless and near-lossless,
which is exactly the pair `.80` and `.81` that F-028 must decide. It is the same
no-unsafe published package whose archive identity, VCS identity, licence texts
and no-Rayon native and wasm graphs the pins gate already enforces. F-028 must
evaluate that path first and record the measured result. The CharLS bridge is
still a legitimate outcome, but choosing it without first measuring the code
already vendored in this repository would be a decision made from a stale note.

**The allocation's note on F-027 is not similarly weakened.** There is no HTJ2K
in the vendor tree, so the spike keeps its full risk and the openjph bridge
keeps its full weight. Both stories are P0, and only one of them got cheaper.

## The defect class this sprint is exposed to

**F-020 is the first story that derives a coordinate rather than retaining
one.** Every prior ingest story was judged on losslessness and this one cannot
be, because a derived value has no source to compare against. Gantry tilt,
spacing calibration and per-frame plane position all produce numbers that look
plausible at any magnitude. Spacing taken from the wrong attribute, an Image
Orientation pair that is not orthonormalised, a tilt sign convention inverted,
or Pixel Spacing preferred where Imager Pixel Spacing is the calibrated one, all
yield a geometry that renders and measures wrong without failing. The
hand-computed fixture requirement is load-bearing here and the cited PS3.3
section belongs beside each derivation.

**F-029 is exposed to building the LUT chain a second time.** HLD section 18
requires that arithmetic to exist exactly once, and F-018 already implemented
modality precedence and the `LINEAR`, `LINEAR_EXACT` and `SIGMOID` functions
with the hand-computed section 18.3 fixtures behind them. F-029's story title
names those stages again, and the title is not a licence to reimplement them.
The new work is the presentation stage and inversion.
`docs/lld/pixel-pipeline.md` already records that inversion must happen
**exactly once**, so a presentation stage that inverts alongside a VOI stage
that also inverts is the specific defect, and it is invisible on any symmetric
window.

**Both codec stories are exposed to the stored-domain boundary.** The vendored
`decode_jpeg_ls_fragment` returns `Vec<f32>`, and the stored domain for these
syntaxes is integer. An `f32` that carries a 16-bit stored value exactly is
correct, and one that does so for every value except the ones near the top of
the range is the quietly-wrong pixel this repository exists to catch. Whichever
path F-028 selects must prove the round trip over the full stored range rather
than over a sample. Near-lossless is a second trap: `.81` is lossy by design, so
"decoded output differs from the source" is expected there and is not evidence
of a defect, while the same statement about `.80` is.

**Capability must stay explicit for a kill-criterion codec.** If F-027 or F-028
concludes its syntax cannot be supported, the registry reports that UID
unavailable. It does not fall back to another decoder and it does not register a
decoder that refuses at call time while claiming capability at registration.

## What done means

- **F-020** derives per-frame geometry, gantry tilt and calibrated spacing from
  the F-019 projection, with each derivation citing its PS3.3 section and
  carrying a hand-computed fixture. Attribute precedence between calibrated and
  uncalibrated spacing is proven rather than assumed, and a frame whose geometry
  cannot be derived is refused rather than defaulted.
- **F-027** records a measured HTJ2K decision before any integration. Whichever
  way it resolves, the outcome is written down with the evidence that produced
  it, and the three corpus rows `.201`, `.202` and `.203` are either decoded and
  compared against corpus truth or reported unavailable.
- **F-028** records a measured JPEG-LS decision that begins from the already
  vendored `ritk-codecs` JPEG-LS module rather than from the allocation's stale
  note. The selected path decodes `.80` and `.81` against corpus truth on native
  and wasm, or reports them unavailable.
- **F-029** adds the presentation and inversion stages to the single existing
  LUT chain without duplicating the modality or VOI arithmetic. Inversion is
  proven to apply exactly once, with a fixture that would fail if it applied
  twice, which a symmetric window will not produce.
- Every concrete decoder added this sprint has a controlled mutation observed
  red for the claimed reason, and the codec benchmark covers it.
- `wasm-bindgen` remains confined to `ocelli-wasm`, and no render-loop or
  network boundary is added by these worker-side modules.

## Dependency order

F-020 depends on F-019, F-027 depends on F-026, and F-028 and F-029 depend on
F-023. All four prerequisites are `done`, so no story begins blocked.

F-027 and F-028 both populate `ocelli-codec` and the same runtime registry, so
their design plans must settle registry ownership, dependency features and
test-file ownership before concurrent implementation, as F-024, F-025 and F-026
had to. F-029 touches `ocelli-pixel`, which F-018 owns as built, so its design
plan states which existing items it extends and which it leaves untouched
before any code is written. F-020 is the only story in the set with no shared
crate contention.

Both P0 stories should reach their decision before the sprint commits to their
implementation weeks. A kill criterion answered late is a kill criterion that
cost the sprint either way.

## Standing expectations

The HLD is authoritative. A design-plan departure is recorded in
`docs/hld/DEVIATIONS.md`, never improvised in implementation.

No patient data enters a prompt, tracked file, fixture, log, error or commit.
The ignored corpus remains behind `corpus/manifest.tsv` and its generators.

The existing `Decoder` trait is the declared extension point. No second codec
abstraction, forwarding wrapper, feature flag without a named user, or generic
parameter with only one present instantiation is added.

Pixel arithmetic, frame boundaries, and Transfer Syntax UID dispatch remain
observable. An unsupported route is unavailable or refused according to its
contract and is never silently treated as the common path.
