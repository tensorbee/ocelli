#!/usr/bin/env python3
"""Build the synthetic video clips the F-241 demux tests read.

Every clip is synthetic. The pictures are ffmpeg's lavfi ``testsrc2`` pattern
and the audio is its ``sine`` source. No patient data, no DICOM and no
real-world recording is involved anywhere in this directory.

ffmpeg is run as a host tool and never read, per docs/SOURCE-POLICY.md. The
exact argument list of every clip is the ``CLIPS`` table below, so each clip is
reproducible from this file. Two clips are built here rather than by ffmpeg.
``mpeg2_gop4.pes`` wraps the MPEG-2 elementary stream in PES packets written
from Rec. ITU-T H.222.0 (08/2023) Table 2-21, and ``garbage.bin`` is fixed
bytes that no container starts with.

The encodes are pinned to one thread with bitexact flags, so the same ffmpeg
build writes the same bytes. ``SHA256SUMS`` records the digest of every clip
and the ffmpeg version line they were made with. A different ffmpeg build may
encode different bytes, and the check says which version it ran against.

Transport streams are named ``.m2t`` rather than ``.ts``, because the
repository's source provenance check reads every ``.ts`` file as TypeScript.

Run without arguments to rebuild every clip in a temporary directory and
compare the digests with ``SHA256SUMS``. Pass ``--write`` to rebuild the clips
in place and rewrite ``SHA256SUMS``.
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMS = HERE / "SHA256SUMS"

BASE = ["-hide_banner", "-loglevel", "error", "-y"]
# Output options pinning one thread and bitexact muxing, so the bytes do not
# depend on the host.
OUTPUT = ["-threads", "1", "-fflags", "+bitexact", "-flags:v", "+bitexact", "-map_metadata", "-1"]
PICTURES = ["-f", "lavfi", "-i", "testsrc2=size=64x64:rate=25"]
# libx264 with a fixed group of 12, two B-frames and no scene cut, so the
# random access points are exactly every twelfth frame.
X264 = [
    "-c:v",
    "libx264",
    "-profile:v",
    "high",
    "-pix_fmt",
    "yuv420p",
    "-x264-params",
    "keyint=12:min-keyint=12:scenecut=0:bframes=2:threads=1",
]
# The same encoder settings with interlaced coding, top field first.
X264_INTERLACED = [*X264[:-1], X264[-1] + ":interlaced=1:tff=1"]
X265 = [
    "-c:v",
    "libx265",
    "-tag:v",
    "hvc1",
    "-x265-params",
    "keyint=8:min-keyint=8:scenecut=0:bframes=2:open-gop=0:"
    "pools=none:frame-threads=1:log-level=error",
]
MPEG2 = ["-c:v", "mpeg2video", "-q:v", "6", "-g", "4", "-bf", "2", "-pix_fmt", "yuv420p"]


def pictures(width: int, height: int) -> list[str]:
    return ["-f", "lavfi", "-i", f"testsrc2=size={width}x{height}:rate=25"]


def x264(params: str) -> list[str]:
    return [*X264[:-1], params + ":threads=1"]


def x265(params: str) -> list[str]:
    return [*X265[:-1], params + ":pools=none:frame-threads=1:log-level=error"]


def tagged(primaries: str, transfer: str, matrix: str, colour_range: str) -> list[str]:
    """Colour properties on the encoded frames, which the encoder writes into
    the stream. ffmpeg 9.0.2 passes only the matrix of the colour output
    options to an encoder, so the frames carry all three code points instead.
    """
    return [
        "-vf",
        f"setparams=color_primaries={primaries}:color_trc={transfer}:"
        f"colorspace={matrix}:range={colour_range}",
    ]


def colr(primaries: str, transfer: str, matrix: str, colour_range: str) -> list[str]:
    """Colour tags on a stream copy, which the MP4 muxer writes as a colr box
    of type nclx and which leave the stream's own headers unchanged."""
    return [
        "-color_primaries",
        primaries,
        "-color_trc",
        transfer,
        "-colorspace",
        matrix,
        "-color_range",
        colour_range,
        "-movflags",
        "+write_colr",
    ]


