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

## Four more holes, all measured in the S03 review's fifth pass

**The workspace has fourteen members and this read thirteen.** `crates/` was
hard-coded here and `Cargo.toml` says `members = ["crates/*", "tools/oracle"]`.
`tools/oracle` is a compiled member with thirteen `.rs` files, checked for
neither `[lints] workspace = true` nor an inner allow. Measured: removing
`[lints] workspace = true` from `tools/oracle/Cargo.toml` AND prepending
`#![allow(clippy::pedantic)]` to `tools/oracle/src/lib.rs` left this check at
exit 0 printing "13 crate(s) inherit the table, 33 .rs file(s)", and left the
census at exit 0. The members are read from the manifest now, globbed, and a
pattern resolving to nothing is refused rather than walked past. The count in
the OK line is derived from that walk rather than written as a literal.

**Whitespace in the lint path defeated `REFUSED_GROUPS`.** Rust tokenises
`clippy :: pedantic` exactly as `clippy::pedantic`. Measured under the pinned
1.97.1 toolchain on a crate carrying `cast_possible_truncation = "deny"` and
one `x as i32`, with the attribute in a module file: no attribute exits 101,
`#![allow(clippy :: pedantic)]` exits 0, `#![allow(clippy:: pedantic)]` exits 0,
and `#![expect(clippy :: cast_possible_truncation)]` exits 0. The captured name
had spaces in it and matched no entry in the set. Names are normalised now, and
a `reason = "..."` clause is stripped before the split so it cannot be read as
a lint name.

**The outer form is not always local.** `#[allow(...)]` on a `mod` item governs
the whole module tree, which is the same scope an inner attribute in that
module's file has. Measured the same way: with `src/inner.rs` carrying one
`x as i32` and `src/lib.rs` reading
`#[allow(clippy::cast_possible_truncation)] pub mod inner;`, cargo clippy goes
from 101 to 0, and `#[allow(clippy::pedantic)] pub mod inner;` does the same.
So the guard refused an inner attribute at a crate root because it covers the
crate, and permitted an outer one on a module, which covers a module. Same
scope, opposite verdicts, and the accept probe planted its outer allow on a
`fn`, so the `mod` case was never exercised. An outer `allow` or `expect` whose
next non-trivial token is `mod` is refused now. On a `fn`, an `impl`, a
`struct`, a statement or an expression it is still permitted, and both
directions are probed.

**A group row in the workspace table itself.** The row regex matched a quoted
level only, so `pedantic = { level = "allow", priority = 1 }` in
`[workspace.lints.clippy]` was invisible and this check went on printing
"5 clippy lint(s) at or above HLD 27.1's level" at exit 0. Measured: that one
row beside `cast_possible_truncation = "deny"` takes cargo clippy from 101 to
0, because the higher priority is applied last and the group wins. The census's
digest over the table caught it, so the gate held, but this check was wrong
about all five. The inline form is parsed now and a group row weaker than
`deny` is refused.

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

# The OUTER form, `#[allow(...)]`, which is refused only when it governs a
# `mod`. On a `fn`, an `impl`, a `struct`, a statement or an expression it is
# the deliberate, visible, LOCAL choice 27.1's note asks for and it stays
# permitted. On a module item it governs the whole module tree, which is the
# scope an inner attribute in that module's file has, and the guard was
# refusing one and permitting the other for opposite stated reasons.
OUTER_ALLOW = re.compile(
    r"#\[[^\]]*?\b(?:allow|expect)\(([^)]*)\)[^\]]*\]")

# Whitespace, line comments, block comments and further outer attributes, all
# of which may sit between an attribute and the item it is attached to. An
# attribute followed by `#[cfg(test)]` and then `mod tests` still governs the
# module, so another attribute is trivia here rather than a terminator.
TRIVIA = re.compile(r"(?:\s+|//[^\n]*|/\*.*?\*/|#!?\[[^\]]*\])*", re.S)

# A module item, with the visibility and `unsafe` qualifiers Rust allows in
# front of it. `mod x;` and `mod x { ... }` are the same scope for this rule.
MODULE_ITEM = re.compile(r"(?:pub\s*(?:\([^)]*\)\s*)?)?(?:unsafe\s+)?mod\b")

