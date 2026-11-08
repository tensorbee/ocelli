#!/usr/bin/env python3
"""Tests for scripts/corpus_synth.py, the synthetic corpus generator.

Run with the interpreter that has pydicom, numpy and the codec plugins:

    /path/to/venv/bin/python -m unittest discover -s scripts/tests -v

Three things are proved here, and they are the three that make a synthetic
corpus worth having.

1. **fixture.** The stored value of a hand-chosen raw word, computed from the
   historical stored-window interpretation of BitsStored (0028,0101), HighBit
   (0028,0102) and PixelRepresentation (0028,0103). Current PS3.3 C.7.6.3.3
   requires HighBit to equal BitsStored minus one, so the HighBit 15 case is
   explicitly legacy nonconforming interoperability evidence. The expected
   values in this file were computed by hand and are shown working. They were
   NOT read back from the generator, and the generator contains no unpacking
   code they could have come from.

2. **unit.** The generator is byte-deterministic. The manifest is a sha256 per
   case, so a generator that stamps a fresh UID or today's date produces a
   different digest on every machine and the manifest stops meaning anything.

3. **conformance.** Every compressed case actually carries the codestream its
   transfer syntax declares, and decodes back to the uncompressed reference.
   A file declaring JPEG-LS around something that is not JPEG-LS fails much
   later, inside the codec story, where it looks like a decoder bug.
"""

from __future__ import annotations

import datetime
import hashlib
import json
import math
import os
import re
import struct
import subprocess
import sys
import shutil
import tempfile
import tomllib
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

try:
    import numpy as np
    import pydicom
except ImportError as exc:  # pragma: no cover - environment, not a defect
    raise unittest.SkipTest(
        f"pydicom and numpy are needed to read what the generator writes: "
        f"{exc}. Run this file with the interpreter that has them."
    ) from exc

# Deliberately outside the skip. A missing generator is a failure, not a
# reason to report the suite as skipped.
import corpus_check  # noqa: E402
import corpus_synth  # noqa: E402


# One read-back route per SYNTAX_CASES filename. The helper owns the choice.
# No route relies on pydicom plugin order. Colour conversion for the baseline
# colour case happens inside libjpeg-turbo and this helper does not convert
# a second time.
READ_BACK = {
    "implicit_vr_le.dcm": "native",
    "explicit_vr_le.dcm": "native",
    "deflated_explicit_vr_le.dcm": "native",
    "explicit_vr_be.dcm": "native",
    "rle_lossless.dcm": "pylibjpeg",
    "jpeg_baseline_rgb8.dcm": "jpeg8-rgb",
    "jpeg_extended_12.dcm": "jpeg8",
    "jpeg_lossless_p14.dcm": "ljpeg",
    "jpeg_lossless_p14_sv1.dcm": "ljpeg",
    "jpegls_lossless.dcm": "pyjpegls",
    "jpegls_lossless_rgb8.dcm": "pyjpegls",
    "jpegls_near_lossless.dcm": "pyjpegls",
    "jpegls_banded_lossless.dcm": "pyjpegls",
    "jpegls_banded_near_lossless.dcm": "pyjpegls",
    "j2k_lossless.dcm": "pylibjpeg",
    "j2k_lossy.dcm": "pylibjpeg",
    # The F-239 colour rows. OpenJPEG applies the codestream's own inverse
    # component transform, so the plugin hands back RGB for all four.
    "j2k_lossless_ybr_rct.dcm": "pylibjpeg",
    "j2k_lossy_ybr_ict.dcm": "pylibjpeg",
    "htj2k_lossless_ybr_rct.dcm": "pylibjpeg",
    "htj2k_lossy_ybr_ict.dcm": "pylibjpeg",
    "htj2k_lossless.dcm": "pylibjpeg",
    "htj2k_lossless_rpcl.dcm": "pylibjpeg",
    "htj2k_lossy.dcm": "pylibjpeg",
    "jpegxl_lossless.dcm": "jpegxl",
    "jpegxl_lossless_rgb8.dcm": "jpegxl",
    "jpegxl_recompression_rgb8.dcm": "unavailable",
    "jpegxl_lossy.dcm": "jpegxl",
    "jpegxl_xyb_rgb8.dcm": "jpegxl",
}
# F-244. An empty-table row is read by its source row's route. pydicom 3.0.2
# reads a zero-length Basic Offset Table as PS3.5 A.4 says, independently of
# Ocelli, and splits frames that span Fragments at their end markers.
for _name, _source in corpus_synth.EMPTY_BOT_CASES.items():
    READ_BACK[_name] = READ_BACK[_source]

# imagecodecs.__version__ and the string jpeg8_version() reports.
DECODED_WITH = {
    "imagecodecs": "2026.10.10",
    "jpeg8_version": "libjpeg_turbo 3.2.0/8.0",
}


class UnavailableSyntax(AssertionError):
    """The registry knows this syntax and refuses to decode it."""


def read_back(ds: "pydicom.Dataset", name: str):
    """Decode one syntax case through its declared route."""
    route = READ_BACK[name]
    if route == "native":
        return pydicom.pixels.pixel_array(ds)
    if route == "pylibjpeg":
        return pydicom.pixels.pixel_array(ds, decoding_plugin="pylibjpeg")
    if route == "pyjpegls":
        return pydicom.pixels.pixel_array(ds, decoding_plugin="pyjpegls")
    import imagecodecs
    from pydicom.encaps import generate_frames
    if route == "unavailable":
        raise UnavailableSyntax(
            f"{corpus_check.UNAVAILABLE_REFUSAL} "
            f"(jxl-jbr 0.2.1 src/reconstruct.rs:457)")
    decoders = {
        "ljpeg": imagecodecs.ljpeg_decode,
        "jpeg8": imagecodecs.jpeg8_decode,
        "jpeg8-rgb": lambda fragment: imagecodecs.jpeg8_decode(
            fragment, outcolorspace="RGB"),
        "jpegxl": imagecodecs.jpegxl_decode,
    }
    if route not in decoders:
        raise AssertionError(f"unknown read-back route {route}")
    count = int(ds.get("NumberOfFrames", 1) or 1)
    frames = [decoders[route](fragment) for fragment in generate_frames(
        ds.PixelData, number_of_frames=count)]
    if len(frames) != count:
        raise AssertionError(f"{name} yielded {len(frames)} frames, not {count}")
    return frames[0] if count == 1 else np.stack(frames)


def decode_array_sha256(array) -> str:
    """SHA-256 of dtype, shape and bytes. The digest table uses this form."""
    import hashlib
    values = np.ascontiguousarray(array)
    head = (
        f"{values.dtype.str} {','.join(str(int(n)) for n in values.shape)}\n"
    ).encode("ascii")
    return hashlib.sha256(head + values.tobytes()).hexdigest()


# ---------------------------------------------------------------------------
# The hand-computed fixture table.
# ---------------------------------------------------------------------------
#
# The retained legacy interpretation places the stored value inside its
# container as follows. This arithmetic explains the nonconforming evidence.
# It does not make HighBit 15 conform to current PS3.3 C.7.6.3.3:
#
#     shift  = HighBit + 1 - BitsStored
#     mask   = (1 << BitsStored) - 1
#     value  = (raw >> shift) & mask
#     if PixelRepresentation == 1 and value has bit (BitsStored - 1) set:
#         value -= 1 << BitsStored          # two's complement sign extension
#
# The generator writes these eight raw 16-bit words as the first eight pixels
# of BOTH signed 12-bit-in-16 cases. Identical bytes, different header, so the
# same word has a different stored value in each file. That is the point: a
# reader that ignores HighBit gets one of the two files right and the other
# wrong, and both look plausible.

PROBE_WORDS = (0xF800, 0x07FF, 0x0FFF, 0x0801, 0x8000, 0x7FF0, 0xFFF0, 0x800F)

# ct_signed_12in16_right.dcm: BitsStored 12, HighBit 11, PixelRepresentation 1.
# shift = 11 + 1 - 12 = 0, mask = 0x0FFF, sign bit = 0x0800.
#
#   0xF800 >> 0 = 0xF800, & 0x0FFF = 0x800 = 2048, sign set -> 2048 - 4096 = -2048
#   0x07FF >> 0 = 0x07FF, & 0x0FFF = 0x7FF = 2047, sign clear ->            2047
#   0x0FFF >> 0 = 0x0FFF, & 0x0FFF = 0xFFF = 4095, sign set -> 4095 - 4096 =    -1
#   0x0801 >> 0 = 0x0801, & 0x0FFF = 0x801 = 2049, sign set -> 2049 - 4096 = -2047
#   0x8000 >> 0 = 0x8000, & 0x0FFF = 0x000 =    0, sign clear ->               0
#   0x7FF0 >> 0 = 0x7FF0, & 0x0FFF = 0xFF0 = 4080, sign set -> 4080 - 4096 =   -16
#   0xFFF0 >> 0 = 0xFFF0, & 0x0FFF = 0xFF0 = 4080, sign set -> 4080 - 4096 =   -16
#   0x800F >> 0 = 0x800F, & 0x0FFF = 0x00F =   15, sign clear ->              15
#
# 0xF800 -> -2048 is the 12-bit signed minimum and 0x07FF -> 2047 the maximum,
# which are the two the header exists to make reachable. 0x0FFF -> -1 is the
# one that matters most in review: a reader that casts the raw word straight to
# i16 reports 4095, which is a perfectly plausible Hounsfield number.
RIGHT_ALIGNED = (-2048, 2047, -1, -2047, 0, -16, -16, 15)

# ct_signed_12in16_left.dcm: legacy nonconforming interoperability evidence.
# BitsStored 12, HighBit 15, PixelRepresentation 1.
# shift = 15 + 1 - 12 = 4, mask = 0x0FFF, sign bit = 0x0800.
#
#   0xF800 >> 4 = 0x0F80, & 0x0FFF = 0xF80 = 3968, sign set -> 3968 - 4096 =  -128
#   0x07FF >> 4 = 0x007F, & 0x0FFF = 0x07F =  127, sign clear ->             127
#   0x0FFF >> 4 = 0x00FF, & 0x0FFF = 0x0FF =  255, sign clear ->             255
#   0x0801 >> 4 = 0x0080, & 0x0FFF = 0x080 =  128, sign clear ->             128
#   0x8000 >> 4 = 0x0800, & 0x0FFF = 0x800 = 2048, sign set -> 2048 - 4096 = -2048
#   0x7FF0 >> 4 = 0x07FF, & 0x0FFF = 0x7FF = 2047, sign clear ->            2047
#   0xFFF0 >> 4 = 0x0FFF, & 0x0FFF = 0xFFF = 4095, sign set -> 4095 - 4096 =    -1
#   0x800F >> 4 = 0x0800, & 0x0FFF = 0x800 = 2048, sign set -> 2048 - 4096 = -2048
#
# 0x800F is the one that catches a missing shift: the low nibble sits below the
# stored field and must be discarded, so it reads the same as 0x8000.
LEFT_ALIGNED = (-128, 127, 255, 128, -2048, 2047, -1, -2048)

