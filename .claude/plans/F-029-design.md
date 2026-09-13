# F-029, LUT chain: modality, VOI (linear / exact / sigmoid), presentation, invert

**Status**: approved
**Epic ref**: E4.7
**Sprint**: S09
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/15-lut-chain.md`, section 18, opening

Transcribed into table rows, which is where `scripts/prose_check.py` relaxes the
voice rules, so the author's text is quoted exactly.

|  |
|----|
| This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

The stage table, transcribed:

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.1, the modality stage

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

|  |
|----|
| *If a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence — do not fold it in here.* |

### Section 18.2, the three VOI functions, character for character

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

### Section 18.3, the fixture table

Soft-tissue CT window, centre 40, width 400, output range 0 to 255.

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

**Row one's `LINEAR_EXACT` value is `0.000`, not `1.594`, under declared
deviation D-13.** Section 18.2 clamps at `x <= c - w/2 = -160`, and
`-160 <= -160` holds. The formula body also evaluates to `0.000` there, from
`((-160 - 40) / 400 + 0.5) * 255`. Section 18.2 is the formula, section 18.3 is
a worked value, and the formula is the specification.

### Section 18.4, the shader side

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

> *A window-level drag updates thirty-two bytes per frame. No texture is
> re-uploaded, and that is the concrete performance claim behind Figure 2.*

**`invert` is one `u32`, and that is the strongest single piece of evidence for
this story's shape.** The shader receives a single resolved inversion flag. It
does not receive a photometric interpretation and a presentation LUT shape and
combine them. Whatever combines them runs on the CPU, exactly once, and F-029 is
the story that writes it.

### DICOM PS3.3 C.11.6, the Presentation LUT

Presentation LUT Shape (2050,0020), VR `CS`, enumerated values `IDENTITY` and
`INVERSE`. `IDENTITY` means the VOI LUT output is already P-Values.
`INVERSE` means the output is inverted before becoming P-Values. A Presentation
LUT Sequence (2050,0010) may carry an explicit LUT instead, and when it is
present Presentation LUT Shape is absent.

### DICOM PS3.3 C.7.6.3.1.2, Photometric Interpretation

`MONOCHROME1` means the minimum stored value is displayed as **white**.
`MONOCHROME2` means the minimum stored value is displayed as black.

## What the specification does not cover

1. **How `MONOCHROME1` and Presentation LUT Shape combine.** Both express an
   inversion. The HLD's stage table, row 3, says "Identity or INVERSE" and then
   "presentation state may override", and stops. PS3.3 names both mechanisms and does not write the
   composition rule as an equation. **This is the single decision in the story
   and it is the one that produces a correct-looking image with wrong values if
   it goes wrong.** The proposed rule is in Approach and it is also Open
   question 1, because it is a decision rather than a transcription.
2. **The output range of a VOI LUT Sequence.** Inversion needs `ymin` and
   `ymax`. For a window they are given. For a `VOILUTSequence` the current
   `VoiTransform` does not retain them. PS3.3 C.11.2.1.1 defines the LUT's
   output range from the descriptor's third value, bits per entry, so the range
   is `0 ..= 2^bits - 1`. This plan uses that and documents it.
3. **Whether stage 4 is in scope.** `docs/sprints/CURRENT_SPRINT.md` says
   "Palette colour and ICC execution stay out of scope unless F-029's design
   plan brings them in explicitly." **This plan does not bring them in.** The
   reason is stated in Approach.
4. **Where the chain's parameters become the section 18.4 uniform.** That is a
   rendering story. F-029 exposes the six scalars and the two `u32` discriminants
   as plain accessors so a later shader story reads them, and writes no WGSL and
   no `#[repr(C)]` struct, because a uniform layout with no shader to consume it
   is a layout nobody can check.

## Approach

**This story adds two things and changes nothing else.** F-018 already
implemented stages 1 and 2 in `crates/ocelli-pixel/src/lut.rs`:
`ModalityTransform` with sequence precedence, `VoiTransform` with sequence
precedence, `VoiFunction::{Linear, LinearExact, Sigmoid}`, the transcribed
formulas, the asymmetric `<=` and `>` comparisons, and the section 18.3 fixtures
under D-13. **The story title names those stages again and the title is not a
licence to reimplement them.** `crates/ocelli-pixel/tests/voi.rs` and
`modality.rs` stay as they are.

### 1. The presentation stage

```rust
/// DICOM PS3.3 C.11.6 Presentation LUT Shape.
pub enum PresentationLutShape { Identity, Inverse }

/// Stage 3. Display -> Display, within one declared output range.
pub struct PresentationTransform {
    shape: PresentationLutShape,
    ymin: f32,
    ymax: f32,
}

impl PresentationTransform {
    pub fn apply(&self, display: Display) -> Display {
        match self.shape {
            PresentationLutShape::Identity => display,
            // PS3.3 C.11.6.1.2. A reflection within the output range.
            PresentationLutShape::Inverse => Display(self.ymin + self.ymax - display.0),
        }
    }
}
```

