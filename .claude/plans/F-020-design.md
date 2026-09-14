# F-020, Per-frame functional groups, gantry tilt, spacing calibration

**Status**: approved
**Epic ref**: E3.5
**Sprint**: S09
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/13-core-types.md`, section 16, the typed coordinate spaces

The marker spaces and the point type, quoted from the tracked file:

```rust
pub enum Canvas {}
pub enum World {}
pub enum Index {}

pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
pub struct Transform<A, B> { m: glam::DMat4, _a: PhantomData<A>, _b: PhantomData<B> }
```

Section 16's own argument, quoted:

> cornerstone3D represents canvas points, world points and voxel indices all as
> `number[]`, and mixing them is a silent, common and expensive bug.

That is the whole reason this story returns typed geometry rather than
`[f64; 3]`.

### `docs/hld/16-volume-representation.md`, section 19

```rust
pub struct Volume {
    pub dims: [u32; 3],
    pub spacing: [f64; 3], // mm
    pub origin: Pt<World>, // IPP of the first slice
    pub direction: glam::DMat3, // derived from ImageOrientationPatient
    ...
}
```

This is **E8's** structure, not this story's. It is transcribed because it is
the only place the HLD says what derived geometry is eventually consumed as,
and it fixes two things F-020 must produce compatibly: spacing in millimetres
as `f64`, and a direction matrix *derived from* Image Orientation Patient
rather than stored raw.

### What the HLD says about gantry tilt and spacing calibration

**Nothing.** `grep -rn -i 'gantry\|tilt\|ImagerPixelSpacing\|PixelMeasures\|
SpacingBetweenSlices\|calibrat' docs/hld/*.md` returns three hits, all in
sections 10 and 26, and all about DICOM Part 14 display calibration, which is a
different subject entirely and is explicitly out of browser reach. **There is no
normative HLD text for the arithmetic in this story.** Per `/design` rule one,
that is recorded here rather than filled with a plausible design presented as
specified. The normative source for the arithmetic is therefore DICOM PS3.3
directly, transcribed below.

### DICOM PS3.3 C.7.6.2.1.1, image position and image orientation

The voxel-to-patient equation, in the standard's own matrix form:

```text
| Px |   | Xx*di  Yx*dj  0  Sx |   | i |
| Py | = | Xy*di  Yy*dj  0  Sy | * | j |
| Pz |   | Xz*di  Yz*dj  0  Sz |   | 0 |
| 1  |   |  0      0     0   1 |   | 1 |
```

where `X` is Image Orientation Patient's first three values (the row
direction), `Y` is its second three (the column direction), `S` is Image
Position Patient, `i` is the **column** index, `j` is the **row** index,
`di` is Pixel Spacing's **column** spacing and `dj` is its **row** spacing.

Pixel Spacing (0028,0030) is `[between rows, between columns]`. So
`PixelSpacing[0]` scales the column direction cosine `Y` and `PixelSpacing[1]`
scales the row direction cosine `X`. This is already implemented and documented
in `crates/ocelli-pixel/src/image_plane.rs::ImagePlane::index_to_world` and
`docs/lld/pixel-pipeline.md`, and F-020 does not re-derive it.

Image Position Patient is the centre of the first transmitted voxel, not its
corner.

### DICOM PS3.3 C.7.6.16.2.2, Pixel Measures Functional Group Macro

Sequence (0028,9110), one item. Its attributes:

| Tag | Name | Type |
|-----|------|------|
| (0028,0030) | Pixel Spacing | 1C |
| (0018,0050) | Slice Thickness | 1C |
| (0018,0088) | Spacing Between Slices | 3 |

### DICOM PS3.3 C.7.6.16.2.3, Plane Position (Patient) Functional Group Macro

Sequence (0020,9113), one item, containing Image Position Patient (0020,0032),
Type 1.

### DICOM PS3.3 C.7.6.16.2.4, Plane Orientation (Patient) Functional Group Macro

Sequence (0020,9116), one item, containing Image Orientation Patient
(0020,0037), Type 1.

### DICOM PS3.3 C.7.6.16.1.1, the functional-group lookup rule

An attribute carried in a functional group macro is looked up in the
Per-frame Functional Groups Sequence item for that frame first, and in the
Shared Functional Groups Sequence item second. An attribute present in both is
a malformed instance. This is already implemented by
`MultiframeMetadata::resolve` in `crates/ocelli-dicom/src/multiframe.rs`, which
returns the winning `FunctionalGroupSource` and retains lower-precedence
duplicates in `DuplicateSources`.

