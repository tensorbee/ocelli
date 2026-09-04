#!/usr/bin/env python3
"""Resolved features must agree across targets. HLD section 4, story E1.7.

The sprint's stated build defect is FALSE PORTABILITY:

    "A workspace can compile on one target while Cargo feature unification
     enables `std`, browser-only bindings or a second GPU pathway on another.
     F-002 and F-007 must prove the actual target builds, not infer
     portability from a host build."

A build proof catches the case where one target stops compiling. It does not
catch the case where both targets compile and one of them quietly resolved a
different feature set, which is the more dangerous half, because nothing goes
red and the difference is in the artefact rather than in the log.

So this compares `cargo tree`'s resolved features for the host and for
wasm32-unknown-unknown, and reports three things separately:

1. A package present on BOTH targets whose feature set differs. This is the
   feature-unification defect and it fails unless declared.
2. A package present on only ONE target. Legitimate and expected, the
   wasm-bindgen chain being the obvious case, so these are compared against
   the declared list rather than reported as differences.
3. A declared entry that no longer describes anything. A stale allowance is as
   misleading as a missing one, so it fails too.

`ci/target-feature-baseline.json` carries the declarations, each with a reason.

Usage:
  python3 scripts/target_feature_check.py
  python3 scripts/target_feature_check.py --write   # re-declare, deliberately
"""

from __future__ import annotations

import argparse
import json
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


def resolved(target: str) -> dict[str, set[str]]:
    """Package name -> the set of features cargo resolved for it.

    `--target` is passed explicitly for BOTH targets, host included. Relying on
    cargo's default would make the host arm's meaning depend on where it ran,
    and the whole point here is to compare two named targets.
    """
    proc = subprocess.run(
        ["cargo", "tree", "-e", "normal", "--target", target,
         "--prefix", "none", "--format", "{p}|{f}"],
        cwd=ROOT, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"cargo tree failed for {target}:\n{proc.stderr.strip()}")

    out: dict[str, set[str]] = {}
    for line in proc.stdout.splitlines():
        line = line.strip()
        if not line or "|" not in line:
            continue
        package, _, features = line.partition("|")
        # "name v1.2.3 (/path)" -> "name". Neither the version nor the path is
        # part of the key, and both omissions are deliberate.
        #
        # The path is machine-local and would make the comparison depend on the
        # checkout location. The VERSION is left out because this gate is about
        # feature resolution and not about dependency versions, which `pins`
        # and `Cargo.lock` already cover. Keying on the version would turn every
        # routine `cargo update` of a transitive like `syn` into a red gate
        # reporting a stale declaration, and a gate that goes red for reasons
        # nobody can act on is a gate people learn to re-baseline.
        parts = package.split()
        if not parts:
            continue
        key = parts[0]
        got = {f for f in features.split(",") if f}
        # A package can appear more than once in a tree, and at more than one
        # version. Union rather than overwrite, which would otherwise depend on
        # line order. The comparison is then "everything this package resolved
        # under host" against "everything it resolved under wasm32", which is
        # the conservative direction.
        out.setdefault(key, set()).update(got)
    return out


def load_baseline() -> dict:
    if not BASELINE.exists():
        return {"feature_differences": {}, "target_only": {}}
    return json.loads(BASELINE.read_text())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()

    host = host_triple()
    try:
        by_target = {host: resolved(host), WASM: resolved(WASM)}
    except RuntimeError as exc:
        print("FAIL: target feature check could not run")
        print(f"  {exc}")
        return 1

    shared = set(by_target[host]) & set(by_target[WASM])
    differences = {
        name: {
            "host": sorted(by_target[host][name]),
            "wasm32": sorted(by_target[WASM][name]),
        }
        for name in sorted(shared)
        if by_target[host][name] != by_target[WASM][name]
    }
    only = {
        "host": sorted(set(by_target[host]) - set(by_target[WASM])),
        "wasm32": sorted(set(by_target[WASM]) - set(by_target[host])),
    }

    if args.write:
        BASELINE.parent.mkdir(parents=True, exist_ok=True)
        BASELINE.write_text(json.dumps({
            "note": "Declared per-target resolution differences, each with a "
                    "reason. Written by scripts/target_feature_check.py "
                    "--write, which is a deliberate act recorded in a design "
                    "plan. A difference not declared here fails the `native` "
                    "gate, and a declaration that no longer describes "
                    "anything fails it too.",
            "host_triple_when_written": host,
            "feature_differences": {
                k: {**v, "reason": "TODO, state why this is legitimate"}
                for k, v in differences.items()
            },
            "target_only": {
                "host": {name: "TODO, state why" for name in only["host"]},
                "wasm32": {name: "TODO, state why" for name in only["wasm32"]},
            },
        }, indent=2) + "\n")
        print(f"  declared {len(differences)} feature difference(s), "
              f"{len(only['host'])} host-only and {len(only['wasm32'])} "
              f"wasm32-only package(s)")
        return 0

    base = load_baseline()
    declared_diff = base.get("feature_differences", {})
    declared_only = base.get("target_only", {"host": {}, "wasm32": {}})

    problems: list[str] = []

    for name, sets in differences.items():
        if name not in declared_diff:
            problems.append(
                f"{name} resolves different features per target and that is "
                f"not declared.\n"
                f"      host   {sets['host']}\n"
                f"      wasm32 {sets['wasm32']}\n"
                f"      This is the feature-unification defect E1.7 exists "
                f"for. Fix the manifest, or declare it with a reason in "
                f"{BASELINE.relative_to(ROOT)}.")

    for which in ("host", "wasm32"):
        for name in only[which]:
            if name not in declared_only.get(which, {}):
                problems.append(
                    f"{name} appears only under {which} and that is not "
                    f"declared. Legitimate target-only dependencies exist, "
                    f"and each one is a claim somebody should have made on "
                    f"purpose.")

    for name in declared_diff:
        if name not in differences:
            problems.append(
                f"{name} is declared as a per-target feature difference and "
                f"no longer is one. A stale declaration is as misleading as a "
                f"missing one. Remove it.")
    for which in ("host", "wasm32"):
        for name in declared_only.get(which, {}):
            if name not in only[which]:
                problems.append(
                    f"{name} is declared as {which}-only and no longer is. "
                    f"Remove it.")

    if problems:
        print("FAIL: resolved features differ across targets")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {len(shared)} package(s) resolve identically on {host} and "
          f"{WASM}, {len(declared_diff)} declared difference(s), "
          f"{len(declared_only.get('host', {}))} host-only and "
          f"{len(declared_only.get('wasm32', {}))} wasm32-only declared")
    return 0


if __name__ == "__main__":
    sys.exit(main())