`ymin + ymax - y` is the reflection of `y` about the midpoint of `[ymin, ymax]`.
For the common `0 ..= 255` range it reduces to `255 - y`, and writing the
general form is what makes it correct for a `VOILUTSequence` whose range is
`0 ..= 65535` and for a non-zero `ymin`.

### 2. Resolving inversion exactly once

```rust
pub struct LutChain {
    modality: ModalityTransform,
    voi: VoiTransform,
    presentation: PresentationTransform,
}

impl LutChain {
    /// Stages 1 to 3 in PS3.3 C.11's order, applied once each.
    pub fn new(
        modality: ModalityTransform,
        voi: VoiTransform,
        photometric: PhotometricInterpretation,
        declared_shape: Option<PresentationLutShape>,
    ) -> Result<Self, PixelError>;

    pub fn apply(&self, stored: Stored) -> Display;
    pub fn map_into(&self, src: &[Stored], dst: &mut [Display]) -> Result<(), PixelError>;

    /// The single resolved flag HLD section 18.4's `invert : u32` carries.
    pub fn inverts(&self) -> bool;
}
```

**The resolution rule, which is the whole story:**

| Presentation LUT Shape (2050,0020) | Photometric Interpretation | Resolved shape |
|------------------------------------|----------------------------|----------------|
| `INVERSE` | any monochrome | `Inverse` |
| `IDENTITY` | any monochrome | `Identity` |
| absent | `MONOCHROME1` | `Inverse` |
| absent | `MONOCHROME2` | `Identity` |
| present or absent | `PALETTE COLOR`, `RGB`, any `YBR_*` | refused, `PixelError::PresentationLutNotApplicable` |

**An explicit Presentation LUT Shape decides alone. It is never combined with
the photometric interpretation.** That is the HLD table's "presentation state
may override" read as an override rather than a composition, and it is the
reading that cannot double-invert. A `MONOCHROME1` instance carrying
`IDENTITY` therefore renders un-inverted, which is what the presentation state
asked for.

The alternative reading, composing the two with an exclusive-or, is what
produces the defect `docs/lld/pixel-pipeline.md` already names. It is also
unfalsifiable on a symmetric window, which is why the fixture below is
deliberately asymmetric.

`PhotometricInterpretation::Monochrome1`'s own doc comment in
`crates/ocelli-pixel/src/stored_pixel.rs` already reads "Minimum stored value is
displayed white **at the presentation stage**", so F-018 chose the stage and
F-029 implements the choice it recorded. No inversion is added to stage 1 or
stage 2 and `ModalityTransform` and `VoiTransform` are not edited to accept an
inversion flag.

### 3. What is deliberately not added

- **Stage 4, palette and ICC.** Out of scope by `CURRENT_SPRINT.md` and not
  brought in. Palette colour is a `Display -> RGB` stage over an index, and the
  index is not a `Display` value at all: PS3.3 C.7.9 maps the **stored** value
  through the palette descriptors, so a palette path that took `Display` as its
  input would be wrong in a way stage ordering alone would not reveal. That is a
  story with its own fixture, not a fourth arm on this one.
- **A `VoiParams` `#[repr(C)]` struct.** See What the specification does not
  cover, item 4.
- **A `PresentationLutSequence` (2050,0010).** `LutDescriptor` already exists and
  could carry one, and nothing in this sprint consumes it. A sequence path with
  no corpus row and no caller is a second place to look with no reader, which
  the `AGENTS.md` structural test rejects. The story adds
  `PixelError::PresentationLutSequenceUnsupported` so the route **reports
  unavailable** rather than silently applying the shape instead, which is HLD
  section 31's rule generalised.

### 4. One accessor added to `VoiTransform`

```rust
/// The stage's output range, needed by stage 3's reflection.
///
/// A window carries its declared `[ymin, ymax]`. A VOI LUT Sequence's range is
/// PS3.3 C.11.2.1.1's `0 ..= 2^bits_per_entry - 1`.
pub fn output_range(&self) -> (f32, f32);
```

This is an accessor over state `VoiSelection` already holds, plus one derived
value for the sequence arm. It adds no arithmetic that stage 2 does not already
own.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. `ocelli-pixel` is `no_std` and holds no
  boundary type. D3 is untouched
- Render-loop allocation: none. `PresentationTransform` and `LutChain` are
  plain scalar state, `apply` allocates nothing, and `map_into` writes into
  caller storage and refuses a length mismatch before the first write, which is
  the contract `ModalityTransform::map_into` and `VoiTransform::map_into`
  already hold
- unsafe: none
- Tier A (WebGPU): full, by the shader reading `LutChain`'s scalars through
  section 18.4's uniform. **No WGSL is written by this story**, so the tier-A
  claim is that the parameters exist in the shape 18.4 names, not that a shader
  consumes them yet
- Tier B (WebGL2): full, same parameters, same values. The arithmetic is
  identical and there is no tier-specific branch
