# F-018, Image plane, pixel, modality-LUT and VOI-LUT modules

**Status**: approved
**Epic ref**: E3.3
**Sprint**: S08
**Estimate**: 3w

## Normative source, transcribed

_The quotations below normalise HLD prose punctuation where the enforced
Markdown voice rules require it. Formula text and signatures are exact._

### `docs/hld/13-core-types.md`, sections 16 and 16.1

```rust
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

```rust
#[derive(Clone, Copy, Debug)] pub struct Stored(pub f32); // raw from pixel data
#[derive(Clone, Copy, Debug)] pub struct Modality(pub f32); // after rescale; HU for CT
#[derive(Clone, Copy, Debug)] pub struct Display(pub f32); // after VOI, in [ymin, ymax]
```

### `docs/hld/15-lut-chain.md`, sections 18 through 18.3

> This is the highest-risk arithmetic in the project. It is specified in
> DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once,
> in ocelli-pixel, and let the shader read the parameters. Do not let a second
> copy of this logic appear anywhere.

| **Stage** | **From -> To** | **Source** |
|----|----|----|
| 1. Modality LUT | Stored -> Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality -> Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display -> Display | Identity or INVERSE. Presentation state may override |
| 4. Palette / ICC | Display -> RGB | Palette colour LUT, or the display colour pipeline |

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

> If a Modality LUT Sequence is present it wins over slope and intercept. PET
> SUV is a separate path and needs the radiopharmaceutical sequence. Do not
> fold it in here.

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

The HLD section 18.3 fixture is transcribed with deviation D-13 applied to the
first `LINEAR_EXACT` value, since the formula clamps `-160` to `0.000`.

| Input (HU) | LINEAR | LINEAR_EXACT |
|----|----|----|
| -160 | 0.000 | 0.000 |
| 40 | 127.819 | 127.500 |
| 240 | 255.000 | 255.000 |
| -60 | 63.910 | 63.750 |

### DICOM PS3.3 C.7.6.2.1.1, C.7.6.3.3, and 10.7.1.3

```text
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

Image Position Patient is the centre of the first voxel. `X` is the row
direction cosine, `Y` is the column direction cosine, `i` is the column index,
and `j` is the row index. The row and column direction cosines have unit length
and are mutually orthogonal. High Bit shall be one less than Bits Stored. Pixel
Spacing values are positive except that row spacing may be zero for a
single-row image and column spacing may be zero for a single-column image.

### DICOM PS3.3 C.7.6.3 and C.11

```text
value = raw >> (HighBit + 1 - BitsStored)
value = value & ((1 << BitsStored) - 1)
if PixelRepresentation == 1:
    sign-extend from bit (BitsStored - 1)
```

Modality LUT Sequence takes precedence over rescale slope and intercept. VOI
LUT Sequence takes precedence over Window Center and Window Width. LUT Data
entries are unsigned integers from zero through `2^n - 1`, where `n` is the
descriptor's bits per entry.

## What the specification does not cover

The HLD does not define validated Rust types for Image Position Patient,
Image Orientation Patient, Pixel Spacing, the Image Pixel module, LUT
descriptors, multi-valued window pairs, or malformed-input errors. It also
does not settle whether a pixel decoder returns stored containers or already
unpacked signed samples. This story keeps codec output as DICOM container bytes
and makes unpacking an explicit `ocelli-pixel` operation, so Bits Stored and
Pixel Representation are interpreted once.

`MONOCHROME1` is represented as evidence on the frame. Presentation inversion
is not applied by modality or VOI mapping, which prevents applying it twice.
Presentation LUT and palette or ICC execution remain later work.

## Approach

1. Add compact validated types in `ocelli-pixel` for image-plane geometry and
   stored-pixel description. Reuse `Pt<World>`, `Pt<Index>`, `Transform`,
   `Stored`, `Modality`, and `Display` from `ocelli-core`.
2. Validate finite image position, finite unit-length and mutually orthogonal
   row and column direction cosines within a dimensionless `2e-6` tolerance,
   then normalise and orthogonalise accepted rounding noise before storage. A
   component rounded to six decimal places differs by at most `0.5e-6`.
   Cauchy-Schwarz bounds the accumulated dot error below
   `2 * sqrt(3) * 0.5e-6 + 3 * (0.5e-6)^2`, about `1.733e-6`, so `2e-6` is a
   conservative input bound. Validate finite nonnegative row and column
   spacing, permitting zero only when its corresponding row or column
   dimension is one. Validate nonzero dimensions, supported sample containers,
   conforming High Bit, signedness, samples per pixel, and planar configuration
   rules.
