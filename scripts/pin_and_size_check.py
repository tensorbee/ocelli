#!/usr/bin/env python3
"""The wgpu exact pin and the wasm size budget. HLD section 27.2 R4 and E1.2.

Two checks that share a file because they share a failure mode: both are
about a number nobody looks at until it is wrong.

**The wgpu pin.** HLD section 15.2 pins `wgpu = "=30.0.1"` and says why:

    "The exact wgpu pin is not fussiness. Agents reliably emit wgpu 0.19-era
     pipeline code; a caret range lets that compile against something subtly
     different from what the shader expects."

R4 adds: "treat GPU code that compiles first try with suspicion". A caret or
tilde range on wgpu re-opens exactly the gap the pin closes, so the range form
is refused, not just a wrong version.

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
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
BUDGET = ROOT / "ci" / "wasm-size-budget.json"
PKG = ROOT / "crates" / "ocelli-wasm" / "pkg"

# HLD section 15.2. Crates whose version must be an EXACT `=` pin.
EXACT_PINNED = {"wgpu"}

# Growth tolerated before the gate fails, as a fraction of the baseline.
# A binary that grows 5% in one story is a story that should say why.
TOLERANCE = 0.05


def check_pins() -> list[str]:
    text = CARGO.read_text()
    block = re.search(r"^\[workspace\.dependencies\]$(.*?)(?=^\[|\Z)",
                      text, re.M | re.S)
    if block is None:
        return ["Cargo.toml has no [workspace.dependencies] section"]

    problems = []
    for crate in sorted(EXACT_PINNED):
        entry = re.search(rf'^\s*{crate}\s*=\s*(.+)$', block.group(1), re.M)
        if entry is None:
            problems.append(
                f"{crate} is not declared in [workspace.dependencies]. "
                f"HLD section 15.2 requires it, pinned exactly.")
            continue
        value = entry.group(1)
        version = re.search(r'"([^"]+)"', value)
        if version is None:
            problems.append(f"{crate}: cannot read a version from {value!r}")
            continue
        spec = version.group(1)
        if not spec.startswith("="):
            problems.append(
                f"{crate} = \"{spec}\" is a RANGE, not an exact pin. "
                f"HLD section 15.2 requires `=`. Agents reliably emit "
                f"wgpu 0.19-era pipeline code and a range lets that compile "
                f"against something subtly different from what the shader "
                f"expects.")
    return problems


def wasm_bytes() -> int | None:
    if not PKG.is_dir():
        return None
    modules = sorted(PKG.glob("*.wasm"))
    if not modules:
        return None
    return max(m.stat().st_size for m in modules)


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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--with-size", action="store_true")
    parser.add_argument("--accept-size", action="store_true")
    args = parser.parse_args()

    problems = check_pins()
    if args.with_size or args.accept_size:
        problems += check_size(args.accept_size)

    if problems:
        print("FAIL: pin or size gate")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print("OK: wgpu pinned exactly" +
          (", wasm size within budget" if args.with_size else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