# Three different Rec. ITU-T H.273 code points in each description, so a
# reader that swaps two fields reads a different value. bt470bg is 5, bt709
# is 1 and smpte170m is 6 in colour_primaries, transfer_characteristics and
# matrix_coefficients alike.
VUI_5_1_6 = tagged("bt470bg", "bt709", "smpte170m", "tv")
VUI_1_6_5_FULL = tagged("bt709", "smpte170m", "bt470bg", "pc")
COLR_1_6_5 = colr("bt709", "smpte170m", "bt470bg", "tv")
COLR_1_6_5_FULL = colr("bt709", "smpte170m", "bt470bg", "pc")
COPY = ["-c", "copy"]
# The MP4 to Annex B conversion, named rather than left for ffmpeg to insert.
H264_ANNEX_B = ["-bsf:v", "h264_mp4toannexb"]
HEVC_ANNEX_B = ["-bsf:v", "hevc_mp4toannexb"]

# Rec. ITU-T H.222.0 (08/2023) 2.4.3.7: a PTS is a 33-bit count of a 90 kHz
# clock, so it wraps at 2**33 = 8589934592 ticks. ffmpeg 9.0.2 starts an
# mpegts output 1.4 seconds, 126000 ticks, after the offset, so an offset of
# 95441.8 seconds puts the first PTS at 8589888000, which is 46592 ticks, about
# 13 frames at 25 per second, before the wrap. The test proves the wrap is in
# the clip.
WRAP_OFFSET_SECONDS = "95441.8"

