#!/usr/bin/env python3
"""Reproduce the scalar-derived QCD fixture and independent raw reference.

The source fixture was generated from a 64 by 64 constant-128 unsigned image
by ritk-codecs 0.6.0 with five levels and scalar step 4, then its COD transform
byte was changed to reversible. This script changes only its QCD representation
from scalar expounded style 2 to scalar derived style 1. OpenJPEG 2.5.4 then
provides the independent decoded reference.

Run without arguments to check tracked bytes, or pass ``--write`` to replace
them after independently installing ``opj_decompress`` 2.5.4.
"""

from __future__ import annotations

import argparse
import hashlib
import subprocess
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SOURCE = HERE / "jpeg2000_quantized_reversible_invalid.j2k"
CODESTREAM = HERE / "jpeg2000_scalar_derived_u8.j2k"
REFERENCE = HERE / "jpeg2000_scalar_derived_u8_openjpeg.raw"

SOURCE_SHA256 = "c1b3005328ac117e804f779b3c9f1d39add889abef950192f11d95d9e74a6954"
CODESTREAM_SHA256 = "78eb076398c0be23f9df5903d3915caf54e16571f741567bb227b842d775df7c"
REFERENCE_SHA256 = "78aacbc3fb34efb8ffa5467b931291ec2bdf5e19564fc45fe97b5affbc893dc6"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def scalar_derived(source: bytes) -> bytes:
    if sha256(source) != SOURCE_SHA256:
        raise ValueError("source fixture digest does not match its recorded provenance")
    qcd = source.find(b"\xff\x5c")
    if qcd < 0 or source.find(b"\xff\x5c", qcd + 2) >= 0:
        raise ValueError("source fixture must carry exactly one QCD marker")
    length = int.from_bytes(source[qcd + 2:qcd + 4], "big")
    payload = source[qcd + 4:qcd + 2 + length]
    if length != 35 or len(payload) != 33 or payload[0] & 0x1f != 2:
        raise ValueError("source fixture is not the recorded scalar-expounded QCD")
    qcd_style_one = b"\xff\x5c\x00\x05" + bytes([(payload[0] & 0xe0) | 1]) + payload[1:3]
    return source[:qcd] + qcd_style_one + source[qcd + 2 + length:]


def pgm_pixels(pgm: bytes) -> bytes:
    position = 0
    tokens: list[bytes] = []
    while len(tokens) < 4:
        while position < len(pgm) and pgm[position] in b" \t\r\n":
            position += 1
        if position < len(pgm) and pgm[position] == ord("#"):
            position = pgm.find(b"\n", position)
            if position < 0:
                raise ValueError("OpenJPEG PGM comment is unterminated")
            continue
        end = position
        while end < len(pgm) and pgm[end] not in b" \t\r\n":
            end += 1
        tokens.append(pgm[position:end])
        position = end
    if tokens != [b"P5", b"64", b"64", b"255"]:
        raise ValueError(f"OpenJPEG PGM header is unexpected: {tokens!r}")
    if position >= len(pgm) or pgm[position] not in b" \t\r\n":
        raise ValueError("OpenJPEG PGM header has no data separator")
    pixels = pgm[position + 1:]
    if len(pixels) != 4096:
        raise ValueError(f"OpenJPEG produced {len(pixels)} bytes, expected 4096")
    return pixels


def reproduce() -> tuple[bytes, bytes]:
    codestream = scalar_derived(SOURCE.read_bytes())
    with tempfile.TemporaryDirectory(prefix="ocelli-j2k-style1-") as directory:
        temporary = Path(directory)
        encoded = temporary / "style1.j2k"
        decoded = temporary / "style1.pgm"
        encoded.write_bytes(codestream)
        subprocess.run(
            ["opj_decompress", "-i", str(encoded), "-o", str(decoded)],
            check=True,
            capture_output=True,
        )
        reference = pgm_pixels(decoded.read_bytes())
    if sha256(codestream) != CODESTREAM_SHA256 or sha256(reference) != REFERENCE_SHA256:
        raise ValueError("reproduced fixture or OpenJPEG reference digest changed")
    return codestream, reference


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    codestream, reference = reproduce()
    if args.write:
        CODESTREAM.write_bytes(codestream)
        REFERENCE.write_bytes(reference)
    elif CODESTREAM.read_bytes() != codestream or REFERENCE.read_bytes() != reference:
        raise ValueError("tracked scalar-derived fixture or reference is stale")
    print("OK: scalar-derived fixture and OpenJPEG 2.5.4 reference reproduce exactly")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
