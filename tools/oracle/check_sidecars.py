#!/usr/bin/env python3
"""Cross-read every oracle sidecar's DICOM attributes with pydicom.

HLD section 11 diffs metadata alongside pixels "because a wrong rescale slope
can still produce a plausible image". That makes the sidecar load-bearing
output of F-010: a sidecar that transcribed a rescale slope wrongly would send
F-011 chasing a pixel difference that is really a metadata bug.

So the sidecar's `attributes` block is read a second time, here, by a different
library in a different language, and the two readings have to agree. This is
the `fixture` row of the design plan's test table: expected values come from
PS3.3 through pydicom, and from the generator's own constants for the named
rows below, never from what the harness printed.

Two checks, and both are needed:

1. Every attribute in every sidecar matches pydicom's reading of the same file.
   Broad, and it cannot be wrong in the same direction as the harness unless
   two independent parsers share a bug.
2. A hand-written expectation table for named synthetic rows, whose values are
   known by construction from `scripts/corpus_synth.py` and PS3.3. Narrow, and
   it would catch two parsers agreeing on the wrong thing.

Failure output for a REAL corpus row names the relative path and the attribute
only, never the value, following `corpus_check.py`'s convention. Synthetic rows
carry no patient identity and print their values, because that is what makes a
mismatch diagnosable.

Exit codes: 0 pass, 1 mismatch, 3 pydicom is not importable under this
interpreter (the caller tries the next one, and a skip is never a pass).

Usage:
  python3 tools/oracle/check_sidecars.py --out tools/oracle/out
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
CORPUS = ROOT / "corpus" / "data"
MANIFEST = ROOT / "corpus" / "manifest.tsv"
UNSUPPORTED = ROOT / "tools" / "oracle" / "unsupported.json"

try:
    import pydicom
except ImportError as error:  # pragma: no cover, exercised by the caller
    print(f"SKIP: this interpreter cannot import pydicom ({error})",
          file=sys.stderr)
    raise SystemExit(3) from error


# The sidecar field name, and the pydicom keyword it is read from. File Meta
# and the two multi-valued VOI attributes are special cased below.
SCALARS = {
    "sopClassUID": "SOPClassUID",
    "modality": "Modality",
    "photometricInterpretation": "PhotometricInterpretation",
    "samplesPerPixel": "SamplesPerPixel",
    "planarConfiguration": "PlanarConfiguration",
    "rows": "Rows",
    "columns": "Columns",
    "bitsAllocated": "BitsAllocated",
    "bitsStored": "BitsStored",
    "highBit": "HighBit",
    "pixelRepresentation": "PixelRepresentation",
    "numberOfFrames": "NumberOfFrames",
    "rescaleSlope": "RescaleSlope",
    "rescaleIntercept": "RescaleIntercept",
    "rescaleType": "RescaleType",
    "voiLutFunction": "VOILUTFunction",
    "sliceThickness": "SliceThickness",
    "lossyImageCompression": "LossyImageCompression",
    "lossyImageCompressionMethod": "LossyImageCompressionMethod",
}

SEQUENCES = {
    "windowCenter": "WindowCenter",
    "windowWidth": "WindowWidth",
    "pixelSpacing": "PixelSpacing",
    "imagePositionPatient": "ImagePositionPatient",
    "imageOrientationPatient": "ImageOrientationPatient",
}

# Hand-written, from PS3.3 and from scripts/corpus_synth.py's own constants.
# Chosen to cover every photometric interpretation and both values of Pixel
# Representation that the corpus's sidecars carry. `_coverage` below refuses to
# pass if that stops being true.
EXPECTED = {
    # PS3.3 C.7.6.3.1.2 MONOCHROME2, and BitsStored equal to BitsAllocated.
    "synthetic/ct_unsigned_16.dcm": {
        "photometricInterpretation": "MONOCHROME2",
        "samplesPerPixel": 1,
        "bitsAllocated": 16,
        "bitsStored": 16,
        "highBit": 15,
        "pixelRepresentation": 0,
        "rows": 12,
        "columns": 20,
        "rescaleSlope": 1.0,
        "rescaleIntercept": 0.0,
        "windowCenter": [40.0],
        "windowWidth": [400.0],
        "voiLutFunction": "LINEAR",
        "pixelSpacing": [0.5, 0.25],
    },
    # PS3.5 8.1.1, which defines the Pixel Cell and says High Bit (0028,0102)
    # is where the high order bit of Bits Stored sits within Bits Allocated.
    # High Bit itself is in PS3.3 C.7.6.3, Table C.7-11c. Signed twelve bits in
    # a sixteen bit container, LEFT aligned: HighBit 15 with BitsStored 12 is
    # the alignment a reader that ignores (0028,0102) gets wrong while still
    # producing plausible numbers. The current edition of PS3.5 8.1.1 requires
    # High Bit to be one less than Bits Stored, so this encoding is one it
    # retired, which is exactly why the corpus carries it.
    "synthetic/ct_signed_12in16_left.dcm": {
        "photometricInterpretation": "MONOCHROME2",
        "bitsAllocated": 16,
        "bitsStored": 12,
        "highBit": 15,
        "pixelRepresentation": 1,
        "rescaleSlope": 1.0,
        "rescaleIntercept": -1024.0,
        "windowCenter": [40.0],
        "windowWidth": [400.0],
    },
    # The same trap, right aligned.
    "synthetic/ct_signed_12in16_right.dcm": {
        "bitsStored": 12,
        "highBit": 11,
        "pixelRepresentation": 1,
        "rescaleIntercept": -1024.0,
    },
    # PS3.3 C.7.6.3.1.2 MONOCHROME1: the minimum value is white. The generator
    # gives this one a window of its own and no Modality LUT at all.
    "synthetic/cr_monochrome1.dcm": {
        "modality": "CR",
        "photometricInterpretation": "MONOCHROME1",
        "bitsStored": 12,
        "highBit": 11,
        "pixelRepresentation": 0,
        "rescaleSlope": None,
        "rescaleIntercept": None,
        "windowCenter": [2048.0],
        "windowWidth": [4096.0],
    },
    # PS3.3 C.7.6.3.1.3, PlanarConfiguration 0, colour by pixel.
    "synthetic/sc_rgb_interleaved.dcm": {
        "photometricInterpretation": "RGB",
        "samplesPerPixel": 3,
        "planarConfiguration": 0,
        "bitsAllocated": 8,
        "bitsStored": 8,
        "highBit": 7,
        "pixelRepresentation": 0,
    },
    # The same image, PlanarConfiguration 1, colour by plane.
    "synthetic/sc_rgb_planar.dcm": {
        "photometricInterpretation": "RGB",
        "planarConfiguration": 1,
    },
    # PS3.3 C.7.6.2.1.1: PixelSpacing[0] is the spacing BETWEEN ROWS. Both the
    # frame and the spacing are non-square, and in opposite senses: 40 columns
    # by 12 rows with row spacing 0.5 and column spacing 0.25, so a transposed
    # index and a transposed spacing are separately visible.
    "synthetic/mr_nonsquare_spacing.dcm": {
        "modality": "MR",
        "rows": 12,
        "columns": 40,
        "pixelSpacing": [0.5, 0.25],
        "sliceThickness": 3.0,
        "windowCenter": [1024.0],
        "windowWidth": [2048.0],
    },
    # PS3.5 8.2.1 permits YBR_FULL_422 or RGB for JPEG Baseline at three
    # samples per pixel, and the encoder chose YBR_FULL_422, so it rewrote
    # (0028,0004) and the row is no longer RGB. A reader that kept RGB would
    # render this as colour noise, which is why it is asserted.
    # Lossy Image Compression and its Method are both Type 3 in PS3.3 Table
    # C.7-9, so the standard does not require them. `corpus_synth.py` writes
    # them, and that is the claim the last two lines check.
    "syntax/jpeg_baseline_rgb8.dcm": {
        "transferSyntaxUID": "1.2.840.10008.1.2.4.50",
        "photometricInterpretation": "YBR_FULL_422",
        "samplesPerPixel": 3,
        "planarConfiguration": 0,
        "bitsAllocated": 8,
        "bitsStored": 8,
        "highBit": 7,
        "pixelRepresentation": 0,
        "rows": 64,
        "columns": 96,
        "lossyImageCompression": "01",
        "lossyImageCompressionMethod": "ISO_10918_1",
    },
    # The uncompressed reference the transfer-syntax cases are encoded from.
    "syntax/reference_mono12.dcm": {
        "transferSyntaxUID": "1.2.840.10008.1.2.1",
        "bitsAllocated": 16,
        "bitsStored": 12,
        "highBit": 11,
        "pixelRepresentation": 0,
        "rows": 64,
        "columns": 96,
        "rescaleIntercept": 0.0,
    },
}


# The other half of the sidecar: what cornerstone3D itself resolved and used.
#
# The `attributes` block above is read from the top-level data set, and for one
# corpus row that is deliberately not where the answer lives.
# `synthetic/ct_multiframe_perframe.dcm` carries its rescale and its window
# only in the per-frame functional groups (PS3.3 C.7.6.16), so `attributes`
# correctly reports absent for both, both readers then agree on "absent", and
# the values that ACTUALLY DROVE THE RENDER are checked by nothing.
#
# That is exactly HLD section 11's "a wrong rescale slope can still produce a
# plausible image", on the one row the corpus wrote to trap it. So the
# reference's own resolved modules are checked too, against values known by
# construction from `scripts/corpus_synth.py`.
#
# Keys are `<module>.<field>`. The value is written in the shape the pinned
# reference returns it, a list where the module carries a list, because a shape
# that moved would mean the pin moved and that is worth going red for too.
EXPECTED_CORNERSTONE = {
    # Frame 0 of three. corpus_synth.py's `case_multiframe` writes
    # slopes ["1", "2", "0.5"], intercepts ["-1024", "-2048", "0"],
    # centres ["40", "300", "-600"] and widths ["400", "1500", "1600"], and
    # this story renders frame 0 only.
    "synthetic/ct_multiframe_perframe.dcm": {
        "modalityLutModule.rescaleSlope": 1.0,
        "modalityLutModule.rescaleIntercept": -1024.0,
        "modalityLutModule.rescaleType": "HU",
        "voiLutModule.windowCenter": [40.0],
        "voiLutModule.windowWidth": [400.0],
        "voiLutModule.voiLUTFunction": "LINEAR",
        # SharedFunctionalGroupsSequence, PixelMeasuresSequence.
        "imagePlaneModule.pixelSpacing": [0.5, 0.25],
        "imagePlaneModule.sliceThickness": 2.5,
    },
    # A single-frame row, where the two readings must AGREE rather than
    # complement each other. This is what makes the row above a trap rather
    # than the normal case.
    "synthetic/ct_signed_12in16_left.dcm": {
        "modalityLutModule.rescaleSlope": 1.0,
        "modalityLutModule.rescaleIntercept": -1024.0,
        "voiLutModule.windowCenter": [40.0],
        "voiLutModule.windowWidth": [400.0],
        "imagePixelModule.bitsStored": 12,
        "imagePixelModule.highBit": 15,
        "imagePixelModule.pixelRepresentation": 1,
    },
}


def _numeric(value):
    """A DICOM numeric string or number as a float, or None."""
    if value is None:
        return None
    return float(value)


def _as_list(value):
    if value is None:
        return None
    if isinstance(value, (list, tuple)) or type(value).__name__ == "MultiValue":
        return [float(item) for item in value]
    return [float(value)]


def _read(path: Path) -> dict:
    """Read one file's attributes the way the page reads them."""
    dataset = pydicom.dcmread(str(path), stop_before_pixels=True)
    read = {"transferSyntaxUID": str(dataset.file_meta.TransferSyntaxUID)}
    for field, keyword in SCALARS.items():
        value = dataset.get(keyword, None)
        if value is None or value == "":
            read[field] = None
        elif keyword in {"RescaleSlope", "RescaleIntercept", "SliceThickness",
                         "NumberOfFrames"}:
            read[field] = _numeric(value)
        elif isinstance(value, int):
            read[field] = int(value)
        else:
            read[field] = str(value)
    for field, keyword in SEQUENCES.items():
        read[field] = _as_list(dataset.get(keyword, None))
    return read


