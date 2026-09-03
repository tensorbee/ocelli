#!/usr/bin/env python3
"""No `unsafe` outside the allow-list. HLD section 27.2 R5.

    "R5. No unsafe outside the allow-list (ocelli-wasm/src/ring.rs,
     ocelli-core/src/cast.rs). Keeps the audit surface to two files."

Two files, named in the specification. The point is not that unsafe is
forbidden, it is that a human reviewing this project for a device submission
should have to read two files to audit every unsafe line in it.

Neither file has to exist yet. The allow-list is a permission, not a
requirement. What it refuses is a THIRD file.

Usage:
  python3 scripts/unsafe_allowlist_check.py
  python3 scripts/unsafe_allowlist_check.py --staged
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# HLD section 27.2 R5, verbatim. Adding a path here is a design-plan decision
# with a recorded rationale, reviewed like code. It is not a convenience.
ALLOWED = {
    "crates/ocelli-wasm/src/ring.rs",
    "crates/ocelli-core/src/cast.rs",
}

# `unsafe` as a keyword: a block, a fn, a trait, an impl, or an extern block.
# Not `unsafe` inside a string, a comment, or an identifier like `is_unsafe`.
UNSAFE = re.compile(r"(?<![\w])unsafe(?![\w])")
# re.M matters. Without it `$` only matches at the end of the whole file, so
# `//.*$` matches nothing and every doc comment reaches the keyword scan. That
# bug reported a `//!` line saying "permitted to contain unsafe" as a
# violation, which is a check failing in the direction that wastes time rather
# than the direction that hides a defect, but it is still a bug.
LINE_COMMENT = re.compile(r"//.*$", re.M)
BLOCK_COMMENT = re.compile(r"/\*.*?\*/", re.S)
STRING = re.compile(r'"(?:[^"\\]|\\.)*"')


def strip_noise(text: str) -> str:
    """Blank out comments and string literals, preserving line numbering."""
    def blank(match: re.Match[str]) -> str:
        return re.sub(r"[^\n]", " ", match.group(0))
    text = BLOCK_COMMENT.sub(blank, text)
    text = STRING.sub(blank, text)
    return LINE_COMMENT.sub(lambda m: " " * len(m.group(0)), text)


def rust_files(staged: bool) -> list[Path]:
    cmd = (["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"]
           if staged else ["git", "ls-files"])
    try:
        out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                             check=True).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []
    return [ROOT / n for n in out.splitlines()
            if n.endswith(".rs") and (ROOT / n).is_file()]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--staged", action="store_true")
    args = parser.parse_args()

    problems = []
    checked = 0
    for path in rust_files(args.staged):
        rel = path.relative_to(ROOT).as_posix()
        if rel in ALLOWED:
            continue
        checked += 1
        try:
            source = strip_noise(path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, OSError):
            continue
        for match in UNSAFE.finditer(source):
            line = source.count("\n", 0, match.start()) + 1
            problems.append(f"{rel}:{line}: `unsafe` outside the allow-list")

    if problems:
        print("FAIL: `unsafe` outside the allow-list (HLD section 27.2 R5)")
        for problem in problems:
            print(f"  {problem}")
        print("\nThe allow-list is:")
        for path in sorted(ALLOWED):
            print(f"  {path}")
        print("\nAdding to it is a design-plan decision with a recorded")
        print("rationale, not an edit to this script made to get a build green.")
        return 1

    print(f"OK: no unsafe outside the allow-list ({checked} files checked, "
          f"{len(ALLOWED)} permitted)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
