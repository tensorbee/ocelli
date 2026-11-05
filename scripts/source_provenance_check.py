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
import json
import re
import subprocess
import sys
import tomllib
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

MANIFESTS = {
    "Cargo.toml", "package.json", "Cargo.lock", "package-lock.json",
    "pyproject.toml", "uv.lock",
}

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


def _pep508_name(spec: str) -> str:
    """The package name, which ends at the first requirement marker."""
    match = re.match(r"\s*([^<>=!~\s;@\[]+)", spec)
    return match.group(1) if match else ""


def _cargo_dependency_names(text: str) -> list[str]:
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return []
    names: list[str] = []

    def take(table) -> None:
        if isinstance(table, dict):
            names.extend(str(key) for key in table)

    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        take(data.get(key))
    workspace = data.get("workspace")
    if isinstance(workspace, dict):
        take(workspace.get("dependencies"))
    target = data.get("target")
    if isinstance(target, dict):
        for spec in target.values():
            if isinstance(spec, dict):
                for key in ("dependencies", "dev-dependencies",
                            "build-dependencies"):
                    take(spec.get(key))
    return names


def _json_dependency_names(text: str) -> list[str]:
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return []
    names: list[str] = []
    for key in ("dependencies", "devDependencies", "optionalDependencies",
                "peerDependencies"):
        table = data.get(key)
        if isinstance(table, dict):
            names.extend(str(name) for name in table)
    bundled = data.get("bundledDependencies")
    if isinstance(bundled, list):
        names.extend(str(name) for name in bundled)
    return names


def _lockfile_names(text: str) -> list[str]:
    return re.findall(r'(?m)^[ \t]*name = "([^"]+)"', text)


def _npm_lock_names(text: str) -> list[str]:
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return []
    packages = data.get("packages")
    if not isinstance(packages, dict):
        return []
    names: list[str] = []
    for key, entry in packages.items():
        if key == "":
            continue
        parts = [part for part in key.split("/")
                 if part and part != "node_modules"]
        if parts:
            names.append(parts[-1])
        if isinstance(entry, dict) and isinstance(entry.get("name"), str):
            names.append(entry["name"])
    return names


def _project_dependency_names(text: str) -> list[str]:
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return []
    project = data.get("project")
    if not isinstance(project, dict):
        return []
    specs = project.get("dependencies")
    if not isinstance(specs, list):
        return []
    return [_pep508_name(spec) for spec in specs if isinstance(spec, str)]


def dependency_names(rel: str, text: str) -> list[str]:
    """Dependency names from one manifest grammar. Other text yields none."""
    filename = Path(rel).name
    if filename == "Cargo.toml":
        return _cargo_dependency_names(text)
    if filename in {"Cargo.lock", "uv.lock"}:
        return _lockfile_names(text)
    if filename == "pyproject.toml":
        return _project_dependency_names(text)
    if filename == "package.json":
        return _json_dependency_names(text)
    if filename == "package-lock.json":
        return _npm_lock_names(text)
    return []


def blocked_in_name(name: str) -> str | None:
    """A blocked project as a prefix of the final segment, with one lib prefix.

    `liberty` and `agrok` do not match. A name that begins with a blocked
    project does, and so does the same name after a single `lib`.
    """
    parts = [part for part in name.split("/") if part]
    segment = (parts[-1] if parts else name).casefold()
    for blocked in BLOCKED:
        if segment.startswith(blocked):
            return blocked
    if segment.startswith("lib"):
        rest = segment[3:]
        for blocked in BLOCKED:
            if rest.startswith(blocked):
                return blocked
    return None


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
        seen: set[str] = set()
        for dep in dependency_names(rel, text):
            blocked = blocked_in_name(dep)
            if blocked is None or blocked in seen:
                continue
            seen.add(blocked)
            problems.append(f"{rel}: depends on {blocked}. {BLOCKED[blocked]}")

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
