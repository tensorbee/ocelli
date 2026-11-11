#!/usr/bin/env python3
"""Generate the synthetic layer of the golden corpus, byte-deterministically.

The corpus has two layers and this script is the first of them (F-009, E2.1).

**Why synthetic at all.** A corpus built only from real public studies cannot be
relied on to contain a legacy nonconforming signed 12-bit-in-16 CT with
HighBit 15, a MONOCHROME1 with a known gradient, a non-square PixelSpacing, or
a deliberately non-uniform slice spacing. Those are the traps, and a trap you
do not have a case for is a trap you find in production.

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

**A tool used to build a case is not a tool used to check it.** DCMTK, OpenJPH,
OpenJPEG and imagecodecs with libjxl produce codestreams here. Whether Ocelli
decodes them correctly is decided by the oracle against cornerstone3D, per HLD
section 11.

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
import struct
import subprocess
import sys
import tempfile
from collections.abc import Callable
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

# Generator-owned coordinates for cases retained in the permanent quirk
# registry. The generator consumes these values and scripts/quirk_check.py
# parses the same literal without importing this module, so a registry path or
# parameter cannot silently point at a different recipe.
QUIRK_CASES = {
    "case_sigmoid_width_half": {
        "paths": ("synthetic/ct_sigmoid_width_half.dcm",),
        "windowCentre": "40",
        "windowWidth": "0.5",
        "voiLutFunction": "SIGMOID",
    },
}

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

# PS3.5 A.4.12. `.111` is a known syntax the registry leaves unavailable.
# jxl-jbr 0.2.1 src/reconstruct.rs:457 panics on a malformed DHT, so the
# row is not in the lossy set and it is not read back as pixels. `.112`
# is the general JPEG XL syntax and is lossy.
JPEGXL_LOSSLESS = "1.2.840.10008.1.2.4.110"
JPEGXL_RECOMPRESSION = "1.2.840.10008.1.2.4.111"
JPEGXL_GENERAL = "1.2.840.10008.1.2.4.112"
JPEGXL_DISTANCE = 1.0
JPEGXL_EFFORT = 7


def register_jpegxl_transfer_syntaxes() -> None:
    """Teach pydicom 3.0.2 the three JPEG XL transfer syntaxes.

    Its UID dictionary has no PS3.5 A.4.12 rows, so `save_as` refuses the
    UID as not a transfer syntax. The names are Table 8.2.15-1. A later
    pydicom that already knows them is left unchanged.
    """
    from pydicom._uid_dict import UID_dictionary
    rows = {
        JPEGXL_LOSSLESS: (
            "JPEG XL Lossless", "Transfer Syntax", "", "", "JPEGXLLossless"),
        JPEGXL_RECOMPRESSION: (
            "JPEG XL JPEG Recompression", "Transfer Syntax", "", "",
            "JPEGXLJPEGRecompression"),
        JPEGXL_GENERAL: (
            "JPEG XL", "Transfer Syntax", "", "", "JPEGXL"),
    }
    for uid, row in rows.items():
        UID_dictionary.setdefault(uid, row)


register_jpegxl_transfer_syntaxes()

LOSSY_TRANSFER_SYNTAXES = frozenset({
    "1.2.840.10008.1.2.4.50",     # JPEG Baseline, DCT
    "1.2.840.10008.1.2.4.51",     # JPEG Extended, DCT
    "1.2.840.10008.1.2.4.81",     # JPEG-LS Near-Lossless
    "1.2.840.10008.1.2.4.91",     # JPEG 2000, irreversible 9/7
    "1.2.840.10008.1.2.4.203",    # HTJ2K, irreversible 9/7
    JPEGXL_GENERAL,               # JPEG XL, lossy
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
    # libjxl writes no version stamp into these codestreams. The pattern is
    # None for the same reason pyjpegls is None.
    "imagecodecs, libjxl": (
        (JPEGXL_LOSSLESS, JPEGXL_RECOMPRESSION, JPEGXL_GENERAL), None),
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
    # A second .80 case, three-component. Appendix A gate A2 recorded the row
    # as owed and F-030 added it. See case_jpegls_rgb8.
    "jpegls_lossless_rgb8.dcm": "1.2.840.10008.1.2.4.80",
    "jpegls_near_lossless.dcm": "1.2.840.10008.1.2.4.81",
    # F-243. A flat two-band frame whose runs reach the end of a line part
    # way through a run segment, which the ramp above never does. See
    # case_jpegls_banded.
    "jpegls_banded_lossless.dcm": "1.2.840.10008.1.2.4.80",
    "jpegls_banded_near_lossless.dcm": "1.2.840.10008.1.2.4.81",
    "j2k_lossless.dcm": "1.2.840.10008.1.2.4.90",
    "j2k_lossy.dcm": "1.2.840.10008.1.2.4.91",
    # The four F-239 colour rows. YBR_RCT and YBR_ICT name the codestream's
    # own multiple component transform (PS3.5 8.2.4), so each row is encoded
    # with that transform on: the two lossless rows reversible with the RCT,
    # the two lossy rows irreversible with the ICT.
    "j2k_lossless_ybr_rct.dcm": "1.2.840.10008.1.2.4.90",
    "j2k_lossy_ybr_ict.dcm": "1.2.840.10008.1.2.4.91",
    "htj2k_lossless_ybr_rct.dcm": "1.2.840.10008.1.2.4.201",
    "htj2k_lossy_ybr_ict.dcm": "1.2.840.10008.1.2.4.203",
    "htj2k_lossless.dcm": "1.2.840.10008.1.2.4.201",
    "htj2k_lossless_rpcl.dcm": "1.2.840.10008.1.2.4.202",
    "htj2k_lossy.dcm": "1.2.840.10008.1.2.4.203",
    # PS3.5 A.4.12, F-229. Five files, three UIDs.
    "jpegxl_lossless.dcm": JPEGXL_LOSSLESS,
    "jpegxl_lossless_rgb8.dcm": JPEGXL_LOSSLESS,
    "jpegxl_recompression_rgb8.dcm": JPEGXL_RECOMPRESSION,
    "jpegxl_lossy.dcm": JPEGXL_GENERAL,
    "jpegxl_xyb_rgb8.dcm": JPEGXL_GENERAL,
}

# The uncompressed case each compressed one must decode back to. Three bit
# depths are needed because JPEG Baseline is 8-bit only and JPEG Extended is
# 12-bit only, so a single base cannot serve every syntax. The colour
# codestream rows are built from the saturated reference, whose marker pixels
# put the component transform's edge cases in the decoded bytes.
REFERENCE_MONO16 = "explicit_vr_le.dcm"
REFERENCE_MONO12 = "reference_mono12.dcm"
REFERENCE_RGB8 = "reference_rgb8.dcm"
REFERENCE_RGB8_SATURATED = "reference_rgb8_saturated.dcm"
REFERENCE_BANDED16 = "reference_banded16.dcm"

# F-243. The two-band formula, s(r, c) = BANDED_LOW if c < cols // 2 else
# BANDED_HIGH, at the reported file's size and values.
BANDED_ROWS, BANDED_COLS = 32, 32
BANDED_LOW, BANDED_HIGH = 224, 1064

SYNTAX_REFERENCE = {name: REFERENCE_MONO16 for name in SYNTAX_CASES}
SYNTAX_REFERENCE["jpeg_extended_12.dcm"] = REFERENCE_MONO12
SYNTAX_REFERENCE["jpeg_baseline_rgb8.dcm"] = REFERENCE_RGB8
SYNTAX_REFERENCE["jpegls_lossless_rgb8.dcm"] = REFERENCE_RGB8
SYNTAX_REFERENCE["jpegxl_lossy.dcm"] = REFERENCE_MONO12
SYNTAX_REFERENCE["jpegxl_lossless_rgb8.dcm"] = REFERENCE_RGB8
SYNTAX_REFERENCE["jpegxl_recompression_rgb8.dcm"] = REFERENCE_RGB8
SYNTAX_REFERENCE["jpegxl_xyb_rgb8.dcm"] = REFERENCE_RGB8
SYNTAX_REFERENCE["jpegls_banded_lossless.dcm"] = REFERENCE_BANDED16
SYNTAX_REFERENCE["jpegls_banded_near_lossless.dcm"] = REFERENCE_BANDED16
SYNTAX_REFERENCE["j2k_lossless_ybr_rct.dcm"] = REFERENCE_RGB8_SATURATED
SYNTAX_REFERENCE["j2k_lossy_ybr_ict.dcm"] = REFERENCE_RGB8_SATURATED
SYNTAX_REFERENCE["htj2k_lossless_ybr_rct.dcm"] = REFERENCE_RGB8_SATURATED
SYNTAX_REFERENCE["htj2k_lossy_ybr_ict.dcm"] = REFERENCE_RGB8_SATURATED

# The photometric each colour codestream row declares. PS3.5 8.2.4: the
# reversible transform is named YBR_RCT and the irreversible one YBR_ICT,
# and no other value is permitted when the transform is on.
COLOUR_ROW_PHOTOMETRIC = {
    "j2k_lossless_ybr_rct.dcm": "YBR_RCT",
    "j2k_lossy_ybr_ict.dcm": "YBR_ICT",
    "htj2k_lossless_ybr_rct.dcm": "YBR_RCT",
    "htj2k_lossy_ybr_ict.dcm": "YBR_ICT",
}

# F-244. PS3.5 A.4: "Decoders of encapsulated Pixel Data, whether
# Single-frame or Multi-frame, need to accept both an empty Basic Offset Table
# (zero length) and a Basic Offset Table filled with 32 bit offset values."
# Every row above carries a populated table, so these rows carry an empty one.
#
# Each single-frame row is its source row re-encapsulated by hand with a
# zero-length table Item and the source's Fragment byte for byte, so its
# decode digest is its source's.
EMPTY_BOT_SOURCES = {
    "rle_lossless_empty_bot.dcm": "rle_lossless.dcm",
    "jpeg_baseline_rgb8_empty_bot.dcm": "jpeg_baseline_rgb8.dcm",
    "jpeg_extended_12_empty_bot.dcm": "jpeg_extended_12.dcm",
    "jpeg_lossless_p14_empty_bot.dcm": "jpeg_lossless_p14.dcm",
    "jpeg_lossless_p14_sv1_empty_bot.dcm": "jpeg_lossless_p14_sv1.dcm",
    "jpegls_lossless_empty_bot.dcm": "jpegls_lossless.dcm",
    "jpegls_near_lossless_empty_bot.dcm": "jpegls_near_lossless.dcm",
    "j2k_lossless_empty_bot.dcm": "j2k_lossless.dcm",
    "j2k_lossy_empty_bot.dcm": "j2k_lossy.dcm",
    "htj2k_lossless_empty_bot.dcm": "htj2k_lossless.dcm",
    "htj2k_lossless_rpcl_empty_bot.dcm": "htj2k_lossless_rpcl.dcm",
    "htj2k_lossy_empty_bot.dcm": "htj2k_lossy.dcm",
    "jpegxl_lossless_empty_bot.dcm": "jpegxl_lossless.dcm",
    "jpegxl_lossy_empty_bot.dcm": "jpegxl_lossy.dcm",
}

# The shape rows. Each names the syntax row whose producer encodes it and the
# shape PS3.5 A.4 and 8.2 allow: one frame in three Fragments, three frames in
# one Fragment each, three frames in two Fragments each, and three frames with
# an Extended Offset Table, PS3.3 C.7.6.3.1.8. A multi-frame row encodes three
# distinct frames, so a swapped or misplaced boundary changes the decode.
EMPTY_BOT_SHAPES = {
    "rle_multiframe_empty_bot.dcm": ("rle_lossless.dcm", "multiframe"),
    "jpegls_lossless_fragmented_empty_bot.dcm":
        ("jpegls_lossless.dcm", "fragmented"),
    "jpeg_lossless_p14_sv1_multiframe_fragmented_empty_bot.dcm":
        ("jpeg_lossless_p14_sv1.dcm", "multiframe-fragmented"),
    "j2k_lossless_multiframe_fragmented_empty_bot.dcm":
        ("j2k_lossless.dcm", "multiframe-fragmented"),
    "j2k_lossless_multiframe_eot.dcm":
        ("j2k_lossless.dcm", "multiframe-extended-offset-table"),
}

EMPTY_BOT_CASES = {
    **EMPTY_BOT_SOURCES,
    **{name: source for name, (source, _) in EMPTY_BOT_SHAPES.items()},
}
MULTIFRAME_EMPTY_BOT = frozenset(
    name for name, (_, shape) in EMPTY_BOT_SHAPES.items()
    if shape.startswith("multiframe"))
MULTIFRAME_FRAMES = 3

for _name, _source in EMPTY_BOT_CASES.items():
    SYNTAX_CASES[_name] = SYNTAX_CASES[_source]
    SYNTAX_REFERENCE[_name] = SYNTAX_REFERENCE[_source]

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


def saturated_rgb_ramp(rows: int, cols: int) -> np.ndarray:
    """The RGB reference the colour codestream rows are encoded from.

    The smooth rgb_ramp underneath keeps lossy ringing small. Four marker
    pixels at the start of the first row put the component transform's edge
    cases where a fixture can name them by index: pure red is the reversible
    transform case whose chroma spans one bit more than the depth, pure
    green and pure blue pin the other two corners, and 200 100 50 is the
    F-239 plan's worked inverse RCT example.
    """
    pixels = rgb_ramp(rows, cols).copy()
    pixels[0, 0] = (255, 0, 0)
    pixels[0, 1] = (0, 255, 0)
    pixels[0, 2] = (0, 0, 255)
    pixels[0, 3] = (200, 100, 50)
    return pixels


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

    Current PS3.3 C.7.6.3.3 requires HighBit to equal BitsStored minus one,
    so HighBit 11 is the conforming case. The HighBit 15 case is retained and
    explicitly labelled as legacy nonconforming interoperability evidence. A
    reader that ignores HighBit gets one of the two right, and both look like
    plausible Hounsfield numbers.
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
    ds.PresentationLUTShape = "INVERSE"
    set_monochrome_pixels(
        ds, ramp(TRAP_ROWS, TRAP_COLS, 65535, np.uint16), 16, 16, 15, 0)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_sigmoid_width_half(out: Path) -> None:
    """A SIGMOID VOI whose legal width is below the LINEAR minimum.

    PS3.3 C.11.2.1.3.1 permits any positive width for SIGMOID. The stored
    ramp maps to modality values 37.75 through 42.5, crossing both shoulders
    of the window around centre 40 without relying on a real study.
    """
    case = QUIRK_CASES["case_sigmoid_width_half"]
    path = Path(case["paths"][0])
    name = path.stem
    ds = new_dataset(name, CT_STORAGE, "CT")
    ct_common(ds)
    ds.RescaleSlope = "0.25"
    ds.RescaleIntercept = "0"
    ds.WindowCenter = case["windowCentre"]
    ds.WindowWidth = case["windowWidth"]
    ds.VOILUTFunction = case["voiLutFunction"]
    pixels = np.tile(np.arange(151, 171, dtype=np.uint16), (TRAP_ROWS, 1))
    set_monochrome_pixels(ds, pixels, 16, 16, 15, 0)
    write(ds, out / path)


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
        # Decimal String has a signed spelling, while JavaScript Number JSON
        # serialization normalises -0 to 0. Emit the canonical zero spelling
        # so the two independent readers can agree exactly.
        position = ["0.000000" if distance * axis == 0
                    else f"{distance * axis:.6f}"
                    for axis in SERIES_NORMAL]

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


def palette_entry(index: int) -> tuple[int, int, int]:
    """Entry `i` is `(i, 255 - i, 3i mod 256)`, modulo 256 on the index.

    Three different functions of the index on purpose. Equal channels would
    satisfy a red-green-blue transposition and a monotone triple would satisfy
    an off-by-one in every channel at once, so neither could be caught.
    """
    low = index % 256
    return low, 255 - low, (low * 3) % 256


def case_palette_color(out: Path, name: str, entries: int, first_mapped: int,
                       bits_allocated: int) -> None:
    """PALETTE COLOR, PS3.3 C.7.9 and C.7.6.3.1.5.

    The corpus had no palette case at all before F-030, so neither the
    zero-means-65536 entry count nor the first-mapped-input offset had a file
    behind them.

    Two traps, one per generated case. `first_mapped` non-zero is the offset:
    stored values below it clamp to the first entry, and a reader that ignores
    (0028,1101) value two shifts the whole colour map. `entries` of 65536 is
    declared as 0, which is the only legal spelling because 65536 does not fit
    in a US, and a reader that takes 0 literally finds an empty table.

    **Bits per entry is 16 in both cases, deliberately.** PS3.3 C.7.6.3.1.6's
    packing for 8-bit LUT Data is a PS3.5 encoding question that belongs to the
    parser story rather than to this one, and guessing it here would put a
    guess into a manifest digest. The 8-bit descriptor path is proven instead
    by `crates/ocelli-pixel/tests/palette.rs`, which exercises the descriptor
    directly and does not depend on any wire packing.
    """
    ds = new_dataset(name, SC_STORAGE, "OT")
    ds.ConversionType = "WSD"
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = "PALETTE COLOR"
    ds.BitsAllocated = bits_allocated
    ds.BitsStored = bits_allocated
    ds.HighBit = bits_allocated - 1
    ds.PixelRepresentation = 0
    ds.Rows, ds.Columns = TRAP_ROWS, TRAP_COLS

    count = TRAP_ROWS * TRAP_COLS
    dtype = np.uint8 if bits_allocated == 8 else np.uint16
    # A ramp across the declared input range, starting at zero so the values
    # below `first_mapped` exercise the low clamp rather than being assumed.
    top = min(first_mapped + entries - 1, (1 << bits_allocated) - 1)
    indices = ramp(TRAP_ROWS, TRAP_COLS, top, dtype)
    ds.PixelData = indices.tobytes()

    # (0028,1101) to (0028,1103). Three values: number of entries, first
    # stored pixel value mapped, bits per entry. 65536 is declared as 0.
    descriptor = [0 if entries == 65_536 else entries, first_mapped, 16]
    ds.RedPaletteColorLookupTableDescriptor = list(descriptor)
    ds.GreenPaletteColorLookupTableDescriptor = list(descriptor)
    ds.BluePaletteColorLookupTableDescriptor = list(descriptor)

    table = np.array([palette_entry(index) for index in range(entries)],
                     dtype=np.uint16)
    ds.RedPaletteColorLookupTableData = table[:, 0].tobytes()
    ds.GreenPaletteColorLookupTableData = table[:, 1].tobytes()
    ds.BluePaletteColorLookupTableData = table[:, 2].tobytes()
    assert len(indices.tobytes()) == count * (bits_allocated // 8)
    write(ds, out / "synthetic" / f"{name}.dcm")


def case_jpegls_rgb8(out: Path) -> None:
    """Multi-component JPEG-LS, the row Appendix A gate A2 recorded as owed.

    `docs/spikes/A2-jpeg-ls.md` measured `Nf != 1` as "REFUSED by the crate,
    and NOT MEASURED here" and said the corpus could not close it, because no
    such row existed. A DICOM JPEG-LS frame can be RGB, so this is a coverage
    hole rather than a curiosity.

    The row exists to prove the refusal is clean against a real three-component
    codestream. `crates/ocelli-codec/src/jpegls.rs` refuses Samples per Pixel
    other than one before the dependency is reached, so nothing decodes this
    yet, and that is the point rather than a gap in the row.

    It reads the already-written RGB reference rather than building a second
    base, so the pair differ only in transfer syntax.
    """
    directory = out / "syntax"
    encode_with_pydicom(directory / REFERENCE_RGB8, "jpegls_lossless_rgb8.dcm",
                        "1.2.840.10008.1.2.4.80",
                        directory / "jpegls_lossless_rgb8.dcm")


def two_band(rows: int, cols: int, low: int, high: int) -> np.ndarray:
    """``s(r, c) = low if c < cols // 2 else high``, unsigned 16-bit."""
    image = np.empty((rows, cols), dtype=np.uint16)
    image[:, : cols // 2] = low
    image[:, cols // 2 :] = high
    return image


def case_jpegls_banded(out: Path) -> None:
    """A flat banded frame under `.80` and `.81`, F-243.

    ITU-T T.87 A.7.1 raises the run index only for a full run segment. In a
    flat region several runs build the index up, and a run that then reaches
    the end of a line part way through a segment is the case a decoder that
    raised the index anyway gets wrong, with no error. The ramp the other
    JPEG-LS rows carry never reaches it, so the corpus could not see that
    defect. This frame is the reported file's shape and values, two bands of
    224 and 1064 across 32 by 32, and its reference is written uncompressed
    here from the formula rather than by any codec.
    """
    directory = out / "syntax"
    ds = new_dataset("syntax_banded16", CT_STORAGE, "CT", study="syntax")
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ct_common(ds)
    ds.RescaleIntercept = "0"
    set_monochrome_pixels(
        ds, two_band(BANDED_ROWS, BANDED_COLS, BANDED_LOW, BANDED_HIGH),
        16, 16, 15, 0)
    normalise(ds, REFERENCE_BANDED16)
    write(ds, directory / REFERENCE_BANDED16)
    reference = directory / REFERENCE_BANDED16
    encode_with_pydicom(reference, "jpegls_banded_lossless.dcm",
                        "1.2.840.10008.1.2.4.80",
                        directory / "jpegls_banded_lossless.dcm")
    encode_with_pydicom(reference, "jpegls_banded_near_lossless.dcm",
                        "1.2.840.10008.1.2.4.81",
                        directory / "jpegls_banded_near_lossless.dcm",
                        jls_error=JPEG_LS_NEAR)


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


def case_rgb_icc(out: Path) -> None:
    """An 8-bit RGB frame carrying a synthetic ICC Profile (0028,2000).

    F-239's AC26 names a file with the element as drawn with the profile not
    applied. The bytes here are written by this script and are deliberately
    not an ICC profile: no third-party profile is copied. F-240 names this
    marker malformed and retains the device RGB values. It remains the
    presence-only case F-239 introduced.
    """
    name = "sc_rgb_icc"
    ds = new_dataset(name, SC_STORAGE, "OT")
    ds.ConversionType = "WSD"
    ds.PixelSpacing = list(NON_SQUARE_SPACING)
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "RGB"
    ds.PlanarConfiguration = 0
    ds.BitsAllocated = 8
    ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.Rows, ds.Columns = TRAP_ROWS, TRAP_COLS
    ds.PixelData = rgb_ramp(TRAP_ROWS, TRAP_COLS).tobytes()
    ds.ICCProfile = b"OCELLI synthetic corpus, not an ICC profile"
    write(ds, out / "synthetic" / f"{name}.dcm")


def synthetic_icc_profile(kind: str) -> bytes:
    """F-240 profiles, ICC.1:2022 7.2, 7.3, 8.2, 10.6, 10.18 and 10.31.

    https://www.color.org/specification/ICC.1-2022-05.pdf
    Colourants are the same published D50 words the independent AC fixtures
    derive. The synthetic device is defined at D50, so no adaptation is needed.
    No profile or implementation from a third-party colour library is copied.
    """
    from hashlib import md5
    from itertools import product
    from struct import pack

    kinds = {"gamma1", "gamma22", "srgb_para", "swap", "lut"}
    if kind not in kinds:
        raise ValueError("unknown synthetic ICC case")
    colourants = [(0x6FA0, 0x38F2, 0x038F),
                  (0x6296, 0xB78A, 0x18DA),
                  (0x24A0, 0x0F85, 0xB6C4)]
    if kind == "swap":
        colourants[0], colourants[1] = colourants[1], colourants[0]
    xyz = lambda words: b"XYZ \0\0\0\0" + pack(">3I", *words)
    if kind == "srgb_para":
        # Type 3: g,a,b,c,d as signed 15.16 numbers, rounded once.
        params = [2.4, 1 / 1.055, 0.055 / 1.055, 1 / 12.92, 0.04045]
        curve = b"para\0\0\0\0" + pack(">HH5i", 3, 0,
                       *(round(value * 65536) for value in params))
    else:
        gamma = 0x0233 if kind == "gamma22" else 0x0100
        curve = b"curv\0\0\0\0" + pack(">IH", 1, gamma)

    def mluc(text: str) -> bytes:
        data = text.encode("utf-16-be")
        return (b"mluc\0\0\0\0" + pack(">II", 1, 12) +
                b"enUS" + pack(">II", len(data), 28) + data)

    tags = [(code, xyz(words)) for code, words in
            zip((b"rXYZ", b"gXYZ", b"bXYZ"), colourants)]
    tags += [(code, curve) for code in (b"rTRC", b"gTRC", b"bTRC")]
    tags += [(b"wtpt", xyz((0xF6D6, 0x10000, 0xD32D))),
             (b"desc", mluc("Ocelli synthetic " + kind)),
             (b"cprt", mluc("Ocelli synthetic profile, MIT OR Apache-2.0"))]
    if kind == "lut":
        # A valid three-channel lut16Type, 10.10, with two-point tables.
        # Its presence must take precedence over the matrix, 8.10. The
        # viewer deliberately does not apply it, so no inverse LUT is needed
        # for this input-class profile, 8.3.2.
        matrix = [65536 if row == col else 0
                  for row in range(3) for col in range(3)]
        table = pack(">2H", 0, 65535) * 3
        grid = b"".join(pack(">3H", *(value * 32768 for value in rgb))
                        for rgb in product(range(2), repeat=3))
        lut = (b"mft2\0\0\0\0" + bytes((3, 3, 2, 0)) +
               pack(">9i2H", *matrix, 2, 2) + table + grid + table)
        tags.append((b"A2B0", lut))

    header = bytearray(128)
    header[8:12] = bytes((4, 0x40, 0, 0))  # ICC v4.4.
    header[12:16] = b"scnr" if kind == "lut" else b"mntr"
    header[16:24] = b"RGB XYZ "
    header[24:36] = pack(">6H", 2026, 11, 8, 0, 0, 0)
    header[36:40] = b"acsp"
    header[44:48] = pack(">I", 1)  # Embedded in the DICOM object.
    header[64:68] = pack(">I", 1)  # Media-relative colorimetric.
    header[68:80] = pack(">3I", 0xF6D6, 0x10000, 0xD32D)
    entries = bytearray(pack(">I", len(tags)))
    body = bytearray()
    start = 132 + 12 * len(tags)
    for signature, payload in tags:
        entries.extend(signature + pack(">II", start + len(body), len(payload)))
        body.extend(payload)
        body.extend(bytes((-len(body)) % 4))
    profile = header + entries + body
    profile[0:4] = pack(">I", len(profile))
    # 7.2.18, profile ID hashes the profile with flags, intent and ID zero.
    hashed = bytearray(profile)
    hashed[44:48] = bytes(4)
    hashed[64:68] = bytes(4)
    profile[84:100] = md5(hashed, usedforsecurity=False).digest()
    return bytes(profile)


def case_rgb_icc_matrix(out: Path, kind: str) -> None:
    """F-240, native RGB with our reproducible matrix/TRC or lookup profile."""
    name = "sc_rgb_icc_" + kind
    ds = new_dataset(name, SC_STORAGE, "OT")
    ds.ConversionType = "WSD"
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "RGB"
    ds.PlanarConfiguration = 0
    ds.BitsAllocated = ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    if kind == "swap":
        values = [[255, 0, 0], [0, 255, 0]]
    elif kind == "lut":
        values = [[10, 20, 30], [200, 100, 50]]
    else:
        values = [[value] * 3 for value in (64, 128, 192, 255)]
    ds.Rows, ds.Columns = 1, len(values)
    ds.PixelData = np.asarray(values, dtype=np.uint8).tobytes()
    ds.ICCProfile = synthetic_icc_profile(kind)
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
    ds = new_dataset(label, CT_STORAGE if kind != "rgb8" and kind != "rgb8_saturated"
                     else SC_STORAGE,
                     "CT" if kind != "rgb8" and kind != "rgb8_saturated" else "OT",
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
    elif kind in ("rgb8", "rgb8_saturated"):
        ds.ConversionType = "WSD"
        ds.SamplesPerPixel = 3
        ds.PhotometricInterpretation = "RGB"
        ds.PlanarConfiguration = 0
        ds.BitsAllocated = 8
        ds.BitsStored = 8
        ds.HighBit = 7
        ds.PixelRepresentation = 0
        ds.Rows, ds.Columns = SYNTAX_ROWS, SYNTAX_COLS
        if kind == "rgb8":
            ds.PixelData = rgb_ramp(SYNTAX_ROWS, SYNTAX_COLS).tobytes()
        else:
            ds.PixelData = saturated_rgb_ramp(SYNTAX_ROWS, SYNTAX_COLS).tobytes()
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


def encapsulate_codestream(ds: FileDataset, syntax: str, codestream: bytes) -> None:
    """Wrap one codestream as this file's Pixel Data, PS3.5 A.4.

    What goes inside the pixel-data item is the codestream, not a JP2 file.
    The uncompressed base carries OW because its 16-bit samples are words,
    and inheriting that here produces a file DCMTK warns about and a strict
    reader may refuse, with nothing about the pixels looking wrong.
    """
    ds.file_meta.TransferSyntaxUID = UID(syntax)
    ds.PixelData = encapsulate([codestream])
    ds["PixelData"].VR = "OB"
    ds["PixelData"].is_undefined_length = True


def ppm_bytes(pixels: np.ndarray) -> bytes:
    """One P6 PPM frame, the colour input opj_compress and ojph_compress take."""
    rows, cols = pixels.shape[0], pixels.shape[1]
    return f"P6\n{cols} {rows}\n255\n".encode("ascii") + np.ascontiguousarray(
        pixels).tobytes()


def encode_with_opj(base: Path, label: str, syntax: str, target: Path,
                    arguments: list[str]) -> None:
    """The OpenJPEG CLI supplies the colour JPEG 2000 Part 1 codestreams.

    pydicom's pylibjpeg-openjpeg encoder leaves the multiple component
    transform off even for a three-sample frame (measured: its use_mct flag
    changes no byte), and PS3.5 8.2.4 makes YBR_RCT and YBR_ICT name that
    transform. opj_compress applies RGB to YCC for three components by
    default, so these two rows are its output, encapsulated here. The
    encoder version it writes into a COM marker is the one BUILT_WITH and
    --tool-versions record.
    """
    ds = pydicom.dcmread(str(base))
    pixels = ds.pixel_array

    with tempfile.TemporaryDirectory(prefix="ocelli-opj-") as scratch:
        source = Path(scratch) / "frame.ppm"
        produced = Path(scratch) / "frame.j2k"
        with source.open("wb") as handle:
            handle.write(ppm_bytes(pixels))
        run(["opj_compress", "-i", str(source), "-o", str(produced), *arguments])
        codestream = produced.read_bytes()

    ds.PhotometricInterpretation = COLOUR_ROW_PHOTOMETRIC[
        target.name]
    encapsulate_codestream(ds, syntax, codestream)
    normalise(ds, label)
    if syntax == "1.2.840.10008.1.2.4.91":
        mark_lossy(ds, "ISO_15444_1")
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


def encode_with_ojph(base: Path, label: str, syntax: str, target: Path,
                     arguments: list[str], colour_transform: bool = False) -> None:
    """OpenJPH supplies the HTJ2K codestream, which is then encapsulated here.

    PS3.5 A.4.10 and A.4.11: what goes inside the pixel-data item is the
    codestream, not a JP2 file, which is exactly what ojph_compress writes to
    a .j2c.

    The samples come out of the reference file whose header this case copies,
    so the pixels encoded and the pixels claimed have one source. Recomputing
    them alongside would be two, and the two would be free to drift. A colour
    row is written as PPM with the codestream's own component transform on,
    because PS3.5 8.2.4 makes YBR_RCT and YBR_ICT name it.
    """
    ds = pydicom.dcmread(str(base))
    pixels = ds.pixel_array

    with tempfile.TemporaryDirectory(prefix="ocelli-ojph-") as scratch:
        if colour_transform:
            source = Path(scratch) / "frame.ppm"
            with source.open("wb") as handle:
                handle.write(ppm_bytes(pixels))
        else:
            source = Path(scratch) / "frame.pgm"
            rows, cols = pixels.shape
            with source.open("wb") as handle:
                handle.write(f"P5\n{cols} {rows}\n65535\n".encode("ascii"))
                handle.write(pixels.astype(">u2").tobytes())
        produced = Path(scratch) / "frame.j2c"
        run(["ojph_compress", "-i", str(source), "-o", str(produced), *arguments])
        codestream = produced.read_bytes()

    if colour_transform:
        ds.PhotometricInterpretation = COLOUR_ROW_PHOTOMETRIC[
            target.name]
    encapsulate_codestream(ds, syntax, codestream)
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


def jpegxl_encode_frame(pixels: np.ndarray, *, lossless: bool, bits: int,
                        photometric: str | None) -> bytes:
    """One JPEG XL codestream through imagecodecs and libjxl.

    `usecontainer` is false. imagecodecs 2026.10.10 still writes an ISO
    BMFF container for a 16-bit lossless frame and for `jpegxl_encode_jpeg`.
    The other encodes are bare codestreams starting FF 0A. The lossy-method
    classifier accepts both framings.
    """
    import imagecodecs
    options: dict[str, object] = {
        "lossless": lossless,
        "bitspersample": bits,
        "effort": JPEGXL_EFFORT,
        "usecontainer": False,
    }
    if photometric is not None:
        options["photometric"] = photometric
    if not lossless:
        options["distance"] = JPEGXL_DISTANCE
    encoded = imagecodecs.jpegxl_encode(np.ascontiguousarray(pixels), **options)
    if not isinstance(encoded, (bytes, bytearray)):
        raise RuntimeError("jpegxl_encode did not return bytes")
    return bytes(encoded)


def write_jpegxl(ds: Dataset, label: str, syntax: str, codestream: bytes,
                 target: Path, lossy_method: str | None) -> None:
    """Encapsulate one codestream. PS3.5 A.4: one fragment, OB, undefined length."""
    ds.file_meta.TransferSyntaxUID = UID(syntax)
    ds.PixelData = encapsulate([codestream])
    ds["PixelData"].VR = "OB"
    ds["PixelData"].is_undefined_length = True
    normalise(ds, label)
    if lossy_method is not None:
        # PS3.3 C.7.6.1.1.5.1 Defined Term for JPEG XL lossy compression,
        # ISO/IEC 18181-1, added by Supplement 232.
        mark_lossy(ds, lossy_method)
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


def encode_jpegxl_cases(directory: Path) -> None:
    """The five JPEG XL rows. PS3.5 A.4.12.

    The pixel sources are the uncompressed syntax bases already in
    `directory`. `--case jpegxl` therefore leaves every other syntax file
    untouched. A full `generate()` writes those bases first and then calls
    this.

    `.111` takes its header from the RGB reference and its fragment from
    `jpegxl_encode_jpeg` of the `.50` JPEG. It is not a copy of the DCMTK
    dataset. That file carries DerivationDescription, and this producer
    must not. `.111` is also absent from `LOSSY_TRANSFER_SYNTAXES`, so the
    Lossy Image Compression attributes stay absent. The imagecodecs
    round trip below only checks that the fixture is a real recompression.
    Ocelli refuses the row as unavailable.

    imagecodecs accepts photometric RGB and GRAY. Its XYB colour-space
    value produced the same bytes as RGB on these ramps, so the XYB row is
    an RGB codestream whose DICOM Photometric Interpretation is XYB.
    """
    import imagecodecs
    from pydicom.encaps import generate_frames

    directory = Path(directory)
    directory.mkdir(parents=True, exist_ok=True)

    def load(name: str) -> FileDataset:
        return pydicom.dcmread(str(directory / name))

    mono16 = load(REFERENCE_MONO16)
    write_jpegxl(
        mono16, "jpegxl_lossless.dcm", JPEGXL_LOSSLESS,
        jpegxl_encode_frame(mono16.pixel_array, lossless=True, bits=16,
                            photometric="GRAY"),
        directory / "jpegxl_lossless.dcm", None)

    rgb = load(REFERENCE_RGB8)
    write_jpegxl(
        rgb, "jpegxl_lossless_rgb8.dcm", JPEGXL_LOSSLESS,
        jpegxl_encode_frame(rgb.pixel_array, lossless=True, bits=8,
                            photometric="RGB"),
        directory / "jpegxl_lossless_rgb8.dcm", None)

    mono12 = load(REFERENCE_MONO12)
    write_jpegxl(
        mono12, "jpegxl_lossy.dcm", JPEGXL_GENERAL,
        jpegxl_encode_frame(mono12.pixel_array, lossless=False, bits=12,
                            photometric="GRAY"),
        directory / "jpegxl_lossy.dcm", "ISO_18181_1")

    xyb = load(REFERENCE_RGB8)
    # Read the samples before the tag changes. pydicom refuses to decode a
    # frame whose Photometric Interpretation it does not know, and XYB is
    # one of those.
    xyb_pixels = xyb.pixel_array
    xyb.PhotometricInterpretation = "XYB"
    write_jpegxl(
        xyb, "jpegxl_xyb_rgb8.dcm", JPEGXL_GENERAL,
        jpegxl_encode_frame(xyb_pixels, lossless=False, bits=8,
                            photometric="RGB"),
        directory / "jpegxl_xyb_rgb8.dcm", "ISO_18181_1")

    baseline = load("jpeg_baseline_rgb8.dcm")
    fragment = bytes(next(generate_frames(baseline.PixelData, number_of_frames=1)))
    recompressed = imagecodecs.jpegxl_encode_jpeg(fragment)
    if not isinstance(recompressed, (bytes, bytearray)):
        raise RuntimeError("jpegxl_encode_jpeg did not return bytes")
    restored = imagecodecs.jpegxl_decode_jpeg(bytes(recompressed))
    if bytes(restored) != fragment:
        raise RuntimeError("JPEG XL recompression did not restore the JPEG fragment")
    header = load(REFERENCE_RGB8)
    header.PhotometricInterpretation = "YBR_FULL_422"
    for name in ("LossyImageCompression", "LossyImageCompressionRatio",
                 "LossyImageCompressionMethod"):
        if name in header:
            del header[name]
    write_jpegxl(
        header, "jpegxl_recompression_rgb8.dcm", JPEGXL_RECOMPRESSION,
        bytes(recompressed), directory / "jpegxl_recompression_rgb8.dcm", None)


# The toolchain corpus/manifest.tsv was generated with, and the only record of
# it. Every entry can move a digest, and the ones that leave no version in the
# file cannot be recovered from the corpus at all, which is why they are here.
# `--tool-versions` asks each tool rather than reading this table. The
# opj_compress entry is the OpenJPEG CLI that encodes the F-239 colour rows,
# a different binary from the library inside pylibjpeg-openjpeg, and its
# version is the COM marker's.
BUILT_WITH = {
    "pydicom": "3.0.2",
    "numpy": "2.5.2",
    "pylibjpeg-openjpeg": "2.5.0",
    "OpenJPEG (the library inside it)": "2.5.2",
    "OpenJPEG (opj_compress CLI)": "2.5.4",
    "pyjpegls": "1.5.1",
    "DCMTK": "3.7.0",
    "OpenJPH": "0.31.0",
    "imagecodecs": "2026.10.10",
    "libjxl (the library inside imagecodecs)": "0.12.0",
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
    for package in ("pydicom", "numpy", "pylibjpeg-openjpeg", "pyjpegls",
                    "imagecodecs"):
        try:
            found[package] = metadata.version(package)
        except metadata.PackageNotFoundError:
            found[package] = "absent"
    try:
        import imagecodecs
        reported = imagecodecs.jpegxl_version().split()
        found["libjxl (the library inside imagecodecs)"] = (
            reported[-1] if reported else "unknown")
    except (ImportError, AttributeError, IndexError):
        found["libjxl (the library inside imagecodecs)"] = "absent"
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
    if shutil.which("opj_compress") is None:
        found["OpenJPEG (opj_compress CLI)"] = "absent"
    else:
        # opj_compress has no version flag. Its usage banner names the
        # openjp2 it was compiled against, which is the version that writes
        # the COM marker and moves the digest.
        banner_run = subprocess.run(["opj_compress", "-h"],
                                    capture_output=True, text=True)
        banner = banner_run.stdout + banner_run.stderr
        opj = re.search(r"openjp2 library v([0-9]+\.[0-9]+\.[0-9]+)", banner)
        found["OpenJPEG (opj_compress CLI)"] = opj.group(1) if opj else "unknown"
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
    # against, which is deliberate: one file, one digest, one meaning. The
    # saturated reference is written by `generate_colour_syntax_layer` below:
    # the F-239 colour codestream rows are built from it, so their component
    # transform's edge cases are in the decoded bytes.
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
    encode_jpegxl_cases(directory)

    # The four F-239 colour rows, from the saturated RGB reference. The two
    # lossless rows carry the reversible transform (YBR_RCT) and the two
    # lossy rows the irreversible one (YBR_ICT), PS3.5 8.2.4. The .201 row is
    # LRCP like its monochrome sibling and the .203 row RPCL like its own.
    generate_colour_syntax_layer(out)


def generate_colour_syntax_layer(out: Path) -> None:
    """Write the saturated RGB reference and the four colour codestream rows.

    Its own `--case` selector, so adding the F-239 rows does not re-encode
    the rows an earlier toolchain produced: a full `syntax` run would move
    every externally encoded digest beside this story's diff.
    """
    directory = out / "syntax"
    directory.mkdir(parents=True, exist_ok=True)
    rgb8_saturated = syntax_base("syntax_rgb8_saturated", "rgb8_saturated")
    write(rgb8_saturated, directory / REFERENCE_RGB8_SATURATED)
    saturated_reference = directory / REFERENCE_RGB8_SATURATED

    encode_with_opj(saturated_reference, "j2k_lossless_ybr_rct.dcm",
                    "1.2.840.10008.1.2.4.90",
                    directory / "j2k_lossless_ybr_rct.dcm", [])
    encode_with_opj(saturated_reference, "j2k_lossy_ybr_ict.dcm",
                    "1.2.840.10008.1.2.4.91",
                    directory / "j2k_lossy_ybr_ict.dcm",
                    ["-I", "-r", str(int(J2K_COMPRESSION_RATIO))])
    encode_with_ojph(saturated_reference, "htj2k_lossless_ybr_rct.dcm",
                     "1.2.840.10008.1.2.4.201",
                     directory / "htj2k_lossless_ybr_rct.dcm",
                     ["-reversible", "true", "-prog_order", "LRCP",
                      "-colour_trans", "true"], colour_transform=True)
    encode_with_ojph(saturated_reference, "htj2k_lossy_ybr_ict.dcm",
                     "1.2.840.10008.1.2.4.203",
                     directory / "htj2k_lossy_ybr_ict.dcm",
                     ["-reversible", "false", "-qstep", str(HTJ2K_QSTEP),
                      "-prog_order", "RPCL", "-colour_trans", "true"],
                     colour_transform=True)


# ---------------------------------------------------------------------------
# Layer 1c: F-244, encapsulated Pixel Data with an empty Basic Offset Table
# ---------------------------------------------------------------------------

# PS3.3 A.38, Enhanced CT Image Storage. Its Multi-frame Functional Groups
# Module, C.7.6.16, carries Number of Frames (0028,0008) as Type 1, which the
# CT Image Storage IOD of the source rows has no module for.
MULTIFRAME_SOP_CLASS = ENHANCED_CT_STORAGE


def pixel_items(value: bytes) -> list[bytes]:
    """The Item values of encapsulated Pixel Data, PS3.5 A.4, table first.

    pydicom keeps the value from the first Item Tag up to the Sequence
    Delimitation Item, which it drops. Every Item is Tag (FFFE,E000) and a
    4-byte little-endian length, then that many bytes.
    """
    items, at = [], 0
    while at < len(value):
        group, element, length = struct.unpack_from("<HHI", value, at)
        if (group, element) == (0xFFFE, 0xE0DD):
            break
        if (group, element) != (0xFFFE, 0xE000):
            raise RuntimeError("encapsulated Pixel Data holds a non-Item tag")
        at += 8
        items.append(bytes(value[at:at + length]))
        at += length
    return items


def item(value: bytes) -> bytes:
    """One Item. PS3.5 A.4: a Fragment is an even number of bytes."""
    if len(value) % 2:
        raise RuntimeError("an Item value must have even length")
    return struct.pack("<HHI", 0xFFFE, 0xE000, len(value)) + value


def even(codestream: bytes) -> bytes:
    """Pad an odd frame with one zero byte, as A.4 allows for its last Fragment."""
    return codestream + b"\x00" if len(codestream) % 2 else codestream


def empty_table(fragments: list[bytes]) -> bytes:
    """Table A.4-1's layout: a zero-length table Item, then the Fragments."""
    return item(b"") + b"".join(item(fragment) for fragment in fragments)


# The end-of-frame marker PS3.5 A.4's frame-boundary readers look for:
# ISO/IEC 10918-1 EOI and ISO/IEC 15444-1 EOC are both FF D9. pydicom 3.0.2
# searches the last ten bytes of a Fragment for it.
END_MARKER = b"\xff\xd9"
MARKER_WINDOW = 10


def split_offset(frame: bytes) -> int:
    """Where a frame is cut into two Fragments.

    The first Fragment's length is 2 mod 4, the lengths dicom-object 0.10.0
    refused, and it is the first such length at or after the middle whose last
    ten bytes hold no end marker, so no reader can take it for a whole frame.
    """
    for offset in range(len(frame) // 2, len(frame) - 2):
        if offset % 4 == 2 and END_MARKER not in frame[offset - MARKER_WINDOW:offset]:
            return offset
    raise RuntimeError("no Fragment split avoids an end marker")


def fragment_split(frame: bytes, parts: int) -> list[bytes]:
    """Cut one frame into `parts` Fragments, the first of each 2 mod 4."""
    if parts == 2:
        cut = split_offset(frame)
        return [frame[:cut], even(frame[cut:])]
    if parts == 3:
        first = split_offset(frame[:len(frame) // 2 + 2])
        second = split_offset(frame[first:]) + first
        return [frame[:first], frame[first:second], even(frame[second:])]
    raise ValueError(f"unsupported Fragment count {parts}")


def write_encapsulated(ds: Dataset, label: str, value: bytes,
                       target: Path) -> None:
    """Write `value` as OB undefined-length Pixel Data under a new identity."""
    ds.PixelData = value
    ds["PixelData"].VR = "OB"
    ds["PixelData"].is_undefined_length = True
    normalise(ds, label)
    target.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(str(target), enforce_file_format=True)


def multiframe_reference(reference: np.ndarray) -> np.ndarray:
    """Three distinct frames: the reference, columns reversed, rows reversed."""
    return np.stack([reference, reference[:, ::-1], reference[::-1, :]])


def multiframe_base(directory: Path, label: str) -> FileDataset:
    """The uncompressed three-frame Enhanced CT every multi-frame row encodes.

    The mono16 syntax reference supplies the pixel module and the samples.
    Geometry and the pixel value transformation move into functional groups,
    PS3.3 C.7.6.16, because the Enhanced CT IOD places them there.
    """
    reference = pydicom.dcmread(str(directory / REFERENCE_MONO16))
    ds = new_dataset(label, MULTIFRAME_SOP_CLASS, "CT", study="syntax")
    ds.NumberOfFrames = str(MULTIFRAME_FRAMES)
    ds.ImageType = ["DERIVED", "PRIMARY", "VOLUME", "NONE"]
    ds.ContentQualification = "RESEARCH"
    set_monochrome_pixels(
        ds, reference.pixel_array, 16, 16, 15, 0)
    ds.PixelData = multiframe_reference(reference.pixel_array) \
        .astype("<u2").tobytes()

    dimension = Dataset()
    dimension.DimensionOrganizationUID = UID(fixed_uid("dimorg:" + label))
    ds.DimensionOrganizationSequence = [dimension]
    index_item = Dataset()
    index_item.DimensionOrganizationUID = dimension.DimensionOrganizationUID
    index_item.DimensionIndexPointer = pydicom.tag.Tag(0x0020, 0x9057)
    index_item.FunctionalGroupPointer = pydicom.tag.Tag(0x0020, 0x9111)
    ds.DimensionIndexSequence = [index_item]

    measures = Dataset()
    measures.PixelSpacing = list(NON_SQUARE_SPACING)
    measures.SliceThickness = "1.0"
    orientation = Dataset()
    orientation.ImageOrientationPatient = [
        "1.0", "0.0", "0.0", "0.0", "1.0", "0.0"]
    transformation = Dataset()
    transformation.RescaleSlope = "1"
    transformation.RescaleIntercept = "0"
    transformation.RescaleType = "HU"
    voi = Dataset()
    voi.WindowCenter = "40"
    voi.WindowWidth = "400"
    voi.VOILUTFunction = "LINEAR"
    shared = Dataset()
    shared.PixelMeasuresSequence = [measures]
    shared.PlaneOrientationSequence = [orientation]
    shared.PixelValueTransformationSequence = [transformation]
    shared.FrameVOILUTSequence = [voi]
    ds.SharedFunctionalGroupsSequence = [shared]

    per_frame = []
    for index in range(MULTIFRAME_FRAMES):
        position = Dataset()
        position.ImagePositionPatient = ["0.0", "0.0", f"{index:.1f}"]
        content = Dataset()
        content.StackID = "1"
        content.InStackPositionNumber = index + 1
        content.DimensionIndexValues = [index + 1]
        frame = Dataset()
        frame.PlanePositionSequence = [position]
        frame.FrameContentSequence = [content]
        per_frame.append(frame)
    ds.PerFrameFunctionalGroupsSequence = per_frame
    return ds


def encode_multiframe(directory: Path, source: str, label: str) -> tuple[
        FileDataset, list[bytes]]:
    """Encode the three-frame base with `source`'s producer, frame by frame.

    Returns the encoded dataset and its three frame codestreams, each
    unpadded where the producer left an odd length.
    """
    from pydicom.encaps import generate_frames

    syntax = SYNTAX_CASES[source]
    base = multiframe_base(directory, label)
    with tempfile.TemporaryDirectory(prefix="ocelli-f244-") as scratch:
        plain = Path(scratch) / "base.dcm"
        write(base, plain)
        if syntax == "1.2.840.10008.1.2.4.70":
            produced = Path(scratch) / "out.dcm"
            run(["dcmcjpeg", "--encode-lossless-sv1", "--uid-never",
                 str(plain), str(produced)])
            ds = pydicom.dcmread(str(produced))
        else:
            ds = pydicom.dcmread(str(plain))
            ds.compress(UID(syntax))
    frames = [bytes(frame) for frame in generate_frames(
        ds.PixelData, number_of_frames=MULTIFRAME_FRAMES)]
    if len(frames) != MULTIFRAME_FRAMES:
        raise RuntimeError(f"{label}: the producer wrote {len(frames)} frames")
    return ds, frames


def encode_empty_bot_cases(directory: Path) -> None:
    """The F-244 rows. Reads the syntax rows already in `directory`.

    `--case empty_bot` therefore leaves every other file untouched, as
    `jpegxl` does. Only the multi-frame rows run a producer, the one their
    source row names in EXTERNAL_ENCODERS or pydicom's own RLE encoder.
    """
    from pydicom.encaps import encapsulate_extended

    directory = Path(directory)
    for name, source in EMPTY_BOT_SOURCES.items():
        ds = pydicom.dcmread(str(directory / source))
        fragments = pixel_items(ds.PixelData)[1:]
        if len(fragments) != 1:
            raise RuntimeError(f"{source} does not hold one Fragment")
        write_encapsulated(ds, name, empty_table(fragments), directory / name)

    for name, (source, shape) in EMPTY_BOT_SHAPES.items():
        if shape == "fragmented":
            ds = pydicom.dcmread(str(directory / source))
            [fragment] = pixel_items(ds.PixelData)[1:]
            write_encapsulated(ds, name, empty_table(
                fragment_split(fragment, 3)), directory / name)
            continue
        ds, frames = encode_multiframe(directory, source, name)
        if shape == "multiframe":
            value = empty_table([even(frame) for frame in frames])
        elif shape == "multiframe-fragmented":
            value = empty_table([part for frame in frames
                                 for part in fragment_split(even(frame), 2)])
        elif shape == "multiframe-extended-offset-table":
            value, offsets, lengths = encapsulate_extended(frames)
            if pixel_items(value)[0]:
                raise RuntimeError("encapsulate_extended wrote a populated table")
            ds.ExtendedOffsetTable = offsets
            ds.ExtendedOffsetTableLengths = lengths
        else:
            raise RuntimeError(f"unknown shape {shape}")
        write_encapsulated(ds, name, value, directory / name)


def case_callables(out: Path) -> dict[str, Callable[[], None]]:
    """Every individually generatable case, in generation order.

    One list, read by both `generate()` and `--case`, so a case cannot exist in
    the full run and be missing from the selector.

    `syntax` is one entry rather than one per syntax because its cases share
    three bases that are written by the same function. `jpegls_rgb8` is
    separate and ordered after it, because it re-reads the RGB reference that
    entry writes. `jpegxl` re-reads those bases too, and it is its own entry
    so one story can add the five JPEG XL files without re-encoding the rest.
    """
    return {
        "ct_signed_12in16_right":
            lambda: case_signed_12in16(out, 11, "ct_signed_12in16_right"),
        "ct_signed_12in16_left":
            lambda: case_signed_12in16(out, 15, "ct_signed_12in16_left"),
        "ct_unsigned_16": lambda: case_unsigned_16(out),
        "ct_sigmoid_width_half": lambda: case_sigmoid_width_half(out),
        "cr_monochrome1": lambda: case_monochrome1(out),
        "mr_nonsquare_spacing": lambda: case_nonsquare_spacing(out),
        "ct_series_uniform":
            lambda: case_series(out, "ct_series_uniform", nonuniform=False),
        "ct_series_nonuniform":
            lambda: case_series(out, "ct_series_nonuniform", nonuniform=True),
        "sc_rgb_interleaved": lambda: case_rgb(out, 0, "sc_rgb_interleaved"),
        "sc_rgb_planar": lambda: case_rgb(out, 1, "sc_rgb_planar"),
        "sc_rgb_icc": lambda: case_rgb_icc(out),
        "sc_rgb_icc_gamma1": lambda: case_rgb_icc_matrix(out, "gamma1"),
        "sc_rgb_icc_gamma22": lambda: case_rgb_icc_matrix(out, "gamma22"),
        "sc_rgb_icc_srgb_para": lambda: case_rgb_icc_matrix(out, "srgb_para"),
        "sc_rgb_icc_swap": lambda: case_rgb_icc_matrix(out, "swap"),
        "sc_rgb_icc_lut": lambda: case_rgb_icc_matrix(out, "lut"),
        "us_ybr_full_422": lambda: case_ybr_full_422(out),
        "ct_multiframe_perframe": lambda: case_multiframe(out),
        "sc_palette_color":
            lambda: case_palette_color(out, "sc_palette_color", 256, 10, 8),
        "sc_palette_color_16":
            lambda: case_palette_color(out, "sc_palette_color_16", 65_536, 0,
                                       16),
        "syntax": lambda: generate_syntax_layer(out),
        "colour_syntax": lambda: generate_colour_syntax_layer(out),
        "jpegls_rgb8": lambda: case_jpegls_rgb8(out),
        "jpegls_banded": lambda: case_jpegls_banded(out),
        "jpegxl": lambda: encode_jpegxl_cases(out / "syntax"),
        "empty_bot": lambda: encode_empty_bot_cases(out / "syntax"),
    }


def require_encoders(tools: tuple[str, ...] =
                     ("dcmcjpeg", "ojph_compress", "opj_compress")) -> None:
    for tool in tools:
        if shutil.which(tool) is None:
            raise RuntimeError(
                f"{tool} is not on PATH. The synthetic layer needs DCMTK, "
                f"OpenJPH and the OpenJPEG CLI, see corpus/README.md.")


def generate(out: Path) -> Path:
    """Write the whole synthetic layer under `out`, replacing what is there."""
    require_encoders()
    out = Path(out)
    for subdirectory in ("synthetic", "syntax"):
        shutil.rmtree(out / subdirectory, ignore_errors=True)
    for build in case_callables(out).values():
        build()
    return out


def generate_one(out: Path, case: str) -> Path:
    """Write ONE case, leaving every other generated file untouched.

    This exists because `generate()` removes and rebuilds the whole synthetic
    and syntax layer, which re-encodes every externally encoded row with
    whatever version of that encoder is installed today. Adding a case that way
    lands a toolchain bump inside an unrelated story's diff and churns digests
    nobody in that story chose to move.

    `--tool-versions` is what makes the drift visible. This is what makes it
    avoidable.
    """
    out = Path(out)
    cases = case_callables(out)
    if case not in cases:
        raise RuntimeError(
            f"unknown case {case!r}. Known: {', '.join(sorted(cases))}")
    # The `syntax` entry shells out to DCMTK and OpenJPH and the
    # `colour_syntax` entry to the OpenJPEG CLI and OpenJPH. `jpegls_rgb8`
    # encodes through pydicom's pyjpegls plugin, and every other case is
    # written by pydicom directly. Each entry demands only the encoders it
    # shells out to.
    if case in ("syntax", "empty_bot"):
        require_encoders()
    elif case == "colour_syntax":
        require_encoders(("ojph_compress", "opj_compress"))
    cases[case]()
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
        ("CT", "synthetic, mono16, signed-12in16, high-bit-15, "
               "legacy-nonconforming"),
    "synthetic/ct_unsigned_16.dcm": ("CT", "synthetic, mono16, unsigned-16"),
    "synthetic/ct_sigmoid_width_half.dcm":
        ("CT", "synthetic, mono16, sigmoid-width-below-one"),
    "synthetic/cr_monochrome1.dcm": ("CR", "synthetic, mono16, monochrome1"),
    "synthetic/mr_nonsquare_spacing.dcm":
        ("MR", "synthetic, mono16, nonsquare-spacing"),
    "synthetic/sc_rgb_interleaved.dcm":
        ("OT", "synthetic, colour, planar-config-0"),
    "synthetic/sc_rgb_planar.dcm": ("OT", "synthetic, colour, planar-config-1"),
    "synthetic/sc_rgb_icc.dcm":
        ("OT", "synthetic, colour, icc-profile-present-named-not-applied"),
    "synthetic/sc_rgb_icc_gamma1.dcm":
        ("OT", "synthetic, colour, icc-gamma1"),
    "synthetic/sc_rgb_icc_gamma22.dcm":
        ("OT", "synthetic, colour, icc-gamma22"),
    "synthetic/sc_rgb_icc_srgb_para.dcm":
        ("OT", "synthetic, colour, icc-srgb_para"),
    "synthetic/sc_rgb_icc_swap.dcm":
        ("OT", "synthetic, colour, icc-swap"),
    "synthetic/sc_rgb_icc_lut.dcm":
        ("OT", "synthetic, colour, icc-lut"),
    "synthetic/us_ybr_full_422.dcm":
        ("US", "synthetic, us, colour, ybr-full-422"),
    "synthetic/sc_palette_color.dcm":
        ("OT", "synthetic, colour, palette-color, first-mapped-input-10"),
    "synthetic/sc_palette_color_16.dcm":
        ("OT", "synthetic, colour, palette-color, entry-count-zero-means-65536"),
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
        REFERENCE_RGB8_SATURATED: "synthetic, colour, syntax-reference",
        REFERENCE_BANDED16: "synthetic, mono16, syntax-reference",
    }
    for name in sorted(set(SYNTAX_CASES) | set(syntax_category)):
        syntax = SYNTAX_CASES.get(name, "1.2.840.10008.1.2.1")
        # The tolerance class and the modality are both decided by whether the
        # case is built on an RGB reference, so they read the same fact rather
        # than two rules that happen to agree. Before F-030 the class was keyed
        # on one exact filename and the modality on the substring, which agreed
        # by coincidence while there was exactly one colour case. A reference
        # row reads its own declared category, which already names the class.
        if name in syntax_category:
            category = syntax_category[name]
            colour = "colour" in (token.strip()
                                  for token in category.split(","))
        else:
            colour = SYNTAX_REFERENCE[name] in (
                REFERENCE_RGB8, REFERENCE_RGB8_SATURATED)
            category = ("synthetic, colour, transfer-syntax" if colour
                        else "synthetic, mono16, transfer-syntax")
        if name in EMPTY_BOT_CASES:
            # F-244. The trap these rows exist for, and their shape.
            category += ", empty-basic-offset-table"
            if name in EMPTY_BOT_SHAPES:
                category += f", {EMPTY_BOT_SHAPES[name][1]}"
        modality = "OT" if colour else "CT"
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


# Controlled comparison inputs are an explicit opt-in, not corpus rows.
COMPARISON_COLOUR_CASES = {
    "rgb8": (512, 200, "RGB", 3, 8),
    "ybr422": (512, 200, "YBR_FULL_422", 3, 8),
    "palette16": (512, 200, "PALETTE COLOR", 1, 16),
    "icc8-gamma22": (8192, 1, "RGB", 3, 8),
    "icc16-table": (8192, 1, "RGB", 3, 16),
}
COMPARISON_INPUT_MANIFEST = "colour-comparison-input.json"


def comparison_icc_profile(case: str) -> bytes | None:
    """Own F240 profiles, ICC.1:2022 7.2.18 and 10.6.

    The two-entry curve occupies the existing 16-byte padded slot. Its tag
    length becomes 16, and the profile ID is recomputed after every edit.
    """
    if case == "icc8-gamma22":
        return synthetic_icc_profile("gamma22")
    if case != "icc16-table":
        return None
    profile = bytearray(synthetic_icc_profile("gamma1"))
    count = struct.unpack_from(">I", profile, 128)[0]
    for index in range(count):
        start = 132 + index * 12
        signature, offset, length = struct.unpack_from(">4sII", profile, start)
        if signature not in (b"rTRC", b"gTRC", b"bTRC"):
            continue
        assert length == 14 and offset + 16 <= len(profile)
        struct.pack_into(">I", profile, start + 8, 16)
        profile[offset:offset + 16] = b"curv\0\0\0\0" + struct.pack(">I2H", 2, 0, 65535)
    hashed = bytearray(profile)
    hashed[44:48] = bytes(4)
    hashed[64:68] = bytes(4)
    hashed[84:100] = bytes(16)
    profile[84:100] = hashlib.md5(hashed, usedforsecurity=False).digest()
    return bytes(profile)


def comparison_colour_row(case: str, edge: int, row: int, frame: int) -> bytes:
    """Stored samples, no colour conversion or display arithmetic here.

    PS3.3 C.7.6.3.1.2 native422 stores Y1,Y2,Cb,Cr per horizontal pair.
    RGB patterns distinguish channels, rows, columns and frame identity.
    The first row also has neutral marker samples, defined independently
    of the ICC converter. At 16 bits they retain low device-code bits.
    """
    columns = np.arange(edge, dtype=np.uint32)
    if case == "ybr422":
        first = np.arange(0, edge, 2, dtype=np.uint32)
        pairs = np.stack(((first + frame) % 256,
                          (first + frame + 1) % 256,
                          np.full(first.shape, (row + 64) % 256),
                          np.full(first.shape, (row + 128 + 3 * frame) % 256)), axis=1)
        if row == 0:
            pairs[0] = (64, 128, 128, 128)
            if len(pairs) > 1:
                pairs[1] = (100, 101, 150, 90)
        return pairs.astype(np.uint8).tobytes()
    if case == "palette16":
        return ((row * edge + columns + 257 * frame) % 65536).astype("<u2").tobytes()
    if case == "icc16-table":
        maximum, factor, dtype = 65536, 257, "<u2"
        markers = (15, 16384, 32768, 49152, 65535)
    else:
        maximum, factor, dtype = 256, 1, np.uint8
        markers = (64, 128, 192, 255)
    values = np.stack(((columns * factor + 31 * frame) % maximum,
                       np.full(columns.shape, (row * factor + 47 * frame) % maximum),
                       ((columns + row) * factor + 71 * frame) % maximum), axis=1)
    if row == 0:
        for at, value in enumerate(markers[:edge]):
            values[at] = (value, value, value)
    return values.astype(dtype).tobytes()


def _comparison_colour_dataset(case: str, edge: int, frame: int,
                                profile: bytes | None) -> FileDataset:
    _, _, photo, samples, bits = COMPARISON_COLOUR_CASES[case]
    label = f"comparison-colour-v1:{case}:{edge}:{frame}"
    ds = new_dataset(label, SC_STORAGE, "OT", study="comparison-colour-v1",
                     series=f"comparison-colour-v1:{case}:{edge}",
                     frame_of_reference="comparison-colour-v1")
    ds.ConversionType = "WSD"
    ds.InstanceNumber = frame + 1
    ds.SeriesNumber = 2
    ds.ImagePositionPatient = ["0", "0", str(frame)]
    ds.ImageOrientationPatient = ["1", "0", "0", "0", "1", "0"]
    ds.PixelSpacing = ["1", "1"]
    ds.Rows = ds.Columns = edge
    ds.SamplesPerPixel = samples
    ds.PhotometricInterpretation = photo
    if samples == 3:
        ds.PlanarConfiguration = 0
    ds.BitsAllocated = ds.BitsStored = bits
    ds.HighBit = bits - 1
    ds.PixelRepresentation = 0
    if case == "palette16":
        indices = np.arange(65536, dtype=np.uint32)
        for colour, values in [("Red", indices), ("Green", 65535 - indices),
                               ("Blue", (257 * indices) % 65536)]:
            setattr(ds, colour + "PaletteColorLookupTableDescriptor", [0, 0, 16])
            setattr(ds, colour + "PaletteColorLookupTableData", values.astype("<u2").tobytes())
    if profile is not None:
        ds.ICCProfile = profile
    return ds


def write_colour_comparison(out: Path, case: str, *, edge: int | None = None,
                            frames: int | None = None) -> Path:
    """Write one controlled target and the unchanged CT calibration class.

    Defaults are the approved measurement sizes. Explicit small dimensions
    exist for unit fixtures and are marked unsuitable for measurements.
    Pixel Data is streamed row-wise, including the 384 MiB RGB16 case. No
    corpus manifest or tracked DICOM is written. Existing nonempty output
    is refused, because replacing another run's evidence is not a generator.
    """
    import json
    if case not in COMPARISON_COLOUR_CASES:
        raise ValueError("unknown controlled colour comparison case")
    standard_edge, standard_frames, photo, samples, bits = COMPARISON_COLOUR_CASES[case]
    edge = standard_edge if edge is None else edge
    frames = standard_frames if frames is None else frames
    if (type(edge) is not int or not 2 <= edge <= 8192 or edge % 2 or
            type(frames) is not int or not 1 <= frames <= 200):
        raise ValueError("invalid controlled colour comparison dimensions")
    out = Path(out)
    if out.exists() and any(out.iterdir()):
        raise ValueError("controlled colour comparison output must be empty")
    out.mkdir(parents=True, exist_ok=True)
    calibration = new_dataset("comparison-colour-v1:calibration", CT_STORAGE, "CT",
                              study="comparison-colour-v1", series="comparison-colour-v1:ct")
    calibration.InstanceNumber = calibration.SeriesNumber = 1
    calibration.Rows = calibration.Columns = 512
    calibration.SamplesPerPixel = 1
    calibration.PhotometricInterpretation = "MONOCHROME2"
    calibration.BitsAllocated = calibration.BitsStored = 16
    calibration.HighBit = 15
    calibration.PixelRepresentation = 0
    calibration.RescaleSlope, calibration.RescaleIntercept = "1", "0"
    calibration.WindowCenter, calibration.WindowWidth = "40", "400"
    calibration.VOILUTFunction = "LINEAR"
    calibration.PixelSpacing = ["1", "1"]
    calibration.ImagePositionPatient = ["0", "0", "0"]
    calibration.ImageOrientationPatient = ["1", "0", "0", "0", "1", "0"]
    calibration.PixelData = np.broadcast_to(np.arange(512, dtype="<u2"), (512, 512)).tobytes()
    calibration_path = out / "calibration-ct" / "000000.dcm"
    write(calibration, calibration_path)
    profile = comparison_icc_profile(case)
    target = out / "colour"
    target.mkdir()
    files = []
    series_uid = None
    for frame in range(frames):
        ds = _comparison_colour_dataset(case, edge, frame, profile)
        series_uid = str(ds.SeriesInstanceUID)
        path = target / f"{frame:06d}.dcm"
        # PS3.5 7.1.2. Long-form explicit VR native Pixel Data, even length.
        stored_samples = 2 if case == "ybr422" else samples
        length = edge * edge * stored_samples * (bits // 8)
        with path.open("wb") as handle:
            pydicom.dcmwrite(handle, ds, implicit_vr=False, little_endian=True,
                            enforce_file_format=True)
            handle.write(struct.pack("<HH2sHI", 0x7FE0, 0x0010,
                                     b"OW" if bits == 16 else b"OB", 0, length))
            for row in range(edge):
                handle.write(comparison_colour_row(case, edge, row, frame))
        files.append({"path": path.relative_to(out).as_posix(),
                      "bytes": path.stat().st_size, "sha256": digest(path)})
    def entry(path: Path) -> dict:
        return {"path": path.relative_to(out).as_posix(),
                "bytes": path.stat().st_size, "sha256": digest(path)}
    manifest = {
        "schemaVersion": 1, "generator": "scripts/corpus_synth.py:comparison-colour-v1",
        "caseId": case, "rows": edge, "columns": edge, "frames": frames,
        "measurementSize": edge == standard_edge and frames == standard_frames,
        "photometric": photo, "samplesPerPixel": samples,
        "bitsAllocated": bits, "bitsStored": bits,
        "seriesUid": series_uid, "pixelPattern": "position-frame-v1",
        "serverCacheMiB": 512 if case == "icc16-table" else 256,
        "profile": None if profile is None else {
            "kind": "gamma22" if case == "icc8-gamma22" else "curv-two-entry-linear",
            "sha256": hashlib.sha256(profile).hexdigest(), "bytes": len(profile)},
        "calibrationDirectory": "calibration-ct", "calibrationFiles": [entry(calibration_path)],
        "targetDirectory": "colour", "targetFiles": files,
    }
    path = out / COMPARISON_INPUT_MANIFEST
    path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n")
    return path


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
    parser.add_argument("--case", metavar="NAME",
                        help="regenerate ONE case, leaving every other "
                             "generated file untouched. Use this to add a "
                             "case without re-encoding rows whose encoder has "
                             "moved since the manifest was built")
    parser.add_argument("--list-cases", action="store_true",
                        help="print the names --case accepts")
    parser.add_argument("--comparison-colour", choices=tuple(COMPARISON_COLOUR_CASES),
                        help="write one controlled colour comparison dataset, requires --out")
    args = parser.parse_args()
    if args.comparison_colour:
        if (not args.out or args.case or args.manifest_rows or args.write_manifest or
                args.tool_versions or args.list_cases):
            parser.error("--comparison-colour needs explicit --out and no corpus mode")
        path = write_colour_comparison(Path(args.out).expanduser(), args.comparison_colour)
        print(f"wrote controlled comparison manifest {path}")
        return 0


    if args.tool_versions:
        return report_tool_versions()
    if args.list_cases:
        print("\n".join(case_callables(Path(".")).keys()))
        return 0

    out = Path(args.out).expanduser() if args.out else corpus_dir()
    if args.case:
        generate_one(out, args.case)
        print(f"wrote case {args.case} under {out}")
        print("next: uv run scripts/corpus_synth.py --write-manifest, "
              "then uv run scripts/corpus_check.py")
        print("every other row's digest is re-read from the file already on "
              "disk, so only the new one moves")
        return 0
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
