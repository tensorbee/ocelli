#!/usr/bin/env python3
"""Build the JPEG-LS codestream fixtures and their independent references.

Two kinds of fixture, and the difference matters.

**Manifest-backed.** The two corpus rows `syntax/jpegls_lossless.dcm` and
`syntax/jpegls_near_lossless.dcm` are extracted to bare codestreams, together
with the Pixel Data of `syntax/explicit_vr_le.dcm`, which is the same synthetic
ramp uncompressed. That uncompressed frame is the **encoder-independent anchor**
for the lossless row: the syntax is lossless, so a correct decoder must
reproduce it exactly, and it comes from `scripts/corpus_synth.py` rather than
from any codec.

**Synthetic.** A 256 by 256 frame carrying every one of the 65,536 unsigned
16-bit values exactly once, plus a 12-bit variant. These exist because
`docs/sprints/CURRENT_SPRINT.md` requires the stored-domain round trip proven
over the full range rather than over a sample: the vendored decoder returns
`Vec<f32>`, and an `f32` that carries a 16-bit stored value exactly for every
value except the ones near the top of the range is the quietly-wrong pixel this
repository exists to catch. The expected bytes are the ramp itself, constructed
here, so nothing in the chain depends on what Ocelli produces.

`pyjpegls` 1.5.1 wraps CharLS and is the encoder. **It is not an oracle for the
decode**: agreeing with a CharLS encode is not independent evidence about a
CharLS decode. The lossless anchor is the uncompressed reference and the
near-lossless anchor is ISO/IEC 14495-1's NEAR bound, both of which are
independent of CharLS.

Run without arguments to check the tracked bytes, or pass ``--write`` to
rebuild them. Rebuilding needs the ignored corpus present.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import jpeg_ls
import numpy as np
import pydicom

HERE = Path(__file__).resolve().parent
CORPUS = HERE.parents[3] / "corpus" / "data" / "syntax"

# ISO/IEC 14495-1 NEAR, matching scripts/corpus_synth.py's JPEG_LS_NEAR.
NEAR = 3

# One encapsulated frame per corpus row.
MANIFEST_ROWS = {
    "jpegls_corpus_lossless.jls": "jpegls_lossless.dcm",
    "jpegls_corpus_near_lossless.jls": "jpegls_near_lossless.dcm",
    # F-030 added `syntax/jpegls_lossless_rgb8.dcm`, the multi-component row
    # Appendix A gate A2 recorded as owed. It is sample-interleaved, ILV = 2,
    # where MULTI_COMPONENT below is line-interleaved, ILV = 1. The adapter's
    # condition is `interleave != 0`, so the two cover different values of it
    # and the corpus-backed one is the row A2 asked for.
    "jpegls_corpus_rgb8.jls": "jpegls_lossless_rgb8.dcm",
}
# Nf and ILV the manifest-backed multi-component row must declare, asserted in
# `build` rather than assumed, because the row's whole purpose is those two
# numbers.
CORPUS_RGB = ("jpegls_corpus_rgb8.jls", 3, 2)
REFERENCE = "jpegls_corpus_reference_u16le.raw"
REFERENCE_SOURCE = "explicit_vr_le.dcm"


def full_range_u16() -> np.ndarray:
    """Every unsigned 16-bit value exactly once, 256 by 256.

    Ascending, so an off-by-one in the unpack shifts every sample rather than
    cancelling, and so the top of the range is present rather than sampled.
    """
    return np.arange(65536, dtype=np.uint16).reshape(256, 256)


def ramp_u12() -> np.ndarray:
    """Every unsigned 12-bit value exactly once, 64 by 64.

    Bits Stored 12 in a 16-bit container is the normal case for CT and CR, not
    the exception, and it is where the mask-and-sign-extend traps live.
    """
    return np.arange(4096, dtype=np.uint16).reshape(64, 64)


SYNTHETIC = {
    "jpegls_full_range_u16": (full_range_u16, 16, 0),
    "jpegls_ramp_u12": (ramp_u12, 12, 0),
    "jpegls_ramp_u12_near3": (ramp_u12, 12, NEAR),
}

# A DICOM JPEG-LS frame can be RGB and neither route Appendix A gate A2
# measured decodes one, so the adapter must refuse a multi-component codestream
# rather than produce a wrong pixel. The refusal cannot be reached through a
# descriptor check alone, because a colour descriptor is rejected before the
# codestream is read, so a genuinely multi-component codestream is needed to
# exercise the header check at all.
MULTI_COMPONENT = "jpegls_rgb8_ilv1.jls"


def encapsulated_frame(path: Path) -> bytes:
    """The single encoded frame of a one-frame encapsulated instance."""
    dataset = pydicom.dcmread(str(path))
    # `generate_fragments` also yields the Basic Offset Table item, so frames
    # are assembled rather than fragments counted.
    frames = list(pydicom.encaps.generate_frames(dataset.PixelData, number_of_frames=1))
    if len(frames) != 1:
        raise SystemExit(f"{path.name}: expected one frame, found {len(frames)}")
    first = frames[0]
    # PS3.5 A.4 pads an odd fragment to an even length. JPEG-LS ends at EOI,
    # so a trailing pad byte is not part of the codestream.
    if first.endswith(b"\x00") and not first.endswith(b"\xff\xd9"):
        first = first[:-1]
    return first


def build() -> dict[str, bytes]:
    artefacts: dict[str, bytes] = {}

    for name, source in MANIFEST_ROWS.items():
        artefacts[name] = encapsulated_frame(CORPUS / source)

    name, want_components, want_interleave = CORPUS_RGB
    found = (components_of(artefacts[name]), interleave_of(artefacts[name]))
    if found != (want_components, want_interleave):
        raise SystemExit(
            f"{name}: SOF55 Nf and SOS ILV are {found}, wanted "
            f"{(want_components, want_interleave)}")

    reference = pydicom.dcmread(str(CORPUS / REFERENCE_SOURCE))
    artefacts[REFERENCE] = bytes(reference.PixelData)

    for stem, (make, bits_stored, near) in SYNTHETIC.items():
        image = make()
        encoded = jpeg_ls.encode_array(image, lossy_error=near)
        artefacts[f"{stem}.jls"] = bytes(encoded)
        artefacts[f"{stem}.raw"] = image.tobytes()
        # The encoder must have used the declared precision, or the fixture is
        # describing a different frame from the one the test claims.
        declared = precision_of(bytes(encoded))
        if declared != bits_stored:
            raise SystemExit(f"{stem}: SOF55 precision {declared}, wanted {bits_stored}")
        if near_of(bytes(encoded)) != near:
            raise SystemExit(f"{stem}: SOS NEAR {near_of(bytes(encoded))}, wanted {near}")

    rgb = np.zeros((16, 16, 3), dtype=np.uint8)
    rgb[..., 0] = np.arange(256, dtype=np.uint8).reshape(16, 16)
    rgb[..., 1] = 128
    rgb[..., 2] = 255 - rgb[..., 0]
    # Line-interleaved, so the codestream declares Nf = 3 and ILV = 1 and both
    # of the adapter's header conditions have something to refuse.
    encoded = bytes(jpeg_ls.encode_array(rgb, interleave_mode=1))
    if components_of(encoded) != 3:
        raise SystemExit(f"{MULTI_COMPONENT}: SOF55 Nf {components_of(encoded)}, wanted 3")
    artefacts[MULTI_COMPONENT] = encoded

    return artefacts


def components_of(codestream: bytes) -> int:
    """Number of components Nf from SOF55, ISO/IEC 14495-1 C.2.2."""
    offset = marker_offset(codestream, 0xF7)
    return codestream[offset + 9]


def interleave_of(codestream: bytes) -> int:
    """ILV from the SOS marker segment, ISO/IEC 14495-1 C.2.3.

    The segment is `Ls Ns (Ci Tm)*Ns NEAR ILV Al/Ah`, so ILV sits one byte past
    NEAR, which is itself two bytes per component past `Ns`.
    """
    offset = marker_offset(codestream, 0xDA)
    payload = codestream[offset + 4 : offset + 2 + int.from_bytes(codestream[offset + 2 : offset + 4], "big")]
    components = payload[0]
    return payload[2 + 2 * components]


def precision_of(codestream: bytes) -> int:
    """Sample precision P from the SOF55 marker segment, ISO/IEC 14495-1 C.2.2."""
    offset = marker_offset(codestream, 0xF7)
    return codestream[offset + 4]


def near_of(codestream: bytes) -> int:
    """NEAR from the SOS marker segment, ISO/IEC 14495-1 C.2.3.

    The segment is `Ls Ns (Ci Tm)*Ns NEAR ILV Al/Ah`, so NEAR sits two bytes per
    component past `Ns`. Reading it from the end of the segment instead lands on
    ILV, which is zero for every non-interleaved frame and therefore looks like
    a lossless NEAR on a near-lossless codestream.
    """
    offset = marker_offset(codestream, 0xDA)
    payload = codestream[offset + 4 : offset + 2 + int.from_bytes(codestream[offset + 2 : offset + 4], "big")]
    components = payload[0]
    return payload[1 + 2 * components]


def marker_offset(codestream: bytes, marker: int) -> int:
    offset = 2  # past SOI
    while offset + 4 <= len(codestream):
        if codestream[offset] != 0xFF:
            raise SystemExit(f"marker 0x{marker:02x} not found")
        found = codestream[offset + 1]
        if found == marker:
            return offset
        length = int.from_bytes(codestream[offset + 2 : offset + 4], "big")
        offset += 2 + length
    raise SystemExit(f"marker 0x{marker:02x} not found")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    arguments = parser.parse_args()

    artefacts = build()
    failures = 0
    for name, payload in sorted(artefacts.items()):
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