### DICOM PS3.3 C.8.7.3.1.1 and C.7.6.1.1.5, gantry tilt

Gantry/Detector Tilt (0018,1120) is a `DS` in the CT Acquisition Details module,
"Nominal angle of tilt in degrees of the scanning gantry". It is **nominal** and
Type 3.

The definitive statement of what tilt means geometrically is not the tag, it is
C.7.6.2.1.1: the inter-frame vector is `IPP(n+1) - IPP(n)`, and the slice normal
is `N = X cross Y`. A stack is **axis-aligned** exactly when the inter-frame
vector is parallel to `N`. When it is not, the stack is a sheared
parallelepiped. The tag is nominal evidence and the geometry is the measurement.

### DICOM PS3.3 C.7.6.1.1.5, Pixel Spacing Calibration

| Tag | Name | Notes |
|-----|------|-------|
| (0018,1164) | Imager Pixel Spacing | Detector spacing at the detector plane, projection imaging |
| (0028,0030) | Pixel Spacing | The spacing that applies to the stored image |
| (0028,0A02) | Pixel Spacing Calibration Type | `GEOMETRY` or `FIDUCIAL` |
| (0028,0A04) | Pixel Spacing Calibration Description | `LO` |

PS3.3 C.7.6.1.1.5 states that when Pixel Spacing and Imager Pixel Spacing are
both present and differ, Pixel Spacing has been **calibrated** to some plane
other than the detector plane, and Pixel Spacing Calibration Type describes how.
Pixel Spacing is the one that applies to measurements in the image.

## What the specification does not cover

Everything in this section is a decision this plan makes, because the HLD makes
none of them.

1. **Which crate owns derived per-frame geometry.** `ocelli-pixel` owns
   `ImagePlane` and the index-to-world transform. `ocelli-dicom` owns the
   lossless metadata projection. The derivation needs both. This plan puts it in
   **`ocelli-dicom`**, in a new `frame_geometry` module, depending on
   `ocelli-pixel` for the validated plane types. Rationale: the inputs are DICOM
   attributes and functional-group resolution, which is `ocelli-dicom`'s
   subject, and the output is `ocelli-pixel`'s already-validated types, so no
   arithmetic is duplicated. `ocelli-pixel` gains no DICOM-tag knowledge, which
   is the boundary `docs/lld/pixel-pipeline.md` already states.
2. **Decimal String parsing.** PS3.5 6.2 defines `DS` as a string with an
   optional sign, digits, an optional fraction and an optional exponent, at most
   16 bytes, padded with a space, multi-valued with backslash separators.
   Nothing in the repository parses one yet. This plan adds one parser, in
   `ocelli-dicom`, used by every `DS` attribute here.
3. **Whether the tilt answer is a boolean or an angle.** This plan produces a
   measured `StackShear` verdict from geometry, and retains the nominal
   (0018,1120) value separately as evidence, without ever letting the tag decide.
4. **What "cannot be derived" means.** This plan refuses rather than defaulting.
   The full refusal table is in Approach.
5. **Whether `ImagerPixelSpacing` may ever be used as image spacing.** This plan
   says no. It is retained as evidence and never silently substituted.
6. **Slice spacing.** This story derives per-frame geometry for ONE instance.
   Cross-instance slice spacing over a series is E8's volume builder and is out
   of scope. Within one multiframe instance, consecutive-frame spacing IS
   derivable and IS in scope, because both frames' Image Position Patient values
   come from the same instance.

## Approach

One new module, `crates/ocelli-dicom/src/frame_geometry.rs`, and one new
`ocelli-dicom` dependency on `ocelli-pixel`.

### The types

```rust
/// Derived, validated geometry for one zero-based frame.
pub struct FrameGeometry {
    plane: ImagePlane,                 // ocelli-pixel, already validated
    spacing_evidence: SpacingEvidence,
    sources: FrameGeometrySources,
}

/// Which functional-group source answered each attribute.
pub struct FrameGeometrySources {
    position: FunctionalGroupSource,
    orientation: FunctionalGroupSource,
    pixel_spacing: FunctionalGroupSource,
    duplicates: DuplicateSources,      // union over the three
}

/// Calibrated against uncalibrated spacing, both retained.
pub struct SpacingEvidence {
    image_mm: PixelSpacing,                     // (0028,0030), the one that applies
    imager_mm: Option<PixelSpacing>,            // (0018,1164), evidence only
    calibration_type: Option<CalibrationType>,  // (0028,0A02)
    relationship: SpacingRelationship,
}

pub enum SpacingRelationship {
    /// Only Pixel Spacing is present.
    ImageOnly,
    /// Both present and equal within the declared tolerance.
    ImagerAgrees,
    /// Both present and different. Pixel Spacing is calibrated.
    Calibrated,
    /// Only Imager Pixel Spacing is present. Refused, see the table below.
    ImagerOnly,
}
```

