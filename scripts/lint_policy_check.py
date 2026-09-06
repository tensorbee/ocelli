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

## Four more routes, all measured in the S03 review's seventh pass

**A comment inside the lint path.** Rust's lexer removes a comment before it
sees a token, so `clippy::/*x*/pedantic` is the same attribute as
`clippy::pedantic` and the spaced form the fifth pass closed was one member of
a family. Measured under the pinned 1.97.1 toolchain: baseline exit 101,
`#![allow(clippy::/*c*/pedantic)]` exit 0,
`#![allow(clippy::/*c*/cast_possible_truncation)]` exit 0, and the
newline-and-`//` form exit 0. Planted in a sandbox clone of this repository the
whole `guards` gate was green. Comments are stripped before the comma split
now, with the same nesting Rust gives them, and an argument list holding an
UNTERMINATED block comment is refused outright: `#![allow(clippy::/*)*/pedantic)]`
is measured at exit 0 too, and there the `)` the capture stopped at was inside
the comment, so what this file read was a fragment.

**`.cargo/config.toml` was read by nothing.** No file under `scripts/`, `bin/`,
`ci/`, `.githooks/` or `.github/` mentioned `rustflags`. Measured on the minimal
crate: `[build] rustflags = ["-Aclippy::pedantic"]` exit 0,
`["-Aclippy::cast_possible_truncation"]` exit 0, the `[target.'cfg(all())']`
form exit 0 and the bare-string form exit 0. Added to a sandbox clone, where
there is no `.cargo/` today, every check stayed green. `rustflag_problems`
refuses the FLAG rather than the file, and says there why.

**A workspace member reached as a path dependency was never walked.** cargo
makes every path dependency of a member a member too. Measured: adding
`vendor/probe` and `probe-vendored = { path = "../../vendor/probe" }` to
`crates/ocelli-core/Cargo.toml` made `cargo metadata --no-deps` report 15
packages while this check printed "14 workspace member(s)" at exit 0 and the
census exited 0. The new crate carried no `[lints] workspace = true`. That is
the fifth pass's `tools/oracle` defect through a different key, and reading the
globs harder would only move the key again, so the set comes from cargo now and
`cargo_metadata_members` records how.

**A module whose source is outside the member directory.** `#[path]` puts it
anywhere, and `member.rglob("*.rs")` never opens it. Measured: a crate whose
`src/lib.rs` reads `#[path = "../../shared_outside/shared.rs"] pub mod shared;`
exits 101, and with `#![allow(clippy::pedantic)]` at the top of that file it
exits 0 while this check never read the file. `#[path]` is followed now,
transitively, and one it cannot resolve is refused rather than skipped.

## Two more routes, both measured in the S03 review's eighth pass

**The crate ROOT itself can be anywhere, and the seventh pass only followed
`#[path]` from a root it assumed.** A manifest's `[lib] path`, `[[bin]] path`,
`[[test]] path`, `[[bench]] path` and `[[example]] path` each put a compilation
root where the manifest says rather than under `src/`, and the walk started
from `member.rglob("*.rs")`, which finds the file that is no longer compiled
and not the one that is. MEASURED under the pinned 1.97.1 toolchain on a
minimal crate carrying `cast_possible_truncation = "deny"`: with
`[lib] path = "../../outside/lib.rs"` and that file holding
`#![allow(clippy::pedantic)]` and one `x as i32`,
`cargo clippy --workspace --all-targets -- -D warnings` exits 0 against a
baseline of 101. Planted in a full copy of this repository, this check exited 0
printing "46 .rs file(s), `#[path]` modules followed", having scanned the
now-unused `crates/ocelli-core/src/lib.rs` and never opened the real crate
root. That is the fifth pass's `tools/oracle` defect through a third key, and
reading the manifest harder here would only move the key a fourth time. So the
queue is SEEDED from `targets[].src_path`, which `cargo metadata` already
returns for every target of every member, and the `#[path]` walk sits on top of
it. `member.rglob("*.rs")` is kept as well, because a module file reached by a
plain `mod x;` is compiled and is named by no target.

**`--cap-lints allow` in `.cargo/config.toml`.** The seventh pass refused `-A`,
`--allow`, `-W` and `--warn`, which are the flags that name a lint.
`--cap-lints` names none and caps every lint in the crate graph, HLD 27.1's
five included, and it was in neither the refused set nor the declared limit.
MEASURED on the minimal crate: `rustflags = ["--cap-lints", "allow"]`,
`["--cap-lints=allow"]` and the bare-string `"--cap-lints allow"` all take
cargo clippy from 101 to 0, `["--cap-lints", "warn"]` does the same, and
`deny` and `forbid` leave it at 101. Planted in a full copy of this repository
this check exited 0 printing "1 cargo config(s) lower no denied lint through
rustflags", which positively asserts the false thing. It is a VALUE flag rather
than a lint-naming one, so it has its own branch: any `--cap-lints` level other
than `deny` or `forbid` is refused, and there are only four levels, so both
weakening values are probed rather than ratcheted.

