#!/usr/bin/env python3
"""Write the DCMTK anchor decodes for gate A2.

THROWAWAY SPIKE CODE. F-X006. Not held to the gate set.

Runs `dcmdjpls` over the two JPEG-LS corpus rows and writes each decoded
`PixelData` into ignored `tools/spikes/out/` as canonical 12288-byte,
little-endian, 6144-sample buffers.

**THE ANCHOR IS CharLS ON BOTH SIDES AND THAT IS THE POINT OF THIS COMMENT.**
`scripts/corpus_synth.py` encoded these two rows with `pyjpegls` 1.5.1, which
wraps CharLS. `dcmdjpls` is DCMTK 3.7.0, which also uses CharLS. So a candidate
agreeing with `dcmdjpls` has agreed with a second reading of the same
implementation, not with an independent one.

The two anchors that are independent are:

- `reference.raw`, the uncompressed `syntax/explicit_vr_le.dcm` PixelData, which
  is exact for `1.2.840.10008.1.2.4.80` because that syntax is lossless. It
  comes from `scripts/corpus_synth.py`'s hand-predictable ramp and not from any
  codec.
- ISO/IEC 14495-1's guarantee for `1.2.840.10008.1.2.4.81`, that the maximum
  absolute error is at most `NEAR`, which `scripts/corpus_synth.py` sets to
  `JPEG_LS_NEAR = 3`. That is the standard rather than an implementation.

Usage:
  uv run tools/spikes/a2-jpeg-ls/anchors.py
"""

from __future__ import annotations

import hashlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import pydicom

ROOT = Path(__file__).resolve().parent.parent.parent.parent
CORPUS = ROOT / "corpus" / "data" / "syntax"
OUT = ROOT / "tools" / "spikes" / "out"

CASES = {
    "jpegls_lossless": "1.2.840.10008.1.2.4.80",
    "jpegls_near_lossless": "1.2.840.10008.1.2.4.81",
}

CANONICAL_BYTES = 12288


def main() -> int:
    if shutil.which("dcmdjpls") is None:
        raise SystemExit("dcmdjpls not on PATH. DCMTK 3.7.0 is the A2 anchor.")

    version = subprocess.run(["dcmdjpls", "--version"], capture_output=True,
                             text=True, check=False).stdout.splitlines()
    print(version[0] if version else "dcmdjpls version unknown")

    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as scratch:
        for name in CASES:
            source = CORPUS / f"{name}.dcm"
            if not source.exists():
                raise SystemExit(f"missing {source}. See corpus/README.md.")
            decoded = Path(scratch) / f"{name}.dcm"
            # `+te` writes explicit VR little endian, which is the canonical
            # byte order and needs no swap on the way out.
            subprocess.run(["dcmdjpls", "+te", str(source), str(decoded)],
                           check=True, capture_output=True)
            ds = pydicom.dcmread(decoded)
            raw = bytes(ds.PixelData)
            if len(raw) != CANONICAL_BYTES:
                raise SystemExit(f"{name}: {len(raw)} bytes, "
                                 f"expected {CANONICAL_BYTES}")
            target = OUT / f"dcmtk_{name}.raw"
            target.write_bytes(raw)
            digest = hashlib.sha256(raw).hexdigest()
            print(f"{target.name:32s} {len(raw):6d} bytes  sha256 {digest}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
