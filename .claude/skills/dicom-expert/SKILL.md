---
name: dicom-expert
description: Deep DICOM reference for Ocelli. The information model, data elements and VRs, transfer syntaxes, the pixel module, the full LUT chain with exact formulas, image plane geometry and volume construction, multiframe and functional groups, SUV, and the interop objects. Load before implementing or reviewing anything that parses DICOM, computes a pixel value, or builds a volume.
---

# DICOM, for Ocelli

Read this before writing or reviewing any code that touches a DICOM attribute,
a pixel value or a coordinate. It is organised around the failure mode this
project actually has: **the pixel that is quietly wrong**.

Every section marked **TRAP** is a defect that produces a plausible image with
wrong values. Those are the ones that reach patients and the ones a screenshot
review cannot catch.

Normative references are to DICOM PS3.x. When this file and the standard
disagree, the standard wins and this file is a bug.

---

## 1. The information model

```
Patient
 └── Study            StudyInstanceUID    (0020,000D)
      └── Series      SeriesInstanceUID   (0020,000E)
           └── Instance  SOPInstanceUID   (0008,0018)
                         SOPClassUID      (0008,0016)
```

**A Series is not a volume.** It is an organisational unit. A single CT series
can contain multiple orientations, multiple time points, localisers mixed with
axials, and duplicate slice positions. Volume construction (§7) has to sort,
group and reject, not just concatenate.

**Frame of Reference** `FrameOfReferenceUID` (0020,0052) is what makes
coordinates comparable. Two series share a spatial frame if and only if they
share this UID. **An annotation is only transferable between series that share
it.** This is the mechanism HLD section 35 relies on for putting a slide layer
and a volume layer in one scene.

`Modality` (0008,0060) drives almost every downstream policy: default window,
whether rescale means Hounsfield units, whether SUV applies, and which
tolerance class the oracle uses.

---

## 2. Data elements, VRs and encoding

An element is `(group, element)`, a **VR**, a length, and a value.

### The VRs that matter here

| VR | Meaning | Notes for parsing |
|----|---------|-------------------|
| `US` / `SS` | 16-bit unsigned / signed | Rows, Columns, BitsAllocated |
| `UL` / `SL` | 32-bit unsigned / signed | |
| `DS` | Decimal string | **A string.** Spacing, rescale, window all arrive as text |
| `IS` | Integer string | Also text |
| `FL` / `FD` | 32/64-bit float | |
| `UI` | UID, max 64 bytes | Padded with a NUL, not a space |
| `CS` | Code string, max 16 | Photometric interpretation, VOI LUT function |
| `SQ` | Sequence | Nested datasets. May have undefined length |
| `OB` / `OW` | Other byte / other word | Pixel data, LUT data |
| `UN` | Unknown | Appears when a private or unrecognised element is read implicitly |

**TRAP: `DS` and `IS` are strings, and multi-valued ones are backslash
separated.** `PixelSpacing` is `"0.488281\0.488281"`. A parser that reads it as
a float reads the first value and silently drops the second, and a
non-square-pixel study then renders with the wrong aspect ratio. Leading and
trailing spaces are legal and must be trimmed. `"1.0e-3"` is legal `DS`.

**TRAP: an element with an odd length does not exist.** Every value is padded
to an even length, with a space for string VRs and a NUL for `UI`. Trim on
read, pad on write.

### Explicit versus implicit VR

Implicit VR Little Endian (`1.2.840.10008.1.2`) carries no VR on the wire. The
VR comes from a dictionary lookup. **A private element with no dictionary entry
has no recoverable VR**, which is why implicit-VR private data is unparseable
in general and why `UN` exists.

### Big endian

`1.2.840.10008.1.2.2`, Explicit VR Big Endian, is **retired** but real files
exist. If you support it, byte-swap the pixel data too, not only the elements.

---

## 3. Transfer syntaxes

The transfer syntax UID is in the file meta group `(0002,0010)`, which is
**always** Explicit VR Little Endian regardless of what it declares for the
dataset.