# RFC 2383's `reason = "..."` clause, removed before the argument list is split
# on commas. Left in, it became a lint name of its own and, worse, a reason
# carrying a comma split into two names neither of which is one.
REASON = re.compile(
    r"""\breason\s*=\s*(?:"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*')""")

# One row of a `[workspace.lints.*]` table, in both TOML forms: the quoted
# level and the inline table. The inline form was invisible, and
# `pedantic = { level = "allow", priority = 1 }` is measured to take cargo
# clippy from 101 to 0 on a crate denying cast_possible_truncation.
#
# It matches the row BODY and not the whole line, because the anchor was `\s*$`
# and a TOML trailing comment is not whitespace. `_row_body` below removes the
# comment first. THE CLASS OF INPUT THAT IS NOW CLOSED: any row of either TOML
# form carrying a trailing `#` comment, at any level, in either
# `[workspace.lints.clippy]` or `[workspace.lints.rust]`. On a REQUIRED row the
# old anchor failed safe, reporting the row missing. On a GROUP row it failed
# OPEN, which is the whole table switched off in one line that this check read
# as absent. Measured under the pinned 1.97.1 toolchain on a minimal crate
# carrying `cast_possible_truncation = "deny"` and one `x as i32`: cargo clippy
# exits 101, and with
# `pedantic = { level = "allow", priority = 1 } # keeps noise down` appended it
# exits 0. What is NOT closed is a row spread over two lines: TOML 1.0 puts an
# inline table on one line, and a `pedantic = { level = "allow",` / `priority =
# 1 }` pair is invisible to this regex, MEASURED at exit 0. The declared
# constant `Cargo.toml:workspace.lints` is the backstop for that and it was
# measured too: the same pair moves the digest from adf2cb2237be28da to
# e3d02e83d8b52dab and `guard_census.py` refuses. That is the division of
# labour between the two mechanisms and it is why both had to be fixed. What
# `table()` adds on its own is that an inline form it CAN see and whose level
# it cannot read yields the empty string, which is weaker than every level in
# `STRENGTH` and is refused rather than skipped.
LINT_ROW = re.compile(
    r'^\s*([A-Za-z_][\w:-]*)\s*=\s*(?:"([a-z]+)"|\{([^}]*)\})\s*$')

# A TOML trailing comment. `#` opens one only outside a string, so the scan
# tracks the quote it is inside rather than splitting on the first `#`: a level
# is never a string carrying one today, and `reason = "see #123"` in an inline
# table is legal TOML and would otherwise truncate the row into something this
# parser reads as unparseable.
def _row_body(line: str) -> str:
    """`line` with any trailing `#` comment removed, quotes respected."""
    quote = ""
    index = 0
    while index < len(line):
        char = line[index]
        if quote:
            # A backslash escapes the next character inside a TOML basic
            # string, so the pair is stepped over together. Skipping only the
            # backslash would leave an escaped quote closing the string.
            if char == "\\" and quote == '"':
                index += 2
                continue
            if char == quote:
                quote = ""
        elif char in "\"'":
            quote = char
        elif char == "#":
            return line[:index]
        index += 1
    return line

# A stricter level satisfies a weaker requirement and not the other way round.
STRENGTH = {"allow": 0, "warn": 1, "deny": 2, "forbid": 3}


def member_patterns(text: str) -> tuple[list[str], list[str]]:
    """The globs `[workspace] members` declares, and what `exclude` removes."""
    block = re.search(r"^\[workspace\]$(.*?)(?=^\[|\Z)", text, re.M | re.S)
    if block is None:
        return [], []

    def listing(key: str) -> list[str]:
        found = re.search(rf"^\s*{key}\s*=\s*\[(.*?)\]", block.group(1),
                          re.M | re.S)
        return re.findall(r'"([^"]+)"', found.group(1)) if found else []

    return listing("members"), listing("exclude")


