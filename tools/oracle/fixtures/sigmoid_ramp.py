"""PS3.3 C.11.2 synthetic SIGMOID shoulders and LINEAR output-code controls."""

import argparse
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "scripts"))
from corpus_synth import CT_STORAGE, new_dataset, write


def generate(out: Path, linear: bool = False) -> None:
    name = "diagnostic_linear_ramp" if linear else "diagnostic_sigmoid_ramp"
    ds = new_dataset(name, CT_STORAGE, "CT")
    ds.Rows, ds.Columns = 4, 20
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = "MONOCHROME2"
    ds.BitsAllocated, ds.BitsStored, ds.HighBit, ds.PixelRepresentation = 16, 16, 15, 0
    ds.PixelSpacing = [1, 1]
    ds.ImageOrientationPatient = [1, 0, 0, 0, 1, 0]
    ds.ImagePositionPatient = [0, 0, 0]
    ds.RescaleSlope, ds.RescaleIntercept = "0.25", "0"
    ds.WindowCenter, ds.WindowWidth, ds.VOILUTFunction = "40", "0.5", "SIGMOID"
    if linear:
        ds.PixelRepresentation = 1
        ds.RescaleSlope = "1"
        ds.WindowWidth, ds.VOILUTFunction = "400", "LINEAR"
        samples = np.array([-200, -160, -159, -100, -60, -1, 0, 39, 40, 41,
                            100, 150, 200, 238, 239, 240, 300, 400, 500, 1000], dtype="<i2")
    else:
        samples = np.arange(151, 171, dtype="<u2")
    ds.PixelData = np.tile(samples, (4, 1)).tobytes()
    write(ds, out / "synthetic" / f"{name}.dcm")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", type=Path)
    parser.add_argument("--linear", action="store_true")
    args = parser.parse_args()
    generate(args.out, args.linear)