**`--force-warn` was measured to weaken too, against the expectation.** It was
put down as a raising flag alongside `-D` and `-F` and it is not one: it forces
the level to warn and outranks the `-D warnings` the `clippy` gate passes.
MEASURED on the same crate: `["--force-warn", "clippy::cast_possible_truncation"]`
exits 0, `["--force-warn=clippy::cast_possible_truncation"]` exits 0 and
`["--force-warn", "clippy::pedantic"]` exits 0, while `["-Dwarnings"]` and
`["-Fclippy::cast_possible_truncation"]` both stay at 101. So it is in
`ALLOWING_FLAGS`, which is itself in the declared-constant ratchet now for the
reason `REFUSED_GROUPS` four lines below it already was: a probe can only ever
write one of the five names, and narrowing the set to that one leaves the guard
correct about a smaller rule and every probe green.

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

import json
import os
import re
import subprocess
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

# A `//` comment, removed after the block comments and after the reason clause.
# The order is the lexer's: a string literal wins over a comment start inside
# it, so `reason = "see https://example.invalid"` must be gone before this runs
# or the rest of the argument list disappears with the URL.
LINE_COMMENT = re.compile(r"//[^\n]*")


def _without_comments(text: str) -> tuple[str, bool]:
    """`text` with Rust comments removed, and whether they were balanced.

    Rust's lexer removes a comment before it sees a token, so
    `clippy::/*x*/pedantic` is the SAME attribute as `clippy::pedantic`. The
    spaced form the fifth pass closed was one member of a family rather than
    the family. MEASURED under the pinned 1.97.1 toolchain on a minimal crate
    carrying `cast_possible_truncation = "deny"` and one `x as i32`:

        no attribute                                    exits 101
        #![allow(clippy::/*c*/pedantic)]                 exits 0
        #![allow(clippy::/*c*/cast_possible_truncation)] exits 0
        #![allow(clippy:: // c<newline>pedantic)]        exits 0
        #![allow(/*c*/clippy::pedantic)]                 exits 0
        #![allow(clippy::pedantic/*c*/)]                 exits 0
        #![allow(clippy::/* /*n*/ */pedantic)]           exits 0

    A scan and not a regex, because of the last row: Rust block comments NEST,
    and a non-greedy `/\\*.*?\\*/` stops at the first `*/` and leaves ` */`
    glued to the name, which then matches nothing and is the same miss again.

    The second return value is False when a block comment was never closed,
    which is not a Rust program and IS what `INNER_ALLOW`'s `[^)]*` capture
    produces when a comment holds a `)`. `#![allow(clippy::/*)*/pedantic)]` is
    measured to exit 0 and the capture stops at `clippy::/*`, so the caller
    refuses that by name rather than reporting an unrecognised lint.
    """
    kept: list[str] = []
    depth = 0
    index = 0
    while index < len(text):
        if text.startswith("/*", index):
            depth += 1
            index += 2
            continue
        if depth and text.startswith("*/", index):
            depth -= 1
            index += 2
            continue
        if not depth:
            kept.append(text[index])
        index += 1
    return LINE_COMMENT.sub("", "".join(kept)), depth == 0

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


def _relative(path: Path) -> str:
    """`path` under this repository, or its whole self when it is outside one.

    A member or a compilation root can sit anywhere, `[lib] path` and a path
    dependency both being measured to put one outside `crates/`, so
    `relative_to(ROOT)` is not always defined and a refusal must still be able
    to name the file it is about.
    """
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return path.as_posix()


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


# A `rustflags` or `RUSTFLAGS` assignment anywhere in a cargo config, which is
# every table cargo reads them from at once: `[build]`, `[target.<triple>]`,
# `[target.'cfg(...)']` and an `[env]` entry setting the variable. Named by KEY
# rather than by table, so a table this parser has never heard of cannot open
# the route by being new.
RUSTFLAG_KEY = re.compile(r"^[^\S\n]*(?:rustflags|RUSTFLAGS)\s*=\s*", re.M)

# The flags that lower a lint's level, NAMES ONLY, so the declared constant
# ratchet in scripts/guards/catalogue.py records the set that decides how
# strict this is. Narrowing it is the widening a probe cannot see, for the same
# reason `REFUSED_GROUPS` above is recorded: `lint-policy.rustflags-allow`
# writes `-Aclippy::pedantic` and can only ever write one of these, so cutting
# the set to `{"-A": 1}` leaves that probe, its accept twin, the census and the
# guard all at exit 0 with the other four unguarded. MEASURED in the S03
# review's eighth pass.
#
# `--force-warn` is here since that pass and it is the one that was expected to
# raise. MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
# `cast_possible_truncation = "deny"` and one `x as i32`, with the `clippy`
# gate's own `-D warnings` passed: `--force-warn clippy::cast_possible_truncation`
# exits 0, the `=` form exits 0 and `--force-warn clippy::pedantic` exits 0. It
# FORCES the level to warn and outranks `-D warnings`, so it silences a denied
# lint exactly as `-A` does. `-D` and `-F` really do raise, measured at 101,
# and are not a route to anywhere this file refuses.
ALLOWING_FLAGS = {"-A": 1, "--allow": 1, "-W": 1, "--warn": 1,
                  "--force-warn": 1}