- Tier C (CPU): **full, and this is the authoritative path.** Deviation D-07
  states tier C reuses `ocelli-pixel` rather than reimplementing the LUT chain,
  and `LutChain::map_into` is exactly the entry point that makes that true

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | The four section 18.3 rows for `LINEAR` and `LINEAR_EXACT`, D-13 applied, **already present** in `crates/ocelli-pixel/tests/voi.rs` and re-asserted through `LutChain` end to end so the composition cannot silently change them | `crates/ocelli-pixel/tests/lut_chain.rs` |
| `fixture` | **Inversion applies exactly once.** Centre 40, width 400, `MONOCHROME1`, no declared shape, input 40 HU. Stage 2 gives `127.819548`, stage 3 gives `0 + 255 - 127.819548 = 127.180451`. A double inversion returns `127.819548`, and the two differ, citing PS3.3 C.11.2.1.2 and C.11.6 | same |
| `fixture` | **The asymmetric row, which is the one that matters.** Input `-60` HU: stage 2 `63.910`, inverted `191.090`. Double inversion gives `63.910` again. A symmetric window cannot separate these and this row can | same |
| `fixture` | `MONOCHROME1` plus an explicit `IDENTITY` resolves to `Identity`, so the image is **not** inverted. This is the override decision made checkable | same |
| `fixture` | `MONOCHROME2` plus an explicit `INVERSE` resolves to `Inverse` | same |
| `fixture` | Inversion over a non-zero `ymin`: range `[16, 235]`, input mapping to `100`, inverted is `16 + 235 - 100 = 151`, hand-computed | same |
| `fixture` | A `VOILUTSequence` with `bits_per_entry` 16 inverts about `0 ..= 65535`, citing PS3.3 C.11.2.1.1 | same |
| `unit` | Every colour photometric interpretation refuses the presentation stage | `crates/ocelli-pixel/src/lut.rs` |
| `unit` | A declared Presentation LUT Sequence reports unsupported rather than falling back to the shape | same |
| `unit` | `map_into` refuses a mismatched destination before writing, asserted by leaving a sentinel intact | same |
| `property` | For `Identity`, `LutChain::apply` equals `voi.apply(modality.apply(x))` for a randomised sweep, so composition adds nothing | `crates/ocelli-pixel/tests/lut_chain.rs` |
| `property` | For `Inverse`, applying stage 3 twice is the identity within `1e-6`, which is the reflection's defining property | same |

**Mutation check, HLD 27.3.** The inversion fixtures are observed red under
three mutations, each named in the implementation note: change the resolution
rule from override to exclusive-or, change the reflection to `ymax - y` so a
non-zero `ymin` breaks, and remove the presentation stage entirely. The third
must go red on the asymmetric row and not only on a boundary value.

## Parity surface covered

`docs/hld/B-parity-surface.md`'s surface table has a row **VOI LUT functions,
count 3, "LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID"**. The three functions were
delivered by F-018 and F-029 does not add a fourth.

**The appendix says `SAMPLED_SIGMOID` and DICOM PS3.3 C.11.2.1.3.1 says
`SIGMOID`.** The implemented `VoiFunction::Sigmoid` matches the standard's
enumerated value, which is what a file on disk actually carries. This is
recorded as an observation rather than a deviation, because the appendix is a
count of cornerstone3D's surface rather than a specification of ours, and it has
no `Covered by` column in this repository to update.

## Deviations

**D-13, already declared**, and this story is its second consumer after F-011.
No new row.

No new deviation is expected. If the operator answers Open question 1 the other
way, the composition rule becomes a departure from nothing written, so it still
needs no `D-NN` row, only a different table in this plan.

## LLD impact

- `docs/lld/pixel-pipeline.md`, which currently says "Presentation inversion and
  palette or ICC execution are not implemented yet" and "`MONOCHROME1` remains
  explicit photometric evidence on `SampleLayout`, so a later presentation stage
  can invert exactly once". Both sentences are replaced by the resolution table
  and the exactly-once claim with the fixture that holds it

## Open questions

None. Both were answered in the S09 consolidated design round.

## Decisions from the S09 design round

**1. Override, not compose.** An explicit Presentation LUT Shape decides alone
and is never combined with `MONOCHROME1`. The resolution table in Approach is
the approved rule and it is the one that cannot double-invert. The exclusive-or
reading was considered and rejected: it is the shape
`docs/lld/pixel-pipeline.md` already names as the defect, and it is
unfalsifiable on a symmetric window. **The consequence to hold in review: a
`MONOCHROME1` instance carrying an explicit `IDENTITY` renders un-inverted.**
That is the presentation state being honoured, and a reviewer who expects
`MONOCHROME1` to always invert will read it as a bug, so the fixture asserting
it carries that sentence in its comment.

**2. A Presentation LUT Sequence reports unsupported.** Not implemented this
story. `LutDescriptor` exists and implementing it would be small, and there is
no corpus row, so the implementation would be tested only by a synthetic fixture
asserting what the code does, which HLD 27.2 R2 rejects. Reporting unavailable
is the honest state and it is HLD section 31's rule.