| UID | Name | Ocelli |
|-----|------|--------|
| `1.2.840.10008.1.2` | Implicit VR Little Endian | native |
| `1.2.840.10008.1.2.1` | Explicit VR Little Endian | native |
| `1.2.840.10008.1.2.1.99` | Deflated Explicit VR LE | deflate |
| `1.2.840.10008.1.2.2` | Explicit VR Big Endian (retired) | native, byte-swapped |
| `1.2.840.10008.1.2.5` | RLE Lossless | rle |
| `1.2.840.10008.1.2.4.50` | JPEG Baseline, process 1, 8-bit | jpeg |
| `1.2.840.10008.1.2.4.51` | JPEG Extended, process 2 and 4, 12-bit | jpeg |
| `1.2.840.10008.1.2.4.57` | JPEG Lossless, process 14 | jpeg |
| `1.2.840.10008.1.2.4.70` | JPEG Lossless, process 14 SV1 | jpeg |
| `1.2.840.10008.1.2.4.80` | JPEG-LS Lossless | **open gate A2** |
| `1.2.840.10008.1.2.4.81` | JPEG-LS Near-Lossless | **open gate A2** |
| `1.2.840.10008.1.2.4.90` | JPEG 2000 Lossless Only | openjp2 |
| `1.2.840.10008.1.2.4.91` | JPEG 2000 | openjp2 |
| `1.2.840.10008.1.2.4.201` | HTJ2K Lossless Only | **open gate A1** |
| `1.2.840.10008.1.2.4.202` | HTJ2K Lossless Only, RPCL | **open gate A1** |
| `1.2.840.10008.1.2.4.203` | HTJ2K | **open gate A1** |

The two open gates are HLD Appendix A, A1 and A2. A1 asks whether HTJ2K decodes
bit-exact through openjp2 under wasm32. A2 asks what the JPEG-LS answer is at
all. Both are architecture decisions, not dependency lines.

### Encapsulated pixel data

For every compressed syntax, `PixelData` (7FE0,0010) has **undefined length**
and contains items:

```
(7FE0,0010) OB, undefined length
  (FFFE,E000) Basic Offset Table   -- may be zero-length
  (FFFE,E000) fragment 1
  (FFFE,E000) fragment 2
  ...
  (FFFE,E0DD) Sequence Delimitation
```

**TRAP: one frame is not one fragment.** A frame may span several fragments,
and one fragment never spans two frames. Frame boundaries come from the Basic
Offset Table, or from the Extended Offset Table (7FE0,0001) and (7FE0,0002)
when present, which is the only reliable option above 4 GB.

**TRAP: an empty Basic Offset Table is legal.** With a zero-length BOT and
multiple frames you must either assume one fragment per frame, or parse
fragment headers to reconstruct. Guessing wrong shifts every frame after the
first, which looks like a scrolling bug rather than a parsing bug.

---

## 4. The pixel module

| Tag | Name | Notes |
|-----|------|-------|
| (0028,0010) | `Rows` | |
| (0028,0011) | `Columns` | |
| (0028,0008) | `NumberOfFrames` | `IS`. Absent means 1 |
| (0028,0002) | `SamplesPerPixel` | 1 monochrome or palette, 3 colour |
| (0028,0004) | `PhotometricInterpretation` | see below |
| (0028,0006) | `PlanarConfiguration` | 0 interleaved RGBRGB, 1 planar RRR GGG BBB |
| (0028,0100) | `BitsAllocated` | 8, 16, 32. The container |
| (0028,0101) | `BitsStored` | how many of them are real |
| (0028,0102) | `HighBit` | position of the most significant stored bit |
| (0028,0103) | `PixelRepresentation` | 0 unsigned, 1 two's-complement signed |

### Unpacking a stored value

```
value = raw >> (HighBit + 1 - BitsStored)
value = value & ((1 << BitsStored) - 1)
if PixelRepresentation == 1:
    sign-extend from bit (BitsStored - 1)
```

**TRAP: `BitsStored` is very often less than `BitsAllocated`.** 12-bit CT and
CR stored in 16-bit containers is the normal case, not the exception. Skipping
the mask leaves 4 bits of whatever the modality left there, which on some
scanners is zero (so it works in testing) and on others is not.

**TRAP: sign extension.** `PixelRepresentation == 1` with `BitsStored == 12`
means bit 11 is the sign bit, not bit 15. A naive `as i16` gives large positive
values where negative Hounsfield units belong, so air reads as bone. This is a
one-line bug that produces a completely plausible image of the wrong thing.

### Photometric interpretation