# `--cap-lints` names no lint and caps EVERY one, so it is not in the set
# above and it has its own branch. It takes a level rather than a lint, and
# there are exactly four levels, MEASURED under the pinned 1.97.1 toolchain on
# a crate denying `cast_possible_truncation`: `allow` takes cargo clippy from
# 101 to 0, `warn` does too because the cap is applied after the `-D warnings`
# the `clippy` gate passes, and `deny` and `forbid` leave it at 101. Both
# weakening values have a probe, so this pair needs no ratchet: unlike
# `REFUSED_GROUPS`, the space a probe would have to cover is closed.
CAP_LINTS = "--cap-lints"
CAP_LINTS_KEEPING = ("deny", "forbid")

CARGO_CONFIG_NAMES = ("config.toml", "config")

# Directories a walk for `.cargo/` must not descend into. Build output and
# installed packages carry thousands of files and none of them is a config
# this repository wrote.
UNWALKED = {".git", "target", "node_modules", ".venv", "corpus", "dist",
            "pkg", "__pycache__"}


def _cargo_configs() -> list[Path]:
    """Every `.cargo/config.toml` or `.cargo/config` in this working tree.

    A WALK and not `git ls-files`, so an untracked one a developer left in
    their own checkout is inside this guard's scope too. That is wider than
    tracked and therefore safe, and the sentence here said "tracked" until the
    S03 review's eighth pass, which is a claim about a smaller set than the
    code reads. cargo does not care whether the file is committed.
    """
    found: list[Path] = []
    for base, directories, _ in os.walk(ROOT):
        directories[:] = sorted(d for d in directories if d not in UNWALKED)
        if Path(base).name != ".cargo":
            continue
        for name in CARGO_CONFIG_NAMES:
            candidate = Path(base) / name
            if candidate.is_file():
                found.append(candidate)
    return sorted(found)


# A `rustflags` value that carries no flag at all. `rustflags = []` is a
# legitimate thing to write, most often left behind when the last flag is
# removed, and `tokens or None` read it as "a value this parser cannot read"
# and refused it. An empty list of arguments lowers nothing, so it is an empty
# ANSWER rather than no answer, and the two must not share a return value in a
# function whose `None` is a refusal.
EMPTY_FLAGS = re.compile(r"""\[\s*\]|""|''""")


def _flag_values(text: str) -> list[tuple[str, list[str] | None]]:
    """Each `rustflags` assignment's tokens, or `None` when unreadable.

    An array's elements and a bare string's whitespace-separated words are the
    same list of arguments to rustc, which is why both forms are measured to
    work and both are read here.

    An UNCLOSED array is `None`, and so is a value holding something that is
    not a string literal, because in both cases there are arguments reaching
    rustc that this function did not read. An array or string that is empty is
    an empty list, which is the distinction the S03 review's eighth pass found
    collapsed.
    """
    values: list[tuple[str, list[str] | None]] = []
    for match in RUSTFLAG_KEY.finditer(text):
        rest = text[match.end():]
        closed = True
        if rest.startswith("["):
            depth = 0
            end = 0
            for index, char in enumerate(rest):
                if char == "[":
                    depth += 1
                elif char == "]":
                    depth -= 1
                    if depth == 0:
                        end = index + 1
                        break
            span = rest[:end] if end else ""
            closed = bool(end)
        else:
            span = rest.splitlines()[0] if rest else ""
        quoted = re.findall(r'"([^"]*)"|\'([^\']*)\'', span)
        tokens = [word for pair in quoted for cell in pair
                  for word in cell.split() if cell]
        label = " ".join(span.split())[:120]
        if tokens:
            values.append((label, tokens))
        elif closed and EMPTY_FLAGS.fullmatch(span.strip()):
            values.append((label, []))
        else:
            values.append((label, None))
    return values


