#!/usr/bin/env python3
"""The `no_std` posture of the core crates, enforced.

Most crates under `crates/` declare

    #![cfg_attr(not(test), no_std)]

and deviation D-09 disables glam's default `std` feature to keep that true.
Nothing checked it. This does.

**Six crates do not declare it, and each absence is deliberate.**
`ocelli-wasm` and `ocelli-native` are the two entry points. `ocelli-render` and
`ocelli-compute` link wgpu, which needs `std`, and that is part of deviation
D-10 rather than an oversight. `ocelli-dicom` links dicom-rs, whose parser,
dictionary and object crates require `std`, under D-18. The expected set below
is explicit. `ocelli-codec` follows HLD section 21's prescribed `HashMap` and
`Arc` registry under F-023. A crate that silently enters or leaves a
self-selected set makes the check agree with the defect. Any posture change
therefore moves a named value in review.

## Why the obvious check does not work

The intuitive test is "does it compile for wasm32". It does not catch this,
and believing it does is worse than having no check:

    cargo check -p ocelli-core --target wasm32-unknown-unknown

`wasm32-unknown-unknown` ships a `std` implementation, and a `no_std` crate may
depend on a `std` crate without any error. So the command above exits 0 whether
or not D-09 is in force. That was measured during S01, by reverting the
workspace entry to `glam = "0.30"` and watching every gate stay green.

A genuinely bare-metal target such as `thumbv7em-none-eabi` would catch it, at
the cost of a toolchain target every clone then downloads for one assertion.

## What this checks instead

The dependency graph, which is where the fact actually lives. For each crate
that declares `no_std`, `cargo tree -e normal,features` is asked whether any
dependency reaches a `std` feature. Feature edges are what Cargo resolves, so
this sees through a default-features flag being dropped, a new dependency
arriving with `std` on, and feature unification pulling `std` in from a third
crate that has nothing to do with the first two.

**Dev-dependencies are excluded, deliberately.** `proptest` and `trybuild` need
`std` and are only ever built for the test target, where `cfg(test)` has
already turned `no_std` off. Including them would make this fail on correct
code, which is the fastest way to get a check deleted.

Usage:
  python3 scripts/no_std_check.py
  python3 scripts/no_std_check.py --verbose
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"

NO_STD = re.compile(r"^\s*#!\[cfg_attr\(not\(test\),\s*no_std\)\]", re.M)

EXPECTED_NO_STD_CRATES = frozenset({
    "ocelli-cache",
    "ocelli-core",
    "ocelli-geom",
    "ocelli-pixel",
    "ocelli-seg",
    "ocelli-viewport",
    "ocelli-volume",
})

# `cargo tree` prefixes every line below the root with box-drawing characters:
#
#     ocelli-core v0.1.0 (/path)
#     └── glam feature "default"
#         ├── glam v0.30.10
#         └── glam feature "std"
#
# So an anchored pattern that expects the crate name first matches nothing, and
# the check reports clean on a tree that is not. That is exactly what this
# script did on its first run, and it was caught only by deliberately breaking
# a crate and watching for red. Search the line, do not anchor it.
STD_FEATURE = re.compile(r'(\S+)\s+feature\s+"std"')


def declares_no_std(crate: Path) -> bool:
    lib = crate / "src" / "lib.rs"
    if not lib.is_file():
        return False
    try:
        return NO_STD.search(lib.read_text(encoding="utf-8")) is not None
    except (UnicodeDecodeError, OSError):
        return False


def feature_tree(name: str) -> tuple[str, str | None]:
    """Return (stdout, error). `cargo tree` needs a resolved graph."""
    try:
        done = subprocess.run(
            ["cargo", "tree", "-p", name, "-e", "normal,features"],
            cwd=ROOT, capture_output=True, text=True, check=False)
    except FileNotFoundError:
        return "", "cargo is not on PATH"
    if done.returncode != 0:
        return "", done.stderr.strip().splitlines()[-1] if done.stderr else \
            f"cargo tree exited {done.returncode}"
    return done.stdout, None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--verbose", action="store_true")
    args = parser.parse_args()

    declared = {c.name for c in CRATES.iterdir()
                if c.is_dir() and declares_no_std(c)}
    if not declared:
        print("FAIL: no crate under crates/ declares no_std.")
        print("Either the posture was abandoned, in which case delete this")
        print("check deliberately, or lib.rs stopped matching the attribute")
        print("this script looks for. Both are worth knowing.")
        return 1

    problems: list[str] = []
    for name in sorted(EXPECTED_NO_STD_CRATES - declared):
        problems.append(f"{name}: stopped declaring no_std")
    for name in sorted(declared - EXPECTED_NO_STD_CRATES):
        problems.append(f"{name}: unexpectedly declares no_std")
    if problems:
        print("FAIL: the declared no_std crate set changed")
        for problem in problems:
            print(f"  {problem}")
        print("Change EXPECTED_NO_STD_CRATES in the same reviewed posture "
              "change if this is deliberate.")
        return 1

    crates = [CRATES / name for name in sorted(EXPECTED_NO_STD_CRATES)]
    for crate in crates:
        name = crate.name
        out, error = feature_tree(name)
        if error is not None:
            problems.append(f"{name}: could not resolve the graph, {error}")
            continue
        hits = [line for line in out.splitlines() if STD_FEATURE.search(line)]
        if hits:
            problems.append(
                f"{name} declares no_std and reaches a std feature:")
            problems += [f"    {h.strip()}" for h in dict.fromkeys(hits)]
        elif args.verbose:
            print(f"  {name}: clean")

    if problems:
        print("FAIL: a no_std crate reaches std through its dependencies")
        for problem in problems:
            print(f"  {problem}")
        print()
        print("This is not fixed by removing the no_std attribute. The")
        print("attribute is what keeps the wasm module small, and the")
        print("dependency is what broke it. Add default-features = false to")
        print("the workspace entry and select the no_std feature the crate")
        print("offers, which for a maths crate is usually libm. See")
        print("docs/hld/DEVIATIONS.md D-09 for the worked case.")
        return 1

    print(f"OK: {len(crates)} no_std crate(s) reach no std feature")
    return 0


if __name__ == "__main__":
    sys.exit(main())
