#!/usr/bin/env python3
"""Encode the JPEG XL fixtures from the synthetic references.

imagecodecs 2026.10.10 with libjxl 0.12.0 is the encoder. The 16-bit ramp is
the existing `jpegls_corpus_reference_u16le.raw` file, SHA-256
`b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609`. The 12-bit
and RGB references are read from the ignored corpus and written here as raw
bytes so the Rust tests do not open that directory.

Orientation 6, an ICC profile and a lossy modular frame are not parameters of
`jpegxl_encode`. Those three fixtures are written with the libjxl 0.12.0 dylib already shipped in the
imagecodecs wheel. No new tool is downloaded.

Run without arguments to check the tracked bytes, or pass `--write` to rebuild
them. Rebuilding needs the ignored corpus and imagecodecs.
"""

from __future__ import annotations

import argparse
import ctypes
import hashlib
import sys
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
CORPUS = REPO / "corpus" / "data" / "syntax"
VENV_PYTHON = REPO / ".venv" / "bin" / "python"
LIBJXL = (
    REPO
    / ".venv"
    / "lib"
    / "python3.12"
    / "site-packages"
    / "imagecodecs"
    / ".dylibs"
    / "libjxl.0.12.0.dylib"
)

ROWS = 64
COLUMNS = 96
SHARED_REFERENCE = "jpegls_corpus_reference_u16le.raw"
SHARED_REFERENCE_SHA256 = (
    "b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609"
)

# Display P3, 536 bytes, the smallest system profile libjxl 0.12.0 accepted
# for a three-channel image on the 2026-11-05 measurement host.
# SHA-256 of /System/Library/ColorSync/Profiles/Display P3.icc on that host.
ICC_SHA256 = "0ff6958f98684c61f6bbdce1368ddeaf3873baf84545baba482e920d92a914c0"
ICC_PATH = Path("/System/Library/ColorSync/Profiles/Display P3.icc")

# These three files are produced by the wheel's libjxl, not by jpegxl_encode.
# Check mode compares the tracked bytes to these digests and does not read
# the system profile or call the dylib.
LIBJXL_FIXTURE_SHA256 = {
    "jpegxl_orient6.jxl": "f625509eeb75c11ad34af6c808c70e870cc4c5d24e7ef5f2d63e86bebd20fe4d",
    "jpegxl_icc.jxl": "f30490729e995c84a3f20ee8468abbcee3ae2a59b54ab263575f890b298d5fe5",
    "jpegxl_modular_xyb.jxl": "aabaf3d7fe2ca9d6aea6c85f06e19c1f58e0310c630ea79a16344c10fa199df1",
}

# Lossy `.112` of the 12-bit ramp. distance 1.0 and effort 7 are libjxl's
# squirrel effort at a moderate distance. The bound in the Rust test is 1.5
# times the max absolute error this script prints for imagecodecs' own decode.
LOSSY_DISTANCE = 1.0
LOSSY_EFFORT = 7

JXL_BOOL = ctypes.c_int


def ensure_tooling() -> None:
    """Re-exec under the repo venv when the system interpreter has no imagecodecs."""
    if "imagecodecs" in sys.modules:
        return
    try:
        import imagecodecs  # noqa: F401
    except ImportError:
        if VENV_PYTHON.is_file() and Path(sys.executable) != VENV_PYTHON:
            import os

            os.execv(str(VENV_PYTHON), [str(VENV_PYTHON), *sys.argv])
        raise SystemExit("imagecodecs is not importable and the repo venv is absent")


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def require_shared_reference() -> bytes:
    raw = (HERE / SHARED_REFERENCE).read_bytes()
    digest = sha256(raw)
    if digest != SHARED_REFERENCE_SHA256:
        raise SystemExit(f"{SHARED_REFERENCE}: digest {digest}")
    return raw


def dicom_pixels(name: str) -> np.ndarray:
    import pydicom

    dataset = pydicom.dcmread(str(CORPUS / name))
    return np.asarray(dataset.pixel_array)


def u16le(array: np.ndarray) -> bytes:
    values = np.ascontiguousarray(array.astype("<u2", copy=False))
    return values.tobytes()


class JxlPreview(ctypes.Structure):
    _fields_ = [("xsize", ctypes.c_uint32), ("ysize", ctypes.c_uint32)]


class JxlAnim(ctypes.Structure):
    _fields_ = [
        ("tps_numerator", ctypes.c_uint32),
        ("tps_denominator", ctypes.c_uint32),
        ("num_loops", ctypes.c_uint32),
        ("have_timecodes", JXL_BOOL),
    ]


