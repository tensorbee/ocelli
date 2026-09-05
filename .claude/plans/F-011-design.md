# F-011, Pixel-diff comparator with per-modality tolerance policy

**Status**: approved
**Epic ref**: E2.3
**Sprint**: S03
**Estimate**: 3w

## Normative source, transcribed

_Transcriptions are verbatim except for one normalisation, the same one F-010's
plan declared: a prose semicolon in the source is written as a comma and an
em-dash as a hyphen, because `scripts/prose_check.py` covers `.claude/plans/`
and `docs/hld/` is exempt. No word is changed. Where the exact bytes matter,
the tracked Markdown under `docs/hld/` wins._

### `docs/hld/22-testing-and-tolerance.md`, section 25, verbatim

The table:

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Unit and property | LUT arithmetic; geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
| Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, verbatim

> Write it down once and hold it. Tuning tolerance per failure is how a suite
> stops meaning anything.
>
> - **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
>   LSB on at least 99.9% of pixels, zero pixels differing by more than 2.
>
> - **Colour and ultrasound:** perceptual difference below a stated threshold,
>   because chroma subsampling and YBR conversion legitimately differ.
>
> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.
>
> - A tolerance change is a pull request with a rationale, reviewed like code.

**This is the whole of the tolerance policy. There is no more of it.** Every
number this story gates on is in those four lines, and every number this story
needs that is not in those four lines is named in
`## What the specification does not cover`.

### `docs/hld/08-validation-architecture.md`, section 11, verbatim

> Cornerstone3D is a correct reference implementation that can render any
> series you own. The harness pushes the same study through both stacks and
> compares frames within a written per-modality tolerance, with metadata
> diffed alongside pixels because a wrong rescale slope can still produce a
> plausible image.
>
> Every pull request renders the corpus in CI. Every field bug becomes a
> permanent fixture. In production, shadow mode renders both libraries and
> alerts on divergence - the oracle running against real clinical traffic, and
> the same corpus a regulatory submission would want to see.

Three clauses of that paragraph are directly this story's, and they are
quoted here rather than paraphrased because each one is a requirement:
"pushes the same study through both stacks", "compares frames within a written
per-modality tolerance", and "with metadata diffed alongside pixels because a
wrong rescale slope can still produce a plausible image". The metadata diff is
not an extra. It is in the same sentence as the pixel comparison.

### `docs/hld/15-lut-chain.md`, section 18, all of it

The preamble, verbatim:

> This is the highest-risk arithmetic in the project. It is specified in DICOM
> PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
> ocelli-pixel, and let the shader read the parameters - do not let a second
> copy of this logic appear anywhere.

The stage table, verbatim:

| **Stage** | **From to To** | **Source** |
|----|----|----|
| 1. Modality LUT | Stored to Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality to Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display to Display | Identity or INVERSE; presentation state may override |
| 4. Palette / ICC | Display to RGB | Palette colour LUT, or the display colour pipeline |

Section 18.1, verbatim:

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

with its note, verbatim: "If a Modality LUT Sequence is present it wins over
slope and intercept. PET SUV is a separate path and needs the
radiopharmaceutical sequence - do not fold it in here."

Section 18.2, its framing sentence verbatim: "The differences between LINEAR
and LINEAR_EXACT are a half and a one, and they are the single most commonly
mis-ported detail in DICOM viewers. Copy these exactly."

And the listing, character by character:

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

Section 18.3, verbatim. "Soft-tissue CT window, centre 40, width 400, output
range 0-255. Hand-computed, and it must be in the test suite before the shader
is written."

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| -160 | 0.000 | 1.594 | LINEAR boundary is c'-w'/2 = -160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| -60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

and the boxed row, verbatim:

> **READ THIS ROW** At the window centre the two functions differ by 0.32 of
> 255. That is invisible to a human comparing screenshots and immediately
> visible to a pixel diff. It is the entire argument for building the oracle
> before writing the code it validates.

Section 18.4, the shader side, verbatim:

```wgsl
// ocelli-render/shaders/voi.wgsl
struct VoiParams {
    center : f32,
    width : f32,
    slope : f32,
    intercept : f32,
    ymin : f32,
    ymax : f32,
    fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
    invert : u32,
};
@group(0) @binding(0) var<uniform> voi : VoiParams;
```

with its note, verbatim: "A window-level drag updates thirty-two bytes per
frame. No texture is re-uploaded, and that is the concrete performance claim
behind Figure 2."

**F-011 implements none of section 18.** It cites it for one reason: 18.3's
divergence is the worked example this comparator has to be able to see, and
18.2's formulas are the source the fixture in `## Approach` is hand-computed
from. The comparator contains no VOI function and no second copy of the LUT
chain. Its fixture frames are literal byte arrays with hand-computed values,
not values produced by code.

### `docs/hld/11-decision-log.md`, section 14, decision D14, verbatim

| **\#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |

and D7, verbatim:

| **\#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |

### `docs/hld/DEVIATIONS.md`, D-04, verbatim

> §11, "Every pull request renders the corpus in CI". CI runs no GPU build and
> no GPU test. The corpus renders locally, in `/verify`, and is required green
> before a push. Operator constraint, GPU CI minutes are expensive. See the
> risk below, this one is not free.

and, from the same file's "The risk carried by D-04, stated plainly":

> HLD §11 makes CI-side corpus rendering the mechanism that "makes generated
> Rust safe to merge at volume", and D7 in the decision log calls the oracle
> the reason generation speed is an advantage rather than a liability. Moving
> that gate off CI moves it onto a human remembering to run it.

### `docs/hld/DEVIATIONS.md`, D-11, verbatim

> Appendix B, "Measured from cornerstone3D v5.8.9 source", and the parity
> target stated as v5.8.9. The oracle pins `@cornerstonejs/core`,
> `@cornerstonejs/tools` and `@cornerstonejs/dicom-image-loader` at exactly
> `5.8.2`. **v5.8.9 does not exist.**

and, from the same file's "D-11, and what it does and does not change":

> The version in Appendix B is not decoration. Decision D7 makes the oracle
> the thing that has to exist before port code, and §11 makes cornerstone3D
> the reference the oracle measures against. So the pinned version is the
> definition of correct for this project.

That last sentence is the one that makes the reference-divergence register in
`## Approach` necessary rather than optional. If the pin is the definition of
correct, then a place where the pin is wrong about PS3.3 has to be written
down, or the comparator reports our correct arithmetic as a defect.

### `docs/lld/oracle.md`, the three things it hands to F-011, verbatim

On output:

> - `<id>.raw`, the RGBA8 bytes the PNG encoder never touched. **F-011
>   compares against these.**

On the sixteen saturated frames:

> A frame can pass all four boundaries and still say very little. The synthetic
> transfer-syntax rows are full-range ramps that declare a soft-tissue CT
> window, so most of the ramp clips to white: `syntax/reference_mono12.dcm`
> comes back 25% black and 71% white, and a codec error in the clipped values
> would not show in a pixel diff.
>
> That is not a failure. The frame is exactly what the file asks for, and the
> plan's rule is that the file's own window wins. So the run **counts and
> names** these rows instead: any frame over
> `informationFloor.extremeFractionWarnAbove` black and white together is
> listed in `run.json` under `lowInformation` and noted on stdout. Sixteen of
> the eighty-nine rendered rows are on that list. F-011 should weight them
> accordingly, and the fuller fix is either a declared category rule here or a
> corrected declared window in `scripts/corpus_synth.py`. Both are later
> stories, and neither should be done by accident.

On the two decimated frames:

> **Eighty-seven of the eighty-nine rendered rows are smaller than the canvas
> and are magnified into it. Two are not.** `real/dx_varepop/00000001.dcm` is
> 879 by 1168 and `real/us_cmb_crc/00000001.dcm` is 590 by 819, both larger
> than 512 in both dimensions, so both are fitted DOWN under `NEAREST`, which
> discards source pixels. A per-modality tolerance written against a magnified
> frame does not automatically hold for a decimated one, so `run.json` lists
> them under `downsampled` rather than leaving F-011 to work it out from the
> sidecar.

On the reference being wrong about SIGMOID:

> **One divergence between that rule and the reference, recorded for F-011.**
> cornerstone3D 5.8.2's `toLowHighRange` applies LINEAR's `(w - 1) / 2` to
> `SAMPLED_SIGMOID` as well, so a file declaring SIGMOID with a width between
> 0 and 1 would be accepted here, correctly under C.11.2.1.3.1, and produce an
> inverted range in the reference: width 0.5 at centre 40 gives lower 39.75 and
> upper 39.25. No corpus row reaches it, because all eighty-five windowed rows
> resolve LINEAR. It is written down because the first SIGMOID row added to the
> corpus will meet it, and because it is the reference's divergence from the
> standard rather than this harness's.

And the sentence that scopes the Rust side:

