# F-020 review, pass 1

**Reviewed**: working tree, `crates/ocelli-dicom/src/frame_geometry.rs`,
`crates/ocelli-dicom/src/lib.rs`, `crates/ocelli-dicom/src/multiframe.rs`,
`crates/ocelli-dicom/Cargo.toml`, `crates/ocelli-pixel/src/image_plane.rs`,
`crates/ocelli-dicom/tests/frame_geometry.rs`
**Result**: 0 defects, 3 smells, 0 nitpicks

## Defects

None.

## Smells

### S1, an unreachable default reports a source for an absent attribute

**Where**: `crates/ocelli-dicom/src/frame_geometry.rs`, `FrameGeometry::derive`,
`pixel_spacing.map_or(FunctionalGroupSource::TopLevel, |value| value.source())`

**What**: The `TopLevel` default says "Pixel Spacing came from the main data
set" for the case where Pixel Spacing resolved to nothing at all. It cannot fire,
because `spacing_evidence` on the line above returns
`MissingPixelSpacing` or `UncalibratedSpacingOnly` when its argument is `None`.
So the default is both unreachable and, if it ever became reachable, wrong.

**Why it is wrong**: `/microscope` section 4. A `map_or` default is the shape
this defect class hides in most comfortably, because it reads as a sensible
fallback rather than as a branch.

**Evidence**: Changed the default from `TopLevel` to `Shared` and re-ran.

```text
P1 pixel_spacing_source default -> Shared     test result: ok. 16 passed; 0 failed
```

Nothing went red, so no fixture distinguishes the two values.

### S2, the empty-steps arm returns a semantically wrong error

**Where**: `crates/ocelli-dicom/src/frame_geometry.rs`, `StackGeometry::derive`,
the `else` arm of the `let Some(step_mm) = projected_steps.first().copied()`
binding, which returns `FrameGeometryError::NonUniformFrameSpacing`.

**What**: `projected_steps` has `frame_count - 1` entries and the function has
already returned for `frame_count < 2`, so it cannot be empty. If it ever were,
"no inter-frame steps" is `StackShear::NotApplicable`, not non-uniform spacing.
The arm also forces a `Vec` allocation that a single pass does not need.

**Why it is wrong**: Same class as S1, with the extra property that the
unreachable branch names a refusal that would be a false statement about the
instance.

**Evidence**: Changed the arm to return `MissingImagePosition` and re-ran.

```text
P2 empty-steps arm -> MissingImagePosition    test result: ok. 16 passed; 0 failed
```

Nothing went red.

### S3, the tolerance constant's stated justification is not a derivation

**Where**: `crates/ocelli-dicom/src/frame_geometry.rs`,
`GEOMETRY_TOLERANCE_MM`'s doc comment, "This is `ocelli-pixel`'s declared `2e-6`
dimensionless direction-cosine tolerance carried into millimetres against a step
no scanner emits at that scale."

**What**: The sentence does not survive reading. `2e-6` is dimensionless and
`1e-6` is in millimetres, and the comment does not say what length the first was
multiplied by to become the second, because it was not multiplied by anything.
The number is fine. The reason given for it is a gesture.

**Why it is wrong**: `/microscope` section 3. A tolerance is the one kind of
constant `.claude/WORKFLOW.md` singles out as a design-plan decision reviewed
like code, so a justification nobody can check is worse here than elsewhere. The
review that matters is the next one, where someone widens it and cites this
comment.

**Evidence**: `2e-6 * 1 mm = 2e-6 mm`, which is not `1e-6 mm`, and no length in
the module is `0.5`. The stated arithmetic does not produce the stated result.

**A justification that does hold** and is checkable: Image Position Patient
arrives as a Decimal String, which PS3.5 section 6.2 caps at sixteen characters.
A patient coordinate is routinely three or four digits before the point, so the
attribute itself carries at most about eleven decimal places and real files
carry four to six. A tolerance of `1e-6 mm` is therefore below the precision the
source attribute can express, which means it absorbs `f64` arithmetic noise and
nothing else. That is the property wanted: the constant must never mask a
difference a file could actually state.

## Nitpicks

None.

## Verified clean

- **The index-to-world transform is not reimplemented.**
  `FrameGeometry::index_to_world` forwards to `ImagePlane::index_to_world`, and
  `git diff` shows no edit inside that method. PS3.3 C.7.6.2.1.1 still has one
  implementation, which was the story's main structural risk.
- **The spacing index is right, and the fixture can tell.**
  `pixel_spacing_row_value_scales_the_column_direction_cosine` uses a
  deliberately non-square `2.0` by `0.5` and asserts `(12, 26, 30)` at column 4,
  row 3, hand-derived from C.7.6.2.1.1 in the comment. Swapping the two indices
  in `pixel_spacing_pair` fails it: 15 passed, 1 failed.
- **Five mutations run, each observed red, each reverted**, with both files
  restored from byte copies and the suite re-run green:

  | Mutation | Result |
  |----------|--------|
  | `PixelSpacing::new(values[0], values[1])` indices swapped | 15 passed, 1 failed |
  | Slice normal negated in `ocelli-pixel` | 14 passed, 2 failed |
  | Shear tolerance widened past 1 mm | 15 passed, 1 failed |
  | Imager-only refusal changed to `MissingPixelSpacing` | 15 passed, 1 failed |
  | Uniformity check defeated | 15 passed, 1 failed |

- **Shear is measured, not read from the tag.** The fixture asserts both
  directions: tilt `0` with sheared geometry reports `Sheared`, and tilt `30`
  with axis-aligned geometry reports `AxisAligned`. Negating the normal in
  `ocelli-pixel` fails two tests, so the projection is load-bearing.
- **The projected step keeps its sign.** A descending stack reports `-2.0`, and
  the fixture asserts the sign rather than the magnitude.
- **Imager Pixel Spacing is never substituted.** Only-imager refuses with a
  distinct variant, and the calibrated case asserts the derived coordinate uses
  `0.139` rather than `0.175`, which is the 26 per cent measurement error that
  would otherwise render correctly.
- **No attribute value reaches an error.** Every `FrameGeometryError` variant
  carries a `Tag`, a `VR`, counts or a `PixelError`, which is the constraint
  `MultiframeError` already holds. Checked variant by variant.
- **The Decimal String parser refuses what `f64::from_str` would accept.**
  `inf`, `-inf`, `nan` and `1_0` are all refused, and the character filter runs
  before the parse, which is why `1_0` is caught rather than relied on. Legal
  spellings, retained padding, a leading `+` and an exponent, are accepted. The
  sixteen-byte cap counts padding, asserted with a fifteen-digit value plus one
  and plus two pad bytes.
- **`MetadataSet` has no removal API and the fixtures no longer pretend it
  does.** Four tests initially failed on `insert` refusing a duplicate tag,
  which is the lossless-record invariant working. The builders now take the
  values up front.
- **Boundary and tier.** No `wasm-bindgen`, no pixel crosses a boundary, no
  `unsafe`, no `as` cast. `grep -n ' as ' crates/ocelli-dicom/src/frame_geometry.rs`
  finds none.
- **Structure.** No new trait, generic or `Box<dyn>`. `DuplicateSources::union`
  and `MultiframeMetadata::metadata` are accessors over existing state.
  `ImagePlane::slice_normal` and `position_vector` exist so a cross-frame
  consumer does not re-derive the cross product, which would be a second copy of
  the plane's geometry.