3. Construct the index-to-world transform from the PS3.3 equation. Keep row
   and column spacing named fields so equal scalar types cannot be swapped. A
   legal zero singleton-axis spacing uses a unit extension along that unused
   transform axis. Every valid index on it is zero, so this preserves all
   represented voxel positions while retaining an invertible transform.
4. Unpack 8, 16, and 32-bit little-endian and big-endian containers by masking
   Bits Stored and sign-extending from `BitsStored - 1`. Refuse a destination
   of the wrong length before writing it.
5. Represent modality selection as either a descriptor-backed LUT or rescale.
   Construction enforces sequence precedence and validates LUT Data as unsigned
   integers bounded by the descriptor's bits per entry. Apply the HLD signature
   for the rescale case and clamped indexed lookup for a LUT descriptor.
6. Represent VOI selection as either a descriptor-backed LUT or one selected
   window pair and `LINEAR`, `LINEAR_EXACT`, or `SIGMOID`. Construction
   enforces sequence precedence and the exact width preconditions.
7. Keep all arithmetic in `ocelli-pixel`. The later shader consumes parameters
   and does not introduce a second policy implementation.
8. Mutate one spacing index, one sign bit, modality precedence, each VOI bound,
   and the SIGMOID exponent sign. Run the named fixtures red and revert.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. LUT descriptors own setup-time data and all
  mapping methods borrow caller-provided storage
- unsafe: none
- Tier A (WebGPU): full CPU-side parameter and evidence preparation
- Tier B (WebGL2): full CPU-side parameter and evidence preparation
- Tier C (CPU): full and authoritative arithmetic implementation

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Non-square Pixel Spacing maps column and row indices through the correct direction cosine, with Image Position Patient at the first voxel centre, citing PS3.3 C.7.6.2.1.1 | `crates/ocelli-pixel/tests/image_plane.rs` |
| fixture | 12-bit signed and unsigned values in 16-bit containers mask and sign-extend from bit 11, citing PS3.3 C.7.6.3 | `crates/ocelli-pixel/tests/stored_pixel.rs` |
| fixture | Rescale and Modality LUT Sequence precedence produce hand-computed values, citing PS3.3 C.11.1 | `crates/ocelli-pixel/tests/modality.rs` |
| fixture | The four HLD section 18.3 rows, corrected by D-13, plus exact lower and upper comparisons and width refusal | `crates/ocelli-pixel/tests/voi.rs` |
| fixture | SIGMOID at input `-60` is `255 / (1 + e)` and a non-positive width is refused, citing PS3.3 C.11.2.1.3.1 | `crates/ocelli-pixel/tests/voi.rs` |
| unit | Malformed dimensions, bit fields, non-unit or skewed plane vectors, spacing outside the singleton exception, descriptor lengths, negative, fractional or overflowing LUT Data, and mismatched window multiplicity are refused before output mutation | module and fixture tests |
| property | An invertible image-plane transform round-trips representative index points within the existing coordinate epsilon | `crates/ocelli-pixel/tests/image_plane.rs` |
| mutation | Spacing swap, sign-bit shift, precedence reversal, VOI comparison, and exponent-sign changes make named fixtures fail | feature review evidence |
| cross-target | The same modules compile for native and `wasm32-unknown-unknown` without wasm-bindgen | `bin/ocelli.sh gate native` and `bin/ocelli.sh cargo check -p ocelli-pixel --all-targets --target wasm32-unknown-unknown` |

## Parity surface covered

- Appendix B `VOI LUT functions`: `LINEAR`, `LINEAR_EXACT`, and the listed
  sampled SIGMOID behavior.
- Appendix B `Transfer syntaxes` is not claimed. This story consumes decoded
  stored containers and does not register a decoder.

## Deviations

- D-13 supplies the corrected `LINEAR_EXACT(-160) = 0.000` fixture value.
- No new deviation is planned.

## LLD impact

- Create `docs/lld/pixel-pipeline.md` with image-plane evidence, stored-value
  extraction, precedence, arithmetic ownership, and presentation inversion.
- Update `docs/lld/README.md` and `docs/lld/core-types.md`.

## Write set

- `crates/ocelli-pixel/Cargo.toml`
- `crates/ocelli-pixel/src/lib.rs`
- new source and fixture files under `crates/ocelli-pixel/`
- `docs/lld/pixel-pipeline.md`
- `docs/lld/README.md`
- `docs/lld/core-types.md`
- shared sprint ledgers during completion

## Open questions

None.
