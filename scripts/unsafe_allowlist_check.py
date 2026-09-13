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

# Vendored third-party packages, which are dependencies that happen to be in
# the tree rather than code this repository writes.
#
# **They are not simply excluded, and the reason is R5's own sentence.** R5's
# payoff is that a device-submission reviewer reads two files to audit every
# unsafe line, and a vendored package with unsafe in it makes that false
# whether the package is tracked or resolved from a registry. Excluding
# `vendor/` silently would leave R5 passing mechanically while its stated
# purpose was weakened, which is the shape `docs/spikes/A2-jpeg-ls.md` already
# warned about for the `charls` routes.
#
# So the count is RECORDED rather than ignored. A vendored package with no
# record is refused, and a package whose count moves is refused, so the number
# lands in front of a reviewer in the diff that changes it. The audit itself,
# which files and which constructs, is in `docs/SOURCE-POLICY.md`.
VENDOR_ROOT = "vendor/"
VENDORED_UNSAFE = {
    # D-20. The published package contains no unsafe Rust at all.
    "ritk-codecs-0.6.0": 0,
    # D-22. 104 constructs in nine files, of which the scalar memory, wavelet
    # and colour paths are the ones reachable on wasm32. The architecture SIMD
    # files hold the rest and are compiled only for x86_64 and aarch64.
    "openjph-core-0.1.0": 104,
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
    vendored: dict[str, int] = {}
    for path in rust_files(args.staged):
        rel = path.relative_to(ROOT).as_posix()
        if rel in ALLOWED:
            continue
        try:
            source = strip_noise(path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, OSError):
            continue
        if rel.startswith(VENDOR_ROOT):
            package = rel[len(VENDOR_ROOT):].split("/", 1)[0]
            vendored[package] = vendored.get(package, 0) + len(
                UNSAFE.findall(source))
            continue
        checked += 1
        for match in UNSAFE.finditer(source):
            line = source.count("\n", 0, match.start()) + 1
            problems.append(f"{rel}:{line}: `unsafe` outside the allow-list")

    for package in sorted(set(vendored) | set(VENDORED_UNSAFE)):
        found = vendored.get(package)
        recorded = VENDORED_UNSAFE.get(package)
        if recorded is None:
            problems.append(
                f"vendored package {package} has {found} `unsafe` construct(s) "
                f"and no recorded count. A vendored package is a dependency "
                f"this repository ships, so its audit surface is recorded in "
                f"VENDORED_UNSAFE and in docs/SOURCE-POLICY.md rather than "
                f"being excluded silently")
        elif found is None:
            problems.append(
                f"vendored package {package} is recorded with {recorded} "
                f"`unsafe` construct(s) and is not in the tree. A record "
                f"nothing reads is not a ratchet")
        elif found != recorded:
            problems.append(
                f"vendored package {package} has {found} `unsafe` "
                f"construct(s), recorded {recorded}. Re-audit it, update "
                f"docs/SOURCE-POLICY.md, and move the number in the same diff")

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

    audit = ", ".join(f"{package} {count}"
                      for package, count in sorted(vendored.items()))
    print(f"OK: no unsafe outside the allow-list ({checked} files checked, "
          f"{len(ALLOWED)} permitted)"
          + (f", vendored audit surface unchanged: {audit}" if vendored else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
