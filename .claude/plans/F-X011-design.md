# F-X011, Cross-machine reference determinism, and what the oracle claims about it

**Status**: approved
**Epic ref**: Y1.6
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a comma because `scripts/prose_check.py` covers this plan. No
other word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/11-decision-log.md`, decision D14

> | D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |

### `docs/hld/08-validation-architecture.md`, section 11

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.

### `docs/hld/22-testing-and-tolerance.md`, section 25.1

> Write it down once and hold it. Tuning tolerance per failure is how a suite
> stops meaning anything.

> - **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
> LSB on at least 99.9% of pixels, zero pixels differing by more than 2.

> - **Systematic bias, monochrome:** signed mean difference over the
> informative region within 0.1 of one display code, evaluated only where
> input identity, declared parameters and geometry already agree.

> - **Colour and ultrasound:** perceptual difference below a stated threshold,
> because chroma subsampling and YBR conversion legitimately differ.

> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
> a quarter pixel.

> - A tolerance change is a pull request with a rationale, reviewed like code.

### `docs/lld/oracle.md`, current determinism claim

> Every gate run renders the corpus **twice, in one browser and one page**, and
> requires identical digests. That is the claim this story can support: stable
> output on one machine and one browser build. It is not a claim of
> cross-machine reproducibility, and D14 is why the stronger claim is not made.
> `run.json` carries the host and the adapter string, so a cross-machine
> difference is attributable rather than mysterious.

### Allocation note for Y1.6

> Measure it on a second machine and a second Chromium build, publish the
> observed bound, and make run.json carry whatever identifies the difference.

## What the specification does not cover

1. D14 says to measure divergence but gives no cross-machine experiment,
   sample count, environment fingerprint or publication format.
2. HLD 25.1 is a candidate-versus-reference tolerance. It does not say that an
   observed reference-versus-reference maximum becomes a future acceptance
   threshold. This plan treats it as measurement only.
3. The HLD states no colour metric or threshold, so cross-machine class-two
   output can be reported but cannot be declared within tolerance.
4. The HLD does not say which two machines or Chromium builds constitute the
   initial evidence.
5. The HLD does not say whether OS patch, architecture, Node, Playwright,
   Chromium revision, SwiftShader renderer and browser capabilities all belong
   in environment identity.

## Approach

1. Define a reference environment fingerprint in `run.json`. It records
   platform, release, architecture, Node version, Playwright version, Chromium
   user agent and executable revision, cornerstone package pins, renderer and
   software-rasterizer detection, effective backend, device pixel ratio and
   relevant WebGL capabilities. Hostname remains diagnostic and is excluded
   from the fingerprint because it is not causal.
2. Add a `reference-determinism` comparator command that takes two complete
   oracle output directories. It first requires identical manifest and render
   input digests, package pins, declared view sets and successful same-machine
   determinism on both sides. A mismatch there is an incomparable experiment,
   not renderer divergence.
3. Compare F-015's exact render hashes when available. Equal hashes establish
   byte identity for that view. Unequal hashes trigger the existing frame
   statistics and geometry comparison symmetrically. Neither environment is
   labelled `ours` or `reference`.
4. Publish a tracked, patient-free measurement record in
   `tools/oracle/reference-determinism.json`. It names both environment
   fingerprints, input digests, view counts, exact-match counts, class-one
   maximum and percentile statistics, signed informative bias, geometry
   maxima, and class-two descriptive statistics. It records no pixels, PNGs,
   DICOM attribute values or real-row sidecar values.
5. The published bound is the observed envelope plus the sample description.
   It is not copied into `tolerance.rs`, does not turn a hash mismatch green,
   and does not claim general reproducibility beyond the measured pair.
6. Run the full corpus twice on machine A with Chromium build A and twice on a
   genuinely different machine B with build B, both with SwiftShader forced.
   Preserve ignored output directories long enough to compare them. The
   tracked record is produced from those completed runs.
7. Add strict validation that both fingerprints differ in machine identity and
   Chromium build for this story's experiment. Comparing copied directories or
   two runs from one browser build must refuse to publish a cross-machine row.
8. Update the oracle's wording to state exactly what was observed. Keep the
   standing same-machine gate as exact identity. Do not make future
   cross-machine exactness a gate unless evidence and policy later justify it.

No tolerance changes are part of the experiment. If the result motivates a
new acceptance bound, that is a separate design-plan decision reviewed like
code.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Measurement consumes completed host-side output
- unsafe: none
- Tier A (WebGPU): n/a. The pinned reference run uses its recorded backend rather than an Ocelli capability tier
- Tier B (WebGL2): measured. The current reference uses WebGL2 through SwiftShader
- Tier C (CPU): n/a. This story measures the cornerstone reference, not Ocelli's CPU renderer

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Environment fingerprints include causal version and renderer fields and exclude hostname | `tools/oracle/tests/environment_test.mjs` and Rust report tests |
| unit | Incomparable inputs, equal fingerprints and missing same-machine determinism are refused | `tools/oracle/src/reference_determinism.rs` |
| property | JSON key order and hostname changes do not alter a fingerprint, while any causal field change does | `tools/oracle/tests/environment_test.mjs` |
| golden | Two complete runs from different machines and Chromium builds produce the tracked observed envelope | `bin/ocelli.sh compare reference-determinism ...` |
| browser | Each source run forces SwiftShader and records the effective renderer and browser build | `tools/oracle/run.mjs` |

No new DICOM or pixel arithmetic is introduced. Existing comparator statistics
are reused, so no duplicate fixture arithmetic is added.

## Parity surface covered

None. Y1.6 is a sprint follow-up and `docs/hld/B-parity-surface.md` has no row
or `Covered by` value for it.

## Deviations

None anticipated. D14 is implemented by publishing a measured and scoped
envelope instead of claiming bit-exact cross-machine reproducibility.

## LLD impact

- `docs/lld/oracle.md` records the environment fingerprint, experiment and
  exact scope of the determinism claim.
- `docs/lld/comparator.md` records the symmetric reference-determinism command
  and measurement schema.

## Anticipated write set

**Create**

- `tools/oracle/src/reference_determinism.rs`
- `tools/oracle/reference-determinism.json`
- `tools/oracle/tests/environment_test.mjs`
- `tools/oracle/tests/reference_determinism.rs`

**Modify**

- `tools/oracle/run.mjs`
- `tools/oracle/src/lib.rs`
- `tools/oracle/src/sidecar.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/src/bin/ocelli-compare.rs`
- `bin/ocelli.sh`
- `docs/lld/oracle.md`
- `docs/lld/comparator.md`

## Dependency and conflict notes

- F-010 supplies the complete reference output and same-machine determinism.
- F-015 should land first so exact output identity has one versioned contract.
  F-X011 can fall back to existing sidecar SHA-256 only if that ordering
  changes.
- F-X011 overlaps F-012 and F-015 in the comparator binary, report, shell
  wrapper and LLD. Run them serially.
- The two source runs must not execute concurrently on one oracle host.
- A second physical machine and a second Chromium build are external evidence,
  not something a local unit test can simulate honestly.

## Open questions

None in the design. The operator has no second machine yet and parked this
story before implementation. When resumed, the tracked measurement may name OS
release and architecture, and even byte-identical results retain the scoped
wording "observed identical for these two environments".