def workspace_members(text: str) -> tuple[list[Path], list[str]]:
    """Every directory `[workspace] members` resolves to, and the refusals.

    Read from the manifest rather than from a hard-coded `crates/`. The
    workspace has fourteen members and `crates/*` is thirteen of them.
    `tools/oracle` is the fourteenth, a compiled member with thirteen `.rs`
    files that was checked for neither `[lints] workspace = true` nor an inner
    allow, and both routes were measured to silence a denied lint.

    A pattern that resolves to no directory carrying a manifest is REFUSED
    rather than skipped. cargo would refuse that workspace too, and a member
    this function cannot find is a member whose sources are not scanned, which
    the walk would otherwise report as a smaller number and a pass.

    `exclude` applies to the GLOB patterns only, because that is what cargo
    does. Measured under the pinned 1.97.1 toolchain on this workspace, with
    `members = ["crates/*", "tools/oracle"]` and `exclude = ["tools/oracle"]`
    added: `cargo metadata --no-deps` reports 14 packages with `ocelli-oracle`
    among them, and clippy compiles it. Applying `exclude` to the explicit
    entry too dropped it here and printed "13 workspace member(s) ... 33 .rs
    file(s)", which is the same pair of numbers the header records as the fifth
    pass's defect, reached through a different key. An explicitly named member
    is named, and a list that removes what another list names is a decision
    cargo does not make on this shape.
    """
    patterns, excluded = member_patterns(text)
    problems: list[str] = []
    if not patterns:
        problems.append(
            "Cargo.toml's [workspace] declares no `members` this parser can "
            "read, so there is no member to inspect and every check below "
            "would answer a question about an empty set in the language of "
            "success.")
        return [], problems
    skip = {(ROOT / name).resolve() for name in excluded}
    members: list[Path] = []
    for pattern in patterns:
        if "*" in pattern:
            hits = sorted(p for p in ROOT.glob(pattern)
                          if (p / "Cargo.toml").is_file())
            # Subtracted AFTER the empty test below, so a glob whose every hit
            # is excluded is reported as an exclusion rather than as a pattern
            # that resolves to nothing, which is a different repair.
            kept = [p for p in hits if p.resolve() not in skip]
        else:
            path = ROOT / pattern
            hits = [path] if (path / "Cargo.toml").is_file() else []
            kept = hits
        if not hits:
            problems.append(
                f"the workspace member `{pattern}` resolves to no directory "
                f"carrying a Cargo.toml. cargo would refuse this workspace, "
                f"and this check would otherwise walk one member fewer, scan "
                f"its sources not at all, and report the smaller number as a "
                f"pass.")
            continue
        members.extend(kept)
    if not members and not problems:
        problems.append(
            f"Cargo.toml's [workspace] declares {len(patterns)} member "
            f"pattern(s) and `exclude` removes every directory they resolve "
            f"to, so there is no member to inspect. An empty walk is not an "
            f"empty finding.")
    return sorted(set(members)), problems


def member_sources(member: Path) -> list[Path]:
    """Every `.rs` file in a workspace member, sorted, build output skipped.

    Not `src/lib.rs`. An inner attribute in `src/main.rs` is a second crate
    root and one in any module file governs that module, both measured, so a
    pass that reads one file answers a question about one file and says so by
    succeeding.

    `<member>/target/` at depth 1 ONLY. The previous rule skipped any path
    component named `target`, and the build directory is at the repository
    root, so all it could ever skip was a source module called `target`. HLD
    section 13 makes that a likely name in a rendering codebase and skipping
    it would be silent.
    """
    return sorted(path for path in member.rglob("*.rs")
                  if path.relative_to(member).parts[:1] != ("target",))


def _names_in(argument_list: str) -> list[str]:
    """The lint names an `allow(...)` or `expect(...)` argument list holds.

    Whitespace inside a name is removed before the comparison, because Rust
    tokenises `clippy :: pedantic` exactly as `clippy::pedantic` and measuring
    it under the pinned toolchain showed the spaced form silencing the lint
    while the set lookup matched nothing. A `reason = "..."` clause is stripped
    first, so it is neither read as a lint name nor split into two by a comma
    inside its own string.
    """
    return [re.sub(r"\s+", "", name)
            for name in REASON.sub("", argument_list).split(",")
            if re.sub(r"\s+", "", name)]