# PS3.3 C.11.2.1.3.1, with c = 40, w = 0.5, ymin = 0, ymax = 255:
#
#     y = 255 / (1 + exp(-4 * (x - 40) / 0.5))
#
# The modality inputs below are the five central stored values after the
# fixture's slope of 0.25. The expected display values are written out rather
# than imported from the generator or read from cornerstone3D.
SIGMOID_MODALITY_VALUES = (39.5, 39.75, 40.0, 40.25, 40.5)
SIGMOID_CENTRE = 40.0
SIGMOID_WIDTH = 0.5
SIGMOID_DISPLAY_MINIMUM = 0.0
SIGMOID_DISPLAY_MAXIMUM = 255.0
SIGMOID_DISPLAY_VALUES = (
    4.586483540333347,
    30.396745115639977,
    127.5,
    224.60325488436,
    250.41351645966665,
)


def stored_value(raw: int, bits_stored: int, high_bit: int,
                 pixel_representation: int) -> int:
    """PS3.3 C.7.6.3.1.4, transcribed. Independent of anything Ocelli does."""
    shift = high_bit + 1 - bits_stored
    value = (raw >> shift) & ((1 << bits_stored) - 1)
    if pixel_representation == 1 and value & (1 << (bits_stored - 1)):
        value -= 1 << bits_stored
    return value


def sigmoid_display(value: float, centre: float, width: float,
                    display_minimum: float, display_maximum: float) -> float:
    """PS3.3 C.11.2.1.3.1, transcribed independently of the generator."""
    extent = display_maximum - display_minimum
    return display_minimum + extent / (
        1.0 + math.exp(-4.0 * (value - centre) / width)
    )


def sigmoid_expectation_boundary() -> None:
    """Execute the independent literals against the transcribed formula."""
    got = tuple(
        sigmoid_display(
            value,
            SIGMOID_CENTRE,
            SIGMOID_WIDTH,
            SIGMOID_DISPLAY_MINIMUM,
            SIGMOID_DISPLAY_MAXIMUM,
        )
        for value in SIGMOID_MODALITY_VALUES
    )
    for actual, expected in zip(got, SIGMOID_DISPLAY_VALUES, strict=True):
        if round(abs(actual - expected), 12) != 0:
            raise AssertionError("SIGMOID expectation fixture mismatch")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_digests(base: Path) -> dict[str, str]:
    return {p.relative_to(base).as_posix(): digest(p)
            for p in sorted(base.rglob("*")) if p.is_file()}


def generated_manifest_rows() -> list[dict[str, str]]:
    return [dict(zip(corpus_check.COLUMNS, line.split("\t")))
            for line in corpus_synth.manifest_rows(CORPUS)]


_TMP: tempfile.TemporaryDirectory | None = None
CORPUS: Path


def setUpModule() -> None:
    global _TMP, CORPUS
    _TMP = tempfile.TemporaryDirectory(prefix="ocelli-synth-")
    CORPUS = Path(_TMP.name) / "a"
    corpus_synth.generate(CORPUS)


def tearDownModule() -> None:
    if _TMP is not None:
        _TMP.cleanup()


class StoredValueFixture(unittest.TestCase):
    """Conforming and retained legacy 12-bit-in-16 expected values."""

    def probe(self, name: str) -> tuple[pydicom.Dataset, tuple[int, ...]]:
        ds = pydicom.dcmread(str(CORPUS / "synthetic" / name))
        words = struct.unpack("<8H", bytes(ds.PixelData)[:16])
        self.assertEqual(words, PROBE_WORDS,
                         "the generator no longer writes the probe words this "
                         "fixture was computed against")
        return ds, words

    def test_right_aligned_header_is_what_the_case_claims(self) -> None:
        ds, _ = self.probe("ct_signed_12in16_right.dcm")
        self.assertEqual((ds.BitsAllocated, ds.BitsStored, ds.HighBit,
                          ds.PixelRepresentation), (16, 12, 11, 1))

    def test_legacy_nonconforming_header_is_what_the_case_claims(self) -> None:
        ds, _ = self.probe("ct_signed_12in16_left.dcm")
        self.assertEqual((ds.BitsAllocated, ds.BitsStored, ds.HighBit,
                          ds.PixelRepresentation), (16, 12, 15, 1))

        rows = corpus_check.load()
        row = next(row for row in rows
                   if row["path"] ==
                   "synthetic/ct_signed_12in16_left.dcm")
        self.assertIn("legacy-nonconforming", corpus_check.tokens(row))

    def test_right_aligned_stored_values(self) -> None:
        ds, words = self.probe("ct_signed_12in16_right.dcm")
        got = tuple(stored_value(w, ds.BitsStored, ds.HighBit,
                                 ds.PixelRepresentation) for w in words)
        self.assertEqual(got, RIGHT_ALIGNED)

    def test_legacy_nonconforming_left_aligned_stored_values(self) -> None:
        ds, words = self.probe("ct_signed_12in16_left.dcm")
        got = tuple(stored_value(w, ds.BitsStored, ds.HighBit,
                                 ds.PixelRepresentation) for w in words)
        self.assertEqual(got, LEFT_ALIGNED)

    def test_the_two_cases_share_bytes_and_differ_in_meaning(self) -> None:
        """The whole reason both cases exist. Same words, different values."""
        self.assertNotEqual(RIGHT_ALIGNED, LEFT_ALIGNED)
        right = bytes(pydicom.dcmread(
            str(CORPUS / "synthetic" / "ct_signed_12in16_right.dcm")).PixelData)
        left = bytes(pydicom.dcmread(
            str(CORPUS / "synthetic" / "ct_signed_12in16_left.dcm")).PixelData)
        self.assertEqual(right[:16], left[:16])


class SigmoidFixture(unittest.TestCase):
    """The width below one that SIGMOID permits, from PS3.3 C.11.2.1.3.1."""

    def case(self) -> pydicom.Dataset:
        return pydicom.dcmread(
            str(CORPUS / "synthetic" / "ct_sigmoid_width_half.dcm")
        )

    def test_width_half_is_declared_as_sigmoid(self) -> None:
        ds = self.case()
        self.assertEqual(ds.VOILUTFunction, "SIGMOID")
        self.assertEqual(float(ds.WindowCenter), 40.0)
        self.assertEqual(float(ds.WindowWidth), 0.5)

    def test_stored_values_cross_the_window_centre(self) -> None:
        ds = self.case()
        first_row = tuple(int(value) for value in ds.pixel_array[0])
        self.assertEqual(first_row, tuple(range(151, 171)))
        modality = tuple(
            value * float(ds.RescaleSlope) + float(ds.RescaleIntercept)
            for value in first_row
        )
        self.assertLess(modality[0], float(ds.WindowCenter))
        self.assertIn(float(ds.WindowCenter), modality)
        self.assertGreater(modality[-1], float(ds.WindowCenter))

    def test_selected_display_values_follow_the_sigmoid_formula(self) -> None:
        sigmoid_expectation_boundary()


# Worst-pixel bounds for the lossy sanity check, as a fraction of full scale,
# measured from what the encoders produce at the settings the generator uses.
#
# The colour case needs its own bound because of its container, not its
# subsampling. Through dcmcjpeg at quality 90, one flag changed at a time: RGB
# with no YBR at all costs 1 of 255, the YBR round trip at 4:4:4 costs 3, the
# shipped 4:2:2 costs 4, and 4:2:0 with twice the subsampling still costs 4. HLD 25.1 names chroma subsampling AND YBR conversion as the
# reason for its second tolerance class, and on this content the second is
# three quarters of it. Hence the name, and hence the selector below keying on
# PhotometricInterpretation. Shipped 0.0157, and 0.0196 on a transposed variant
# of the same shape, so this bound is 1.5 times the worst measured.
# Re-measured with imagecodecs jpeg8_decode and outcolorspace RGB: 4 of 255,
# ratio 0.015686. The bound is unchanged.
# libjxl 0.12.0 at distance 1.0 and effort 7 on reference_rgb8. TRANSFORM_MAX
# times 255 is 1.275 levels. The same encoder's smallest positive distance
# on this ramp is already 7 levels, so the generic fraction cannot hold.
# This ceiling is that encode's measured maximum absolute error. It is not
# a change to TRANSFORM_MAX.
JPEGXL_XYB_MAX_ABS = 21

YBR_EIGHT_BIT_MAX = 0.03
# The three transform cases at their configured settings: JPEG extended at
# quality 90 gives 0.00049, JPEG 2000 at ratio 20 gives 0.00003, HTJ2K at
# qstep 0.001 gives 0.00063, so this is the worst of them rounded up an order.
# Re-measured with imagecodecs jpeg8_decode: the extended 12-bit case is 2 of
# 4095, ratio 0.000488. JPEG 2000 and HTJ2K stay on pylibjpeg and the figures
# above are unchanged. The bound is unchanged.
TRANSFORM_MAX = 0.005

# The identity constants a regenerated case must carry. Written out here rather
# than imported from the generator: a test that reads the value it is checking
# proves only that the generator agrees with itself.
FIXED_DATE = "20200101"
FIXED_TIME = "120000.000000"
INSTANCE_UID_ARC = "2.25."


class Determinism(unittest.TestCase):
    """A regenerated corpus must have identical digests, or the manifest lies."""

    def test_two_runs_in_separate_processes_are_byte_identical(self) -> None:
        """Separate processes, deliberately.

        Running both generations inside one interpreter proves much less than
        it looks like it does: a module-level constant evaluated from the
        clock, the process id or a fresh UUID is computed ONCE at import and
        then agrees with itself for the rest of the run. Two processes, at two
        different wall-clock moments, is the cheapest thing that actually
        catches that class.
        """
        with tempfile.TemporaryDirectory(prefix="ocelli-synth-runs-") as scratch:
            digests = []
            for run in ("first", "second"):
                target = Path(scratch) / run
                result = subprocess.run(
                    [sys.executable, str(ROOT / "scripts" / "corpus_synth.py"),
                     "--out", str(target)],
                    capture_output=True, text=True)
                self.assertEqual(result.returncode, 0,
                                 result.stderr or result.stdout)
                digests.append(tree_digests(target))

            first, second = digests
            self.assertTrue(first, "the generator produced no files")
            self.assertEqual(sorted(first), sorted(second))
            differing = [name for name, sha in first.items()
                         if second[name] != sha]
            self.assertEqual(differing, [],
                             "these cases are not byte-deterministic, so "
                             "their manifest digests cannot be trusted")

    def test_no_case_carries_a_clock_reading(self) -> None:
        """A date or time taken from the clock is the commonest way a
        generated corpus stops matching its manifest, and it is invisible
        until someone regenerates on another day."""
        today = datetime.date.today().strftime("%Y%m%d")
        self.assertNotEqual(FIXED_DATE, today,
                            "pick a fixed date that is not today, or this "
                            "test cannot tell the two apart")
        for path in sorted(CORPUS.rglob("*.dcm")):
            with self.subTest(case=path.relative_to(CORPUS).as_posix()):
                ds = pydicom.dcmread(str(path), stop_before_pixels=True)
                for keyword in ("StudyDate", "SeriesDate", "ContentDate",
                                "AcquisitionDate", "InstanceCreationDate"):
                    if keyword in ds:
                        self.assertEqual(getattr(ds, keyword), FIXED_DATE)
                for keyword in ("StudyTime", "SeriesTime", "ContentTime",
                                "AcquisitionTime", "InstanceCreationTime"):
                    if keyword in ds:
                        self.assertEqual(getattr(ds, keyword), FIXED_TIME)

    def test_every_instance_uid_is_derived_not_generated(self) -> None:
        """ISO/IEC 9834-8 gives `2.25.` to UUID-derived OIDs. A UID outside it
        is either a real institution's root, which this corpus has no business
        carrying, or a freshly generated one, which breaks the digest."""
        for path in sorted(CORPUS.rglob("*.dcm")):
            with self.subTest(case=path.relative_to(CORPUS).as_posix()):
                ds = pydicom.dcmread(str(path), stop_before_pixels=True)
                for uid in (ds.SOPInstanceUID, ds.StudyInstanceUID,
                            ds.SeriesInstanceUID,
                            ds.file_meta.MediaStorageSOPInstanceUID,
                            ds.file_meta.ImplementationClassUID):
                    self.assertTrue(str(uid).startswith(INSTANCE_UID_ARC),
                                    f"{uid} is not in the {INSTANCE_UID_ARC} arc")