> `tools/oracle/Cargo.toml` and `src/lib.rs` are the Rust side that will run
> Ocelli under F-011 onwards.

And the sentence F-X007 will invalidate:

> HLD section 28 says the goal of the first two weeks is to diff one windowed
> 2D image, and F-011 is a pixel-diff comparator, so every reference frame here
> is a **stack** render of one frame of one instance. Volume and MPR reference
> renders are not produced.

### `docs/lld/corpus.md`, the tolerance-class column, verbatim

| Token | Meaning |
|-------|---------|
| `synthetic`, `real` | the layer. Exactly one |
| `mono16` | HLD 25.1 tolerance class one |
| `colour`, `us` | HLD 25.1 tolerance class two |
| anything else | which trap the case exists for |

and, from the same file's "What the corpus does not have":

> **HLD 25.1 states no tolerance for 8-bit monochrome at all.** That ultrasound
> case is absorbed into class two by modality.

### `docs/sprints/CURRENT_SPRINT.md`, the defect class, verbatim

> **A verdict is only as good as its tolerance and its reference, and both fail
> silently.**
>
> The dangerous comparator defect is a tolerance that absorbs a real
> divergence. HLD section 25.1 fixes the numbers in advance for exactly this
> reason: monochrome 16-bit is maximum absolute difference of 1 LSB on at least
> 99.9% of pixels with zero pixels differing by more than 2. A comparator that
> passes the corpus on the first run is more suspicious than one that fails,
> and the answer to a failure is never a wider tolerance. That is a design-plan
> decision with a recorded rationale, reviewed like code.
>
> The second half is subtler and S02 found it. **The reference can be wrong.**
> Where cornerstone3D diverges from PS3.3, a diff measures our correct
> arithmetic against its incorrect arithmetic and reports the difference as
> ours. Decision D14 publishes a measured divergence, so the comparator has to
> be able to say which side a difference is on. The SIGMOID case is the known
> instance and it will not be the last.

What done means, verbatim:

> - **F-011** returns a per-row verdict against HLD section 25.1's written
>   tolerances, decides what to do about the sixteen low-information rows, and
>   can attribute a divergence to a side.

The sequencing constraint, verbatim:

> F-011 is the one with a real ordering constraint inside the sprint: F-X007
> adds reference renders that F-011 will then compare, so a comparator written
> before F-X007 lands must not assume stack-only input.

### `docs/hld/24-agent-code-standards.md`, section 27.2, rules R2 and R3, verbatim

| **\#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R3 | Every function doing pixel arithmetic needs a fixture test with hand-computed values, citing the DICOM section | This is the defect class that reaches patients |

**A comparator is pixel arithmetic.** It reads two frames and computes a
difference distribution over them, and every number it emits is a number a
reviewer will trust. R3 therefore applies to it in full, and the fixtures in
`## Tests` are mandatory rather than good practice.

### `docs/hld/B-parity-surface.md`, Appendix B

The design command's step 6 says to read "the parity checklist rows this story
covers, in `docs/hld/B-parity-surface.md`, via the `Covered by` column keyed on
the epic ref". **Appendix B as tracked in this repository has no `Covered by`
column.** Its table is Surface, Count, Notes. The row this story is adjacent to
is, verbatim:

| **Surface** | **Count** | **Notes** |
|----|----|----|
| VOI LUT functions | 3 | LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID |

F-011 implements none of the three. See `## Parity surface covered`.

---

## What the specification does not cover

Eleven decisions this plan makes that HLD 25.1 and section 11 do not make for
it. Each is a place a reviewer should look hardest, and the ones that are not
safe to decide inside a design plan are repeated in `## Open questions` with
the decision they block.

1. **What "1 LSB" is a unit of.** 25.1 names the class "Monochrome 16-bit"
   and the bound "1 LSB". The artefact being compared is an RGBA8 canvas
   frame, so the only LSB present is one 8-bit display code, and the
   reference emits nothing else. 16-bit stored values never reach the
   comparison on either side. This plan reads the bound as one 8-bit display
   code, because that is the only reading the instrument can evaluate, and
   `## Approach` proves by hand computation what that reading costs.

2. **Whether the 99.9% denominator includes the letterbox.** The canvas is
   512 by 512 and every frame is fitted into it, so a frame carries background
   that is not image. `syntax/reference_mono12.dcm` is 25% background exactly.
   Background pixels are the declared clear colour on both sides and agree by
   construction, so counting them inside the denominator inflates the pass
   rate by up to a quarter. 25.1 says "of pixels" and excludes nothing. This
   plan evaluates the gating predicate over the literal full frame, and
   reports the same statistics over the image rectangle alongside, so the
   inflation is visible rather than absorbed.

3. **The perceptual metric for class two.** 25.1 says "perceptual difference"
   and names no metric.

4. **The threshold for class two.** 25.1 says "below a stated threshold" and
   states none.

5. **8-bit monochrome.** 25.1 has no class for it. `docs/lld/corpus.md`
   already records this and absorbs `real/us_cmb_crc/00000001.dcm` into class
   two by modality, which under the answer to items 3 and 4 leaves it with no
   evaluable bound at all.

6. **The verdict vocabulary.** 25.1 gives a predicate. It does not say what a
   comparator reports, what a run-level result is, or what happens to a view
   that satisfies the predicate while carrying no information. This plan
   defines an outcome of `pass`, `fail` or `unmeasured` plus a qualifier set,
   and the reason it is three values and not two is in `## Approach`.

7. **What to do when a view is inside the tolerance and still wrong.** The
   arithmetic in `## Approach` shows that a LINEAR against LINEAR_EXACT swap
   cannot exceed one display code anywhere, so 25.1's monochrome rule can
   never fail one. 25.1 does not name a second statistic. This plan measures
   and reports one and does not gate on it without an operator decision.

8. **Attribution.** Section 11 requires metadata diffed alongside pixels and
   gives the reason. It does not say what to do when the two readings
   disagree, and it does not contemplate the reference being wrong. D14
   requires a measured divergence, which is a number with a side attached, so
   the attribution ladder in `## Approach` is this plan's construction and not
   the HLD's.

9. **A mechanism for recording that the reference is wrong.** Nothing in
   `docs/hld/` names one. `unsupported.json` is the shape F-010 invented for
   the adjacent problem and this plan copies its strictness discipline.

10. **Quantisation.** Neither 25.1 nor section 18 says how a display value
    becomes an 8-bit code. The comparator does no quantising of its own, it
    reads bytes that are already quantised, so this only bites in the
    hand-computed fixtures, where the rounding rule is declared at the fixture
    and is a stated HLD 27.3 review item.

11. **The run-level gate rule.** 25.1 is a per-frame predicate. What makes a
    corpus run green is not written anywhere, and this plan states it.

Two more things the specification does cover and which are worth restating so
nobody re-derives them: the tolerance numbers themselves, which are fixed in
advance and are not this story's to choose, and the requirement that a
tolerance change is a reviewed pull request. **This story changes no
tolerance, and the numbers live as constants in source rather than in a
configuration file, precisely so that changing one is a diff a reviewer sees.**

---

## Approach

### What F-011 compares today, stated plainly

**No port code exists.** D7 is holding and this story does not break it. There
is no Ocelli renderer, so there is no Ocelli frame, so the comparator has no
second side of the corpus to compare against. Pretending otherwise, by
building a stub renderer or by shipping a comparator whose only exercise is the
reference against itself, would produce a green gate that means nothing. That
is the exact defect class this sprint names.

So the comparator is built as a function of two directories, and today those
two directories are filled by three things, none of which is a renderer.

1. **Hand-constructed fixture frame pairs**, built in memory inside the test,
   whose divergence is known by construction because both sides' bytes are
   written out by hand. This is the mandatory 27.2 R3 layer and it is the only
   thing that proves the comparator's own arithmetic. It runs in
   `cargo test --workspace`, which is a floor gate, so it runs in CI with no
   GPU and no corpus. That is a real strengthening of what D-04 leaves covered.

2. **Identity over the real reference output.** The comparator run over
   `tools/oracle/out/` against itself exercises the loader, the identifier
   mapping, the tolerance-class resolution, the sidecar contract and the report
   shape over all eighty-nine rendered views. It proves the plumbing and it
   proves nothing about detection, which is why it is never allowed to be the
   only corpus-scale exercise.

3. **A declared mutation set** applied in memory to real reference frames, with
   the verdict each mutation must produce written down beside it. This is what
   proves detection at corpus scale. It follows F-010's own pattern for the
   fault catalogue: the catalogue is production data in `src/`, the runner is
   in `tests/`, and a guard nobody has watched fail is not a guard.

**The candidate side is a directory contract, not a call into a renderer.**
When the port lands, the Ocelli half writes `<id>.raw` and `<id>.json` in the
shape `docs/lld/oracle.md` already specifies, points the comparator at it, and
nothing in the comparator changes. That is also what makes deviation D-07's
tier A against tier C divergence bound the same tool with two candidate
directories and no reference, at no additional cost.

