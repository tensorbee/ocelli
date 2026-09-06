# F-013, Metadata diff harness (LUT values, geometry, spacing)

**Status**: approved
**Epic ref**: E2.5
**Sprint**: S04
**Estimate**: 2w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a comma because `scripts/prose_check.py` covers this plan. No
other word is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/08-validation-architecture.md`, section 11

> The harness pushes the same study through both stacks and compares frames
> within a written per-modality tolerance, with metadata diffed alongside
> pixels because a wrong rescale slope can still produce a plausible image.

### `docs/hld/04-boundary-and-data-path.md`, section 6

> The LUT chain is an explicit shader stage rather than a CPU pass. All of it
> reads from one uniform block, so a window-level drag uploads a few bytes per
> frame instead of re-uploading a texture. It is also the single riskiest piece
> of arithmetic in the project, which is why section 18 specifies it to the
> formula and why the oracle diffs LUT values as well as pixels.

### `docs/hld/15-lut-chain.md`, sections 18, 18.1 and 18.2

> This is the highest-risk arithmetic in the project. It is specified in DICOM
> PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
> ocelli-pixel, and let the shader read the parameters - do not let a second
> copy of this logic appear anywhere.

| **Stage** | **From to To** | **Source** |
|----|----|----|
| 1. Modality LUT | Stored to Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality to Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display to Display | Identity or INVERSE, presentation state may override |
| 4. Palette / ICC | Display to RGB | Palette colour LUT, or the display colour pipeline |

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

> If a Modality LUT Sequence is present it wins over slope and intercept. PET
> SUV is a separate path and needs the radiopharmaceutical sequence - do not
> fold it in here.

The three VOI formulas, character for character:

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

Section 18.3's four rows, with D-13's correction shown explicitly:

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| -160 | 0.000 | 0.000 | Both lower boundaries are -160 and both comparisons are `<=`, so both clamp. D-13 corrects the HLD table's 1.594 |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| -60 | 63.910 | 63.750 | Mid-lower quarter, catches sign and slope errors |

### `docs/hld/22-testing-and-tolerance.md`, sections 25 and 25.1

> | **Layer** | **What it proves** | **Where it comes from** |
> |----|----|----|
> | Unit and property | LUT arithmetic, geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
> | Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
> | Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
> a quarter pixel.

### DICOM PS3.3 C.7.6.2.1.1, as required by the repository DICOM workflow

With `X = IOP[0..3]` the row direction, `Y = IOP[3..6]` the column direction,
`i` the column index and `j` the row index:

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`PixelSpacing[0]` is the spacing between rows and multiplies the column
direction cosine. `PixelSpacing[1]` is the spacing between columns and
multiplies the row direction cosine.

## What the specification does not cover

1. "Metadata" has no complete field list. F-011 deliberately compares only
   fields that decide the current pixels and camera.
2. The HLD does not define the three independent readings. The existing oracle
   provides file attributes from `dicom-parser`, cornerstone modules, and a
   pydicom cross-read. A future Ocelli candidate adds a fourth producer.
3. The HLD gives geometry tolerance but does not define exact equality for
   declared DS values, missing versus empty values, array order, signed zero or
   NaN handling.
4. The HLD does not say how multiframe per-frame functional groups are matched
   to top-level metadata.
5. The HLD does not define a committed truth format for synthetic rows.

## Approach

1. Add `tools/oracle/metadata-truth.json`, hand-authored from PS3.3 and the
   constants in `scripts/corpus_synth.py`. It contains no generated expected
   values and no real-row values. Each entry cites its PS3.3 section and names
   whether the value is top-level, shared functional group or per-frame.
2. Cover the metadata that can quietly change pixels or geometry today:
   pixel module fields, transfer syntax, photometric interpretation, planar
   configuration, modality LUT parameters, VOI parameters and function,
   presentation inversion, IPP, IOP, pixel spacing, slice thickness, resolved
   volume dimensions, origin, direction and consecutive projected slice gaps.
3. Keep one canonical owner per truth domain. `metadata-truth.json` owns
   metadata and literal display samples. The existing `volume-truth.json`
   remains the sole owner of series geometry. Python and Rust read both files,
   and no expected value is copied into two languages or between the files.
4. Compare declared DICOM numbers exactly after parsing, including array order.
   Compare derived world geometry within 1e-6 mm. Distinguish absent, empty and
   present values where the DICOM type permits that distinction. Refuse NaN.
5. Compute geometry truth only for named synthetic fixtures. The non-square
   fixture uses `PixelSpacing = [0.5, 0.25]`, non-axis-aligned IOP and at least
   two nonzero `(i, j)` samples so swapping either index turns the fixture red.
   Volume spacing comes from consecutive projected IPP values, never
   `SliceThickness` or `SpacingBetweenSlices`.
6. Do not implement a LUT evaluator in the harness. Compare the declared and
   resolved LUT parameters and assert the four section 18.3 values as literal,
   independently computed fixture evidence. Actual LUT arithmetic remains one
   implementation in `ocelli-pixel` when that crate lands.
7. Extend the attribution ladder with a metadata-truth component before pixel
   comparison. A side disagreeing with committed synthetic truth is named. If
   both sides agree with one another and disagree with truth, the run fails and
   names an instrument or shared-reference problem rather than passing.
8. For real rows, compare the independent readers without writing values into
   logs, reports or tracked artifacts. Existing `<withheld, real corpus row>`
   behaviour remains.
9. Add declared mutations for transposed pixel spacing, reversed IOP vectors,
   wrong rescale intercept, wrong window function, and using a top-level value
   where a per-frame functional group wins. Each mutation has a named expected
   attribution.

Pass 3 showed that a helper-only lazy-reader test did not protect the
production caller. The standing catalogue therefore also combines a committed
metadata mismatch with refusals on either input frame of the same view. Its expected
metadata-truth record can be produced only while `compare_runs` keeps real
frame I/O behind the metadata verdict.

No tolerance is changed and no DICOM file is added to git.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. This is host-side validation after rendering
- unsafe: none
- Tier A (WebGPU): n/a. Metadata truth is renderer-independent
- Tier B (WebGL2): n/a. Metadata truth is renderer-independent
- Tier C (CPU): n/a. The same sidecar contract applies when tier C output exists

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Modality and VOI parameter truth plus the four hand-computed display values from PS3.3 C.11, including D-13 | `tools/oracle/tests/metadata_fixture.rs` |
| fixture | Non-square spacing maps column index through `PixelSpacing[1]` and row index through `PixelSpacing[0]`, per PS3.3 C.7.6.2.1.1 | `tools/oracle/tests/metadata_fixture.rs` |
| fixture | Consecutive projected IPP gaps expose non-uniform spacing and ignore thickness tags, per PS3.3 C.7.6.2.1.1 | `tools/oracle/check_sidecars.py` against synthetic series |
| unit | Missing, empty, ordered multi-value, signed-zero and NaN cases are handled deliberately | `tools/oracle/src/metadata.rs` |
| property | Swapping the two spacing components or the two direction vectors fails at least one non-square geometry sample | `tools/oracle/tests/metadata_fixture.rs` |
| golden | Reference and candidate metadata are compared beside pixels for every declared view | oracle comparator gate |
| browser | Per-frame functional-group values that drove the reference are present in the sidecar and checked | `tools/oracle/tests/sidecar_test.mjs` |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column and no row
keyed to E2.5. The harness validates LUT and geometry surfaces but does not add
one.

## Deviations

D-13 supplies the corrected first value in the section 18.3 fixture. No new
deviation is anticipated.

## Implementation corrections

The initial implementation exposed seven defects in pass 1 review. Their
remediation changes the anticipated shape in four ways.

- `volume-truth.json` remains the only series-geometry truth. The duplicate
  volume values were removed from `metadata-truth.json`, and both independent
  readers now load the same domain owner.
- `tools/oracle/src/sidecar.rs` deliberately retains the complete sidecar JSON.
  `metadata.rs` reads declared JSON pointers from that retained value, so no
  second typed schema or duplicated field list was added.
- Metadata truth creates its failure record directly from sidecars before any
  frame is read. Missing JSON members and explicit null are distinct, and
  Python uses the same bit-exact numeric comparison as Rust.
- Resolved-field truth binds mechanically to a raw-DICOM source pointer. The
  browser-safe `metadata-sources.mjs` derives per-frame, shared or top-level
  provenance from dicom-parser's data set. A changed label therefore fails.
- Mixed, opposite and shared failures are unattributed. Reports derived from a
  real corpus path replace parameter values recursively before serialization.

`tools/oracle/page/app.mjs` is added to the write set because it owns the
independent dicom-parser attribute read. The generated unsigned CT fixture now
declares Presentation LUT Shape `INVERSE`, which provides positive inversion
evidence. Its DICOM digest changes, while its rendered hash is measured rather
than assumed. The series generator also emits canonical positive zero so exact
cross-reader equality is possible. `docs/lld/README.md` is added because the
repository's additive LLD contributor index must change with the LLD headers.

Pass 2 found that the repaired metadata-before-frame ordering had no durable
regression. `ocelli-compare.rs` now keeps all frame reads behind the lazy
`metadata_before_frames` boundary. The standing combined mutation supplies a
genuine metadata-truth divergence and makes either input frame reader refuse
if invoked. It requires a metadata-truth failure with no statistics. Moving
either production frame read across that boundary therefore makes the oracle
mutation gate red.

## LLD impact

- `docs/lld/comparator.md` records the metadata truth contract, field set and
  attribution order.
- `docs/lld/oracle.md` records the expanded sidecar fields and pydicom
  cross-read.

## Anticipated write set

**Create**

- `tools/oracle/metadata-truth.json`
- `tools/oracle/src/metadata.rs`
- `tools/oracle/src/metadata-sources.mjs`
- `tools/oracle/tests/metadata_fixture.rs`

**Modify**

- `tools/oracle/src/lib.rs`
- `tools/oracle/src/attribution.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/src/sidecar.rs`
- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/src/mutations.rs`
- `tools/oracle/src/sidecar.mjs`
- `tools/oracle/volume-truth.json`
- `tools/oracle/check_sidecars.py`
- `scripts/guards/catalogue.py`
- `ci/guard-probe-budget.json`
- `docs/runbooks/guard-verification.md`
- `tools/oracle/tests/sidecar_test.mjs`
- `scripts/corpus_synth.py`
- `scripts/tests/test_corpus_synth.py`
- `corpus/manifest.tsv`
- `docs/lld/corpus.md`
- `docs/lld/README.md`
- `docs/lld/comparator.md`
- `docs/lld/oracle.md`

## Dependency and conflict notes

- F-011 and F-X007 are done and supply the attribution ladder, sidecars and
  volume truth.
- F-013 overlaps F-012 and F-015 in the comparator binary, report and LLD.
  Run them serially.
- F-X012 may change the reference-divergence handling for VOI function values.
  Land or integrate it before finalising F-013's VOI mutations.
- The oracle is a serial runtime resource.

## Open questions

None. The initial surface is the pixel and geometry-driving fields in approach
item 2. Palette, ICC, and LUT Sequence contents wait for their render paths.
`metadata-truth.json` replaces the existing metadata and display tables.
`volume-truth.json` continues to own series geometry, so no truth becomes two.