`ImagePlane`, `ImagePositionPatient`, `ImageOrientationPatient`, `PixelSpacing`
and `ImageDimensions` are **reused from `ocelli-pixel` unchanged**. F-020 adds
no second validation of a direction cosine and no second index-to-world
transform. `FrameGeometry::index_to_world` forwards to
`ImagePlane::index_to_world`.

### Resolution, per frame

For each of the three attributes, call the existing
`MultiframeMetadata::resolve` with the group tag, the attribute tag and an
explicit `TopLevelFallback`:

| Attribute | Group tag | `TopLevelFallback` | Why |
|-----------|-----------|--------------------|-----|
| Image Position Patient (0020,0032) | Plane Position (0020,9113) | `Allowed` | A legacy single-frame CT carries it at the top level and has no functional groups at all |
| Image Orientation Patient (0020,0037) | Plane Orientation (0020,9116) | `Allowed` | Same |
| Pixel Spacing (0028,0030) | Pixel Measures (0028,9110) | `Allowed` | Same |
| Slice Thickness (0018,0050) | Pixel Measures (0028,9110) | `Allowed` | Retained as evidence, never used as spacing |
| Spacing Between Slices (0018,0088) | Pixel Measures (0028,9110) | `Allowed` | Retained as evidence, never used as spacing |

Imager Pixel Spacing (0018,1164), Pixel Spacing Calibration Type (0028,0A02) and
Gantry/Detector Tilt (0018,1120) are **top-level only**. They are not members of
any functional group macro, so they are read from the `MetadataSet` directly and
never resolved through a group.

`DuplicateSources` from each resolution is unioned into
`FrameGeometrySources::duplicates`, so a caller can see that a per-frame value
won over a shared one that also existed. C.7.6.16.1.1 makes that malformed, and
this story **retains it as observable evidence rather than refusing**, because
F-019 already chose retention over refusal for the same condition and reversing
that here would make one instance parse under one module and not the other.

### Stack shear, measured

For an instance with two or more frames whose geometry all derives:

```text
N          = normalize(X cross Y)                 // from frame 0's orientation
step(n)    = IPP(n+1) - IPP(n)
parallel   = |normalize(step(n)) dot N|
```

`parallel == 1` exactly means axis-aligned. The verdict is

```rust
pub enum StackShear {
    /// Every inter-frame step is parallel to the slice normal.
    AxisAligned { step_mm: f64 },
    /// At least one step is not. The volume is a sheared parallelepiped.
    Sheared { max_offaxis_mm: f64 },
    /// Fewer than two frames, so there is no inter-frame vector.
    NotApplicable,
}
```

with a **declared** tolerance, not a tuned one: the off-axis component of each
step must exceed `1e-6 mm` before `Sheared` is reported. That figure is the
`2e-6` dimensionless direction-cosine tolerance already declared in
`crates/ocelli-pixel/src/image_plane.rs` carried into millimetres against a step
whose magnitude is at least the sub-micrometre scale no scanner emits. It is
chosen once, here, and any change is a design-plan decision per
`.claude/WORKFLOW.md`'s tolerance policy.

Gantry/Detector Tilt (0018,1120), when present, is parsed and retained as
`nominal_tilt_degrees: Option<f64>` and is **never** used to compute anything.
A fixture asserts that an instance whose tag says 0 and whose geometry is
sheared reports `Sheared`, and that an instance whose tag says 30 and whose
geometry is axis-aligned reports `AxisAligned`. Both directions matter, because
the tag is Type 3 and nominal.

`step_mm` is the signed projection `step dot N` for the axis-aligned case, so a
descending stack is negative rather than silently absolute. Within one instance
the steps must also be **consistent**: if the projected steps differ from their
median by more than `1e-6 mm`, the result is `Sheared` is not the right answer,
so a third variant is not added. Instead `AxisAligned` carries only the case
where every projected step is equal within that tolerance, and a non-uniform
axis-aligned stack reports `FrameGeometryError::NonUniformFrameSpacing`, which
is a refusal. That is the `dicom-expert` rule "non-uniform spacing is real, and
a volume model that assumes uniform spacing must detect and refuse".