### Where it lives, and in what language

The comparator is Rust, in the existing `ocelli-oracle` crate under
`tools/oracle`. `docs/lld/oracle.md` already says that crate "is the Rust side
that will run Ocelli under F-011 onwards", the workspace lints that matter most
to this arithmetic are Rust lints (`cast_possible_truncation`,
`cast_precision_loss`, `cast_sign_loss` and `float_cmp` all denied), and the
side this comparator will eventually drive is Rust. It is a library plus one
binary, `ocelli-compare`.

It reads the reference half's output and writes nothing back into it. **F-011
modifies no file under `tools/oracle/src/*.mjs`, `page/`, `run.mjs` or
`render-params.json`.** That is deliberate and it is the answer to the F-X007
sequencing constraint, because those are exactly the files F-X007 has to
change.

### The input contract, written down so F-X007 can keep it true

The comparator asserts every one of these on load and refuses rather than
assuming. Writing them down is the mechanism that stops F-X007 from breaking
the comparator silently.

- `run.json` exists on both sides and carries `manifestSha256`,
  `renderParamsSha256`, `unsupportedSha256`, `rows`, `lowInformation.rows` and
  `downsampled`.
- **Every `*Sha256` key present on either side is present on both and equal.**
  Not a fixed list of two, because F-X007's plan adds `volumeParamsSha256` and
  `volumeTruthSha256` and a fixed list would silently stop covering the inputs
  that decide the frames. `packages` is exempt, because the candidate side is
  not cornerstone3D. A disagreement is refused before a single pixel is read,
  because `render-params.json`'s own note says a change there "changes every
  reference frame", and two frames produced under different parameters are two
  correct frames that differ.
- **The view list is every frame the run produced, and the comparator proves it
  found them all.** Today that is `rows[]`. F-X007's plan keeps `rows[]`
  stack-only on purpose, to preserve the accounting identity
  `readBack + unsupported === applicable`, and puts its twelve reformats in
  `volumes[].frames[]`. So the view list is the union of the declared frame
  lists, and **the comparator refuses a run where a `.raw` file exists in the
  directory that no declared list names.** That refusal is the guard: it is what
  turns "F-011 silently compared 89 of 101 views and reported success" into a
  failure, and it keeps working when a later story adds a third list.
- Every view entry carries `id`, and `ok` where the list has an applicability
  notion. **The identifier is opaque.** The comparator never reconstructs it
  from a manifest path and never globs for its work list, so F-X007 may mint
  `volume__real__ct_cmb_mml__SAGITTAL` and nothing here needs to know the
  scheme.
- **Each sidecar carries a top-level `kind`, and the comparator switches on it
  and refuses an unknown value.** F-X007's plan adds `"stack"` to the existing
  eighty-nine and `"volume-reformat"` to the new twelve, and names this "the
  single field that makes a comparator written before F-X007 lands not assume
  stack-only input". An unknown `kind` is a refusal and never a default to
  `stack`, because defaulting is how a new view kind gets compared under the
  wrong rules.
- **The mapping from a manifest path to identifiers is one to many, and a
  census entry may be keyed on either.** `lowInformation.rows` and
  `downsampled` are keyed on `path` today, and F-X007's plan gives those
  entries a `kind` and keys a volume frame on `id` where a stack entry uses
  `path`. So the resolver accepts both keys and returns a set of view
  identifiers. Today that set has one member per path.
- For every applicable view there exist `<id>.raw`, `<id>.png` and `<id>.json`.
  The comparator reads `.raw` and never `.png`, because the PNG encoder is a
  second transformation and comparing its output would measure it.
- Each sidecar carries `frame.width`, `frame.height`, `frame.sha256`,
  `frame.statistics`, `row.categories`, `voi`, `camera`, `attributes`,
  `cornerstoneMetadata` and `image`. A `volume-reformat` sidecar's `row` block
  refers to the series rather than to one instance, which changes nothing for
  the tolerance class, because the class token is a property of the pixel data
  and every member of a series carries the same one.
- Each `.raw` is `width * height * 4` bytes and hashes to `frame.sha256`. The
  reference half hashes twice already, in the page and in the driver. The
  comparator hashes a third time at read, because a file edited after the run
  would otherwise be compared as if somebody had rendered it.
- The identifier sets on the two sides are equal. An identifier on one side
  only is an `absent` verdict and a run failure, never a skip.
- The frame dimensions of the two sides agree per view, read from each sidecar
  rather than from `render-params.json`, because F-X007 may render a view at a
  size the base canvas does not declare.

### Tolerance class resolution

The class comes from the manifest tokens carried in the sidecar's
`row.categories`, matching `CLASS_TOKENS` in `scripts/corpus_check.py`
(`mono16` for class one, `colour` or `us` for class two). It is never rederived
from the modality, because `corpus_check.py --coverage` already fails a row
that declares no class, so the token is a checked declaration and the modality
is not. A row declaring both classes is refused. One corpus row carries both
`us` and `colour`, `synthetic/us_ybr_full_422.dcm`, and both are class two, so
that row is class two and not a conflict. Today the eighty-nine rendered views
are eighty-four class one and five class two.

### The per-view record, and what gates

For every view the comparator produces one record with four components. They
are evaluated in this order and the first that answers attributes the
divergence, which is the concrete answer to "can attribute a divergence to a
side".

**Component 1, input identity.** Both sides' `run.json` digests agree and each
view's `row.sha256` matches the manifest. A mismatch attributes the divergence
to the inputs and not to either renderer, and it is a refusal rather than a
verdict.

**Component 2, parameters.** The two sides' declared values for everything that
decides the compared pixels are compared for exact equality: from `voi`, the
source, centre, width, function and origin. From `image`, slope, intercept,
minimum and maximum pixel value, number of components and data type. From
`attributes`, `rescaleSlope`, `rescaleIntercept`, `windowCenter`,
`windowWidth`, `voiLutFunction`, `photometricInterpretation`, `samplesPerPixel`,
`planarConfiguration`, `rows`, `columns`, `bitsAllocated`, `bitsStored`,
`highBit` and `pixelRepresentation`.

This is HLD section 11's "metadata diffed alongside pixels because a wrong
rescale slope can still produce a plausible image", and it runs before the
pixels rather than beside them, because a parameter divergence explains a pixel
divergence and the reverse is not true.

Floating-point values here are DICOM-declared numbers read from a file, not
computed results, so the comparison is exact equality implemented as
`f64::to_bits` equality. That satisfies the denied `float_cmp` lint honestly
rather than by an allow, it makes the intent visible, and it is correct: two
readers of one `DS` value that disagree is a finding and not a tolerance. A
`NaN` in any of these fields is refused outright rather than compared.

When the sides disagree, the side that disagrees with the third, independent
reading is the one at fault. That third reading exists today: the sidecar's
`attributes` block is read straight from the bytes by `dicom-parser` in the
page, independently of the render path, and `check_sidecars.py` cross-reads it
under pydicom. So a reference that resolved a field differently from the bytes
is attributable without anything new. **The full three-way metadata harness is
F-013, E2.5, S04.** F-011 diffs only the fields that decide the compared pixels
and the compared camera, and says so.

**Component 3, geometry.** 25.1's third bullet, applied to the two sides'
`camera` blocks. `position`, `focalPoint`, `viewUp` and `parallelScale` are
world millimetres and are compared within 1e-6 mm. The canvas half of the
bullet is applied to the derived image extent: `millimetresPerCanvasPixel` is
`2 * parallelScale / canvasHeight`, so the image's extent in canvas pixels
follows from the row and column spacing, and the two sides' extents must agree
within a quarter pixel. This is the same derivation `canvasScale` in
`tools/oracle/src/params.mjs` already performs, and `## Open questions` names
the second-copy problem that creates.

**Component 4, pixels.** Over the two `.raw` buffers.

Alpha is asserted to be 255 on every pixel of both sides and is never included
in a difference. The reference already records `opaque` per frame, and a frame
that is not fully opaque is a refusal, because a difference in alpha is a
difference in the canvas and not in the image.

For a class-one view, the frame is asserted to be monochrome on both sides,
meaning R equals G equals B on every pixel. A `mono16` token over a frame that
is not monochrome means the token is lying, and that is a refusal. The
comparison is then over the single channel.

For a class-two view the comparison is per channel.

The statistics, per view:

- `pixels`, the sample count.
- `maxAbsDiff`.
- `countAtZero`, `countAtOne`, `countAtTwo`, `countOverTwo`, which is the whole
  histogram 25.1 needs and no more.
