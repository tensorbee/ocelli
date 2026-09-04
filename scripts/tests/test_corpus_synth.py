#!/usr/bin/env python3
"""Tests for scripts/corpus_synth.py, the synthetic corpus generator.

Run with the interpreter that has pydicom, numpy and the codec plugins:

    /path/to/venv/bin/python -m unittest discover -s scripts/tests -v

Three things are proved here, and they are the three that make a synthetic
corpus worth having.

1. **fixture.** The stored value of a hand-chosen raw word, computed from
   PS3.3 C.7.6.3.1.4 and the definitions of BitsStored (0028,0101), HighBit
   (0028,0102) and PixelRepresentation (0028,0103). The expected values in
   this file were computed by hand from the standard and are shown working.
   They were NOT read back from the generator, and the generator contains no
   unpacking code they could have come from.

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
import os
import re
import struct
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

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


# ---------------------------------------------------------------------------
# The hand-computed fixture table.
# ---------------------------------------------------------------------------
#
# PS3.3 C.7.6.3.1.4 and PS3.5 8.1.1 define the stored value inside its
# container:
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

# ct_signed_12in16_left.dcm: BitsStored 12, HighBit 15, PixelRepresentation 1.
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


def stored_value(raw: int, bits_stored: int, high_bit: int,
                 pixel_representation: int) -> int:
    """PS3.3 C.7.6.3.1.4, transcribed. Independent of anything Ocelli does."""
    shift = high_bit + 1 - bits_stored
    value = (raw >> shift) & ((1 << bits_stored) - 1)
    if pixel_representation == 1 and value & (1 << (bits_stored - 1)):
        value -= 1 << bits_stored
    return value


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
    """The values a 12-bit-in-16 case must unpack to, from PS3.3 C.7.6.3.1.4."""

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

    def test_left_aligned_header_is_what_the_case_claims(self) -> None:
        ds, _ = self.probe("ct_signed_12in16_left.dcm")
        self.assertEqual((ds.BitsAllocated, ds.BitsStored, ds.HighBit,
                          ds.PixelRepresentation), (16, 12, 15, 1))

    def test_right_aligned_stored_values(self) -> None:
        ds, words = self.probe("ct_signed_12in16_right.dcm")
        got = tuple(stored_value(w, ds.BitsStored, ds.HighBit,
                                 ds.PixelRepresentation) for w in words)
        self.assertEqual(got, RIGHT_ALIGNED)

    def test_left_aligned_stored_values(self) -> None:
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
YBR_EIGHT_BIT_MAX = 0.03
# The three transform cases at their configured settings: JPEG extended at
# quality 90 gives 0.00049, JPEG 2000 at ratio 20 gives 0.00003, HTJ2K at
# qstep 0.001 gives 0.00063, so this is the worst of them rounded up an order.
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
                    "htj2k_lossy.dcm": 2}           # RPCL
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

    def test_lossless_syntaxes_round_trip_exactly(self) -> None:
        for name, uid in corpus_synth.SYNTAX_CASES.items():
            if uid in corpus_synth.LOSSY_TRANSFER_SYNTAXES:
                continue
            with self.subTest(case=name):
                ds = self.case(name)
                reference = pydicom.dcmread(
                    str(CORPUS / "syntax" / corpus_synth.SYNTAX_REFERENCE[name]))
                self.assertTrue(
                    np.array_equal(ds.pixel_array, reference.pixel_array),
                    f"{name} declares a lossless syntax and did not round-trip")

    def test_jpeg_ls_near_lossless_holds_its_declared_near_bound(self) -> None:
        """ISO/IEC 14495-1 guarantees max abs error <= NEAR. This is the one
        lossy case with a bound the standard actually promises, so it is
        asserted exactly rather than as a sanity range."""
        ds = self.case("jpegls_near_lossless.dcm")
        reference = pydicom.dcmread(str(CORPUS / "syntax" / "explicit_vr_le.dcm"))
        error = np.abs(ds.pixel_array.astype(np.int64)
                       - reference.pixel_array.astype(np.int64))
        self.assertLessEqual(int(error.max()), corpus_synth.JPEG_LS_NEAR)

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
                got, want = ds.pixel_array, reference.pixel_array
                self.assertEqual(got.shape, want.shape)
                full_scale = float(1 << ds.BitsStored) - 1.0
                error = np.abs(got.astype(np.float64) - want.astype(np.float64))
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
        self.fail(f"unrecognised codestream {frame[:4].hex()}")

    def test_the_classifier_refuses_a_codestream_it_cannot_name(self) -> None:
        """The fall-through is unreachable for the cases shipped today, so
        nothing else reaches it. A future lossy case whose codestream this
        cannot parse must not be quietly declared to be one of the four."""
        with self.assertRaises(self.failureException):
            self.codestream_method(b"\x00\x01\x02\x03")


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
        found = [name for name, uid in corpus_synth.SYNTAX_CASES.items()
                 if uid == corpus_synth.INTERNAL_ENCODER_SYNTAX]
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
    # pylibjpeg-openjpeg wheel. Everything else must be a project pin.
    NOT_UV = ("OpenJPEG (the library inside it)", "DCMTK", "OpenJPH")

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


if __name__ == "__main__":
    unittest.main()
