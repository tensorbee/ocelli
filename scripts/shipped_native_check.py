#!/usr/bin/env python3
"""The shipped native build compiles no objc2 crate on either Apple target.

F-238, acceptance rows AC1 and AC2. The shipped native archive is
`ocelli-native` built with `--no-default-features`, which turns off its
`metal` feature and, through it, wgpu's Metal backend and the objc2 crates the
backend reaches. This check reads the RESOLVED graph for that feature set and
refuses any package whose name is `objc2` or begins with `objc2-`, for
`aarch64-apple-darwin` and for `x86_64-apple-darwin`.

It reads the graph and not the manifest feature lists on purpose. A check that
read the feature lists would stay green when a manifest edit put `metal` back
on a path the resolver still follows, which is exactly the planted state the
`shipped-native.metal-planted` probe builds. The command is `cargo tree` over
normal AND build edges: a build-dependency is compiled when the archive is
built, which is the verb the story row uses, and `licence_check.py`'s
`NATIVE_TREE_COMMAND` walks the same kinds so the two walks of "the shipped
graph" cannot disagree. Dev edges stay out, because a dev-dependency is
test-only and compiles into nothing the archive ships, and the dev graph is
also where a development build deliberately keeps Metal.

The refusal is over resolved packages and not over any manifest, so a
`metal` that reaches the graph through a path nobody forwarded is refused
all the same. A dev build that keeps Metal is not checked here at all,
because `gate gpu` needs it and the decode-graph guard already pins the
workspace resolve.

Usage:
  python3 scripts/shipped_native_check.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The two Apple targets the story names. Resolving for a triple needs no
# toolchain for that triple, because `cargo tree` resolves cfg rather than
# compiling, which is measured for the bindgen isolation check's wasm32 pass.
APPLE_TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin")

# `--prefix none` prints one package per line as `name vVersion`, with a
# ` (/path)` suffix for path dependencies and a trailing ` (*)` marking a
# subtree printed before. The name anchor ends at the space before `v`, so
# neither suffix can reach the name. Normal plus build edges, the same kinds
# `licence_check.py` walks for the archive's notices: a build-dependency is
# compiled at build time, and a dev-dependency is test-only and ships in
# nothing.
COMMAND = (
    "cargo", "tree", "--locked", "--offline", "-p", "ocelli-native",
    "--no-default-features", "-e", "normal,build", "--prefix", "none",
    "--format", "{p}",
)

# The objc2 family is `objc2` and every `-`-suffixed framework crate under
# it. A name that merely contains the substring, `notobjc2`, does not match,
# which the unit suite pins.
OBJC2 = re.compile(r"^objc2(?:-|$)")
PACKAGE = re.compile(r"^(?P<name>[A-Za-z0-9_-]+) v(?P<version>\S+)")


def package_names(stdout: str) -> list[tuple[str, str]]:
    """Every (name, version) the tree printed, repeats collapsed."""
    found: list[tuple[str, str]] = []
    seen: set[tuple[str, str]] = set()
    for raw in stdout.splitlines():
        match = PACKAGE.match(raw.strip())
        if match is None:
            continue
        pair = (match.group("name"), match.group("version"))
        if pair not in seen:
            seen.add(pair)
            found.append(pair)
    return found


def graph_problems(target: str, code: int, stdout: str,
                   stderr: str) -> list[str]:
    """The refusals for one target's resolved graph."""
    if code != 0:
        detail = ""
        for line in (stderr or stdout).splitlines():
            if line.strip():
                detail = line.strip()
                break
        return [
            f"the shipped native graph did not resolve for {target}: {detail}"
        ]
    return [
        f"{name} v{version} reaches the shipped native graph for {target}"
        for name, version in package_names(stdout)
        if OBJC2.match(name) is not None
    ]


def main() -> int:
    problems: list[str] = []
    for target in APPLE_TARGETS:
        completed = subprocess.run(
            [*COMMAND, "--target", target], cwd=ROOT,
            capture_output=True, text=True, check=False)
        problems.extend(graph_problems(
            target, completed.returncode, completed.stdout, completed.stderr))
    if problems:
        print("FAIL: shipped native graph")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: the shipped native graph compiles no objc2 crate on "
          + ", ".join(APPLE_TARGETS))
    return 0


if __name__ == "__main__":
    sys.exit(main())
