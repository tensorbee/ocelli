#!/usr/bin/env python3
"""The exact dependency pins and the wasm size budget. HLD 27.2 R4 and E1.2.

Two checks that share a file because they share a failure mode: both are
about a number nobody looks at until it is wrong.

**The wgpu pin.** HLD section 15.2 pins `wgpu = "=30.0.1"` and says why:

    "The exact wgpu pin is not fussiness. Agents reliably emit wgpu 0.19-era
     pipeline code; a caret range lets that compile against something subtly
     different from what the shader expects."

R4 adds: "treat GPU code that compiles first try with suspicion". A caret or
tilde range on wgpu re-opens exactly the gap the pin closes, so the range form
is refused, not just a wrong version.

**`=` is not the pin. `=` and a full version is.** The S03 sprint review's
third pass measured the residue: this check tested only that the spec starts
with `=`, and `wgpu = "=30"` passed. Cargo reads a partial version after `=`
as a whole band, which is demonstrable rather than a reading of the
documentation:

    wgpu = "=30.0.0"   cargo metadata  -> Downgrading wgpu v30.0.1 -> v30.0.0
    wgpu = "=30"       cargo metadata  -> resolves against 30.0.1, no change
    wgpu = "=30"       cargo update -p wgpu --precise 30.0.0  -> exit 0
    wgpu = "=30.0.1"   cargo update -p wgpu --precise 30.0.0  -> exit 101,
                       "failed to select a version for the requirement"

A spec that both resolves 30.0.1 and permits 30.0.0 is `=30.*`, and two
versions is a range. So an exact pin is `=` followed by a major, a minor and a
patch, and `=30` and `=30.0` are refused with the rest.

**The wasm-bindgen pin.** Added by F-002 and not in section 15.2's list, which
predates the build pipeline story. `wasm-pack` runs a wasm-bindgen CLI whose
version must match the crate version, so a range lets the two drift and the
mismatch reads as a build break rather than as a resolution change.

**How the entry is read, and why it is not a regex.** Cargo accepts two forms,
`wgpu = "=30.0.1"` and `wgpu = { version = "=30.0.1", features = [...] }`.
The first version of this script took the FIRST quoted string in the entry as
the version, which is positional rather than structural, and the S03 sprint
review measured what that costs:

    wgpu = { default-features = false, features = ["=noop"], version = "30" }

printed `OK: wgpu pinned exactly`. The version read was `=noop`, which starts
with `=`, and the real version was a caret range nobody looked at. A hard rule
in CLAUDE.md, "wgpu is pinned exactly", was defeated by the ORDER of the keys.
So the file is parsed with `tomllib` and the version is taken from the
`version` key. A table that declares no version at all is refused rather than
guessed at.

**The published package licences.** The workspace declares `MIT OR
Apache-2.0`, so the generated wasm package must carry both grants. The crate
uses relative symlinks to the repository originals, and this check compares
the generated package bytes with those originals. A copied or stale legal text
is refused rather than treated as equivalent.

**The size budget.** Story E1.2 is "wasm-pack build pipeline with a hard size
budget gate", and Appendix A gate A4 asks whether binary size and cold start
land within budget at all, estimating 3 to 8 MB uncompressed before tuning with
Naga dominating. Unmeasured today, by the HLD's own admission.

So the budget starts as a RECORDED MEASUREMENT rather than a guess. The first
run writes the observed size to `ci/wasm-size-budget.json` and passes. After
that a regression beyond the tolerance fails. A budget invented before the
first measurement would be either meaningless or immediately wrong.

Usage:
  python3 scripts/pin_and_size_check.py                 # pin only
  python3 scripts/pin_and_size_check.py --with-size     # pin + size
  python3 scripts/pin_and_size_check.py --accept-size   # re-baseline
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
BUDGET = ROOT / "ci" / "wasm-size-budget.json"
PKG = ROOT / "crates" / "ocelli-wasm" / "pkg"

PACKAGE_LICENCES = ("LICENSE-MIT", "LICENSE-APACHE")

# Crates whose version must be an EXACT `=` pin, with the reason a range is
# refused. The reason is printed on failure, because "pin it exactly" without
# the reason is the kind of rule that gets relaxed by the next person in a
# hurry.
EXACT_PINNED = {
    "wgpu":
        "HLD section 15.2 requires `=`. Agents reliably emit wgpu 0.19-era "
        "pipeline code and a range lets that compile against something "
        "subtly different from what the shader expects.",
    # F-002 (E1.2). Not in HLD section 15.2's list, which predates the build
    # pipeline story.
    "wasm-bindgen":
        "`wasm-pack` runs a wasm-bindgen CLI whose version must match the "
        "crate version. A range lets the two drift, and the resulting "
        "version-mismatch reads as a build break rather than as a resolution "
        "change.",
}

# Growth tolerated before the gate fails, as a fraction of the baseline.
# A binary that grows 5% in one story is a story that should say why.
TOLERANCE = 0.05

# An exact pin: `=`, optional whitespace Cargo allows after the operator, then
# all three of major, minor and patch, then the optional pre-release and build
# metadata semver permits. One version and no band. `=30` and `=30.0` are
# comparators over a whole minor or patch band and are refused here.
EXACT_PIN = re.compile(
    r"^=\s*\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")


def declared_version(entry: object) -> str | None:
    """The version a `[workspace.dependencies]` entry declares.

    A bare string IS the version. A table declares it under the `version`
    key, and the rest of the table is features, a path or a
    `default-features` flag, any of which may hold a string that starts with
    `=`. `None` means the entry declares no version this check can read, and
    that is refused rather than assumed to be exact.
    """
    if isinstance(entry, str):
        return entry
    if isinstance(entry, dict):
        version = entry.get("version")
        return version if isinstance(version, str) else None
    return None


def check_pins() -> list[str]:
    try:
        cargo = tomllib.loads(CARGO.read_text())
    except tomllib.TOMLDecodeError as error:
        return [f"Cargo.toml does not parse as TOML: {error}"]
    declared = cargo.get("workspace", {}).get("dependencies")
    if not isinstance(declared, dict):
        return ["Cargo.toml has no [workspace.dependencies] section"]

    problems = []
    for crate, reason in sorted(EXACT_PINNED.items()):
        if crate not in declared:
            problems.append(
                f"{crate} is not declared in [workspace.dependencies], "
                f"and it must be, pinned exactly. {reason}")
            continue
        spec = declared_version(declared[crate])
        if spec is None:
            problems.append(
                f"{crate}: cannot read a version from "
                f"{declared[crate]!r}. An entry with no `version` key is "
                f"not an exact pin, whatever else the table holds. {reason}")
            continue
        if not spec.strip().startswith("="):
            problems.append(
                f"{crate} = \"{spec}\" is a RANGE, not an exact pin. {reason}")
            continue
        if "," in spec:
            problems.append(
                f"{crate} = \"{spec}\" declares MORE THAN ONE comparator, "
                f"which is a range whatever the first one says. An exact pin "
                f"is one `=` and one full major.minor.patch. {reason}")
            continue
        if not EXACT_PIN.match(spec.strip()):
            problems.append(
                f"{crate} = \"{spec}\" pins with `=` and a PARTIAL version, "
                f"which Cargo reads as a whole band. Measured on wgpu: `=30` "
                f"resolves against 30.0.1, and under it "
                f"`cargo update -p wgpu --precise 30.0.0` exits 0 where "
                f"`=30.0.1` refuses that version. Two versions is a range. "
                f"Write `=` and a full major.minor.patch. {reason}")
    return problems


def wasm_bytes() -> int | None:
    if not PKG.is_dir():
        return None
    modules = sorted(PKG.glob("*.wasm"))
    if not modules:
        return None
    return max(m.stat().st_size for m in modules)


def check_package_licences(pkg: Path = PKG,
                           source_root: Path = ROOT) -> list[str]:
    """Require both generated grants to match the repository originals."""
    problems = []
    for name in PACKAGE_LICENCES:
        source = source_root / name
        packaged = pkg / name
        if not source.is_file():
            problems.append(
                f"repository licence {name} is absent, so the generated "
                f"package cannot be checked against its legal source")
            continue
        if packaged.is_symlink():
            problems.append(
                f"generated package licence {name} is a symlink, not a "
                f"regular file under {pkg}. A published package must contain "
                f"both grants rather than links to bytes outside it.")
            continue
        if not packaged.is_file():
            problems.append(
                f"generated package licence {name} is absent under "
                f"{pkg}. The package declares MIT OR Apache-2.0 and must "
                f"ship both grants.")
            continue
        if packaged.read_bytes() != source.read_bytes():
            problems.append(
                f"generated package licence {name} is not byte-identical to "
                f"the repository original. Rebuild from the package-local "
                f"relative symlink rather than copying legal text.")
    return problems


def check_size(accept: bool) -> list[str]:
    size = wasm_bytes()
    if size is None:
        return [f"no .wasm found under {PKG.relative_to(ROOT)}. "
                f"Run `bin/ocelli.sh wasm` first."]

    if not BUDGET.exists() or accept:
        BUDGET.parent.mkdir(parents=True, exist_ok=True)
        BUDGET.write_text(json.dumps({
            "bytes": size,
            "tolerance": TOLERANCE,
            "note": "Recorded measurement, not a guess. HLD Appendix A gate "
                    "A4 estimates 3-8 MB uncompressed before tuning, with "
                    "Naga dominating, and says it is unmeasured. Re-baseline "
                    "deliberately with --accept-size and say why in the "
                    "design plan.",
        }, indent=2) + "\n")
        verb = "re-baselined" if accept else "baselined"
        print(f"  wasm size {verb} at {size:,} bytes "
              f"({size / 1_048_576:.2f} MiB)")
        return []

    baseline = json.loads(BUDGET.read_text())["bytes"]
    ceiling = int(baseline * (1 + TOLERANCE))
    print(f"  wasm size {size:,} bytes, baseline {baseline:,}, "
          f"ceiling {ceiling:,}")
    if size > ceiling:
        return [f"wasm module is {size:,} bytes, over the "
                f"{ceiling:,} byte ceiling ({TOLERANCE:.0%} above the "
                f"{baseline:,} byte baseline). Either reduce it or "
                f"re-baseline with --accept-size and record why."]
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--with-size", action="store_true")
    parser.add_argument("--accept-size", action="store_true")
    args = parser.parse_args(argv)

    problems = check_pins()
    if args.with_size or args.accept_size:
        problems += check_size(args.accept_size)
        problems += check_package_licences()

    if problems:
        print("FAIL: pin or size gate")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {', '.join(sorted(EXACT_PINNED))} pinned exactly" +
          (", wasm size within budget, package licences match"
           if args.with_size else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
