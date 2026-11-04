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
a lint, which is the wrong way round.

**`CHANGELOG.md` is not in scope at all.** Not "its released sections are
exempt", which is what an earlier version of this docstring implied and what
`.claude/WORKFLOW.md` repeated. The file is not in INCLUDE_EXACT and does not
sit under an INCLUDE_PREFIXES directory, so `## Unreleased` is unchecked too.
S01 found a stale gate count there that nothing would have caught.

Fenced blocks, inline code spans, link targets and markdown table rows are
exempt, the last because a row's cells are data rather than sentences. Indentation is
not a code signal this checker honours, because a four-space indent under a
list bullet is a continuation paragraph rather than code and exempting it would
let real prose past both rules.

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
                 "docs/DEVELOPER_SETUP.md", "CONTRIBUTING.md", "SECURITY.md"}

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
        if fenced:
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


def listed_names(staged: bool) -> list[str]:
    # T is a type change, such as a symlink becoming a file. That new blob
    # is what the commit records. D stays out: a deletion has no blob.
    command = (["git", "diff", "--cached", "--name-only", "-z",
                "--diff-filter=ACMRT"] if staged else ["git", "ls-files", "-z"])
    completed = subprocess.run(command, cwd=ROOT, capture_output=True,
                               text=True, check=True)
    return [name for name in completed.stdout.split("\0") if name]


def load_text(name: str, staged: bool) -> str | None:
    # Staged mode reads the index blob. A reverted worktree copy is not the
    # commit. Tracked mode reads the worktree file when it is present.
    if staged:
        # The stage is explicit. `:0:name` is stage 0 of `name`, so a path
        # whose name is `0:name` has to be written `:0:0:name`.
        completed = subprocess.run(["git", "show", f":0:{name}"], cwd=ROOT,
                                   capture_output=True, text=True, check=True)
        return completed.stdout
    path = ROOT / name
    if not path.is_file():
        return None
    return path.read_text(encoding="utf-8")


def texts(staged: bool) -> tuple[list[tuple[str, str]], list[str]]:
    found: list[tuple[str, str]] = []
    problems: list[str] = []
    for name in listed_names(staged):
        if not in_scope(name):
            continue
        try:
            text = load_text(name, staged)
        except (subprocess.CalledProcessError, OSError, UnicodeError):
            problems.append(f"{name}: prose source is unreadable")
            continue
        if text is None:
            continue
        found.append((name, text))
    return found, problems


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
        try:
            targets, unread = texts(args.staged)
        except (subprocess.CalledProcessError, OSError, UnicodeError):
            print("FAIL: voice inventory is unavailable")
            return 1
        checked = len(targets)
        problems += unread
        for label, text in targets:
            problems += scan(text, label)

    if problems:
        print("FAIL: voice rules")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: voice rules clean over {checked} file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