- `fractionWithinOneLsb`, which is `(countAtZero + countAtOne) / pixels`.
- `signedMeanDiff`, candidate minus reference.
- `differingFraction`, `|d| >= 1`.
- `rowsTouched` and `columnsTouched`, the number of distinct canvas rows and
  columns carrying any difference.
- `backgroundDiffering` and `imageDiffering`, split on the derived image
  rectangle.
- `informativeFraction`, the fraction of image-rectangle pixels not clipped to
  the same extreme on both sides.
- The same statistics restricted to the informative subset.

**The gating predicate for class one, and only this:**

> `fractionWithinOneLsb >= 0.999` and `countOverTwo == 0`

Read it carefully, because it is easy to get wrong in a way that is harder or
softer than written. At least 99.9% of pixels are within 1, and no pixel
exceeds 2. A pixel differing by exactly 2 is permitted, for up to 0.1% of the
frame. A pixel differing by 3 is never permitted, at any count. The constants
`0.999`, `1` and `2` live as named constants in `tools/oracle/src/tolerance.rs`
with 25.1 quoted above them, and a test asserts each constant against the
quoted text, so a silent widening changes a line a reviewer reads.

**There is no gating predicate for class two**, because 25.1 states no
threshold. See below.

### The arithmetic that says this tolerance is not enough, hand-computed

This is the most important paragraph in the plan and it is the reason the
verdict has three outcomes rather than two.

From 18.2's transcribed formulas, at the soft-tissue window, centre 40, width
400, output range 0 to 255, so `c' = 39.5` and `w' = 399`:

```text
LINEAR(x)       = ((x - 39.5) / 399 + 0.5) * 255
LINEAR_EXACT(x) = ((x - 40)   / 400 + 0.5) * 255

LINEAR(x) - LINEAR_EXACT(x)
  = 255 * [ (x - 39.5)/399 - (x - 40)/400 ]
  = 255 * [ (400(x - 39.5) - 399(x - 40)) / (399 * 400) ]
  = 255 * [ (400x - 15800 - 399x + 15960) / 159600 ]
  = 255 * (x + 160) / 159600
```

At `x = -160` the difference is 0, which is LINEAR's lower clamp. At
`x = 239`, LINEAR's upper clamp, it is `255 * 399 / 159600 = 0.6375`. Over the
whole window it never exceeds `255 * 400 / 159600 = 0.639`.

**A difference bounded by 0.639 of a display code cannot survive quantisation
to 8 bits as more than one code, under any monotone quantiser.** So a pure
LINEAR against LINEAR_EXACT swap always yields `maxAbsDiff <= 1`,
`countOverTwo == 0` and `fractionWithinOneLsb == 1.0`, which passes 25.1's
monochrome predicate by a wide margin. The divergence the HLD calls "the entire
argument for building the oracle before writing the code it validates" cannot
be failed by the tolerance the HLD writes down, when the comparison happens at
8 bits.

Sixteen hand-computed values, from the 18.2 formulas above, rounded half away
from zero. None of the sixteen lands on a half, so the table is independent of
which of the three common rounding rules is used, which is why these sixteen
were chosen. Stored value equals HU here because the row's slope is 1 and its
intercept is 0.

| HU | LINEAR | code | LINEAR_EXACT | code | difference |
|----|--------|------|--------------|------|------------|
| 100 | 166.165 | 166 | 165.750 | 166 | 0 |
| 101 | 166.805 | 167 | 166.388 | 166 | 1 |
| 102 | 167.444 | 167 | 167.025 | 167 | 0 |
| 103 | 168.083 | 168 | 167.663 | 168 | 0 |
| 104 | 168.722 | 169 | 168.300 | 168 | 1 |
| 105 | 169.361 | 169 | 168.938 | 169 | 0 |
| 106 | 170.000 | 170 | 169.575 | 170 | 0 |
| 107 | 170.639 | 171 | 170.213 | 170 | 1 |
| 108 | 171.278 | 171 | 170.850 | 171 | 0 |
| 109 | 171.917 | 172 | 171.488 | 171 | 1 |
| 110 | 172.556 | 173 | 172.125 | 172 | 1 |
| 111 | 173.195 | 173 | 172.763 | 173 | 0 |
| 112 | 173.835 | 174 | 173.400 | 173 | 1 |
| 113 | 174.474 | 174 | 174.038 | 174 | 0 |
| 114 | 175.113 | 175 | 174.675 | 175 | 0 |
| 115 | 175.752 | 176 | 175.313 | 175 | 1 |

Seven of sixteen differ, all by exactly 1, all in the same direction. Over a
window-wide uniform distribution of stored values the differing fraction is the
mean of `255 * (x + 160) / 159600` over the window, which at the window centre
is `255 * 200 / 159600 = 0.3195`, so roughly a third of in-window pixels differ
by one code and the verdict is `pass`.

**What the comparator does about it.** It reports `signedMeanDiff`, which for
this fixture is `+7/16 = +0.4375` and for a random one-code noise field is
approximately zero. A systematic, one-signed, large-fraction difference with
parameters and geometry agreeing is the signature of a window-function
divergence, and it is visible in the record whether or not it gates. Whether it
may gate is `## Open questions` item 2, because adding a gating statistic that
25.1 does not name is adding a tolerance, and that is an operator decision and
not an implementer's.

### The sixteen low-information rows, decided

**Decision: measure the informative fraction, mark the view, never count it as
a pass, and do not re-render.**

The comparator reads `run.json`'s `lowInformation.rows` rather than rederiving
saturation from the frames, because F-010 already computes it against a
declared threshold in `render-params.json` and a second derivation would be
free to drift from the first. It resolves each listed path to its view
identifiers, and for each such view it:

- computes `informativeFraction` over the image rectangle, meaning the fraction
  of image pixels that are not clipped to the same extreme in both frames,
- evaluates 25.1's predicate over the full frame exactly as for any other view,
  and evaluates it again over the informative subset,
- attaches the qualifier `weak` and sets the outcome to `unmeasured` when the
  informative fraction is below the declared floor.

`unmeasured` is a third outcome and not a synonym for `pass`. The run-level
report gives three counts, and against today's corpus a run of eighty-nine
views reports as "67 pass, 0 fail, 22 unmeasured" rather than as "89 compared".
The twenty-two are the sixteen saturated views, the five class-two views, and
`real/dx_varepop/00000001.dcm`, the one decimated view that is not already a
class-two one. `real/us_cmb_crc/00000001.dcm` is decimated and class two and is
counted once, with both qualifiers. This is `docs/lld/oracle.md`'s own "Covered
is not the same as measured" made into a number the gate carries, and the
number is uncomfortable on purpose: **a quarter of the corpus is covered and
not measured, and that was true before this story and invisible.**

**What F-011 deliberately does not do.** It does not add a second render at a
wider window, and it does not touch `render-params.json` or
`scripts/corpus_synth.py`. Both are named in `docs/lld/oracle.md` as later
stories, `render-params.json`'s own note says a change there changes every
reference frame and is a reviewed pull request, and neither should be done by
accident. A comparator that widened a window to make its own numbers look
better would be the tolerance-widening failure in a different coat.

### The two decimated rows, decided

**Decision: gate on geometry, report the pixel statistics, never count them as
a pass, and never widen the pixel tolerance for them.**

`real/dx_varepop/00000001.dcm` renders at 0.438 canvas pixels per source pixel
and `real/us_cmb_crc/00000001.dcm` at 0.625. Under `NEAREST`, a sub-source-pixel
difference in the fit selects a different source pixel, so the difference at a
canvas pixel becomes the difference between two unrelated stored values, which
is unbounded. A perfect LUT chain can fail 25.1 by hundreds of codes on these
two rows for a reason that has nothing to do with the LUT chain.

Worse, and this is the part that has to be said rather than assumed: **25.1's
own geometry bound does not rescue them.** A quarter of a canvas pixel is
0.57 source pixels at 0.438 scale, which is more than half a source pixel, so
two cameras agreeing inside 25.1's written canvas tolerance can still land on
different source pixels. No written tolerance covers a decimated frame.

So these two views get outcome `unmeasured` with qualifier `decimated`. Their
geometry component still gates, at 25.1's written bound. Their pixel statistics
are computed and reported in full, including `rowsTouched` and
`columnsTouched`, because a resampling phase error touches whole rows and
columns while a LUT error is scattered, and that distinction is the useful
signal. The alternative, giving those two rows a magnified render, is a change
to `render-params.json` and therefore a reviewed change to every reference
frame, which is a story of its own.

### Class two, and the honest handling of an unstated threshold

25.1 requires class two to be "below a stated threshold" and states no
threshold and names no metric. There are two available responses and only one
of them is honest.

The comparator computes and reports, per channel, `maxAbsDiff`, the 99.9th
percentile of `|d|`, `differingFraction` and `signedMeanDiff`, and returns
outcome `unmeasured` with qualifier `unstated-threshold`. **It never returns
`pass` for a class-two view**, because a pass would be a claim against a
threshold nobody wrote, and D14 says to claim measured divergence rather than a
bound we invented.

