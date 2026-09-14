#!/usr/bin/env python3
"""Extract the three HTJ2K codestream fixtures and their reference.

The corpus rows `syntax/htj2k_lossless.dcm`, `syntax/htj2k_lossless_rpcl.dcm`
and `syntax/htj2k_lossy.dcm` are `corpus/manifest.tsv` lines 35, 36 and 37, all
encoded from the same synthetic ramp by `scripts/corpus_synth.py`. The
uncompressed form of that ramp is `syntax/explicit_vr_le.dcm`'s Pixel Data.

**The reference is an exact anchor for `.201` and `.202` and is not one for
`.203`.** The two lossless syntaxes must reproduce it byte for byte. The
irreversible syntax is lossy by design, so its output is compared against the
divergence F-X013 measured rather than against the ramp.

The reference file is not written again here. `generate_jpegls.py` already
writes `jpegls_corpus_reference_u16le.raw` from the same source, and a second
copy of the same 12,288 bytes under a second name would be a second thing to
keep in step. This script verifies that file's digest instead.

Run without arguments to check the tracked bytes, or pass ``--write`` to rebuild
them. Rebuilding needs the ignored corpus present.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import pydicom

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parents[3] / "corpus" / "data" / "syntax"

MANIFEST_ROWS = {
    "htj2k_corpus_lossless.j2c": "htj2k_lossless.dcm",
    "htj2k_corpus_lossless_rpcl.j2c": "htj2k_lossless_rpcl.dcm",
    "htj2k_corpus_lossy.j2c": "htj2k_lossy.dcm",
}

# Written by generate_jpegls.py from syntax/explicit_vr_le.dcm. Shared rather
# than duplicated, and its digest is the one docs/spikes/A2-jpeg-ls.md and
# docs/spikes/A1-htj2k-route.md both record for `R`.
SHARED_REFERENCE = "jpegls_corpus_reference_u16le.raw"
SHARED_REFERENCE_SHA256 = (
    "b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609"
)


def encapsulated_frame(path: Path) -> bytes:
    """The single encoded frame of a one-frame encapsulated instance."""
    dataset = pydicom.dcmread(str(path))
    frames = list(pydicom.encaps.generate_frames(dataset.PixelData, number_of_frames=1))
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
    if not has_cap(frame):
        raise SystemExit(f"{path.name}: frame carries no CAP marker, so it is not HTJ2K")
    return frame


def has_cap(codestream: bytes) -> bool:
    """Whether the main header carries CAP (0xff50), ISO/IEC 15444-1 A.5.2.

    CAP is what distinguishes an HTJ2K codestream from JPEG 2000 Part 1. A
    fixture without it would be testing the wrong thing entirely.
    """
    offset = 2  # past SOC
    while offset + 4 <= len(codestream):
        if codestream[offset] != 0xFF:
            return False
        marker = codestream[offset + 1]
        if marker == 0x90:  # SOT, end of the main header
            return False
        if marker == 0x50:
            return True
        length = int.from_bytes(codestream[offset + 2 : offset + 4], "big")
        offset += 2 + length
    return False


def progression_order(codestream: bytes) -> int:
    """SGcod's progression order byte from COD, ISO/IEC 15444-1 A.6.1.

    0 is LRCP, 1 RLCP, 2 RPCL, 3 PCRL, 4 CPRL. PS3.5 requires RPCL for transfer
    syntax `.202` and constrains neither `.201` nor `.203`, so this is a
    one-directional discriminator rather than a partition.
    """
    offset = 2
    while offset + 4 <= len(codestream):
        if codestream[offset] != 0xFF:
            raise SystemExit("COD not found")
        marker = codestream[offset + 1]
        length = int.from_bytes(codestream[offset + 2 : offset + 4], "big")
        if marker == 0x52:
            return codestream[offset + 5]
        if marker == 0x90:
            raise SystemExit("COD not found before SOT")
        offset += 2 + length
    raise SystemExit("COD not found")


def is_reversible(codestream: bytes) -> bool:
    """SPcod's wavelet transform byte from COD, ISO/IEC 15444-1 A.6.1.

    1 is the reversible 5/3 kernel and 0 is the irreversible 9/7 one. This is
    what separates transfer syntaxes `.201` and `.202` from `.203`.
    """
    offset = 2
    while offset + 4 <= len(codestream):
        if codestream[offset] != 0xFF:
            raise SystemExit("COD not found")
        marker = codestream[offset + 1]
        length = int.from_bytes(codestream[offset + 2 : offset + 4], "big")
        if marker == 0x52:
            return codestream[offset + 13] == 1
        if marker == 0x90:
            raise SystemExit("COD not found before SOT")
        offset += 2 + length
    raise SystemExit("COD not found")


# What each fixture must carry for the test that uses it to mean anything.
# `progression` is the SGcod byte and `reversible` is the SPcod wavelet byte.
#
# **PS3.5 constrains RPCL for `.202` only.** `.201` has no progression
# constraint, so a `.202` codestream is also a valid `.201` one and the two are
# not mutually exclusive. The fixtures are chosen so the distinction is
# visible anyway: the `.201` row is LRCP, so a test can show that `.202`
# refuses it, and the `.202` row is RPCL, so a test can show that `.201`
# accepts it. Requiring the `.201` row to be non-RPCL is a property of these
# fixtures, not of the transfer syntax.
EXPECTED = {
    "htj2k_corpus_lossless.j2c": {"progression": 0, "reversible": True},
    "htj2k_corpus_lossless_rpcl.j2c": {"progression": 2, "reversible": True},
    "htj2k_corpus_lossy.j2c": {"progression": 2, "reversible": False},
}


def build() -> dict[str, bytes]:
    artefacts: dict[str, bytes] = {}
    for name, source in MANIFEST_ROWS.items():
        frame = encapsulated_frame(CORPUS / source)
        want = EXPECTED[name]
        order = progression_order(frame)
        if order != want["progression"]:
            raise SystemExit(f"{name}: progression order {order}, wanted {want['progression']}")
        if is_reversible(frame) != want["reversible"]:
            raise SystemExit(f"{name}: reversibility is not {want['reversible']}")
        artefacts[name] = frame
    return artefacts


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()

    shared = HERE / SHARED_REFERENCE
    if not shared.exists():
        print(f"MISSING {SHARED_REFERENCE}, which generate_jpegls.py writes")
        return 1
    digest = hashlib.sha256(shared.read_bytes()).hexdigest()
    if digest != SHARED_REFERENCE_SHA256:
        print(f"DIFFERS {SHARED_REFERENCE}  {digest}")
        return 1
    print(f"ok      {SHARED_REFERENCE}  shared with the JPEG-LS fixtures")

    failures = 0
    for name, payload in sorted(build().items()):
        target = HERE / name
        digest = hashlib.sha256(payload).hexdigest()
        if arguments.write:
            target.write_bytes(payload)
            print(f"wrote {name}  {len(payload)} bytes  {digest}")
            continue
        if not target.exists():
            print(f"MISSING {name}")
            failures += 1
        elif target.read_bytes() != payload:
            print(f"DIFFERS {name}  rebuilt {digest}")
            failures += 1
        else:
            print(f"ok      {name}  {len(payload)} bytes  {digest}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