class MetadataAudit(unittest.TestCase):
    """Manifest coverage labels must agree with PS3.3 and PS3.10 metadata."""

    def problems(self, rows: list[dict[str, str]]) -> list[str]:
        return corpus_check.metadata_problems(rows, CORPUS)

    def test_generated_manifest_labels_match_every_file(self) -> None:
        self.assertEqual(self.problems(generated_manifest_rows()), [])

    def test_a_wrong_modality_is_named_by_relative_path(self) -> None:
        rows = generated_manifest_rows()
        rows[0] = {**rows[0], "modality": "MR"}
        self.assertEqual(
            self.problems(rows),
            [f"{rows[0]['path']}: manifest modality does not match the file"])

    def test_a_wrong_transfer_syntax_is_named_by_relative_path(self) -> None:
        rows = generated_manifest_rows()
        rows[0] = {**rows[0],
                   "transfer_syntax": "1.2.840.10008.1.2"}
        self.assertEqual(
            self.problems(rows),
            [f"{rows[0]['path']}: manifest transfer syntax does not match "
             "the file"])

    def test_mono16_must_match_the_pixel_module(self) -> None:
        rows = generated_manifest_rows()
        row = next(r for r in rows if r["path"].endswith("reference_rgb8.dcm"))
        row["category"] = "synthetic, mono16"
        self.assertEqual(
            self.problems([row]),
            [f"{row['path']}: mono16 category does not match the pixel module"])

    def test_colour_must_match_the_pixel_module(self) -> None:
        rows = generated_manifest_rows()
        row = next(r for r in rows if r["path"].endswith("reference_mono12.dcm"))
        row["category"] = "synthetic, colour"
        self.assertEqual(
            self.problems([row]),
            [f"{row['path']}: colour category does not match the pixel module"])

    def test_us_must_match_modality(self) -> None:
        rows = generated_manifest_rows()
        row = next(r for r in rows if r["path"].endswith("reference_mono12.dcm"))
        row["category"] = "synthetic, us"
        self.assertEqual(
            self.problems([row]),
            [f"{row['path']}: us category does not match Modality"])

    def test_metadata_dispatch_fails_when_the_interpreter_is_absent(self) -> None:
        environment = os.environ.copy()
        environment["OCELLI_PYTHON"] = "/definitely/missing/ocelli-python"
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts" / "corpus_tests.py"),
             "--metadata-check"],
            cwd=ROOT,
            env=environment,
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn("FAIL: no interpreter can import", result.stdout)
        self.assertIn("absent reader is a failure rather than a skip",
                      result.stdout)