It also does not implement a perceptual metric. Choosing CIEDE2000 without a
threshold produces a number nobody can act on, and the metric and the threshold
are one decision, not two. `## Open questions` item 3 asks for both together.

Five of the eighty-nine rendered views are class two, and one of them,
`real/us_cmb_crc/00000001.dcm`, carries `decimated` as well and is also the
8-bit monochrome row that 25.1 has no class for at all.

### The reference-divergence register

A committed `tools/oracle/reference-divergence.json`, whose digest the
comparison record carries the way `run.json` carries `unsupportedSha256`. It
exists because D-11 makes the pin "the definition of correct for this project",
and a place where the pin is wrong about PS3.3 has to be recorded or our
correct arithmetic is reported as a defect.

Each entry carries an identifier, a match condition over sidecar fields, the
PS3.3 citation, what 5.8.2 does, what the standard requires, the expected shape
of the pixel effect, a `reachable` flag with its reason, and the F-ID that
raised it. When a view's divergence is explained by a matching entry, the
outcome is `unmeasured` with qualifier `reference-divergence` and the record
names the entry, rather than `fail`.

Its only entry on day one is the SIGMOID width case F-010 recorded, and that
entry carries `reachable: false` with the reason that all eighty-five windowed
corpus rows resolve LINEAR. So the `unsupported.json` strictness discipline
applies in the direction that can be true today: **an entry marked unreachable
that fires is a run failure**, because the claim was wrong. The opposite
direction, an entry no row exercises failing the run, cannot be applied while
the corpus cannot reach it, and applying it would fail every run from the first
one. Resolving the SIGMOID case itself is F-X012, S04. F-011 builds the
register that will hold it.

**A new entry is a reviewed change with a rationale, exactly like a tolerance
change.** The register is the one mechanism in this design that can turn a
failure into a non-failure, so it gets the same handling as the other one.

### The attribution ladder, in order

1. Inputs disagree, by digest. Attributed to the inputs. Refusal.
2. Parameters disagree. Attributed to the side that disagrees with the
   independent reading of the bytes. Qualifier `parameter-divergence`.
3. A register entry matches. Attributed to the reference, with its PS3.3
   citation. Qualifier `reference-divergence`.
4. Geometry is outside 25.1's bound, or the difference is confined to the
   letterbox or to whole rows and columns. Attributed to the fit rather than to
   the LUT chain.
5. Pixels diverge with parameters and geometry agreeing. **Attributed to
   ours**, by default, because section 11 makes cornerstone3D the reference and
   D-11 makes the pin the definition of correct until an entry in the register
   says otherwise. The signed mean and the spatial statistics are reported so a
   human can raise that entry, and raising one is reviewed.

Rule 5's default direction matters. It is the conservative one, and it is what
makes the instrument useful under D7: the burden is on us to show the reference
is wrong, not on the reference to show it is right.

### The run-level rule

A comparison run is green when every view has outcome `pass` or `unmeasured`,
there are zero `absent` views, and the census of `unmeasured` views and their
qualifiers matches the committed `tools/oracle/compare-expectations.json`
exactly, in both directions. A view that leaves the census is a coverage gain
that has to be recorded, and a view that joins it is a coverage loss that has
to be explained. That is `unsupported.json`'s discipline, and it is what stops
`unmeasured` from becoming a place things go to be forgotten.

### Where it runs

- `bin/ocelli.sh compare` is the new command. `bin/ocelli.sh gate oracle`
  becomes `"$0" oracle && "$0" compare`, chained with `&&` for the reason the
  file already comments on, that a case arm returns the status of its last
  command.
- The fixture and unit tests run under `cargo test --workspace`, which is the
  `test` gate, which is in `--floor`. **They need no GPU, no browser and no
  corpus**, so the comparator's own arithmetic is proved in CI under D-04.
- The corpus-scale identity and mutation exercises are the `ocelli-compare`
  binary's own subcommands invoked by `bin/ocelli.sh compare`, and they are
  **not** `#[ignore]` tests. An ignored test that needs `out/` reads as a pass
  when it did not run, and this repository refuses skips that report as
  successes.
- No new gate is added to the `GATES` list. The comparator is part of what
  `oracle` means.

### Structural choices, and what is deliberately not built

- **No trait.** There is one comparator, one tolerance policy and one report
  writer, each with one implementation today. `AGENTS.md`'s rule is that a new
  trait needs two implementers today.
- **No generic parameter.** The class-one and class-two paths are two functions
  over the same statistics struct, not one function over a class parameter.
- **No configuration file for the tolerance.** Constants in source, with 25.1
  quoted above them.
- **No second copy of the LUT chain.** The comparator contains no VOI function.
  The fixture in this plan is a table of hand-computed literals.
- **No PNG output.** A difference visualisation is written as `<id>.diff.raw`
  in the same RGBA8 shape as the frames it came from, beside the reference's
  own `<id>.png` pair, rather than pulling in an image encoder. F-012 and F-015
  can revisit that with a named user.

---

## Boundary and tier

- wasm-bindgen: not touched. The comparator is host-side Rust in
  `tools/oracle`, which is `publish = false` test infrastructure, and it names
  no browser API.
- Pixels across the boundary: no. The comparator reads files on the host. It
  never runs in a worker, never enters a wasm instance and never crosses D3's
  boundary in either direction. A naive design would have put a comparator
  inside `ocelli-wasm` so it could see the renderer's buffers, and that is the
  thing D3 forbids.
- Render-loop allocation: none, and there is no render loop. The comparator
  allocates two frame buffers and one statistics accumulator per view and
  reuses them across views, so peak memory is two frames rather than the
  corpus.
- unsafe: none. Neither of the two permitted files is touched.
- Tier A (WebGPU): n/a. The comparator renders nothing and asks for no adapter.
  It is the instrument that will measure a tier A frame, not a producer of one.
- Tier B (WebGL2): n/a, for the same reason. Note that the reference frames it
  reads were produced on WebGL2 under SwiftShader, which `run.json` records,
  but that is a property of the input and not a tier requirement of this code.
- Tier C (CPU): n/a as a rendering tier. Worth stating positively rather than
  only as "not applicable": because the candidate side is a directory contract
  and not a renderer, deviation D-07's requirement that "the divergence bound
  has to cover tier A against tier C" is served by this same binary with two
  candidate directories and no reference, and needs no code change when
  F-X001 to F-X004 land.

---

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | 25.1's class-one predicate at each of its three boundaries: exactly 99.9% within 1 passes, one pixel below that fails, one pixel at difference 3 fails at any count, and a frame with 0.1% of pixels at difference 2 passes. Hand-computed from the transcribed 25.1 text, on frames of a size chosen so 0.1% is an integer count | `tools/oracle/tests/tolerance_fixture.rs` |
| `fixture` | The LINEAR against LINEAR_EXACT divergence pair, the sixteen hand-computed values in `## Approach` from PS3.3 C.11.2.1.2 and C.11.2.1.3.2 as transcribed in HLD 18.2. Asserts `maxAbsDiff == 1`, `countOverTwo == 0`, `fractionWithinOneLsb == 1.0`, outcome `pass`, and `signedMeanDiff == 0.4375`. **This fixture asserts that the written tolerance passes a real divergence**, which is the finding, not a bug in the test | `tools/oracle/tests/voi_divergence_fixture.rs` |
| `fixture` | The 18.3 table's four rows quantised to 8 bits, showing that all four sample points collapse to identical display codes, and the separate half-value case at the window centre where the rounding rule decides. Cites PS3.3 C.11.2.1.2, C.11.2.1.3.2 and HLD 18.3 | `tools/oracle/tests/voi_divergence_fixture.rs` |
| `fixture` | The canvas-scale derivation, against `docs/lld/oracle.md`'s worked case: 64 by 96 at spacing [0.5, 0.25] with `parallelScale` 16 gives 8 canvas pixels per source pixel vertically and 4 horizontally. Cites PS3.3 C.7.6.2.1.1 for which spacing element is the row spacing | `tools/oracle/tests/geometry_fixture.rs` |
| `fixture` | The image-rectangle partition, against the same row's recorded `blackFraction` of 0.25 exactly, so the letterbox derivation is checked against a number the reference measured | `tools/oracle/tests/geometry_fixture.rs` |
| `fixture` | The geometry bound, 1e-6 mm in world and a quarter pixel in canvas, at each boundary. Cites HLD 25.1's third bullet | `tools/oracle/tests/geometry_fixture.rs` |
| `unit` | Every loader refusal, each observed red: dimensions disagreeing between sides, an identifier on one side only, a `.raw` whose digest does not match its sidecar, a `.raw` whose length is not `width * height * 4`, a non-monochrome frame under a `mono16` token, alpha not 255, a `NaN` in a LUT parameter, a row declaring two tolerance classes, disagreeing `manifestSha256`, disagreeing `renderParamsSha256` | `tools/oracle/src/*.rs` under `#[cfg(test)]` |
| `unit` | Path to identifier resolution is one to many, proved on a synthetic `run.json` where one manifest path yields three view identifiers, and a census entry keyed on `id` resolves as well as one keyed on `path`. This is the F-X007 guard and it is written before F-X007 exists | `tools/oracle/src/sidecar.rs` |
| `unit` | A `.raw` in the directory that no declared frame list names fails the run, proved on a synthetic directory. This is the guard against a later story adding a frame list the comparator does not read, and it is the one that stops a partial comparison reporting as a complete one | `tools/oracle/src/sidecar.rs` |
| `unit` | An unknown sidecar `kind` is refused rather than defaulted to `stack` | `tools/oracle/src/sidecar.rs` |
| `unit` | The register: an entry marked unreachable that fires is a failure, a matching entry converts a divergence to `reference-divergence`, and a non-matching entry does not | `tools/oracle/src/attribution.rs` |
| `unit` | The census: a view entering or leaving `compare-expectations.json` fails the run, in both directions | `tools/oracle/src/report.rs` |
| `property` | The difference statistics are symmetric under swapping the two sides, except `signedMeanDiff`, which negates exactly. Run under `proptest`, which is already a workspace dev-dependency | `tools/oracle/tests/symmetry.rs` |
| `golden` | Over the real reference output: identity across all rendered views yields zero differences and the expected census, and every mutation in the declared catalogue flips the verdict to the outcome written beside it. Not a `cargo test`, because it needs `out/`, and an ignored test reads as a pass when it did not run | `tools/oracle/src/mutations.rs` as the catalogue, run by `bin/ocelli.sh compare` |
| `conformance` | none. F-011 decodes nothing |
| `browser` | none. F-011 adds no browser code and no Playwright path |