| Value | Meaning |
|-------|---------|
| `MONOCHROME1` | **Minimum value is WHITE.** Inverted |
| `MONOCHROME2` | Minimum value is black. The common case |
| `PALETTE COLOR` | Index into the palette LUTs (§5.4) |
| `RGB` | |
| `YBR_FULL` | Full-range YCbCr, no subsampling |
| `YBR_FULL_422` | Chroma subsampled 4:2:2 |
| `YBR_PARTIAL_422` | Video-range YCbCr |
| `YBR_ICT` | Irreversible colour transform, JPEG 2000 lossy |
| `YBR_RCT` | Reversible colour transform, JPEG 2000 lossless |

**TRAP: `MONOCHROME1` is not rare.** Plain radiography and some mammography
use it. Treating it as `MONOCHROME2` produces a photographic negative, which
is at least visible, but the same bug applied *after* a VOI inversion cancels
out and produces a correct-looking image with wrong intermediate values, which
the oracle catches and a human does not.

**TRAP: the codec may already have converted the colour space.** A JPEG
decoder usually outputs RGB even though the DICOM header still says
`YBR_FULL_422`. Converting again darkens and shifts hue. Decide once, at the
codec boundary, who owns the colour transform, and record it on the frame.

**TRAP: `PlanarConfiguration` is only meaningful for uncompressed data.** For
encapsulated syntaxes the codec defines the layout. A header saying 1 for a
JPEG frame is to be ignored, not honoured.

---

## 5. The LUT chain

**This is the highest-risk arithmetic in the project.** PS3.3 C.11. The stages
apply strictly in order, and implementing them once in `ocelli-pixel` and
letting the shader read the parameters is HLD section 18's explicit
instruction. Do not let a second copy of this logic appear anywhere.

```
Stored  --[1. Modality LUT]-->  Modality  --[2. VOI LUT]-->  Display
        --[3. Presentation LUT]-->  Display  --[4. Palette / ICC]-->  RGB
```

### 5.1 Modality LUT, Stored to Modality

```
modality_value = stored * RescaleSlope + RescaleIntercept
```

`RescaleSlope` (0028,1053), `RescaleIntercept` (0028,1052), both `DS`. For CT
the result is Hounsfield units. `RescaleType` (0028,1054) names the unit.

**A `ModalityLUTSequence` (0028,3000), when present, takes precedence over
slope and intercept.** Not "in addition to". Applying both double-transforms.

**PET is a separate path.** Do not fold SUV into this stage. See §9.

### 5.2 VOI LUT, Modality to Display

`WindowCenter` (0028,1050), `WindowWidth` (0028,1051), both `DS` and both
**potentially multi-valued** with matching indices. `VOILUTFunction` (0028,1056)
is `LINEAR`, `LINEAR_EXACT` or `SIGMOID`, defaulting to `LINEAR`.

A `VOILUTSequence` (0028,3010), when present, takes precedence over the window
values.

**The three functions. Copy these exactly.**

```
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
//   x <= c' - w'/2      -> ymin
//   x >  c' + w'/2      -> ymax
//   else  y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin

// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
//   x <= c - w/2        -> ymin
//   x >  c + w/2        -> ymax
//   else  y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin

// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
//   y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

**TRAP: the difference between `LINEAR` and `LINEAR_EXACT` is a half and a
one, and it is the most commonly mis-ported detail in DICOM viewers.**

The HLD's worked fixture, soft-tissue CT, centre 40, width 400, output 0 to 255:

| Input (HU) | LINEAR | LINEAR_EXACT | Why this row |
|------------|--------|--------------|--------------|
| -160 | 0.000 | 1.594 | LINEAR's boundary is `c' - w'/2 = -160` exactly, and the comparison is `<=`, so it clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer sees by eye |
| 240 | 255.000 | 255.000 | LINEAR's upper bound is `c' + w'/2 = 239`, so 240 clamps |
| -60 | 63.910 | 63.750 | Mid-lower quarter, catches sign and slope errors |

**These four rows must be in the test suite before the shader is written.** At
the window centre the two functions differ by 0.32 of 255, which is invisible
in a screenshot and immediate in a pixel diff. That is the entire argument for
building the oracle first.

**TRAP: the comparison operators are asymmetric.** Lower bound is `<=`, upper
bound is `>`. Writing both as `<` and `>` moves one boundary pixel value, which
no test that samples the middle of the range will ever find.

**TRAP: `w >= 1` for LINEAR, `w > 0` for LINEAR_EXACT.** A width of 1 makes
`w' = 0` and divides by zero. A width of 0 arrives from real scanners.

