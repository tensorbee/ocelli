# F-012, Candidate comparison gate contract and verification plumbing

**Status**: approved
**Epic ref**: E2.4
**Sprint**: S05
**Estimate**: 3w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a comma because `scripts/prose_check.py` covers this plan. No
other word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/08-validation-architecture.md`, section 11

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.
>
> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic,
> and the same corpus a regulatory submission would want to see.

### `docs/hld/22-testing-and-tolerance.md`, sections 25 and 25.1

> | **Layer** | **What it proves** | **Where it comes from** |
> |----|----|----|
> | Unit and property | LUT arithmetic, geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
> | Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
> | Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

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

### `docs/hld/11-decision-log.md`, decision D7

> | D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |

### `docs/hld/DEVIATIONS.md`, D-04

> | D-04 | section 11, "Every pull request renders the corpus in CI" | CI runs no GPU build and no GPU test. The corpus renders locally, in `/verify`, and is required green before a push | Operator constraint, GPU CI minutes are expensive. See the risk below, this one is not free. | Bootstrap |

The mechanical replacement transcribed from the same deviation is:

> 1. **Outside the bootstrap exception below, `/verify` runs the oracle locally
> and `push` is refused without it.** `scripts/verify_ledger.py` records the
> corpus result against the exact head commit. A push whose head has no green
> corpus record for it is refused by `.githooks/pre-push`.
> 2. **CI asserts the ledger, without a GPU.** The CI floor re-reads the ledger
> entry for the pushed head and fails when it is missing, stale or red.
> 3. **A GPU corpus run is available on manual dispatch** for a release or when
> a divergence is suspected.

## What the specification does not cover

1. Section 11 requires the render in pull-request CI. D-04 expressly forbids
   that path and substitutes local verification plus ledger validation. The
   operator retained D-04 in the S04 design round.
2. The HLD does not define coverage loss. The current comparator has four
   outcomes and the current corpus reports measured and unmeasured views.
   `CURRENT_SPRINT.md` supplies the missing acceptance rule: a failure and a
   coverage drop both fail, with different output.
3. The HLD does not define whether unsupported rows, declared volume refusals,
   or class-two views with no written threshold belong in the verdict count.
4. The HLD does not define a machine-readable gate report or exit-code
   vocabulary.
5. No Ocelli candidate renderer exists yet. The S05 design round split the
   original story rather than comparing the reference with itself and calling
   that a candidate verdict. F-012 lands the explicit directory contract,
   coverage result and ledger fields. F-X021 activates them as a required
   local gate after F-052 supplies the last view kind in the current oracle.

## Approach

Under the retained D-04 arrangement, F-012 makes a real candidate verdict
representable and attestable without pretending one exists today. F-X021 makes
that verdict mandatory after the Ocelli renderer can emit every current oracle
view. Neither story claims that CI rendered a corpus it did not possess.

1. Add an explicit comparator `gate` command over one reference directory and
   one candidate directory. Both flags are required and the two resolved paths
   must differ. It runs the existing structural, input, parameter, geometry and
   pixel checks once. It writes the existing per-view report plus a run-level
   coverage block.
2. Define `claimedVerdictViews` as views whose written predicate was evaluated,
   which means `pass + fail`. Keep `unmeasured`, `absent`, unsupported source
   rows and declared volume refusals in separate named counts. Never report
   `views examined` as `views judged`.
3. Treat a view entering or leaving the committed unmeasured census as a
   coverage change. A new unmeasured or absent view is `coverage-loss`. A
   predicate failure is `comparison-failure`. Both exit nonzero and their
   summaries use different labels.
4. Keep the existing identity and declared mutation catalogue as instrument
   self-tests. The gate path compares the supplied directories and does not
   silently default a missing candidate to the reference.
5. Extend the verification record with the comparator report digest, verdict
   and claimed verdict count when `--comparison-report` is supplied. Refuse a
   report that is red, malformed or claims zero judged views. Emit those fields
   in the commit trailer so CI can validate them without the local report.
6. Add `--require-comparison` to the ledger assertion and commit check. It is
   not enabled by the ordinary feature or sprint profiles until F-X021 lands,
   because requiring output no renderer can produce would disable every push.
7. Add standing probes for a missing explicit candidate, a candidate resolving
   to the reference directory, a one-pixel comparison failure, a missing view,
   a new unmeasured view, a malformed report and a record that says zero
   failures after judging zero views.

No tolerance changes are part of this story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none in the comparator. The reference renderer keeps its existing allocation behaviour
- unsafe: none
- Tier A (WebGPU): n/a. The gate consumes renderer output and does not select an Ocelli tier
- Tier B (WebGL2): n/a. The pinned reference currently renders through SwiftShader and records that fact
- Tier C (CPU): n/a. The directory contract can gate tier C output when it exists

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Run-level accounting distinguishes judged, unmeasured, absent and unsupported views | `tools/oracle/src/report.rs` |
| unit | Comparison failure and coverage loss produce distinct nonzero results | `tools/oracle/src/bin/ocelli-compare.rs` |
| golden | An explicit candidate directory is judged against a full reference run and preserves the exact unmeasured census | `ocelli-compare gate` against controlled output directories |
| browser | The existing reference path still reaches, decodes, presents and reads back every applicable row or records a declared refusal | `tools/oracle/run.mjs` and the oracle gate |
| property | Removing any declared view, or moving any view into `unmeasured`, cannot leave the run green | `tools/oracle/tests/coverage.rs` |
| unit | A green comparison report records its digest and positive claimed-verdict count, while malformed, red and zero-judgement reports are refused | `scripts/verify_ledger.py` guard probes |
| conformance | The corpus still covers each transfer syntax the registry claims | existing `corpus` gate |

No new pixel or geometry arithmetic is introduced, so this story needs no new
hand-computed fixture.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` contains no `Covered by` column and no row
keyed to E2.4. This story makes parity evidence binding rather than adding a
viewport, tool, blend mode, LUT function, transfer syntax, segmentation
representation, event or adapter.

## Deviations

D-04 is load-bearing. No other deviation is anticipated.

## LLD impact

- `docs/lld/comparator.md` records the gate command, coverage vocabulary and
  report schema.
- `docs/lld/oracle.md` records where the gate runs and the exact claim CI can
  make under D-04.

## Anticipated write set

Under the standing D-04 arrangement:

- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/src/sidecar.rs`
- `tools/oracle/tests/coverage.rs`, new
- `bin/ocelli.sh`
- `scripts/verify_ledger.py`
- `scripts/guards/catalogue.py`
- `scripts/guard_probe.py`
- `docs/runbooks/guard-verification.md`
- `docs/lld/comparator.md`
- `docs/lld/oracle.md`
- `docs/sprints/allocation.json`
- `docs/sprints/SPRINT_PLAN.md`

If D-04 is retired, also `docs/hld/DEVIATIONS.md`. The required workflow runner
and corpus provisioning paths are deliberately not claimed as exact until the
operator chooses that execution model.

## Dependency and conflict notes

- F-011 is done and supplies the comparator, census and mutation catalogue.
- F-013 and F-015 have landed. Their current report and hash contracts are the
  base F-012 extends.
- F-X010 and F-X019 have landed. F-012 extends their existing ledger and guard
  contracts without changing the CI workflow or pre-push requirement.
- The oracle is a serial runtime resource.

## Open questions

None. The operator retained D-04 and approved the split in the S05 design
round. F-X021 preserves activation of the original full-corpus candidate claim.