class JxlBasicInfo(ctypes.Structure):
    _fields_ = [
        ("have_container", JXL_BOOL),
        ("xsize", ctypes.c_uint32),
        ("ysize", ctypes.c_uint32),
        ("bits_per_sample", ctypes.c_uint32),
        ("exponent_bits_per_sample", ctypes.c_uint32),
        ("intensity_target", ctypes.c_float),
        ("min_nits", ctypes.c_float),
        ("relative_to_max_display", JXL_BOOL),
        ("linear_below", ctypes.c_float),
        ("uses_original_profile", JXL_BOOL),
        ("have_preview", JXL_BOOL),
        ("have_animation", JXL_BOOL),
        ("orientation", ctypes.c_int),
        ("num_color_channels", ctypes.c_uint32),
        ("num_extra_channels", ctypes.c_uint32),
        ("alpha_bits", ctypes.c_uint32),
        ("alpha_exponent_bits", ctypes.c_uint32),
        ("alpha_premultiplied", JXL_BOOL),
        ("preview", JxlPreview),
        ("animation", JxlAnim),
        ("intrinsic_xsize", ctypes.c_uint32),
        ("intrinsic_ysize", ctypes.c_uint32),
        ("padding", ctypes.c_uint8 * 100),
    ]


class JxlPixelFormat(ctypes.Structure):
    _fields_ = [
        ("num_channels", ctypes.c_uint32),
        ("data_type", ctypes.c_int),
        ("endianness", ctypes.c_int),
        ("align", ctypes.c_size_t),
    ]


def load_libjxl():
    if not LIBJXL.is_file():
        raise SystemExit(f"libjxl dylib is absent at {LIBJXL}")
    lib = ctypes.CDLL(str(LIBJXL))
    lib.JxlEncoderCreate.restype = ctypes.c_void_p
    lib.JxlEncoderCreate.argtypes = [ctypes.c_void_p]
    lib.JxlEncoderDestroy.argtypes = [ctypes.c_void_p]
    lib.JxlEncoderInitBasicInfo.argtypes = [ctypes.POINTER(JxlBasicInfo)]
    lib.JxlEncoderSetBasicInfo.restype = ctypes.c_int
    lib.JxlEncoderSetBasicInfo.argtypes = [ctypes.c_void_p, ctypes.POINTER(JxlBasicInfo)]
    lib.JxlEncoderFrameSettingsCreate.restype = ctypes.c_void_p
    lib.JxlEncoderFrameSettingsCreate.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
    lib.JxlEncoderSetFrameLossless.restype = ctypes.c_int
    lib.JxlEncoderSetFrameLossless.argtypes = [ctypes.c_void_p, JXL_BOOL]
    lib.JxlEncoderFrameSettingsSetOption.restype = ctypes.c_int
    lib.JxlEncoderFrameSettingsSetOption.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int64]
    lib.JxlEncoderAddImageFrame.restype = ctypes.c_int
    lib.JxlEncoderAddImageFrame.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(JxlPixelFormat),
        ctypes.c_void_p,
        ctypes.c_size_t,
    ]
    lib.JxlEncoderCloseInput.argtypes = [ctypes.c_void_p]
    lib.JxlEncoderProcessOutput.restype = ctypes.c_int
    lib.JxlEncoderProcessOutput.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.POINTER(ctypes.c_uint8)),
        ctypes.POINTER(ctypes.c_size_t),
    ]
    lib.JxlEncoderSetICCProfile.restype = ctypes.c_int
    lib.JxlEncoderSetICCProfile.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t]
    lib.JxlEncoderGetError.restype = ctypes.c_int
    lib.JxlEncoderGetError.argtypes = [ctypes.c_void_p]
    if lib.JxlEncoderVersion() != 12000:
        raise SystemExit(f"libjxl version {lib.JxlEncoderVersion()}, wanted 12000")
    return lib


