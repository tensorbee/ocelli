---
name: dicom-tooling
description: Python and command-line DICOM tooling for Ocelli's corpus and fixture work. pydicom and DCMTK for inspecting cases, computing expected values by hand, building synthetic fixtures, and adding corpus rows. Load when working on the corpus, the oracle, or any hand-computed fixture. For the standard itself, load dicom-expert.
---

# DICOM tooling, for corpus and fixture work

**No DICOM parsing in this project is done in Python.** The core is Rust. This
skill covers the tooling *around* it: assembling and inspecting the corpus,
computing the expected value of a fixture independently of the implementation,
and building synthetic cases.

That independence is the point. HLD 27.2 R2:

> Tests derive from the spec or the oracle, never from reading the
> implementation. An agent asked to test a function will assert what it does,
> not what it should do.

A fixture whose expected value was produced by the Rust code it checks proves
the code agrees with itself. Computing it here, in a different language from a
different reading of PS3.3, is what makes it a test.

For the standard itself, the LUT formulas, the geometry and the traps, load
`dicom-expert`.

## Setup

```bash
python3 -m pip install 'pydicom[all]' numpy
python3 -m pip install pylibjpeg pylibjpeg-libjpeg pylibjpeg-openjpeg
brew install dcmtk          # macOS
```

`pylibjpeg-openjpeg` matters: without a decoder handler, `ds.pixel_array`
raises on every compressed transfer syntax, which is most of a real corpus.

---

## 1. Inspecting a candidate corpus case

Before a case earns a manifest row, know what it is. The manifest's
`modality`, `transfer_syntax` and `category` columns are not decoration, they
are what makes the corpus's coverage checkable.

```python
import pydicom

ds = pydicom.dcmread(path, stop_before_pixels=True)   # fast, metadata only

print(ds.file_meta.TransferSyntaxUID)                 # -> manifest column
print(ds.Modality, ds.SOPClassUID)
print(ds.Rows, ds.Columns, ds.get("NumberOfFrames", 1))
print(ds.BitsAllocated, ds.BitsStored, ds.HighBit, ds.PixelRepresentation)
print(ds.PhotometricInterpretation, ds.SamplesPerPixel)
print(ds.get("RescaleSlope"), ds.get("RescaleIntercept"))
print(ds.get("WindowCenter"), ds.get("WindowWidth"), ds.get("VOILUTFunction"))
print(ds.get("ImagePositionPatient"), ds.get("ImageOrientationPatient"))
print(ds.get("PixelSpacing"))
```

`stop_before_pixels=True` is the difference between seconds and minutes across
a whole collection.

### Command-line equivalent

```bash
dcmdump image.dcm                          # everything
dcmdump +P "0028,1050" +P "0028,1051" f.dcm   # window centre and width
dcmdump --print-filename +P "0008,0060" *.dcm  # modality across a directory
```

### Confirming a file is DICOM at all

`.githooks/pre-commit` checks for `DICM` at byte 128. The same test in the
shell, useful when triaging a directory of extensionless files:

```bash
dd if=suspect bs=1 skip=128 count=4 2>/dev/null   # prints DICM if it is one
```

---

## 2. Coverage the corpus must have

The tolerance policy in `docs/hld/22-testing-and-tolerance.md` section 25.1
distinguishes classes, and **an untested class has an untested tolerance**.
Check coverage before declaring F-009 done:

```python
import collections, pydicom
from pathlib import Path

seen = collections.Counter()
for p in Path(corpus).rglob("*"):
    if not p.is_file():
        continue
    try:
        ds = pydicom.dcmread(str(p), stop_before_pixels=True)
    except pydicom.errors.InvalidDicomError:
        continue
    seen[(ds.Modality,
          str(ds.file_meta.TransferSyntaxUID),
          ds.PhotometricInterpretation,
          int(ds.BitsAllocated))] += 1

for key, n in sorted(seen.items()):
    print(n, key)
```

The minimum set the oracle needs:

- **Monochrome 16-bit** signed and unsigned, `BitsStored < BitsAllocated`.
  This is where the mask-and-sign-extend traps live.
- **`MONOCHROME1`**, at least one. It is the inversion trap.
- **Colour**, RGB and a `YBR_FULL_422`, for the chroma and planar paths.
- **Non-square `PixelSpacing`**, which is the only case that catches a
  transposed spacing index.
- **One case per transfer syntax** the codec registry will claim.
- **A multiframe or enhanced instance**, for per-frame functional groups.
- **A series with non-uniform slice spacing**, so the volume builder's refusal
  path is exercised rather than assumed.

---

## 3. Computing a fixture's expected value

This is the part that matters. Compute from the **formula in the standard**,
transcribed, not from anything Ocelli produces.

```python
import math

def voi_linear(x, c, w, ymin=0.0, ymax=255.0):
    """PS3.3 C.11.2.1.2. Requires w >= 1."""
    assert w >= 1
    c_ = c - 0.5
    w_ = w - 1
    if x <= c_ - w_ / 2:
        return ymin
    if x > c_ + w_ / 2:                      # NOTE: strict >, and <= above
        return ymax
    return ((x - c_) / w_ + 0.5) * (ymax - ymin) + ymin


def voi_linear_exact(x, c, w, ymin=0.0, ymax=255.0):
    """PS3.3 C.11.2.1.3.2. Requires w > 0. No -0.5 and no -1."""
    assert w > 0
    if x <= c - w / 2:
        return ymin
    if x > c + w / 2:
        return ymax
    return ((x - c) / w + 0.5) * (ymax - ymin) + ymin


def voi_sigmoid(x, c, w, ymin=0.0, ymax=255.0):
    """PS3.3 C.11.2.1.3.1. Requires w > 0."""
    assert w > 0
    return (ymax - ymin) / (1 + math.exp(-4 * (x - c) / w)) + ymin


# The HLD section 18.3 table. Soft tissue, centre 40, width 400, 0 to 255.
for hu in (-160, 40, 240, -60):
    print(f"{hu:6}  {voi_linear(hu, 40, 400):8.3f}  "
          f"{voi_linear_exact(hu, 40, 400):8.3f}")
```

Expected, and these four rows go into the Rust suite **before** the shader is
written:

```
  -160     0.000     1.594
    40   127.819   127.500
   240   255.000   255.000
   -60    63.910    63.750
```

**At the window centre the two functions differ by 0.32 of 255.** Invisible in
a screenshot, immediate in a pixel diff. That single number is the argument for
the whole oracle.

### The formula you will see everywhere, and must not use

```python
# WRONG. Neither LINEAR nor LINEAR_EXACT. Do not copy this into anything.
img_min = center - width // 2
img_max = center + width // 2
```

It omits `c - 0.5` and `w - 1`, so it is not `LINEAR`, and it omits the `+ 0.5`
centring, so it is not `LINEAR_EXACT` either. It is extremely common in
tutorials and blog posts because it produces a picture that looks right. If you
see it in a reference you are consulting, that reference is not authoritative
on the LUT chain. See `docs/SOURCE-POLICY.md`.

### Stored to modality, checked by hand

```python
def modality_value(stored, slope, intercept):
    """PS3.3 C.11.1. A ModalityLUTSequence, if present, REPLACES this."""
    return stored * slope + intercept
```

### Unpacking a stored value, checked by hand

```python
def stored_value(raw, bits_stored, high_bit, pixel_representation):
    v = (raw >> (high_bit + 1 - bits_stored)) & ((1 << bits_stored) - 1)
    if pixel_representation == 1 and v & (1 << (bits_stored - 1)):
        v -= 1 << bits_stored              # two's complement sign extension
    return v

assert stored_value(0xF800, 12, 11, 1) == -2048   # 12-bit signed minimum
assert stored_value(0x07FF, 12, 11, 1) == 2047    # 12-bit signed maximum
assert stored_value(0x0FFF, 12, 11, 0) == 4095    # 12-bit unsigned maximum
```

