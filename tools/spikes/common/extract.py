#!/usr/bin/env python3
"""Extract the five compressed codestreams and the uncompressed reference.

THROWAWAY SPIKE CODE. F-X006 answers Appendix A gates A1 and A2 and this
script exists to feed those two measurements. It is not held to the gate set
per `/spike` step 2, it is imported by nothing under `crates/` or
`tools/oracle/`, and it is deleted when the gates are closed.

What it writes, all into ignored `tools/spikes/out/`:

  htj2k_lossless.j2c        1.2.840.10008.1.2.4.201, HTJ2K reversible
  htj2k_lossless_rpcl.j2c   1.2.840.10008.1.2.4.202, HTJ2K reversible RPCL
  htj2k_lossy.j2c           1.2.840.10008.1.2.4.203, HTJ2K irreversible
  jpegls_lossless.jls       1.2.840.10008.1.2.4.80
  jpegls_near_lossless.jls  1.2.840.10008.1.2.4.81, NEAR 3
  j2k_lossless.j2c          1.2.840.10008.1.2.4.90, the A1 CONTROL
  j2k_lossy.j2c             1.2.840.10008.1.2.4.91, the A1 CONTROL
  reference.raw             explicit_vr_le.dcm PixelData, the anchor R

The two `j2k_*` rows are not part of either gate's question. They are JPEG 2000
Part 1, decoded by the same `openjp2` build through a different code path from
Part 15's, and they separate "openjp2 does not work on wasm32" from "openjp2's
HTJ2K path does not work on wasm32". Those are different findings with
different consequences, and without a control they are indistinguishable.

`reference.raw` is the canonical form every decode is reduced to: 12288 bytes,
little-endian u16, 6144 samples, 64 rows by 96 columns. That shape is not
chosen. It is what `scripts/corpus_synth.py` produced, and every one of the
five compressed cases was encoded from `syntax/explicit_vr_le.dcm`.

Nothing under `tools/spikes/out/` is ever committed. `.gitignore` covers it and
`scripts/staged_content_check.py` refuses it by path, because `git add -f`
exists and a codestream extracted from a corpus row is derived from that row.

Usage:
  uv run tools/spikes/common/extract.py
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

import pydicom
from pydicom.encaps import generate_pixel_data_frame

ROOT = Path(__file__).resolve().parent.parent.parent.parent
CORPUS = ROOT / "corpus" / "data" / "syntax"
OUT = ROOT / "tools" / "spikes" / "out"

REFERENCE = "explicit_vr_le.dcm"

# case name -> (corpus file, output suffix, expected transfer syntax UID)
CASES = {
    "htj2k_lossless": ("htj2k_lossless.dcm", ".j2c", "1.2.840.10008.1.2.4.201"),
    "htj2k_lossless_rpcl": ("htj2k_lossless_rpcl.dcm", ".j2c",
                            "1.2.840.10008.1.2.4.202"),
    "htj2k_lossy": ("htj2k_lossy.dcm", ".j2c", "1.2.840.10008.1.2.4.203"),
    "jpegls_lossless": ("jpegls_lossless.dcm", ".jls",
                        "1.2.840.10008.1.2.4.80"),
    "jpegls_near_lossless": ("jpegls_near_lossless.dcm", ".jls",
                             "1.2.840.10008.1.2.4.81"),
    # Controls, JPEG 2000 Part 1. See the header.
    "j2k_lossless": ("j2k_lossless.dcm", ".j2c", "1.2.840.10008.1.2.4.90"),
    "j2k_lossy": ("j2k_lossy.dcm", ".j2c", "1.2.840.10008.1.2.4.91"),
}

CANONICAL_BYTES = 12288
CANONICAL_ROWS = 64
CANONICAL_COLS = 96


def digest(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def check_geometry(ds: pydicom.Dataset, label: str) -> None:
    """Refuse anything that is not the canonical 64 by 96 unsigned 16-bit.

    A geometry surprise here would present later as a total mismatch and would
    read as a decoder defect, which is exactly the confusion this spike must
    not create.
    """
    actual = (int(ds.Rows), int(ds.Columns), int(ds.BitsAllocated),
              int(ds.PixelRepresentation))
    expected = (CANONICAL_ROWS, CANONICAL_COLS, 16, 0)
    if actual != expected:
        raise SystemExit(f"{label}: geometry {actual}, expected {expected}")


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)

    reference_path = CORPUS / REFERENCE
    if not reference_path.exists():
        raise SystemExit(f"missing {reference_path}. See corpus/README.md.")
    ds = pydicom.dcmread(reference_path)
    check_geometry(ds, REFERENCE)
    raw = bytes(ds.PixelData)
    if len(raw) != CANONICAL_BYTES:
        raise SystemExit(f"{REFERENCE}: {len(raw)} bytes, "
                         f"expected {CANONICAL_BYTES}")
    (OUT / "reference.raw").write_bytes(raw)
    print(f"reference.raw            {len(raw):6d} bytes  sha256 {digest(raw)}")

    for case, (name, suffix, uid) in CASES.items():
        path = CORPUS / name
        if not path.exists():
            raise SystemExit(f"missing {path}. See corpus/README.md.")
        ds = pydicom.dcmread(path)
        check_geometry(ds, name)
        actual_uid = str(ds.file_meta.TransferSyntaxUID)
        if actual_uid != uid:
            raise SystemExit(f"{name}: transfer syntax {actual_uid}, "
                             f"expected {uid}")
        frames = list(generate_pixel_data_frame(ds.PixelData))
        if len(frames) != 1:
            raise SystemExit(f"{name}: {len(frames)} frames, expected 1")
        payload = frames[0]
        target = OUT / f"{case}{suffix}"
        target.write_bytes(payload)
        print(f"{target.name:24s} {len(payload):6d} bytes  "
              f"sha256 {digest(payload)}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