def rustflag_problems() -> list[str]:
    """`.cargo/config.toml` is read by cargo and was read by nothing here.

    No file under `scripts/`, `bin/`, `ci/`, `.githooks/` or `.github/`
    mentioned `rustflags` before the S03 review's seventh pass. MEASURED under
    the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`:

        no .cargo/config.toml                                  exits 101
        [build] rustflags = ["-Aclippy::pedantic"]              exits 0
        [build] rustflags = ["-Aclippy::cast_possible_truncation"] exits 0
        [target.'cfg(all())'] rustflags = ["-Aclippy::pedantic"] exits 0
        [build] rustflags = "-Aclippy::pedantic"                exits 0

    Added to a sandbox clone of this repository, where there is no `.cargo/`
    today, every check stayed green.

    **The FLAG is refused and not the file, and that is a decision.** A
    `.cargo/config.toml` is the ordinary home for a target runner, a linker
    choice, an alias and `[net]` settings, none of which touches a lint level.
    Refusing the file's existence would refuse a legitimate state, which is the
    runbook's own sentence about a guard that fails on everything, and it would
    push a real need into an undeclared workaround where nothing watches it.
    Refusing an `-A`, `--allow`, `-W`, `--warn` or `--force-warn` that names one
    of HLD 27.1's five lints or one of `REFUSED_GROUPS`, and any `--cap-lints`
    level other than `deny` or `forbid`, names exactly the things measured to
    switch a denied lint off, and it fails CLOSED on a `rustflags` value this
    parser cannot read at all.

    The limit, stated exactly, and it was not exhaustive until the S03 review's
    eighth pass. Two flags are OUT OF SCOPE by not existing here rather than by
    decision: cargo also reads `$CARGO_HOME/config.toml`, the `RUSTFLAGS`
    environment variable and `--config` on the command line, and none of those
    is in this repository. This check is about what the repository ships. A
    developer's own environment is theirs, and CI's is in
    `.github/workflows/ci.yml`, which sets no `RUSTFLAGS`. What that list did
    NOT name, and had to, is `--cap-lints`: it sat in neither the refused set
    nor the declared limit, and it is MEASURED to take cargo clippy from 101 to
    0 at `allow` and at `warn`. It has its own branch below, because it is a
    value flag rather than a lint-naming one.
    """
    problems: list[str] = []
    denied = set(REQUIRED_CLIPPY) | set(REQUIRED_RUST)
    for config in _cargo_configs():
        where = config.relative_to(ROOT).as_posix()
        for span, tokens in _flag_values(config.read_text(encoding="utf-8")):
            if tokens is None:
                problems.append(
                    f"{where} sets `rustflags` to `{span}`, which this parser "
                    f"cannot read as a list of arguments. A rustflags value "
                    f"reaches rustc whatever this file makes of it, and "
                    f"`-Aclippy::pedantic` there is MEASURED to take cargo "
                    f"clippy from 101 to 0 on a crate that denies "
                    f"cast_possible_truncation. Write it as an array of "
                    f"strings, or as one quoted string, so the flags can be "
                    f"read.")
                continue
            index = 0
            while index < len(tokens):
                token = tokens[index]
                index += 1
                # `--cap-lints` first, because it names a LEVEL rather than a
                # lint and the loop below is written around a lint name.
                if token == CAP_LINTS or token.startswith(f"{CAP_LINTS}="):
                    if token == CAP_LINTS:
                        level = tokens[index] if index < len(tokens) else ""
                        index += 1
                    else:
                        level = token[len(CAP_LINTS) + 1:]
                    if level in CAP_LINTS_KEEPING:
                        continue
                    problems.append(
                        f"{where} carries `{CAP_LINTS} "
                        f"{level or '(no level this parser could read)'}` in "
                        f"`rustflags`, which caps EVERY lint for every crate "
                        f"cargo builds from this directory, HLD 27.1's five "
                        f"included. It names none of them, so nothing else "
                        f"here sees it: it is not a lint-naming flag and it "
                        f"was in neither the refused set nor this check's "
                        f"declared limit. MEASURED under the pinned 1.97.1 "
                        f"toolchain on a crate denying "
                        f"cast_possible_truncation, with the `clippy` gate's "
                        f"own -D warnings passed: `allow` takes cargo clippy "
                        f"from 101 to 0, and so does `warn`, because the cap "
                        f"is applied after -D warnings rather than before it. "
                        f"Only `deny` and `forbid` leave the table standing. "
                        f"Cap at one of those, or allow the single lint at "
                        f"the expression that needs it, with a reason.")
                    continue
                lint = ""
                for flag in sorted(ALLOWING_FLAGS, key=len, reverse=True):
                    if token == flag:
                        lint = tokens[index] if index < len(tokens) else ""
                        index += 1
                        break
                    if token.startswith(f"{flag}="):
                        lint = token[len(flag) + 1:]
                        break
                    if not flag.startswith("--") and token.startswith(flag):
                        lint = token[len(flag):]
                        break
                if not lint:
                    continue
                bare = lint.removeprefix("clippy::")
                if bare not in denied and lint not in REFUSED_GROUPS:
                    continue
                problems.append(
                    f"{where} carries `{token}` in `rustflags`, which lowers "
                    f"`{lint}` for every crate cargo builds from this "
                    f"directory. HLD 27.1 denies it, `[workspace.lints]` says "
                    f"so and a rustflag outranks the manifest, so the "
                    f"`clippy` gate and this check would both stay green over "
                    f"a smaller set of rules. MEASURED under the pinned "
                    f"1.97.1 toolchain: `rustflags = [\"-Aclippy::pedantic\"]` "
                    f"takes cargo clippy from 101 to 0 on a crate denying "
                    f"cast_possible_truncation, and so does the "
                    f"`[target.'cfg(all())']` form. `--force-warn` is in the "
                    f"same set and was expected to raise: it FORCES the level "
                    f"to warn and outranks the -D warnings this project's "
                    f"`clippy` gate passes, measured at 101 to 0 on the same "
                    f"crate. Allow the single lint at the expression that "
                    f"needs it, with a reason.")
    return problems


