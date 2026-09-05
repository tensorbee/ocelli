#!/usr/bin/env python3
"""HLD section 27.1's denied lints are still denied. F-X009.

Section 27.1 gives the table verbatim:

    [workspace.lints.rust]
    unsafe_code = "deny"   # allow-listed per file, see 27.2

    [workspace.lints.clippy]
    cast_possible_truncation = "deny"
    cast_precision_loss = "deny"
    cast_sign_loss = "deny"
    float_cmp = "deny"
    indexing_slicing = "warn"

and its note: "The dangerous defect class here is arithmetic, and `as` casts
are where silent arithmetic errors live. Every conversion should be a
deliberate, visible choice."

## The hole this closes

The `clippy` gate runs `-D warnings`, which turns whatever is enabled into an
error. **Nothing asserted that the right things were enabled.** A lint moved
from `deny` to `allow` in `[workspace.lints]`, or a crate that stops carrying
`lints.workspace = true`, is silent: the gate still passes, and it passes over
a smaller set of rules. That is the widening class the guard census exists to
catch, and this is the specific case of it that costs the most.

Found by taking the census for F-X009, and it is the same shape as the two
guards the S01 runbook exercise produced.

## The one departure, declared rather than discovered

`unsafe_code = "deny"` is NOT in `Cargo.toml`. HLD 27.2 R5 is enforced instead
by `scripts/unsafe_allowlist_check.py` over the whole tree, in the `unsafe`
gate and in the CI floor. That substitution is stronger than the lint for the
thing R5 actually asks for, because a per-file `#![allow(unsafe_code)]` would
silence the lint and would not silence the script. This check requires ONE of
the two and names which one it found, so removing the script without adding the
lint is refused.

Usage: python3 scripts/lint_policy_check.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
CRATES = ROOT / "crates"
RUNNER = ROOT / "bin" / "ocelli.sh"

# HLD 27.1, transcribed. The table is the specification and this is the only
# copy of it in the repository outside `docs/hld/`.
REQUIRED_CLIPPY = {
    "cast_possible_truncation": "deny",
    "cast_precision_loss": "deny",
    "cast_sign_loss": "deny",
    "float_cmp": "deny",
    "indexing_slicing": "warn",
}
REQUIRED_RUST = {"unsafe_code": "deny"}

# A stricter level satisfies a weaker requirement and not the other way round.
STRENGTH = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}


def table(text: str, name: str) -> dict[str, str]:
    block = re.search(rf"^\[{re.escape(name)}\]$(.*?)(?=^\[|\Z)", text,
                      re.M | re.S)
    if block is None:
        return {}
    found = {}
    for line in block.group(1).splitlines():
        match = re.match(r'^\s*([a-z_:]+)\s*=\s*"([a-z]+)"', line)
        if match:
            found[match.group(1)] = match.group(2)
    return found


def main() -> int:
    text = CARGO.read_text(encoding="utf-8")
    clippy = table(text, "workspace.lints.clippy")
    rust = table(text, "workspace.lints.rust")

    problems: list[str] = []

    for lint, wanted in sorted(REQUIRED_CLIPPY.items()):
        level = clippy.get(lint)
        if level is None:
            problems.append(
                f"clippy lint `{lint}` is in HLD 27.1's table and is not in "
                f"[workspace.lints.clippy]. The `clippy` gate runs "
                f"-D warnings over whatever is enabled, so a lint that is not "
                f"enabled costs nothing and catches nothing.")
            continue
        if STRENGTH.get(level, 0) < STRENGTH[wanted]:
            problems.append(
                f"clippy lint `{lint}` is '{level}' and HLD 27.1 requires "
                f"'{wanted}'. The dangerous defect class here is arithmetic, "
                f"and `as` casts are where silent arithmetic errors live. "
                f"Weakening one is a design-plan decision with a recorded "
                f"rationale, not an edit made to get a build green.")

    # The declared departure. One of the two mechanisms must be present.
    lint_level = rust.get("unsafe_code")
    script = (ROOT / "scripts" / "unsafe_allowlist_check.py").is_file()
    gate = 'unsafe|no|' in RUNNER.read_text(encoding="utf-8")
    if lint_level is not None and STRENGTH.get(lint_level, 0) >= 2:
        unsafe_by = f"[workspace.lints.rust] unsafe_code = \"{lint_level}\""
    elif script and gate:
        unsafe_by = ("scripts/unsafe_allowlist_check.py in the `unsafe` gate, "
                     "which is the declared substitution")
    else:
        unsafe_by = ""
        problems.append(
            "HLD 27.1 denies `unsafe_code` and neither mechanism is present. "
            "Either declare it in [workspace.lints.rust], or keep "
            "scripts/unsafe_allowlist_check.py in the `unsafe` gate. One of "
            "the two, and removing the script without adding the lint leaves "
            "HLD 27.2 R5 enforced by nothing.")

    # Every crate inherits the table. A crate that stops is the second way to
    # the same place, and no lint level notices it.
    crates = sorted(c for c in CRATES.iterdir() if (c / "Cargo.toml").is_file())
    if not crates:
        problems.append(
            "no crate under crates/ carries a manifest, so this check has "
            "nothing to inspect and would say so by succeeding.")
    for crate in crates:
        manifest = (crate / "Cargo.toml").read_text(encoding="utf-8")
        if not re.search(r"^\[lints\]\s*$\s*^workspace\s*=\s*true\s*$",
                         manifest, re.M):
            problems.append(
                f"{crate.name} does not inherit the workspace lint table. "
                f"`[lints]` with `workspace = true` is what makes HLD 27.1 "
                f"apply to it, and a crate without it compiles under a "
                f"smaller set of rules while the `clippy` gate stays green.")

    # A crate-level allow puts a denied lint back to sleep for a whole crate.
    for crate in crates:
        lib = crate / "src" / "lib.rs"
        if not lib.is_file():
            continue
        source = lib.read_text(encoding="utf-8")
        for lint in sorted(REQUIRED_CLIPPY) + sorted(REQUIRED_RUST):
            if re.search(rf"#!\[allow\([^)]*\b(clippy::)?{lint}\b", source):
                problems.append(
                    f"{crate.name}/src/lib.rs re-allows `{lint}` for the "
                    f"whole crate. HLD 27.1 denies it, and a crate-wide allow "
                    f"is not the deliberate, visible choice 27.1's note asks "
                    f"for. Allow it at the expression that needs it, with a "
                    f"reason.")

    if problems:
        print("FAIL: the HLD 27.1 lint policy")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {len(REQUIRED_CLIPPY)} clippy lint(s) at or above HLD 27.1's "
          f"level, {len(crates)} crate(s) inherit the table, unsafe_code "
          f"denied by {unsafe_by}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
