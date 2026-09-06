#!/usr/bin/env python3
"""Parse and execute the explicitly marked examples in canonical skills.

Markers are a narrow, column-zero protocol. The checker parses every skill
before starting any process, then runs each example under python3 in its own
temporary working directory. Unmarked fences are documentation, not code.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, Sequence

ROOT = Path(__file__).resolve().parent.parent
SKILLS = ROOT / ".claude" / "skills"

START = re.compile(
    r"<!-- ocelli-example: id=([a-z0-9]+(?:-[a-z0-9]+)*) "
    r"interpreter=python3 mode=(stdout|assert) -->"
)
BACKTICK_FENCE_START = re.compile(r"^ {0,3}(`{3,})[^`]*$")
TILDE_FENCE_START = re.compile(r"^ {0,3}(~{3,}).*$")
LIST_ITEM = re.compile(r"^( {0,3})([-+*]|[0-9]{1,9}[.)])( {1,4})(.*)$")
END = "<!-- /ocelli-example -->"
PYTHON_FENCE = "```python"
OUTPUT_FENCE = "```text"
CLOSE_FENCE = "```"
MARKER_TOKEN = "ocelli-example"
TIMEOUT_SECONDS = 5.0
DIAGNOSTIC_LIMIT = 2_000


class ExampleError(ValueError):
    """A malformed declaration or failed example."""


@dataclass(frozen=True)
class Example:
    id: str
    mode: str
    code: str
    expected_stdout: str | None
    source: Path
    line: int


@dataclass(frozen=True)
class Fence:
    character: str
    minimum: int
    container: str
    depth: int


def _fail(path: Path, line: int, message: str) -> ExampleError:
    return ExampleError(f"{path}:{line}: {message}")


def _block(lines: list[str], start: int, path: Path,
           kind: str) -> tuple[list[str], int]:
    """Read through an exact closing fence and return its body and index."""
    end = start
    while end < len(lines) and lines[end] != CLOSE_FENCE:
        if MARKER_TOKEN in lines[end]:
            raise _fail(path, end + 1,
                        f"{kind} fence contains an example marker")
        end += 1
    if end == len(lines):
        raise _fail(path, start, f"{kind} fence is not closed")
    return lines[start:end], end


def _quoted_body(line: str) -> tuple[int, str]:
    """Return the blockquote depth and content after its container markers."""
    depth = 0
    body = line
    while True:
        match = re.match(r"^ {0,3}> ?", body)
        if match is None:
            return depth, body
        depth += 1
        body = body[match.end():]


def _opening_run(line: str) -> str | None:
    """Return a valid CommonMark fence run, excluding its info string."""
    for pattern in (BACKTICK_FENCE_START, TILDE_FENCE_START):
        match = pattern.fullmatch(line)
        if match is not None:
            return match.group(1)
    return None


def _fence_start(line: str, list_indent: int | None) -> Fence | None:
    """Recognise a top-level, blockquoted or current-list fenced block."""
    if list_indent is not None and line.startswith(" " * list_indent):
        run = _opening_run(line[list_indent:])
        if run is not None:
            return Fence(run[0], len(run), "list", list_indent)

    quote_depth, quoted = _quoted_body(line)
    if quote_depth:
        run = _opening_run(quoted)
        if run is not None:
            return Fence(run[0], len(run), "quote", quote_depth)

    run = _opening_run(line)
    if run is not None:
        return Fence(run[0], len(run), "top", 0)
    return None


def _fence_line(line: str, fence: Fence) -> str | None:
    """Return content relative to an active fence's container."""
    if fence.container == "quote":
        depth, body = _quoted_body(line)
        return body if depth >= fence.depth else None
    if fence.container == "list":
        prefix = " " * fence.depth
        return line[fence.depth:] if line.startswith(prefix) else None
    return line