def cargo_metadata_members() -> tuple[dict[Path, list[Path]] | None, str]:
    """Each member cargo reports and the source ROOTS it compiles, or why not.

    `[workspace] members` is not the member set. cargo additionally makes a
    PATH DEPENDENCY of a member a member when the dependency lives inside the
    workspace directory, and MEASURED in the S03 review's seventh pass: adding
    `vendor/probe` and `probe-vendored = { path = "../../vendor/probe" }` to
    `crates/ocelli-core/Cargo.toml` made `cargo metadata --no-deps` report 15
    packages while this check printed "14 workspace member(s)" at exit 0 and the
    census exited 0 beside it. The new crate carried no `[lints] workspace =
    true` and clippy compiled it. That is the fifth pass's `tools/oracle`
    defect reached through a different key, and reading the globs harder would
    only move the key again.

    The sentence above said "every path dependency" until the eighth pass and
    that is wider than cargo. MEASURED on a minimal workspace under the pinned
    1.97.1 toolchain: a path dependency at `../../../outsidedep`, outside the
    workspace directory, leaves `cargo metadata --no-deps` reporting ONE
    package, and moving the same crate to `vendor/insidedep` inside the
    workspace makes it two. Nothing in this function turns on the difference,
    because the answer comes from cargo either way, and a rule stated wider
    than it is invites the next reader to trust it somewhere it does not hold.

    **The TARGETS are read as well, and that is the eighth pass's fix.** A
    manifest's `[lib] path`, `[[bin]] path`, `[[test]] path`, `[[bench]] path`
    and `[[example]] path` each put a compilation root wherever the manifest
    says, so the crate root need not be under the member directory at all.
    MEASURED under the pinned 1.97.1 toolchain: with
    `[lib] path = "../../outside/lib.rs"` and that file carrying
    `#![allow(clippy::pedantic)]` and one `x as i32`, cargo clippy exits 0
    against a baseline of 101, and in a full copy of this repository this check
    exited 0 having scanned the now-unused `src/lib.rs` and never opened the
    real root. `targets[].src_path` is cargo's own answer to where each root
    is, so the walk is seeded from it rather than from an assumption about
    `src/`. The paths come back unnormalised, as
    `crates/a/../../outside/lib.rs`, and are resolved here.

    So the set comes from cargo, which is the only thing that knows the rule.
    Two invocations, deliberately, and the OK line says which one answered:

    `--locked --offline` first. It writes NOTHING: `--locked` refuses to
    rewrite `Cargo.lock` and `--offline` refuses to reach the network, so a
    guard run cannot leave a developer's tree dirty or a CI runner waiting on a
    registry. MEASURED on a fresh copy of this repository with no `target/`, it
    answers in 0.03 seconds.

    `--offline` alone second, and a plain run third, each only when the one
    before it failed. The lock is complete here so the first form answers, but
    a mid-edit manifest leaves it stale, and refusing a developer's whole lint
    gate over a stale lock would be a guard refusing a legitimate state. The
    second form may rewrite `Cargo.lock` from what is already on disk and the
    third may reach the network, which is exactly the order of preference: the
    quietest form that can answer.

    A `None` here is not a smaller answer, it is no answer, and the caller
    refuses rather than narrowing to the globs in silence.
    """
    attempts = (
        ("--locked --offline", ["cargo", "metadata", "--no-deps",
                                "--format-version", "1", "--locked",
                                "--offline"]),
        ("--offline, the lock was stale", ["cargo", "metadata", "--no-deps",
                                           "--format-version", "1",
                                           "--offline"]),
        ("a resolving run", ["cargo", "metadata", "--no-deps",
                             "--format-version", "1"]),
    )
    why = ""
    for label, argv in attempts:
        try:
            done = subprocess.run(argv, cwd=ROOT, capture_output=True,
                                  text=True, check=False)
        except OSError as error:
            return None, f"cargo could not be run at all ({error})"
        if done.returncode != 0:
            tail = (done.stderr.strip().splitlines() or ["no output"])[-1]
            why = (f"`{' '.join(argv)}` exited {done.returncode}: {tail}")
            continue
        try:
            packages = json.loads(done.stdout)["packages"]
        except (ValueError, KeyError, TypeError) as error:
            why = f"cargo metadata printed no readable package list ({error})"
            continue
        if not packages:
            why = "cargo metadata reported no package at all"
            continue
        found: dict[Path, list[Path]] = {}
        for package in packages:
            directory = Path(package["manifest_path"]).parent
            roots = {Path(target["src_path"]).resolve()
                     for target in package.get("targets", [])
                     if target.get("src_path")}
            found[directory] = sorted(set(found.get(directory, [])) | roots)
        return found, label
    return None, why