### Spacing precedence

```text
image_mm   := Pixel Spacing (0028,0030), resolved through Pixel Measures
imager_mm  := Imager Pixel Spacing (0018,1164), top level only
```

| Present | Result |
|---------|--------|
| `image_mm` only | `ImageOnly`, spacing is `image_mm` |
| both, equal within `1e-9 mm` | `ImagerAgrees`, spacing is `image_mm` |
| both, different | `Calibrated`, spacing is `image_mm`, `imager_mm` retained |
| `imager_mm` only | **refused**, `FrameGeometryError::UncalibratedSpacingOnly` |
| neither | **refused**, `FrameGeometryError::MissingPixelSpacing` |

The fourth row is the load-bearing one and it is the reason this table exists.
Substituting detector spacing for image spacing on a projection radiograph
produces a geometry that renders correctly and measures wrong by the
source-to-image magnification factor, which is typically 10 to 20 per cent. That
is precisely the quietly-wrong number this repository exists to catch, so the
answer is a refusal and the caller decides.

The `1e-9 mm` equality tolerance is a `DS` round-trip tolerance, not a physical
one: both values arrive as decimal text and agreeing exactly as text is the
common case, so this only absorbs a differing number of trailing zeros.

### The refusal table

Every route that cannot produce a derived value refuses. `FrameGeometryError`
variants, all carrying tags, counts and sources only, never attribute values,
which is the constraint `MultiframeError` already holds:

| Variant | Condition |
|---------|-----------|
| `MissingImagePosition` | (0020,0032) resolves to nothing |
| `MissingImageOrientation` | (0020,0037) resolves to nothing |
| `MissingPixelSpacing` | (0028,0030) resolves to nothing and (0018,1164) also absent |
| `UncalibratedSpacingOnly` | (0028,0030) absent, (0018,1164) present |
| `InvalidDecimalString { tag }` | A `DS` value is not PS3.5 6.2 conforming |
| `DecimalStringMultiplicity { tag, expected, actual }` | Wrong value count |
| `InvalidVr { tag, found }` | The attribute's VR is not the one PS3.6 assigns |
| `Plane(PixelError)` | `ocelli-pixel` refused the assembled plane |
| `NonUniformFrameSpacing` | Projected inter-frame steps are not equal within tolerance |
| `Multiframe(MultiframeError)` | Resolution itself failed |

No variant has a default and no variant has a fallback.

### Decimal String parsing

