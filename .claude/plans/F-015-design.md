# F-015, Stable render-hash emission from the comparator

**Status**: approved
**Epic ref**: E2.7
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a comma because `scripts/prose_check.py` covers this plan. No
other word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/27-phase1-hooks.md`, section 38

> Everything above is post-parity except these. Each costs a few weeks now and
> a rewrite later, which is the only reason they appear in a parity plan at all.

> | **Hook** | **Story** | **Now** | **Later** |
> |----|----|----|----|
> | Stable render hashes from the oracle | E2.7 | 2 wk | Little - but free here, since E2 computes them anyway |

> **Phase 1 grows from 382 to 397 engineer-weeks** to carry these. That is the
> whole cost of keeping every option in Part III open.

### `docs/hld/11-decision-log.md`, decision D14

> | D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |

### `docs/hld/08-validation-architecture.md`, section 11

> The harness pushes the same study through both stacks and compares frames
> within a written per-modality tolerance, with metadata diffed alongside
> pixels because a wrong rescale slope can still produce a plausible image.

### `docs/hld/22-testing-and-tolerance.md`, section 25

> | **Layer** | **What it proves** | **Where it comes from** |
> |----|----|----|
> | Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |

### Existing oracle output contract, `docs/lld/oracle.md`

> `<id>.raw`, the RGBA8 bytes the PNG encoder never touched. **The comparator
> reads these and never the PNG**, because the PNG encoder is a second
> transformation and comparing its output would measure it.

## What the specification does not cover

1. The HLD does not define the hash algorithm, byte domain, framing, ordering,
   version or whether hashes are per-view or per-run.
2. The HLD does not say whether metadata and presentation state enter the
   render hash. E36.2 later builds an attestation from presentation state plus
   output hash, which argues for keeping those as separate inputs here.
3. The HLD does not define how padding, PNG encoding, report ordering,
   timestamps or environment identity are excluded.
4. D14 rejects a bit-exact reproducibility claim. A stable hash therefore
   identifies exact output bytes. It does not define an acceptable divergence
   across machines or tiers.

## Approach

1. Emit a versioned `sha256-rgba8-v1` hash for every reference and candidate
   view from the comparator. Hash the exact `.raw` bytes after the loader has
   verified their declared length and digest, before difference statistics,
   report serialisation, PNG encoding or any other reduction.
2. Domain-separate and frame the input as fixed ASCII version bytes followed
   by length-prefixed view kind, identifier, width, height, format token and
   the exact tightly packed pixel byte length and bytes. Length prefixes make
   concatenation unambiguous. Width and height prevent the same byte string
   being reinterpreted under a different shape.
3. Refuse any frame whose byte length is not exactly `width * height * 4`.
   Hash no allocation padding. The existing loader already checks this and the
   hash path reuses the checked `Frame`, not a second file reader.
4. Emit a run render hash over the ordered list of per-view records sorted by
   the comparator's canonical identifier. Each entry is length-prefixed and
   carries kind, id and per-view hash. An absent or duplicate view is already a
   run failure and cannot be hidden by aggregation.
5. Keep input identity, presentation parameters, environment identity and
   comparator verdict beside the render hash, not inside it. F-151 can later
   attest `presentation state + output hash` without redefining what output
   means. F-X011 can compare equal hashes across environments or measure the
   pixels behind unequal hashes.
6. Add hash fields to the stable JSON report and concise stdout. Do not place
   patient-derived frames or PNGs in git. A digest is evidence attached to an
   ignored run and does not make that output publishable.
7. Before the story is claimed, run the existing one-pixel mutation and record
   the baseline hash, mutated hash and red comparator verdict in the progress
   note. A mutation that changes comparator statistics but not the hash fails
   the story.
8. Pin a tiny, hand-authored byte fixture to exact expected SHA-256 strings.
   Also test that changing one pixel, dimensions, kind or identifier changes
   the corresponding digest, while report key order and elapsed time do not.

No tolerance changes and no cryptographic signature or attestation format are
part of this story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Hashing is host-side after readback and reuses loaded frame bytes
- unsafe: none
- Tier A (WebGPU): full as an output identity when a tier A candidate directory exists
- Tier B (WebGL2): full. The current reference output is WebGL2 through SwiftShader
- Tier C (CPU): full as an output identity when a tier C candidate directory exists

The algorithm is identical for all tiers. Equal hashes mean equal declared
RGBA8 output bytes. Unequal hashes require the comparator's measured
divergence and never silently resolve to a different tier-specific rule.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | `sha256-rgba8-v1` has exact expected digests for a small tightly packed RGBA8 frame | `tools/oracle/tests/render_hash_fixture.rs` |
| unit | Shape, kind and identifier are domain-separated and report ordering cannot affect the hash | `tools/oracle/src/render_hash.rs` |
| property | Flipping any one byte changes the per-view digest and changes the run digest | `tools/oracle/tests/render_hash_fixture.rs` |
| golden | The full corpus emits one hash per declared view and one aggregate per side | comparator identity run |
| golden | A declared one-pixel mutation changes both hashes and is observed red | comparator mutation catalogue |

The hash performs no pixel arithmetic, so HLD 27.2 R3 does not require a DICOM
fixture. The literal byte fixture independently proves the byte contract.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column and no row
keyed to E2.7. Section 38 names E2.7 as a Phase 1 hook rather than a parity
surface row.

## Deviations

None. D14 is followed by keeping exact identity separate from the measured
divergence claim.

## LLD impact

`docs/lld/comparator.md` records the versioned hash byte contract, report
fields and what equality does and does not claim.

## Anticipated write set

**Create**

- `tools/oracle/src/render_hash.rs`
- `tools/oracle/tests/render_hash_fixture.rs`

**Modify**

- `tools/oracle/src/lib.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/src/mutations.rs`
- `tools/oracle/compare-out/compare.json`
- `docs/lld/comparator.md`

## Dependency and conflict notes

- F-011 is done and already verifies each frame's SHA-256 while loading it.
  F-015 must reuse the loaded frame and must not add a second raw-file path.
- F-015 overlaps F-012 and F-013 in the comparator binary, report and LLD.
  Run them serially.
- F-X011 consumes these hashes as exact-identity signals but must not turn hash
  inequality into a new tolerance.
- F-151 consumes the output hash later and owns presentation-state attestation.

## Open questions

None. Emit per-view and run-level hashes. Use the domain-separated, shape-aware
per-view format so identical bytes cannot silently identify different frames.
