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
about all five. A group row weaker than `deny` is refused now, in every TOML
spelling, because the table is parsed rather than matched. See "The class,
named in the eleventh pass" below.

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

## Two more routes, both measured in the S03 review's ninth pass

**`include!` is a fourth way to compile a file.** The set of files this check
reads is RECONSTRUCTED rather than known, and every pass since the fifth has
found a new key to it: the member glob, `targets[].src_path`, `#[path]`, and
now this. MEASURED under the pinned 1.97.1 toolchain on a minimal crate
carrying `cast_possible_truncation = "deny"`: `crates/a/src/lib.rs` reading
`include!("../../../outside/hidden.rs")`, where `hidden.rs` carries
`#[allow(clippy::pedantic)]` on a `#[path = "inner.rs"] pub mod` and the module
holds one `x as i32`, takes `cargo clippy --workspace --all-targets --
-D warnings` from 101 to 0, and `member_sources` returned only
`crates/a/src/lib.rs`. Planted in a full copy of this repository the same pair
holds and this check printed its usual "46 .rs file(s)" line at exit 0.
`member_sources` follows `include!` now, refuses one it cannot resolve, and its
docstring states plainly that the set is reconstructed and how, because it
claimed "every `.rs` file a workspace member compiles" through four passes in
which that sentence was false. **rustc's own dep-info was considered as the
authority and rejected with a reason**, which `member_sources` records in full.

**`RUSTFLAG_KEY`'s `^` anchor could not see a dotted or quoted key.** TOML
spells `[build] rustflags = [...]` three ways and the regex matched one of
them. MEASURED under the pinned 1.97.1 toolchain: `build.rustflags =
["-Aclippy::cast_possible_truncation"]` takes cargo clippy from 101 to 0,
`build.rustflags = ["-Aclippy::pedantic"]` does, `[build]` with `"rustflags" =
[...]` does, and `target."cfg(all())".rustflags = [...]` does, while
`_flag_values` returned `[]` for each. Planted in a full copy of this
repository this check printed "1 cargo config(s) lower no denied lint through
rustflags" at exit 0, which is the eighth pass's `--cap-lints` outcome exactly.
The config is parsed with `tomllib` now and the three keys are read by NAME,
which retires the whole regex-against-TOML class from that function.

## The class, named in the eleventh pass, and what closing it looks like

The lesson above was learned in ONE parser of this file and not in the other.
`LINT_ROW` went on reading `[workspace.lints.*]` with a regex twenty lines
away, and five passes closed five spellings of the same group row one
alternation at a time: the inline table, the trailing comment, the dotted key,
the two-line row and finally the QUOTED key. The last of those defeated the row
regex and the declared constant that was supposed to backstop it at the same
time, because that constant was a text slice captured by another regex over the
same grammar. Two parsers agreeing with each other is not either of them
agreeing with TOML.

So there is no row regex any more. `workspace_lints` reads the tables with
`tomllib`, `lint_levels` reduces a row to a level, `workspace_lints_rows`
renders the parsed rows for the ratchet, and `inherits_workspace_lints` reads
`lints.workspace` as a key path rather than as two regexes plus a line scanner.
All five spellings, and the ones nobody has thought of, are one code path.
`tomllib` is stdlib on 3.11 and up, `pyproject.toml` requires 3.12 and the CI
job pins 3.12. A document `tomllib` cannot parse is REFUSED here rather than
read as an empty table, which is the direction the regexes had wrong.

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

import functools
import json
import os
import re
import subprocess
import sys
import tomllib
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

# `LINT_ROW` and `_row_body` USED TO SIT HERE, and the eleventh review pass
# named the class rather than the spelling: a regex against TOML closes the one
# spelling whoever wrote it thought of, and TOML has more. The record, in the
# order the passes found them, every one measured under the pinned 1.97.1
# toolchain on a minimal crate carrying `cast_possible_truncation = "deny"` and
# one `x as i32`, baseline cargo clippy exit 101:
#
#     pedantic = { level = "allow", priority = 1 }              exit 0, pass 6
#     ... } # keeps noise down                                  exit 0, pass 6
#     pedantic.level = "allow" / pedantic.priority = 1          exit 0, pass 9
#     pedantic = { level = "allow",  / priority = 1 }           exit 0, pass 6
#     "pedantic" = { level = "allow", priority = 1 }            exit 0, pass 11
#
# The first three were patched into the regex one alternation at a time. The
# fourth was declared a residual. The FIFTH, the quoted key, defeated the regex
# and its declared backstop together, because the constant's capture ended at
# the last line beginning with a BARE key and `"` is not in `[\w.-]`.
#
# `tomllib` is stdlib on 3.11 and up, `pyproject.toml` requires 3.12 and the CI
# job pins 3.12, and this file was ALREADY reading cargo configs with it twenty
# lines below. So all five spellings are one code path now, `workspace_lints`
# is the only reader of these tables, and the same function is what the
# declared constant in `scripts/guards/catalogue.py` records, so the two
# mechanisms cannot disagree with each other or with the grammar.
#
# What fails CLOSED and did not before: a `Cargo.toml` this parser cannot parse
# at all. The regexes returned an empty table and the check went on to report
# five absent lints. `main` refuses the document instead.
#
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
    """The globs `[workspace] members` declares, and what `exclude` removes.

    **The third hand-rolled reader of this document, and the S03 review's
    twelfth pass deleted it rather than repairing it.** It required
    `[workspace]` to be a literal table HEADER on its own line and each entry
    to be double quoted, so `workspace.members = ["crates/*"]`, a single-quoted
    literal string and a member listed under a dotted `workspace.members`
    sub-key were each read as no members at all. It sat in the same file as
    `workspace_lints`, which had already stopped reading this document with a
    regex for exactly that reason, three passes earlier.

    Reading no members is not a silent pass here, because `main` refuses a
    workspace whose member list it cannot resolve, but it is the same
    "returned an empty result for a document it could not read" shape that
    every other reader in this file has now had removed.

    A parse error yields no members, and that branch is unreachable rather
    than permissive: `main` parses this same text with `workspace_lints`
    FIRST and returns on a parse error before this function is called.
    """
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return [], []
    workspace = document.get("workspace")
    if not isinstance(workspace, dict):
        return [], []

    def listing(key: str) -> list[str]:
        found = workspace.get(key)
        if not isinstance(found, list):
            return []
        return [entry for entry in found if isinstance(entry, str)]

    return listing("members"), listing("exclude")


