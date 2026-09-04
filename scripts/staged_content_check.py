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

# DICOM Part 10 preamble: 128 zero bytes then "DICM". A file with no suffix
# is checked by magic, because `anon001` is a very normal way to receive one.
DICM_OFFSET = 128
DICM_MAGIC = b"DICM"


def looks_like_dicom(path: Path) -> bool:
    if path.suffix.lower() in DICOM_SUFFIXES:
        return True
    try:
        with path.open("rb") as handle:
            handle.seek(DICM_OFFSET)
            return handle.read(4) == DICM_MAGIC
    except OSError:
        return False


def staged(tracked: bool) -> list[str]:
    cmd = (["git", "ls-files"] if tracked else
           ["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"])
    try:
        return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                              check=True).stdout.split()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tracked", action="store_true")
    args = parser.parse_args()

    problems = []
    for name in staged(args.tracked):
        path = ROOT / name
        if not path.is_file():
            continue
        parts = set(Path(name).parts)

        if looks_like_dicom(path):
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

        size = path.stat().st_size
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