One function, `parse_decimal_string(text: &str) -> Result<f64, ...>`, in the
same module, applied to every `DS` here. It trims leading and trailing spaces
per PS3.5 6.2, refuses an empty result, refuses a value longer than 16 bytes
including retained padding (the same shape `parse_frame_count` already uses for
`IS`'s 12-byte limit), and parses with Rust's `f64::from_str`, refusing a
non-finite result. Multi-valued attributes are read from
`MetadataValue::Text(Vec<String>)`, which F-017 already splits on the backslash,
so this parser never sees a separator.

**`f64::from_str` accepts three spellings `DS` does not**: `inf`, `nan` and a
leading `+`/`-` on those. The non-finite check refuses the first two. A leading
`+` on a number IS legal `DS`, and `f64::from_str` accepts it, so no extra rule
is needed. `f64::from_str` also accepts `1_0`, which `DS` does not, so the
parser rejects any byte outside `[0-9+-.eE]` before calling it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. This story touches no pixel data at all
- Render-loop allocation: none. Derivation happens at ingest, in a worker,
  before anything reaches a device. `FrameGeometry` is `Copy`-sized and borrows
  nothing from the metadata set after construction
- unsafe: none
- Tier A (WebGPU): n/a. Derived geometry is CPU metadata work and the resolved
  tier does not select a derivation
- Tier B (WebGL2): n/a, same reason
- Tier C (CPU): n/a, same reason. **This is not an omission.** Deriving a
  coordinate from a tag happens once, in a worker, on every tier, and a
  tier-gated geometry path would be exactly the second copy HLD section 18's
  rule forbids for the LUT chain, with the added property that it would only run
  on hardware nobody develops on

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | Index-to-world for a **non-square** spacing frame, hand-computed from PS3.3 C.7.6.2.1.1, proving `PixelSpacing[0]` scales the column direction cosine | `crates/ocelli-dicom/tests/frame_geometry.rs` |
| `fixture` | Per-frame position wins over shared position, with the shared value retained in `DuplicateSources`, citing PS3.3 C.7.6.16.1.1 | same |
| `fixture` | A sheared three-frame stack whose (0018,1120) says 0 reports `Sheared`, and an axis-aligned stack whose (0018,1120) says 30 reports `AxisAligned`, citing PS3.3 C.7.6.2.1.1 | same |
| `fixture` | `Calibrated` spacing: (0028,0030) 0.139 against (0018,1164) 0.175, image spacing wins, imager spacing retained, citing PS3.3 C.7.6.1.1.5 | same |
| `fixture` | Imager-only spacing is refused, hand-checked against C.7.6.1.1.5 | same |
| `fixture` | Non-uniform projected frame spacing is refused rather than averaged | same |
| `unit` | `parse_decimal_string` against the PS3.5 6.2 grammar: padding, `+`, exponent, the 16-byte limit, `inf`, `nan`, `1_0` | `crates/ocelli-dicom/src/frame_geometry.rs` |
| `unit` | Every refusal variant fires on its stated condition and carries no attribute value | same |
| `property` | `index_to_world` composed with its inverse returns the original index within `1e-9` for a randomised orthonormal orientation and non-square spacing | `crates/ocelli-dicom/tests/frame_geometry.rs` |
| `conformance` | `corpus/data/synthetic/ct_multiframe_perframe.dcm` derives geometry for every declared frame, against values `scripts/corpus_synth.py` wrote | `crates/ocelli-dicom/tests/corpus.rs` |

**Mutation check, HLD 27.3.** Each of the four arithmetic fixtures is observed
red with one constant mutated, and the mutation is named in the implementation
note: swap the two spacing indices, negate the normal, change the shear
tolerance exponent, and prefer `imager_mm` over `image_mm`. A `unit` row is not
sufficient for any of them and none is claimed as such.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` in this repository has no `Covered by`
column and no row keyed on E3.5, which every prior plan in `.claude/plans/` has
also recorded. The `/design` step 6 instruction describes a column the tracked
appendix does not carry.

## Deviations

None expected. The HLD specifies nothing this plan contradicts, because it
specifies nothing about this subject at all, which is recorded in **What the
specification does not cover** rather than raised as a `D-NN` row. A deviation
records a departure from written text and there is no written text here.

One thing to watch during implementation: `ocelli-dicom` already left the
`no_std` set under **D-18**, so adding `ocelli-pixel` (which is `no_std`) as a
dependency is a one-way widening and needs no new row.

## LLD impact

- `docs/lld/dicom-ingest.md`, a new section on derived frame geometry, and the
  sentence at line 171 that currently defers to F-020 is updated to point at it
- `docs/lld/pixel-pipeline.md`, one sentence recording that `ImagePlane` now has
  a second consumer and that the index-to-world transform is still implemented
  exactly once

## Open questions

None. Three were raised in the draft and all three were resolved against
existing evidence in the repository rather than referred to the operator. They
are recorded below rather than deleted, because a question that was answered and
a question nobody asked look identical later.

## Decisions taken in this plan, and what decided them

**1. `FrameGeometry` lives in `ocelli-dicom`, not `ocelli-geom`.**
`crates/ocelli-geom/` is a bare scaffold with only `src/lib.rs`, and
`docs/sprints/allocation.json` puts its subject at E11, hit-testing and
measurement mathematics. Putting DICOM-tag-derived geometry there would give
that crate a `ocelli-dicom` dependency it does not otherwise need, and would put
functional-group resolution two crates away from the resolver F-019 wrote. The
crate name invites the other answer and the dependency direction settles it.

**2. A per-frame and shared duplicate is retained observably, not refused.**
PS3.3 C.7.6.16.1.1 makes the duplicate malformed. F-019 already chose retention,
in `DuplicateSources`, and that choice is merged and documented in
`docs/lld/dicom-ingest.md`. Refusing here would mean one instance parses under
`multiframe` and fails under `frame_geometry`, which is worse than either rule
applied consistently. **If the project later prefers refusal, it is one change
in both modules and it is F-019's rule that moves**, not this one.

**3. Cross-instance slice spacing is out of scope.** Within one multiframe
instance, consecutive-frame spacing is derived and is in scope, because both
frames' Image Position Patient values come from the same instance. Sorting and
spacing a series of separate instances is E8's volume builder. Both the story
title, "per-frame functional groups", and `CURRENT_SPRINT.md`'s "per-frame plane
position" support that boundary.