def _equal(left, right) -> bool:
    if left is None or right is None:
        return left is None and right is None
    if isinstance(left, list) or isinstance(right, list):
        if not isinstance(left, list) or not isinstance(right, list):
            return False
        return len(left) == len(right) and all(
            _equal(a, b) for a, b in zip(left, right))
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return float(left) == float(right)
    return str(left) == str(right)


def _show(row_path: str, value) -> str:
    """A value, unless the row is real, in which case only its shape."""
    if row_path.startswith("real/"):
        return "<withheld, real corpus row>"
    return json.dumps(value)


def _coverage(sidecars: dict) -> list[str]:
    """The expectation table still ASSERTS every case the corpus renders.

    Naming a row is not covering it. A row can sit in EXPECTED with five
    assertions, none of them about photometric interpretation, and a check that
    only asked which rows are named would call that value covered. So a row
    counts towards a field only when its own entry asserts THAT FIELD.
    """
    problems = []
    for field in ("photometricInterpretation", "pixelRepresentation"):
        present = {
            json.dumps(side["attributes"].get(field))
            for side in sidecars.values()
            if side.get("attributes")
        }
        covered = {
            json.dumps(sidecars[path]["attributes"].get(field))
            for path, expected in EXPECTED.items()
            if field in expected
            and path in sidecars
            and sidecars[path].get("attributes")
        }
        missing = sorted(present - covered)
        if missing:
            problems.append(
                f"the expectation table in check_sidecars.py ASSERTS {field} "
                f"on no row whose value is {', '.join(missing)}, and the "
                f"corpus renders one. The design plan asks for at least one "
                f"hand-written row per photometric interpretation and per "
                f"pixel representation, and naming a row without asserting "
                f"the field is not covering it."
            )
    return problems