---

## 4. Geometry, computed independently

```python
import numpy as np

def voxel_to_patient(ipp, iop, pixel_spacing, col_i, row_j):
    """PS3.3 C.7.6.2.1.1.

    pixel_spacing is [between ROWS, between COLUMNS]. So [0] scales the COLUMN
    direction cosine and [1] scales the ROW direction cosine. Getting this
    backwards is invisible on square pixels and wrong on everything else.
    """
    ipp = np.asarray(ipp, dtype=float)
    row_dir = np.asarray(iop[0:3], dtype=float)
    col_dir = np.asarray(iop[3:6], dtype=float)
    return (ipp
            + col_i * float(pixel_spacing[1]) * row_dir
            + row_j * float(pixel_spacing[0]) * col_dir)


def slice_spacing(datasets):
    """Derive spacing from projected IPP. NEVER from SpacingBetweenSlices."""
    row_dir = np.asarray(datasets[0].ImageOrientationPatient[0:3], float)
    col_dir = np.asarray(datasets[0].ImageOrientationPatient[3:6], float)
    normal = np.cross(row_dir, col_dir)
    normal /= np.linalg.norm(normal)

    projected = sorted(
        float(np.dot(np.asarray(ds.ImagePositionPatient, float), normal))
        for ds in datasets)
    gaps = np.diff(projected)
    return gaps.min(), gaps.max(), np.median(gaps)
```

If `min` and `max` differ from the median by more than a small tolerance, the
series has **non-uniform spacing** and the volume builder must refuse it rather
than average. Rendering it as uniform distorts geometry in a way no measurement
tool flags.

---

## 5. Synthetic fixtures

Sometimes the case you need does not exist in any public collection: a
specific `BitsStored`, a `MONOCHROME1` with a known gradient, a deliberately
non-uniform series. Build it, and design the values so the expected output is
computable by hand.

```python
import numpy as np, pydicom, datetime
from pydicom.dataset import Dataset, FileDataset
from pydicom.uid import generate_uid, ExplicitVRLittleEndian

def synthetic_ct(path, rows=8, cols=8, bits_stored=12, signed=True):
    """A tiny CT whose pixel values are their own linear index.

    Small and predictable on purpose. An 8x8 frame can be asserted value by
    value in a test, and a gradient makes an off-by-one in the unpack visible.
    """
    meta = Dataset()
    meta.MediaStorageSOPClassUID = "1.2.840.10008.5.1.4.1.1.2"
    meta.MediaStorageSOPInstanceUID = generate_uid()
    meta.TransferSyntaxUID = ExplicitVRLittleEndian

    ds = FileDataset(str(path), {}, file_meta=meta, preamble=b"\0" * 128)
    ds.SOPClassUID = meta.MediaStorageSOPClassUID
    ds.SOPInstanceUID = meta.MediaStorageSOPInstanceUID
    ds.StudyInstanceUID = generate_uid()
    ds.SeriesInstanceUID = generate_uid()
    ds.FrameOfReferenceUID = generate_uid()
    ds.Modality = "CT"
    ds.StudyDate = datetime.date.today().strftime("%Y%m%d")

    ds.Rows, ds.Columns = rows, cols
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = "MONOCHROME2"
    ds.BitsAllocated = 16
    ds.BitsStored = bits_stored
    ds.HighBit = bits_stored - 1
    ds.PixelRepresentation = 1 if signed else 0

    ds.RescaleSlope = "1"
    ds.RescaleIntercept = "-1024"
    ds.WindowCenter = "40"
    ds.WindowWidth = "400"
    ds.VOILUTFunction = "LINEAR"

    ds.ImagePositionPatient = ["0", "0", "0"]
    ds.ImageOrientationPatient = ["1", "0", "0", "0", "1", "0"]
    ds.PixelSpacing = ["0.5", "0.25"]        # deliberately NON-SQUARE
    ds.SliceThickness = "1.0"

    dtype = np.int16 if signed else np.uint16
    values = np.arange(rows * cols, dtype=dtype).reshape(rows, cols)
    ds.PixelData = values.tobytes()
    ds.save_as(str(path), enforce_file_format=True)
    return ds
```