### 5.3 Presentation LUT

`PresentationLUTShape` (2050,0020) is `IDENTITY` or `INVERSE`. A presentation
state may override it. `MONOCHROME1` is conceptually an inversion here, and
deciding whether you apply it at this stage or at stage 1 is a decision to make
once and write down, because applying it in both places cancels.

### 5.4 Palette colour and ICC

Palette descriptors (0028,1101) red, (0028,1102) green, (0028,1103) blue, and
data (0028,1201) / (0028,1202) / (0028,1203).

**LUT Descriptor is three values: number of entries, first stored value
mapped, bits per entry.**

**TRAP: a descriptor's first value of 0 means 65536 entries, not zero.**

**TRAP: the second value is the first *input* value mapped.** Inputs below it
clamp to the first entry, above the range clamp to the last. Ignoring the
offset shifts the whole colour map.

**TRAP: `bits per entry` of 16 with 8-bit-looking data.** Some files declare 16
bits and store values in the high byte. Read the descriptor, do not infer.

ICC (0028,2000) matters for whole-slide imaging, where colour management is a
correctness requirement rather than a nicety. Scanner RGB to CIEXYZ to sRGB via
the embedded profile, as a real pipeline stage after the LUT chain.

---

## 6. Image plane geometry

| Tag | Name | Meaning |
|-----|------|---------|
| (0020,0032) | `ImagePositionPatient` | LPS mm of the **centre of the first voxel** |
| (0020,0037) | `ImageOrientationPatient` | 6 values, row and column direction cosines |
| (0028,0030) | `PixelSpacing` | **`[between rows, between columns]`** |
| (0018,0050) | `SliceThickness` | |
| (0018,0088) | `SpacingBetweenSlices` | |
| (0018,1164) | `ImagerPixelSpacing` | detector spacing, projection imaging |

The patient coordinate system is **LPS**: +x to the patient's Left, +y
Posterior, +z Superior. Not RAS. Anything imported from a neuroimaging tool is
probably RAS and needs flipping on x and y.

### Voxel to patient, PS3.3 C.7.6.2.1.1

With `X = IOP[0..3]` the row direction and `Y = IOP[3..6]` the column
direction, `i` the **column** index and `j` the **row** index:

```
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

**TRAP: `PixelSpacing` is `[row spacing, column spacing]`, and row spacing is
the vertical one.** So `PixelSpacing[0]` multiplies the *column direction
cosine* and `PixelSpacing[1]` multiplies the *row direction cosine*. Getting
this backwards is invisible on the overwhelmingly common square-pixel study and
wrong on every non-square one, which is most ultrasound and much mammography.

**TRAP: IPP is the centre of the first voxel, not its corner.** A half-voxel
offset propagates into every measurement and every fused overlay, and it is
exactly the size that looks like acceptable registration error.

The slice normal is `N = X × Y`. It should already be a unit vector because the
cosines are normalised, but real files carry values that are not quite
normalised, so normalise and do not assume.

---

## 7. Building a volume from a series

This is where most viewers are quietly wrong.

1. **Group by `FrameOfReferenceUID` and `ImageOrientationPatient`.** A series
   containing a localiser has two orientations and is not one volume.
2. **Sort by IPP projected onto the normal**, `dot(IPP, N)`, ascending. **Never
   sort by `InstanceNumber` or `SliceLocation`.**
3. **Derive slice spacing from consecutive projected positions**, not from a
   tag.
4. **Check the spacing is consistent.** Compare every gap against the median.
5. **Reject or merge duplicate positions.**

**TRAP: `SpacingBetweenSlices` and `SliceThickness` are not slice spacing.**
Thickness is the reconstructed slab thickness and may overlap or gap.
`SpacingBetweenSlices` is frequently absent, and when present is frequently
wrong. The projected IPP difference is the ground truth. Using the tag on an
overlapping-reconstruction CT compresses or stretches the volume along z, which
makes every sagittal and coronal reformat wrong by a constant factor while the
axial view looks perfect.

**TRAP: non-uniform spacing is real** in dose-modulated and multi-slab
acquisitions. A volume model that assumes uniform spacing must detect and
refuse, not silently average. Rendering a variable-spacing series as uniform
distorts geometry in a way no measurement tool will flag.

**TRAP: gantry tilt.** `GantryDetectorTilt` (0018,1120) non-zero means the
volume is a **sheared** parallelepiped, not a box. Detect it from the geometry
rather than the tag, by checking whether the inter-slice vector is parallel to
the slice normal. If it is not, the volume is sheared and a naive
axis-aligned 3D texture upload skews the reformats.

---

## 8. Multiframe and enhanced SOP classes

Enhanced CT, MR, PET, XA and the whole-slide class put every frame in one
instance and carry geometry per frame.

| Tag | Name |
|-----|------|
| (5200,9229) | `SharedFunctionalGroupsSequence` |
| (5200,9230) | `PerFrameFunctionalGroupsSequence` |

**The lookup order is per-frame first, then shared.** An attribute present in
both is a malformed file, and per-frame wins.

Useful groups inside them: `PlanePositionSequence` (IPP per frame),
`PlaneOrientationSequence` (IOP per frame), `PixelMeasuresSequence`
(PixelSpacing, SliceThickness), `FrameVOILUTSequence` (window per frame),
`PixelValueTransformationSequence` (rescale per frame),
`FrameContentSequence` (stack and temporal position), `RealWorldValueMappingSequence`.

**TRAP: rescale and window can differ per frame.** A legacy-shaped reader that
reads `RescaleSlope` once from the top-level dataset gets nothing (it is not
there) or gets a stale value, and applies one frame's transform to all of them.

**TRAP: `DimensionIndexSequence` defines the frame ordering**, and frames are
not required to be stored in spatial order.

---

## 9. PET and SUV

Standardised uptake value is **not** part of the LUT chain. It is a separate
path and needs the radiopharmaceutical sequence.

```
activity = stored * RescaleSlope + RescaleIntercept          // Bq/ml
SUVbw    = activity * PatientWeight_kg * 1000 / decayed_dose_Bq
```

with

```
decayed_dose = RadionuclideTotalDose
             * 2 ^ ( -elapsed_seconds / RadionuclideHalfLife )