**On the mandatory `fixture` rows.** 27.2 R3 applies because the comparator is
pixel arithmetic. Every fixture above derives from the transcribed
specification text or from a value the reference measured, never from reading
the comparator. Each hand-computed number in the plan above is reproduced in
the fixture with its source cited at the assertion, so 27.3's "LUT and geometry
arithmetic against the cited specification section, not against the comment
above it" has something to check against.

**On the mutation catalogue.** It follows `src/faults.mjs`'s split, the
catalogue as production data in `src/` and the runner separate, and it exists
for the reason `docs/lld/oracle.md` gives: a guard nobody has watched fail is
not a guard. The initial catalogue is `+1 on a declared fraction of pixels`,
`+2 on exactly 0.1% of pixels`, `+3 on one pixel`, `+1 on 0.2% of pixels`,
`a one-pixel canvas translation`, `a channel swap on a colour view`,
`alpha set to 254 on one pixel`, `a changed rescale slope in the sidecar with
pixels untouched`, and `a changed camera parallelScale within and outside
1e-6 mm`. Each names the outcome it must produce. Per
`docs/sprints/CURRENT_SPRINT.md`, the mutation that proves a guard must not be
run in the same command that adds the guard.

---

## Parity surface covered

**None.** Appendix B's rows are cornerstone3D surfaces to be reimplemented, and
F-011 reimplements nothing. Its adjacent row is "VOI LUT functions | 3 |
LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID", and F-011 implements none of the three.
What it does for that row is make the difference between the first two
measurable and record the reference's divergence on the third.

Appendix B in this repository has no `Covered by` column, so the design
command's step 6 cannot be followed literally. Raised in `## Open questions`.

---

## Deviations

**None cited.** No existing `D-NN` row is relied on by this design, and none is
contradicted.

Two candidate new rows are described precisely in `## Open questions`, items 3
and 8, for the operator to apply to `docs/hld/DEVIATIONS.md`. This plan does
not cite either by number, because `scripts/deviation_check.py` refuses a plan
citing a row that does not exist, and because assigning a number to an
unapproved row is the thing that check is for.

Two existing rows are load-bearing context rather than citations, and are
transcribed above for that reason: D-04 is why the fixture layer running in the
floor gate matters, and D-11 is why the register in `## Approach` exists.

---

## LLD impact

- **`docs/lld/comparator.md`, new.** The comparator, the tolerance predicate as
  implemented, the input contract, the verdict vocabulary, the attribution
  ladder, the register, the census, and what it compares today given that no
  port code exists.

  A new file rather than a section in `docs/lld/oracle.md`, for one reason
  worth stating: F-X007 rewrites large parts of `oracle.md` in this same
  sprint, and putting F-011's design there guarantees a merge conflict in the
  document that explains the instrument. The two files cross-reference.

- **`docs/lld/oracle.md`, modified minimally.** "This half compares nothing"
  gains a pointer to the new file, and the `<id>.raw` line's "F-011 compares
  against these" becomes a statement of fact rather than a forward reference.
  Two lines, deliberately, to keep the F-X007 merge surface small.

- **`docs/lld/README.md`.** One row in the table.

---

## Open questions

Fourteen. Items 1 to 7 are blocking in the sense that the comparator's verdict
for some set of views cannot be settled without them, and this plan states a
recommendation for each so that a `--draft` run can proceed and the batch
decision round has something to accept or reject.

**1. What is "1 LSB" a unit of in 25.1's monochrome rule?**
One 8-bit display code in the compared frame, or one unit of the 16-bit stored
value? The two readings differ by a factor of 256.
*Blocks*: the pass predicate for 84 of the 89 rendered views, and therefore
every class-one verdict this project will ever emit.
*Evidence*: the reference emits RGBA8 canvas frames and nothing else, so the
16-bit reading is not evaluable against this instrument at all. The 8-bit
reading is the only one the instrument can answer, and the arithmetic in
`## Approach` shows exactly what it costs.
*Recommendation*: adopt the 8-bit reading explicitly in 25.1 or in a deviation
row, so that a later reader does not assume the stricter one was meant and
believe the comparator is 256 times tighter than it is.

**2. May the comparator gate on a statistic that 25.1 does not name?**
Specifically a bound on `signedMeanDiff` over the image rectangle, applied only
when parameters and geometry agree, which is what a systematic window-function
divergence trips and random noise does not.
*Blocks*: whether F-011 can fail a LINEAR against LINEAR_EXACT swap at all.
Under 25.1 as written and read at 8 bits, it provably cannot, because the
maximum divergence is 0.6375 of a code.
*Options*: (a) report only and never gate, which is pure D14 and leaves the
comparator unable to catch the project's own headline defect. (b) State a bias
bound in 25.1 through a reviewed HLD change and gate on it. (c) Wait for a
16-bit readback path, which does not exist on either side and is not in the
backlog.
*Recommendation*: (b). This is adding a tolerance, which is explicitly an
operator decision and not an implementer's, which is why it is a question and
not a design choice above.

**3. The class-two perceptual metric and its threshold, together.**
25.1 names neither.
*Blocks*: the verdict for 5 of 89 rendered views, and every colour and
ultrasound row this project will ever add.
*Recommendation*: F-011 returns `unmeasured` and reports per-channel
statistics, and metric plus threshold are decided together in one later story,
because a metric without a threshold produces a number nobody can act on.
*Candidate deviation row*, for the operator to apply at the next free number:
"HLD says: §25.1, colour and ultrasound frames must show a perceptual
difference below a stated threshold. We do: the comparator measures per-channel
difference statistics for class-two views and returns `unmeasured`, never
`pass`. Why: §25.1 states no threshold and names no metric, so a pass would be
a claim against a bound nobody wrote, which D14 forbids. The measurement is
published and the bound is not claimed. Raised: F-011."

**4. 8-bit monochrome has no class in 25.1 at all.**
`real/us_cmb_crc/00000001.dcm` is an 8-bit MONOCHROME2 ultrasound, absorbed
into class two by modality per `docs/lld/corpus.md`. Under item 3's answer that
leaves an 8-bit greyscale frame with no evaluable bound, even though the
monochrome rule obviously wants to apply to it. It is also one of the two
decimated views, so it collects two `unmeasured` qualifiers.
*Blocks*: the verdict for that view, and the shape of the class table when a
second 8-bit greyscale row is added.
*Recommendation*: raise with the HLD as a fourth bullet in 25.1 rather than
deciding it in a comparator.

**5. The sixteen low-information rows: is `unmeasured` enough?**
This plan's decision is to measure `informativeFraction`, mark the view `weak`,
set the outcome to `unmeasured`, and not re-render.
*Blocks*: whether the oracle gate can be green while sixteen views cannot show
a codec error in their clipped values.
*Open part*: should a `weak` view whose informative fraction is zero be a
`fail` rather than `unmeasured`, and where should the informative floor be
declared, in 25.1 beside the tolerance or in `render-params.json` beside
`informationFloor`? The second is where the existing saturation threshold
lives, and putting a comparator threshold there means the reference half's
configuration decides a comparator verdict.
*Recommendation*: declare the floor in `tools/oracle/src/tolerance.rs` beside
the 25.1 constants, and keep zero-information views as `unmeasured` rather than
`fail`, because they are a corpus problem and failing them would pressure
somebody to widen a window.