def _fence_closes(line: str, fence: Fence) -> bool:
    body = _fence_line(line, fence)
    if body is None:
        return False
    return re.fullmatch(
        rf" {{0,3}}{re.escape(fence.character)}{{{fence.minimum},}}[ \t]*",
        body,
    ) is not None


def parse_skill(path: Path) -> list[Example]:
    """Parse every marked example in one skill and reject near-markers."""
    try:
        lines = path.read_bytes().decode("utf-8").splitlines()
    except UnicodeDecodeError as error:
        raise ExampleError(f"{path}: invalid UTF-8: {error}") from error
    examples: list[Example] = []
    index = 0
    unmarked_fence: Fence | None = None
    list_indent: int | None = None
    while index < len(lines):
        line = lines[index]
        if unmarked_fence is not None:
            if _fence_line(line, unmarked_fence) is None:
                unmarked_fence = None
                continue
            if _fence_closes(line, unmarked_fence):
                unmarked_fence = None
            index += 1
            continue

        item = LIST_ITEM.fullmatch(line)
        if item is not None:
            list_indent = sum(len(item.group(part)) for part in (1, 2, 3))
            fence = _fence_start(item.group(4), 0)
            if fence is not None:
                unmarked_fence = Fence(
                    fence.character, fence.minimum, "list", list_indent
                )
                index += 1
                continue
        elif line.strip() and (list_indent is None
                               or len(line) - len(line.lstrip(" "))
                               < list_indent):
            list_indent = None

        fence = _fence_start(line, list_indent)
        if fence is not None:
            unmarked_fence = fence
            index += 1
            continue

        if MARKER_TOKEN not in line:
            index += 1
            continue

        match = START.fullmatch(line)
        if match is None:
            if line == END:
                raise _fail(path, index + 1, "example end marker has no start")
            raise _fail(
                path,
                index + 1,
                "example marker must use the exact column-zero grammar",
            )

        example_id, mode = match.groups()
        marker_line = index + 1
        index += 1
        if index == len(lines) or lines[index] != PYTHON_FENCE:
            raise _fail(path, index + 1,
                        "example marker must be followed by a python fence")

        code_lines, index = _block(lines, index + 1, path, "python")
        if not code_lines or not any(part.strip() for part in code_lines):
            raise _fail(path, marker_line, "example code is empty")
        code = "\n".join(code_lines) + "\n"

        index += 1
        expected: str | None = None
        if mode == "stdout":
            if index == len(lines) or lines[index] != OUTPUT_FENCE:
                raise _fail(path, index + 1,
                            "stdout example must have a following text fence")
            output_lines, index = _block(lines, index + 1, path, "output")
            if not output_lines:
                raise _fail(path, marker_line,
                            "stdout example has empty expected output")
            expected = "\n".join(output_lines) + "\n"
            index += 1

        if index == len(lines) or lines[index] != END:
            raise _fail(path, index + 1,
                        "example must end immediately after its fences")

        examples.append(Example(example_id, mode, code, expected, path,
                                marker_line))
        index += 1
    return examples


def canonical_skill_paths(skills: Path | None = None,
                          repository: Path = ROOT) -> list[Path]:
    """List canonical skills and refuse a path resolving outside that root."""
    expected = repository / ".claude" / "skills"
    candidate = expected if skills is None else skills
    if candidate.absolute() != expected.absolute():
        raise ExampleError(f"{candidate}: is not this repository's skills root")
    if candidate.is_symlink():
        raise ExampleError(f"{candidate}: canonical skills root is a symlink")
    try:
        repo_root = repository.resolve(strict=True)
        root = candidate.resolve(strict=True)
    except OSError as error:
        raise ExampleError(f"{candidate}: cannot resolve canonical skills root") from error
    if not root.is_relative_to(repo_root):
        raise ExampleError(f"{candidate}: canonical skills root escapes repository")
    paths = sorted(candidate.glob("*/SKILL.md"))
    for path in paths:
        try:
            resolved = path.resolve(strict=True)
        except OSError as error:
            raise ExampleError(f"{path}: cannot resolve canonical skill") from error
        if not resolved.is_relative_to(root):
            raise ExampleError(f"{path}: canonical skill symlink escapes {candidate}")
    return paths