class Conformance(unittest.TestCase):
    """Each syntax case carries the codestream its transfer syntax declares."""

    def case(self, name: str) -> pydicom.Dataset:
        return pydicom.dcmread(str(CORPUS / "syntax" / name))

    def frame(self, ds: pydicom.Dataset) -> bytes:
        from pydicom.encaps import generate_frames
        return next(generate_frames(ds.PixelData, number_of_frames=1))

    def test_every_registry_syntax_has_a_case(self) -> None:
        """Read out of the written files, not out of the generator's table.
        The registry list itself lives in corpus_check, which is where the
        coverage gate reads it from, so there is one of it."""
        declared = {str(self.case(name).file_meta.TransferSyntaxUID)
                    for name in corpus_synth.SYNTAX_CASES}
        self.assertEqual(declared, set(corpus_check.REGISTRY_TRANSFER_SYNTAXES))

    def test_declared_syntax_matches_the_file(self) -> None:
        for name, expected in corpus_synth.SYNTAX_CASES.items():
            with self.subTest(case=name):
                ds = self.case(name)
                self.assertEqual(str(ds.file_meta.TransferSyntaxUID), expected)

    def test_encapsulated_pixel_data_is_ob_with_undefined_length(self) -> None:
        """PS3.5 Table 7.1-1 and A.4: encapsulated PixelData (7FE0,0010) is OB
        with an undefined length, never OW.

        This is exactly the class of defect this corpus exists to avoid
        shipping: the file decodes fine through a reader that trusts the
        transfer syntax and is rejected or warned about by one that reads the
        VR, and nothing about the pixels looks wrong either way.
        """
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid in corpus_synth.NATIVE_TRANSFER_SYNTAXES:
                continue
            with self.subTest(case=name):
                element = self.case(name)["PixelData"]
                self.assertEqual(str(element.VR), "OB")
                self.assertTrue(element.is_undefined_length)

    def test_jpeg_family_codestreams_start_with_soi(self) -> None:
        """ISO/IEC 10918-1 B.1.1.3: every JPEG interchange stream starts FFD8."""
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid not in ("1.2.840.10008.1.2.4.50", "1.2.840.10008.1.2.4.51",
                           "1.2.840.10008.1.2.4.57", "1.2.840.10008.1.2.4.70",
                           "1.2.840.10008.1.2.4.80", "1.2.840.10008.1.2.4.81"):
                continue
            with self.subTest(case=name):
                self.assertEqual(self.frame(self.case(name))[:2], b"\xff\xd8")

    def test_jpeg_ls_codestreams_carry_sof55(self) -> None:
        """ISO/IEC 14495-1: JPEG-LS uses SOF55, marker FFF7, not a DCT SOF."""
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid not in ("1.2.840.10008.1.2.4.80", "1.2.840.10008.1.2.4.81"):
                continue
            with self.subTest(case=name):
                self.assertIn(b"\xff\xf7", self.frame(self.case(name))[:64])

    def test_jpeg_2000_codestreams_start_with_soc_siz(self) -> None:
        """ISO/IEC 15444-1 A.4.1 and A.5.1: SOC (FF4F) then SIZ (FF51)."""
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if not uid.startswith("1.2.840.10008.1.2.4.9") and \
               not uid.startswith("1.2.840.10008.1.2.4.20"):
                continue
            with self.subTest(case=name):
                self.assertEqual(self.frame(self.case(name))[:4],
                                 b"\xff\x4f\xff\x51")

    def test_htj2k_codestreams_carry_the_cap_marker(self) -> None:
        """ISO/IEC 15444-15: an HTJ2K codestream signals Part 15 in CAP (FF50),
        and sets the extended-capabilities bit 0x4000 in SIZ Rsiz. A plain
        JPEG 2000 codestream has neither, which is what makes this a check
        rather than a restatement of the filename."""
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid not in ("1.2.840.10008.1.2.4.201", "1.2.840.10008.1.2.4.202",
                           "1.2.840.10008.1.2.4.203"):
                continue
            with self.subTest(case=name):
                data = self.frame(self.case(name))
                rsiz = struct.unpack(">H", data[6:8])[0]
                self.assertTrue(rsiz & 0x4000,
                                f"SIZ Rsiz is 0x{rsiz:04x}, extended "
                                f"capabilities bit not set")
                self.assertIn(b"\xff\x50", data[:96])

    def test_htj2k_progression_order_matches_the_syntax(self) -> None:
        """PS3.5 A.4.10: the .202 syntax is the RPCL-constrained one, so its
        COD progression order byte must be 2. The .201 case is written LRCP so
        that the two are distinguishable rather than nominally different."""
        expected = {"htj2k_lossless.dcm": 0,        # LRCP
                    "htj2k_lossless_rpcl.dcm": 2,   # RPCL
                    "htj2k_lossy.dcm": 2,           # RPCL
                    "htj2k_lossless_ybr_rct.dcm": 0,   # LRCP
                    "htj2k_lossy_ybr_ict.dcm": 2}      # RPCL
        for name, order in expected.items():
            with self.subTest(case=name):
                data = self.frame(self.case(name))
                index = data.index(b"\xff\x52")     # COD
                self.assertEqual(data[index + 5], order)

    def test_rle_segment_header_is_well_formed(self) -> None:
        """PS3.5 Annex G: the RLE header is 64 bytes and its first value is the
        segment count.

        A 16-bit sample is split into one segment per BYTE, most significant
        first, so a single-sample-per-pixel 16-bit frame has TWO segments, not
        one. A decoder that assumes one segment per sample reads the high byte
        plane as a whole image and produces a picture of the right shape.
        """
        ds = self.case("rle_lossless.dcm")
        data = self.frame(ds)
        self.assertGreaterEqual(len(data), 64)
        expected = ds.SamplesPerPixel * (ds.BitsAllocated // 8)
        self.assertEqual(expected, 2)
        self.assertEqual(struct.unpack("<L", data[:4])[0], expected)

    def test_jpegxl_codestreams_are_jpeg_xl(self) -> None:
        """ISO/IEC 18181-2. A bare codestream starts FF 0A.

        An ISO BMFF container has the box type JXL at byte 4. imagecodecs
        2026.10.10 writes that container for the 16-bit lossless encode and
        for jpegxl_encode_jpeg even when usecontainer is false. The other
        three rows are bare codestreams.
        """
        boxed = {"jpegxl_lossless.dcm", "jpegxl_recompression_rgb8.dcm"}
        # F-244. An empty-table row carries its source's codestream.
        boxed |= {name for name, source in corpus_synth.EMPTY_BOT_CASES.items()
                  if source in boxed}
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid not in (corpus_synth.JPEGXL_LOSSLESS,
                           corpus_synth.JPEGXL_RECOMPRESSION,
                           corpus_synth.JPEGXL_GENERAL):
                continue
            with self.subTest(case=name):
                data = self.frame(self.case(name))
                if name in boxed:
                    self.assertEqual(data[4:8], b"JXL ")
                else:
                    self.assertEqual(data[:2], b"\xff\x0a")

    def test_lossless_syntaxes_round_trip_exactly(self) -> None:
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid in corpus_synth.LOSSY_TRANSFER_SYNTAXES:
                continue
            # `.111` is the unavailable refusal. A pixel compare would
            # claim a decode the registry does not perform.
            if uid == corpus_synth.JPEGXL_RECOMPRESSION:
                continue
            with self.subTest(case=name):
                ds = self.case(name)
                reference = pydicom.dcmread(
                    str(CORPUS / "syntax" / corpus_synth.SYNTAX_REFERENCE[name]))
                want = reference.pixel_array
                if name in corpus_synth.MULTIFRAME_EMPTY_BOT:
                    want = corpus_synth.multiframe_reference(want)
                self.assertTrue(
                    np.array_equal(read_back(ds, name), want),
                    f"{name} declares a lossless syntax and did not round-trip")

    def test_jpegxl_recompression_is_unavailable(self) -> None:
        """`.111` is known and the read-back route refuses it.

        jxl-jbr 0.2.1 src/reconstruct.rs:457 panics on a malformed DHT, so
        the corpus row expects the unavailable refusal rather than pixels.
        """
        name = "jpegxl_recompression_rgb8.dcm"
        self.assertEqual(READ_BACK[name], "unavailable")
        self.assertIn(
            corpus_synth.JPEGXL_RECOMPRESSION,
            corpus_check.UNAVAILABLE_REGISTRY_SYNTAXES)
        self.assertEqual(
            self.case(name).file_meta.TransferSyntaxUID,
            corpus_synth.JPEGXL_RECOMPRESSION)
        with self.assertRaises(UnavailableSyntax) as caught:
            read_back(self.case(name), name)
        message = str(caught.exception)
        self.assertIn(corpus_check.UNAVAILABLE_REFUSAL, message)
        self.assertIn("jxl-jbr 0.2.1", message)
        self.assertIn("reconstruct.rs:457", message)

    def test_jpeg_ls_near_lossless_holds_its_declared_near_bound(self) -> None:
        """ISO/IEC 14495-1 guarantees max abs error <= NEAR. This is the one
        lossy syntax with a bound the standard actually promises, so every
        `.81` case is asserted exactly rather than as a sanity range."""
        names = [name for name, uid in corpus_synth.SYNTAX_CASES.items()
                 if uid == "1.2.840.10008.1.2.4.81"]
        self.assertEqual(sorted(names), ["jpegls_banded_near_lossless.dcm",
                                         "jpegls_near_lossless.dcm",
                                         "jpegls_near_lossless_empty_bot.dcm"])
        for name in names:
            with self.subTest(case=name):
                ds = self.case(name)
                reference = pydicom.dcmread(
                    str(CORPUS / "syntax" / corpus_synth.SYNTAX_REFERENCE[name]))
                error = np.abs(read_back(ds, name).astype(np.int64)
                               - reference.pixel_array.astype(np.int64))
                self.assertLessEqual(int(error.max()), corpus_synth.JPEG_LS_NEAR)

    def test_the_banded_reference_is_the_two_band_formula(self) -> None:
        """F-243. s(r, c) = 224 if c < 16 else 1064 over 32 by 32, typed in
        here rather than read from the generator, so the reference the banded
        JPEG-LS rows decode against is the formula and not whatever the
        generator wrote."""
        ds = self.case("reference_banded16.dcm")
        self.assertEqual(str(ds.file_meta.TransferSyntaxUID),
                         "1.2.840.10008.1.2.1")
        self.assertEqual((ds.Rows, ds.Columns, ds.BitsAllocated,
                          ds.BitsStored, ds.PixelRepresentation),
                         (32, 32, 16, 16, 0))
        expected = np.array([[224 if column < 16 else 1064
                              for column in range(32)] for _ in range(32)],
                            dtype=np.uint16)
        self.assertTrue(np.array_equal(ds.pixel_array, expected))

    def test_lossy_syntaxes_decode_to_the_right_shape_and_stay_close(self) -> None:
        """No standard promises a bound for DCT or irreversible wavelet, so
        this is a sanity check that the case is the image it claims to be and
        not a different one. It is NOT the tolerance policy, which lives in
        HLD section 25.1 and applies to the oracle, not to the corpus.

        Bounded on the WORST pixel, not the mean, because a mean averages a
        badly wrong region away and that is the defect class this repository
        exists to distrust. Section 25.1 states its own tolerances the same
        way, as a maximum difference and a cap on outliers.
        """
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid not in corpus_synth.LOSSY_TRANSFER_SYNTAXES:
                continue
            if uid == "1.2.840.10008.1.2.4.81":
                continue    # covered exactly by the NEAR bound above
            with self.subTest(case=name):
                ds = self.case(name)
                reference = pydicom.dcmread(
                    str(CORPUS / "syntax" / corpus_synth.SYNTAX_REFERENCE[name]))
                got, want = read_back(ds, name), reference.pixel_array
                self.assertEqual(got.shape, want.shape)
                full_scale = float(1 << ds.BitsStored) - 1.0
                error = np.abs(got.astype(np.float64) - want.astype(np.float64))
                if ds.PhotometricInterpretation == "XYB":
                    self.assertGreater(float(error.max()), 0.0)
                    self.assertLessEqual(float(error.max()), JPEGXL_XYB_MAX_ABS,
                                         f"{name} exceeded the recorded "
                                         f"JPEG XL ceiling")
                    continue
                bound = (YBR_EIGHT_BIT_MAX
                         if ds.PhotometricInterpretation.startswith("YBR")
                         else TRANSFORM_MAX)
                self.assertLessEqual(float(error.max()) / full_scale, bound,
                                     f"{name} decoded to a materially "
                                     f"different image")

    def test_lossy_cases_declare_their_lossiness(self) -> None:
        """PS3.3 C.7.6.1.1.5. Asserted in both directions, and the absence on
        the lossless cases is the half that catches an attribute set too
        widely."""
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            with self.subTest(case=name):
                ds = self.case(name)
                lossy = uid in corpus_synth.LOSSY_TRANSFER_SYNTAXES
                self.assertEqual("LossyImageCompression" in ds, lossy)
                self.assertEqual("LossyImageCompressionMethod" in ds, lossy)
                if lossy:
                    self.assertEqual(ds.LossyImageCompression, "01")
                    self.assertEqual(ds.LossyImageCompressionMethod,
                                     self.codestream_method(self.frame(ds)))

    def codestream_method(self, frame: bytes) -> str:
        """The PS3.3 C.7.6.1.1.5.1 Defined Term the bytes actually are.

        Derived from the codestream rather than from the generator, because a
        term read back from the code that wrote it proves only that the writer
        agrees with itself, and this attribute is the one that says which
        algorithm ran.
        """
        if frame[:2] == b"\xff\xd8":                     # ISO/IEC 10918-1 SOI
            return ("ISO_14495_1" if b"\xff\xf7" in frame[:64]  # SOF55
                    else "ISO_10918_1")
        if frame[:4] == b"\xff\x4f\xff\x51":              # SOC then SIZ
            rsiz = struct.unpack(">H", frame[6:8])[0]
            return "ISO_15444_15" if rsiz & 0x4000 else "ISO_15444_1"
        # ISO/IEC 18181-1. PS3.3 C.7.6.1.1.5.1 Defined Term ISO_18181_1,
        # Supplement 232. A bare codestream starts FF 0A. An ISO BMFF
        # container carries the box type JXL at byte 4.
        if frame[:2] == b"\xff\x0a" or frame[4:8] == b"JXL ":
            return "ISO_18181_1"
        self.fail(f"unrecognised codestream {frame[:4].hex()}")

    def test_the_classifier_refuses_a_codestream_it_cannot_name(self) -> None:
        """The fall-through is unreachable for the cases shipped today, so
        nothing else reaches it. A future lossy case whose codestream this
        cannot parse must not be quietly given a name."""
        with self.assertRaises(self.failureException):
            self.codestream_method(b"\x00\x01\x02\x03")


class ReaderRoutes(unittest.TestCase):
    """Every syntax case names one decoder, and the decoder version is pinned."""

    def test_every_syntax_has_one_declared_decoder(self) -> None:
        self.assertEqual(set(READ_BACK), set(corpus_synth.SYNTAX_CASES))
        allowed = {"native", "pylibjpeg", "pyjpegls", "ljpeg", "jpeg8",
                   "jpeg8-rgb", "jpegxl", "unavailable"}
        for name, route in READ_BACK.items():
            with self.subTest(case=name):
                self.assertIn(route, allowed)
                self.assertNotIn("gdcm", route)
                self.assertNotIn("pylibjpeg-libjpeg", route)
        self.assertEqual(READ_BACK["jpeg_lossless_p14.dcm"], "ljpeg")
        self.assertEqual(READ_BACK["jpeg_lossless_p14_sv1.dcm"], "ljpeg")
        self.assertEqual(READ_BACK["jpegls_lossless.dcm"], "pyjpegls")
        self.assertEqual(READ_BACK["jpegls_lossless_rgb8.dcm"], "pyjpegls")
        self.assertEqual(READ_BACK["jpegls_near_lossless.dcm"], "pyjpegls")
        self.assertEqual(READ_BACK["jpegls_banded_lossless.dcm"], "pyjpegls")
        self.assertEqual(READ_BACK["jpegls_banded_near_lossless.dcm"], "pyjpegls")
        self.assertEqual(READ_BACK["jpeg_baseline_rgb8.dcm"], "jpeg8-rgb")
        self.assertEqual(READ_BACK["jpeg_extended_12.dcm"], "jpeg8")
        self.assertEqual(READ_BACK["j2k_lossy.dcm"], "pylibjpeg")
        self.assertEqual(READ_BACK["htj2k_lossy.dcm"], "pylibjpeg")
        self.assertEqual(READ_BACK["rle_lossless.dcm"], "pylibjpeg")
        self.assertEqual(READ_BACK["jpegxl_lossless.dcm"], "jpegxl")
        self.assertEqual(READ_BACK["jpegxl_lossless_rgb8.dcm"], "jpegxl")
        self.assertEqual(READ_BACK["jpegxl_recompression_rgb8.dcm"], "unavailable")
        self.assertEqual(READ_BACK["jpegxl_lossy.dcm"], "jpegxl")
        self.assertEqual(READ_BACK["jpegxl_xyb_rgb8.dcm"], "jpegxl")

    def test_reader_versions_match_decoded_with(self) -> None:
        import imagecodecs
        self.assertEqual(imagecodecs.__version__, DECODED_WITH["imagecodecs"])
        self.assertEqual(imagecodecs.jpeg8_version(),
                         DECODED_WITH["jpeg8_version"])


class EncoderProvenance(unittest.TestCase):
    """Which encoder owns which syntax, and which of them leave a version.

    `corpus_synth.EXTERNAL_ENCODERS` is keyed by transfer syntax and the case
    names are derived from `SYNTAX_CASES`, so an attribution can only be
    changed by moving a syntax between producers, which these tests catch.
    """

    # A stamp any of these encoders might leave. Written out here rather than
    # imported, so that a producer claiming to leave nothing is checked against
    # a list this file controls. Case insensitive, so a future lowercase
    # variant does not slip past the "leaves none" assertion.
    STAMPS = (rb"OpenJPH Ver [0-9.]+", rb"Created by OpenJPEG version [0-9.]+",
              rb"DCMTK", rb"OFFIS", rb"CharLS", rb"pyjpegls")

    def raw(self, name: str) -> bytes:
        return (CORPUS / "syntax" / name).read_bytes()

    def compressed(self) -> set[str]:
        return {name for name, uid in corpus_synth.SYNTAX_CASES.items()
                if uid not in corpus_synth.NATIVE_TRANSFER_SYNTAXES}

    def internal_case(self) -> str:
        # F-244's empty-table rows reuse or re-run the same encoder. The
        # case whose bytes the plugins must reproduce is the source row.
        found = [name for name, uid in corpus_synth.SYNTAX_CASES.items()
                 if uid == corpus_synth.INTERNAL_ENCODER_SYNTAX
                 and name not in corpus_synth.EMPTY_BOT_CASES]
        self.assertEqual(len(found), 1)
        return found[0]

    def test_every_compressed_syntax_has_exactly_one_declared_producer(self) -> None:
        claimed: list[str] = []
        for syntaxes, _ in corpus_synth.EXTERNAL_ENCODERS.values():
            claimed += list(syntaxes)
        self.assertEqual(len(claimed), len(set(claimed)),
                         "a syntax is claimed by two encoders")
        declared = {corpus_synth.SYNTAX_CASES[name] for name in self.compressed()}
        self.assertEqual(set(claimed) | {corpus_synth.INTERNAL_ENCODER_SYNTAX},
                         declared,
                         "the producer table and the compressed cases disagree")

    def test_a_producer_that_claims_a_version_leaves_one(self) -> None:
        for producer, (_, pattern) in corpus_synth.EXTERNAL_ENCODERS.items():
            if pattern is None:
                continue
            for name in corpus_synth.cases_for(producer):
                with self.subTest(producer=producer, case=name):
                    # Not assertRegex: that prints the whole DICOM object on
                    # failure, which is unreadable at 16 KB a subtest.
                    self.assertIsNotNone(
                        re.search(pattern, self.raw(name)),
                        f"{name} carries no {pattern!r} for {producer}")

    def test_a_producer_that_claims_no_version_leaves_none(self) -> None:
        """The cases that carry nothing are the reason the versions are written
        down at all, so "carries nothing" is the assertion that matters."""
        for producer, (_, pattern) in corpus_synth.EXTERNAL_ENCODERS.items():
            if pattern is not None:
                continue
            for name in corpus_synth.cases_for(producer):
                with self.subTest(producer=producer, case=name):
                    raw = self.raw(name)
                    found = [s for s in self.STAMPS
                             if re.search(s, raw, re.IGNORECASE)]
                    self.assertEqual(found, [], f"{name} carries {found}")

    def test_the_stampless_producers_are_told_apart_by_the_fingerprint(self) -> None:
        """DCMTK and pyjpegls both leave no version, so nothing above can tell
        their syntaxes apart and the two sets could be exchanged unnoticed.

        DCMTK writes a DerivationDescription on every case it encodes and no
        other producer here writes one, so its presence is exactly the
        discriminator. Asserted both ways: on all of DCMTK's cases and on none
        of the others.
        """
        mine = set(corpus_synth.cases_for("DCMTK, dcmcjpeg"))
        self.assertTrue(mine)
        for name in sorted(self.compressed()):
            with self.subTest(case=name):
                ds = pydicom.dcmread(str(CORPUS / "syntax" / name),
                                     stop_before_pixels=True)
                self.assertEqual(corpus_synth.DCMTK_FINGERPRINT in ds,
                                 name in mine)

    def test_the_internally_encoded_case_is_plugin_independent(self) -> None:
        """The one compressed case that is not external encoder output. Every
        plugin pydicom offers for it must produce the same bytes, or it belongs
        in the table above instead."""
        from pydicom.pixels.encoders import RLELosslessEncoder
        reference = CORPUS / "syntax" / corpus_synth.REFERENCE_MONO16
        want = bytes(pydicom.dcmread(
            str(CORPUS / "syntax" / self.internal_case())).PixelData)
        plugins = RLELosslessEncoder.available_plugins
        self.assertGreaterEqual(len(plugins), 1)
        for plugin in plugins:
            with self.subTest(plugin=plugin):
                ds = pydicom.dcmread(str(reference))
                ds.compress(pydicom.uid.RLELossless, encoding_plugin=plugin)
                self.assertEqual(bytes(ds.PixelData), want)


class ToolchainPins(unittest.TestCase):
    """`BUILT_WITH` and the uv project pins are the same versions twice.

    The README's copy was deleted, because a reader can be sent to the source.
    CI installs the checked-in lockfile, so the project metadata and generator
    table are joined here instead. This is the only remaining way the pair can
    drift without something noticing.
    """

    WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
    PROJECT = ROOT / "pyproject.toml"

    def workflow(self) -> str:
        return self.WORKFLOW.read_text(encoding="utf-8")

    # BUILT_WITH entries uv does not install directly. OpenJPH and DCMTK have
    # a test below each. OpenJPEG has none, because it ships inside the pinned
    # pylibjpeg-openjpeg wheel. The opj_compress entry is that project's CLI
    # binary, which encodes the F-239 colour rows and is installed beside
    # DCMTK, not by uv. libjxl ships inside imagecodecs and encodes the JPEG
    # XL rows. Everything else must be a project pin.
    NOT_UV = ("OpenJPEG (the library inside it)", "OpenJPEG (opj_compress CLI)",
              "DCMTK", "OpenJPH", "libjxl (the library inside imagecodecs)")

    def test_uv_pins_agree_with_built_with(self) -> None:
        project = tomllib.loads(self.PROJECT.read_text(encoding="utf-8"))
        pinned = dict(re.findall(
            r"^([A-Za-z0-9_-]+)==([0-9][0-9A-Za-z.]*)$",
            "\n".join(project["project"]["dependencies"]), re.MULTILINE,
        ))
        expected = set(corpus_synth.BUILT_WITH) - set(self.NOT_UV)
        self.assertEqual(set(pinned) & set(corpus_synth.BUILT_WITH), expected,
                         "a pin was dropped, or BUILT_WITH gained an entry CI "
                         "does not install")
        for name in sorted(expected):
            with self.subTest(package=name):
                self.assertEqual(pinned[name], corpus_synth.BUILT_WITH[name])

        self.assertIn("uv sync --locked", self.workflow())
        self.assertTrue((ROOT / "uv.lock").is_file())

    def test_ci_pins_openjph_to_the_version_it_was_built_with(self) -> None:
        tag = re.search(r"--branch\s+([0-9][0-9A-Za-z.]*)", self.workflow())
        self.assertIsNotNone(tag, "the CI OpenJPH clone lost its --branch pin")
        self.assertEqual(tag.group(1), corpus_synth.BUILT_WITH["OpenJPH"])

    def test_ci_does_not_pin_dcmtk(self) -> None:
        """The documented exception. `ci.yml` says DCMTK comes unpinned from
        the distribution and explains what follows, so a pin appearing later
        would leave that comment describing something that is not there."""
        install = re.search(r"apt-get install[^\n]*", self.workflow())
        self.assertIsNotNone(install)
        # A bare token. An apt pin is `dcmtk=3.6.7-9.1build4`, one token with
        # one equals sign, which `dcmtk==` would not have caught either.
        self.assertIn("dcmtk", install.group(0).split())


def encapsulated_items(ds: "pydicom.Dataset") -> list[bytes]:
    """PS3.5 A.4 Items, read here rather than through the generator.

    Each Item is Tag (FFFE,E000), a 4-byte little-endian length and that many
    bytes. The first is the Basic Offset Table. pydicom drops the Sequence
    Delimitation Item (FFFE,E0DD) from the value it keeps.
    """
    value, items, at = bytes(ds.PixelData), [], 0
    while at < len(value):
        tag = value[at:at + 4]
        if tag == b"\xfe\xff\xdd\xe0":
            break
        if tag != b"\xfe\xff\x00\xe0":
            raise AssertionError(f"Item Tag expected, found {tag.hex()}")
        (length,) = struct.unpack("<I", value[at + 4:at + 8])
        items.append(value[at + 8:at + 8 + length])
        at += 8 + length
    return items


# PS3.4 B.5, Enhanced CT Image Storage. Its IOD, PS3.3 A.38, carries the
# Multi-frame Functional Groups Module whose Number of Frames is Type 1.
ENHANCED_CT = "1.2.840.10008.5.1.4.1.1.2.1"


class EmptyBasicOffsetTable(unittest.TestCase):
    """F-244, PS3.5 A.4 and 8.2, and PS3.3 C.7.6.3.1.8.

    Expected values come from the standard and from the source rows, which
    carry populated tables, never from the generator's own helpers.
    """

    def case(self, name: str) -> pydicom.Dataset:
        return pydicom.dcmread(str(CORPUS / "syntax" / name))

    def names(self) -> list[str]:
        return sorted(corpus_synth.EMPTY_BOT_CASES)

    def test_nineteen_rows_one_per_decodable_syntax_and_five_shapes(self) -> None:
        self.assertEqual(len(corpus_synth.EMPTY_BOT_SOURCES), 14)
        self.assertEqual(len(corpus_synth.EMPTY_BOT_SHAPES), 5)
        syntaxes = {corpus_synth.SYNTAX_CASES[name]
                    for name in corpus_synth.EMPTY_BOT_SOURCES}
        encapsulated = (set(corpus_check.REGISTRY_TRANSFER_SYNTAXES)
                        - corpus_synth.NATIVE_TRANSFER_SYNTAXES
                        - set(corpus_check.UNAVAILABLE_REGISTRY_SYNTAXES))
        self.assertEqual(syntaxes, encapsulated)

    def test_the_first_item_is_an_empty_table(self) -> None:
        """Table A.4-1: the Basic Offset Table Item has length 00000000H."""
        for name in self.names():
            with self.subTest(case=name):
                items = encapsulated_items(self.case(name))
                self.assertEqual(items[0], b"")
                self.assertGreaterEqual(len(items), 2)
                for fragment in items[1:]:
                    # A.4: "an even number of bytes greater or equal to two".
                    self.assertGreaterEqual(len(fragment), 2)
                    self.assertEqual(len(fragment) % 2, 0)

    def test_single_frame_rows_keep_their_source_fragment(self) -> None:
        for name, source in corpus_synth.EMPTY_BOT_SOURCES.items():
            with self.subTest(case=name):
                source_items = encapsulated_items(self.case(source))
                # The source carries A.4's one-frame table, a single 0.
                self.assertEqual(source_items[0], b"\x00" * 4)
                self.assertEqual(encapsulated_items(self.case(name))[1:],
                                 source_items[1:])

    def test_every_row_has_its_own_instance(self) -> None:
        for name, source in corpus_synth.EMPTY_BOT_CASES.items():
            with self.subTest(case=name):
                ds, original = self.case(name), self.case(source)
                self.assertNotEqual(ds.SOPInstanceUID, original.SOPInstanceUID)
                self.assertEqual(ds.file_meta.TransferSyntaxUID,
                                 original.file_meta.TransferSyntaxUID)

    def test_single_frame_decodes_equal_their_source(self) -> None:
        for name, source in corpus_synth.EMPTY_BOT_CASES.items():
            if name in corpus_synth.MULTIFRAME_EMPTY_BOT:
                continue
            with self.subTest(case=name):
                self.assertEqual(
                    decode_array_sha256(read_back(self.case(name), name)),
                    decode_array_sha256(read_back(self.case(source), source)))

    def test_fragmented_rows_start_a_fragment_at_2_mod_4(self) -> None:
        """dicom-object 0.10.0 refused a first Fragment of length 2 mod 4."""
        jls = encapsulated_items(
            self.case("jpegls_lossless_fragmented_empty_bot.dcm"))[1:]
        self.assertEqual(len(jls), 3)
        self.assertEqual(len(jls[0]) % 4, 2)
        [whole] = encapsulated_items(self.case("jpegls_lossless.dcm"))[1:]
        self.assertEqual(b"".join(jls), whole)
        for name in ("jpeg_lossless_p14_sv1_multiframe_fragmented_empty_bot.dcm",
                     "j2k_lossless_multiframe_fragmented_empty_bot.dcm"):
            with self.subTest(case=name):
                fragments = encapsulated_items(self.case(name))[1:]
                self.assertEqual(len(fragments), 6)
                for first in fragments[0::2]:
                    self.assertEqual(len(first) % 4, 2)
                    # ISO/IEC 10918-1 SOI or ISO/IEC 15444-1 SOC opens a frame.
                    self.assertIn(first[:2], (b"\xff\xd8", b"\xff\x4f"))
                    # No end marker where pydicom or cornerstone3D would end
                    # the frame early.
                    self.assertNotIn(b"\xff\xd9", first[-10:])
                for last in fragments[1::2]:
                    self.assertIn(b"\xff\xd9", last[-3:])

    def test_multiframe_rows_decode_to_three_distinct_frames_in_order(self) -> None:
        reference = self.case(corpus_synth.REFERENCE_MONO16).pixel_array
        expected = [reference, np.fliplr(reference), np.flipud(reference)]
        self.assertFalse(np.array_equal(expected[0], expected[1]))
        self.assertFalse(np.array_equal(expected[0], expected[2]))
        self.assertFalse(np.array_equal(expected[1], expected[2]))
        fragment_count = {
            "rle_multiframe_empty_bot.dcm": 3,
            "jpeg_lossless_p14_sv1_multiframe_fragmented_empty_bot.dcm": 6,
            "j2k_lossless_multiframe_fragmented_empty_bot.dcm": 6,
            "j2k_lossless_multiframe_eot.dcm": 3,
        }
        self.assertEqual(set(fragment_count), corpus_synth.MULTIFRAME_EMPTY_BOT)
        for name, count in fragment_count.items():
            with self.subTest(case=name):
                ds = self.case(name)
                self.assertEqual(ds.SOPClassUID, ENHANCED_CT)
                self.assertEqual(int(ds.NumberOfFrames), 3)
                self.assertEqual(len(encapsulated_items(ds)) - 1, count)
                decoded = read_back(ds, name)
                self.assertEqual(decoded.shape, (3, *reference.shape))
                for frame, want in enumerate(expected):
                    self.assertTrue(np.array_equal(decoded[frame], want),
                                    f"frame {frame}")

    def test_the_extended_offset_table_points_at_each_fragment(self) -> None:
        """PS3.3 C.7.6.3.1.8: offsets to each frame's one Fragment Item Tag,
        from the first Item Tag after the empty table, first entry 0, and
        Lengths of each frame's encoded bytes."""
        ds = self.case("j2k_lossless_multiframe_eot.dcm")
        fragments = encapsulated_items(ds)[1:]
        offsets = struct.unpack(f"<{len(fragments)}Q", ds.ExtendedOffsetTable)
        lengths = struct.unpack(f"<{len(fragments)}Q",
                                ds.ExtendedOffsetTableLengths)
        expected, at = [], 0
        for fragment in fragments:
            expected.append(at)
            at += 8 + len(fragment)
        self.assertEqual(list(offsets), expected)
        self.assertEqual(offsets[0], 0)
        for fragment, length in zip(fragments, lengths, strict=True):
            self.assertIn(len(fragment) - length, (0, 1))
            self.assertEqual(len(fragment) - length, length % 2)


class EmptyBotGeneratorRefusals(unittest.TestCase):
    """F-244's eight generator refusals, each driven directly.

    A sandbox probe cannot reach them, because this module imports pydicom
    at module scope and `.venv` is not tracked, which is the import reason the
    catalogue's `corpus-synth` entry gives for every refusal in this file.
    """

    def test_a_non_item_tag_is_refused(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "non-Item tag"):
            corpus_synth.pixel_items(b"\x08\x00\x16\x00\x00\x00\x00\x00")

    def test_an_odd_item_value_is_refused(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "even length"):
            corpus_synth.item(b"\x01")

    def test_a_frame_with_no_marker_free_split_is_refused(self) -> None:
        with self.assertRaisesRegex(RuntimeError, "avoids an end marker"):
            corpus_synth.split_offset(b"\xff\xd9" * 32)

    def test_an_unsupported_fragment_count_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError, "Fragment count 4"):
            corpus_synth.fragment_split(b"\x00" * 64, 4)

    def test_a_source_with_two_fragments_is_refused(self) -> None:
        with mock.patch.object(corpus_synth, "pixel_items",
                               return_value=[b"", b"ab", b"cd"]), \
             self.assertRaisesRegex(RuntimeError, "does not hold one Fragment"):
            corpus_synth.encode_empty_bot_cases(CORPUS / "syntax")

    def test_a_producer_writing_the_wrong_frame_count_is_refused(self) -> None:
        with mock.patch("pydicom.encaps.generate_frames",
                        return_value=iter([b"ab"])), \
             self.assertRaisesRegex(RuntimeError, "wrote 1 frames"):
            corpus_synth.encode_multiframe(CORPUS / "syntax",
                                           "rle_lossless.dcm", "probe")

    def test_a_populated_table_beside_an_extended_table_is_refused(self) -> None:
        shapes = {"probe.dcm": ("rle_lossless.dcm",
                                "multiframe-extended-offset-table")}
        populated = corpus_synth.item(b"\x00" * 4) + corpus_synth.item(b"ab")
        with mock.patch.object(corpus_synth, "EMPTY_BOT_SOURCES", {}), \
             mock.patch.object(corpus_synth, "EMPTY_BOT_SHAPES", shapes), \
             mock.patch("pydicom.encaps.encapsulate_extended",
                        return_value=(populated, b"", b"")), \
             self.assertRaisesRegex(RuntimeError, "populated table"):
            corpus_synth.encode_empty_bot_cases(CORPUS / "syntax")

    def test_an_unknown_shape_is_refused(self) -> None:
        shapes = {"probe.dcm": ("rle_lossless.dcm", "nonsense")}
        with mock.patch.object(corpus_synth, "EMPTY_BOT_SOURCES", {}), \
             mock.patch.object(corpus_synth, "EMPTY_BOT_SHAPES", shapes), \
             self.assertRaisesRegex(RuntimeError, "unknown shape"):
            corpus_synth.encode_empty_bot_cases(CORPUS / "syntax")
        self.assertFalse((CORPUS / "syntax" / "probe.dcm").exists())