def workspace_members(
        text: str) -> tuple[list[Path], dict[Path, list[Path]], list[str], str]:
    """The member set cargo builds, with the manifest globs as a fallback.

    Returns the members, the compilation ROOTS cargo reports for each, the
    refusals and where the set came from. The roots are empty when cargo could
    not be asked, which is a state this function already refuses on its own
    account, so the source walk falls back to the member directory rather than
    to nothing.

    `manifest_members` below keeps every refusal the glob read already carried,
    because those are about the manifest a person writes and cargo's answer
    cannot be substituted for them: a pattern resolving to nothing makes cargo
    refuse the whole workspace, so without them the message would name cargo's
    exit status rather than the line to fix.

    When cargo answers, its set is the one that is walked. When it does not,
    this REFUSES and walks the globs anyway, so the pass below still runs and
    still reports what it can while the run as a whole is red. Falling back
    quietly would be the narrowing this function exists to end.
    """
    members, problems = manifest_members(text)
    reported, why = cargo_metadata_members()
    if reported is None:
        problems.append(
            f"the workspace member set could not be read from cargo, and "
            f"`[workspace] members` alone is not that set: {why}. cargo makes "
            f"every path dependency of a member a member too, MEASURED at 15 "
            f"packages against this file's 14, and a crate reached that way "
            f"carries no `[lints] workspace = true` and is compiled by "
            f"`cargo clippy --workspace` regardless. The globs are walked "
            f"below so the rest of this check still reports, and the run is "
            f"red because the set it walked is known to be the narrower one.")
        return (members, {}, problems,
                "the manifest globs, which cargo is wider than")
    return (sorted(reported), reported, problems,
            f"`cargo metadata --no-deps` ({why})")


def manifest_members(text: str) -> tuple[list[Path], list[str]]:
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


# `#[path = "..."]` on a module item, which puts that module's source
# anywhere, INCLUDING outside the member directory. MEASURED under the pinned
# 1.97.1 toolchain: a crate whose `src/lib.rs` reads
# `#[path = "../../shared_outside/shared.rs"] pub mod shared;` and whose
# `shared.rs` holds one `x as i32` exits 101, and with
# `#![allow(clippy::pedantic)]` at the top of that file it exits 0. A walk of
# `member.rglob("*.rs")` never opens it.
MODULE_PATH = re.compile(r"#\[\s*path\s*=\s*\"([^\"]+)\"\s*\]")


