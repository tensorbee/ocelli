#!/usr/bin/env python3
"""Every deviation from the HLD is declared in docs/hld/DEVIATIONS.md.

HLD Part II opens with the rule this enforces:

    "Where it gives a formula, a layout or a signature, that is the intended
     implementation and a deviation should be raised rather than improvised."

Two directions, both mechanical:

1. A design plan or review that cites a `D-NN` must find that row. A plan
   claiming an approved deviation that is not recorded is the failure mode
   this exists for.
2. `Cargo.toml` is checked against the specific claims D-01 makes about it, so
   a deviation cannot be recorded and then quietly not taken, or taken further
   than recorded. A stale deviation row is as misleading as a missing one.

Usage: python3 scripts/deviation_check.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEVIATIONS = ROOT / "docs" / "hld" / "DEVIATIONS.md"
CITATION = re.compile(r"\bD-(\d{2})\b")


def declared() -> set[str]:
    text = DEVIATIONS.read_text()
    return {f"D-{n}" for n in re.findall(r"^\|\s*D-(\d{2})\s*\|", text, re.M)}


def main() -> int:
    problems: list[str] = []
    rows = declared()
    if not rows:
        problems.append("DEVIATIONS.md declares no deviations. It should at "
                        "least carry the bootstrap set.")

    # 1. Citations resolve.
    for folder in (".claude/plans", ".claude/reviews", "docs/lld"):
        base = ROOT / folder
        if not base.is_dir():
            continue
        for path in base.rglob("*.md"):
            for match in CITATION.finditer(path.read_text(encoding="utf-8")):
                token = f"D-{match.group(1)}"
                if token not in rows:
                    rel = path.relative_to(ROOT).as_posix()
                    problems.append(
                        f"{rel} cites {token}, which is not a row in "
                        f"docs/hld/DEVIATIONS.md. A deviation is raised "
                        f"there, reviewed like code, before it is relied on.")

    # 2. D-01's specific claims about Cargo.toml are still true.
    cargo = (ROOT / "Cargo.toml").read_text()
    if "D-01" in rows:
        if 'rust-version = "1.97.1"' not in cargo:
            problems.append(
                "D-01 records rust-version 1.97.1 and Cargo.toml does not say "
                "that. Either the deviation was reverted and the row is stale, "
                "or the toolchain moved without updating it.")
        if 'resolver = "3"' not in cargo:
            problems.append(
                "D-01 records resolver 3 and Cargo.toml does not say that.")

    if problems:
        print("FAIL: deviation register")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: {len(rows)} deviations declared, every citation resolves, "
          f"D-01 matches Cargo.toml")
    return 0


if __name__ == "__main__":
    sys.exit(main())