class SyntaxSeriesShape(unittest.TestCase):
    """The transfer-syntax cases must not form one degenerate series."""

    def read_all(self) -> dict[str, pydicom.Dataset]:
        return {path.name: pydicom.dcmread(str(path), stop_before_pixels=True)
                for path in sorted((CORPUS / "syntax").glob("*.dcm"))}

    def test_each_case_is_its_own_series_and_spatial_frame(self) -> None:
        """Inheriting the base's SeriesInstanceUID puts every mono16 case in
        one series: instances sharing a frame of reference, all at
        ImagePositionPatient [0, 0, 0], all InstanceNumber 1, declaring
        different transfer syntaxes each. No scanner produces that, and every
        rule in PS3.3 C.7.6.2 volume construction fires against it. The oracle
        pushes "the same study through both stacks" (HLD section 11), so a
        consumer grouping by series meets duplicate slice positions.
        """
        datasets = self.read_all()
        self.assertGreater(len(datasets), 1)
        for keyword in ("SeriesInstanceUID", "FrameOfReferenceUID"):
            with self.subTest(attribute=keyword):
                grouped: dict[str, list[str]] = {}
                for name, ds in datasets.items():
                    grouped.setdefault(str(getattr(ds, keyword)), []).append(name)
                shared = {uid: names for uid, names in grouped.items()
                          if len(names) > 1}
                self.assertEqual(shared, {})

    def test_they_stay_one_study(self) -> None:
        """The grouping that IS intended, so a reader can browse `syntax/` as
        one thing. Asserted so that fixing the series does not quietly scatter
        the study too."""
        studies = {str(ds.StudyInstanceUID) for ds in self.read_all().values()}
        self.assertEqual(len(studies), 1)