def encode_with_libjxl(
    pixels: np.ndarray, *, orientation: int, icc: bytes | None, modular: bool = False
) -> bytes:
    """One lossless or lossy frame through the wheel's libjxl, for options imagecodecs hides."""
    lib = load_libjxl()
    channels = 1 if pixels.ndim == 2 else int(pixels.shape[2])
    enc = lib.JxlEncoderCreate(None)
    info = JxlBasicInfo()
    lib.JxlEncoderInitBasicInfo(ctypes.byref(info))
    info.xsize = int(pixels.shape[1])
    info.ysize = int(pixels.shape[0])
    info.bits_per_sample = 8
    info.num_color_channels = channels
    info.orientation = orientation
    info.uses_original_profile = 1 if icc is not None else 0
    if lib.JxlEncoderSetBasicInfo(enc, ctypes.byref(info)) != 0:
        raise SystemExit(f"JxlEncoderSetBasicInfo failed ({lib.JxlEncoderGetError(enc)})")
    if icc is not None:
        buffer = (ctypes.c_uint8 * len(icc)).from_buffer_copy(icc)
        if lib.JxlEncoderSetICCProfile(enc, buffer, len(icc)) != 0:
            raise SystemExit(f"JxlEncoderSetICCProfile failed ({lib.JxlEncoderGetError(enc)})")
    settings = lib.JxlEncoderFrameSettingsCreate(enc, None)
    # Orientation has no effect on the refusal, so a lossy frame is enough.
    # ICC needs the original profile, which libjxl pairs with a lossless frame.
    lossless = 1 if icc is not None else 0
    if lib.JxlEncoderSetFrameLossless(settings, lossless) != 0:
        raise SystemExit(f"JxlEncoderSetFrameLossless failed ({lib.JxlEncoderGetError(enc)})")
    if modular:
        # JXL_ENC_FRAME_SETTING_MODULAR is 11 in libjxl encode.h. A lossy
        # modular frame keeps `xyb_encoded`, so `.110` can only refuse it
        # through the image header, not through the VarDCT frame encoding.
        if lib.JxlEncoderFrameSettingsSetOption(settings, 11, 1) != 0:
            raise SystemExit(f"modular frame setting failed ({lib.JxlEncoderGetError(enc)})")
    # JXL_TYPE_UINT8 is 2. See libjxl types.h.
    pixel_format = JxlPixelFormat(channels, 2, 0, 0)
    raw = np.ascontiguousarray(pixels).tobytes()
    pixel_buffer = (ctypes.c_uint8 * len(raw)).from_buffer_copy(raw)
    if lib.JxlEncoderAddImageFrame(settings, ctypes.byref(pixel_format), pixel_buffer, len(raw)) != 0:
        raise SystemExit(f"JxlEncoderAddImageFrame failed ({lib.JxlEncoderGetError(enc)})")
    lib.JxlEncoderCloseInput(enc)
    out = bytearray()
    chunk = ctypes.create_string_buffer(1 << 16)
    while True:
        next_out = ctypes.cast(chunk, ctypes.POINTER(ctypes.c_uint8))
        avail = ctypes.c_size_t(len(chunk))
        status = lib.JxlEncoderProcessOutput(enc, ctypes.byref(next_out), ctypes.byref(avail))
        written = len(chunk) - avail.value
        out += chunk.raw[:written]
        if status == 0:
            break
        if status != 2:
            raise SystemExit(f"JxlEncoderProcessOutput failed ({lib.JxlEncoderGetError(enc)})")
    lib.JxlEncoderDestroy(enc)
    return bytes(out)


def encode_imagecodecs(array: np.ndarray, **kwargs) -> bytes:
    import imagecodecs

    kwargs.setdefault("effort", LOSSY_EFFORT)
    kwargs.setdefault("usecontainer", False)
    encoded = imagecodecs.jpegxl_encode(np.ascontiguousarray(array), **kwargs)
    if not isinstance(encoded, (bytes, bytearray)):
        raise SystemExit("jpegxl_encode did not return bytes")
    return bytes(encoded)