def parse_all(paths: Iterable[Path]) -> list[Example]:
    """Parse all skills and validate the global declaration before running."""
    examples: list[Example] = []
    seen: dict[str, Example] = {}
    for path in sorted(paths):
        for example in parse_skill(path):
            previous = seen.get(example.id)
            if previous is not None:
                raise _fail(
                    example.source,
                    example.line,
                    f"duplicate example id {example.id!r}, first declared at "
                    f"{previous.source}:{previous.line}",
                )
            seen[example.id] = example
            examples.append(example)
    if not examples:
        raise ExampleError("no marked skill examples found")
    return examples


def _bounded(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        value = value.decode("utf-8", errors="replace")
    if len(value) <= DIAGNOSTIC_LIMIT:
        return value
    omitted = len(value) - DIAGNOSTIC_LIMIT
    return f"{value[:DIAGNOSTIC_LIMIT]}\n... {omitted} characters omitted"


def execute(example: Example) -> None:
    """Run one already-parsed example with bounded failure diagnostics."""
    with tempfile.TemporaryDirectory(prefix="ocelli-skill-example-") as work:
        environment = {
            "HOME": work,
            "LANG": "C",
            "LC_ALL": "C",
            "PATH": os.environ.get("PATH", os.defpath),
            "TMPDIR": work,
        }
        argv = ["python3", "-I", "-B", "-"]
        try:
            completed = subprocess.run(
                argv,
                input=example.code.encode("utf-8"),
                cwd=work,
                env=environment,
                capture_output=True,
                timeout=TIMEOUT_SECONDS,
                check=False,
            )
        except subprocess.TimeoutExpired as error:
            detail = _bounded(error.stderr or error.stdout)
            suffix = f": {detail}" if detail else ""
            raise ExampleError(
                f"{example.source}:{example.line}: {example.id} timed out "
                f"after {TIMEOUT_SECONDS:g}s{suffix}"
            ) from error

    if completed.returncode != 0:
        detail = _bounded(completed.stderr or completed.stdout).rstrip()
        raise ExampleError(
            f"{example.source}:{example.line}: {example.id} exited "
            f"{completed.returncode}: {detail}"
        )
    if completed.stderr:
        raise ExampleError(
            f"{example.source}:{example.line}: {example.id} wrote stderr: "
            f"{_bounded(completed.stderr).rstrip()}"
        )
    if example.mode == "assert":
        if completed.stdout:
            raise ExampleError(
                f"{example.source}:{example.line}: {example.id} assert mode "
                f"wrote stdout: {_bounded(completed.stdout).rstrip()}"
            )
        return
    expected = (example.expected_stdout or "").encode("utf-8")
    if completed.stdout != expected:
        raise ExampleError(
            f"{example.source}:{example.line}: {example.id} stdout differs\n"
            f"expected:\n{_bounded(expected)}"
            f"actual:\n{_bounded(completed.stdout)}"
        )


def check_paths(paths: Iterable[Path],
                runner: Callable[[Example], None] = execute) -> list[Example]:
    """Parse the entire selection, then execute it in declaration order."""
    examples = parse_all(paths)
    for example in examples:
        runner(example)
    return examples


def main(argv: Sequence[str] | None = None) -> int:
    args = list(sys.argv[1:] if argv is None else argv)
    if args:
        print("FAIL: skill example checker takes no arguments", file=sys.stderr)
        return 2
    try:
        examples = check_paths(canonical_skill_paths())
    except (ExampleError, OSError) as error:
        print(f"FAIL: skill examples\n  {error}", file=sys.stderr)
        return 1
    print(f"OK: {len(examples)} marked skill example(s) passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