# The cargo-config keys that carry rustflags, read from a PARSED document
# rather than matched in the text. `RUSTFLAG_KEY` was
# `^[^\S\n]*(?:rustflags|RUSTFLAGS)\s*=\s*`, whose `^` anchor requires the key
# to start its line, and TOML has two spellings that do not: the dotted key and
# the quoted key. MEASURED under the pinned 1.97.1 toolchain on a minimal crate
# carrying `cast_possible_truncation = "deny"` and one `x as i32`, with the
# `clippy` gate's own `-D warnings` passed:
#
#     no .cargo/config.toml                                  exits 101
#     build.rustflags = ["-Aclippy::cast_possible_truncation"] exits 0
#     build.rustflags = ["-Aclippy::pedantic"]                exits 0
#     [build] then "rustflags" = ["-Aclippy::pedantic"]       exits 0
#     target."cfg(all())".rustflags = ["-Aclippy::pedantic"]  exits 0
#
# and `_flag_values` returned `[]` for each of the first three. Planted in a
# full copy of this repository the guard printed "1 cargo config(s) lower no
# denied lint through rustflags" at exit 0, which is the eighth pass's
# `--cap-lints` outcome exactly: a positive assertion of the false thing.
#
# `tomllib` is stdlib on 3.11 and up and `pyproject.toml` requires 3.12, so
# there is no reason to read TOML with a regex here. That removes the whole
# regex-against-TOML class from this function, and the `lint-policy` limit
# already recorded that a dotted key defeated `LINT_ROW`, so the lesson had
# been learned in one parser of this file and not in the other. The eleventh
# pass finished the job: `workspace_lints` reads the lint tables the same way
# and there is no regex over TOML left in this file.
#
# `[env] RUSTFLAGS` is read too and it is NOT a route today. MEASURED the same
# way: `[env] RUSTFLAGS = "-Aclippy::pedantic"` leaves cargo clippy at 101,
# because `[env]` sets the variable for the processes cargo spawns rather than
# for cargo's own flag resolution, and `CARGO_ENCODED_RUSTFLAGS` there is 101
# as well. It is refused anyway, because it is the same intent written one
# table over and this guard should not depend on cargo's precedence between the
# two staying where it is.
BUILD_TABLE = "build"
TARGET_TABLE = "target"
ENV_TABLE = "env"
RUSTFLAGS_KEY = "rustflags"
RUSTFLAGS_VARIABLE = "RUSTFLAGS"

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


