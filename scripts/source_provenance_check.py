#!/usr/bin/env python3
"""The source-provenance policy, enforced. docs/SOURCE-POLICY.md and HLD 27.2 R7.

Appendix A gate A6 says this policy must be agreed in writing BEFORE any agent
touches any repository, and the policy says why it is sharper here than usual:

    "Translating source into Rust is a translation, which is an exclusive right
     of the copyright holder, so a copyleft licence blocks READING, not merely
     depending. Agent-assisted development sharpens this: exposure cannot be
     shown to be absent after the fact, which weakens any clean-room position."

Three projects are read-blocked and none may be opened by a person or an agent
on this project: **dwv** (GPL-3.0) and **Horos** (LGPL-3 with a linked AGPL-3
component, Grok). Grok itself is listed because Horos links it, and an AGPL
component in a browser-delivered product would trigger network-use disclosure.

Where a blocked project's idea is worth having, take it from the standard.
dwv's annotations-as-DICOM-SR is in DICOM PS3.3 and PS3.16, which is where dwv
took it from.

What this checks, over tracked text:

1. No blocked project appears as a dependency in any manifest.
2. No blocked project appears as a URL anywhere, which is the form an agent's
   exposure actually takes: a fetched file, a cited source, a link in a plan.
3. Any mention of a blocked project sits next to an explicit negation, so
   this policy file and the HLD may name them and a design plan citing one as
   a source may not.

Usage:
  python3 scripts/source_provenance_check.py            # tracked files
  python3 scripts/source_provenance_check.py --staged   # staged files only
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# HLD Appendix C.2.1, the "Read?" column set to NO.
BLOCKED = {
    "dwv": "GPL-3.0. Translating it into Rust is a derivative work, and agent "
           "exposure weakens any clean-room position. Take the idea from "
           "DICOM PS3.3 and PS3.16 instead.",
    "horos": "LGPL-3 with a linked AGPL-3 component. Same trap as dwv.",
    "grok": "AGPL-3 JPEG 2000. Horos links it. An AGPL component in a "
            "browser-delivered product triggers network-use disclosure.",
}

BLOCKED_URLS = [
    re.compile(r"github\.com/ivmartel/dwv", re.I),
    re.compile(r"github\.com/horosproject", re.I),
    re.compile(r"github\.com/GrokImageCompression", re.I),
    re.compile(r"\bhorosproject\.org", re.I),
]

# A mention within this many characters of a negation is policy prose, not use.
NEGATIONS = re.compile(
    r"\b(not|never|no|refus|block|forbid|prohibit|must not|may not|avoid|"
    r"excluded|NO)\b", re.I)

MANIFESTS = {"Cargo.toml", "package.json", "Cargo.lock", "package-lock.json"}

TEXT_SUFFIXES = {".md", ".rs", ".ts", ".tsx", ".js", ".mjs", ".json", ".toml",
                 ".yaml", ".yml", ".sh", ".py", ".html", ".wgsl", ".tsv"}

# Files whose job is to state the policy. They name the blocked projects by
# necessity. Every one of them is still checked for URLs.
POLICY_FILES = {
    "scripts/source_provenance_check.py",
    "docs/SOURCE-POLICY.md",
    "docs/hld/26-differentiating-capabilities.md",
    "docs/hld/24-agent-code-standards.md",
    "CLAUDE.md",
    "AGENTS.md",
    ".claude/WORKFLOW.md",
    ".claude/commands/design.md",
    ".claude/commands/implement-feature.md",
    ".agents/skills/design/SKILL.md",
    ".agents/skills/implement-feature/SKILL.md",
}


def listed_names(staged: bool) -> list[str]:
    # T is a type change, such as a symlink becoming a file. That new blob
    # is what the commit records. D stays out: a deletion has no blob.
    command = (["git", "diff", "--cached", "--name-only", "-z",
                "--diff-filter=ACMRT"] if staged else ["git", "ls-files", "-z"])
    completed = subprocess.run(command, cwd=ROOT, capture_output=True,
                               text=True, check=True)
    return [name for name in completed.stdout.split("\0") if name]


def readable(name: str) -> bool:
    path = Path(name)
    return path.suffix in TEXT_SUFFIXES or path.name in MANIFESTS


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
        if not readable(name):
            continue
        try:
            text = load_text(name, staged)
        except (subprocess.CalledProcessError, OSError, UnicodeError):
            problems.append(f"{name}: source is unreadable")
            continue
        if text is None:
            continue
        found.append((name, text))
    return found, problems


def check_text(rel: str, text: str) -> list[str]:
    path = Path(rel)
    problems = []

    for pattern in BLOCKED_URLS:
        for match in pattern.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            problems.append(
                f"{rel}:{line}: links a read-blocked project ({match.group(0)}). "
                f"No one on this project, human or agent, may open it.")

    if path.name in MANIFESTS:
        for name, why in BLOCKED.items():
            if re.search(rf'"[^"]*\b{name}\b[^"]*"\s*:', text, re.I) or \
               re.search(rf'^\s*{name}\s*=', text, re.I | re.M):
                problems.append(f"{rel}: depends on {name}. {why}")

    if rel in POLICY_FILES:
        return problems

    for name, why in BLOCKED.items():
        for match in re.finditer(rf"\b{name}\b", text, re.I):
            start = max(0, match.start() - 200)
            window = text[start:match.end() + 200]
            if NEGATIONS.search(window):
                continue
            line = text.count("\n", 0, match.start()) + 1
            problems.append(
                f"{rel}:{line}: names '{match.group(0)}' with no statement "
                f"that it is out of bounds. {why}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--staged", action="store_true")
    args = parser.parse_args()

    try:
        files, unread = texts(args.staged)
    except (subprocess.CalledProcessError, OSError, UnicodeError):
        print("FAIL: source-provenance inventory is unavailable")
        return 1
    problems = list(unread)
    problems.extend(problem for rel, text in files
                    for problem in check_text(rel, text))

    if problems:
        print("FAIL: source-provenance policy violated "
              "(docs/SOURCE-POLICY.md, HLD section 27.2 R7)")
        for problem in problems:
            print(f"  {problem}")
        print("\nThere is no allowlist for this check. If a blocked project is")
        print("genuinely being described as out of bounds, say so in the same")
        print("sentence, which is what the policy asks for anyway.")
        return 1

    print(f"OK: source-provenance policy clean over {len(files)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