def member_sources(member: Path,
                   roots: list[Path] | None = None
                   ) -> tuple[list[Path], list[str]]:
    """Every `.rs` file a workspace member compiles, and the refusals.

    Not `src/lib.rs`. An inner attribute in `src/main.rs` is a second crate
    root and one in any module file governs that module, both measured, so a
    pass that reads one file answers a question about one file and says so by
    succeeding.

    `<member>/target/` at depth 1 ONLY. The previous rule skipped any path
    component named `target`, and the build directory is at the repository
    root, so all it could ever skip was a source module called `target`. HLD
    section 13 makes that a likely name in a rendering codebase and skipping
    it would be silent.

    `#[path]` is followed, transitively, because a module's source need not
    live under the member at all and the measurement above shows what that
    costs. Resolution is relative to the declaring file's own directory and
    then, failing that, to the directory Rust uses for a module inside a
    non-`mod.rs` parent. A `#[path]` neither rule can resolve is REFUSED and
    not skipped: it names a file this pass did not read, and an unread file is
    the state that produced the measurement.

    **`roots` is cargo's own list of this member's compilation roots**, from
    `targets[].src_path`, and the queue is seeded from it. Until the S03
    review's eighth pass this function's docstring claimed "every `.rs` file a
    workspace member compiles" while the queue was `member.rglob("*.rs")`,
    which is neither necessary nor sufficient: a manifest's `[lib] path`,
    `[[bin]] path` or `[[test]] path` puts a root anywhere, MEASURED to take
    cargo clippy from 101 to 0 with an inner group allow in a root outside the
    member and this check to exit 0 having read the file that is no longer
    compiled. The rglob is KEPT alongside it, because a module reached by a
    plain `mod x;` is compiled and is named by no target, and dropping it
    would trade one narrowing for another.

    A root cargo names and this pass cannot open is REFUSED, on the same
    argument as an unresolvable `#[path]`: it is a file that is compiled and
    was not read.
    """
    found: dict[Path, None] = {}
    problems: list[str] = []
    queue = [path for path in sorted(member.rglob("*.rs"))
             if path.relative_to(member).parts[:1] != ("target",)]
    for root in roots or []:
        if root.is_file():
            queue.append(root)
            continue
        problems.append(
            f"cargo reports `{root}` as a compilation root of the workspace "
            f"member `{_relative(member)}` and this pass could not open it, "
            f"so a file clippy compiles went unread. An inner group allow at "
            f"the top of a crate root is MEASURED to take cargo clippy from "
            f"101 to 0, and a root this check never reads is the state that "
            f"produced that measurement.")
    while queue:
        path = queue.pop(0)
        resolved = path.resolve()
        if resolved in found:
            continue
        found[resolved] = None
        try:
            source = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for named in MODULE_PATH.findall(source):
            candidates = [path.parent / named, path.parent / path.stem / named]
            for candidate in candidates:
                if candidate.is_file():
                    queue.append(candidate)
                    break
            else:
                problems.append(
                    f"{_relative(path)} declares "
                    f"`#[path = \"{named}\"]` and neither "
                    f"{candidates[0]} nor {candidates[1]} is a file, so this "
                    f"pass did not read the module it names. A module source "
                    f"outside the member directory is invisible to a walk of "
                    f"the member, and an inner group allow in one is MEASURED "
                    f"to take cargo clippy from 101 to 0.")
    return sorted(found), problems


def inherits_workspace_lints(manifest: str) -> bool:
    """Does this member manifest inherit `[workspace.lints]`.

    TWO spellings, and the guard read one of them until the S03 review's
    eighth pass. `[lints]` with `workspace = true` under it is the common
    form. The top-level dotted `lints.workspace = true` is the same table
    written on one line, it is what several crates in the wild write, and
    MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: with that line
    above `[package]`, cargo clippy exits 101, so the table IS inherited,
    while the previous regex found nothing and this check refused a legitimate
    manifest at exit 1.

    The line has to sit before the first table header to be the top-level
    `lints` table. MEASURED the same way: written under `[package]` it is
    `package.lints`, cargo prints `unused manifest key: package.lints` and
    clippy exits 0, so that spelling really is a member that does not inherit
    and refusing it is right.

    Trailing `#` comments are removed with `_row_body`, quote-aware, so
    `workspace = true  # HLD 27.1` is not read as a different value.
    """
    body = "\n".join(_row_body(line) for line in manifest.splitlines())
    block = re.search(r"^\[lints\]\s*$(.*?)(?=^\[|\Z)", body, re.M | re.S)
    if block is not None and re.search(r"^\s*workspace\s*=\s*true\s*$",
                                       block.group(1), re.M):
        return True
    head: list[str] = []
    for line in body.splitlines():
        if line.lstrip().startswith("["):
            break
        head.append(line)
    return re.search(r"^\s*lints\.workspace\s*=\s*true\s*$",
                     "\n".join(head), re.M) is not None


def _names_in(argument_list: str) -> list[str]:
    """The lint names an `allow(...)` or `expect(...)` argument list holds.

    Whitespace inside a name is removed before the comparison, because Rust
    tokenises `clippy :: pedantic` exactly as `clippy::pedantic` and measuring
    it under the pinned toolchain showed the spaced form silencing the lint
    while the set lookup matched nothing. COMMENTS are removed for the same
    reason and it is the same defect one lexer rule further on: `_without_comments`
    records the measurements. A `reason = "..."` clause is stripped before both,
    so it is neither read as a lint name, nor split into two by a comma inside
    its own string, nor truncated at a `//` that is part of a URL.
    """
    body, _ = _without_comments(REASON.sub("", argument_list))
    return [re.sub(r"\s+", "", name)
            for name in body.split(",")
            if re.sub(r"\s+", "", name)]


def _outer_module_attributes(source: str) -> list[re.Match[str]]:
    """Every OUTER `allow`/`expect` whose next non-trivial token is a `mod`.

    One reader, because three callers need the same answer: `module_allows`,
    the unreadable-argument refusal below, and any future pass over the same
    scope. Written out twice it would be two rules that drift.
    """
    found: list[re.Match[str]] = []
    for match in OUTER_ALLOW.finditer(source):
        rest = source[match.end():]
        trivia = TRIVIA.match(rest)
        tail = rest[trivia.end():] if trivia else rest
        if MODULE_ITEM.match(tail):
            found.append(match)
    return found