class SyntheticTraps(unittest.TestCase):
    """Each trap case carries the attribute combination it exists for."""

    def read(self, *parts: str) -> pydicom.Dataset:
        return pydicom.dcmread(str(CORPUS.joinpath("synthetic", *parts)))

    def test_monochrome1_case_is_monochrome1(self) -> None:
        ds = self.read("cr_monochrome1.dcm")
        self.assertEqual(ds.PhotometricInterpretation, "MONOCHROME1")

    def test_unsigned_ct_declares_presentation_inversion(self) -> None:
        """PS3.3 C.11.6.1.2.2, INVERSE reverses the presentation output."""
        ds = self.read("ct_unsigned_16.dcm")
        self.assertEqual(ds.PresentationLUTShape, "INVERSE")

    def test_pixel_spacing_is_non_square_where_it_should_be(self) -> None:
        ds = self.read("mr_nonsquare_spacing.dcm")
        self.assertEqual([float(v) for v in ds.PixelSpacing], [0.5, 0.25])
        self.assertNotEqual(ds.Rows, ds.Columns)

    def test_planar_pair_decodes_to_the_same_image(self) -> None:
        """PlanarConfiguration 0 and 1 are two layouts of one image. A reader
        that ignores (0028,0006) renders the second as colour noise."""
        interleaved = self.read("sc_rgb_interleaved.dcm")
        planar = self.read("sc_rgb_planar.dcm")
        self.assertEqual(interleaved.PlanarConfiguration, 0)
        self.assertEqual(planar.PlanarConfiguration, 1)
        self.assertTrue(np.array_equal(interleaved.pixel_array,
                                       planar.pixel_array))

    def test_uniform_series_spacing_is_uniform(self) -> None:
        gaps = self.projected_gaps("ct_series_uniform")
        self.assertEqual(len(gaps), 9)
        for gap in gaps:
            self.assertAlmostEqual(gap, 2.5, places=9)

    def test_nonuniform_series_has_one_gap_off_the_median(self) -> None:
        """The volume builder must refuse this rather than average it."""
        gaps = self.projected_gaps("ct_series_nonuniform")
        self.assertAlmostEqual(float(np.median(gaps)), 2.5, places=9)
        off = [g for g in gaps if abs(g - 2.5) > 1e-9]
        self.assertEqual(len(off), 2)
        self.assertAlmostEqual(max(off), 3.75, places=9)
        self.assertAlmostEqual(min(off), 1.25, places=9)

    def projected_gaps(self, series: str) -> list[float]:
        """Slice spacing from projected IPP, per PS3.3 C.7.6.2.1.1. Never from
        SpacingBetweenSlices, which is the tag this project must not trust."""
        paths = sorted((CORPUS / "synthetic" / series).glob("*.dcm"))
        sets = [pydicom.dcmread(str(p), stop_before_pixels=True) for p in paths]
        iop = [float(v) for v in sets[0].ImageOrientationPatient]
        normal = np.cross(np.array(iop[0:3]), np.array(iop[3:6]))
        normal = normal / np.linalg.norm(normal)
        projected = sorted(
            float(np.dot(np.array([float(v) for v in ds.ImagePositionPatient]),
                         normal)) for ds in sets)
        return [b - a for a, b in zip(projected, projected[1:])]

    def test_multiframe_carries_rescale_per_frame_and_not_at_top_level(self) -> None:
        """PS3.3 C.7.6.16: per-frame functional groups are consulted before
        shared. A legacy-shaped reader looks for a top-level RescaleSlope and
        must find nothing here rather than a value it can apply to all frames."""
        ds = self.read("ct_multiframe_perframe.dcm")
        self.assertNotIn("RescaleSlope", ds)
        self.assertNotIn("WindowCenter", ds)
        per_frame = ds.PerFrameFunctionalGroupsSequence
        self.assertEqual(len(per_frame), int(ds.NumberOfFrames))
        slopes = [float(f.PixelValueTransformationSequence[0].RescaleSlope)
                  for f in per_frame]
        centres = [float(f.FrameVOILUTSequence[0].WindowCenter)
                   for f in per_frame]
        self.assertEqual(len(set(slopes)), len(slopes))
        self.assertEqual(len(set(centres)), len(centres))

    def test_ybr_full_422_frame_is_two_bytes_per_pixel(self) -> None:
        """PS3.3 C.7.6.3.1.2: YBR_FULL_422 stores Y1 Y2 Cb Cr for each
        horizontal pair, so a frame is Rows * Columns * 2 bytes, not * 3."""
        ds = self.read("us_ybr_full_422.dcm")
        self.assertEqual(ds.PhotometricInterpretation, "YBR_FULL_422")
        self.assertEqual(ds.SamplesPerPixel, 3)
        self.assertEqual(ds.Columns % 2, 0)
        self.assertEqual(len(bytes(ds.PixelData)), ds.Rows * ds.Columns * 2)

    def test_palette_case_carries_a_non_zero_first_mapped_input(self) -> None:
        """PS3.3 C.7.6.3.1.5, the descriptor's second value.

        The offset is the trap. A first mapped input of zero makes it invisible,
        so the case would prove nothing about a reader that ignores the value.
        """
        ds = self.read("sc_palette_color.dcm")
        self.assertEqual(ds.PhotometricInterpretation, "PALETTE COLOR")
        self.assertEqual(ds.SamplesPerPixel, 1)
        for descriptor in (ds.RedPaletteColorLookupTableDescriptor,
                           ds.GreenPaletteColorLookupTableDescriptor,
                           ds.BluePaletteColorLookupTableDescriptor):
            self.assertEqual(list(descriptor), [256, 10, 16])
        # Some stored values are below the first mapped input, so the low clamp
        # is exercised by the frame rather than only by the descriptor.
        values = np.frombuffer(bytes(ds.PixelData), dtype=np.uint8)
        self.assertTrue(bool((values < 10).any()))
        self.assertTrue(bool((values >= 10).any()))

    def test_palette_16_declares_zero_meaning_65536_entries(self) -> None:
        """PS3.3 C.7.6.3.1.5: "When the number of table entries is equal to
        2^16 then this value shall be 0."

        65,536 does not fit in a US, so a full 16-bit palette has no other legal
        spelling. A reader that takes the 0 literally finds an empty table.
        """
        ds = self.read("sc_palette_color_16.dcm")
        self.assertEqual(ds.PhotometricInterpretation, "PALETTE COLOR")
        self.assertEqual(ds.BitsAllocated, 16)
        for descriptor, data in (
                (ds.RedPaletteColorLookupTableDescriptor,
                 ds.RedPaletteColorLookupTableData),
                (ds.GreenPaletteColorLookupTableDescriptor,
                 ds.GreenPaletteColorLookupTableData),
                (ds.BluePaletteColorLookupTableDescriptor,
                 ds.BluePaletteColorLookupTableData)):
            self.assertEqual(list(descriptor), [0, 0, 16])
            # The declared 0 and the actual 65,536 entries, at two bytes each.
            self.assertEqual(len(bytes(data)), 65_536 * 2)

    def test_palette_channels_are_three_different_functions_of_the_index(
            self) -> None:
        """Equal channels would satisfy a red-green-blue transposition and a
        monotone triple would satisfy an off-by-one in every channel at once.
        Neither could then be caught by anything reading this case."""
        ds = self.read("sc_palette_color.dcm")
        channels = [np.frombuffer(bytes(data), dtype="<u2")
                    for data in (ds.RedPaletteColorLookupTableData,
                                 ds.GreenPaletteColorLookupTableData,
                                 ds.BluePaletteColorLookupTableData)]
        self.assertFalse(np.array_equal(channels[0], channels[1]))
        self.assertFalse(np.array_equal(channels[0], channels[2]))
        self.assertFalse(np.array_equal(channels[1], channels[2]))


