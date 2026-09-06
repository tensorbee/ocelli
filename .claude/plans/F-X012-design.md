# F-X012, The reference's own SIGMOID width divergence, and what D14's bound says about it

**Status**: approved
**Epic ref**: Y1.7
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/15-lut-chain.md`, section 18.2

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

### `docs/hld/22-testing-and-tolerance.md`, section 25.1

```text
Write it down once and hold it. Tuning tolerance per failure is how a suite stops meaning anything.

- **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference ≤ 1 LSB on at least 99.9% of pixels; zero pixels differing by more than 2.

- **Systematic bias, monochrome:** signed mean difference over the informative region within 0.1 of one display code, evaluated only where input identity, declared parameters and geometry already agree.

- A tolerance change is a pull request with a rationale, reviewed like code.
```

### `docs/hld/11-decision-log.md`, decisions D7 and D14

```text
| D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |
| D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |
```

### `tools/oracle/reference-divergence.json`, the existing entry

```text
"id": "sigmoid-width-below-one",
"raisedBy": "F-010",
"reachable": false,
"citation": "PS3.3 C.11.2.1.3.1",
"referenceDoes": "cornerstone3D 5.8.2's utilities.windowLevel.toLowHighRange applies LINEAR's (w - 1) / 2 to SAMPLED_SIGMOID as well as to LINEAR, so a width between 0 and 1 produces an INVERTED range: width 0.5 at centre 40 gives lower 39.75 and upper 39.25.",
"standardRequires": "C.11.2.1.3.1 gives SIGMOID as y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin, which divides by w and therefore requires only w > 0. C.11.2.1.2 requires w >= 1 for LINEAR because LINEAR divides by w - 1. The minimum width is per function and not shared, and the difference between the two is a half and a one."
```

## What the specification does not cover

The HLD says cornerstone3D is the reference and separately says D14 publishes
measured divergence. It does not say how a known PS3.3 defect in that reference
affects the bound.

This plan decides that the normative SIGMOID formula remains correct. A known
reference defect is attributed to the reference and excluded from claims about
Ocelli's divergence. It is not absorbed by widening section 25.1. The measured
reference defect is published separately with its direction and magnitude.

## Approach

1. Add a deterministic synthetic CT case with `VOILUTFunction = SIGMOID`,
   centre 40, width 0.5, and stored values on both sides of the centre. Generate
   it into ignored `corpus/data` and add only its manifest row and generator.
2. Add an independent generator fixture citing PS3.3 C.11.2.1.3.1. Assert the
   DICOM attributes exactly and compute selected expected display values from
   the transcribed formula, not from cornerstone3D or Ocelli.
3. Render the row through the normal oracle. Assert the sidecar retains
   `SIGMOID` and width 0.5. Record the observed reference output and the
   inverted low and high range the pinned reference derives.
4. Change the divergence entry to reachable with a reason naming the new row.
   Add a comparator fixture where a PS3.3-correct candidate differs from the
   reference and is attributed to `reference`, never to `ours`.
5. Keep the global monochrome tolerance unchanged. The case is a declared
   reference divergence, not evidence that one LSB is too strict. State that
   D14's Ocelli bound is against the standard-correct output for this case,
   while the separately reported cornerstone difference is not inside that
   bound.
6. Update oracle, comparator, and corpus LLDs. Update any exact corpus or view
   census that the new generated row changes, based on observed output rather
   than a hand-edited assumed count.

### Anticipated implementation write set

- `scripts/corpus_synth.py`
- `scripts/tests/test_corpus_synth.py`
- `corpus/manifest.tsv`
- `tools/oracle/reference-divergence.json`
- `tools/oracle/src/attribution.rs`
- `tools/oracle/tests/params_test.mjs`
- `tools/oracle/compare-expectations.json`, only if the observed census changes
- `docs/lld/corpus.md`
- `docs/lld/oracle.md`
- `docs/lld/comparator.md`

No tolerance file or HLD file changes.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. This is reference and comparator evidence
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | Width 0.5 is valid for SIGMOID and selected stored values map to independently computed display values under PS3.3 C.11.2.1.3.1 | `scripts/tests/test_corpus_synth.py` |
| `unit` | The oracle preserves the file's SIGMOID function and width rather than applying LINEAR's minimum | `tools/oracle/tests/params_test.mjs` |
| `unit` | A matching reachable register entry attributes a PS3.3-correct candidate divergence to the reference | `tools/oracle/src/attribution.rs` |
| `golden` | The new corpus row renders through cornerstone3D 5.8.2 and the observed defect is recorded with direction and magnitude | oracle output and comparator fixture |

Mutate the synthetic function to LINEAR, change width to 1, and disable the
reference attribution in separate runs. Each must turn its intended proof red.

## Parity surface covered

`docs/hld/B-parity-surface.md` contains:

| VOI LUT functions | 3 | LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID |

This story adds exercised SAMPLED_SIGMOID coverage. Appendix B has no
`Covered by` column and no Y1.7 mapping to edit.

## Deviations

Existing D-11 defines the installable reference pin. No new deviation is
needed. The project follows PS3.3 and records the reference defect through the
existing reviewed divergence mechanism. The tolerance is unchanged.

## LLD impact

`docs/lld/oracle.md` records the new row and measured reference behaviour.
`docs/lld/comparator.md` states which side D14's bound describes and how known
reference defects are excluded. `docs/lld/corpus.md` records the synthetic
case and its fixture provenance.

## Open questions

None. State D14's Ocelli bound against standard-correct output and publish the
reference defect separately. Preserve `raisedBy: F-010`, add
`resolvedBy: F-X012`, and validate both provenance events.