```

Inputs: `RadiopharmaceuticalInformationSequence` (0054,0016),
`RadionuclideTotalDose` (0018,1074), `RadionuclideHalfLife` (0018,1075),
`RadiopharmaceuticalStartDateTime` (0018,1078) or `RadiopharmaceuticalStartTime`
(0018,1072), `PatientWeight` (0010,1030), `Units` (0054,1001),
`DecayCorrection` (0054,1102), `SeriesTime` (0008,0031).

**TRAP: the elapsed time is from injection to *series* time, not acquisition
time**, for the common `START` decay correction. Different vendors disagree
here and it is a known source of cross-vendor SUV divergence.

**TRAP: `Units` must be `BQML`.** If it says `CNTS` or `GML`, the formula does
not apply and computing it anyway yields a number that looks like an SUV.

**TRAP: SUV is a clinical number a radiologist may act on.** If any input is
missing, report unavailable. Never substitute a default weight or a default
half-life.

---

## 10. Interop objects

| Object | What it is | Ocelli |
|--------|-----------|--------|
| **SR, TID 1500** | Measurement report as a content-item tree | HLD D13, the annotation type **IS** an SR content tree |
| **SEG** | Segmentation as a binary or fractional labelmap instance | read and write |
| **RTSTRUCT** | Contours as point lists per structure | import to contour representation |
| **GSPS** | Greyscale softcopy presentation state | read **and write** |
| **Parametric map** | Derived per-voxel real-world values | |

**HLD decision D13: the in-memory annotation type is a DICOM SR content tree.**
Not an internal model with SR adapters. The drawing layer renders from it, it
does not own it. Concepts are coded with SNOMED CT and UCUM, which makes every
measurement training-data-grade by construction.

**TRAP: SEG frames are not in series order.** A SEG instance carries its own
`PerFrameFunctionalGroupsSequence` with `SegmentIdentificationSequence`, and a
frame maps to a source instance via `DerivationImageSequence`. Assuming frame
*n* of the SEG matches slice *n* of the series misaligns the overlay, usually
by a small amount that looks like a segmentation quality problem.

**TRAP: RTSTRUCT contours are in patient coordinates, not voxel indices**, and
their z values will not exactly equal any slice position. Match with a
tolerance derived from slice spacing.

---

## 11. DICOMweb

| Service | Purpose |
|---------|---------|
| **QIDO-RS** | Query. `GET /studies?PatientID=...` |
| **WADO-RS** | Retrieve. `GET /studies/{s}/series/{s}/instances/{i}` |
| **STOW-RS** | Store. `POST /studies` |
| **WADO-URI** | Legacy single-object retrieve |

Frame-level retrieval is `.../instances/{i}/frames/{f}`, and it is **mandatory
at gigapixel scale**. Pulling whole instances for a whole-slide image is not a
performance problem, it is an impossibility.

Metadata comes back as `application/dicom+json`, where every element is keyed
by its eight-hex-digit tag with a `vr` and a `Value` array.

**TRAP: DICOM JSON values are always arrays**, even single-valued ones.
`PersonName` is an object with `Alphabetic`, and binary values arrive as
`InlineBinary` base64 or as a `BulkDataURI` to fetch separately.

---

## 12. The review checklist

When reviewing anything in this area, check these specifically. Each has
produced a plausible image with wrong values:

- [ ] `BitsStored` masking applied, and sign extension from `BitsStored - 1`
- [ ] `PixelRepresentation` honoured
- [ ] `MONOCHROME1` inversion applied exactly once
- [ ] `PixelSpacing[0]` multiplies the **column** direction cosine
- [ ] Slice spacing derived from projected IPP, not from a tag
- [ ] Non-uniform spacing detected and refused rather than averaged
- [ ] VOI boundary comparisons are `<=` low and `>` high
- [ ] `LINEAR` uses `c - 0.5` and `w - 1`, `LINEAR_EXACT` does not
- [ ] `ModalityLUTSequence` and `VOILUTSequence` take precedence, not addition
- [ ] Palette descriptor's first-value-mapped offset applied, 0 means 65536
- [ ] Per-frame functional groups consulted before shared
- [ ] Colour transform applied once, at a recorded boundary
- [ ] Multi-valued `DS` parsed as multi-valued
- [ ] SUV inputs all present, `Units` is `BQML`, or reported unavailable
- [ ] A hand-computed fixture exists citing the PS3.3 section, and it goes red
      when one constant in the implementation is mutated

---

## 13. Where to look in the standard

| Part | Contains |
|------|----------|
| **PS3.3** | Information object definitions. The pixel, image plane, VOI and LUT modules. **C.11 is the LUT chain.** C.7.6.2 is image plane geometry |
| **PS3.4** | Service classes. Storage, query/retrieve SOP classes |
| **PS3.5** | Data structures and encoding. VRs, transfer syntaxes, encapsulation |
| **PS3.6** | The data dictionary. Every tag, name and VR |
| **PS3.10** | The file format, preamble and file meta information |
| **PS3.14** | Greyscale standard display function. Reachable only from the desktop target |
| **PS3.15** | Security and de-identification profiles |
| **PS3.16** | Content mapping resource. Template definitions including TID 1500 |
| **PS3.18** | DICOMweb |

### Where to actually look it up

| Resource | Use it for |
|----------|------------|
| [DICOM Standard Browser](https://dicom.innolitics.com/ciods) | **The fastest way to answer "which module is this attribute in, and is it required?"** Browse by CIOD, module or attribute. Every entry links back to the PS3.3 section, gives the tag, keyword, VR, VM and the Type (1, 1C, 2, 2C, 3), and shows the module's place in the object |
| [DICOM Standard, current edition](https://www.dicomstandard.org/current) | The normative text. What you cite |
| [DICOM Browser, tag search](https://dicom.innolitics.com/ciods/ct-image) | A worked example: the CT Image CIOD, showing which of the pixel and plane attributes are Type 1 and which are optional |

**The Type column is the one people skip and should not.** Type 1 is required
and may not be empty. Type 2 is required and **may** be empty, which is why
`PatientName` can legitimately be a zero-length value and a parser that treats
absent and empty the same is wrong. Type 1C and 2C are conditional, and the
condition is the part that matters. Type 3 is optional, so a reader that
depends on it needs a defined fallback.

**Cite the standard section, not this file and not the browser, in a fixture
comment.** This document is a convenience and can be wrong. The browser is a
convenience and can lag an edition. PS3.3 C.11.2.1.2 cannot.