**Deliberately non-square `PixelSpacing`.** A square-pixel synthetic fixture
cannot catch a transposed spacing index, which is one of the traps this project
is most exposed to.

**HighBit is `BitsStored - 1`** here, meaning the value is right-aligned. Real
scanners also produce left-aligned data where `HighBit` is 15, so make one of
each.

### Do not put a synthetic fixture in git either

It is still a `.dcm` and the pre-commit hook still refuses it, deliberately.
Generate it into `$OCELLI_CORPUS_DIR` from a **committed generator script**,
which is better than a committed binary anyway: the script says what the case
is for, and a binary does not.

---

## 6. Adding to the manifest

```bash
export OCELLI_CORPUS_DIR=/path/to/corpus
python3 scripts/corpus_check.py --add "$OCELLI_CORPUS_DIR/ct/case001.dcm" \
  --modality CT --transfer-syntax 1.2.840.10008.1.2.1 \
  --category stack-window --source "TCIA <collection>" \
  --licence "CC BY 3.0" --licence-url "https://..." \
  --url ""
python3 scripts/corpus_check.py
```

Commit the manifest row. Never the file.

**`licence` and `licence_url` are not optional.** A case whose terms are
unrecorded cannot be redistributed or cited, so it is not usable as evidence
in the thing this corpus exists to support.

---

## 7. DCMTK, for the things pydicom is awkward at

```bash
dcmconv +te in.dcm out.dcm            # decompress to Explicit VR Little Endian
dcmdjpeg in.dcm out.dcm               # decode JPEG to native
dcmdjpls in.dcm out.dcm               # decode JPEG-LS, useful for gate A2
dcmcjp2k in.dcm out.dcm               # encode JPEG 2000
dcmj2pnm --write-png in.dcm out.png   # a quick look, NOT a reference render
```

**`dcmj2pnm` output is not an oracle.** It applies its own windowing and its
own rounding. The reference is cornerstone3D through the harness, per HLD
section 11. Use DCMTK to see whether a file is intact, not to decide whether a
pixel is right.

`dcmdjpls` is genuinely useful for Appendix A gate A2, because it gives an
independent JPEG-LS decode to compare a candidate Rust or CharLS path against.
Same for gate A1: decode HTJ2K two ways and compare bit for bit, which is
exactly what the gate asks.

---

## 8. Looking an attribute up

[**DICOM Standard Browser**](https://dicom.innolitics.com/ciods) is the fastest
route from "what is this tag" to the normative section. Search a keyword or a
tag, and it gives the VR, the VM, the containing module, the **Type**, and a
link into PS3.3.

Reach for it whenever a case carries an attribute you did not expect, which
during corpus work is most days. Two questions it answers immediately and
`dcmdump` does not:

- **Is this attribute required, and may it be empty?** Type 1 must be present
  and non-empty. Type 2 must be present and **may** be empty. A parser that
  treats an absent Type 2 and an empty Type 2 identically is wrong, and real
  files rely on the difference.
- **What is the condition on a 1C or 2C?** A conditional attribute's absence is
  only legal under its stated condition, so a missing `RescaleSlope` on a CT
  means something different from a missing one on an ultrasound.

Confirm anything load-bearing against the [current
standard](https://www.dicomstandard.org/current) before it becomes a fixture's
expected value. The browser can lag an edition, and a fixture comment cites the
standard, not the browser.

## 9. Checklist before calling a corpus case done

- [ ] Digest recorded, and `scripts/corpus_check.py` passes
- [ ] `licence` and `licence_url` are real and resolve
- [ ] Modality, transfer syntax and category recorded accurately
- [ ] It is not committed to git
- [ ] If synthetic, its generator script **is** committed and says what the
      case is for
- [ ] Any expected value derived from it was computed from PS3.3, not from
      Ocelli's output
