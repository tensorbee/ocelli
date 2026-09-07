#!/usr/bin/env python3
"""Focused executable boundaries for permanent quirk mutations. F-014."""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path

import pydicom

sys.path.insert(0, str(Path(__file__).resolve().parent))
import corpus_synth  # noqa: E402


def sigmoid_generator_boundary() -> None:
    """Generate only the SIGMOID case and check its defining attributes."""
    with tempfile.TemporaryDirectory(prefix="ocelli-quirk-") as temporary:
        out = Path(temporary)
        corpus_synth.case_sigmoid_width_half(out)
        path = out / "synthetic" / "ct_sigmoid_width_half.dcm"
        dataset = pydicom.dcmread(path)
        function = str(dataset.VOILUTFunction)
        if function != "SIGMOID":
            raise AssertionError(f"expected SIGMOID, found {function}")
        width = float(dataset.WindowWidth)
        if width != 0.5:
            raise AssertionError(f"expected width 0.5, found {width}")


def main() -> int:
    sigmoid_generator_boundary()
    print("OK: SIGMOID generator mutation boundary is green")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