def unreadable_allows(source: str) -> list[str]:
    """Attributes whose argument list this parser did not read to its end.

    `INNER_ALLOW` and `OUTER_ALLOW` capture up to the first `)`, and a block
    comment may hold one. MEASURED under the pinned 1.97.1 toolchain on a
    minimal crate denying `cast_possible_truncation`:
    `#![allow(clippy::/*)*/pedantic)]` takes cargo clippy from 101 to 0 and the
    capture stops at `clippy::/*`, which is in no set this file compares
    against. An unterminated block comment inside the capture therefore means
    the capture is a fragment, and a fragment is refused by name rather than
    passed over as an unrecognised lint. Fail closed, because the fragment is
    exactly the shape a deliberate bypass takes.

    The scope is the scope the refusals already have: inner attributes, and
    outer attributes on a `mod`. An outer attribute on a `fn` is the local,
    visible choice HLD 27.1's note asks for whether or not this parser can read
    its argument list.
    """
    found: list[str] = []
    for match in (list(INNER_ALLOW.finditer(source))
                  + _outer_module_attributes(source)):
        _, balanced = _without_comments(REASON.sub("", match.group(1)))
        if not balanced:
            found.append(" ".join(match.group(0).split()))
    return found


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
    for match in _outer_module_attributes(source):
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
    members, roots, member_problems, member_source = workspace_members(text)
    problems += member_problems
    for member in members:
        where = _relative(member)
        try:
            manifest = (member / "Cargo.toml").read_text(encoding="utf-8")
        except OSError as error:
            # A bare traceback, until the S03 review's eighth pass. This is
            # the presentation `scripts/ci_floor_check.py` stopped giving in
            # the fifth pass: fail-closed is not the same as actionable, and a
            # member cargo names whose manifest cannot be read is a real
            # refusal that has to arrive under the FAIL header with the rest.
            problems.append(
                f"{where} is a workspace member cargo reports and its "
                f"Cargo.toml could not be read ({error}). Whether it inherits "
                f"HLD 27.1's table is therefore unknown, and an unknown is not "
                f"an inheritance.")
            continue
        if not inherits_workspace_lints(manifest):
            problems.append(
                f"{where} does not inherit the workspace lint table. "
                f"`[lints]` with `workspace = true`, or the top-level dotted "
                f"`lints.workspace = true`, is what makes HLD 27.1 apply to "
                f"it, and a member without it compiles under a smaller set of "
                f"rules while the `clippy` gate stays green.")

    # An inner `allow` or `expect` puts a denied lint back to sleep for a
    # whole crate or a whole module, and an OUTER one on a `mod` item does the
    # same for that module tree. By name, and by any group that contains one.
    named = set(REQUIRED_CLIPPY) | set(REQUIRED_RUST)
    scanned = 0
    for member in members:
        sources, source_problems = member_sources(member, roots.get(member))
        problems += source_problems
        for path in sources:
            scanned += 1
            where = _relative(path)
            source = path.read_text(encoding="utf-8")
            for attribute in unreadable_allows(source):
                problems.append(
                    f"{where} carries `{attribute}`, whose argument list this "
                    f"check could not read to its end: a block comment inside "
                    f"it is never closed, which means the `)` this parser "
                    f"stopped at was inside the comment rather than at the end "
                    f"of the list. MEASURED under the pinned 1.97.1 "
                    f"toolchain: `#![allow(clippy::/*)*/pedantic)]` takes "
                    f"cargo clippy from 101 to 0 on a crate denying "
                    f"cast_possible_truncation. Rust's lexer removes the "
                    f"comment before it reads the name and this file cannot, "
                    f"so the attribute is refused rather than read as an "
                    f"unrecognised lint. Write the attribute without a "
                    f"comment inside its argument list.")
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

    # `.cargo/config.toml`, which cargo reads and nothing here did.
    problems += rustflag_problems()

    if problems:
        print("FAIL: the HLD 27.1 lint policy")
        for problem in problems:
            print(f"  {problem}")
        return 1

    # Every number here is derived from the walk that produced it. The member
    # count was the literal `crates/` glob until the fifth pass, and it read
    # thirteen while cargo built fourteen. Where the set CAME FROM is printed
    # since the seventh pass, because the globs and cargo's answer are two
    # different sets and the difference was measured at one crate.
    configs = _cargo_configs()
    print(f"OK: {len(REQUIRED_CLIPPY)} clippy lint(s) at or above HLD 27.1's "
          f"level and no group row weaker than deny, {len(members)} workspace "
          f"member(s) from {member_source} inherit the table, "
          f"{scanned} .rs file(s), cargo's own target roots seeded and "
          f"`#[path]` modules followed, carry no inner "
          f"allow or expect of a denied lint or of a group holding one, and "
          f"none on a `mod` item, {len(configs)} cargo config(s) lower no "
          f"denied lint through rustflags, unsafe_code denied by {unsafe_by}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
