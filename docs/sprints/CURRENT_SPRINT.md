# Current sprint, S08

**Milestone**: M2, DICOM ingest and the pixel pipeline.
**Branch**: `sprint/s08`
**Opened**: 2026-09-10
**Goal**: Add the image-plane, pixel, modality-LUT, VOI-LUT, multiframe, and
enhanced SOP contracts, then populate the codec registry with JPEG, RLE,
Deflate, raw endian, and JPEG 2000 decoders validated against the corpus.

| F-ID | Epic ref | Story | Layer | Est | Status |
|------|----------|-------|-------|-----|--------|
| F-018 | E3.3 | Image plane, pixel, modality-LUT and VOI-LUT modules | Rust | 3w | pending |
| F-019 | E3.4 | Multiframe and enhanced SOP class handling | Rust | 4w | pending |
| F-024 | E4.2 | JPEG baseline / extended / lossless via jpeg-decoder | Rust | 3w | pending |
| F-025 | E4.3 | RLE, deflate, raw little- and big-endian | Rust | 2w | pending |
| F-026 | E4.4 | JPEG 2000 via openjp2, validated against the corpus | Rust | 4w | pending |

**The Status column above is hand-typed and nothing derives it, so it goes
stale.** `docs/sprints/BACKLOG.md` is the authority. Read the two together:

```bash
grep -c '^| F-[0-9]' docs/sprints/CURRENT_SPRINT.md
grep '^| F-' docs/sprints/BACKLOG.md | awk -F'|' '$4 ~ / S08 / {print $2, $9}'
```

## What this sprint is

S08 turns the explicit ingest and codec-dispatch contracts from S07 into the
first usable pixel pipeline. It defines image-plane and stored-pixel evidence,
implements the modality and VOI stages once in Rust, represents multiframe and
enhanced objects without losing where a value came from, and registers the
first concrete decoders behind the existing caller-provided-buffer boundary.
The result must remain identical on native and wasm targets and must be
measured against the existing synthetic and real corpus evidence.

This sprint does not add the F-020 per-frame functional-group, gantry-tilt, or
spacing-calibration work. It does not add JPEG-LS or HTJ2K, which remain the
separate F-028 and F-027 decisions. It also does not move LUT arithmetic into a
shader or create a rendering API.

## What is carried in

- **F-X011** remains pending because its acceptance evidence requires a second
  physical machine and none is available. It is unfinished M1 evidence, but it
  is not a dependency of any S08 story.
- S07 closed F-017 and F-023, which are the declared prerequisites for every
  S08 story. No S07 implementation story is carried into this sprint.

## The defect class this sprint is exposed to

**A decoded pixel can look plausible while being numerically or structurally
wrong.** Stored-value extraction must respect signedness, Bits Stored, High
Bit, byte order, samples, planar configuration, frame bounds, and exact output
length. A decoder that produces the expected dimensions but shifts bits,
swaps channels, accepts trailing compressed data, or partially writes its
destination is not correct.

The LUT chain is exposed to boundary comparisons, precedence, and rounding.
Modality LUT Sequence takes precedence over rescale slope and intercept. The
LINEAR and LINEAR_EXACT VOI functions differ by a half and a one at their
boundaries, so visual inspection cannot establish correctness. The
hand-computed HLD section 18.3 values and controlled mutations must do that.

Multiframe objects add a source-precedence defect class. Top-level, shared,
and frame-specific declarations must not be silently conflated, and frame
indices or fragments must not drift across frame boundaries. F-019 must leave
the richer per-frame geometry and calibration rules explicitly owned by F-020
rather than embedding an incomplete second interpretation.

Codec portability is part of correctness. Native-only registration, a C
library that cannot build for `wasm32-unknown-unknown`, or a dependency feature
that differs silently across targets would make the same Transfer Syntax UID
mean different things by platform. Capability must stay explicit when a
decoder is unavailable.

## What done means

- **F-018** provides typed image-plane and stored-pixel evidence plus one Rust
  implementation of modality and VOI mapping. It proves Modality LUT
  precedence and the LINEAR, LINEAR_EXACT, and SIGMOID boundary rules with
  hand-computed fixtures before any shader consumes the parameters.
- **F-019** handles multiframe and enhanced SOP input through the lossless
  metadata model, with explicit frame counts, frame selection, and source
  provenance. Truncated, inconsistent, and out-of-range frame declarations
  are refused without claiming the F-020 geometry-calibration scope.
- **F-024** registers JPEG baseline, extended, and lossless decoding through
  the F-023 `Decoder` contract. Each supported UID writes exactly one checked
  frame into caller-provided output and is compared with corpus truth.
- **F-025** registers RLE, Deflate, and raw little-endian and big-endian paths.
  It proves segment and stream termination, endian and signed-value handling,
  exact output length, and refusal without partial success.
- **F-026** registers JPEG 2000 through the approved openjp2 path and validates
  every claimed syntax against the corpus on native and wasm. Dependency,
  memory, and error boundaries remain explicit, with no unreviewed fallback.
- Every concrete decoder has a controlled mutation observed red for the
  claimed reason. The codec benchmark gains a real subject before any
  optimisation claim is made.
- `wasm-bindgen` remains confined to `ocelli-wasm`, and no render-loop or
  network boundary is added by these worker-side modules.

## Dependency order

F-018 and F-019 depend on F-017, which is done. F-024, F-025, and F-026 depend
on F-023, which is done. The generated allocation declares no dependency among
the five S08 stories, so none begins blocked.

F-018 and F-019 both consume metadata and may touch shared DICOM module types.
F-024, F-025, and F-026 all populate `ocelli-codec` and the same runtime
registry. Their design plans must settle shared public types, registry
ownership, dependency features, and test-file ownership before concurrent
implementation. F-026 must also prove that the selected openjp2 path is viable
on wasm before treating its decoder surface as settled.

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