def allowed_lints(source: str) -> list[tuple[str, str]]:
    """Every lint name allowed by an inner attribute, with the attribute.

    Returns (name, attribute text) so a refusal can quote what it found. The
    name keeps its tool prefix, because `clippy::pedantic` and a bare
    `pedantic` are different things to clippy and only one of them is a group.
    """
    found: list[tuple[str, str]] = []
    for match in INNER_ALLOW.finditer(source):
        attribute = " ".join(match.group(0).split())
        for name in _names_in(match.group(1)):
            found.append((name, attribute))
    return found


def module_allows(source: str) -> list[tuple[str, str]]:
    """Every lint name an OUTER attribute allows on a `mod` item.

    The scope is the whole module tree, measured: with one `x as i32` in
    `src/inner.rs` and `src/lib.rs` reading
    `#[allow(clippy::cast_possible_truncation)] pub mod inner;`, cargo clippy
    goes from 101 to 0 under the pinned 1.97.1 toolchain. That is the scope an
    inner attribute in `src/inner.rs` has, and the guard refused one and
    permitted the other.
    """
    found: list[tuple[str, str]] = []
    for match in OUTER_ALLOW.finditer(source):
        rest = source[match.end():]
        trivia = TRIVIA.match(rest)
        tail = rest[trivia.end():] if trivia else rest
        if not MODULE_ITEM.match(tail):
            continue
        attribute = " ".join(match.group(0).split())
        for name in _names_in(match.group(1)):
            found.append((name, attribute))
    return found


