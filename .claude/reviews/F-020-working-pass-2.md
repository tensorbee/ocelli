# F-020 review, pass 2

**Reviewed**: working tree after pass 1's remediation,
`crates/ocelli-dicom/src/frame_geometry.rs`, `lib.rs`, `multiframe.rs`,
`Cargo.toml`, `crates/ocelli-pixel/src/image_plane.rs`,
`crates/ocelli-dicom/tests/frame_geometry.rs`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## What pass 1 raised, and what closed it

### S1, an unreachable default reported a source for an absent attribute

Closed by moving the decision rather than by testing the default. The Pixel
Spacing absence is now decided in `FrameGeometry::derive`, before the source is
needed, so `pixel_spacing` is a plain value afterwards and there is no default
to get wrong. `spacing_evidence` takes a present element and an already-read
`Option<PixelSpacing>`, and Imager Pixel Spacing gets its own reader, which also
says in one place that the attribute belongs to no functional group macro.

The three duplicate sets now fold in one expression rather than through a
`mut` accumulator with a conditional arm.

### S2, the empty-steps arm returned a semantically wrong error

Closed. `StackGeometry::derive` is now a single pass with no `Vec`: it carries
`first_step_mm: Option<f64>` and a `uniform` flag, comparing each projected step
against the first as it goes. The no-step case returns
`StackShear::NotApplicable`, which is a true statement about an instance with no
inter-frame vector, rather than a refusal claiming non-uniform spacing. **Every
arm is now a true statement even where it is unreachable**, which is the
property worth having, since an unreachable arm's job is to be right if the
surrounding invariant ever changes.

The `Vec::with_capacity(frame_count - 1)` allocation is gone as a side effect.

### S3, the tolerance's stated justification was not a derivation

Closed, and the number is unchanged at `1e-6 mm`. **A tolerance was not widened
or narrowed.** Only the reason is replaced, with one that can be checked: Image
Position Patient arrives as a Decimal String, which PS3.5 section 6.2 caps at
sixteen characters, and a patient coordinate routinely carries three or four
digits before the point, so the attribute can express about eleven decimal
places at the very most and real files carry four to six. `1e-6 mm` is therefore
below the precision the source attribute can state, which is the property
wanted: it absorbs `f64` arithmetic noise and can never mask a difference a file
could actually express.

## What pass 2 found, fixed and re-verified

### Two source accessors had no fixture

`FrameGeometrySources::pixel_spacing` and `::orientation` were public, returned a
resolved value, and nothing asserted either. Found by probe rather than by
reading: replacing `pixel_spacing: pixel_spacing.source()` with a hard-coded
`FunctionalGroupSource::Shared` left the suite green at 16 passed.

The per-frame fixture now asserts all four of `position`, `orientation`,
`pixel_spacing` and `duplicates`, and a second block asserts the other end of
PS3.3 C.7.6.16.1.1's order, a legacy instance answering all three from the main
data set with an empty duplicate set. All three accessors are now falsifiable.

### A probe silently did nothing, for the second time this sprint

The M4 probe replaced `UncalibratedSpacingOnly` with `MissingPixelSpacing`, and
that spelling already occurs in the same `if` expression's other arm, so the
script's "replacement already present" guard fired and nothing was mutated. The
test run in the same command then printed `ok. 16 passed`, which reads exactly
like the refusal being untested.

This is the same failure mode F-029's review recorded, arriving from the other
direction: there the anchor was ambiguous, here the replacement was already
present. **Both guards are needed and both have now fired on real work.** The
probe was re-run with a spelling absent from the file and the refusal fails a
test by name.

## Verified clean

- **Sixteen fixtures pass.** The `frame_geometry` target reports sixteen passed
  and none failed. The whole crate is green across its ten targets, and
  `bin/ocelli.sh clippy ocelli-dicom` exits 0.
- **Eight mutations, each observed red, each reverted**, with both source files
  restored from byte copies and the suite re-run green afterwards:

  | Mutation | Result |
  |----------|--------|
  | `PixelSpacing::new(values[0], values[1])` indices swapped | 15 passed, 1 failed |
  | Slice normal negated in `ocelli-pixel` | 14 passed, 2 failed |
  | Shear tolerance widened past 1 mm | 15 passed, 1 failed |
  | Imager-only refusal weakened | 15 passed, 1 failed |
  | Uniformity check defeated | 15 passed, 1 failed |
  | Pixel Spacing source falsified | 15 passed, 1 failed |
  | Position source falsified | 15 passed, 1 failed |
  | Orientation source falsified | 15 passed, 1 failed |

- **PS3.3 C.7.6.2.1.1 still has one implementation.**
  `FrameGeometry::index_to_world` forwards to `ImagePlane::index_to_world`, and
  `git diff` shows no edit inside that method. The two accessors added to
  `ocelli-pixel`, `slice_normal` and `position_vector`, exist so a cross-frame
  consumer does not re-derive the cross product, which would be the second copy.
- **The spacing index is right and the fixture can tell.** Non-square `2.0` by
  `0.5`, asserting `(12, 26, 30)` at column 4 row 3, hand-derived in the comment
  from C.7.6.2.1.1. A square-pixel fixture could not separate the two indices.
- **Shear is measured from geometry in both directions.** Tilt `0` with sheared
  geometry reports `Sheared`, tilt `30` with axis-aligned geometry reports
  `AxisAligned`. The tag decides nothing and negating the normal fails two tests.
- **The projected step keeps its sign**, asserted at `-2.0` for a descending
  stack rather than at its magnitude.
- **Imager Pixel Spacing is never substituted.** Imager-only refuses with its own
  variant, and the calibrated fixture asserts the derived coordinate uses
  `0.139` rather than `0.175`.
- **No attribute value reaches an error.** Every `FrameGeometryError` variant
  carries only a `Tag`, a `VR`, counts or a `PixelError`. Checked variant by
  variant against the constraint `MultiframeError` already holds.
- **The Decimal String parser refuses what `f64::from_str` accepts.** `inf`,
  `-inf`, `nan` and `1_0` refused, legal padding, `+` and exponent accepted, and
  the sixteen-byte cap counts padding, asserted at fifteen digits plus one and
  plus two pad bytes.
- **Boundary and tier.** No `wasm-bindgen`, no pixel crosses a boundary, no
  `unsafe`, no `as` cast, and the one allocation pass 1 found is gone.
- **Structure.** No new trait, generic or `Box<dyn>`. `DuplicateSources::union`
  and `MultiframeMetadata::metadata` are accessors over existing state.
