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
| `problems += [` | Python | several collected at once, or one written as a list |
| `problems.extend(` | Python | the same, spelled as a method call |
| `return ["..."]` | Python | a check that RETURNS its problems rather than collecting into a caller's list |
| `print("FAIL...` | Python | a refusal that prints and returns 1 without collecting |
| `sys.exit("...")` | Python | an immediate refusal carrying a message |
| `raise SystemExit(` or `raise SomethingError(` | Python | the same, in a module that is imported rather than run |
| `throw new Error(` | JavaScript | the oracle's and the harness's refusals |
| `echo "FAIL...` | shell | the two `ci/` guards and the hooks |
| `exit 1` | shell | a shell refusal with no message of its own |

The middle three arrived in the S03 review's sixth pass and they were not
hypothetical. `docs/lld/guards.md` claimed `entry_sites` meant "a refusal added
to an already-claimed file moves a number", and that was false for any refusal
built as a list: adding `problems += [f"..."]` to `scripts/lint_policy_check.py`
left the census headline byte-identical and exit 0. Eight refusals in scanned
guard files were invisible, `scripts/pin_and_size_check.py`'s wasm SIZE CEILING
among them, which is gate A4's own number. Deleting that ceiling left the
headline unmoved and only a probe caught it. The declared limit below said the
scan misses a returned refusal "without a message of its own", and every one of
these carries a message, so the declaration did not cover them.

A site's IDENTITY is its file plus a normalised fragment of the message it
produces, and deliberately not `file:line`. Moving a refusal within a file must
not churn the catalogue, and rewording a message must force a re-read of what
the refusal is for. That is the same trade `docs/lld/oracle.md` records for the
fault catalogue's `expect` fragments.

**What that identity costs.** Two different refusals in one file whose messages
normalise to the same words are one site, so a probe on either reads as
covering both. That is deliberate for the bare `exit 1` shape, which has no
words of its own, and it is a real loss everywhere else. `sites_collapsed()`
below counts it and `scripts/guard_census.py` prints the number, so the loss is
reported rather than assumed to be zero.

## What it does not find

A refusal expressed as a bare return value that a caller turns into an exit
status, `return False` and `return 1` above all, because those carry no words
and there is nothing for a catalogue entry to claim. A returned list of MESSAGES
is found, which is the correction the sixth pass made. Also not found: a message
built into a local variable and appended in a later statement, and a refusal
inside `crates/`. The last is the scope boundary decision 7 of
`.claude/plans/F-X009-design.md` records: a runtime refusal inside a crate is
that crate's story's test, not this one's.