class IccProfileFixtures(unittest.TestCase):
    """F-240, independent header, tag and device-value checks, ICC.1:2022."""

    def profile(self, kind: str) -> tuple[pydicom.Dataset, dict[bytes, bytes]]:
        path = CORPUS / "synthetic" / f"sc_rgb_icc_{kind}.dcm"
        ds = pydicom.dcmread(path)
        data = bytes(ds.ICCProfile)
        self.assertEqual(data[8:12], bytes.fromhex("04400000"))
        self.assertEqual(data[16:24], b"RGB XYZ ")
        self.assertEqual(data[36:40], b"acsp")
        self.assertEqual(data[44:48], bytes.fromhex("00000001"))
        self.assertEqual(struct.unpack_from(">I", data)[0], len(data))
        count = struct.unpack_from(">I", data, 128)[0]
        tags: dict[bytes, bytes] = {}
        next_offset = 132 + count * 12
        for index in range(count):
            code, offset, size = struct.unpack_from(">4sII", data, 132 + index * 12)
            self.assertNotIn(code, tags)
            self.assertEqual(offset, next_offset)
            self.assertEqual(offset % 4, 0)
            self.assertLessEqual(offset + size, len(data))
            tags[code] = data[offset:offset + size]
            next_offset = offset + size + (-size) % 4
        self.assertEqual(next_offset, len(data))
        self.assertTrue({b"desc", b"cprt", b"wtpt", b"rXYZ", b"gXYZ",
                         b"bXYZ", b"rTRC", b"gTRC", b"bTRC"} <= tags.keys())
        return ds, tags

    def test_unknown_icc_kind_is_refused_by_fixed_name(self) -> None:
        # The helper accepts only the five documented synthetic cases.
        # Unknown input must not silently become a gamma1 profile.
        for kind in ["", "gamma2", "GAMMA1", "unknown"]:
            with self.subTest(kind=kind):
                with self.assertRaisesRegex(ValueError,
                                            "^unknown synthetic ICC case$"):
                    corpus_synth.synthetic_icc_profile(kind)

    def test_committed_profile_bytes_reproduce_from_the_generator(self) -> None:
        base = ROOT / "examples" / "folder-viewer" / "tests" / "fixtures"
        for kind in ["gamma1", "gamma22", "srgb_para", "swap", "lut"]:
            self.assertEqual((base / f"icc_{kind}.icc").read_bytes(),
                             corpus_synth.synthetic_icc_profile(kind))

    def test_profiles_encode_the_standard_version_and_curve_words(self) -> None:
        for kind, gamma in [("gamma1", 0x0100), ("gamma22", 0x0233)]:
            with self.subTest(kind=kind):
                ds, tags = self.profile(kind)
                self.assertEqual(ds.pixel_array.tolist(),
                                 [[[64, 64, 64], [128, 128, 128],
                                   [192, 192, 192], [255, 255, 255]]])
                self.assertEqual(tags[b"rTRC"][:12],
                                 b"curv" + bytes(4) + bytes.fromhex("00000001"))
                self.assertEqual(struct.unpack_from(">H", tags[b"rTRC"], 12)[0], gamma)
                self.assertEqual(tags[b"gTRC"], tags[b"rTRC"])
                self.assertEqual(tags[b"bTRC"], tags[b"rTRC"])

    def test_parametric_fixture_uses_type_three_and_fixed_point_parameters(self) -> None:
        _, tags = self.profile("srgb_para")
        self.assertEqual(tags[b"rTRC"][:12], b"para" + bytes(4) + b"\x00\x03\x00\x00")
        # ICC.1 10.18, round the five sRGB parameters once to signed15.16.
        self.assertEqual(struct.unpack_from(">5i", tags[b"rTRC"], 12),
                         (157286, 62119, 3417, 5072, 2651))

    def test_swap_and_lookup_traps_preserve_their_raw_device_values(self) -> None:
        ds, tags = self.profile("swap")
        self.assertEqual(ds.pixel_array.tolist(), [[[255, 0, 0], [0, 255, 0]]])
        self.assertEqual(struct.unpack_from(">3I", tags[b"rXYZ"], 8),
                         (0x6296, 0xB78A, 0x18DA))
        self.assertEqual(struct.unpack_from(">3I", tags[b"gXYZ"], 8),
                         (0x6FA0, 0x38F2, 0x038F))
        ds, tags = self.profile("lut")
        self.assertEqual(ds.pixel_array.tolist(), [[[10, 20, 30], [200, 100, 50]]])
        self.assertEqual(bytes(ds.ICCProfile)[12:16], b"scnr")
        self.assertEqual(tags[b"A2B0"][:12], b"mft2" + bytes(4) + b"\x03\x03\x02\x00")
        self.assertEqual(struct.unpack_from(">2H", tags[b"A2B0"], 48), (2, 2))


class CaseSelector(unittest.TestCase):
    """`--case`, added by F-030, and the refusal that keeps it honest.

    `generate()` removes and rebuilds the whole synthetic and syntax layer,
    which re-encodes every externally encoded row with whatever version of that
    encoder is installed. `--case` exists so a story that adds one case does not
    also land a toolchain bump it did not choose, and its refusal exists so a
    mistyped name cannot silently generate nothing and report success.
    """

    def test_an_unknown_case_is_refused_and_names_the_known_ones(self) -> None:
        with self.assertRaises(RuntimeError) as caught:
            corpus_synth.generate_one(Path("."), "no-such-case")
        message = str(caught.exception)
        self.assertIn("unknown case", message)
        # The refusal lists what it would have accepted, so a typo is one run
        # to fix rather than a hunt through the generator.
        self.assertIn("sc_palette_color", message)

    def test_every_generated_case_is_reachable_by_name(self) -> None:
        """One list, read by both `generate()` and `--case`.

        A case present in the full run and missing from the selector would be
        regenerable only by rebuilding everything, which is the exact thing the
        selector exists to avoid.
        """
        names = list(corpus_synth.case_callables(Path(".")))
        self.assertEqual(len(names), len(set(names)))
        for expected in ("sc_palette_color", "sc_palette_color_16",
                         "jpegls_rgb8", "syntax"):
            self.assertIn(expected, names)

    def test_the_manifest_categories_name_every_synthetic_case(self) -> None:
        """A case that generates a file with no CATEGORIES row would make
        `--write-manifest` raise, but only after the file was written."""
        # syntax, jpegls_rgb8, jpegls_banded, jpegxl and empty_bot write under syntax/
        # and are named by SYNTAX_CASES rather than by CATEGORIES.
        synthetic = {name for name in corpus_synth.case_callables(Path("."))
                     if name not in ("syntax", "colour_syntax", "jpegls_rgb8",
                                     "jpegls_banded", "jpegxl", "empty_bot")}
        recorded = {Path(path).stem for path in corpus_synth.CATEGORIES}
        recorded |= set(corpus_synth.SERIES_CATEGORY)
        self.assertEqual(synthetic - recorded, set())


