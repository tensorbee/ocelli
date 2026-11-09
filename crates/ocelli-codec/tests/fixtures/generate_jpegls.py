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

**Run-mode triggers, F-243.** ITU-T T.87 A.7.1 raises RUNindex only for a
full ``2^J[RUNindex]`` run segment. A run that reaches the end of a line with
fewer samples left is written as one ``1`` bit without raising it. The
two-band frames below reach that case, and their expected bytes are the
two-band formula ``s(r, c) = A if c < cols // 2 else B``, built here and never
by a codec. They are encoded by CharLS through imagecodecs, with one exception:
the 12-bit frame without an LSE marker is encoded by pyjpegls
``encode_buffer`` with ``bits_stored`` 12, because that is the call the second
reviewer's frame came from. For near-lossless frames the formula gives the
T.87 A.4.4 bound and a CharLS decode, stored beside the fixture, gives the
exact parity reference. imagecodecs and pyjpegls both wrap CharLS, which is in
bounds to depend on, and the versions are pinned below because a different
CharLS may encode different bytes.

Run without arguments to check the tracked bytes, or pass ``--write`` to
rebuild them. Rebuilding needs the ignored corpus present.
"""

from __future__ import annotations

import argparse
import hashlib
import sys
from pathlib import Path

import imagecodecs
import jpeg_ls
import numpy as np
import pydicom

# The CharLS build every imagecodecs-encoded fixture and every stored CharLS
# decode came from. A different build may encode different bytes, which would
# invalidate the near-lossless parity references, so the check refuses it.
IMAGECODECS_VERSION = "2026.10.10"
CHARLS_VERSION = "charls 2.4.4"
PYJPEGLS_VERSION = "1.5.1"

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
    # F-243. The banded rows reach the run-mode trigger the ramp never does.
    "jpegls_corpus_banded_lossless.jls": "jpegls_banded_lossless.dcm",
    "jpegls_corpus_banded_near3.jls": "jpegls_banded_near_lossless.dcm",
}
BANDED_REFERENCE = "jpegls_corpus_banded_reference_u16le.raw"
BANDED_REFERENCE_SOURCE = "reference_banded16.dcm"
BANDED_NEAR_CHARLS = "jpegls_corpus_banded_near3_charls.raw"
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

# The reported file's two values, 16-bit and 12-bit.
BAND_LOW, BAND_HIGH = 224, 1064

# The 8-bit levels for the default-threshold CLAMP fixture, in 4 by 4 blocks,
# ``s(r, c) = BLOCK_LEVELS[(r // 4 + c // 4) % 3]``. At P 8 and NEAR 34,
# C.2.4.1.1.1's CLAMP gives T3 = 177 where a saturating clamp gives 255, so a
# gradient from 177 upward quantizes to 4 under the first and to 3 or 4 under
# the second. The source edges are 110, 130 and 240, and near-lossless
# reconstruction moves the gradients the decoder sees by up to 2 * NEAR.
#
# The two-band formula cannot reach this. Measured on 2026-11-07, every
# two-band 8-bit frame tried at NEAR 34 (seven level pairs, four sizes) and
# four three-band frames decoded identically under both readings. This frame
# was found by the same measurement, among noise and block candidates, and is
# the replacement the plan's RED rule asks for.
BLOCK_LEVELS = (10, 140, 250)
NEAR_CLAMP = 34

# The same frame as the second reviewer's, 97 bytes, SOF55 P 12, no LSE.
TWELVE_BIT_NO_LSE_LENGTH = 97

# Two hand-built scans, P 8, one line of 13 zero samples, ISO/IEC 14495-1
# F.1 step 18, J from A.2.1 step 3. Every sample is in run mode. 0xff is eight hits with
# J = 0, 0, 0, 0, 1, 1, 1, 1, so the run reaches 1 + 1 + 1 + 1 + 2 + 2 + 2 + 2
# = 12 at RUNindex 8, where J is 2. The next byte follows 0xff, so it carries a
# stuffed zero bit and seven data bits (A.1).
#   0x30 = 0 011 0000: data bits 0 1 1 0 0 0 0. A miss, then J = 2 remainder
#   bits 11 = 3, so the run would be 12 + 3 = 15, past the 13 samples left.
#   0x40 = 0 100 0000: data bits 1 0 0 0 0 0 0. A hit whose count is
#   min(4, 13 - 12) = 1, a partial segment that ends the line.
#
# The third is one line of 4 zeros: four hits with J = 0 fill it, and the
# rest of the 0xff byte is padded with ones, followed by its stuffing byte.
# CharLS 2.4.4 refuses it because the padding is not zero, and so does Ocelli.
#
# Each entry is the scan, the number of zero samples on the line, and whether
# CharLS 2.4.4 decodes it.
RUN_HAND_SCANS = {
    "jpegls_run_miss_past_line_u8.jls": (bytes([0xFF, 0x30]), 13, False),
    "jpegls_run_partial_hit_u8.jls": (bytes([0xFF, 0x40]), 13, True),
    "jpegls_final_ff_one_padded_u8.jls": (bytes([0xFF, 0x00]), 4, False),
}


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
    banded = pydicom.dcmread(str(CORPUS / BANDED_REFERENCE_SOURCE))
    artefacts[BANDED_REFERENCE] = bytes(banded.PixelData)
    artefacts[BANDED_NEAR_CHARLS] = charls_decode(
        artefacts["jpegls_corpus_banded_near3.jls"], "<u2")

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

    artefacts.update(run_mode_fixtures())
    return artefacts


def two_band(rows: int, cols: int, low: int, high: int, dtype: type) -> np.ndarray:
    """``s(r, c) = low if c < cols // 2 else high``, the plan's formula."""
    image = np.empty((rows, cols), dtype=dtype)
    image[:, : cols // 2] = low
    image[:, cols // 2 :] = high
    return image


def charls_encode(image: np.ndarray, near: int, precision: int,
                  bits: int | None = None) -> bytes:
    """CharLS through imagecodecs, with P and NEAR asserted, not assumed."""
    encoded = bytes(imagecodecs.jpegls_encode(image, level=near, bitspersample=bits))
    if precision_of(encoded) != precision:
        raise SystemExit(f"CharLS wrote P {precision_of(encoded)}, wanted {precision}")
    if near_of(encoded) != near:
        raise SystemExit(f"CharLS wrote NEAR {near_of(encoded)}, wanted {near}")
    return encoded


def charls_decode(codestream: bytes, dtype: str) -> bytes:
    """CharLS's own decode, little-endian, the cornerstone3D parity reference."""
    return np.asarray(imagecodecs.jpegls_decode(codestream)).astype(dtype).tobytes()


def has_lse(codestream: bytes) -> bool:
    """Whether an LSE marker segment precedes SOS, ISO/IEC 14495-1 C.2.4.1."""
    offset = 2
    while offset + 4 <= len(codestream) and codestream[offset] == 0xFF:
        found = codestream[offset + 1]
        if found == 0xF8:
            return True
        if found == 0xDA:
            return False
        offset += 2 + int.from_bytes(codestream[offset + 2 : offset + 4], "big")
    raise SystemExit("SOS not found")


def charls_accepts(codestream: bytes) -> bool:
    """CharLS's verdict on a stream, recorded rather than assumed."""
    try:
        imagecodecs.jpegls_decode(codestream)
    except Exception:  # imagecodecs raises its own JpeglsError type
        return False
    return True


def hand_frame(scan: bytes, columns: int) -> bytes:
    """SOI, SOF55 P 8 one line of ``columns``, SOS NEAR 0, scan, EOI."""
    sof = bytes([0xFF, 0xF7, 0x00, 0x0B, 0x08, 0x00, 0x01, 0x00, columns,
                 0x01, 0x01, 0x11, 0x00])
    sos = bytes([0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00])
    return b"\xff\xd8" + sof + sos + scan + b"\xff\xd9"


def final_ff_frame() -> tuple[np.ndarray, bytes]:
    """The first constant 16-bit CharLS frame whose scan ends 0xff and a stuffing byte.

    Searched over 1 to 8 rows and 1 to 64 columns, all zero, so the end-of-scan
    check has a real encoder's final 0xff to accept rather than a hand-built one.
    """
    for rows in range(1, 9):
        for cols in range(1, 65):
            image = np.zeros((rows, cols), dtype=np.uint16)
            encoded = charls_encode(image, 0, 16)
            if encoded[-4] == 0xFF and encoded[-3] & 0x80 == 0:
                return image, encoded
    raise SystemExit("no constant frame ends its scan with 0xff")


def run_mode_fixtures() -> dict[str, bytes]:
    found = (imagecodecs.__version__, imagecodecs.jpegls_version(),
             jpeg_ls.__version__)
    wanted = (IMAGECODECS_VERSION, CHARLS_VERSION, PYJPEGLS_VERSION)
    if found != wanted:
        raise SystemExit(f"encoder versions {found}, wanted {wanted}")

    out: dict[str, bytes] = {}
    small = two_band(8, 4, BAND_LOW, BAND_HIGH, np.uint16)
    out["jpegls_two_band_u16_8x4.jls"] = charls_encode(small, 0, 16)
    out["jpegls_two_band_u16_8x4.raw"] = small.astype("<u2").tobytes()

    reported = two_band(32, 32, BAND_LOW, BAND_HIGH, np.uint16)
    out["jpegls_two_band_u16_32x32.jls"] = charls_encode(reported, 0, 16)
    out["jpegls_two_band_u16_32x32.raw"] = reported.astype("<u2").tobytes()
    near = charls_encode(reported, NEAR, 16)
    out["jpegls_two_band_u16_32x32_near3.jls"] = near
    out["jpegls_two_band_u16_32x32_near3_charls.raw"] = charls_decode(near, "<u2")

    twelve = bytes(jpeg_ls.encode_buffer(reported.astype("<u2").tobytes(), 32, 32, 1, 12))
    if precision_of(twelve) != 12 or near_of(twelve) != 0 or has_lse(twelve):
        raise SystemExit("12-bit two-band frame is not P 12, NEAR 0, without LSE")
    if len(twelve) != TWELVE_BIT_NO_LSE_LENGTH:
        raise SystemExit(f"12-bit two-band frame is {len(twelve)} bytes, wanted 97")
    out["jpegls_two_band_u12_32x32_no_lse.jls"] = twelve
    out["jpegls_two_band_u12_32x32_no_lse.raw"] = reported.astype("<u2").tobytes()

    final_image, final = final_ff_frame()
    out["jpegls_final_ff_u16.jls"] = final
    out["jpegls_final_ff_u16.raw"] = final_image.astype("<u2").tobytes()

    rows, cols = np.indices((32, 32))
    banded8 = np.array(BLOCK_LEVELS, dtype=np.uint8)[(rows // 4 + cols // 4) % 3]
    clamp = charls_encode(banded8, NEAR_CLAMP, 8)
    if has_lse(clamp):
        raise SystemExit("the NEAR 34 8-bit frame carries an LSE, so defaults are not exercised")
    out["jpegls_banded_u8_near34.jls"] = clamp
    out["jpegls_banded_u8_near34.raw"] = banded8.tobytes()
    out["jpegls_banded_u8_near34_charls.raw"] = charls_decode(clamp, "u1")

    for name, (scan, columns, accepted) in RUN_HAND_SCANS.items():
        frame = hand_frame(scan, columns)
        if charls_accepts(frame) != accepted:
            raise SystemExit(f"{name}: CharLS accepts {not accepted}, recorded {accepted}")
        if accepted and charls_decode(frame, "u1") != bytes(columns):
            raise SystemExit(f"{name}: CharLS does not decode {columns} zeros")
        out[name] = frame

    # Review pass 2's two smallest damaged streams that read as a valid scan of
    # another image one byte shorter, with the original final zero byte left
    # over. CharLS 2.4.4 refuses both. Each must be refused, never drawn.
    small = np.array([[2, 4, 1], [2, 114, 113], [2, 115, 114]], dtype=np.uint8)
    encoded = charls_encode(small, 0, 8)
    if encoded.hex() != ("ffd8fff7000b080003000301011100ffda00080101000000007ed000"
                         "00076bcb00ffd9"):
        raise SystemExit("the 3 by 3 P 8 stream is not review pass 2's")
    shorter = encoded[:-3] + b"\x80" + encoded[-3:]
    ramp = np.array([[0, 32765, 65535]] * 3, dtype=np.uint16)
    flipped = bytearray(charls_encode(ramp, 2, 16))
    # The least significant bit of the first scan byte.
    flipped[scan_offset(bytes(flipped))] ^= 0x01
    for name, damaged in (("jpegls_damaged_shorter_u8_3x3.jls", shorter),
                          ("jpegls_damaged_shorter_u16_near2_3x3.jls", bytes(flipped))):
        if charls_accepts(damaged):
            raise SystemExit(f"{name}: CharLS opens it, recorded refused")
        out[name] = damaged

    ct = ct_like(64, 12)
    out["jpegls_ct_like_u12_64.jls"] = charls_encode(ct, 0, 12, bits=12)
    out["jpegls_ct_like_u12_64.raw"] = ct.astype("<u2").tobytes()
    near_ct = charls_encode(ct, 2, 12, bits=12)
    out["jpegls_ct_like_u12_64_near2.jls"] = near_ct
    out["jpegls_ct_like_u12_64_near2_charls.raw"] = charls_decode(near_ct, "<u2")

    streams, verdicts = charls_corpus(out)
    out[CHARLS_STREAMS] = streams
    out[CHARLS_VERDICTS] = verdicts
    return out


def ct_like(size: int, bits: int) -> np.ndarray:
    """A synthetic CT-like slice: a disc of soft tissue at 1040 in air at 24,
    a brighter core, a smooth texture and Gaussian noise of sigma 18, from a
    fixed seed. No corpus pixel goes into it. Real CT slices encoded this way
    reach a context RESET with an odd negative B, which this reproduces."""
    rng = np.random.default_rng(1)
    rows, cols = np.indices((size, size))
    radius = np.hypot(rows - size / 2, cols - size / 2) / (size / 2)
    body = (np.where(radius < 0.85, 1040.0, 24.0) + np.where(radius < 0.3, 200.0, 0.0)
            + 60 * np.sin(cols / 9.0) * np.cos(rows / 13.0))
    image = body + rng.normal(0, 18, (size, size))
    return np.clip(np.rint(image), 0, (1 << bits) - 1).astype(np.uint16)


# The CharLS agreement corpus, F-243 by the lead's ruling D3. Deterministic
# CharLS 2.4.4 streams over P 8, 12 and 16, NEAR 0 and 3, four shapes and six
# contents, plus the run-mode, end-of-scan and CT-like fixtures above, each
# with the mutations below: the stream itself, its final 1, 2, 3 or 5 scan
# bytes dropped, bytes inserted before EOI (zero bytes, non-zero bytes, an
# 0xff with its stuffing, a DNL, a second EOI and 0xff fill bytes), zero bytes,
# 0x80 and EOI inserted at the start of the scan, a third of the way in and
# before its last byte, every single-bit flip in the first and in the last
# three scan bytes, and the scan cut to 10, 50 and 90 per cent. Each case
# records its damage class, CharLS's verdict and, when CharLS opens the
# stream, the first 16 hex digits of the SHA-256 of its little-endian samples. The Rust test
# applies the same mutations and holds Ocelli to the same verdict and the same
# samples, and lists by name every case where the two differ.
CHARLS_STREAMS = "jpegls_charls_streams.tsv"
CHARLS_VERDICTS = "jpegls_charls_verdicts.tsv"
CORPUS_CONTENTS = ("flat", "twoband", "blocks", "disc", "ramp", "random")
CORPUS_SHAPES = ((1, 13), (8, 4), (5, 100), (17, 29))
MUTATIONS = (
    ["none"] + [f"drop:{count}" for count in (1, 2, 3, 5)]
    + [f"ins:{inserted}" for inserted in
       ("00", "0000", "01", "80", "7f", "ff00", "ffdc00040001", "ffd9", "ff", "ffff")]
    + [f"at:{where}:{inserted}" for where in ("start", "third", "last")
       for inserted in ("00", "0000", "80", "ffd9")]
    + [f"flip:{back}:{bit}" for back in (1, 2, 3) for bit in range(8)]
    + [f"lead:{bit}" for bit in range(8)]
    + [f"cut:{percent}" for percent in (10, 50, 90)])


def damage_class(mutation: str) -> str:
    """The named damage class a mutation belongs to, independent of any decoder."""
    kind, _, argument = mutation.partition(":")
    if kind == "none":
        return "undamaged"
    if kind == "ins":
        if argument in ("00", "0000"):
            return "zero bytes before EOI"
        if argument in ("ff", "ffff"):
            return "fill bytes before EOI"
        return "other bytes before EOI"
    if kind == "at":
        inserted = argument.split(":")[1]
        if inserted in ("00", "0000"):
            return "zero bytes inside the scan"
        if inserted == "ffd9":
            return "early EOI"
        return "a byte inserted inside the scan"
    if kind in ("drop", "cut"):
        return "dropped final bytes"
    return "flipped bit"


def corpus_image(kind: str, rows: int, cols: int, bits: int) -> np.ndarray:
    top = (1 << bits) - 1
    r, c = np.indices((rows, cols))
    low, high = top // 7, (top * 5) // 7
    if kind == "flat":
        image = np.full((rows, cols), low)
    elif kind == "twoband":
        image = np.where(c < cols // 2, low, high)
    elif kind == "blocks":
        image = np.array((low // 2, low + top // 4, top - top // 60))[(r // 4 + c // 4) % 3]
    elif kind == "disc":
        image = np.where(np.hypot(r - rows / 2, c - cols / 2) < min(rows, cols) / 3, high, low)
    elif kind == "ramp":
        image = (r * cols + c) * top // max(1, rows * cols - 1)
    else:
        image = np.random.default_rng(rows * 1000 + cols + bits).integers(0, top + 1, (rows, cols))
    return image.astype(np.uint8 if bits <= 8 else np.uint16)


def scan_offset(codestream: bytes) -> int:
    """Offset of the first entropy-coded byte, just past the SOS segment."""
    offset = 2
    while True:
        length = int.from_bytes(codestream[offset + 2 : offset + 4], "big")
        if codestream[offset + 1] == 0xDA:
            return offset + 2 + length
        offset += 2 + length


def mutate(codestream: bytes, mutation: str) -> bytes | None:
    """One named mutation, exactly as `tests/jpegls.rs` applies it."""
    start = scan_offset(codestream)
    scan = len(codestream) - 2 - start
    body, eoi = codestream[:-2], codestream[-2:]
    kind, _, argument = mutation.partition(":")
    if kind == "none":
        return codestream
    if kind == "drop":
        count = int(argument)
        return None if count >= scan else body[:-count] + eoi
    if kind == "ins":
        return body + bytes.fromhex(argument) + eoi
    if kind == "at":
        where, inserted = argument.split(":")
        index = start + {"start": 0, "third": scan // 3, "last": scan - 1}[where]
        return codestream[:index] + bytes.fromhex(inserted) + codestream[index:]
    if kind == "lead":
        flipped = bytearray(codestream)
        flipped[start] ^= 1 << int(argument)
        return bytes(flipped)
    if kind == "flip":
        back, bit = (int(value) for value in argument.split(":"))
        if back > scan:
            return None
        flipped = bytearray(codestream)
        flipped[len(codestream) - 2 - back] ^= 1 << bit
        return bytes(flipped)
    if kind == "cut":
        return codestream[: start + scan * int(argument) // 100] + eoi
    raise SystemExit(f"unknown mutation {mutation}")


def charls_digest(codestream: bytes) -> str:
    """CharLS's verdict: `refuse`, or the digest of its little-endian samples."""
    try:
        decoded = np.asarray(imagecodecs.jpegls_decode(codestream))
    except Exception:  # imagecodecs raises its own JpeglsError type
        return "refuse"
    dtype = "<u2" if precision_of(codestream) > 8 else "u1"
    return hashlib.sha256(decoded.astype(dtype).tobytes()).hexdigest()[:16]


def charls_corpus(artefacts: dict[str, bytes]) -> tuple[bytes, bytes]:
    streams: list[tuple[str, bytes]] = []
    for bits in (8, 12, 16):
        for near in (0, 3):
            for rows, cols in CORPUS_SHAPES:
                for kind in CORPUS_CONTENTS:
                    image = corpus_image(kind, rows, cols, bits)
                    streams.append((f"p{bits}_n{near}_{rows}x{cols}_{kind}",
                                    charls_encode(image, near, bits, bits=bits)))
    for name in ("jpegls_two_band_u16_8x4.jls", "jpegls_final_ff_u16.jls",
                 "jpegls_run_partial_hit_u8.jls", "jpegls_final_ff_one_padded_u8.jls",
                 "jpegls_ct_like_u12_64.jls", "jpegls_ct_like_u12_64_near2.jls"):
        streams.append((name.removesuffix(".jls"), artefacts[name]))
    stream_lines = [f"{name}\t{codestream.hex()}" for name, codestream in streams]
    verdict_lines = []
    for name, codestream in streams:
        for mutation in MUTATIONS:
            mutated = mutate(codestream, mutation)
            if mutated is not None:
                verdict_lines.append(
                    f"{name}\t{mutation}\t{damage_class(mutation)}\t{charls_digest(mutated)}")
    header = "# written by generate_jpegls.py, CharLS 2.4.4 through imagecodecs 2026.10.10"
    return (("\n".join([header, *stream_lines]) + "\n").encode("ascii"),
            ("\n".join([header, *verdict_lines]) + "\n").encode("ascii"))


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