def _self_test() -> int:
    """Exercise the helpers that only ever run on a mismatch.

    `_show` decides whether a value reaches the terminal, and it is reached
    only when a comparison has already failed, which no gate run ever does. A
    redaction nobody has watched work is not a redaction, and the regression
    that leaks a real corpus value into gate output would otherwise be
    invisible to every check in this repository.
    """
    problems: list[str] = []

    def check(condition: bool, what: str) -> None:
        if not condition:
            problems.append(what)

    # The redaction. A real row's value never reaches the terminal, whatever
    # its type. corpus/README.md: every real row carries burned-in-unchecked.
    for value in (12, "MONOCHROME2", [0.5, 0.25], None):
        shown = _show("real/ct_cmb_mml/00000001.dcm", value)
        check(shown == "<withheld, real corpus row>",
              f"a real row's value {value!r} was shown as {shown}")
        check(json.dumps(value) not in shown,
              f"a real row's value {value!r} leaked into {shown}")
    # A synthetic row has no patient identity, and its value is what makes a
    # mismatch diagnosable.
    check(_show("synthetic/ct_unsigned_16.dcm", 12) == "12",
          "a synthetic row's value was withheld, which makes a mismatch "
          "undiagnosable for no benefit")
    check(_show("syntax/reference_mono12.dcm", "MONOCHROME2") == '"MONOCHROME2"',
          "a syntax row's value was withheld")

    # The comparison. Absence is not zero, and a shorter list is not a prefix.
    check(_equal(None, None), "None does not equal None")
    check(not _equal(None, 0), "None compared equal to 0")
    check(not _equal(0, None), "0 compared equal to None")
    check(_equal(1, 1.0), "1 does not equal 1.0")
    check(_equal([0.5, 0.25], [0.5, 0.25]), "equal lists compared unequal")
    check(not _equal([0.5], [0.5, 0.25]), "a shorter list compared equal")
    check(not _equal([0.25, 0.5], [0.5, 0.25]), "order was ignored")
    check(not _equal(40, [40]), "a scalar compared equal to a one-element list")
    check(not _equal("MONOCHROME1", "MONOCHROME2"), "two strings compared equal")

    if problems:
        print("FAIL: check_sidecars self test")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: check_sidecars self test, redaction and comparison")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(ROOT / "tools" / "oracle" / "out"))
    parser.add_argument(
        "--self-test", action="store_true", dest="self_test",
        help="check the redaction and comparison helpers, which only ever run "
             "on a mismatch, and exit")
    parser.add_argument(
        "--partial", action="store_true",
        help="the run rendered a subset of the manifest, so the three "
             "completeness claims do not apply: a sidecar for every row, a "
             "sidecar for every named fixture row, and an assertion for every "
             "value the corpus renders. Every per-sidecar comparison and every "
             "fixture assertion still runs.")
    args = parser.parse_args()
    if args.self_test:
        return _self_test()
    out = Path(args.out)

    if not out.is_dir():
        print(f"FAIL: {out} does not exist, so there is nothing to check")
        return 1

    sidecars = {}
    for path in sorted(out.glob("*.json")):
        if path.name == "run.json":
            continue
        sidecar = json.loads(path.read_text())
        sidecars[sidecar["row"]["path"]] = sidecar

    problems: list[str] = []

    # The set of sidecars on disk, checked against the manifest independently
    # of what the driver believed it wrote.
    manifest_rows = []
    for index, line in enumerate(MANIFEST.read_text().splitlines()):
        if index == 0 or not line:
            continue
        manifest_rows.append(line.split("\t")[0])
    unsupported = json.loads(UNSUPPORTED.read_text())
    claimed = {row for entry in unsupported["entries"] for row in entry["rows"]}
    expected_rows = set(manifest_rows) - claimed
    if not args.partial:
        for missing in sorted(expected_rows - set(sidecars)):
            problems.append(f"{missing}: no sidecar in {out}, and "
                            f"unsupported.json does not account for the row")
    for extra in sorted(set(sidecars) - set(manifest_rows)):
        problems.append(f"{extra}: a sidecar for a path the manifest does not "
                        f"carry")

    if not sidecars:
        print(f"FAIL: no sidecars in {out}")
        return 1

    compared = 0
    for row_path, sidecar in sorted(sidecars.items()):
        attributes = sidecar.get("attributes")
        if attributes is None:
            # The page could not parse the file independently. That is recorded
            # rather than hidden, and it must be recorded.
            if not sidecar.get("attributesError"):
                problems.append(
                    f"{row_path}: the sidecar has neither attributes nor an "
                    f"attributesError, so nothing says why they are absent")
            continue

        source = CORPUS / row_path
        if not source.is_file():
            problems.append(f"{row_path}: not present under corpus/data")
            continue
        try:
            truth = _read(source)
        except Exception as error:  # noqa: BLE001, a parse failure IS the news
            problems.append(f"{row_path}: pydicom could not read it ({error})")
            continue

        for field, expected in truth.items():
            actual = attributes.get(field, "<absent from the sidecar>")
            if not _equal(expected, actual):
                problems.append(
                    f"{row_path}: {field} is {_show(row_path, actual)} in the "
                    f"sidecar and {_show(row_path, expected)} in the file")
        compared += 1

    fixtures = 0
    for row_path, expected in sorted(EXPECTED.items()):
        sidecar = sidecars.get(row_path)
        if sidecar is None:
            if not args.partial:
                problems.append(
                    f"{row_path}: the hand-written expectation table names "
                    f"this row and no sidecar was produced for it")
            continue
        attributes = sidecar.get("attributes") or {}
        for field, value in expected.items():
            actual = attributes.get(field, "<absent from the sidecar>")
            if not _equal(value, actual):
                problems.append(
                    f"{row_path}: {field} is {_show(row_path, actual)} and "
                    f"PS3.3 with scripts/corpus_synth.py says "
                    f"{_show(row_path, value)}")
        fixtures += 1

    resolved = 0
    for row_path, expected in sorted(EXPECTED_CORNERSTONE.items()):
        sidecar = sidecars.get(row_path)
        if sidecar is None:
            if not args.partial:
                problems.append(
                    f"{row_path}: EXPECTED_CORNERSTONE names this row and no "
                    f"sidecar was produced for it")
            continue
        modules = sidecar.get("cornerstoneMetadata") or {}
        for key, value in expected.items():
            module_name, _, field = key.partition(".")
            module = modules.get(module_name)
            if not isinstance(module, dict):
                problems.append(
                    f"{row_path}: the sidecar carries no {module_name} from "
                    f"the reference, so the values that drove the render are "
                    f"recorded by nothing")
                continue
            actual = module.get(field, "<absent from the module>")
            if not _equal(value, actual):
                problems.append(
                    f"{row_path}: the reference resolved {key} as "
                    f"{_show(row_path, actual)} and scripts/corpus_synth.py "
                    f"writes {_show(row_path, value)}")
        resolved += 1

    # `_coverage` is a claim about the expectation table against THE WHOLE
    # CORPUS, so it is one of the three completeness checks a partial run
    # cannot make. Left running, it fails any `--rows` selection whose sidecars
    # happen to carry a value no selected EXPECTED row asserts, with a message
    # that is then also false: `--rows real/` would report that the table
    # asserts photometricInterpretation on no MONOCHROME2 row, when it does, on
    # a row that run did not render.
    if not args.partial:
        problems.extend(_coverage(sidecars))

    if problems:
        print("FAIL: oracle sidecar metadata")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: {compared} sidecar(s) agree with pydicom, {fixtures} "
          f"hand-written fixture row(s) agree with PS3.3, {resolved} row(s) "
          f"of resolved reference metadata agree with the generator")
    return 0


if __name__ == "__main__":
    sys.exit(main())
