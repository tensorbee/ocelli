#!/usr/bin/env python3
"""Extract the F-239 colour codestream fixtures and their references.

The corpus rows `syntax/j2k_lossless_ybr_rct.dcm`,
`syntax/j2k_lossy_ybr_ict.dcm`, `syntax/htj2k_lossless_ybr_rct.dcm` and
`syntax/htj2k_lossy_ybr_ict.dcm` are `corpus/manifest.tsv` rows encoded from
the saturated RGB reference `syntax/reference_rgb8_saturated.dcm` by
`scripts/corpus_synth.py`. The uncompressed form of that reference is its own
Pixel Data, written here as one raw file. OpenJPEG's own decode of the two
lossy rows is written beside them, because ISO/IEC 15444-1 G.3's coefficients
are what an independent conforming decoder applies and the lossy rows have no
byte-exact anchor.

**The reference is an exact anchor for the two lossless rows.** The syntax is
lossless, so both decodes must reproduce the saturated reference byte for
byte, whatever encoder produced the codestream. The lossy rows are compared
against OpenJPEG's decode instead and the largest per-channel difference is
asserted to the recorded bound in the two codec test files.

Run without arguments to check the tracked bytes against the corpus, or
pass ``--write`` to rebuild them. Both need the ignored corpus present.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import numpy as np
import pydicom

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parents[3] / "corpus" / "data" / "syntax"

# fixture name -> (corpus row, what it is)
CODESTREAMS = {
    "j2k_colour_lossless_ybr_rct.j2k": "j2k_lossless_ybr_rct.dcm",
    "j2k_colour_lossy_ybr_ict.j2k": "j2k_lossy_ybr_ict.dcm",
    "htj2k_colour_lossless_ybr_rct.j2c": "htj2k_lossless_ybr_rct.dcm",
    "htj2k_colour_lossy_ybr_ict.j2c": "htj2k_lossy_ybr_ict.dcm",
}
REFERENCE = "j2k_colour_reference_rgb8le.raw"
OPENJPEG_DECODES = {
    "j2k_colour_lossy_ict_openjpeg.raw": "j2k_lossy_ybr_ict.dcm",
    "htj2k_colour_lossy_ict_openjpeg.raw": "htj2k_lossy_ybr_ict.dcm",
}


def digest(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def encapsulated_frame(path: Path) -> bytes:
    """The single encoded frame of a one-frame encapsulated instance."""
    dataset = pydicom.dcmread(str(path))
    frames = list(pydicom.encaps.generate_frames(dataset.PixelData,
                                                 number_of_frames=1))
    if len(frames) != 1:
        raise SystemExit(f"{path.name}: expected one frame, found {len(frames)}")
    frame = frames[0]
    # PS3.5 A.4 pads an odd fragment to an even length. A JPEG 2000 codestream
    # ends at EOC, so a trailing pad byte is not part of it.
    if frame.endswith(b"\x00") and not frame.endswith(b"\xff\xd9"):
        frame = frame[:-1]
    if not frame.startswith(b"\xff\x4f"):
        raise SystemExit(f"{path.name}: frame does not start with SOC")
    if not frame.endswith(b"\xff\xd9"):
        raise SystemExit(f"{path.name}: frame does not end with EOC")
    return frame


def openjpeg_decode(path: Path) -> bytes:
    """OpenJPEG's own decode of one corpus row, through the pydicom plugin."""
    dataset = pydicom.dcmread(str(path))
    array = pydicom.pixels.pixel_array(dataset, decoding_plugin="pylibjpeg")
    if array.dtype != np.uint8 or array.ndim != 3 or array.shape[2] != 3:
        raise SystemExit(f"{path.name}: expected an interleaved RGB8 frame")
    return np.ascontiguousarray(array).tobytes()


def reference_pixels() -> bytes:
    dataset = pydicom.dcmread(str(CORPUS / "reference_rgb8_saturated.dcm"))
    return bytes(dataset.PixelData)


def write(path: Path, data: bytes) -> None:
    path.write_bytes(data)
    print(f"wrote {path.name} {digest(data)[:16]}...")


def expected_fixtures() -> dict[str, bytes]:
    """Every fixture's bytes as the corpus produces them right now."""
    expected = {}
    for fixture, row in CODESTREAMS.items():
        expected[fixture] = encapsulated_frame(CORPUS / row)
    expected[REFERENCE] = reference_pixels()
    for fixture, row in OPENJPEG_DECODES.items():
        expected[fixture] = openjpeg_decode(CORPUS / row)
    return expected


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true",
                        help="rebuild the fixtures from the ignored corpus")
    args = parser.parse_args()

    names = (*CODESTREAMS, REFERENCE, *OPENJPEG_DECODES)
    if not args.write:
        missing = [name for name in names if not (HERE / name).is_file()]
        if missing:
            print(f"missing fixtures: {', '.join(missing)}. Pass --write")
            return 1
        try:
            expected = expected_fixtures()
        except (FileNotFoundError, SystemExit) as error:
            print(f"the ignored corpus does not yield the fixtures, so the "
                  f"tracked bytes cannot be checked: {error}")
            return 1
        drifted = [name for name in names
                   if (HERE / name).read_bytes() != expected[name]]
        if drifted:
            print("tracked fixtures differ from the corpus: "
                  f"{', '.join(drifted)}")
            print("re-run with --write to rebuild them, and read the diff "
                  "before staging it")
            return 1
        print("OK: every colour fixture matches the corpus byte for byte")
        return 0

    for fixture, row in CODESTREAMS.items():
        write(HERE / fixture, encapsulated_frame(CORPUS / row))
    write(HERE / REFERENCE, reference_pixels())
    for fixture, row in OPENJPEG_DECODES.items():
        write(HERE / fixture, openjpeg_decode(CORPUS / row))
    return 0


if __name__ == "__main__":
    sys.exit(main())