class JpegXlEncoderRefusals(unittest.TestCase):
    """F-229's three generator refusals, each driven by a stubbed encoder.

    The locked imagecodecs never returns these shapes, so each test replaces
    one imagecodecs entry point for the duration of one call. The bases are
    copied out of the module's generated corpus so nothing there is written.
    """

    def bases(self, scratch: str) -> Path:
        target = Path(scratch) / "syntax"
        target.mkdir()
        for name in (corpus_synth.REFERENCE_MONO16, corpus_synth.REFERENCE_MONO12,
                     corpus_synth.REFERENCE_RGB8, "jpeg_baseline_rgb8.dcm"):
            shutil.copyfile(CORPUS / "syntax" / name, target / name)
        return target

    def test_a_frame_encoder_returning_no_bytes_is_refused(self) -> None:
        import imagecodecs
        pixels = np.zeros((2, 2), dtype=np.uint8)
        with mock.patch.object(imagecodecs, "jpegxl_encode", return_value=None):
            with self.assertRaises(RuntimeError) as caught:
                corpus_synth.jpegxl_encode_frame(
                    pixels, lossless=True, bits=8, photometric="GRAY")
        self.assertIn("jpegxl_encode did not return bytes", str(caught.exception))

    def test_a_recompression_returning_no_bytes_is_refused(self) -> None:
        import imagecodecs
        with tempfile.TemporaryDirectory(prefix="ocelli-jxl-") as scratch:
            directory = self.bases(scratch)
            with mock.patch.object(imagecodecs, "jpegxl_encode_jpeg",
                                   return_value=None):
                with self.assertRaises(RuntimeError) as caught:
                    corpus_synth.encode_jpegxl_cases(directory)
            self.assertFalse(
                (directory / "jpegxl_recompression_rgb8.dcm").exists())
        self.assertIn("jpegxl_encode_jpeg did not return bytes",
                      str(caught.exception))

    def test_a_recompression_that_does_not_restore_the_jpeg_is_refused(self) -> None:
        """PS3.5 10.19. A `.111` fragment must reconstruct the `.50` bytes."""
        import imagecodecs
        with tempfile.TemporaryDirectory(prefix="ocelli-jxl-") as scratch:
            directory = self.bases(scratch)
            with mock.patch.object(imagecodecs, "jpegxl_decode_jpeg",
                                   return_value=b"\xff\xd8\xff\xd9"):
                with self.assertRaises(RuntimeError) as caught:
                    corpus_synth.encode_jpegxl_cases(directory)
            self.assertFalse(
                (directory / "jpegxl_recompression_rgb8.dcm").exists())
        self.assertIn("did not restore the JPEG fragment", str(caught.exception))


class MultiComponentJpegLs(unittest.TestCase):
    """The row Appendix A gate A2 recorded as owed, added by F-030.

    A2 measured `Nf != 1` as "REFUSED by the crate, and NOT MEASURED here" and
    said the corpus could not close the gap, because no multi-component JPEG-LS
    row existed. A DICOM JPEG-LS frame can be RGB, so it was a coverage hole.
    """

    def test_the_colour_jpegls_row_is_three_component(self) -> None:
        path = CORPUS / "syntax" / "jpegls_lossless_rgb8.dcm"
        if not path.is_file():
            self.skipTest("the ignored corpus is not present")
        ds = pydicom.dcmread(str(path))
        self.assertEqual(str(ds.file_meta.TransferSyntaxUID),
                         "1.2.840.10008.1.2.4.80")
        self.assertEqual(ds.SamplesPerPixel, 3)
        self.assertEqual(ds.PhotometricInterpretation, "RGB")

    def test_the_codestream_declares_nf_three_and_sample_interleave(
            self) -> None:
        """ISO/IEC 14495-1 C.2.2 SOF55 and C.2.3 SOS.

        Read from the codestream rather than from the data set, because the two
        can disagree and it is the codestream the decoder refuses on. `ILV = 2`
        is sample interleave, which is what distinguishes this row from the
        line-interleaved repository-local fixture in
        `crates/ocelli-codec/tests/fixtures/jpegls_rgb8_ilv1.jls`.
        """
        path = CORPUS / "syntax" / "jpegls_lossless_rgb8.dcm"
        if not path.is_file():
            self.skipTest("the ignored corpus is not present")
        ds = pydicom.dcmread(str(path))
        frames = list(pydicom.encaps.generate_frames(ds.PixelData,
                                                     number_of_frames=1))
        self.assertEqual(len(frames), 1)
        codestream = frames[0]

        start = codestream.find(b"\xff\xf7")
        self.assertGreater(start, 0)
        # Lf(2) P(1) Y(2) X(1..2) Nf, per C.2.2.
        self.assertEqual(codestream[start + 9], 3)

        sos = codestream.find(b"\xff\xda")
        self.assertGreater(sos, start)
        components = codestream[sos + 4]
        self.assertEqual(components, 3)
        # Ns (Ci Tm)*Ns NEAR ILV, per C.2.3.
        self.assertEqual(codestream[sos + 4 + 2 * components + 2], 2)



class ControlledColourComparisonFixtures(unittest.TestCase):
    """F239AC32/F240AC16 inputs from their approved dimensions and PS3.5."""

    def test_closed_measurement_sizes_match_the_approved_cases(self) -> None:
        self.assertEqual(corpus_synth.COMPARISON_COLOUR_CASES, {
            "rgb8": (512, 200, "RGB", 3, 8),
            "ybr422": (512, 200, "YBR_FULL_422", 3, 8),
            "palette16": (512, 200, "PALETTE COLOR", 1, 16),
            "icc8-gamma22": (8192, 1, "RGB", 3, 8),
            "icc16-table": (8192, 1, "RGB", 3, 16),
        })

    def test_small_inputs_are_deterministic_and_keep_calibration_separate(self) -> None:
        for case in ["rgb8", "ybr422", "palette16", "icc8-gamma22", "icc16-table"]:
            with self.subTest(case=case), tempfile.TemporaryDirectory() as first, \
                    tempfile.TemporaryDirectory() as second:
                roots = [Path(first), Path(second)]
                manifests = [json.loads(corpus_synth.write_colour_comparison(
                    root, case, edge=6, frames=2).read_text()) for root in roots]
                self.assertEqual(manifests[0], manifests[1])
                manifest = manifests[0]
                self.assertFalse(manifest["measurementSize"])
                self.assertEqual(len(manifest["targetFiles"]), 2)
                self.assertEqual(len(manifest["calibrationFiles"]), 1)
                calibration = pydicom.dcmread(roots[0] / manifest["calibrationFiles"][0]["path"])
                self.assertEqual((calibration.Modality, calibration.Rows,
                                  calibration.Columns, calibration.PhotometricInterpretation),
                                 ("CT", 512, 512, "MONOCHROME2"))
                self.assertEqual((float(calibration.WindowCenter), float(calibration.WindowWidth)),
                                 (40, 400))
                target_uids = set()
                for number, item in enumerate(manifest["targetFiles"], 1):
                    path = roots[0] / item["path"]
                    self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), item["sha256"])
                    self.assertEqual(path.stat().st_size, item["bytes"])
                    ds = pydicom.dcmread(path)
                    self.assertEqual((ds.Rows, ds.Columns, ds.InstanceNumber), (6, 6, number))
                    self.assertEqual(ds.PhotometricInterpretation, manifest["photometric"])
                    self.assertEqual(ds.SamplesPerPixel, manifest["samplesPerPixel"])
                    self.assertEqual(ds.BitsStored, manifest["bitsStored"])
                    self.assertEqual(ds.HighBit, ds.BitsStored - 1)
                    self.assertEqual(ds.PixelRepresentation, 0)
                    self.assertEqual(str(ds.file_meta.TransferSyntaxUID), "1.2.840.10008.1.2.1")
                    self.assertNotIn("WindowCenter", ds)
                    target_uids.add(str(ds.SeriesInstanceUID))
                self.assertEqual(target_uids, {manifest["seriesUid"]})
                self.assertNotEqual(str(calibration.SeriesInstanceUID), manifest["seriesUid"])
                self.assertEqual(manifest["serverCacheMiB"], 512 if case == "icc16-table" else 256)

    def test_native_stored_patterns_preserve_channels_pairs_and_low_device_bits(self) -> None:
        rgb = np.frombuffer(corpus_synth.comparison_colour_row("rgb8", 6, 1, 0), dtype=np.uint8)
        self.assertEqual(rgb.reshape(6, 3)[5].tolist(), [5, 1, 6])
        later = np.frombuffer(corpus_synth.comparison_colour_row("rgb8", 6, 1, 1), dtype=np.uint8)
        self.assertEqual(later.reshape(6, 3)[5].tolist(), [36, 48, 77])
        ybr = corpus_synth.comparison_colour_row("ybr422", 6, 0, 0)
        self.assertEqual(list(ybr), [64, 128, 128, 128, 100, 101, 150, 90, 4, 5, 64, 128])
        palette = np.frombuffer(corpus_synth.comparison_colour_row("palette16", 6, 1, 1), dtype="<u2")
        self.assertEqual(palette.tolist(), list(range(263, 269)))
        high = np.frombuffer(corpus_synth.comparison_colour_row("icc16-table", 6, 0, 0), dtype="<u2").reshape(6, 3)
        self.assertEqual(high[:5].tolist(), [[v, v, v] for v in [15, 16384, 32768, 49152, 65535]])
        self.assertEqual(len(corpus_synth.comparison_colour_row("icc16-table", 8192, 1, 0)), 49152)

    def test_palette_descriptor_and_tables_have_the_declared_sixteen_bit_words(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            manifest = json.loads(corpus_synth.write_colour_comparison(root, "palette16", edge=6, frames=1).read_text())
            ds = pydicom.dcmread(root / manifest["targetFiles"][0]["path"])
            for name, expected in [("Red", 100), ("Green", 65435), ("Blue", 25700)]:
                self.assertEqual(list(getattr(ds, name + "PaletteColorLookupTableDescriptor")), [0, 0, 16])
                data = getattr(ds, name + "PaletteColorLookupTableData")
                self.assertEqual(len(data), 131072)
                self.assertEqual(struct.unpack_from("<H", data, 200)[0], expected)

    def test_icc_profiles_keep_own_gamma22_and_valid_two_entry_curve_ids(self) -> None:
        gamma = corpus_synth.comparison_icc_profile("icc8-gamma22")
        self.assertEqual(gamma, corpus_synth.synthetic_icc_profile("gamma22"))
        table = corpus_synth.comparison_icc_profile("icc16-table")
        self.assertIsNotNone(table)
        count = struct.unpack_from(">I", table, 128)[0]
        seen = 0
        for index in range(count):
            signature, offset, length = struct.unpack_from(">4sII", table, 132 + index * 12)
            if signature in [b"rTRC", b"gTRC", b"bTRC"]:
                self.assertEqual(length, 16)
                self.assertEqual(table[offset:offset + 16], b"curv\0\0\0\0" + struct.pack(">I2H", 2, 0, 65535))
                seen += 1
        self.assertEqual(seen, 3)
        for data in [gamma, table]:
            hashed = bytearray(data)
            hashed[44:48], hashed[64:68], hashed[84:100] = bytes(4), bytes(4), bytes(16)
            self.assertEqual(data[84:100], hashlib.md5(hashed, usedforsecurity=False).digest())

    def test_invalid_selector_dimensions_and_nonempty_output_refuse(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with self.assertRaisesRegex(ValueError, "unknown controlled"):
                corpus_synth.write_colour_comparison(root, "constructor")
            for kwargs in [{"edge": 3}, {"edge": 0}, {"edge": True}, {"frames": 0}, {"frames": 201}]:
                with self.assertRaisesRegex(ValueError, "invalid controlled"):
                    corpus_synth.write_colour_comparison(root, "rgb8", **kwargs)
            (root / "kept.txt").write_text("keep")
            with self.assertRaisesRegex(ValueError, "must be empty"):
                corpus_synth.write_colour_comparison(root, "rgb8", edge=6, frames=1)
            self.assertEqual((root / "kept.txt").read_text(), "keep")

    def test_explicit_cli_mode_cannot_fall_into_corpus_generation(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            for args in [["--comparison-colour", "rgb8"],
                         ["--comparison-colour", "rgb8", "--out", tmp, "--case", "rgb8"]]:
                proc = subprocess.run([sys.executable, str(ROOT / "scripts/corpus_synth.py"), *args],
                                      capture_output=True, text=True, check=False)
                self.assertEqual(proc.returncode, 2)
                self.assertIn("needs explicit --out and no corpus mode", proc.stderr)
                self.assertEqual(list(Path(tmp).iterdir()), [])


if __name__ == "__main__":
    unittest.main()
