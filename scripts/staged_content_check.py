#!/usr/bin/env python3
"""Refuse a commit carrying patient data or a build artefact.

The corpus never enters this repository (`corpus/README.md`), and the reason is
not only size. A set labelled de-identified can still carry burned-in pixel
annotation, which is exactly why HLD story E22.3 exists. A repository that
never contains DICOM cannot leak one, so the check is a hard refusal with no
allowlist rather than a scan for identifiers.

It also refuses the built wasm module and node_modules, which `.gitignore`
already covers, because `git add -f` exists and a 4 MB binary in history is not
removable by a later commit.

Usage:
  python3 scripts/staged_content_check.py            # staged (pre-commit)
  python3 scripts/staged_content_check.py --tracked  # audit the whole tree
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DICOM_SUFFIXES = {".dcm", ".dicom", ".ima"}
ARTEFACT_PARTS = {"node_modules", "target", "pkg", "dist"}
MAX_BYTES = 2 * 1024 * 1024

# Oracle output (F-010). A reference frame of a real corpus row is a rendered
# PICTURE of patient data, and every real row in corpus/manifest.tsv carries
# `burned-in-unchecked` because HLD story E22.3, which would detect burned-in
# annotation, is not built. The DICOM refusal below would not catch a PNG, and
# neither would the size limit: a 512 by 512 PNG is tens of kilobytes.
#
# `.gitignore` already covers this path. That is not enough on its own, because
# `git add -f` exists and an image in history is not removed by a later commit.
# The guard belongs here, as a mechanism, and not there, as a habit.
ORACLE_OUTPUT_PREFIXES = ("tools/oracle/out/",)

# Spike output (F-X006, Appendix A gates A1 and A2). The same reasoning as
# above, applied to a different derived artefact. The throwaway harnesses write
# raw JPEG 2000 and JPEG-LS codestreams and decoded pixel buffers, all extracted
# from corpus rows, into this directory. A `.j2c` or a `.jls` is not DICOM by
# magic bytes, so the refusal below would not catch it, and at a kilobyte or two
# neither would the size limit.
#
# Separate from ORACLE_OUTPUT_PREFIXES rather than folded into it, because the
# two carry different remedies and a guard that names the wrong one wastes the
# time of the person who trips it.
SPIKE_OUTPUT_PREFIXES = ("tools/spikes/out/",)

# Comparator output (F-011). The same reasoning again, applied to the third
# derived artefact. `<id>.diff.raw` is the per-lane absolute difference between
# two frames of a corpus row, so it is derived from that row exactly as a
# reference frame is, and `compare.json` names every view. A `.raw` is not
# DICOM by magic bytes and a difference image of a 512 by 512 frame is a
# megabyte, which is under the size limit.
#
# Separate from ORACLE_OUTPUT_PREFIXES for the reason the spike block gives:
# the remedy is a different command, and a guard that names the wrong one
# wastes the time of the person who trips it.
COMPARE_OUTPUT_PREFIXES = ("tools/oracle/compare-out/",)

# DICOM Part 10 preamble: 128 zero bytes then "DICM". A file with no suffix
# is checked by magic, because `anon001` is a very normal way to receive one.
DICM_OFFSET = 128
DICM_MAGIC = b"DICM"


def looks_like_dicom(name: str, data: bytes) -> bool:
    if Path(name).suffix.lower() in DICOM_SUFFIXES:
        return True
    end = DICM_OFFSET + len(DICM_MAGIC)
    return len(data) >= end and data[DICM_OFFSET:end] == DICM_MAGIC


def listed_names(tracked: bool) -> list[str]:
    # NUL-separated names. A space is part of one path. Splitting on
    # whitespace dropped that path and the guard then reported the empty
    # remainder as clean.
    # T is a type change, such as a symlink becoming a file. That new blob
    # is what the commit records. D stays out: a deletion has no blob.
    command = (["git", "ls-files", "-z"] if tracked else
               ["git", "diff", "--cached", "--name-only", "-z",
                "--diff-filter=ACMRT"])
    completed = subprocess.run(command, cwd=ROOT, capture_output=True,
                               text=True, check=True)
    return [name for name in completed.stdout.split("\0") if name]


def index_bytes(name: str) -> bytes:
    # The stage is explicit. `:0:name` is stage 0 of `name`, so a path
    # whose name is `0:name` has to be written `:0:0:name`.
    completed = subprocess.run(["git", "show", f":0:{name}"], cwd=ROOT,
                               capture_output=True, check=True)
    return completed.stdout


def payload(name: str, tracked: bool) -> bytes:
    # Staged mode reads the index. A deleted or reverted worktree file is
    # not the commit. Tracked mode reads the worktree when that file is
    # still there, and the index when it is not.
    path = ROOT / name
    if tracked and path.is_file():
        return path.read_bytes()
    return index_bytes(name)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tracked", action="store_true")
    args = parser.parse_args()

    try:
        names = listed_names(args.tracked)
    except (subprocess.CalledProcessError, OSError, UnicodeError):
        print("FAIL: staged content inventory is unavailable")
        return 1

    problems = []
    for name in names:
        path = Path(name)
        try:
            data = payload(name, args.tracked)
        except (OSError, subprocess.CalledProcessError):
            problems.append(f"{name}: content is unreadable")
            continue
        parts = set(path.parts)

        if name.startswith(ORACLE_OUTPUT_PREFIXES):
            problems.append(
                f"{name}: oracle output. A reference frame of a real corpus "
                f"row is a rendered picture of patient data, and every real "
                f"row is marked burned-in-unchecked. Nothing under "
                f"tools/oracle/out/ is ever committed. Regenerate it with "
                f"`bin/ocelli.sh oracle`.")
            continue

        if name.startswith(COMPARE_OUTPUT_PREFIXES):
            problems.append(
                f"{name}: comparator output. A difference image under "
                f"tools/oracle/compare-out/ is derived from a corpus row the "
                f"same way a reference frame is, and every real row is marked "
                f"burned-in-unchecked. Nothing here is ever committed. "
                f"Regenerate it with `bin/ocelli.sh compare`.")
            continue

        if name.startswith(SPIKE_OUTPUT_PREFIXES):
            problems.append(
                f"{name}: spike output. Raw codestreams and decoded pixel "
                f"buffers under tools/spikes/out/ are extracted from corpus "
                f"rows and are derived from them, so nothing here is ever "
                f"committed. Regenerate it with "
                f"`uv run tools/spikes/common/extract.py`.")
            continue

        if looks_like_dicom(name, data):
            problems.append(
                f"{name}: this is DICOM. No patient data enters this "
                f"repository, ever. There is no allowlist. Put it under "
                f"the ignored corpus/data directory and add a manifest row "
                f"with `uv run scripts/corpus_check.py --add`.")
            continue

        if parts & ARTEFACT_PARTS:
            problems.append(
                f"{name}: build artefact. It is gitignored, so this was "
                f"forced. A binary in history is not removed by a later "
                f"commit.")
            continue

        size = len(data)
        if size > MAX_BYTES and path.suffix not in {".md", ".json", ".tsv",
                                                    ".lock", ".docx", ".xlsx"}:
            problems.append(
                f"{name}: {size:,} bytes, over the {MAX_BYTES:,} byte limit. "
                f"If it belongs in the repository, say why in the design plan "
                f"and add its suffix to this script.")

    if problems:
        print("FAIL: staged content")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: no patient data or build artefacts staged")
    return 0


if __name__ == "__main__":
    sys.exit(main())
