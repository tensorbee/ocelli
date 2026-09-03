#!/usr/bin/env python3
"""Voice rules over operator-facing prose.

Two rules, both about keeping generated prose plain:

- **No em-dash.** Use a hyphen, a comma, or rewrite the sentence.
- **No prose semicolon.** Use a full stop or a comma.

Scope is deliberately narrow. It covers what this project WRITES:
`.claude/plans/`, `.claude/reviews/`, `docs/sprints/`, `docs/lld/`, the root
markdown, and commit messages.

**`docs/hld/` is exempt and that is not an oversight.** Those files are cut
from the author's `.docx` by `scripts/split_hld.py` with nothing reworded.
Applying a voice rule to them would mean editing the specification to satisfy
a lint, which is the wrong way round. Same for `CHANGELOG.md`, whose released
sections are frozen once published.

Code, identifiers and fenced blocks are exempt everywhere.

Usage:
  python3 scripts/prose_check.py
  python3 scripts/prose_check.py --staged
  python3 scripts/prose_check.py --commit-msg .git/COMMIT_EDITMSG
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

INCLUDE_PREFIXES = (".claude/plans/", ".claude/reviews/", ".claude/commands/",
                    ".claude/skills/", "docs/sprints/", "docs/lld/",
                    "docs/runbooks/", "docs/spikes/")
INCLUDE_EXACT = {"README.md", "CLAUDE.md", "AGENTS.md", ".claude/WORKFLOW.md",
                 "corpus/README.md", "docs/hld/DEVIATIONS.md",
                 "docs/RELEASE.md", "docs/SOURCE-POLICY.md",
                 "docs/DEVELOPER_SETUP.md"}

EM_DASH = re.compile(r"[—–]")
SEMICOLON = re.compile(r";")

FENCE = re.compile(r"^\s*```")
INLINE_CODE = re.compile(r"`[^`]*`")
LINK_TARGET = re.compile(r"\]\([^)]*\)")


def in_scope(rel: str) -> bool:
    if rel in INCLUDE_EXACT:
        return True
    return rel.endswith(".md") and rel.startswith(INCLUDE_PREFIXES)


def scan(text: str, label: str) -> list[str]:
    problems, fenced = [], False
    for number, raw in enumerate(text.splitlines(), start=1):
        if FENCE.match(raw):
            fenced = not fenced
            continue
        if fenced or raw.lstrip().startswith(("    ", "\t")):
            continue
        line = LINK_TARGET.sub("", INLINE_CODE.sub("", raw))
        # A markdown table row is structure and its cells are DATA, not
        # sentences. Several are imported verbatim from the backlog
        # spreadsheet, and rewriting received data to satisfy a lint is the
        # wrong way round. Both rules are relaxed inside a row.
        if line.lstrip().startswith("|"):
            continue
        if EM_DASH.search(line):
            problems.append(f"{label}:{number}: em-dash. Use a hyphen, a "
                            f"comma, or rewrite the sentence.")
        if SEMICOLON.search(line):
            problems.append(f"{label}:{number}: semicolon in prose. Use a "
                            f"full stop or a comma.")
    return problems


def files(staged: bool) -> list[Path]:
    cmd = (["git", "diff", "--cached", "--name-only", "--diff-filter=ACMR"]
           if staged else ["git", "ls-files"])
    try:
        out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                             check=True).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        return []
    return [ROOT / n for n in out.splitlines()
            if in_scope(n) and (ROOT / n).is_file()]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--staged", action="store_true")
    parser.add_argument("--commit-msg", metavar="PATH")
    args = parser.parse_args()

    problems = []
    if args.commit_msg:
        path = Path(args.commit_msg)
        body = "\n".join(l for l in path.read_text().splitlines()
                         if not l.startswith("#"))
        problems += scan(body, "commit message")
        checked = 1
    else:
        targets = files(args.staged)
        checked = len(targets)
        for path in targets:
            problems += scan(path.read_text(encoding="utf-8"),
                             path.relative_to(ROOT).as_posix())

    if problems:
        print("FAIL: voice rules")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: voice rules clean over {checked} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