def build() -> tuple[dict[str, bytes], int]:
    import imagecodecs

    ramp16 = np.frombuffer(require_shared_reference(), dtype="<u2").reshape(ROWS, COLUMNS)
    mono12 = dicom_pixels("reference_mono12.dcm")
    rgb8 = dicom_pixels("reference_rgb8.dcm")
    if mono12.shape != (ROWS, COLUMNS) or int(mono12.max()) != 4095:
        raise SystemExit("reference_mono12.dcm is not the 64 by 96 12-bit ramp")
    if rgb8.shape != (ROWS, COLUMNS, 3):
        raise SystemExit("reference_rgb8.dcm is not 64 by 96 RGB")
    if list(mono12.reshape(-1)[:6]) != [0, 0, 1, 1, 2, 3]:
        raise SystemExit("reference_mono12.dcm first samples are not 0, 0, 1, 1, 2, 3")

    jpeg = (HERE / "jpeg_baseline_rgb.jpg").read_bytes()
    recompressed = imagecodecs.jpegxl_encode_jpeg(jpeg)
    restored = imagecodecs.jpegxl_decode_jpeg(recompressed)
    if restored != jpeg:
        raise SystemExit("jpegxl_encode_jpeg did not round-trip jpeg_baseline_rgb.jpg")

    lossless16 = encode_imagecodecs(ramp16, lossless=True, bitspersample=16)
    decoded16 = imagecodecs.jpegxl_decode(lossless16)
    if not np.array_equal(decoded16, ramp16):
        raise SystemExit("16-bit lossless round trip is not exact")

    lossless12 = encode_imagecodecs(mono12.astype("<u2"), lossless=True, bitspersample=12)
    decoded12 = imagecodecs.jpegxl_decode(lossless12)
    if not np.array_equal(decoded12, mono12):
        raise SystemExit("12-bit lossless round trip is not exact")

    lossless_rgb = encode_imagecodecs(rgb8, lossless=True, bitspersample=8, photometric="RGB")
    decoded_rgb = imagecodecs.jpegxl_decode(lossless_rgb)
    if not np.array_equal(decoded_rgb, rgb8):
        raise SystemExit("RGB lossless round trip is not exact")

    lossy12 = encode_imagecodecs(
        mono12.astype("<u2"),
        lossless=False,
        bitspersample=12,
        distance=LOSSY_DISTANCE,
        effort=LOSSY_EFFORT,
    )
    decoded_lossy = imagecodecs.jpegxl_decode(lossy12)
    if decoded_lossy.shape != mono12.shape:
        raise SystemExit(f"lossy decode shape {decoded_lossy.shape}")
    error = int(np.max(np.abs(decoded_lossy.astype(np.int32) - mono12.astype(np.int32))))

    small = np.arange(8 * 16, dtype=np.uint8).reshape(8, 16)
    two = np.stack(
        [
            np.arange(8 * 16, dtype=np.uint8).reshape(8, 16),
            np.arange(128, 256, dtype=np.uint8).reshape(8, 16),
        ]
    )
    two_frames = encode_imagecodecs(two, lossless=True, bitspersample=8, photometric="GRAY")

    alpha = np.arange(8 * 8 * 4, dtype=np.uint8).reshape(8, 8, 4)
    alpha_frame = encode_imagecodecs(alpha, lossless=True, bitspersample=8)

    pq = encode_imagecodecs(small, lossless=False, bitspersample=8, transfer="PQ", photometric="GRAY")
    floats = (np.arange(16, dtype=np.float32).reshape(4, 4) / np.float32(16.0))
    float_frame = encode_imagecodecs(floats, lossless=False, bitspersample=32, photometric="GRAY")

    container = encode_imagecodecs(small, lossless=True, bitspersample=8, photometric="GRAY", usecontainer=True)

    artefacts = {
        "jpegxl_lossless_16.jxl": lossless16,
        "jpegxl_lossless_12.jxl": lossless12,
        "jpegxl_lossless_rgb8.jxl": lossless_rgb,
        "jpegxl_recompression.jxl": bytes(recompressed),
        "jpegxl_lossy_12.jxl": lossy12,
        "jpegxl_two_frames.jxl": two_frames,
        "jpegxl_alpha.jxl": alpha_frame,
        "jpegxl_pq.jxl": pq,
        "jpegxl_float.jxl": float_frame,
        "jpegxl_container.jxl": container,
        "jpegxl_reference_mono12_u16le.raw": u16le(mono12),
        "jpegxl_reference_rgb8.raw": np.ascontiguousarray(rgb8).tobytes(),
    }
    return artefacts, error


def libjxl_fixtures() -> dict[str, bytes]:
    """Encode orientation 6, the ICC frame and lossy modular XYB. Rebuilds need the system profile."""
    icc = ICC_PATH.read_bytes()
    if sha256(icc) != ICC_SHA256:
        raise SystemExit("Display P3 profile digest changed, so the ICC fixture would drift")
    small = np.arange(8 * 16, dtype=np.uint8).reshape(8, 16)
    orient = encode_with_libjxl(small, orientation=6, icc=None)
    icc_frame = encode_with_libjxl(
        np.arange(8 * 8 * 3, dtype=np.uint8).reshape(8, 8, 3),
        orientation=1,
        icc=icc,
    )
    modular_xyb = encode_with_libjxl(
        np.arange(8 * 8 * 3, dtype=np.uint8).reshape(8, 8, 3),
        orientation=1,
        icc=None,
        modular=True,
    )
    return {
        "jpegxl_orient6.jxl": orient,
        "jpegxl_icc.jxl": icc_frame,
        "jpegxl_modular_xyb.jxl": modular_xyb,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    ensure_tooling()
    artefacts, error = build()
    print(f"imagecodecs lossy max abs error {error}")
    print(f"lossy bound at 1.5 times that error is {error + error // 2 + (1 if error % 2 else 0)}")
    if args.write:
        artefacts.update(libjxl_fixtures())
    mismatched = []
    for name, data in artefacts.items():
        path = HERE / name
        if args.write:
            path.write_bytes(data)
            print(f"wrote {name} {len(data)} {sha256(data)}")
        elif not path.is_file() or path.read_bytes() != data:
            mismatched.append(name)
    for name, digest in LIBJXL_FIXTURE_SHA256.items():
        path = HERE / name
        if not path.is_file() or sha256(path.read_bytes()) != digest:
            mismatched.append(name)
        elif args.write:
            fresh = artefacts.get(name)
            if fresh is None or sha256(fresh) != digest:
                mismatched.append(name)
    if mismatched:
        raise SystemExit("fixture drift: " + ", ".join(mismatched))
    if not args.write:
        print("jpeg xl fixtures match")


if __name__ == "__main__":
    main()
