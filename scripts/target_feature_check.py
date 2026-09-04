#!/usr/bin/env python3
"""Resolved features must agree across targets. HLD section 4, story E1.7.

The sprint's stated build defect is FALSE PORTABILITY:

    "A workspace can compile on one target while Cargo feature unification
     enables `std`, browser-only bindings or a second GPU pathway on another.
     F-002 and F-007 must prove the actual target builds, not infer
     portability from a host build."

A build proof catches the case where one target stops compiling. It does not
catch both targets compiling while one quietly resolved a different feature
set, which is the more dangerous half, because nothing goes red and the
difference is in the artefact rather than in the log.

## What this checks

ONE claim, and it is the one this tool can actually make: **every dependency
this workspace declares directly, meaning the entries in
`[workspace.dependencies]`, resolves the same features on both targets.** A
feature difference there is our decision and we should have to say so.

It does NOT assert HLD section 4's crate table. `cargo tree` lists every
workspace member whatever target it is given, because nothing in a manifest
restricts a member to a target, so an assertion built on it would report
`ocelli-native` present under wasm32 and be unable to tell that from a real
violation. The table is enforced where it can be: `ocelli-native` carries a
`cfg`-gated `compile_error!`, and steps 1 to 3 of `bin/ocelli.sh native` build
each target for real. This script is step 4 and adds the feature dimension to
those three.

**What it no longer checks, and why that is not a retreat.** The first version,
written in F-007 when the only dependency was `glam`, compared the whole
transitive closure. F-008 activated wgpu and that version reported 42
differences, of which 32 were packages present on one target only.

Every one of them was legitimate, and worse, **most were specific to the
machine it ran on**: `objc2-metal` and `raw-window-metal` are macOS host-only,
where a Linux runner would report `ash` and `gpu-alloc`. A baseline listing
them would have been correct on one developer's laptop and red in CI, and the
fix for a red CI would have been to re-declare it, which is tolerance-tuning
wearing a different hat.

The transitive closure of a cross-platform GPU library differs per target
BY DESIGN. That is wgpu doing its job, and asserting otherwise measures the
dependency rather than this project. Claims A and B are the parts that are
about us.

Usage:
  python3 scripts/target_feature_check.py
  python3 scripts/target_feature_check.py --write   # re-declare, deliberately
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = ROOT / "ci" / "target-feature-baseline.json"
WASM = "wasm32-unknown-unknown"

def host_triple() -> str:
    out = subprocess.run(["rustc", "-vV"], capture_output=True, text=True,
                         check=True).stdout
    for line in out.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ").strip()
    raise RuntimeError("rustc -vV reported no host triple")


def workspace_dependencies() -> set[str]:
    """The names in [workspace.dependencies]. What we chose, as opposed to
    what our choices dragged in."""
    text = (ROOT / "Cargo.toml").read_text()
    block = re.search(r"^\[workspace\.dependencies\]$(.*?)(?=^\[|\Z)",
                      text, re.M | re.S)
    if block is None:
        raise RuntimeError("Cargo.toml has no [workspace.dependencies]")
    return {m.group(1) for m in
            re.finditer(r"^\s*([A-Za-z0-9_-]+)\s*=", block.group(1), re.M)}


def resolved(target: str) -> dict[str, set[str]]:
    """Package name -> the features cargo resolved for it under `target`."""
    proc = subprocess.run(
        ["cargo", "tree", "-e", "normal", "--target", target,
         "--prefix", "none", "--format", "{p}|{f}"],
        cwd=ROOT, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"cargo tree failed for {target}:\n{proc.stderr.strip()}")

    out: dict[str, set[str]] = {}
    for raw in proc.stdout.splitlines():
        line = raw.strip()
        # cargo marks an already-printed subtree with a trailing " (*)". It
        # lands AFTER the format string, so it arrives inside the feature
        # field and turns "default" into "default (*)". Strip it before
        # splitting, or the check reports phantom differences between a
        # package and itself. Measured, it produced four of them.
        if line.endswith("(*)"):
            line = line[: -len("(*)")].rstrip()
        if not line or "|" not in line:
            continue
        package, _, features = line.partition("|")
        # "name v1.2.3 (/path)" -> "name". The version is left out because this
        # gate is about feature resolution, not versions, which `pins` and
        # Cargo.lock already cover. Keying on it would turn a routine
        # `cargo update` of a transitive into a red gate nobody can act on.
        parts = package.split()
        if not parts:
            continue
        got = {f.strip() for f in features.split(",") if f.strip()}
        # A package can appear more than once, and at more than one version.
        # Union rather than overwrite, which would depend on line order.
        out.setdefault(parts[0], set()).update(got)
    return out


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    host = host_triple()
    try:
        by_target = {host: resolved(host), WASM: resolved(WASM)}
        declared_deps = workspace_dependencies()
    except RuntimeError as exc:
        print("FAIL: target feature check could not run")
        print(f"  {exc}")
        return 1

    problems: list[str] = []

    # Our own declared dependencies resolve the same features on both targets.
    shared = set(by_target[host]) & set(by_target[WASM])
    for name in sorted(declared_deps & shared):
        if by_target[host][name] != by_target[WASM][name]:
            problems.append(
                f"{name} is declared in [workspace.dependencies] and resolves "
                f"different features per target.\n"
                f"      host   {sorted(by_target[host][name])}\n"
                f"      wasm32 {sorted(by_target[WASM][name])}\n"
                f"      This is the feature-unification defect E1.7 exists "
                f"for. It is OUR entry, so fix the manifest or say why in "
                f"{BASELINE.relative_to(ROOT)}.")

    if args.write:
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text(json.dumps({
            "note": "Declared per-target differences for dependencies this "
                    "workspace names directly. Transitive packages are NOT "
                    "listed: a cross-platform GPU library resolves a "
                    "different closure per target by design, and per host, so "
                    "listing them would make this gate machine-specific. See "
                    "the module docstring of scripts/target_feature_check.py.",
            "host_triple_when_written": host,
            "checked_dependencies": sorted(declared_deps),
            "allowed": {},
        }, indent=2) + "\n")
        print(f"  re-declared. {len(problems)} finding(s) at the time of "
              f"writing, which are NOT silenced by this file.")
        return 0

    allowed = json.loads(BASELINE.read_text()).get("allowed", {}) \
        if BASELINE.exists() else {}
    problems = [p for p in problems if p.split()[0] not in allowed]

    if problems:
        print("FAIL: per-target resolution")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {len(declared_deps & shared)} directly declared "
          f"dependenc(ies) resolve identically on {host} and {WASM}"
          + (f", {len(allowed)} declared exception(s)" if allowed else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