Nor does it find one in prose. A Python docstring or comment that quotes a
refusal shape, such as the table above, is masked before the scan runs, because
a documentation row counted as a refusal inflates the census's headline with
prose and makes deleting a paragraph turn the gate red.
"""

from __future__ import annotations

import hashlib
import io
import re
import subprocess
import tokenize
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
    # The three list shapes. A refusal does not stop being one because it was
    # written with `+=` instead of `.append`, and eight in scanned guard files
    # were invisible until the S03 review's sixth pass, including the wasm size
    # ceiling of HLD Appendix A gate A4. `return [f"..."]` is the shape a check
    # that hands its problems back to a caller uses, which is how
    # `scripts/pin_and_size_check.py` and `scripts/bench_check.py` are written
    # throughout.
    "py-problem-list": re.compile(r"problems\s*\+=\s*\[", re.M),
    "py-problem-extend": re.compile(r"problems\.extend\(", re.M),
    "py-return-list": re.compile(r"return\s*\[\s*f?[\"']", re.M),
    # `scripts/verify_ledger.py` refuses by printing and returning 1 rather
    # than by collecting, so every one of its eight refusal sites is invisible
    # to the `problems.append` shape: it has none. Adding this shape found
    # them, which is the bidirectional discipline working in the direction that
    # matters.
    #
    # The optional placeholder before FAIL is not cosmetic. `guard_probe.py`
    # writes `print(f"{RED}FAIL{OFF}: the guard harness")`, so without it the
    # census that refuses a refusal no entry claims could not see the
    # top-level refusal of the file that runs it.
    "py-print-fail": re.compile(r"print\(\s*f?[\"'](?:\{[^{}\"']*\})?FAIL",
                                re.M),
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


# Which bracket opens each shape's message. A call's argument list is in
# parentheses and a list refusal's is in square brackets, and reading the wrong
# one takes the message from whatever punctuation came next.
SHAPE_BRACKET = {"py-problem-list": "[", "py-return-list": "["}


def _balanced(text: str, open_at: int, opening: str = "(") -> str:
    """The argument list of a call whose opening bracket is at `open_at`."""
    closing = ")" if opening == "(" else "]"
    depth = 0
    index = open_at
    while index < len(text):
        char = text[index]
        if char in "\"'`":
            match = STRING.match(text, index)
            index = match.end() if match else index + 1
            continue
        if char == opening:
            depth += 1
        elif char == closing:
            depth -= 1
            if depth == 0:
                return text[open_at + 1:index]
        index += 1
    return text[open_at:open_at + 400]


def mask_python_prose(text: str) -> str:
    """Blank Python docstrings and comments, preserving every byte offset.

    The shape table at the top of this module is SEVEN markdown rows quoting
    seven refusal shapes, of which five are detectable when the table is
    scanned unmasked: `raise SystemExit(` is followed by a backtick where the
    shape wants a quote, and a bare `exit 1` inside a table cell is not at the
    start of its line. So the census's headline counted five refusals of prose
    and deleting the documentation turned the gate red. The earlier count here
    said five rows and five sites, which happened to name the right number of
    sites for the wrong reason. Masking is by replacement with spaces rather
    than deletion, so line numbers and the bracket-matching in `_balanced` are
    unaffected.

    A refusal message is never a triple-quoted literal in this repository and
    never lives in a comment, so nothing real is masked. A file that does not
    tokenise is left alone: this module reports refusals, it does not police
    syntax.
    """
    try:
        tokens = list(tokenize.generate_tokens(io.StringIO(text).readline))
    except (tokenize.TokenError, IndentationError, SyntaxError, ValueError):
        return text
    offsets = [0]
    for line in text.splitlines(keepends=True):
        offsets.append(offsets[-1] + len(line))
    out = list(text)
    for token in tokens:
        body = token.string.lstrip("rbufRBUF")
        if token.type == tokenize.COMMENT:
            pass
        elif token.type == tokenize.STRING and body[:3] in ('"""', "'''"):
            pass
        else:
            continue
        try:
            start = offsets[token.start[0] - 1] + token.start[1]
            end = offsets[token.end[0] - 1] + token.end[1]
        except IndexError:  # pragma: no cover, a truncated token table
            continue
        for index in range(start, min(end, len(out))):
            if out[index] != "\n":
                out[index] = " "
    return "".join(out)


def _sites_in(rel: str, text: str) -> list[Site]:
    if rel.endswith(".py"):
        text = mask_python_prose(text)
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
                opening = SHAPE_BRACKET.get(shape, "(")
                bracket = text.index(opening, match.start())
                message = _normalise(_balanced(text, bracket, opening))
            if not message:
                message = f"{rel}:{line}"
            found.append(Site(file=rel, shape=shape, message=message,
                              line=line))
    # A bare shell refusal has one identity per file, not one per occurrence,
    # and any two refusals whose messages normalise identically collapse the
    # same way. `sites_collapsed` counts what that costs.
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


def _read(rel: str, base: Path) -> str | None:
    path = base / rel
    if not path.is_file():
        return None
    try:
        return path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return None


def discover(root: Path | None = None) -> list[Site]:
    base = root or ROOT
    sites: list[Site] = []
    for rel in tracked_sources():
        text = _read(rel, base)
        if text is not None:
            sites.extend(_sites_in(rel, text))
    return sites


def sites_collapsed(root: Path | None = None) -> int:
    """How many refusals a site's words-based identity merged away.

    Reported rather than assumed to be zero, because a probe on one of two
    refusals that normalise alike reads as covering both. The bare `exit 1`
    shape is excluded: it has no words of its own and one identity per file is
    its declared identity rather than a loss.
    """
    base = root or ROOT
    collapsed = 0
    for rel in tracked_sources():
        text = _read(rel, base)
        if text is None:
            continue
        if rel.endswith(".py"):
            text = mask_python_prose(text)
        raw = 0
        for shape, pattern in SHAPES.items():
            if shape == "sh-exit":
                continue
            raw += len(pattern.findall(text))
        kept = len([s for s in _sites_in(rel, text) if s.shape != "sh-exit"])
        collapsed += max(raw - kept, 0)
    return collapsed


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
