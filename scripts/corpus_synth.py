#!/usr/bin/env python3
"""Generate the synthetic layer of the golden corpus, byte-deterministically.

The corpus has two layers and this script is the first of them (F-009, E2.1).

**Why synthetic at all.** A corpus built only from real public studies cannot be
relied on to contain a signed 12-bit-in-16 CT with HighBit 15, a MONOCHROME1
with a known gradient, a non-square PixelSpacing, or a deliberately non-uniform
slice spacing. Those are the traps, and a trap you do not have a case for is a
trap you find in production.

**Why a script and not committed files.** `.githooks/pre-commit` refuses a
staged DICOM by magic bytes as well as by suffix, with no allowlist, and it is
right to. A script is also better than a binary would be: it says what each
case is for, and a binary does not.

**Why byte-determinism is a hard requirement.** `corpus/manifest.tsv` records a
sha256 per case. A generator that stamps today's date or a fresh UID produces a
different digest on every machine, and a manifest that never matches is a
manifest nobody reads. So: UIDs are derived by hash from the case name in the
`2.25.` UUID-derived arc, dates are fixed, and every file is written by pydicom
with a fixed implementation identity even when another tool produced the
codestream inside it. `scripts/tests/test_corpus_synth.py` generates twice into
different directories and compares every digest.

One thing this cannot pin: a codestream is whatever its encoder produced, so an
encoder version change can move a digest. `EXTERNAL_ENCODERS` below says which
encoder owns which syntax and which of them leave a version behind. **The ones
that leave nothing are the ones to watch**, because for those a bump moves the
digest silently and `--tool-versions` is the only thing that will say so.

A moved digest is not a hole in the manifest. It is the manifest telling you
the thing the tolerance policy was measured against moved.

**A tool used to build a case is not a tool used to check it.** DCMTK, OpenJPH
and OpenJPEG produce codestreams here. Whether Ocelli decodes them correctly is
decided by the oracle against cornerstone3D, per HLD section 11.

Usage:
  .venv/bin/python scripts/corpus_synth.py           # write to corpus/data
  uv run scripts/corpus_synth.py --out DIR          # write somewhere else
  uv run scripts/corpus_synth.py --manifest-rows    # print the rows, no files
  uv run scripts/corpus_synth.py --write-manifest   # regenerate the owned rows
  uv run scripts/corpus_synth.py --tool-versions    # what would produce them
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import subprocess
import sys
import tempfile
from importlib import metadata
from pathlib import Path

import numpy as np
import pydicom
from pydicom.dataset import Dataset, FileDataset, FileMetaDataset
from pydicom.encaps import encapsulate
from pydicom.uid import UID

sys.path.insert(0, str(Path(__file__).resolve().parent))
from corpus_check import (HEADER, MANIFEST,  # noqa: E402
                          corpus_dir, digest)

# ---------------------------------------------------------------------------
# Determinism
# ---------------------------------------------------------------------------

# ISO/IEC 9834-8 assigns the `2.25.` arc to UUIDs rendered as a single integer.
# Deriving that integer by hash from the case name gives a UID that is unique,
# stable across machines, and obviously not a real institution's.
UID_ARC = "2.25."
UID_SALT = "ocelli-f009-synthetic:"

IMPLEMENTATION_VERSION = "OCELLI_F009_1"

# Fixed and in the past, so nothing here can be mistaken for an acquisition.
STUDY_DATE = "20200101"
STUDY_TIME = "120000.000000"

PATIENT_NAME = "Synthetic^Ocelli"
PATIENT_ID = "OCELLI-SYNTH"

# ---------------------------------------------------------------------------
# Case geometry
# ---------------------------------------------------------------------------

# The trap cases are tiny on purpose: a 12 by 20 frame can be asserted value by
# value. Rows differ from Columns so that a transposed index is visible.
TRAP_ROWS, TRAP_COLS = 12, 20

# The transfer-syntax cases have to clear OpenJPEG's floor, which the trap size
# above does not. Six resolution levels is five decompositions, each halving the
# image, so the short side must be at least 2**5 = 32 samples or
# opj_start_compress() fails outright. Bisected against the same encoder path
# the generator uses, and it agrees with the arithmetic: 31 fails, 32 succeeds.
#
# 64 is that floor doubled, chosen for headroom rather than because 64 is the
# limit. Anyone needing a smaller transfer-syntax case has 32 to work with.
SYNTAX_ROWS, SYNTAX_COLS = 64, 96

# Deliberately non-square everywhere except where a case says otherwise. A
# square-pixel fixture cannot catch a transposed PixelSpacing index, and
# PS3.3 C.7.6.2.1.1 makes PixelSpacing[0] the spacing BETWEEN ROWS.
NON_SQUARE_SPACING = ["0.5", "0.25"]

CT_STORAGE = "1.2.840.10008.5.1.4.1.1.2"
ENHANCED_CT_STORAGE = "1.2.840.10008.5.1.4.1.1.2.1"
CR_STORAGE = "1.2.840.10008.5.1.4.1.1.1"
MR_STORAGE = "1.2.840.10008.5.1.4.1.1.4"
SC_STORAGE = "1.2.840.10008.5.1.4.1.1.7"
US_STORAGE = "1.2.840.10008.5.1.4.1.1.6.1"

# The eight raw words the two signed 12-bit-in-16 cases open with. Identical
# bytes in both files, so the same word means different things under the two
# headers. The expected stored values are hand-computed in
# scripts/tests/test_corpus_synth.py from PS3.3 C.7.6.3.1.4, and deliberately
# not computed here, because a generator that also states the answer is not
# being tested by a test that reads it.
PROBE_WORDS = (0xF800, 0x07FF, 0x0FFF, 0x0801, 0x8000, 0x7FF0, 0xFFF0, 0x800F)

# The oblique orientation the two CT series use. Both direction cosines are
# exactly unit length and their cross product is exactly unit length, so every
# slice position below is an exact decimal and a reviewer can check the
# arithmetic without a calculator.
#   row = (0.8, 0.6, 0), col = (0, 0, -1), normal = row x col = (-0.6, 0.8, 0)
SERIES_ORIENTATION = ["0.8", "0.6", "0.0", "0.0", "0.0", "-1.0"]
SERIES_NORMAL = (-0.6, 0.8, 0.0)
SERIES_SPACING = 2.5
SERIES_SLICES = 10

# Slice 7 is displaced by half a gap. The median gap stays 2.5, and the two
# gaps either side of it become 3.75 and 1.25. A volume builder that averages
# instead of refusing produces a plausible reformat that is wrong by a
# constant factor, which no measurement tool flags.
NONUNIFORM_SLICE = 7
NONUNIFORM_OFFSET = 1.25

JPEG_LS_NEAR = 3       # ISO/IEC 14495-1 NEAR: the guaranteed max abs error
JPEG_QUALITY = 90      # IJG quality factor for the two lossy JPEG cases
J2K_COMPRESSION_RATIO = 20.0
HTJ2K_QSTEP = 0.001

LOSSY_TRANSFER_SYNTAXES = frozenset({
    "1.2.840.10008.1.2.4.50",     # JPEG Baseline, DCT
    "1.2.840.10008.1.2.4.51",     # JPEG Extended, DCT
    "1.2.840.10008.1.2.4.81",     # JPEG-LS Near-Lossless
    "1.2.840.10008.1.2.4.91",     # JPEG 2000, irreversible 9/7
    "1.2.840.10008.1.2.4.203",    # HTJ2K, irreversible 9/7
})

# The four syntaxes whose pixel data is native rather than encapsulated. Every
# other case in SYNTAX_CASES is a codestream some encoder produced.
NATIVE_TRANSFER_SYNTAXES = frozenset({
    "1.2.840.10008.1.2",          # Implicit VR Little Endian
    "1.2.840.10008.1.2.1",        # Explicit VR Little Endian
    "1.2.840.10008.1.2.1.99",     # Deflated Explicit VR Little Endian
    "1.2.840.10008.1.2.2",        # Explicit VR Big Endian, retired
})

# Which external encoder produces which transfer syntax, and the version stamp
# it leaves in the file, or None when it leaves none. Keyed by SYNTAX, not by
# filename, so that the case names live in SYNTAX_CASES and nowhere else.
EXTERNAL_ENCODERS = {
    "DCMTK, dcmcjpeg": (
        ("1.2.840.10008.1.2.4.50", "1.2.840.10008.1.2.4.51",
         "1.2.840.10008.1.2.4.57", "1.2.840.10008.1.2.4.70"), None),
    "pyjpegls": (
        ("1.2.840.10008.1.2.4.80", "1.2.840.10008.1.2.4.81"), None),
    "OpenJPEG, through pylibjpeg-openjpeg": (
        ("1.2.840.10008.1.2.4.90", "1.2.840.10008.1.2.4.91"),
        rb"Created by OpenJPEG version [0-9.]+"),
    "OpenJPH, ojph_compress": (
        ("1.2.840.10008.1.2.4.201", "1.2.840.10008.1.2.4.202",
         "1.2.840.10008.1.2.4.203"), rb"OpenJPH Ver [0-9.]+"),
}

# The one compressed syntax no external encoder touches. pydicom implements RLE
# itself, PS3.5 Annex G being fully specified, and every plugin it offers emits
# identical bytes, which the test asserts rather than assuming.
INTERNAL_ENCODER_SYNTAX = "1.2.840.10008.1.2.5"

# DCMTK writes this and nothing else here does, which is what tells its four
# cases from pyjpegls's two. They are the two producers that leave no version,
# so without a discriminator their syntaxes could be exchanged unnoticed.
DCMTK_FINGERPRINT = "DerivationDescription"


def cases_for(producer: str) -> tuple[str, ...]:
    """The filenames one producer owns, read out of SYNTAX_CASES."""
    syntaxes = set(EXTERNAL_ENCODERS[producer][0])
    return tuple(sorted(name for name, uid in SYNTAX_CASES.items()
                        if uid in syntaxes))

# filename -> the transfer syntax it declares.
SYNTAX_CASES = {
    "implicit_vr_le.dcm": "1.2.840.10008.1.2",
    "explicit_vr_le.dcm": "1.2.840.10008.1.2.1",
    "deflated_explicit_vr_le.dcm": "1.2.840.10008.1.2.1.99",
    "explicit_vr_be.dcm": "1.2.840.10008.1.2.2",
    "rle_lossless.dcm": "1.2.840.10008.1.2.5",
    "jpeg_baseline_rgb8.dcm": "1.2.840.10008.1.2.4.50",
    "jpeg_extended_12.dcm": "1.2.840.10008.1.2.4.51",
    "jpeg_lossless_p14.dcm": "1.2.840.10008.1.2.4.57",
    "jpeg_lossless_p14_sv1.dcm": "1.2.840.10008.1.2.4.70",
    "jpegls_lossless.dcm": "1.2.840.10008.1.2.4.80",
    "jpegls_near_lossless.dcm": "1.2.840.10008.1.2.4.81",
    "j2k_lossless.dcm": "1.2.840.10008.1.2.4.90",
    "j2k_lossy.dcm": "1.2.840.10008.1.2.4.91",
    "htj2k_lossless.dcm": "1.2.840.10008.1.2.4.201",
    "htj2k_lossless_rpcl.dcm": "1.2.840.10008.1.2.4.202",
    "htj2k_lossy.dcm": "1.2.840.10008.1.2.4.203",
}

# The uncompressed case each compressed one must decode back to. Three bit
# depths are needed because JPEG Baseline is 8-bit only and JPEG Extended is
# 12-bit only, so a single base cannot serve all sixteen.
REFERENCE_MONO16 = "explicit_vr_le.dcm"
REFERENCE_MONO12 = "reference_mono12.dcm"
REFERENCE_RGB8 = "reference_rgb8.dcm"

SYNTAX_REFERENCE = {name: REFERENCE_MONO16 for name in SYNTAX_CASES}
SYNTAX_REFERENCE["jpeg_extended_12.dcm"] = REFERENCE_MONO12
SYNTAX_REFERENCE["jpeg_baseline_rgb8.dcm"] = REFERENCE_RGB8

# What every generated row records. These files are the output of a script in
# this repository, so they carry this repository's own terms (see LICENSE). The
# URL is the Apache half, which is the one LICENSE itself cites by URL.
SYNTH_SOURCE = "Ocelli synthetic, scripts/corpus_synth.py"
SYNTH_LICENCE = "MIT OR Apache-2.0"
SYNTH_LICENCE_URL = "https://www.apache.org/licenses/LICENSE-2.0"

MANIFEST_OWNED_PREFIXES = ("synthetic/", "syntax/")


def fixed_uid(label: str) -> str:
    """A stable UID for a case name. Same input, same UID, on any machine."""
    packed = hashlib.sha256((UID_SALT + label).encode("utf-8")).digest()[:16]
    return UID_ARC + str(int.from_bytes(packed, "big"))


def ramp(rows: int, cols: int, maximum: int, dtype: type) -> np.ndarray:
    """A monotone ramp filling exactly 0 to `maximum` over the frame.

    Monotone and smooth on purpose: it makes an off-by-one in an unpack
    visible, and it keeps the DCT and irreversible-wavelet cases from being
    dominated by ringing that would tell us nothing about the container.
    """
    count = rows * cols
    values = (np.arange(count, dtype=np.int64) * maximum) // (count - 1)
    return values.astype(dtype).reshape(rows, cols)


def rgb_ramp(rows: int, cols: int) -> np.ndarray:
    """Red along the columns, green along the rows, blue the mean of the two.

    Smooth horizontally so that 4:2:2 chroma subsampling, which averages
    horizontally, costs almost nothing. The case exists to exercise the colour
    path, not to measure a codec.
    """
    red = (np.arange(cols, dtype=np.int64) * 255) // (cols - 1)
    green = (np.arange(rows, dtype=np.int64) * 255) // (rows - 1)
    red = np.broadcast_to(red, (rows, cols))
    green = np.broadcast_to(green[:, None], (rows, cols))
    blue = (red + green) // 2
    return np.stack([red, green, blue], axis=-1).astype(np.uint8)


# ---------------------------------------------------------------------------
# Dataset construction
# ---------------------------------------------------------------------------

def new_dataset(label: str, sop_class: str, modality: str,
                transfer_syntax: str = "1.2.840.10008.1.2.1",
                study: str | None = None,
                series: str | None = None,
                frame_of_reference: str | None = None) -> FileDataset:
    """A minimal instance with every identifier derived from `label`."""
    meta = FileMetaDataset()
    meta.FileMetaInformationVersion = b"\x00\x01"
    meta.MediaStorageSOPClassUID = UID(sop_class)
    meta.MediaStorageSOPInstanceUID = UID(fixed_uid("sop:" + label))
    meta.TransferSyntaxUID = UID(transfer_syntax)
    meta.ImplementationClassUID = UID(fixed_uid("implementation"))
    meta.ImplementationVersionName = IMPLEMENTATION_VERSION

    ds = FileDataset(label, {}, file_meta=meta, preamble=b"\0" * 128)
    ds.SpecificCharacterSet = "ISO_IR 192"
    ds.SOPClassUID = meta.MediaStorageSOPClassUID
    ds.SOPInstanceUID = meta.MediaStorageSOPInstanceUID
    ds.StudyInstanceUID = UID(fixed_uid("study:" + (study or label)))
    ds.SeriesInstanceUID = UID(fixed_uid("series:" + (series or label)))
    ds.FrameOfReferenceUID = UID(
        fixed_uid("for:" + (frame_of_reference or series or label)))

    ds.PatientName = PATIENT_NAME
    ds.PatientID = PATIENT_ID
    ds.PatientBirthDate = ""          # Type 2, present and empty is correct
    ds.PatientSex = ""
    ds.StudyDate = STUDY_DATE
    ds.StudyTime = STUDY_TIME
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = STUDY_TIME
    ds.AccessionNumber = ""
    ds.ReferringPhysicianName = ""
    ds.StudyID = "1"
    ds.SeriesNumber = "1"
    ds.InstanceNumber = "1"
    ds.Modality = modality
    ds.StudyDescription = "Ocelli synthetic corpus"
    ds.SeriesDescription = label
    return ds


def set_monochrome_pixels(ds: Dataset, array: np.ndarray, bits_allocated: int,
                          bits_stored: int, high_bit: int,
                          pixel_representation: int,
                          photometric: str = "MONOCHROME2") -> None:
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = photometric
    ds.BitsAllocated = bits_allocated
    ds.BitsStored = bits_stored
    ds.HighBit = high_bit
    ds.PixelRepresentation = pixel_representation
    ds.Rows, ds.Columns = int(array.shape[0]), int(array.shape[1])
    ds.PixelData = array.tobytes()


def write(ds: FileDataset, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    little = str(ds.file_meta.TransferSyntaxUID) != "1.2.840.10008.1.2.2"
    implicit = str(ds.file_meta.TransferSyntaxUID) == "1.2.840.10008.1.2"
    pydicom.dcmwrite(str(path), ds, implicit_vr=implicit, little_endian=little,
                     enforce_file_format=True)


# ---------------------------------------------------------------------------
# Layer 1: the trap cases
# ---------------------------------------------------------------------------

def trap_frame(probe: bool) -> np.ndarray:
    """A 12 by 20 frame of raw 16-bit words.

    When `probe` is set the first eight words are PROBE_WORDS, which the
    fixture asserts against a hand-computed table. The rest is a ramp in steps
    of 16, which is one step of a left-aligned 12-bit field and 16 steps of a
    right-aligned one, so the same bytes are meaningful under either header.
    """
    count = TRAP_ROWS * TRAP_COLS
    words = np.array([(index * 16) & 0xFFFF for index in range(count)],
                     dtype=np.uint16)
    if probe:
        words[:len(PROBE_WORDS)] = np.array(PROBE_WORDS, dtype=np.uint16)
    return words.reshape(TRAP_ROWS, TRAP_COLS)


def ct_common(ds: Dataset) -> None:
    ds.RescaleSlope = "1"
    ds.RescaleIntercept = "-1024"
    ds.RescaleType = "HU"
    ds.WindowCenter = "40"
    ds.WindowWidth = "400"
    ds.VOILUTFunction = "LINEAR"
    ds.ImagePositionPatient = ["0.0", "0.0", "0.0"]
    ds.ImageOrientationPatient = ["1.0", "0.0", "0.0", "0.0", "1.0", "0.0"]
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.SliceThickness = "1.0"


def case_signed_12in16(out: Path, high_bit: int, name: str) -> None:
    """BitsStored 12 in a 16-bit container, signed, at both alignments.

    PS3.3 C.7.6.3.1.4. Real scanners produce both: HighBit 11 is right
    aligned, HighBit 15 is left aligned. A reader that ignores HighBit gets
    one of the two right, and both look like plausible Hounsfield numbers.
    """
    ds = new_dataset(name, CT_STORAGE, "CT")
    ct_common(ds)
    set_monochrome_pixels(ds, trap_frame(probe=True), 16, 12, high_bit, 1)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_unsigned_16(out: Path) -> None:
    """The plain baseline: BitsStored equals BitsAllocated, unsigned."""
    name = "ct_unsigned_16"
    ds = new_dataset(name, CT_STORAGE, "CT")
    ct_common(ds)
    ds.RescaleIntercept = "0"
    set_monochrome_pixels(
        ds, ramp(TRAP_ROWS, TRAP_COLS, 65535, np.uint16), 16, 16, 15, 0)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_monochrome1(out: Path) -> None:
    """MONOCHROME1: minimum value is WHITE (PS3.3 C.7.6.3.1.2).

    Plain radiography really does use it. Rendered as MONOCHROME2 it is a
    photographic negative, which is at least visible. Inverted twice, once
    here and once in the presentation stage, it cancels and produces a
    correct-looking image with wrong intermediate values.
    """
    name = "cr_monochrome1"
    ds = new_dataset(name, CR_STORAGE, "CR")
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.WindowCenter = "2048"
    ds.WindowWidth = "4096"
    set_monochrome_pixels(ds, ramp(TRAP_ROWS, TRAP_COLS, 4095, np.uint16),
                          16, 12, 11, 0, photometric="MONOCHROME1")
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_nonsquare_spacing(out: Path) -> None:
    """Non-square pixels and a non-square frame, so a transposed index shows.

    PS3.3 C.7.6.2.1.1: PixelSpacing[0] is the spacing between ROWS, so it
    scales the COLUMN direction cosine. Backwards is invisible on the
    overwhelmingly common square-pixel study.
    """
    name = "mr_nonsquare_spacing"
    ds = new_dataset(name, MR_STORAGE, "MR")
    ds.ImagePositionPatient = ["0.0", "0.0", "0.0"]
    ds.ImageOrientationPatient = ["1.0", "0.0", "0.0", "0.0", "1.0", "0.0"]
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.SliceThickness = "3.0"
    ds.WindowCenter = "1024"
    ds.WindowWidth = "2048"
    set_monochrome_pixels(ds, ramp(TRAP_ROWS, TRAP_COLS * 2, 4095, np.uint16),
                          16, 12, 11, 0)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_series(out: Path, name: str, nonuniform: bool) -> None:
    """Ten oblique slices, uniformly spaced or with one gap off the median."""
    directory = out / "synthetic" / name
    for index in range(SERIES_SLICES):
        distance = index * SERIES_SPACING
        if nonuniform and index == NONUNIFORM_SLICE:
            distance += NONUNIFORM_OFFSET
        position = [f"{distance * axis:.6f}" for axis in SERIES_NORMAL]

        label = f"{name}/slice_{index:03d}"
        ds = new_dataset(label, CT_STORAGE, "CT", series=name, study=name,
                         frame_of_reference=name)
        ct_common(ds)
        ds.InstanceNumber = str(index + 1)
        ds.ImageOrientationPatient = list(SERIES_ORIENTATION)
        ds.ImagePositionPatient = position
        ds.SliceThickness = f"{SERIES_SPACING}"
        # SpacingBetweenSlices is deliberately absent. It is frequently wrong
        # when present, and the ground truth is the projected IPP difference.
        set_monochrome_pixels(
            ds, (trap_frame(probe=False) + index * 16).astype(np.uint16),
            16, 12, 11, 0)
        write(ds, directory / f"slice_{index:03d}.dcm")


def case_rgb(out: Path, planar: int, name: str) -> None:
    """The same colour image in both PlanarConfiguration layouts.

    PS3.3 C.7.6.3.1.3. 0 is RGBRGB, 1 is RRR GGG BBB. A reader that ignores
    (0028,0006) renders the second as colour noise, which is at least loud.
    """
    ds = new_dataset(name, SC_STORAGE, "OT")
    ds.ConversionType = "WSD"
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "RGB"
    ds.PlanarConfiguration = planar
    ds.BitsAllocated = 8
    ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.Rows, ds.Columns = TRAP_ROWS, TRAP_COLS
    pixels = rgb_ramp(TRAP_ROWS, TRAP_COLS)
    if planar == 1:
        pixels = np.transpose(pixels, (2, 0, 1))
    ds.PixelData = np.ascontiguousarray(pixels).tobytes()
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_ybr_full_422(out: Path) -> None:
    """Uncompressed YBR_FULL_422, the chroma path and the colour tolerance class.

    PS3.3 C.7.6.3.1.2: the chroma channels are subsampled two to one
    horizontally and each pair of pixels is stored Y1 Y2 Cb Cr. A frame is
    therefore Rows * Columns * 2 bytes, not * 3, and a reader that sizes the
    buffer from SamplesPerPixel alone over-reads by half a frame.
    """
    name = "us_ybr_full_422"
    ds = new_dataset(name, US_STORAGE, "US")
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "YBR_FULL_422"
    ds.PlanarConfiguration = 0
    ds.BitsAllocated = 8
    ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.Rows, ds.Columns = TRAP_ROWS, TRAP_COLS
    ds.PixelSpacing = list(NON_SQUARE_SPACING)

    luma = ramp(TRAP_ROWS, TRAP_COLS, 255, np.uint8)
    data = bytearray()
    for row in range(TRAP_ROWS):
        for pair in range(TRAP_COLS // 2):
            blue_chroma = (row * 8 + pair * 4) % 256
            data += bytes((int(luma[row, pair * 2]),
                           int(luma[row, pair * 2 + 1]),
                           blue_chroma,
                           255 - blue_chroma))
    ds.PixelData = bytes(data)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_multiframe(out: Path) -> None:
    """Enhanced CT with rescale and window differing per frame.

    PS3.3 C.7.6.16: per-frame functional groups are consulted BEFORE shared
    ones. Rescale and window live only in the per-frame groups here and are
    absent from the top level, so a legacy-shaped reader that reads
    RescaleSlope once from the dataset finds nothing rather than finding one
    frame's transform and applying it to all of them.
    """
    name = "ct_multiframe_perframe"
    frames = 3
    ds = new_dataset(name, ENHANCED_CT_STORAGE, "CT")
    ds.NumberOfFrames = str(frames)
    ds.ImageType = ["DERIVED", "PRIMARY", "VOLUME", "NONE"]
    ds.InstanceNumber = "1"
    ds.ContentQualification = "RESEARCH"

    stack = np.concatenate([
        (trap_frame(probe=False).astype(np.int32) - 1000 + index * 1000)
        .astype(np.int16).reshape(1, TRAP_ROWS, TRAP_COLS)
        for index in range(frames)])
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = "MONOCHROME2"
    ds.BitsAllocated = 16
    ds.BitsStored = 16
    ds.HighBit = 15
    ds.PixelRepresentation = 1
    ds.Rows, ds.Columns = TRAP_ROWS, TRAP_COLS
    ds.PixelData = stack.astype("<i2").tobytes()

    dimension = Dataset()
    dimension.DimensionOrganizationUID = UID(fixed_uid("dimorg:" + name))
    ds.DimensionOrganizationSequence = [dimension]
    index_item = Dataset()
    index_item.DimensionOrganizationUID = dimension.DimensionOrganizationUID
    index_item.DimensionIndexPointer = pydicom.tag.Tag(0x0020, 0x9057)
    index_item.FunctionalGroupPointer = pydicom.tag.Tag(0x0020, 0x9111)
    ds.DimensionIndexSequence = [index_item]

    measures = Dataset()
    measures.PixelSpacing = list(NON_SQUARE_SPACING)
    measures.SliceThickness = "2.5"
    measures.SpacingBetweenSlices = "2.5"
    orientation = Dataset()
    orientation.ImageOrientationPatient = list(SERIES_ORIENTATION)
    shared = Dataset()
    shared.PixelMeasuresSequence = [measures]
    shared.PlaneOrientationSequence = [orientation]
    ds.SharedFunctionalGroupsSequence = [shared]

    slopes = ["1", "2", "0.5"]
    intercepts = ["-1024", "-2048", "0"]
    centres = ["40", "300", "-600"]
    widths = ["400", "1500", "1600"]

    per_frame = []
    for index in range(frames):
        distance = index * SERIES_SPACING
        position = Dataset()
        position.ImagePositionPatient = [f"{distance * axis:.6f}"
                                         for axis in SERIES_NORMAL]
        transformation = Dataset()
        transformation.RescaleSlope = slopes[index]
        transformation.RescaleIntercept = intercepts[index]
        transformation.RescaleType = "HU"
        voi = Dataset()
        voi.WindowCenter = centres[index]
        voi.WindowWidth = widths[index]
        voi.VOILUTFunction = "LINEAR"
        content = Dataset()
        content.StackID = "1"
        content.InStackPositionNumber = index + 1
        content.DimensionIndexValues = [index + 1]

        frame = Dataset()
        frame.PlanePositionSequence = [position]
        frame.PixelValueTransformationSequence = [transformation]
        frame.FrameVOILUTSequence = [voi]
        frame.FrameContentSequence = [content]
        per_frame.append(frame)
    ds.PerFrameFunctionalGroupsSequence = per_frame

    write(ds, out / "synthetic" / f"{name}.dcm")


# ---------------------------------------------------------------------------
# Layer 1b: one case per transfer syntax the codec registry will claim
# ---------------------------------------------------------------------------

def syntax_base(label: str, kind: str) -> FileDataset:
    """The uncompressed content every transfer-syntax case is built from."""
    ds = new_dataset(label, CT_STORAGE if kind != "rgb8" else SC_STORAGE,
                     "CT" if kind != "rgb8" else "OT",
                     study="syntax")
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    if kind == "mono16":
        ct_common(ds)
        ds.RescaleIntercept = "0"
        set_monochrome_pixels(
            ds, ramp(SYNTAX_ROWS, SYNTAX_COLS, 65535, np.uint16), 16, 16, 15, 0)
    elif kind == "mono12":
        ct_common(ds)
        ds.RescaleIntercept = "0"
        set_monochrome_pixels(
            ds, ramp(SYNTAX_ROWS, SYNTAX_COLS, 4095, np.uint16), 16, 12, 11, 0)
    elif kind == "rgb8":
        ds.ConversionType = "WSD"
        ds.SamplesPerPixel = 3
        ds.PhotometricInterpretation = "RGB"
        ds.PlanarConfiguration = 0
        ds.BitsAllocated = 8
        ds.BitsStored = 8
        ds.HighBit = 7
        ds.PixelRepresentation = 0
        ds.Rows, ds.Columns = SYNTAX_ROWS, SYNTAX_COLS
        ds.PixelData = rgb_ramp(SYNTAX_ROWS, SYNTAX_COLS).tobytes()
    else:
        raise ValueError(f"unknown syntax base: {kind}")
    return ds


def normalise(ds: Dataset, label: str) -> None:
    """Strip the producing tool's identity so the digest is ours, not theirs.

    DCMTK stamps its own ImplementationClassUID and version into the file meta
    group. Those are constants for a given DCMTK build, but they are not
    constants of this corpus, and a manifest digest that moves when a tool is
    upgraded for an unrelated reason is a manifest that gets ignored.

    The series and frame of reference are re-derived here too, and that is not
    cosmetic. Left inherited from the base, every mono16 syntax case lands in
    ONE series: fourteen instances sharing a frame of reference, all at
    ImagePositionPatient [0, 0, 0], all InstanceNumber 1, declaring fourteen
    different transfer syntaxes. No scanner can produce that, and a consumer
    that groups by series (the volume builder, or the oracle pushing "the same
    study through both stacks") meets duplicate slice positions it must reject.
    Each case is its own single-instance series in its own spatial frame. The
    study stays shared, so `syntax/` is still browsable as one thing.
    """
    ds.file_meta.ImplementationClassUID = UID(fixed_uid("implementation"))
    ds.file_meta.ImplementationVersionName = IMPLEMENTATION_VERSION
    ds.file_meta.FileMetaInformationVersion = b"\x00\x01"
    if "SourceApplicationEntityTitle" in ds.file_meta:
        del ds.file_meta.SourceApplicationEntityTitle
    sop = UID(fixed_uid("sop:" + label))
    ds.SOPInstanceUID = sop
    ds.file_meta.MediaStorageSOPInstanceUID = sop
    ds.SeriesInstanceUID = UID(fixed_uid("series:" + label))
    ds.FrameOfReferenceUID = UID(fixed_uid("for:" + label))


def mark_lossy(ds: Dataset, method: str) -> None:
    """PS3.3 C.7.6.1.1.5. A lossy instance has to say so, and say how."""
    ds.LossyImageCompression = "01"
    ds.LossyImageCompressionMethod = method


def encode_with_pydicom(base: FileDataset, label: str, syntax: str,
                        target: Path, **options: object) -> None:
    ds = pydicom.dcmread(str(base))
    ds.compress(UID(syntax), **options)
    normalise(ds, label)
    if syntax == "1.2.840.10008.1.2.4.81":
        mark_lossy(ds, "ISO_14495_1")
    elif syntax == "1.2.840.10008.1.2.4.91":
        mark_lossy(ds, "ISO_15444_1")
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


def encode_with_dcmcjpeg(base: Path, label: str, target: Path,
                         arguments: list[str]) -> None:
    """DCMTK supplies the four JPEG syntaxes pydicom has no encoder for."""
    with tempfile.TemporaryDirectory(prefix="ocelli-dcmtk-") as scratch:
        produced = Path(scratch) / "out.dcm"
        run(["dcmcjpeg", *arguments, "--uid-never", str(base), str(produced)])
        ds = pydicom.dcmread(str(produced))
    normalise(ds, label)
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


def encode_with_ojph(base: Path, label: str, syntax: str, target: Path,
                     arguments: list[str]) -> None:
    """OpenJPH supplies the HTJ2K codestream, which is then encapsulated here.

    PS3.5 A.4.10 and A.4.11: what goes inside the pixel-data item is the
    codestream, not a JP2 file, which is exactly what ojph_compress writes to
    a .j2c.

    The samples come out of the reference file whose header this case copies,
    so the pixels encoded and the pixels claimed have one source. Recomputing
    them alongside would be two, and the two would be free to drift.
    """
    ds = pydicom.dcmread(str(base))
    pixels = ds.pixel_array

    with tempfile.TemporaryDirectory(prefix="ocelli-ojph-") as scratch:
        source = Path(scratch) / "frame.pgm"
        produced = Path(scratch) / "frame.j2c"
        rows, cols = pixels.shape
        with source.open("wb") as handle:
            handle.write(f"P5\n{cols} {rows}\n65535\n".encode("ascii"))
            handle.write(pixels.astype(">u2").tobytes())
        run(["ojph_compress", "-i", str(source), "-o", str(produced), *arguments])
        codestream = produced.read_bytes()

    ds.file_meta.TransferSyntaxUID = UID(syntax)
    ds.PixelData = encapsulate([codestream])
    # PS3.5 Table 7.1-1 and A.4: encapsulated PixelData is OB with an undefined
    # length. The uncompressed base carries OW because its 16-bit samples are
    # words, and inheriting that here produces a file DCMTK warns about and a
    # strict reader may refuse, with nothing about the pixels looking wrong.
    ds["PixelData"].VR = "OB"
    ds["PixelData"].is_undefined_length = True
    normalise(ds, label)
    if syntax == "1.2.840.10008.1.2.4.203":
        # PS3.3 C.7.6.1.1.5.1 lists ISO_15444_15 as a Defined Term for
        # LossyImageCompressionMethod, "High-Throughput JPEG 2000 Irreversible
        # Compression", alongside ISO_10918_1, ISO_14495_1 and ISO_15444_1
        # which the other lossy cases here use. Checked against the standard,
        # not inferred from the shape of its siblings.
        mark_lossy(ds, "ISO_15444_15")
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


# The toolchain corpus/manifest.tsv was generated with, and the only record of
# it. Every entry can move a digest, and the ones that leave no version in the
# file cannot be recovered from the corpus at all, which is why they are here.
# `--tool-versions` asks each tool rather than reading this table.
BUILT_WITH = {
    "pydicom": "3.0.2",
    "numpy": "2.5.2",
    "pylibjpeg-openjpeg": "2.5.0",
    "OpenJPEG (the library inside it)": "2.5.2",
    "pyjpegls": "1.5.1",
    "DCMTK": "3.7.0",
    "OpenJPH": "0.31.0",
}


def openjph_version() -> str:
    """OpenJPH has no version flag, so ask it the way the corpus does.

    The COM marker it writes into every codestream is both the version that
    matters and the version that moves the digest, which is a better answer
    than a flag would be anyway.

    Returns "absent" rather than raising when the tool is not installed. This
    runs from `--tool-versions`, which corpus/README.md tells a developer to
    run first when their digests do not match, and a traceback instead of the
    report is worst exactly then.
    """
    if shutil.which("ojph_compress") is None:
        return "absent"
    with tempfile.TemporaryDirectory(prefix="ocelli-ojph-probe-") as scratch:
        source = Path(scratch) / "probe.pgm"
        produced = Path(scratch) / "probe.j2c"
        with source.open("wb") as handle:
            handle.write(b"P5\n32 32\n65535\n")
            handle.write(np.zeros((32, 32), dtype=">u2").tobytes())
        try:
            run(["ojph_compress", "-i", str(source), "-o", str(produced),
                 "-reversible", "true"])
        except (OSError, RuntimeError):
            return "unknown"
        found = re.search(rb"OpenJPH Ver ([0-9.]+)", produced.read_bytes())
    return found.group(1).decode("ascii").rstrip(".") if found else "unknown"


def tool_versions() -> dict[str, str]:
    """What is actually installed, asked of each tool rather than assumed."""
    found: dict[str, str] = {}
    for package in ("pydicom", "numpy", "pylibjpeg-openjpeg", "pyjpegls"):
        try:
            found[package] = metadata.version(package)
        except metadata.PackageNotFoundError:
            found[package] = "absent"
    try:
        # The binding's own version and the OpenJPEG it wraps are different
        # numbers, and it is the library's that lands in the COM marker.
        from openjpeg.utils import get_openjpeg_version
        found["OpenJPEG (the library inside it)"] = ".".join(
            str(part) for part in get_openjpeg_version())
    except (ImportError, AttributeError, TypeError):
        found["OpenJPEG (the library inside it)"] = "unknown"

    if shutil.which("dcmcjpeg") is None:
        found["DCMTK"] = "absent"
    else:
        banner = subprocess.run(["dcmcjpeg", "--version"], capture_output=True,
                                text=True).stdout
        dcmtk = re.search(r"v([0-9]+\.[0-9]+\.[0-9]+)", banner)
        found["DCMTK"] = dcmtk.group(1) if dcmtk else "unknown"
    found["OpenJPH"] = openjph_version()
    return found


def report_tool_versions() -> int:
    """Print the installed versions beside BUILT_WITH, and say if they differ.

    `corpus_check.py` reports a moved digest and cannot say why. This is what
    says why, and for the producers that leave no version in the file it is the
    only thing that can.
    """
    found = tool_versions()
    drifted = []
    print(f"{'tool':36s} {'built with':12s} {'here':12s}")
    for name, expected in BUILT_WITH.items():
        actual = found.get(name, "unknown")
        mark = "" if actual == expected else "   <- differs"
        if mark:
            drifted.append(name)
        print(f"{name:36s} {expected:12s} {actual:12s}{mark}")
    if drifted:
        print(f"\n{len(drifted)} tool(s) differ from what corpus/manifest.tsv "
              f"was built with. Regenerating will very likely change the "
              f"digest of the rows that tool produced. That is a toolchain "
              f"bump, not a corrupted corpus, and the two are worth telling "
              f"apart before anyone edits the manifest. Which rows are "
              f"external encoder output, and whose:")
        for producer in EXTERNAL_ENCODERS:
            for name in cases_for(producer):
                print(f"    syntax/{name:28s} {producer}")
        return 1
    print("\nOK: the toolchain matches BUILT_WITH, "
          "what corpus/manifest.tsv was generated with")
    return 0


def run(command: list[str]) -> None:
    result = subprocess.run(command, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"{command[0]} failed ({result.returncode}): "
                           f"{result.stderr.strip() or result.stdout.strip()}")


def generate_syntax_layer(out: Path) -> None:
    directory = out / "syntax"
    directory.mkdir(parents=True, exist_ok=True)

    mono16 = syntax_base("syntax_mono16", "mono16")
    mono12 = syntax_base("syntax_mono12", "mono12")
    rgb8 = syntax_base("syntax_rgb8", "rgb8")

    # The three uncompressed references. explicit_vr_le.dcm is both the
    # 1.2.840.10008.1.2.1 case and the reference every mono16 case is compared
    # against, which is deliberate: one file, one digest, one meaning.
    write(mono16, directory / REFERENCE_MONO16)
    write(mono12, directory / REFERENCE_MONO12)
    write(rgb8, directory / REFERENCE_RGB8)
    reference16 = directory / REFERENCE_MONO16

    for name, syntax in (("implicit_vr_le.dcm", "1.2.840.10008.1.2"),
                         ("explicit_vr_be.dcm", "1.2.840.10008.1.2.2"),
                         ("deflated_explicit_vr_le.dcm",
                          "1.2.840.10008.1.2.1.99")):
        ds = syntax_base("syntax_mono16", "mono16")
        normalise(ds, name)
        ds.file_meta.TransferSyntaxUID = UID(syntax)
        if syntax == "1.2.840.10008.1.2.2":
            # PS3.5 7.3: big endian byte-swaps the pixel data too, not only
            # the element headers. Writing little-endian words under a
            # big-endian syntax is a file that decodes to garbage.
            ds.PixelData = ramp(SYNTAX_ROWS, SYNTAX_COLS, 65535,
                                np.uint16).astype(">u2").tobytes()
        write(ds, directory / name)

    encode_with_pydicom(reference16, "rle_lossless.dcm",
                        "1.2.840.10008.1.2.5", directory / "rle_lossless.dcm")
    encode_with_pydicom(reference16, "jpegls_lossless.dcm",
                        "1.2.840.10008.1.2.4.80",
                        directory / "jpegls_lossless.dcm")
    encode_with_pydicom(reference16, "jpegls_near_lossless.dcm",
                        "1.2.840.10008.1.2.4.81",
                        directory / "jpegls_near_lossless.dcm",
                        jls_error=JPEG_LS_NEAR)
    encode_with_pydicom(reference16, "j2k_lossless.dcm",
                        "1.2.840.10008.1.2.4.90",
                        directory / "j2k_lossless.dcm")
    encode_with_pydicom(reference16, "j2k_lossy.dcm",
                        "1.2.840.10008.1.2.4.91",
                        directory / "j2k_lossy.dcm",
                        j2k_cr=[J2K_COMPRESSION_RATIO])

    encode_with_dcmcjpeg(reference16, "jpeg_lossless_p14.dcm",
                         directory / "jpeg_lossless_p14.dcm",
                         ["--encode-lossless"])
    encode_with_dcmcjpeg(reference16, "jpeg_lossless_p14_sv1.dcm",
                         directory / "jpeg_lossless_p14_sv1.dcm",
                         ["--encode-lossless-sv1"])
    encode_with_dcmcjpeg(directory / REFERENCE_RGB8, "jpeg_baseline_rgb8.dcm",
                         directory / "jpeg_baseline_rgb8.dcm",
                         ["--encode-baseline", "--quality", str(JPEG_QUALITY)])
    encode_with_dcmcjpeg(directory / REFERENCE_MONO12, "jpeg_extended_12.dcm",
                         directory / "jpeg_extended_12.dcm",
                         ["--encode-extended", "--bits-force-12",
                          "--quality", str(JPEG_QUALITY)])

    # PS3.5 A.4.10 constrains the .202 syntax to RPCL, so .201 is written LRCP
    # to make the pair distinguishable rather than only differently labelled.
    encode_with_ojph(reference16, "htj2k_lossless.dcm",
                     "1.2.840.10008.1.2.4.201",
                     directory / "htj2k_lossless.dcm",
                     ["-reversible", "true", "-prog_order", "LRCP"])
    encode_with_ojph(reference16, "htj2k_lossless_rpcl.dcm",
                     "1.2.840.10008.1.2.4.202",
                     directory / "htj2k_lossless_rpcl.dcm",
                     ["-reversible", "true", "-prog_order", "RPCL"])
    encode_with_ojph(reference16, "htj2k_lossy.dcm",
                     "1.2.840.10008.1.2.4.203",
                     directory / "htj2k_lossy.dcm",
                     ["-reversible", "false", "-qstep", str(HTJ2K_QSTEP),
                      "-prog_order", "RPCL"])


def generate(out: Path) -> Path:
    """Write the whole synthetic layer under `out`, replacing what is there."""
    for tool in ("dcmcjpeg", "ojph_compress"):
        if shutil.which(tool) is None:
            raise RuntimeError(
                f"{tool} is not on PATH. The synthetic layer needs DCMTK and "
                f"OpenJPH, see corpus/README.md.")

    out = Path(out)
    for subdirectory in ("synthetic", "syntax"):
        shutil.rmtree(out / subdirectory, ignore_errors=True)

    case_signed_12in16(out, 11, "ct_signed_12in16_right")
    case_signed_12in16(out, 15, "ct_signed_12in16_left")
    case_unsigned_16(out)
    case_monochrome1(out)
    case_nonsquare_spacing(out)
    case_series(out, "ct_series_uniform", nonuniform=False)
    case_series(out, "ct_series_nonuniform", nonuniform=True)
    case_rgb(out, 0, "sc_rgb_interleaved")
    case_rgb(out, 1, "sc_rgb_planar")
    case_ybr_full_422(out)
    case_multiframe(out)
    generate_syntax_layer(out)
    return out


# ---------------------------------------------------------------------------
# Manifest rows
# ---------------------------------------------------------------------------

# The `category` column is a comma-separated token list. The first token is the
# layer, one of `synthetic` or `real`. At least one token is a tolerance class
# from HLD section 25.1: `mono16`, `colour` or `us`. The rest name the trap.
# scripts/corpus_check.py --coverage reads exactly this.
CATEGORIES = {
    "synthetic/ct_signed_12in16_right.dcm":
        ("CT", "synthetic, mono16, signed-12in16, high-bit-11"),
    "synthetic/ct_signed_12in16_left.dcm":
        ("CT", "synthetic, mono16, signed-12in16, high-bit-15"),
    "synthetic/ct_unsigned_16.dcm": ("CT", "synthetic, mono16, unsigned-16"),
    "synthetic/cr_monochrome1.dcm": ("CR", "synthetic, mono16, monochrome1"),
    "synthetic/mr_nonsquare_spacing.dcm":
        ("MR", "synthetic, mono16, nonsquare-spacing"),
    "synthetic/sc_rgb_interleaved.dcm":
        ("OT", "synthetic, colour, planar-config-0"),
    "synthetic/sc_rgb_planar.dcm": ("OT", "synthetic, colour, planar-config-1"),
    "synthetic/us_ybr_full_422.dcm":
        ("US", "synthetic, us, colour, ybr-full-422"),
    "synthetic/ct_multiframe_perframe.dcm":
        ("CT", "synthetic, mono16, multiframe, per-frame-functional-groups"),
}
SERIES_CATEGORY = {
    "ct_series_uniform": ("CT", "synthetic, mono16, series, uniform-spacing"),
    "ct_series_nonuniform":
        ("CT", "synthetic, mono16, series, nonuniform-spacing"),
}


def manifest_rows(out: Path) -> list[str]:
    """One TSV row per generated file, digests read from what was written."""
    rows: list[dict[str, str]] = []

    for path, (modality, category) in sorted(CATEGORIES.items()):
        rows.append({"path": path, "modality": modality,
                     "transfer_syntax": "1.2.840.10008.1.2.1",
                     "category": category})
    for series, (modality, category) in sorted(SERIES_CATEGORY.items()):
        for index in range(SERIES_SLICES):
            rows.append({"path": f"synthetic/{series}/slice_{index:03d}.dcm",
                         "modality": modality,
                         "transfer_syntax": "1.2.840.10008.1.2.1",
                         "category": category})

    syntax_category = {
        REFERENCE_MONO12: "synthetic, mono16, syntax-reference",
        REFERENCE_RGB8: "synthetic, colour, syntax-reference",
    }
    for name in sorted(set(SYNTAX_CASES) | set(syntax_category)):
        syntax = SYNTAX_CASES.get(name, "1.2.840.10008.1.2.1")
        if name in syntax_category:
            category = syntax_category[name]
        elif name == "jpeg_baseline_rgb8.dcm":
            category = "synthetic, colour, transfer-syntax"
        else:
            category = "synthetic, mono16, transfer-syntax"
        modality = "OT" if "rgb8" in name else "CT"
        rows.append({"path": f"syntax/{name}", "modality": modality,
                     "transfer_syntax": syntax, "category": category})

    lines = []
    for row in rows:
        target = out / row["path"]
        if not target.is_file():
            raise RuntimeError(f"{row['path']} was not generated")
        lines.append("\t".join([
            row["path"], row["modality"], row["transfer_syntax"],
            row["category"], SYNTH_SOURCE, SYNTH_LICENCE, SYNTH_LICENCE_URL,
            digest(target), "",
        ]))
    return lines


def write_manifest(rows: list[str]) -> None:
    """Replace the generator-owned rows in place, leaving every other row."""
    existing = MANIFEST.read_text(encoding="utf-8").splitlines()
    if not existing or existing[0] != HEADER:
        sys.exit(f"manifest header must be: {HEADER}")
    kept = [line for line in existing[1:]
            if line.strip()
            and not line.split("\t")[0].startswith(MANIFEST_OWNED_PREFIXES)]
    MANIFEST.write_text("\n".join([HEADER, *rows, *kept]) + "\n",
                        encoding="utf-8")
    print(f"wrote {len(rows)} generated rows, kept {len(kept)} others")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", metavar="DIR",
                        help="corpus root, default corpus/data")
    parser.add_argument("--manifest-rows", action="store_true",
                        help="print the TSV rows for an already-generated set")
    parser.add_argument("--write-manifest", action="store_true",
                        help="regenerate the synthetic rows in the manifest")
    parser.add_argument("--tool-versions", action="store_true",
                        help="compare the installed encoders against the ones "
                             "corpus/manifest.tsv was built with")
    args = parser.parse_args()

    if args.tool_versions:
        return report_tool_versions()

    out = Path(args.out).expanduser() if args.out else corpus_dir()
    if not (args.manifest_rows or args.write_manifest):
        generate(out)
        written = sum(1 for path in out.rglob("*.dcm")
                      if path.relative_to(out).as_posix()
                      .startswith(MANIFEST_OWNED_PREFIXES))
        print(f"wrote {written} cases under {out}")
        print("next: uv run scripts/corpus_synth.py --write-manifest, "
              "then uv run scripts/corpus_check.py")
        print("if a digest moved, uv run scripts/corpus_synth.py "
              "--tool-versions says whether the toolchain did")
        return 0

    rows = manifest_rows(out)
    if args.write_manifest:
        write_manifest(rows)
    else:
        print("\n".join(rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