**6. The two decimated rows: `unmeasured`, or excluded, or re-rendered?**
This plan's decision is `unmeasured` with a gating geometry component.
*Blocks*: the verdict for 2 of 89 views, one of which is also the only real
class-two row.
*Evidence*: 25.1's quarter-canvas-pixel geometry bound is 0.57 source pixels at
the DX row's scale, so it does not guarantee the two sides sample the same
source pixel, and no written tolerance covers a decimated frame.
*Recommendation*: keep this plan's answer. The alternative that would make them
measurable, a magnified render for those two rows, is a change to
`render-params.json` and therefore to every reference frame, and belongs in its
own reviewed story.

**7. Is a comparator with no Ocelli input an acceptable done state for F-011?**
There is no port code, by D7, so there is no second side of the corpus.
*Blocks*: the story's whole shape.
*This plan's answer*: yes, and the three exercises in `## Approach` are what
make it non-vacuous, hand-constructed fixtures in the floor gate, identity over
the real output, and a declared mutation set that proves detection.
*Confirm*: that the operator does not want a stub Ocelli renderer built to give
the comparator something to eat. This plan refuses to build one, because a stub
renderer would be port code written before it is designed, and its frames would
prove only that the stub and the comparator agree.

**8. HLD 18.3's fixture table, row 1, LINEAR_EXACT.**
The table gives `LINEAR_EXACT(-160) = 1.594`. 18.2's transcribed LINEAR_EXACT
clamps at `x <= c - w/2`, which is `40 - 200 = -160`, and `-160 <= -160` holds,
so the value is `ymin`, which is 0.000. Ignoring the clamp does not help,
because the formula body at that input is
`((-160 - 40) / 400 + 0.5) * 255 = (-0.5 + 0.5) * 255 = 0.000`. Rows 2, 3 and 4
of the same table reproduce exactly from the transcribed formulas, and the
0.32 headline figure is unaffected.
*Blocks*: which value F-011's fixture cites, and whether 18.3's table may be
used as a fixture source at all. It also blocks the same question for
`crates/ocelli-pixel/src/lut.rs`, which is entry 3 of the first-ten-files list
and whose fixtures 18.3 says "must be in the test suite before the shader is
written".
*Recommendation*: this plan's fixtures compute from 18.2's formulas and cite
PS3.3 directly, and the discrepancy is recorded rather than reconciled by
guessing. A candidate deviation row, for the operator at the next free number:
"HLD says: §18.3's fixture table gives LINEAR_EXACT(-160) = 1.594 at centre 40,
width 400, range 0-255. We do: fixtures compute LINEAR_EXACT from §18.2's
transcribed formula, which yields 0.000 at that input. Why: §18.2 clamps at
`x <= c - w/2 = -160` and the formula body evaluates to 0.000 there in any
case. The other three rows of the table reproduce exactly. §18.2 is the formula
and §18.3 is a worked value, and where they disagree the formula is the
specification. Raised: F-011." A follow-up story to reconcile the table is
worth opening, the way F-X012 was opened for the SIGMOID case.

**9. The deviation numbering collision.**
`docs/hld/DEVIATIONS.md` runs to twelve rows, and the HLD's own decision log
runs to D14. `CLAUDE.md` already warns that "D7 and D-07 are different things".
Items 3 and 8 would add the thirteenth and fourteenth rows, at which point the
two namespaces overlap completely and the fourteenth deviation is one hyphen
apart from decision D14, where one of them means "claim measured divergence"
and the other would mean "18.3's table row is not the formula".
*Blocks*: nothing technical, but it is cheaper to decide the convention before
two rows are added in one sprint than after.
*Options*: continue the sequence and rely on the hyphen, or move deviations to
a prefix that cannot collide.

**10. `serde` and `serde_json` in `[workspace.dependencies]`.**
The comparator reads `run.json` and eighty-nine sidecars. Neither crate is in
HLD 15.2's dependency list.
*Blocks*: the `Cargo.toml` change.
*Precedent*: `proptest` and `trybuild` are already in that block with a comment
saying they are dev tooling outside 15.2's scope and no deviation row.
`ocelli-oracle` is `publish = false` test infrastructure, so neither crate
enters the shipped surface or the wasm size budget.
*Recommendation*: comment-only, following the existing precedent. Confirm, or
require a row.

**11. Where the comparator's output lives, and who owns the guard change.**
A difference visualisation of a real corpus row is a rendered picture of
patient data, exactly like a reference frame, and every real row carries
`burned-in-unchecked`. So the output directory needs the same treatment
`tools/oracle/out/` gets: an entry in `.gitignore` and, because `git add -f`
exists, an entry in `ORACLE_OUTPUT_PREFIXES` in
`scripts/staged_content_check.py`.
*Blocks*: merge order against F-X009, which is giving every guard a standing
test in this same sprint and touches that same file.
*The collision is sharper than a shared file.* F-X009's plan puts
`ORACLE_OUTPUT_PREFIXES` in its declared-constant ratchet, and states that "a
change to any of these fails the census until the recorded value is updated in
the same change". So if F-X009 lands first, F-011's one-line prefix addition
fails F-X009's census until F-011 also updates the catalogue, which is the
ratchet working as designed and is worth knowing before it happens rather than
after.
*Recommendation*: F-011 adds the prefix and the ignore line, F-X009 adds the
standing test for it, and whichever lands second updates the other's record in
the same change. F-011's own new refusals, which are Rust and not scripts,
should be registered in whatever catalogue F-X009 builds rather than left as
the only refusals in the repository with nothing watching them.

**12. The register's strictness, in the direction that cannot be applied yet.**
`unsupported.json` is strict in both directions and
`docs/lld/oracle.md` explains why. The register's only entry is unreachable by
today's corpus, so "an entry no row exercises fails the run" would fail every
run from the first. This plan applies the direction that can be true: an entry
marked `reachable: false` that fires is a failure.
*Blocks*: whether the register is accepted as a mechanism at all, given it is
the one thing in this design that can turn a failure into a non-failure.
*Confirm*: the asymmetry, and that a new entry is reviewed with a rationale the
way a tolerance change is.

**13. Two copies of the canvas-scale derivation.**
`canvasScale` in `tools/oracle/src/params.mjs` computes canvas pixels per
source pixel in JavaScript. The geometry component needs the same derivation in
Rust.
*Blocks*: the geometry component's implementation, and the F-X007 merge.
*Options*: (a) accept two copies, with the Rust one asserted against
`docs/lld/oracle.md`'s hand-computed worked case in the floor gate and
cross-checked against `run.json`'s recorded `canvasPixelsPerSourcePixel` for
the two decimated rows in the oracle gate. (b) Ask the reference half to record
`canvasPixelsPerSourcePixel` for every row rather than only the decimated two,
which removes the second copy and is a change to `src/params.mjs` and
`src/sidecar.mjs`, both of which F-X007 also changes.
*Recommendation*: (b) if F-X007 lands first, (a) otherwise. This is a real
smell either way and it is named rather than hidden, because §18's "do not let
a second copy of this logic appear anywhere" is about the LUT chain and its
reasoning generalises.

**14. Two mechanisms for "the reference is wrong", in one sprint.**
F-X007's plan gives each `volumes[]` entry a `referenceDivergence` field and a
committed `volume-truth.json`, recording where the reference's volume geometry
departs from the truth the corpus generator knows by construction. This plan
gives the comparator a committed `reference-divergence.json` register.
*Blocks*: whether the repository ends S03 with one place a reference divergence
is recorded or two, and it is much cheaper to answer now.
*The distinction this plan believes is real*: F-X007's is a per-run
**measurement** of one specific thing, geometry against a known truth, written
into the run record. This plan's is a **declared expectation** with a match
condition and a PS3.3 citation, committed, and it is the thing that converts a
pixel divergence into an attributed non-failure. They are an observation and a
policy.
*Recommendation*: keep both, and have the comparator read F-X007's
`referenceDivergence` as an input to the attribution ladder at step 3, so a
volume view whose geometry the reference got wrong is attributed to the
reference without a hand-written register entry. Confirm, or unify.

---

## Write set

Everything the implementation will create or modify. Ledgers written by
`/complete-feature` are excluded, as are gitignored scratch files.

**Create**

