#!/usr/bin/env python3
"""Find every refusal site in this repository's guard sources.

A census that counts what somebody remembered to list is not a census. This
module finds the sites mechanically, so a refusal added next month is
discovered by the machine rather than by the author remembering to declare it.

## What a refusal site is

The shapes a refusal takes in this repository, and nothing else:

| Shape | Language | Looks like |
|-------|----------|------------|
| `problems.append(` | Python | a collected problem, printed under a `FAIL:` header |
| `print("FAIL...` | Python | a refusal that prints and returns 1 without collecting |
| `sys.exit("...")` | Python | an immediate refusal carrying a message |
| `raise SystemExit(` or `raise SomethingError(` | Python | the same, in a module that is imported rather than run |
| `throw new Error(` | JavaScript | the oracle's and the harness's refusals |
| `echo "FAIL...` | shell | the two `ci/` guards and the hooks |
| `exit 1` | shell | a shell refusal with no message of its own |

A site's IDENTITY is its file plus a normalised fragment of the message it
produces, and deliberately not `file:line`. Moving a refusal within a file must
not churn the catalogue, and rewording a message must force a re-read of what
the refusal is for. That is the same trade `docs/lld/oracle.md` records for the
fault catalogue's `expect` fragments.

## What it does not find

A refusal expressed as a return value that a caller turns into an exit status
without a message of its own, and a refusal inside `crates/`. The second is the
scope boundary decision 7 of `.claude/plans/F-X009-design.md` records: a
runtime refusal inside a crate is that crate's story's test, not this one's.
"""

from __future__ import annotations

import hashlib
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent

# The scan roots. `crates/` is deliberately absent, which is decision 7 of the
# design plan: a runtime refusal inside a crate belongs to that crate's story.
SCAN_ROOTS = ("scripts/", "ci/", ".githooks/", "bin/", "tools/")

# Sources that are generated, vendored or are themselves test fixtures. A test
# suite's own assertions are not guards, and scanning them would make every
# `throw new Error` in a test a site needing a catalogue entry.
SCAN_EXCLUDE = (
    "tools/oracle/tests/",
    "tools/bench/tests/",
    "scripts/tests/",
    "tools/oracle/node_modules/",
    "tools/bench/node_modules/",
    "tools/oracle/page/vendor/",
)

SCAN_SUFFIXES = {".py", ".mjs", ".js", ".sh"}

STRING = re.compile(r'"(?:[^"\\]|\\.)*"' r"|'(?:[^'\\]|\\.)*'"
                    r"|`(?:[^`\\]|\\.)*`")

SHAPES: dict[str, re.Pattern[str]] = {
    "py-problem": re.compile(r"problems\.append\(", re.M),
    # `scripts/verify_ledger.py` refuses by printing and returning 1 rather
    # than by collecting, and four of its five branches are invisible to the
    # `problems.append` shape. Adding this shape found them, which is the
    # bidirectional discipline working in the direction that matters.
    "py-print-fail": re.compile(r"print\(\s*f?[\"']FAIL", re.M),
    "py-exit": re.compile(r"sys\.exit\(\s*(?=[\"'f])", re.M),
    # `raise SystemExit(` and `raise SomethingError(` are the same refusal in
    # a module that is imported rather than run. `scripts/guards/sandbox.py`
    # refuses this way and the shape was added because the census reported the
    # entry for it as claiming nothing, which is the bidirectional discipline
    # working in the direction that matters.
    "py-raise": re.compile(r"raise (?:SystemExit|[A-Za-z]*Error)\(\s*(?=[\"'f])",
                           re.M),
    "js-throw": re.compile(r"throw new Error\(", re.M),
    "sh-fail": re.compile(r'echo\s+"FAIL', re.M),
    "sh-exit": re.compile(r"^\s*exit\s+1\s*$", re.M),
}


@dataclass(frozen=True)
class Site:
    """One refusal, identified by its file and the words it prints."""

    file: str
    shape: str
    message: str
    line: int

    @property
    def key(self) -> str:
        digest = hashlib.sha256(self.message.encode("utf-8")).hexdigest()
        return f"{self.file}#{digest[:10]}"


def _normalise(text: str) -> str:
    """Collapse a message expression to the words it prints.

    Concatenated literals, f-string placeholders and line wrapping are all
    formatting rather than meaning, so they are removed. What survives is the
    prose a person reads when the guard fires, which is what the catalogue
    claims and what a reword must change.
    """
    pieces = []
    for match in STRING.finditer(text):
        body = match.group(0)[1:-1]
        pieces.append(body)
    joined = " ".join(pieces) if pieces else text
    joined = re.sub(r"\{[^{}]*\}", " ", joined)
    joined = joined.replace("\\n", " ").replace("\\t", " ")
    joined = re.sub(r"\s+", " ", joined)
    return joined.strip()


def _balanced(text: str, open_at: int) -> str:
    """The argument list of a call whose opening bracket is at `open_at`."""
    depth = 0
    index = open_at
    while index < len(text):
        char = text[index]
        if char in "\"'`":
            match = STRING.match(text, index)
            index = match.end() if match else index + 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return text[open_at + 1:index]
        index += 1
    return text[open_at:open_at + 400]


def _sites_in(rel: str, text: str) -> list[Site]:
    found: list[Site] = []
    for shape, pattern in SHAPES.items():
        for match in pattern.finditer(text):
            line = text.count("\n", 0, match.start()) + 1
            if shape in {"sh-fail", "sh-exit"}:
                raw = text[match.start():text.find("\n", match.start())]
                message = _normalise(raw) or f"exit 1 at line {line}"
                if shape == "sh-exit":
                    # A bare `exit 1` carries no words of its own, so its
                    # identity is the file and the shape. There are eight in
                    # the repository and each is the tail of a guard whose
                    # message was printed above it.
                    message = f"{rel}: bare shell refusal"
            else:
                bracket = text.index("(", match.start())
                message = _normalise(_balanced(text, bracket))
            if not message:
                message = f"{rel}:{line}"
            found.append(Site(file=rel, shape=shape, message=message,
                              line=line))
    # A bare shell refusal has one identity per file, not one per occurrence.
    deduped: dict[str, Site] = {}
    for site in found:
        deduped.setdefault(site.key, site)
    return sorted(deduped.values(), key=lambda s: (s.line, s.shape))


def tracked_sources() -> list[str]:
    out = subprocess.run(["git", "ls-files"], cwd=ROOT, capture_output=True,
                         text=True, check=True).stdout
    names = []
    for name in out.splitlines():
        if not name.startswith(SCAN_ROOTS):
            continue
        if name.startswith(SCAN_EXCLUDE):
            continue
        if Path(name).suffix in SCAN_SUFFIXES or name.startswith(".githooks/"):
            names.append(name)
    return sorted(names)


def discover(root: Path | None = None) -> list[Site]:
    base = root or ROOT
    sites: list[Site] = []
    for rel in tracked_sources():
        path = base / rel
        if not path.is_file():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        sites.extend(_sites_in(rel, text))
    return sites


if __name__ == "__main__":
    by_file: dict[str, list[Site]] = {}
    for site in discover():
        by_file.setdefault(site.file, []).append(site)
    total = 0
    for name, group in sorted(by_file.items()):
        print(f"\n### {name}  ({len(group)})")
        for site in group:
            total += 1
            print(f"  {site.key}  [{site.shape}] {site.message[:150]}")
    print(f"\n{total} refusal site(s) in {len(by_file)} file(s)")