# (output name, ffmpeg arguments between BASE and OUTPUT, or a builder name).
# An input named ``@name`` is an earlier clip in this table.
CLIPS: list[tuple[str, list[str] | str]] = [
    # AC3 and AC4: 60 frames, a group of 12, two B-frames.
    ("h264_gop12.mp4", [*PICTURES, "-frames:v", "60", *X264]),
    ("h264_gop12.m2t", ["-i", "@h264_gop12.mp4", *COPY, "-f", "mpegts"]),
    ("h264_gop12.h264", ["-i", "@h264_gop12.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    (
        "h264_gop12_wrap.m2t",
        ["-i", "@h264_gop12.mp4", *COPY, "-output_ts_offset", WRAP_OFFSET_SECONDS, "-f", "mpegts"],
    ),
    ("h264_gop12_trailing_zero.m2t", "trailing_zero_ts"),
    ("h264_gop12_two_per_pes.m2t", "two_per_pes_ts"),
    # Three slices a picture and one IDR in 100 frames. x264 writes
    # log2_max_pic_order_cnt_lsb_minus4 1, a 5-bit count of two a frame, so
    # the count wraps six times, and a picture spans NAL units. No
    # B-frame pyramid, so two non-reference pictures follow each other and
    # only pic_order_cnt_lsb tells them apart. A fade to red gives P slices
    # chroma weights in their pred_weight_table.
    (
        "h264_slices.mp4",
        [
            *pictures(48, 48),
            "-frames:v",
            "100",
            "-vf",
            "fade=in:0:50:color=red",
            *x264("keyint=infinite:scenecut=0:bframes=2:b-pyramid=none:slices=3:weightp=2"),
        ],
    ),
    ("h264_slices.h264", ["-i", "@h264_slices.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    ("h264_slices.m2t", ["-i", "@h264_slices.mp4", *COPY, "-f", "mpegts"]),
    # No B-frames, which x264 codes with pic_order_cnt_type 2, and a 4-bit
    # frame_num that wraps in 40 frames.
    ("h264_poc2.mp4", [*pictures(32, 32), "-frames:v", "40", *x264("keyint=infinite:scenecut=0:bframes=0")]),
    ("h264_poc2.h264", ["-i", "@h264_poc2.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    (
        "h264_audio.mp4",
        [
            *PICTURES,
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=16000",
            "-frames:v",
            "24",
            *X264,
            "-c:a",
            "aac",
            "-b:a",
            "16k",
            "-t",
            "0.96",
        ],
    ),
    ("h264_audio.m2t", ["-i", "@h264_audio.mp4", *COPY, "-f", "mpegts"]),
    # Colour descriptions, F-241 plan item 6.
    ("h264_vui.mp4", [*PICTURES, "-frames:v", "4", *VUI_5_1_6, *X264]),
    ("h264_vui_full.mp4", [*PICTURES, "-frames:v", "4", *VUI_1_6_5_FULL, *X264]),
    ("h264_plain.mp4", [*PICTURES, "-frames:v", "4", *X264]),
    ("h264_colr.mp4", ["-i", "@h264_plain.mp4", *COPY, *COLR_1_6_5]),
    ("h264_colr_full.mp4", ["-i", "@h264_plain.mp4", *COPY, *COLR_1_6_5_FULL]),
    ("h264_vui_and_colr.mp4", ["-i", "@h264_vui.mp4", *COPY, *COLR_1_6_5]),
    ("h264_vui.h264", ["-i", "@h264_vui.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    # 56 is not a whole number of 16-sample macroblocks either way.
    ("h264_crop.mp4", [*pictures(56, 56), "-frames:v", "4", *X264]),
    # High 10 at QP 1, which x264 writes with a pic_init_qp_minus26 below -26,
    # inside the -( 26 + QpBdOffsetY ) bound of Rec. ITU-T H.264 (08/2024)
    # 7.4.2.2 only because the SPS is 10-bit.
    (
        "h264_high10_qp1.mp4",
        [
            *PICTURES,
            "-frames:v",
            "4",
            "-c:v",
            "libx264",
            "-profile:v",
            "high10",
            "-pix_fmt",
            "yuv420p10le",
            "-x264-params",
            "keyint=12:min-keyint=12:scenecut=0:bframes=2:qp=1:threads=1",
        ],
    ),
    ("h264_high10_qp1.h264", ["-i", "@h264_high10_qp1.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    # A B-frame pyramid, review pass 4 S1. Three B-frames between anchors
    # with b-pyramid=normal make the middle B-frame a reference picture,
    # nal_ref_idc above 0, whose slices carry dec_ref_pic_marking after both
    # reference picture list modifications, and x264 writes
    # memory_management_control_operation 1 there to drop it again.
    (
        "h264_bpyramid.mp4",
        [*PICTURES, "-frames:v", "48", *x264("keyint=24:min-keyint=24:scenecut=0:bframes=3:b-pyramid=normal:ref=4")],
    ),
    ("h264_bpyramid.h264", ["-i", "@h264_bpyramid.mp4", *COPY, *H264_ANNEX_B, "-f", "h264"]),
    ("h264_bpyramid.m2t", ["-i", "@h264_bpyramid.mp4", *COPY, "-f", "mpegts"]),
    # The same Annex B stream as h264_gop12.h264 with its first SPS and PPS
    # swapped, a PPS before the SPS it names, which 7.4.1.2.1 allows.
    ("h264_gop12_pps_first.h264", "pps_before_sps"),
    ("h264_interlaced.mp4", [*PICTURES, "-frames:v", "4", *X264_INTERLACED]),
    (
        "h264_fragmented.mp4",
        [*PICTURES, "-frames:v", "12", *X264, "-movflags", "frag_keyframe+empty_moov"],
    ),
    ("hevc_main.mp4", [*PICTURES, "-frames:v", "16", *VUI_5_1_6, "-pix_fmt", "yuv420p", *X265]),
    ("hevc_main.m2t", ["-i", "@hevc_main.mp4", *COPY, "-f", "mpegts"]),
    ("hevc_main.h265", ["-i", "@hevc_main.mp4", *COPY, *HEVC_ANNEX_B, "-f", "hevc"]),
    ("hevc_main_trailing_zero.m2t", "hevc_trailing_zero_ts"),
    ("hevc_main10.mp4", [*PICTURES, "-frames:v", "4", "-pix_fmt", "yuv420p10le", *X265]),
    # x265 writes a VUI with video_signal_type_present_flag 1 and no colour
    # description here. The copy adds a colr box saying full range, which the
    # stream's own range of 0 must outrank.
    ("hevc_main10_colr_full.mp4", ["-i", "@hevc_main10.mp4", *COPY, *COLR_1_6_5_FULL]),
    ("hevc_main10.h265", ["-i", "@hevc_main10.mp4", *COPY, *HEVC_ANNEX_B, "-f", "hevc"]),
    # One IRAP picture in 80 frames. log2-max-poc-lsb=4 makes x265 4.3 write
    # log2_max_pic_order_cnt_lsb_minus4 2, a 6-bit count of one a frame, so
    # the count wraps once, at frame 64. x265 4.3 with slices=2 writes slice
    # segment headers ffmpeg 9.0.2's decoder reports as overread, so no
    # multi-slice HEVC clip is made here.
    (
        "hevc_poc.mp4",
        [
            *pictures(32, 32),
            "-frames:v",
            "80",
            "-pix_fmt",
            "yuv420p",
            *x265("keyint=1000:min-keyint=1000:scenecut=0:bframes=2:open-gop=0:ctu=16:log2-max-poc-lsb=4"),
        ],
    ),
    ("hevc_poc.h265", ["-i", "@hevc_poc.mp4", *COPY, *HEVC_ANNEX_B, "-f", "hevc"]),
    # 60 is not a whole number of 8-sample minimum coding blocks either way.
    ("hevc_crop.mp4", [*pictures(60, 60), "-frames:v", "4", "-pix_fmt", "yuv420p", *X265]),
    # AC5.
    ("mpeg2_gop4.mp4", [*PICTURES, "-frames:v", "12", *VUI_5_1_6, *MPEG2]),
    ("mpeg2_gop4.m2t", ["-i", "@mpeg2_gop4.mp4", *COPY, "-f", "mpegts"]),
    ("mpeg2_gop4.m2v", ["-i", "@mpeg2_gop4.mp4", *COPY, "-f", "mpeg2video"]),
    ("mpeg2_gop4.mpg", ["-i", "@mpeg2_gop4.m2v", *COPY, "-f", "vob"]),
    ("mpeg2_gop4.pes", "bare_pes"),
    ("mpeg2_tr_wrap.m2v", "temporal_reference_wrap"),
    ("mpeg2_crop.m2v", [*pictures(56, 56), "-frames:v", "4", *MPEG2, "-f", "mpeg2video"]),
    ("garbage.bin", "garbage"),
    ("h264_delta_scale_high.h264", "delta_scale_high"),
    ("h264_delta_scale_low.h264", "delta_scale_low"),
]

# Rec. ITU-T H.222.0 (08/2023) Table 2-22, the first video stream_id.
VIDEO_STREAM_ID = 0xE0
# PES_packet_length is 16 bits and counts the three header bytes after it.
PES_PAYLOAD_LIMIT = 0xFFFF - 3


def bare_pes(directory: Path) -> bytes:
    """Wrap ``mpeg2_gop4.m2v`` in PES packets with no system layer around them.

    Each packet is Rec. ITU-T H.222.0 (08/2023) Table 2-21: the start code
    prefix 0x000001, the stream_id, a 16-bit PES_packet_length, then the
    '10' marker byte with every flag clear, a zero flags byte and a zero
    PES_header_data_length, so no PTS or DTS is present.
    """
    stream = (directory / "mpeg2_gop4.m2v").read_bytes()
    out = bytearray()
    for start in range(0, len(stream), PES_PAYLOAD_LIMIT):
        chunk = stream[start : start + PES_PAYLOAD_LIMIT]
        out += b"\x00\x00\x01" + bytes([VIDEO_STREAM_ID])
        out += (len(chunk) + 3).to_bytes(2, "big")
        out += b"\x80\x00\x00"
        out += chunk
    return bytes(out)


def garbage(_directory: Path) -> bytes:
    """Sixty-four bytes that open no container the demuxer knows."""
    return bytes(range(0x40, 0x80))


# A transport stream written from Rec. ITU-T H.222.0 (08/2023): Table 2-2
# packets, a Table 2-30 PAT, a Table 2-33 PMT with one stream_type 0x1B
# stream, Table 2-21 PES packets, and the Annex A CRC_32.
PMT_PID = 0x1000
VIDEO_PID = 0x0100
PACKET = 188


def crc32_mpeg(data: bytes) -> int:
    """Annex A: polynomial 0x04C11DB7, registers starting at 1, most
    significant bit first, no final inversion."""
    register = 0xFFFFFFFF
    for byte in data:
        register ^= byte << 24
        for _ in range(8):
            register = ((register << 1) ^ 0x04C11DB7) if register & 0x80000000 else register << 1
            register &= 0xFFFFFFFF
    return register


def section(table_id: int, extension: int, body: bytes) -> bytes:
    # section_syntax_indicator 1, '0', reserved '11', then section_length,
    # which counts from the next byte through the CRC_32.
    length = 5 + len(body) + 4
    head = bytes([table_id, 0xB0 | (length >> 8), length & 0xFF])
    head += extension.to_bytes(2, "big") + bytes([0xC1, 0x00, 0x00])
    content = head + body
    return content + crc32_mpeg(content).to_bytes(4, "big")


def packets(pid: int, payload: bytes, counter: list[int]) -> bytes:
    """Splits one payload unit across packets, padding the last packet with
    an adaptation field of stuffing bytes, Table 2-6."""
    out = bytearray()
    first = True
    position = 0
    while position < len(payload) or first:
        chunk = payload[position : position + PACKET - 4]
        header = bytes([0x47, (0x40 if first else 0x00) | (pid >> 8), pid & 0xFF])
        if len(chunk) == PACKET - 4:
            out += header + bytes([0x10 | counter[0]]) + chunk
        else:
            stuffing = PACKET - 4 - len(chunk)
            adaptation = bytes([stuffing - 1]) + (bytes([0x00]) + b"\xff" * (stuffing - 2) if stuffing > 1 else b"")
            out += header + bytes([0x30 | counter[0]]) + adaptation + chunk
        counter[0] = (counter[0] + 1) % 16
        position += len(chunk)
        first = False
    return bytes(out)


def timestamp(prefix: int, value: int) -> bytes:
    """A 33-bit PTS or DTS behind a 4-bit prefix, with its three marker bits."""
    return bytes(
        [
            (prefix << 4) | ((value >> 29) & 0x0E) | 1,
            (value >> 22) & 0xFF,
            ((value >> 14) & 0xFE) | 1,
            (value >> 7) & 0xFF,
            ((value << 1) & 0xFE) | 1,
        ]
    )


def pes_packet(payload: bytes, pts: int | None, dts: int | None) -> bytes:
    fields = b""
    flags = 0x00
    if pts is not None and dts is not None:
        flags, fields = 0xC0, timestamp(0b0011, pts) + timestamp(0b0001, dts)
    elif pts is not None:
        flags, fields = 0x80, timestamp(0b0010, pts)
    header = bytes([0x80, flags, len(fields)]) + fields
    # PES_packet_length 0, unbounded, which 2.4.3.7 allows for video in a
    # transport stream.
    return b"\x00\x00\x01" + bytes([VIDEO_STREAM_ID]) + b"\x00\x00" + header + payload


def is_h264_slice(header: int) -> bool:
    """nal_unit_type 1 or 5, Rec. ITU-T H.264 (08/2024) Table 7-1."""
    return header & 0x1F in (1, 5)


def is_hevc_slice(header: int) -> bool:
    """nal_unit_type 0 to 31, Rec. ITU-T H.265 (07/2024) Table 7-1."""
    return (header >> 1) & 0x3F <= 31


def access_units(stream: bytes, is_slice=is_h264_slice) -> list[bytes]:
    """Splits a byte stream whose pictures are one slice each into access
    units: the run of NAL units up to and including each coded slice, with
    each unit's start code kept."""
    starts = []
    position = 0
    while (found := stream.find(b"\x00\x00\x01", position)) != -1:
        begin = found - 1 if found > 0 and stream[found - 1] == 0 else found
        starts.append((begin, found + 3))
        position = found + 3
    units = []
    current = bytearray()
    for index, (begin, payload) in enumerate(starts):
        end = starts[index + 1][0] if index + 1 < len(starts) else len(stream)
        current += stream[begin:end]
        if is_slice(stream[payload]):
            units.append(bytes(current))
            current = bytearray()
    return units


def reference_timestamps(directory: Path, name: str = "h264_gop12.m2t") -> list[tuple[int, int]]:
    """The PTS and DTS of every video PES packet of a transport stream clip
    in order, read with ffprobe as a host tool."""
    completed = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "packet=pts,dts",
            "-of",
            "csv=p=0",
            str(directory / name),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return [tuple(int(value) for value in line.split(",")[:2]) for line in completed.stdout.split()]


def transport_stream(units: list[tuple[bytes, int | None, int | None]], stream_type: int = 0x1B) -> bytes:
    pat = section(0x00, 1, (1).to_bytes(2, "big") + (0xE000 | PMT_PID).to_bytes(2, "big"))
    stream = bytes([stream_type]) + (0xE000 | VIDEO_PID).to_bytes(2, "big") + b"\xf0\x00"
    pmt = section(0x02, 1, (0xE000 | VIDEO_PID).to_bytes(2, "big") + b"\xf0\x00" + stream)
    out = packets(0x0000, b"\x00" + pat, [0]) + packets(PMT_PID, b"\x00" + pmt, [0])
    counter = [0]
    for payload, pts, dts in units:
        out += packets(VIDEO_PID, pes_packet(payload, pts, dts), counter)
    return out


def trailing_zero_ts(directory: Path) -> bytes:
    """One access unit a PES packet with its own PTS and DTS, each payload
    ending in one trailing_zero_8bits byte, Rec. ITU-T H.264 (08/2024) B.1.1.
    The zero belongs to the byte stream between two access units, so the
    next access unit still commences in the next PES packet."""
    units = access_units((directory / "h264_gop12.h264").read_bytes())
    times = reference_timestamps(directory)
    assert len(units) == len(times) == 60
    return transport_stream([(unit + b"\x00", pts, dts) for unit, (pts, dts) in zip(units, times)])


def hevc_trailing_zero_ts(directory: Path) -> bytes:
    """The HEVC form of ``trailing_zero_ts``, from ``hevc_main.h265`` and the
    times of ``hevc_main.m2t``, with stream_type 0x24, Table 2-34."""
    units = access_units((directory / "hevc_main.h265").read_bytes(), is_hevc_slice)
    times = reference_timestamps(directory, "hevc_main.m2t")
    assert len(units) == len(times) == 16
    return transport_stream(
        [(unit + b"\x00", pts, dts) for unit, (pts, dts) in zip(units, times)], 0x24
    )


def two_per_pes_ts(directory: Path) -> bytes:
    """Two access units a PES packet with one PTS, which 2.4.3.7 gives to the
    first, so the second access unit, decode index 1, has none."""
    units = access_units((directory / "h264_gop12.h264").read_bytes())
    times = reference_timestamps(directory)
    pairs = []
    for index in range(0, len(units), 2):
        pts, dts = times[index]
        pairs.append((units[index] + units[index + 1], pts, dts))
    return transport_stream(pairs)


def temporal_reference_wrap(directory: Path) -> bytes:
    """``mpeg2_gop4.m2v`` with its group of pictures headers removed and every
    temporal_reference renumbered from 1020 in display order, so the count
    wraps from 1023 to 0 after four frames.

    Rec. ITU-T H.262 (02/2000) 6.2.2 makes group_of_pictures_header optional,
    and 6.3.9 says temporal_reference counts up by one per frame in display
    order modulo 1024, resetting only at a group header. A picture's display
    position is its group's first position plus its temporal_reference in that
    group. The field is the first 10 bits after the picture start code, 6.2.3.
    """
    stream = (directory / "mpeg2_gop4.m2v").read_bytes()
    out = bytearray()
    group_base = -1
    group_count = 0
    position = 0
    starts = []
    while (found := stream.find(b"\x00\x00\x01", position)) != -1:
        starts.append(found)
        position = found + 3
    for index, start in enumerate(starts):
        end = starts[index + 1] if index + 1 < len(starts) else len(stream)
        unit = bytearray(stream[start:end])
        code = unit[3]
        if code == 0xB8:
            group_base = group_base + group_count if group_base >= 0 else 0
            group_count = 0
            continue
        if code == 0x00:
            reference = (unit[4] << 2) | (unit[5] >> 6)
            group_count = max(group_count, reference + 1)
            renumbered = (group_base + reference + 1020) % 1024
            unit[4] = renumbered >> 2
            unit[5] = ((renumbered & 0x03) << 6) | (unit[5] & 0x3F)
        out += unit
    return bytes(out)


class BitWriter:
    """u(n), ue(v) and se(v) of Rec. ITU-T H.264 (08/2024) 9.1 and Table 9-3,
    then rbsp_trailing_bits and emulation prevention, 7.3.1."""

    def __init__(self) -> None:
        self.bits: list[int] = []

    def u(self, width: int, value: int) -> "BitWriter":
        self.bits += [(value >> (width - 1 - i)) & 1 for i in range(width)]
        return self

    def ue(self, value: int) -> "BitWriter":
        code = value + 1
        return self.u(code.bit_length() - 1, 0).u(code.bit_length(), code)

    def se(self, value: int) -> "BitWriter":
        return self.ue(2 * value - 1 if value > 0 else -2 * value)

    def nal(self, header: bytes) -> bytes:
        bits = self.bits + [1]
        bits += [0] * (-len(bits) % 8)
        raw = bytes(int("".join(map(str, bits[i : i + 8])), 2) for i in range(0, len(bits), 8))
        out, zeros = bytearray(header), 0
        for byte in raw:
            if zeros >= 2 and byte <= 3:
                out.append(3)
                zeros = 0
            out.append(byte)
            zeros = zeros + 1 if byte == 0 else 0
        return bytes(out)


def delta_scale_sps(delta: int) -> bytes:
    """An Annex B High profile SPS, Rec. ITU-T H.264 (08/2024) 7.3.2.1.1,
    whose first scaling list opens with ``delta``. 7.4.2.1.1.1 bounds
    delta_scale to -128 to +127, so both clips built from this are malformed.
    The stream ends inside the SPS, so the demuxer must refuse at the field
    rather than run past it."""
    writer = BitWriter()
    writer.u(8, 100).u(8, 0).u(8, 10).ue(0)  # profile_idc, flags, level_idc, id
    writer.ue(1).ue(0).ue(0).u(1, 0)  # chroma_format_idc 1, bit depths, bypass
    writer.u(1, 1).u(1, 1).se(delta)  # scaling matrix, list 0 present, delta
    for _ in range(15):
        writer.se(0)
    return b"\x00\x00\x00\x01" + writer.nal(b"\x67")


def delta_scale_high(_directory: Path) -> bytes:
    """delta_scale 2**31 - 1, the review's input that overflowed i32."""
    return delta_scale_sps(2**31 - 1)


def delta_scale_low(_directory: Path) -> bytes:
    """delta_scale -(2**31 - 1), the lowest value se(v) gives in 32 bits."""
    return delta_scale_sps(-(2**31 - 1))


def pps_before_sps(directory: Path) -> bytes:
    """``h264_gop12.h264`` with its first two parameter set NAL units, the
    SPS, nal_unit_type 7, and the PPS, 8, in Table 7-1, exchanged."""
    stream = (directory / "h264_gop12.h264").read_bytes()
    starts = []
    position = 0
    while (found := stream.find(b"\x00\x00\x01", position)) != -1:
        starts.append(found - 1 if found > 0 and stream[found - 1] == 0 else found)
        position = found + 3
    units = [stream[start:end] for start, end in zip(starts, starts[1:] + [len(stream)])]
    kinds = [unit[unit.index(b"\x00\x00\x01") + 3] & 0x1F for unit in units]
    sps, pps = kinds.index(7), kinds.index(8)
    assert pps == sps + 1, kinds[:6]
    units[sps], units[pps] = units[pps], units[sps]
    return stream[: starts[0]] + b"".join(units)


BUILDERS = {
    "pps_before_sps": pps_before_sps,
    "delta_scale_high": delta_scale_high,
    "delta_scale_low": delta_scale_low,
    "bare_pes": bare_pes,
    "garbage": garbage,
    "trailing_zero_ts": trailing_zero_ts,
    "two_per_pes_ts": two_per_pes_ts,
    "hevc_trailing_zero_ts": hevc_trailing_zero_ts,
    "temporal_reference_wrap": temporal_reference_wrap,
}


def ffmpeg_version() -> str:
    completed = subprocess.run(
        ["ffmpeg", "-version"], check=True, capture_output=True, text=True
    )
    return completed.stdout.splitlines()[0].strip()


def build(directory: Path) -> None:
    for name, recipe in CLIPS:
        output = directory / name
        if isinstance(recipe, str):
            output.write_bytes(BUILDERS[recipe](directory))
            continue
        args = [str(directory / a[1:]) if a.startswith("@") else a for a in recipe]
        completed = subprocess.run(
            ["ffmpeg", *BASE, *args, *OUTPUT, str(output)],
            check=False,
            capture_output=True,
            text=True,
        )
        if completed.returncode != 0:
            raise SystemExit(f"ffmpeg failed for {name}:\n{completed.stderr}")


def digests(directory: Path) -> dict[str, str]:
    return {
        name: hashlib.sha256((directory / name).read_bytes()).hexdigest()
        for name, _ in CLIPS
    }


def read_sums() -> tuple[str, dict[str, str]]:
    version = ""
    sums: dict[str, str] = {}
    for line in SUMS.read_text().splitlines():
        if line.startswith("# "):
            version = line[2:]
        elif line.strip():
            digest, name = line.split(maxsplit=1)
            sums[name] = digest
    return version, sums


def write_sums(version: str, sums: dict[str, str]) -> None:
    lines = [f"# {version}"]
    lines += [f"{digest}  {name}" for name, digest in sums.items()]
    SUMS.write_text("\n".join(lines) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="rebuild in place")
    arguments = parser.parse_args()
    version = ffmpeg_version()
    if arguments.write:
        build(HERE)
        write_sums(version, digests(HERE))
        print(f"wrote {len(CLIPS)} clips with {version}")
        return 0
    recorded_version, recorded = read_sums()
    scratch = Path(tempfile.mkdtemp(prefix="f241-video-"))
    try:
        build(scratch)
        rebuilt = digests(scratch)
    finally:
        shutil.rmtree(scratch)
    failures = 0
    for name, digest in rebuilt.items():
        tracked = hashlib.sha256((HERE / name).read_bytes()).hexdigest()
        if recorded.get(name) != digest or tracked != digest:
            failures += 1
            print(f"MISMATCH {name}")
    if failures:
        print(f"{failures} of {len(CLIPS)} clips differ")
        print(f"recorded with {recorded_version}")
        print(f"rebuilt with {version}")
        return 1
    print(f"OK: {len(CLIPS)} clips rebuild byte for byte with {version}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