- `tools/oracle/src/frame.rs`
- `tools/oracle/src/sidecar.rs`
- `tools/oracle/src/tolerance.rs`
- `tools/oracle/src/geometry.rs`
- `tools/oracle/src/attribution.rs`
- `tools/oracle/src/mutations.rs`
- `tools/oracle/src/report.rs`
- `tools/oracle/src/bin/ocelli-compare.rs`
- `tools/oracle/reference-divergence.json`
- `tools/oracle/compare-expectations.json`
- `tools/oracle/tests/tolerance_fixture.rs`
- `tools/oracle/tests/voi_divergence_fixture.rs`
- `tools/oracle/tests/geometry_fixture.rs`
- `tools/oracle/tests/symmetry.rs`
- `docs/lld/comparator.md`

**Modify**

- `tools/oracle/src/lib.rs`, the scaffold doc comment and `CRATE_NAME` test
  give way to the comparator's module declarations. The crate stops describing
  itself as a scaffold.
- `tools/oracle/Cargo.toml`, dependencies and the `[[bin]]` entry.
- `Cargo.toml`, `serde` and `serde_json` into `[workspace.dependencies]`, each
  with the comment the block's convention requires.
- `bin/ocelli.sh`, a `compare` command, its usage line, and the `oracle` gate
  arm becoming `"$0" oracle && "$0" compare`.
- `scripts/staged_content_check.py`, one prefix in `ORACLE_OUTPUT_PREFIXES`.
- `.gitignore`, one line.
- `docs/lld/oracle.md`, two lines.
- `docs/lld/README.md`, one table row.
- `docs/hld/DEVIATIONS.md`, **only** if the operator approves the candidate
  rows in `## Open questions` items 3 and 8.

**Not touched, deliberately**

`tools/oracle/run.mjs`, `tools/oracle/src/*.mjs`, `tools/oracle/page/`,
`tools/oracle/render-params.json`, `tools/oracle/unsupported.json`,
`tools/oracle/check_sidecars.py`, `scripts/corpus_synth.py`,
`corpus/manifest.tsv`, and every crate under `crates/`.

---

## Out of scope, named

Each of these is somebody's story and none is F-011's.

- Any Ocelli renderer, decoder, LUT chain or port code. D7.
- Re-rendering the sixteen saturated rows at a wider window, or correcting
  their declared window in `scripts/corpus_synth.py`.
- Giving the two decimated rows a magnified render.
- The full metadata diff harness, F-013, E2.5, S04. F-011 diffs only the fields
  that decide the compared pixels and the compared camera.
- Stable render-hash emission, F-015, E2.7, S04.
- The CI gate that renders the corpus per pull request, F-012, E2.4, S04.
- Volume and MPR views, F-X007, beyond not assuming stack-only input and
  writing the contract down.
- Resolving the reference's SIGMOID divergence, F-X012, S04. F-011 builds the
  register that holds it.
- A perceptual colour metric, and any CIEDE2000 implementation.
- Standing tests for the repository's existing guards, F-X009. F-011 provides
  standing tests for its own.

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round. Four of
these were the operator's and are marked as such.

**1. "1 LSB" is one 8-bit display code.** The reference emits RGBA8 canvas
frames and nothing else, so the 16-bit reading is not evaluable against this
instrument. Adopt the 8-bit reading explicitly in the comparator's constants and
say so at the site, so a later reader cannot assume the stricter one was meant.

**2. OPERATOR DECISION. A bias bound is added to HLD 25.1 and F-011 gates on
it.** `docs/hld/22-testing-and-tolerance.md` now carries a fourth bullet:
signed mean difference over the image rectangle within **0.1 of one display
code**, evaluated only where input identity, declared parameters and geometry
already agree. The reason is this plan's own finding, that
`LINEAR(x) - LINEAR_EXACT(x) = 255 * (x + 160) / 159600` peaks at 0.6375 of a
code and so can never exceed one code after quantisation, which makes the
existing maximum-difference rule pass the project's headline defect everywhere.

The bound is **0.1** and the derivation is on the record rather than chosen for
roundness. It sits roughly three times below the 0.32 mean the section 18.3
worked example produces at the window centre, and far above the zero expected
when two implementations agree, because the divergence is one-sided and
rounding noise is not. **The known risk is stated rather than discovered later:
two correct implementations using different rounding conventions could carry a
small systematic bias with no defect behind it.** That is why the bound is
evaluated only after the parameter rung has already agreed, and why the
rounding convention is part of what the parameter rung compares. If a real
divergence is later measured that this bound misclassifies, widening it is a
reviewed change and not a fix.

**3. OPERATOR DECISION. Class two returns `unmeasured` and publishes
statistics.** Deviation **D-16** is applied. The comparator measures per-channel
difference statistics and never returns `pass` for a class-two view, because a
pass would be a claim against a bound nobody wrote. Metric and threshold are
chosen together in a later story.

**4. 8-bit monochrome stays in class two.** The operator did not add a fourth
class to 25.1 for it, so `real/us_cmb_crc/00000001.dcm` remains absorbed by
modality per `docs/lld/corpus.md` and collects both the class-two and the
decimated qualifier. Record it in `docs/lld/comparator.md` as a known gap with
the two qualifiers named, so the next 8-bit greyscale row does not re-derive it.

**5. The sixteen low-information rows keep this plan's answer.** Measure
`informativeFraction`, qualifier `weak`, outcome `unmeasured`, do not re-render.
**The floor is declared in `tools/oracle/src/tolerance.rs` beside the 25.1
constants**, not in `render-params.json`, because the reference half's
configuration must not decide a comparator verdict. A zero-information view
stays `unmeasured` rather than `fail`, since it is a corpus problem and failing
it would pressure somebody to widen a window.

**6. The two decimated rows keep this plan's answer**, `unmeasured` with a
gating geometry component. A magnified render for those two is a change to
`render-params.json` and therefore to every reference frame, and belongs in its
own reviewed story.

**7. Yes, F-011 ships with no Ocelli input, and no stub renderer is built.**
Decision D7 is that the oracle exists before the port code. A stub renderer
would be port code written before it is designed and its frames would prove only
that the stub and the comparator agree. The three exercises in `## Approach` are
what make the story non-vacuous.

**8. OPERATOR DECISION. HLD 18.3 row 1. Deviation D-13 is applied.** Fixtures
compute LINEAR_EXACT from section 18.2's formula, which yields `0.000` at
`x = -160`. `docs/hld/15-lut-chain.md` is not edited, because 18.3 is a worked
value the author computed and the deviation register is where a departure is
recorded. **A follow-up story to reconcile the table is opened**, the way F-X012
was opened for the SIGMOID case. Note for whoever takes it: the row cannot
demonstrate what it intends at any input, because both lower boundaries are
-160, and the asymmetry lives at the upper bound where LINEAR clamps at 239 and
LINEAR_EXACT at 240.

**9. The deviation numbering convention is unchanged.** Rows continue as D-13 to
D-16 and the hyphen is what distinguishes them from decisions D1 to D14.
`CLAUDE.md` already carries the warning and it is now load-bearing rather than
precautionary, since D-14 and decision D14 both exist. Nothing is renumbered,
because renumbering would invalidate every citation already written.

**10. `serde` and `serde_json` enter `[workspace.dependencies]` with a comment
and no deviation row**, following the `proptest` and `trybuild` precedent
already in that block. `ocelli-oracle` is `publish = false` test infrastructure,
so neither crate enters the shipped surface or the wasm size budget.

**11. F-011 adds the output prefix to `scripts/staged_content_check.py` and the
`.gitignore` line. F-X009 lands last and absorbs it** into the declared-constant
ratchet and adds the standing test. F-011's own new refusals are Rust rather
than scripts and are listed in its handoff so F-X009's census registers them
rather than leaving them as the only refusals in the repository with nothing
watching them.

**12. The register's asymmetric strictness is accepted.** An entry marked
`reachable: false` that fires is a failure, and the other direction is not
applied while the only entry is unreachable by today's corpus. A new entry is
reviewed with a rationale exactly as a tolerance change is, and
`docs/lld/comparator.md` says so.

**13. The canvas-scale derivation is not duplicated.** Take option (b):
**F-X007 records `canvasPixelsPerSourcePixel` for every row** rather than only
the two decimated ones, and F-011 reads it. F-X007 is in the same wave and
already owns `src/params.mjs` and `src/sidecar.mjs`, so this is one line there
against a second copy of a derivation here. Section 18's "do not let a second
copy of this logic appear anywhere" is about the LUT chain and its reasoning
generalises.

**14. Both reference-divergence mechanisms are kept, and they are connected.**
F-X007's `referenceDivergence` is a per-run measurement of geometry against a
known truth. This plan's `reference-divergence.json` is a declared expectation
with a match condition and a PS3.3 citation. The comparator **reads F-X007's
field as an input to the attribution ladder at rung 3**, so a volume view whose
geometry the reference got wrong is attributed without a hand-written entry.
`docs/lld/comparator.md` states the distinction so the next person does not
unify them by accident.
