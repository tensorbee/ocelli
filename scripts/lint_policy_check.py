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

## The hole the S03 sprint review found, and what was measured

The crate-allow pass matched by LINT NAME only, and it read `src/lib.rs` and
nothing else. Both halves were bypasses and both are closed here.

**A group allow switches the whole table off in one line.** Every lint in
27.1's table belongs to a clippy group, so `#![allow(clippy::pedantic)]`
names none of the five and disables four of them. Measured on a minimal crate
carrying `cast_possible_truncation = "deny"` and one `x as i32`, under the
pinned 1.97.1 toolchain:

    no attribute                        cargo clippy exits 101
    #![allow(clippy::pedantic)]         cargo clippy exits 0
    #![allow(clippy::restriction)]      exits 0 for indexing_slicing
    #![allow(warnings)]                 exits 101
    #![allow(clippy::all)]              exits 101

So `pedantic` and `restriction` are the two that reach this table TODAY, and
that is a fact about clippy 1.97.1 rather than a law: `float_cmp` sat in
`correctness` until clippy 1.76 and was moved. The other seven names are
refused on 27.1's own note instead, that every conversion should be a
deliberate visible choice, which a crate-wide blanket allow is not. Refusing
them costs nothing, because a crate that needs one of those groups off can say
so at the item that needs it.

**An inner attribute is not confined to a crate root.** `#![allow(...)]` at the
top of a module file applies to that module and everything under it, so
`crates/x/src/inner.rs` carrying one silences the lint for that module while
`src/lib.rs` stays clean. Measured the same way: with `src/lib.rs` carrying
nothing but `pub mod inner;` and the attribute in `src/inner.rs`, clippy goes
from 101 to 0. `src/main.rs` is a second crate ROOT and was never read at all.
So every `.rs` file under each crate is walked.

The `#[allow(...)]` outer form on one item is deliberately NOT refused. That is
the visible, local choice 27.1's note asks for. The gap between the two is one
character, which is why the refusal names the whole attribute it found.

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

# HLD 27.1, transcribed. The table is the specification. There is a second copy
# in Cargo.toml's [workspace.lints.clippy], carrying the same five rows at the
# same levels, and comparing the two is this check's whole job. A rule with one
# copy has nothing to be compared against.
REQUIRED_CLIPPY = {
    "cast_possible_truncation": "deny",
    "cast_precision_loss": "deny",
    "cast_sign_loss": "deny",
    "float_cmp": "deny",
    "indexing_slicing": "warn",
}
REQUIRED_RUST = {"unsafe_code": "deny"}

# Group and blanket names no crate may allow. NAMES ONLY, so the declared
# constant ratchet in scripts/guards/catalogue.py records the set that decides
# how strict this is and not a sentence somebody may reword. Narrowing this
# list is the widening a probe cannot see, because a probe can only ever write
# one of these names and the guard stays correct about its new, smaller rule.
REFUSED_GROUPS = {
    "clippy::pedantic",
    "clippy::restriction",
    "clippy::all",
    "clippy::correctness",
    "clippy::style",
    "clippy::complexity",
    "clippy::perf",
    "clippy::suspicious",
    "warnings",
}

# What each group reaches, for the refusal message. Measured under the pinned
# 1.97.1 toolchain, and the header records the runs. Only two of the nine
# reach 27.1's table TODAY, and that is a fact about clippy 1.97.1 rather than
# a law, which is why the other seven are refused as blanket allows and are
# not claimed to disable anything.
GROUP_REACHES = {
    "clippy::pedantic": "cast_possible_truncation, cast_precision_loss, "
                        "cast_sign_loss and float_cmp, measured",
    "clippy::restriction": "indexing_slicing, measured",
}
BLANKET = ("no lint in HLD 27.1's table under clippy 1.97.1, and it is a "
           "blanket allow over a whole crate or module, which 27.1's note "
           "rules out on its own")

# An INNER attribute, `#![...]`, and every `allow(...)` inside it including one
# reached through `cfg_attr`. The outer `#[allow(...)]` form on a single item is
# not matched, deliberately, because that is the visible local choice 27.1
# permits. Names are then compared whole rather than by substring: the previous
# `\b(clippy::)?{lint}\b` search could only ever find a name it was already
# looking for, which is how a group allow naming none of the five passed.
#
# `expect` is matched alongside `allow`, and leaving it out was a live bypass
# in the first version of this fix. It is the RFC 2383 form, stable since Rust
# 1.81, and it silences a lint exactly as `allow` does while additionally
# warning if the lint never fires. Measured under the pinned 1.97.1 toolchain
# on a crate carrying `cast_possible_truncation = "deny"` and one `x as i32`:
# no attribute exits 101, `#![expect(clippy::cast_possible_truncation)]` exits
# 0, and `#![expect(clippy::pedantic)]` exits 0. Both the named and the group
# route were open. Note that `expect` reaches `pedantic` where `allow` does
# not reach `warnings`, so the two attributes are not even the same shape of
# hole.
INNER_ALLOW = re.compile(
    r"#!\[[^\]]*?\b(?:allow|expect)\(([^)]*)\)[^\]]*\]")