def table(text: str, name: str) -> dict[str, str]:
    """One `[workspace.lints.*]` table, in both of TOML's forms.

    The inline form was invisible, so a group row switching the whole table off
    left this check printing "5 clippy lint(s) at or above HLD 27.1's level"
    at exit 0. A row whose inline table carries no readable `level` yields the
    empty string, which is weaker than every level in `STRENGTH` and is
    therefore refused rather than skipped.

    A trailing `#` comment is removed before the row is matched. `LINT_ROW`
    anchors on `\\s*$` and a comment is not whitespace, so the sixth pass
    measured `pedantic = { level = "allow", priority = 1 } # keeps noise down`
    read as no row at all: this check printed "no group row weaker than deny"
    and exited 0 while cargo clippy went from 101 to 0.

    The block runs to the next `[` header and not to the first blank line. A
    blank line does not end a TOML table, so a row after one is still in it.
    """
    block = re.search(rf"^\[{re.escape(name)}\]$(.*?)(?=^\[|\Z)", text,
                      re.M | re.S)
    if block is None:
        return {}
    found = {}
    for line in block.group(1).splitlines():
        match = LINT_ROW.match(_row_body(line))
        if match is None:
            continue
        if match.group(2) is not None:
            found[match.group(1)] = match.group(2)
            continue
        level = re.search(r'level\s*=\s*"([a-z]+)"', match.group(3))
        found[match.group(1)] = level.group(1) if level else ""
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

    # A group row in the workspace table itself. The inner-attribute pass below
    # watches a crate putting a lint back to sleep, and nothing watched the
    # table doing it in one line.
    for table_name, prefix, rows in (
            ("[workspace.lints.clippy]", "clippy::", clippy),
            ("[workspace.lints.rust]", "", rust)):
        for lint, level in sorted(rows.items()):
            if f"{prefix}{lint}" not in REFUSED_GROUPS:
                continue
            if STRENGTH.get(level, 0) >= STRENGTH["deny"]:
                continue
            problems.append(
                f"{table_name} carries the lint GROUP `{lint}` at "
                f"'{level or 'a level this parser cannot read'}'. HLD 27.1's "
                f"table names five lints and no group, and a group row weaker "
                f"than 'deny' switches lints off without naming one of them. "
                f"Measured under the pinned 1.97.1 toolchain: adding "
                f"`pedantic = {{ level = \"allow\", priority = 1 }}` beside "
                f"`cast_possible_truncation = \"deny\"` takes cargo clippy "
                f"from 101 to 0, because the higher priority is applied last "
                f"and the group wins. Allow the single lint at the expression "
                f"that needs it, with a reason.")

    # The declared departure. One of the two mechanisms must be present.
    lint_level = rust.get("unsafe_code")
    script = (ROOT / "scripts" / "unsafe_allowlist_check.py").is_file()
    # The ARM, not the GATES table row. `'unsafe|no|' in RUNNER` matched the
    # row in the gate inventory, which is a description, so replacing the
    # `unsafe)` arm with `true` left this check asserting a substitution it had
    # not established. The `ci` gate catches that separately and this one is no
    # longer wrong about it.
    gate = re.search(
        r"^\s*unsafe\)[^\n]*?python3 scripts/unsafe_allowlist_check\.py",
        RUNNER.read_text(encoding="utf-8"), re.M) is not None
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

    # Every workspace MEMBER inherits the table. A member that stops is the
    # second way to the same place, and no lint level notices it. The members
    # are read from the manifest rather than assumed to be `crates/*`, because
    # they are not: `tools/oracle` is the fourteenth.
    members, member_problems = workspace_members(text)
    problems += member_problems
    for member in members:
        where = member.relative_to(ROOT).as_posix()
        manifest = (member / "Cargo.toml").read_text(encoding="utf-8")
        if not re.search(r"^\[lints\]\s*$\s*^workspace\s*=\s*true\s*$",
                         manifest, re.M):
            problems.append(
                f"{where} does not inherit the workspace lint table. "
                f"`[lints]` with `workspace = true` is what makes HLD 27.1 "
                f"apply to it, and a member without it compiles under a "
                f"smaller set of rules while the `clippy` gate stays green.")

    # An inner `allow` or `expect` puts a denied lint back to sleep for a
    # whole crate or a whole module, and an OUTER one on a `mod` item does the
    # same for that module tree. By name, and by any group that contains one.
    named = set(REQUIRED_CLIPPY) | set(REQUIRED_RUST)
    scanned = 0
    for member in members:
        for path in member_sources(member):
            scanned += 1
            where = path.relative_to(ROOT).as_posix()
            source = path.read_text(encoding="utf-8")
            attributes = ([(n, a, "inner") for n, a in allowed_lints(source)] +
                          [(n, a, "module") for n, a in module_allows(source)])
            for name, attribute, shape in attributes:
                scope = (
                    "an inner attribute is not the deliberate, visible choice "
                    "27.1's note asks for: at a crate root it covers the "
                    "crate and in a module file it covers that module"
                    if shape == "inner" else
                    "an outer attribute on a `mod` item covers the whole "
                    "module tree, measured, which is the same scope as an "
                    "inner attribute inside that module and not the local "
                    "choice 27.1's note asks for")
                bare = name.removeprefix("clippy::")
                if bare in named:
                    problems.append(
                        f"{where} re-allows `{bare}` with `{attribute}`. HLD "
                        f"27.1 denies it, and {scope}. Allow it at the "
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
                        f"`{attribute}`, {why} {scope[0].upper()}{scope[1:]}. "
                        f"Allow the single lint at the expression that needs "
                        f"it, with a reason.")

    # A scan that read nothing is not a scan that found nothing. Measured
    # while proving the `expect` route above: a tree whose crates carry no
    # `.rs` file at all printed `0 .rs file(s) carry no inner allow` and
    # exited 0, which is AGENTS.md's named failure of answering a question
    # about an empty set in the language of success.
    if members and not scanned:
        problems.append(
            f"{len(members)} workspace member(s) inherit the lint table and "
            f"not one `.rs` file was read, so the attribute pass proved "
            f"nothing and would have said OK. Either the layout moved or "
            f"`member_sources` stopped finding sources.")

    if problems:
        print("FAIL: the HLD 27.1 lint policy")
        for problem in problems:
            print(f"  {problem}")
        return 1

    # Every number here is derived from the walk that produced it. The member
    # count was the literal `crates/` glob until the fifth pass, and it read
    # thirteen while cargo built fourteen.
    print(f"OK: {len(REQUIRED_CLIPPY)} clippy lint(s) at or above HLD 27.1's "
          f"level and no group row weaker than deny, {len(members)} workspace "
          f"member(s) resolved from "
          f"{', '.join(repr(p) for p in member_patterns(text)[0])} inherit "
          f"the table, "
          f"{scanned} .rs file(s) carry no inner allow or expect of a denied "
          f"lint or of a group holding one, and none on a `mod` item, "
          f"unsafe_code denied by {unsafe_by}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
