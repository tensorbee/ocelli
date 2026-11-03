# F-099, RTSTRUCT import to the contour representation

**Status**: approved
**Epic ref**: E15.6
**Sprint**: S35
**Estimate**: 4w

## Normative source, transcribed

DICOM PS3.3 C.8.8.6 names the contour geometric types. F-064 already
stores those types on `Contour` as patient millimetres. `present_contour`
projects them. This story does not call `present_contour` and does not
change that projection.

The dicom-expert note says RTSTRUCT contours are patient coordinates, not
voxel indices, and their z values will not exactly equal a slice position.
Match with a tolerance derived from slice spacing.

The HLD does not give the tolerance fraction.

## What the specification does not cover

The HLD does not say the fraction of slice spacing. This plan decides it,
and it is not an HLD formula.

`import_rtstruct_contour` builds one `Contour`. The segment is at least 1.
The geometric type uses the existing `kind_len` rule in `contour.rs`:
one point, at least two points for an open type, and at least three points
for a closed planar type. A second copy of that match is a defect.

Every point is finite patient millimetres. `abs(point.z - slice_z)` must
be less than or equal to half the caller slice spacing. Spacing must be
finite and greater than 0. One point outside that slab refuses the whole
contour. The stored z is the caller z. The function does not snap z to
the slice and it does not convert to canvas pixels.

Segment 0 is refused. The presenter already has no colour for it.

## Problem and reproducer

Treating an RTSTRUCT point as a canvas pixel draws the contour on the
wrong plane. Snapping z would hide a point that does not lie on the slice.

## Scope, API and compatibility

The function lives in `crates/ocelli-seg/src/contour.rs` and is public.
It returns `Result<Contour, ContourError>`. A new error variant is not
added. A bad length uses `ContourError::Length`. A z outside the slab or
a bad spacing uses `ContourError::OutOfPlane`. Segment 0 uses
`ContourError::MissingColour`. A non-finite point uses `ContourError::Frame`.

No TypeScript copy. No change to `present_contour`.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `import_rtstruct_contour` | segment, kind, patient points, slice z, spacing | half spacing, existing length rule | one `Contour`, or an error |

## State transitions and invariants

The function does not keep state. The returned points are a new vector.
The caller slice is not written.

## Ownership, commit and recovery

A refusal returns an error and no contour. The caller point slice is
unchanged.

## Numbered acceptance inventory

1. A closed planar triangle at z 10, slice z 10, spacing 2, segment 1,
   returns those three points unchanged.
2. A point at z 10.9 with spacing 2 is inside, because half spacing is 1.
   A point at z 11.1 is `OutOfPlane`.
3. Spacing 0 is `OutOfPlane`. A non-finite x is `Frame`.
4. One point with closed planar kind is `Length`. Segment 0 is
   `MissingColour`.
5. The returned z for an in-slab point is the caller z, not the slice z.

## Approach

Call the existing length helper. Then test each z. Then copy the points
into a `Contour`.

## Dependencies, write ownership and resources

Depends on F-064, which is done. This story edits `contour.rs` only to add
the import function and does not change `present_contour`. It writes the
test, the plan, and the LLD sentence.

## Verification and evidence reuse

The feature profile is the completion gate. The function does not draw, so
it does not change CPU candidate bytes.

## Boundary and tier

No pixels cross the boundary. No `wasm-bindgen`. No `unsafe`.

- Tier A (WebGPU): n/a. Drawing stays `present_contour`.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. The import does not need a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC5, hand-stated in this plan. PS3.3 C.8.8.6 names the types. The half-spacing test is the local tolerance. | `crates/ocelli-seg/tests/rtstruct_import.rs` |

## LLD impact

The shell does not export this function. One sentence in the segmentation
row of `docs/lld/README.md` names the import. If that row cannot take the
sentence, `docs/lld/public-api.md` says the shell does not export it.

## Parity surface covered

Appendix B names RTSTRUCT as an adapter. This story adds the import into
`Contour`. It does not parse a Part 10 file and it does not establish a
rendered parity view.

## Open questions

None. The HLD does not specify the spacing fraction. The half-spacing rule
above is the local decision and it is approved with this plan.