# A stricter level satisfies a weaker requirement and not the other way round.
STRENGTH = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}


def crate_sources(crate: Path) -> list[Path]:
    """Every `.rs` file in a crate, sorted, with build output skipped.

    Not `src/lib.rs`. An inner attribute in `src/main.rs` is a second crate
    root and one in any module file governs that module, both measured, so a
    pass that reads one file answers a question about one file and says so by
    succeeding.
    """
    return sorted(path for path in crate.rglob("*.rs")
                  if "target" not in path.relative_to(crate).parts)


def allowed_lints(source: str) -> list[tuple[str, str]]:
    """Every lint name allowed by an inner attribute, with the attribute.

    Returns (name, attribute text) so a refusal can quote what it found. The
    name keeps its tool prefix, because `clippy::pedantic` and a bare
    `pedantic` are different things to clippy and only one of them is a group.
    """
    found: list[tuple[str, str]] = []
    for match in INNER_ALLOW.finditer(source):
        attribute = " ".join(match.group(0).split())
        for name in match.group(1).split(","):
            name = name.strip()
            if name:
                found.append((name, attribute))
    return found


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

    # An inner `allow` or `expect` puts a denied lint back to sleep for a
    # whole crate or a whole module. By name, and by any group that
    # contains one.
    named = set(REQUIRED_CLIPPY) | set(REQUIRED_RUST)
    scanned = 0
    for crate in crates:
        for path in crate_sources(crate):
            scanned += 1
            where = path.relative_to(ROOT).as_posix()
            for name, attribute in allowed_lints(
                    path.read_text(encoding="utf-8")):
                bare = name.removeprefix("clippy::")
                if bare in named:
                    problems.append(
                        f"{where} re-allows `{bare}` with `{attribute}`. HLD "
                        f"27.1 denies it, and an inner attribute is not the "
                        f"deliberate, visible choice 27.1's note asks for: at "
                        f"a crate root it covers the crate and in a module "
                        f"file it covers that module. Allow it at the "
                        f"expression that needs it, with a reason.")
                    continue
                if name in REFUSED_GROUPS:
                    reaches = GROUP_REACHES.get(name)
                    why = (
                        f"which reaches {reaches}. A group allow names none "
                        f"of HLD 27.1's five lints and switches them off "
                        f"anyway, so this check and the `clippy` gate would "
                        f"both stay green over a smaller set of rules. That "
                        f"is the arithmetic defect class CLAUDE.md names as "
                        f"the one that reaches patients."
                        if reaches else
                        f"which reaches {BLANKET}. It is refused here rather "
                        f"than measured, because which lint sits in which "
                        f"group is a clippy release detail and not a law: "
                        f"float_cmp sat in `correctness` until clippy 1.76.")
                    problems.append(
                        f"{where} allows the lint group `{name}` with "
                        f"`{attribute}`, {why} Allow the single lint at the "
                        f"expression that needs it, with a reason.")

    # A scan that read nothing is not a scan that found nothing. Measured
    # while proving the `expect` route above: a tree whose crates carry no
    # `.rs` file at all printed `0 .rs file(s) carry no inner allow` and
    # exited 0, which is AGENTS.md's named failure of answering a question
    # about an empty set in the language of success.
    if crates and not scanned:
        problems.append(
            f"{len(crates)} crate(s) inherit the lint table and not one "
            f"`.rs` file was read, so the inner-attribute pass proved "
            f"nothing and would have said OK. Either the crate layout moved "
            f"or `crate_sources` stopped finding sources.")

    if problems:
        print("FAIL: the HLD 27.1 lint policy")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {len(REQUIRED_CLIPPY)} clippy lint(s) at or above HLD 27.1's "
          f"level, {len(crates)} crate(s) inherit the table, {scanned} "
          f".rs file(s) carry no inner allow or expect of a denied lint or "
          f"of a group "
          f"holding one, unsafe_code denied by {unsafe_by}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