@functools.cache
def _cargo_configs() -> tuple[Path, ...]:
    """Every `.cargo/config.toml` or `.cargo/config` in this working tree.

    A WALK and not `git ls-files`, so an untracked one a developer left in
    their own checkout is inside this guard's scope too. That is wider than
    tracked and therefore safe, and the sentence here said "tracked" until the
    S03 review's eighth pass, which is a claim about a smaller set than the
    code reads. cargo does not care whether the file is committed.

    CACHED, since the S03 review's tenth pass. Two callers want the same
    answer in one run, `rustflag_problems` and `main`'s OK line, and the walk
    was done twice over the whole tree for it. The cache lives for the process
    and this script is a process, so nothing here observes the tree twice and
    disagrees with itself either. The return is a tuple for the same reason:
    a cached list is a shared mutable, and a caller that sorted it in place
    would change what the other caller sees.
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
    return tuple(sorted(found))


def _argument_tokens(value: object) -> list[str] | None:
    """A rustflags value as a list of arguments, or `None` when unreadable.

    cargo accepts two forms and both are measured to work: an ARRAY, whose
    elements are passed to rustc one argument each, and a bare STRING, which
    cargo splits on whitespace. Anything else is a value this function did not
    read, and there are arguments reaching rustc either way, so it is `None`
    and the caller refuses.

    An array element is split on whitespace as well, which cargo does not do.
    That is deliberate and it is the fail-CLOSED direction: `["-A clippy::pedantic"]`
    is one argument rustc rejects rather than a flag it honours, so splitting
    can only ever make this function see a flag it would otherwise miss, and
    the bare-string form needs the split anyway.

    An array or string that is EMPTY is an empty list rather than `None`.
    `rustflags = []` is what a config holds when the last flag is removed, it
    lowers nothing, and an empty ANSWER must not share a return value with no
    answer in a function whose `None` is a refusal.
    """
    if isinstance(value, str):
        return value.split()
    if isinstance(value, list):
        if not all(isinstance(item, str) for item in value):
            return None
        return [word for item in value for word in item.split()]
    return None


def _config_flag_values(
        text: str) -> tuple[list[tuple[str, list[str] | None]], str]:
    """Every rustflags setting a cargo config declares, and why not.

    Returns the settings as (key path, arguments) and a refusal reason that is
    the empty string when the document parsed. A cargo config that is not TOML
    is REFUSED rather than read as declaring nothing: cargo would refuse it
    too, and a parse failure read as an empty answer is this file's own named
    failure of answering a question about an empty set in the language of
    success.

    Three key paths, named because cargo names them: `build.rustflags`,
    `target.<any>.rustflags` and the `RUSTFLAGS` entry of `[env]`. Read from a
    parsed document, so TOML's dotted and quoted spellings of each are the same
    key here, which the `^`-anchored regex this replaced could not see.

    An `[env]` entry may be a bare string or the `{ value = "...", force =
    true }` table, and both are read.
    """
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        return [], str(error)
    if not isinstance(document, dict):  # pragma: no cover, tomllib returns one
        return [], "the document is not a table"

    found: list[tuple[str, list[str] | None]] = []

    def top(name: str) -> dict:
        """One top-level table, or an empty one. NOT `table()` below, which
        reads a `[workspace.lints.*]` table out of `Cargo.toml`'s text."""
        value = document.get(name)
        return value if isinstance(value, dict) else {}

    if RUSTFLAGS_KEY in top(BUILD_TABLE):
        found.append((f"{BUILD_TABLE}.{RUSTFLAGS_KEY}",
                      _argument_tokens(top(BUILD_TABLE)[RUSTFLAGS_KEY])))
    for triple, settings in sorted(top(TARGET_TABLE).items()):
        if isinstance(settings, dict) and RUSTFLAGS_KEY in settings:
            found.append((f"{TARGET_TABLE}.{triple}.{RUSTFLAGS_KEY}",
                          _argument_tokens(settings[RUSTFLAGS_KEY])))
    entry = top(ENV_TABLE).get(RUSTFLAGS_VARIABLE)
    if entry is not None:
        if isinstance(entry, dict):
            entry = entry.get("value")
        found.append((f"{ENV_TABLE}.{RUSTFLAGS_VARIABLE}",
                      _argument_tokens(entry)))
    return found, ""


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
    level other than `deny` or `forbid`, covers everything measured to switch a
    denied lint off, and it fails CLOSED on a `rustflags` value this parser
    cannot read at all.

    That set is a SUPERSET of what is measured to weaken, and this sentence
    said "exactly" until the S03 review's ninth pass. `-W` and `--warn` are in
    it and neither switches a denied lint off under the `clippy` gate's own
    `-D warnings`: MEASURED under the pinned 1.97.1 toolchain,
    `-Wclippy::cast_possible_truncation` exits 101,
    `--warn=clippy::cast_possible_truncation` exits 101 and
    `-Wclippy::pedantic` exits 101, because the lint is still a warning and
    `-D warnings` promotes it. Refusing them is still right, because they lower
    the level the manifest sets and the gate's `-D warnings` is the only thing
    holding the line, and it is right by decision rather than by measurement.
    `--force-warn` is the one in that family that really does weaken, measured
    at 0 and recorded beside `ALLOWING_FLAGS`.

    **"They lower the level the manifest sets" is true of a GROUP too, and the
    S03 review's tenth pass raised the opposite and the measurement settled
    it.** The reading offered was that `clippy::pedantic` defaults to allow, so
    `-W` on it only raises, and refusing a project that turns pedantic on as
    warnings is refusing a legitimate config. Half of that is right and the
    half it leaves out is the half this check is about. MEASURED under the
    pinned 1.97.1 toolchain on the minimal crate, WITHOUT the gate's
    `-D warnings` so the flag is read on its own: with no rustflags cargo
    clippy exits 101, and with `["-Wclippy::pedantic"]` it exits 0 and prints
    `warning: casting i64 to i32 may truncate the value` where the error was.
    A group flag outranks the manifest for every member lint, so the same flag
    that raises a hundred pedantic lints demotes the four rows of 27.1's table
    that pedantic reaches. Restricting the `-W` refusal to the five NAMED
    lints, which was the other repair offered, would therefore have been a
    fail-open through `clippy::pedantic` and `clippy::restriction`. What was
    wrong was only the MESSAGE, which told a group it was denied by 27.1 and
    by `[workspace.lints]`, and neither names a group. It is split now:
    `GROUP_REACHES` for the two measured to reach the table, and a blanket
    refusal by decision for the other seven.

    `-D` and `-F` are in neither `ALLOWING_FLAGS` nor `REFUSED_GROUPS`'s reach
    and are accepted on any name, group included. MEASURED:
    `["-Dclippy::pedantic"]` exits 101 with the gate's `-D warnings` and 101
    without it, so a project turning pedantic on as an error is a legitimate
    state this check must not refuse, and
    `lint-policy.deny-a-group-is-permitted` watches that direction.

    **The config is PARSED since the S03 review's ninth pass, and the SPELLING
    was the hole.** `RUSTFLAG_KEY` anchored on `^`, so TOML's dotted key and
    quoted key were never matched at all. MEASURED under the pinned 1.97.1
    toolchain on the minimal crate: `build.rustflags =
    ["-Aclippy::cast_possible_truncation"]` takes cargo clippy from 101 to 0,
    `build.rustflags = ["-Aclippy::pedantic"]` does, `[build]` with `"rustflags"
    = [...]` does, and `target."cfg(all())".rustflags = [...]` does, while
    `_flag_values` returned `[]` for each. Planted in a full copy of this
    repository the guard printed "1 cargo config(s) lower no denied lint
    through rustflags" at exit 0, which is the eighth pass's `--cap-lints`
    outcome exactly. `tomllib` reads the document now, which removes the whole
    regex-against-TOML class from this function rather than one more spelling
    of it, and a config that does not parse is refused rather than read as
    declaring nothing.

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

    One further limit, declared rather than closed: `[env] RUSTFLAGS` is read
    and refused here and it is NOT a route today, MEASURED at 101 along with
    `[env] CARGO_ENCODED_RUSTFLAGS`, because `[env]` sets a variable for the
    processes cargo spawns rather than for cargo's own flag resolution.
    Refusing it is a decision and not a measurement, and the reason is that it
    is the same intent one table over.
    """
    problems: list[str] = []
    denied = set(REQUIRED_CLIPPY) | set(REQUIRED_RUST)
    for config in _cargo_configs():
        where = config.relative_to(ROOT).as_posix()
        settings, unparseable = _config_flag_values(
            config.read_text(encoding="utf-8"))
        if unparseable:
            problems.append(
                f"{where} is a cargo config and does not parse as TOML "
                f"({unparseable}). cargo refuses a config it cannot parse, so "
                f"this is not a working state, and reading a parse failure as "
                f"a file that declares no rustflags would answer a question "
                f"about an empty set in the language of success. "
                f"`build.rustflags = [\"-Aclippy::pedantic\"]` in this file is "
                f"MEASURED to take cargo clippy from 101 to 0 on a crate that "
                f"denies cast_possible_truncation, and a document this check "
                f"cannot parse may hold exactly that.")
            continue
        for span, tokens in settings:
            if tokens is None:
                problems.append(
                    f"{where} sets `{span}` to a value this parser "
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
                # WHY this name is refused, and the two answers are not the
                # same claim. The message said "HLD 27.1 denies it,
                # `[workspace.lints]` says so" for every name, and for a GROUP
                # that is false twice over: 27.1 names five lints and no group,
                # and `[workspace.lints.clippy]` carries no group row. The S03
                # review's tenth pass raised it, and the measurement it rests
                # on turned out to cut the other way, so the wording is split
                # here rather than the refusal narrowed. See `GROUP_REACHES`
                # and this function's docstring.
                if lint in REFUSED_GROUPS:
                    reaches = GROUP_REACHES.get(lint, BLANKET)
                    because = (
                        f"which sets `{lint}` for every crate cargo builds "
                        f"from this directory. A GROUP is not a row of HLD "
                        f"27.1's table and `[workspace.lints]` carries none, "
                        f"so what is refused here is the group reaching the "
                        f"table under it: `{lint}` is {reaches}. MEASURED "
                        f"under the pinned 1.97.1 toolchain on a crate "
                        f"denying cast_possible_truncation with one "
                        f"`x as i32`, `rustflags = [\"-Wclippy::pedantic\"]` "
                        f"takes cargo clippy from 101 to 0 and demotes that "
                        f"error to a warning, because a group flag outranks "
                        f"the manifest for every member lint. It RAISES the "
                        f"hundred-odd pedantic lints the manifest never sets "
                        f"in the same breath, and this check does not weigh "
                        f"one against the other: the four rows of 27.1's "
                        f"table it lowers are the whole of what it is asked "
                        f"about. A group this file is not measured to reach "
                        f"the table is refused by DECISION, as a blanket "
                        f"level over a set whose membership clippy owns and "
                        f"may change")
                else:
                    because = (
                        f"which lowers `{lint}` for every crate cargo builds "
                        f"from this directory. HLD 27.1 denies it, "
                        f"`[workspace.lints]` says so and a rustflag outranks "
                        f"the manifest, so the `clippy` gate and this check "
                        f"would both stay green over a smaller set of rules")
                problems.append(
                    f"{where} carries `{token}` in `rustflags`, "
                    f"{because}. MEASURED under the pinned "
                    f"1.97.1 toolchain: `rustflags = [\"-Aclippy::pedantic\"]` "
                    f"takes cargo clippy from 101 to 0 on a crate denying "
                    f"cast_possible_truncation, and so does the "
                    f"`[target.'cfg(all())']` form. `--force-warn` is in the "
                    f"same set and was expected to raise: it FORCES the level "
                    f"to warn and outranks the -D warnings this project's "
                    f"`clippy` gate passes, measured at 101 to 0 on the same "
                    f"crate. `-D` and `-F` really do raise, measured at 101 "
                    f"for `-Dclippy::pedantic` with and without the gate's own "
                    f"-D warnings, and are not refused here at all. Allow the "
                    f"single lint at the expression that needs it, with a "
                    f"reason.")
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
#
# **TWO SPELLINGS of the same key were skipped in silence until the S03
# review's tenth pass, and this file already knew both of them elsewhere.**
# `#\[\s*path` required `path` to be the first thing inside the attribute, and
# `\"([^\"]+)\"` required a plain string literal. So:
#
# - `#[cfg_attr(all(), path = "...")]` matched nothing. `INNER_ALLOW` above
#   deliberately reads an `allow` reached through `cfg_attr` and says so, and
#   the same wrapper one attribute over was invisible here.
# - `#[path = r"..."]` matched nothing. `INCLUDE_PATH` below already carries
#   `(?:r#*)?` for exactly this, because a raw string literal is the same file
#   name written another way. The distance between the two was written here as
#   "twenty lines" and is twenty-six, which is the kind of number that is wrong
#   the first time either regex moves and is worth nobody's arithmetic.
#
# MEASURED under the pinned 1.97.1 toolchain on a minimal workspace carrying
# `cast_possible_truncation = "deny"`, with the module source holding
# `#![allow(clippy::pedantic)]` and one `x as i32`: the plain form is refused
# at guard exit 1, the `cfg_attr` form gave guard exit 0 and the raw-string
# form gave guard exit 0, and all three take `cargo clippy --workspace
# --all-targets -- -D warnings` from 101 to 0. Planted in a full clone of this
# repository both took clippy from 101 to 0 while the guard printed its usual
# "46 .rs file(s)" line at exit 0, having read neither file. This is not a
# fifth key. It is key three with two spellings the code did not read, and the
# effect is the one every earlier finding had: a file clippy compiles, passed
# over in silence rather than refused.
#
# The DECLARED LIMIT this widening buys, and it is `INNER_ALLOW`'s exactly.
# The `cfg_attr` predicate is not evaluated, so `#[cfg_attr(any(), path =
# "gone.rs")]` names a file rustc never resolves and this pass refuses for not
# finding it. Refusing there is the fail-closed direction of a predicate this
# file cannot evaluate without being a compiler, and the alternative, skipping
# every `cfg_attr`, is the hole measured above.
MODULE_PATH = re.compile(r"#\[[^\]]*?\bpath\s*=\s*(?:r#*)?\"([^\"]+)\"")

# `include!(...)`, the FOURTH key to the same set and the one none of the three
# above reaches. It pastes another file's tokens in at this point, so that file
# is compiled and is named by no target, no glob of the member and no `#[path]`.
# MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
# `cast_possible_truncation = "deny"`: with `crates/a/src/lib.rs` reading
# `include!("../../../outside/hidden.rs")`, that file carrying
# `#[allow(clippy::pedantic)]` on a `#[path = "inner.rs"] pub mod` and the
# module holding one `x as i32`, `cargo clippy --workspace --all-targets --
# -D warnings` exits 0 against a baseline of 101. Planted in a full copy of
# this repository, clippy went from 101 to 0 the same way and this check
# printed the same "46 .rs file(s)" line at exit 0, having read neither file.
#
# TWO patterns, and the second is not a duplicate of the first. `INCLUDE_ANY`
# finds the macro whatever its argument is, and `INCLUDE_PATH` reads the
# argument when it is a plain string literal. An `include!` the second cannot
# read is REFUSED by the first, because `include!(concat!(env!("OUT_DIR"),
# "/generated.rs"))` names a real file this check cannot resolve without
# running the build, and a generated file carrying a group allow is the same
# hole through a path nobody typed.
#
# Rust's own resolution rule, and it differs from `#[path]`'s: the argument is
# relative to the directory of the file the `include!` is written in. MEASURED
# from the dep-info of the run above, `crates/a/src/../../../outside/hidden.rs`.
INCLUDE_ANY = re.compile(r"\binclude!\s*[(\[{]")
INCLUDE_PATH = re.compile(r"\binclude!\s*[(\[{]\s*(?:r#*)?\"([^\"]*)\"")


def member_sources(member: Path,
                   roots: list[Path] | None = None
                   ) -> tuple[list[tuple[Path, str]], list[str], dict[str, int]]:
    """A RECONSTRUCTION of the `.rs` files a member compiles, with their text.

    **Read that first sentence as written.** This function does not know the
    set clippy compiles. It rebuilds it from keys, and four keys have been
    found so far, each by a review pass measuring the previous shape open:

    1. the member directory walked with `rglob("*.rs")`, which misses a source
       that is not under the member,
    2. `targets[].src_path` from `cargo metadata`, which is where a manifest's
       `[lib] path`, `[[bin]] path`, `[[test]] path`, `[[bench]] path` or
       `[[example]] path` puts a compilation root,
    3. `#[path = "..."]` on a module item, followed transitively,
    4. `include!("...")`, which pastes a file in at a point and is named by
       none of the other three.

    Keys 1 to 3 were found in the fifth, seventh and eighth passes and key 4 in
    the ninth. **The docstring here claimed "every `.rs` file a workspace member
    compiles" through all of them, and that sentence was false every time.** It
    is written this way now so the next reader sees the shape of the risk
    rather than a guarantee: the class is "a file clippy compiles that this
    function does not read", and finding a fifth key would surprise nobody.

    **Why rustc's own answer is not the authority here, which is a decision.**
    `target/<profile>/deps/*.d` lists exactly the files each compilation read,
    the `include!`d file and the `#[path]` module among them, MEASURED on the
    crate above as `crates/a/src/lib.rs
    crates/a/src/../../../outside/hidden.rs
    crates/a/src/../../../outside/inner.rs`. It is the right answer and it
    cannot be had cheaply enough to be the one this check depends on:

    - **It exists only after a build, and a stale one narrows in silence.**
      MEASURED: with the crate built clean and the `include!` then added, every
      `.d` still lists `crates/a/src/lib.rs` alone while cargo clippy exits 0
      over the group allow. A guard reading that would print a smaller set and
      pass, which is the exact failure this whole sequence is about.
    - **Freshness cannot be checked by mtime without refusing normal work.**
      Any edit makes every `.d` older than the sources, so a guard refusing on
      that basis refuses on every developer run that follows a keystroke.
    - **Making it fresh means this check builds.** MEASURED in a clone of this
      repository with a warm registry and no `target/`,
      `cargo check --workspace --all-targets` takes 10.8 seconds, and it would
      be paid again inside every `lint-policy` probe, whose sandbox is a fresh
      copy of `git ls-files` and therefore has no `target/` either. The
      `guards` gate compiles nothing today.
    - **A workspace that does not compile produces no dep-info at all**, so
      every compile error would arrive as a lint-policy refusal.

    So the reconstruction stays and it is declared rather than claimed. What
    that costs is stated in the `lint-policy` entry's `limit` in
    `scripts/guards/catalogue.py` and in `docs/lld/guards.md`.

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

    **The TEXT is returned with each path**, so `main` does not open the file a
    second time. It did, with no guard on the second read, and a `.rs` file
    under a member that is a broken symlink or is not UTF-8 gave a raw
    traceback at exit 1 while this function silently carried the same path
    forward as scanned. That is fail-closed by accident in one place and
    fail-OPEN in the other, and one read means one answer: a file that cannot
    be read is a refusal here and appears in no caller's list.

    **The third return value is how many files each FOLLOWED key put on the
    queue**, and it exists because the OK line said "`#[path]` modules
    followed and `include!` followed" as a fixed string. This repository holds
    neither, so two of that line's four clauses described a capability and not
    the run, and a reader had no way to tell a followed key from an unexercised
    one. The count is taken at the queue rather than from the attributes seen,
    so removing the follow reports zero rather than reporting an attribute this
    pass did nothing with.
    """
    found: dict[Path, str] = {}
    seen: set[Path] = set()
    problems: list[str] = []
    # What the two FOLLOWED keys actually reached, counted where the file is
    # put on the queue and nowhere else. The OK line reports these, and it has
    # to be the queue rather than a count of the attributes seen: a version
    # that counted attributes would go on printing "1 `#[path]` module
    # followed" with the following removed, which is the sentence in the
    # language of success that this whole module exists to stop writing.
    #
    # `root` is the same measurement one key over and it is the S03 review's
    # ELEVENTH pass. The OK line said "cargo's own target roots seeded" as an
    # unconditional f-string literal, so `lint-policy.clean-crate-root-outside-
    # the-member`, the accept probe whose whole subject is that seeding, was
    # asserting a sentence the guard prints whether it seeds anything or not:
    # MEASURED, with `for root in roots or []` changed to `for root in []` the
    # probe stayed GREEN. That is the class pass 10 fixed in the two clauses
    # beside it and left standing in the third.
    followed = {"path": 0, "include": 0, "root": 0}
    queue = [path for path in sorted(member.rglob("*.rs"))
             if path.relative_to(member).parts[:1] != ("target",)]
    for root in roots or []:
        if root.is_file():
            queue.append(root)
            followed["root"] += 1
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
        if resolved in seen:
            continue
        seen.add(resolved)
        try:
            source = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as error:
            problems.append(
                f"{_relative(path)} is reached by this check's walk of the "
                f"workspace member `{_relative(member)}` and could not be read "
                f"({error}). A broken symlink or a file that is not UTF-8 is "
                f"still a file clippy may compile, and an unread file is the "
                f"state every measurement in this module's header was taken "
                f"in. It was passed over in silence until the S03 review's "
                f"ninth pass, while `main` read the same path again with no "
                f"guard at all and gave a traceback.")
            continue
        found[resolved] = source
        for named in MODULE_PATH.findall(source):
            candidates = [path.parent / named, path.parent / path.stem / named]
            for candidate in candidates:
                if candidate.is_file():
                    queue.append(candidate)
                    followed["path"] += 1
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
        # `include!` LAST, and read the same way `#[path]` is: by following it
        # where it can be followed and refusing it where it cannot. The
        # resolution rule is rustc's own and it is not `#[path]`'s, so it is
        # written out rather than shared: the argument is relative to the
        # directory of the file the macro is written in, full stop, with no
        # second candidate.
        for match in INCLUDE_ANY.finditer(source):
            named_match = INCLUDE_PATH.match(source, match.start())
            if named_match is None:
                problems.append(
                    f"{_relative(path)} carries "
                    f"`{source[match.start():match.start() + 60].splitlines()[0]}`"
                    f" and this pass could not read a file name out of it. "
                    f"`include!` pastes another file's tokens in at that "
                    f"point, so that file is compiled and is named by no "
                    f"target, no glob of the member and no `#[path]`. "
                    f"MEASURED under the pinned 1.97.1 toolchain: an "
                    f"`include!`d file carrying "
                    f"`#[allow(clippy::pedantic)]` on a module takes cargo "
                    f"clippy from 101 to 0 while this check reads neither "
                    f"file. An argument built by `concat!` or `env!` names a "
                    f"real file that only the build knows, so it is refused "
                    f"rather than passed over. Write the path as a plain "
                    f"string literal, or put the generated code behind a "
                    f"module this check can walk.")
                continue
            candidate = path.parent / named_match.group(1)
            if candidate.is_file():
                queue.append(candidate)
                followed["include"] += 1
                continue
            problems.append(
                f"{_relative(path)} declares "
                f"`include!(\"{named_match.group(1)}\")` and {candidate} is "
                f"not a file, so this pass did not read the source it pastes "
                f"in. rustc resolves that argument against the directory of "
                f"the file the macro is written in, MEASURED from the "
                f"dep-info of a run where it resolved to "
                f"`crates/a/src/../../../outside/hidden.rs`, and a file this "
                f"check never reads is the state an inner or module-level "
                f"group allow was MEASURED to survive at cargo clippy exit 0.")
    return sorted(found.items()), problems, followed


def inherits_workspace_lints(manifest: str) -> tuple[bool, str]:
    """Does this member manifest inherit `[workspace.lints]`, and a parse error.

    TWO spellings, and the guard read one of them until the S03 review's
    eighth pass. `[lints]` with `workspace = true` under it is the common
    form. The top-level dotted `lints.workspace = true` is the same table
    written on one line, it is what several crates in the wild write, and
    MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: with that line
    above `[package]`, cargo clippy exits 101, so the table IS inherited,
    while the previous regex found nothing and this check refused a legitimate
    manifest at exit 1.

    The two spellings are ONE key path to `tomllib`, `lints.workspace`, which
    is the eleventh pass's structural repair: the regex pair this replaced had
    to be told about each spelling, and the quoted third, `[lints]` with
    `"workspace" = true`, was told to neither of them. Same for a trailing
    comment, which the regexes needed a quote-aware line scanner to strip and
    a TOML parser removes by construction.

    A member that writes `lints` under `[package]` really does NOT inherit,
    MEASURED the same way: cargo prints `unused manifest key: package.lints`
    and clippy exits 0. That falls out of the key path here, where the previous
    version needed a hand-rolled "lines before the first table header" scan to
    tell the two apart.
    """
    try:
        document = tomllib.loads(manifest)
    except tomllib.TOMLDecodeError as error:
        return False, str(error)
    lints = document.get("lints")
    return (isinstance(lints, dict) and lints.get("workspace") is True), ""


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


# The two tables HLD 27.1 is about. Named here so `workspace_lints` and the
# declared constant that records its output cannot come to disagree about which
# tables are in scope.
LINT_TABLES = ("clippy", "rust")


def workspace_lints(text: str) -> tuple[dict[str, dict[str, object]], str]:
    """`[workspace.lints.clippy]` and `[workspace.lints.rust]`, PARSED.

    Returns each table's rows exactly as `tomllib` gives them, and a parse
    error, which is the empty string when the document parsed.

    **This is the only reader of these tables in the repository**, and that is
    the eleventh review pass's finding rather than a preference. Five spellings
    of one group row were found in five passes, each one measured to take cargo
    clippy from 101 to 0, and each was answered by another alternation in a
    regex. The quoted key, `"pedantic" = { level = "allow", priority = 1 }`,
    defeated the row regex and the declared constant that was supposed to
    backstop it at the same time. A parser cannot be taught the grammar one
    spelling at a time, so it is not taught, it is imported: TOML's dotted key,
    quoted key, inline table, multi-line row and trailing comment are the same
    document to `tomllib` and are the same code path here.

    A row is returned VERBATIM rather than reduced to a level, because a
    reduction loses `priority`, which is what decides whether a group row
    outranks a named lint. `lint_levels` does the reduction where a level is
    what the caller wants, and `workspace_lints_rows` records the whole row.
    """
    try:
        document = tomllib.loads(text)
    except tomllib.TOMLDecodeError as error:
        return {}, str(error)
    node: object = document
    for key in ("workspace", "lints"):
        node = node.get(key) if isinstance(node, dict) else None
    tables: dict[str, dict[str, object]] = {}
    for name in LINT_TABLES:
        rows = node.get(name) if isinstance(node, dict) else None
        tables[name] = dict(rows) if isinstance(rows, dict) else {}
    return tables, ""


def lint_levels(rows: dict[str, object]) -> dict[str, str]:
    """One lints table reduced to name -> level.

    A row whose level this function cannot read yields the EMPTY STRING, which
    is weaker than every level in `STRENGTH` and is therefore refused rather
    than skipped. That covers a level that is not a string and an inline table
    or dotted group carrying no `level` key at all.
    """
    levels: dict[str, str] = {}
    for name, value in rows.items():
        if isinstance(value, str):
            levels[name] = value
            continue
        level = value.get("level") if isinstance(value, dict) else None
        levels[name] = level if isinstance(level, str) else ""
    return levels


def workspace_lints_rows(text: str) -> str | None:
    """Both lints tables as sorted `table.name = <json>` rows, or `None`.

    `None` when the document does not parse, so the declared-constant ratchet
    in `scripts/guards/catalogue.py` refuses a `Cargo.toml` this reader cannot
    read rather than recording a digest over nothing.

    The recorded value is the PARSED ROWS and not a slice of the file's text,
    which is what makes the ratchet and the guard above agree with the grammar
    instead of with each other. The text slice it replaces was captured by a
    regex that backtracked to the last line starting with a bare key, so a
    quoted key appended after that line was outside the recorded value: cargo
    exit 0, guard exit 0, census exit 0, all three at once. Sorted, so the same
    table written in a different order records the same digest and a row added,
    removed, renamed, re-levelled or re-prioritised anywhere in either table
    moves it.
    """
    tables, error = workspace_lints(text)
    if error:
        return None
    return "\n".join(
        f"{table}.{name} = {json.dumps(value, sort_keys=True)}"
        for table in sorted(tables)
        for name, value in sorted(tables[table].items()))


UNSAFE_GATE_COMMAND = "python3 scripts/unsafe_allowlist_check.py"


def _unsafe_gate_runs_the_script() -> tuple[bool, str]:
    """Does `bin/ocelli.sh`'s `unsafe` arm really run the allowlist check.

    Returns the answer and a refusal for a runner that cannot be read at all,
    which is not the same as an arm that does not run the script and must not
    be reported as one.

    **This was a regex over shell until the S03 review's thirteenth pass, and
    a TRAILING COMMENT satisfied it.** The pattern was
    `^\\s*unsafe\\)[^\\n]*?python3 scripts/unsafe_allowlist_check\\.py` with
    `re.M`, and `[^\\n]*?` reaches a `#` as happily as it reaches a command.
    MEASURED in a real clone, with the arm rewritten to `unsafe)      python3
    scripts/prose_check.py ;;  # python3 scripts/unsafe_allowlist_check.py` and
    the `- run: python3 scripts/unsafe_allowlist_check.py` step deleted from
    `.github/workflows/ci.yml`: `bash -n` 0, this check exit 0 PRINTING
    "unsafe_code denied by scripts/unsafe_allowlist_check.py in the `unsafe`
    gate, which is the declared substitution", `scripts/ci_floor_check.py` exit
    0, the census exit 0 and both unit suites exit 0.
    `scripts/unsafe_allowlist_check.py` then ran nowhere, so HLD 27.1's
    `unsafe_code` deny and HLD 27.2 R5 were enforced by nothing, and the guard
    said so in the language of success.

    **The repair already existed two files over and did not reach this
    caller**, which is the propagation failure the review has now named three
    times. `ci_floor_check.gate_commands` reads the arm through the one shell
    tokenizer this repository has: comments are stripped from the grammar
    rather than by a `[^\\n]*?`, the arm ends at its own `;;` rather than at
    the end of a line, and a `;;` inside a quote or a here-document does not
    end it. `scripts/guards/catalogue.py`'s `_ci_arm_commands` already called
    it, so this is the third caller getting the answer the other two had.

    Imported lazily and by path for the same reason the catalogue gives: this
    guard must not depend on another guard at module scope, and it is run from
    directories that are not `scripts/`.
    """
    import sys as _sys
    if str(ROOT / "scripts") not in _sys.path:
        _sys.path.insert(0, str(ROOT / "scripts"))
    import ci_floor_check
    try:
        runner = RUNNER.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        return False, (
            f"{_relative(RUNNER)} cannot be read as UTF-8 text ({error}). "
            f"Whether the `unsafe` gate still runs "
            f"scripts/unsafe_allowlist_check.py is therefore unknown, and an "
            f"unknown is not an enforcement.")
    try:
        arms = ci_floor_check.gate_commands(runner)
    except RuntimeError as error:
        return False, (
            f"{_relative(RUNNER)} cannot be read for its gate arms "
            f"({error}). Whether the `unsafe` gate still runs "
            f"scripts/unsafe_allowlist_check.py is therefore unknown, and an "
            f"unknown is not an enforcement.")
    return UNSAFE_GATE_COMMAND in arms.get("unsafe", []), ""


def _fail(problems: list[str]) -> int:
    """Print the refusals under the one header this guard has.

    Two callers since the eleventh pass, because an unparseable `Cargo.toml`
    ends the run where it is found rather than falling through the checks that
    would then be reasoning about an empty table.
    """
    print("FAIL: the HLD 27.1 lint policy")
    for problem in problems:
        print(f"  {problem}")
    return 1


def main() -> int:
    # Read under the same FAIL header every other refusal here prints. The
    # call was unguarded until the S03 review's twelfth pass, so a
    # `Cargo.toml` that is not valid UTF-8 arrived as a `UnicodeDecodeError`
    # traceback: fail-closed, and still the wrong way to tell a maintainer
    # what to do, which is the fifth pass's finding in the CI floor check one
    # file over. A missing file is caught here for the same reason, and it is
    # not hypothetical: this guard is run inside disposable clones by
    # `scripts/guard_probe.py`.
    problems: list[str] = []
    try:
        text = CARGO.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        problems.append(
            f"{_relative(CARGO)} cannot be read as UTF-8 text ({error}). "
            f"Whether HLD 27.1's five lints are denied is therefore unknown, "
            f"and an unknown is not a denial. cargo reads a manifest as UTF-8 "
            f"too, so a file this refuses is a file cargo refuses.")
        return _fail(problems)

    tables, toml_error = workspace_lints(text)
    if toml_error:
        # Fail CLOSED, and this branch is new with the eleventh pass. The
        # regexes this replaced returned an empty table for a document they
        # could not read, so an unparseable Cargo.toml would have arrived as
        # five separate "lint is not in the table" refusals naming the wrong
        # problem, and a version of that shape one step weaker would have been
        # silence. It returns here rather than continuing, because every check
        # below would otherwise be reasoning about an empty table.
        problems.append(
            f"{_relative(CARGO)} cannot be parsed as TOML ({toml_error}). "
            f"Whether HLD 27.1's five lints are denied is therefore unknown, "
            f"and an unknown is not a denial. This check reads the lint "
            f"tables with `tomllib` rather than with a regex, so a document "
            f"cargo accepts and this parser rejects is a real disagreement "
            f"about the grammar and needs a person.")
        return _fail(problems)
    clippy = lint_levels(tables["clippy"])
    rust = lint_levels(tables["rust"])

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
    gate, gate_problem = _unsafe_gate_runs_the_script()
    if gate_problem:
        problems.append(gate_problem)
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
        inherits, manifest_error = inherits_workspace_lints(manifest)
        if manifest_error:
            # Fail CLOSED on a member manifest `tomllib` cannot read, on the
            # same argument as the workspace document above. The regex pair
            # this replaced answered "does not inherit" for an unreadable
            # manifest, which is the right DIRECTION and the wrong sentence:
            # the refusal named a missing table rather than a document nobody
            # could parse.
            problems.append(
                f"{where} is a workspace member cargo reports and its "
                f"Cargo.toml cannot be parsed as TOML ({manifest_error}). "
                f"Whether it inherits HLD 27.1's table is therefore unknown, "
                f"and an unknown is not an inheritance.")
            continue
        if not inherits:
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
    # What the two FOLLOWED keys actually found in this run. The OK line named
    # all four keys in a fixed string, so it read as though every one had been
    # exercised while this repository contains no `#[path]` and no `include!`
    # at all: two of its four clauses described a capability and not the run,
    # which is the shape of claim the whole `member_sources` docstring exists
    # to stop making. The S03 review's tenth pass raised it, in the same pass
    # that found `MODULE_PATH` blind to two spellings of key three, so "four
    # keys" was doubly optimistic. `member_sources` counts them where it puts
    # the file on the QUEUE, so a zero says zero and a key that stopped being
    # followed reports zero rather than reporting the attribute it saw and did
    # nothing with.
    module_paths = 0
    includes = 0
    seeded_roots = 0
    for member in members:
        sources, source_problems, followed = member_sources(
            member, roots.get(member))
        problems += source_problems
        module_paths += followed["path"]
        includes += followed["include"]
        seeded_roots += followed["root"]
        # The text comes back with the path. `main` read every file a second
        # time until the S03 review's ninth pass, with no guard on the read, so
        # a `.rs` file under a member that is a broken symlink or is not UTF-8
        # gave a raw traceback at exit 1 rather than a refusal under the FAIL
        # header, which is the presentation the eighth pass had just stopped
        # giving four lines of reasoning earlier in this same file.
        for path, source in sources:
            scanned += 1
            where = _relative(path)
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
        return _fail(problems)

    # Every number here is derived from the walk that produced it. The member
    # count was the literal `crates/` glob until the fifth pass, and it read
    # thirteen while cargo built fourteen. Where the set CAME FROM is printed
    # since the seventh pass, because the globs and cargo's answer are two
    # different sets and the difference was measured at one crate.
    #
    # The two FOLLOWED keys are counted rather than named. They were a fixed
    # string until the S03 review's tenth pass, so the line read as though all
    # four keys had been exercised over a repository holding no `#[path]` and
    # no `include!`, and a run that followed nothing said so in the language of
    # having followed things.
    #
    # The SEEDED ROOTS are counted for the same reason since the eleventh pass,
    # and the reason is sharper here, because that clause is the whole subject
    # of an accept probe. `lint-policy.clean-crate-root-outside-the-member`
    # expected the literal words and passed with the seeding deleted, MEASURED.
    # It expects the count now, and the count is taken where the root is put on
    # the queue.
    configs = _cargo_configs()
    print(f"OK: {len(REQUIRED_CLIPPY)} clippy lint(s) at or above HLD 27.1's "
          f"level and no group row weaker than deny, {len(members)} workspace "
          f"member(s) from {member_source} inherit the table, "
          f"{scanned} .rs file(s), a set RECONSTRUCTED from four keys with "
          f"the member globbed, {seeded_roots} cargo target root(s) seeded, "
          f"{module_paths} `#[path]` module(s) followed and {includes} "
          f"`include!`(s) followed, carry no inner "
          f"allow or expect of a denied lint or of a group holding one, and "
          f"none on a `mod` item, {len(configs)} cargo config(s) lower no "
          f"denied lint through rustflags, unsafe_code denied by {unsafe_by}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
