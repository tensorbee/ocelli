#!/usr/bin/env python3
"""No `unsafe` outside the allow-list. HLD section 27.2 R5.

    "R5. No unsafe outside the allow-list (ocelli-wasm/src/ring.rs,
     ocelli-core/src/cast.rs). Keeps the audit surface to two files."

Two files, named in the specification. The point is not that unsafe is
forbidden, it is that a human reviewing this project for a device submission
should have to read two files to audit every unsafe line in it.

Neither file has to exist yet. The allow-list is a permission, not a
requirement. What it refuses is a THIRD file.

## The scan is a tokenizer, not a strip-and-search

F-229's reviews planted an `unsafe` block past five shapes the string regex
answered by blanking code: a char literal holding a quote, the same escaped,
a raw string ending in a backslash, a raw string holding a quote, and a
line comment holding an unpaired quote. The nested block comment
`/* /* unsafe */ unsafe */` was reported the other way, as code, because the
block-comment regex stopped at the first `*/`. One left-to-right pass
replaces all three regexes. It recognises line comments, doc comments,
nested block comments, string literals, byte strings, C strings, raw
strings of any hash count in their byte and C forms, character and byte
literals, lifetimes and labels, and raw identifiers, and reports `unsafe`
only as a keyword token: `r#unsafe` is an identifier and is not reported.
The token is reported inside an attribute too, where `#[unsafe(no_mangle)]`
is edition 2024's spelling and rustc refuses it. An unterminated literal,
comment or attribute is a refusal naming the file and the line the
construct starts on, never a blank to the end of the file.

It also refuses an attribute that weakens `unsafe_code`: an allow, an
expect or a warn of the lint, inner or outer, in every tracked Rust file
outside `vendor/`. The workspace table is `forbid` since F-232, so rustc
refuses those too, and this refusal is what keeps the trybuild files,
which no lint table reaches, honest about the same rule. The vendored
packages are audited by their recorded counts instead, and an allow
inside one lowers nothing because no lint table reaches them.

A doc comment's `rust` tagged and untagged code fences are read as code,
recursively: rustdoc compiles and runs them as doctests, `cargo test`
reaches them, and no lint table does. Fences tagged `text` or another
language are comment text.

## The first story that writes an allowed file

The two allowed files do not exist yet, so nothing carries an exception
today. The story that writes `ring.rs` or `cast.rs` makes this recorded
sequence by design, from F-232's plan: move the workspace table to `deny`,
add `#![forbid(unsafe_code)]` to every other crate root, put
`#[allow(unsafe_code)]` on that one `mod` declaration, and teach
`scripts/lint_policy_check.py` the one exception. Nothing here needs
editing for it, because the allow-list is a permission and not a
requirement.

Usage:
  python3 scripts/unsafe_allowlist_check.py
  python3 scripts/unsafe_allowlist_check.py --staged
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# HLD section 27.2 R5, verbatim. Adding a path here is a design-plan decision
# with a recorded rationale, reviewed like code. It is not a convenience.
ALLOWED = {
    "crates/ocelli-wasm/src/ring.rs",
    "crates/ocelli-core/src/cast.rs",
}

# Vendored third-party packages, which are dependencies that happen to be in
# the tree rather than code this repository writes.
#
# **They are not simply excluded, and the reason is R5's own sentence.** R5's
# payoff is that a device-submission reviewer reads two files to audit every
# unsafe line, and a vendored package with unsafe in it makes that false
# whether the package is tracked or resolved from a registry. Excluding
# `vendor/` silently would leave R5 passing mechanically while its stated
# purpose was weakened, which is the shape `docs/spikes/A2-jpeg-ls.md` already
# warned about for the `charls` routes.
#
# So the count is RECORDED rather than ignored. A vendored package with no
# record is refused, and a package whose count moves is refused, so the number
# lands in front of a reviewer in the diff that changes it. The audit itself,
# which files and which constructs, is in `docs/SOURCE-POLICY.md`. The count
# is the number of `unsafe` keyword tokens, re-derived by the tokenizer on
# every run and never read back from here.
VENDOR_ROOT = "vendor/"
VENDORED_UNSAFE = {
    # D-20. The published package contains no unsafe Rust at all.
    "ritk-codecs-0.6.0": 0,
    # D-22. 104 constructs in nine files, of which the scalar memory, wavelet
    # and colour paths are the ones reachable on wasm32. The architecture SIMD
    # files hold the rest and are compiled only for x86_64 and aarch64.
    "openjph-core-0.1.0": 104,
}

# The attribute levels that weaken `unsafe_code`, and the lint they name.
# `deny` and `forbid` raise a level and are never a refusal here.
WEAKENING_LEVELS = ("allow", "expect", "warn")
WEAKENED_LINT = "unsafe_code"

# The prefixes Rust lexes before a quote. An `r`-suffixed prefix opens a raw
# string, `b` and `c` open byte and C strings, and `b` before a quote opens a
# byte literal.
RAW_PREFIXES = ("r", "br", "cr")
STRING_PREFIXES = ("r", "b", "br", "c", "cr")


class LexError(Exception):
    """A file that does not tokenize, with the line the construct starts on.

    Refusing is the fail-closed direction. The regexes this replaced answered
    an unterminated literal by blanking to the end of the file, which is
    exactly how an `unsafe` block after one went unseen.
    """

    def __init__(self, line: int, message: str) -> None:
        super().__init__(f"line {line}: {message}")
        self.line = line
        self.message = message


def _ident_at(text: str, start: int) -> int:
    """The index just past the identifier that begins at `start`."""
    end = start
    while end < len(text) and (text[end].isalnum() or text[end] == "_"):
        end += 1
    return end


def _scan_string(text: str, i: int, line: int) -> tuple[int, int]:
    """Consume a string literal. `text[i]` is its opening quote.

    Returns the index and line just past the closing quote. Escapes are
    skipped two characters at a time, which is what makes a backslash before
    a newline a line continuation rather than a terminator.
    """
    n = len(text)
    start_line = line
    i += 1
    while i < n:
        c = text[i]
        if c == '"':
            return i + 1, line
        if c == "\\":
            if i + 1 >= n:
                break
            if text[i + 1] == "\n":
                line += 1
            i += 2
            continue
        if c == "\n":
            line += 1
        i += 1
    raise LexError(start_line, "unterminated string literal")


def _scan_raw_string(text: str, i: int, hashes: int,
                     line: int) -> tuple[int, int]:
    """Consume a raw string. `text[i]` is its opening quote.

    `hashes` is the count in the opening delimiter, which is part of the
    delimiter and not decoration: `r#"a " b"#` holds a quote and
    `br##"x "# y"##` holds one and ends in one. A closing quote followed by
    MORE hashes than the opening is refused, as rustc 1.97.1 refuses it
    with "too many `#` when terminating raw string", rather than read as
    a terminator or as body text.
    """
    n = len(text)
    start_line = line
    i += 1
    while i < n:
        c = text[i]
        if c == '"':
            j = i + 1
            while j < n and text[j] == "#":
                j += 1
            found = j - i - 1
            if found == hashes:
                return j, line
            if found > hashes:
                raise LexError(start_line,
                               "too many `#` when terminating raw string")
            i += 1
            continue
        if c == "\n":
            line += 1
        i += 1
    raise LexError(start_line, "unterminated raw string literal")


def _scan_quote(text: str, i: int, line: int) -> tuple[int, int]:
    """Consume a char or byte literal, or a lifetime or a label.

    `text[i]` is the quote. The disambiguation is rustc's own: an identifier
    followed by another quote is a char literal, and an identifier followed
    by anything else is a lifetime or a label, so `'a` ends at the
    identifier and does not blank the line after it.
    """
    n = len(text)
    j = i + 1
    if j >= n:
        raise LexError(line, "unterminated character literal")
    c = text[j]
    if c == "\\":
        j += 1
        if j >= n:
            raise LexError(line, "unterminated character literal")
        escape = text[j]
        if escape == "x":
            j += 3  # the x and its two hex digits
        elif escape == "u":
            close = text.find("}", j)
            if close == -1:
                raise LexError(line, "unterminated character literal")
            j = close + 1  # past the closing brace, onto the quote
        else:
            j += 1
        if j < n and text[j] == "'":
            return j + 1, line
        raise LexError(line, "unterminated character literal")
    if c.isalpha() or c == "_":
        j = _ident_at(text, j)
        if j < n and text[j] == "'":
            return j + 1, line
        return j, line  # 'a, 'static, '_
    if c == "'" or j + 1 >= n:
        # `''` is no char literal rustc accepts, and neither is a lone quote
        # holding nothing at end of file.
        raise LexError(line, "unterminated character literal")
    if text[j + 1] == "'":
        return j + 2, line  # one non-identifier character, such as '"'
    raise LexError(line, "unterminated character literal")


def _is_doc_line(text: str, i: int) -> bool:
    """Whether `text[i:]` starts a `///` or `//!` doc comment line.

    `////` and longer are divider comments, which rustc lexes as plain
    comments and rustdoc does not read as documentation.
    """
    n = len(text)
    if not (i + 2 < n and text[i] == "/" and text[i + 1] == "/"):
        return False
    third = text[i + 2]
    fourth = text[i + 3] if i + 3 < n else ""
    return third == "!" or (third == "/" and fourth != "/")


def _is_doc_block(text: str, i: int) -> bool:
    """Whether `text[i:]` starts a `/** ... */` or `/*! ... */` doc comment.

    `/**/` is the empty plain comment and `/***` opens a plain comment,
    which is how rustc lexes them and rustdoc reads none of them.
    """
    n = len(text)
    if not (i + 1 < n and text[i] == "/" and text[i + 1] == "*"):
        return False
    third = text[i + 2] if i + 2 < n else ""
    if third == "!":
        return True
    if third != "*":
        return False
    fourth = text[i + 3] if i + 3 < n else ""
    return fourth != "/" and fourth != "*"


def _skip_trivia(text: str, i: int, line: int) -> tuple[int, int]:
    """Past whitespace and plain comments, which Rust allows between tokens.

    Stops on a doc comment (`///`, `//!`, `/**`, `/*!`) for the caller to
    read, and on any other token. An unterminated block comment raises
    `LexError` naming the line it starts on.
    """
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
        elif c in " \t\r":
            i += 1
        elif c == "/":
            nxt = text[i + 1] if i + 1 < n else ""
            if nxt == "/":
                if _is_doc_line(text, i):
                    break  # a doc comment, which the caller reads
                # `//` ends at the newline. The quote of a `// say "`
                # comment is text and opens nothing.
                end = text.find("\n", i + 2)
                i = n if end == -1 else end
            elif nxt == "*":
                if _is_doc_block(text, i):
                    break  # a doc comment, which the caller reads
                i, line, _body = _block_comment(text, i, line)
            else:
                break
        else:
            break
    return i, line


def _block_comment(text: str, i: int, line: int) -> tuple[int, int, str]:
    """Consume one block comment. `text[i]` is its opening `/`.

    Block comments nest, so `/* /* unsafe */ unsafe */` is one comment and
    neither half is code. Returns the index and line just past the closing
    `*/`, and the body between the delimiters. An unterminated comment
    raises `LexError` naming the line it starts on.
    """
    n = len(text)
    start_line = line
    body_start = i + 2
    depth = 1
    i += 2
    while depth and i < n:
        if text[i] == "/" and i + 1 < n and text[i + 1] == "*":
            depth += 1
            i += 2
        elif text[i] == "*" and i + 1 < n and text[i + 1] == "/":
            depth -= 1
            i += 2
        else:
            if text[i] == "\n":
                line += 1
            i += 1
    if depth:
        raise LexError(start_line, "unterminated block comment")
    return i, line, text[body_start:i - 2]


def _fence_code(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    """The (start line, code) of every Rust fence in one doc comment.

    `lines` pairs each content line of the comment with its number in the
    file. A fence tagged `rust` or untagged is what rustdoc compiles and
    runs as a doctest, so its content is read as code. A fence tagged
    `text` or any other language is comment text, and its lines are still
    tracked so that they cannot open a fence of their own. A fence still
    open when the comment ends runs to its end, as markdown closes it.
    """
    blocks: list[tuple[int, str]] = []
    open_fence: tuple[int, int, bool, list[str]] | None = None
    for abs_line, content in lines:
        body = content.lstrip()
        if open_fence is None:
            if body.startswith("```"):
                ticks = len(body) - len(body.lstrip("`"))
                info = body[ticks:].strip()
                lang = info.replace(",", " ").split()[0] if info else ""
                open_fence = (ticks, abs_line, lang in ("", "rust"), [])
        else:
            ticks, start, is_rust, buffer_ = open_fence
            seen = len(body) - len(body.lstrip("`"))
            if seen >= ticks and body[seen:].strip() == "":
                # The opener line is the fence marker and its info string,
                # never code, so the content starts on the next line.
                if is_rust and buffer_:
                    blocks.append((start + 1, "\n".join(buffer_)))
                open_fence = None
            else:
                buffer_.append(content)
    if open_fence is not None:
        ticks, start, is_rust, buffer_ = open_fence
        if is_rust and buffer_:
            blocks.append((start + 1, "\n".join(buffer_)))
    return blocks


def _fence_hits(blocks: list[tuple[int, str]]) -> tuple[list[int], list[int]]:
    """Scan each fence's code and map its lines back onto the file's."""
    unsafe: list[int] = []
    weakening: list[int] = []
    for start, code in blocks:
        more_unsafe, more_weakening = scan(code)
        unsafe.extend(start - 1 + hit for hit in more_unsafe)
        weakening.extend(start - 1 + hit for hit in more_weakening)
    return unsafe, weakening


def _doc_line_fences(text: str, i: int,
                     line: int) -> tuple[int, int, list[int], list[int]]:
    """Read the Rust code fences of one doc comment block as code.

    `text[i]` is the first `/` of a `///` or `//!` line. The block is the
    consecutive doc comment lines from here, which rustdoc reads as one
    document. Returns the index and line just past the block, then the
    absolute `unsafe` and weakening line numbers of its `rust` tagged and
    untagged fences.
    """
    n = len(text)
    lines: list[tuple[int, str]] = []
    while True:
        end = text.find("\n", i)
        lines.append((line, text[i + 3:] if end == -1 else text[i + 3:end]))
        if end == -1:
            i = n
            break
        i = end + 1
        line += 1
        j = i
        while j < n and text[j] in " \t\r":
            j += 1
        if not _is_doc_line(text, j):
            break
    return i, line, *_fence_hits(_fence_code(lines))


def _doc_block_fences(text: str, i: int,
                      line: int) -> tuple[int, int, list[int], list[int]]:
    """Read the Rust code fences of one `/** ... */` or `/*! ... */` doc
    comment as code.

    `text[i]` is the opening `/`. The comment is skipped with nesting, and
    its body lines get the `* ` unindent rustdoc applies to block doc
    lines. Returns the index and line just past the comment, then the
    absolute `unsafe` and weakening line numbers of its fences.
    """
    start_line = line
    i, line, body = _block_comment(text, i, line)
    lines: list[tuple[int, str]] = []
    for offset, raw in enumerate(body.split("\n")):
        content = raw.lstrip()
        if content.startswith("*"):
            content = content[1:]
        lines.append((start_line + offset, content))
    return i, line, *_fence_hits(_fence_code(lines))


def scan(text: str) -> tuple[list[int], list[int]]:
    """One left-to-right pass over Rust source.

    Returns the line numbers of `unsafe` keyword tokens and of attributes
    weakening `unsafe_code`. Raises `LexError` for an unterminated literal,
    comment or attribute. Comments and literals are skipped whole rather
    than blanked, so nothing after one is ever read as code or the reverse.

    `unsafe` is reported inside an attribute too: `#[unsafe(no_mangle)]` is
    edition 2024's spelling and rustc refuses it. The `rust` tagged and
    untagged code fences of doc comments are read as code, recursively,
    because rustdoc compiles and runs them as doctests and no lint table
    reaches them.
    """
    unsafe_lines: list[int] = []
    weakening: list[int] = []
    n = len(text)
    i = 0
    line = 1
    # Square brackets entered by `#[` or `#![` and every bracket inside them.
    # Identifiers lexed inside are the attribute's lint names, and strings and
    # comments inside are skipped by the same rules as anywhere else, so
    # `reason = "..."` text cannot name a lint.
    attr_depth = 0
    attr_line = 0
    attr_names: list[str] = []
    while i < n:
        i, line = _skip_trivia(text, i, line)
        if i >= n:
            break
        c = text[i]
        if c == "/":
            # The only comments left here are doc comments, which
            # _skip_trivia stops on. Their `rust` and untagged fences are
            # code, and their hits are file lines already.
            nxt = text[i + 1] if i + 1 < n else ""
            if nxt == "/":
                i, line, doc_unsafe, doc_weakening = \
                    _doc_line_fences(text, i, line)
                unsafe_lines.extend(doc_unsafe)
                weakening.extend(doc_weakening)
            elif nxt == "*":
                i, line, doc_unsafe, doc_weakening = \
                    _doc_block_fences(text, i, line)
                unsafe_lines.extend(doc_unsafe)
                weakening.extend(doc_weakening)
            else:
                i += 1  # a lone slash is punctuation, not a comment
        elif c == '"':
            i, line = _scan_string(text, i, line)
        elif c == "'":
            i, line = _scan_quote(text, i, line)
        elif c == "#":
            # `#`, an optional `!` and `[` are three tokens, and Rust allows
            # whitespace and comments between any two of them, so
            # `# [allow(unsafe_code)]` and `#/**/[allow(unsafe_code)]` are
            # spellings of the attribute the weakening refusal must find.
            # The line counter is only committed when the `[` follows,
            # because the trivia is re-read otherwise.
            j, probe_line = _skip_trivia(text, i + 1, line)
            if j < n and text[j] == "!":
                j, probe_line = _skip_trivia(text, j + 1, probe_line)
            if j < n and text[j] == "[":
                if attr_depth == 0:
                    attr_names = []
                    attr_line = line
                attr_depth += 1
                i = j + 1
                line = probe_line
            else:
                i += 1
        elif c == "[":
            if attr_depth:
                attr_depth += 1
            i += 1
        elif c == "]":
            if attr_depth:
                attr_depth -= 1
                if attr_depth == 0 and WEAKENED_LINT in attr_names \
                        and any(name in attr_names
                                for name in WEAKENING_LEVELS):
                    weakening.append(attr_line)
            i += 1
        elif c.isalpha() or c == "_":
            end = _ident_at(text, i)
            word = text[i:end]
            nxt = text[end] if end < n else ""
            if word in RAW_PREFIXES and nxt == "#":
                hashes_end = end
                while hashes_end < n and text[hashes_end] == "#":
                    hashes_end += 1
                if hashes_end < n and text[hashes_end] == '"':
                    i, line = _scan_raw_string(
                        text, hashes_end, hashes_end - end, line)
                else:
                    # `r#name` is a raw identifier. Its name is never a
                    # keyword, so the whole token is consumed here.
                    i = _ident_at(text, hashes_end)
            elif word in STRING_PREFIXES and nxt == '"':
                if word.endswith("r"):
                    i, line = _scan_raw_string(text, end, 0, line)
                else:
                    i, line = _scan_string(text, end, line)
            elif word == "b" and nxt == "'":
                i, line = _scan_quote(text, end, line)
            elif word == "unsafe":
                # Inside an attribute too: `#[unsafe(no_mangle)]` and
                # `#[cfg_attr(x, unsafe(no_mangle))]` are `unsafe` code
                # rustc refuses, so the token is reported wherever it is.
                unsafe_lines.append(line)
            elif attr_depth:
                attr_names.append(word)
            i = max(i, end)
        else:
            i += 1
    if attr_depth:
        # An attribute still open at end of file is a lexically broken
        # file. Skipping it silently would hide every `unsafe` after the
        # unclosed `#[`, which is the blank-to-EOF shape this scanner
        # exists to refuse.
        raise LexError(attr_line, "unterminated attribute")
    return unsafe_lines, weakening


def rust_files(staged: bool) -> list[Path]:
    # The complete index includes unchanged vendored packages. A changed-name
    # subset cannot establish the recorded dependency audit surface.
    out = subprocess.run(["git", "ls-files", "-z"], cwd=ROOT,
                         capture_output=True, text=True, check=True).stdout
    return [ROOT / n for n in out.split("\0")
            if n.endswith(".rs") and (staged or (ROOT / n).is_file())]



def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--staged", action="store_true")
    args = parser.parse_args()

    problems = []
    checked = 0
    vendored: dict[str, int] = {}
    try:
        paths = rust_files(args.staged)
    except (subprocess.CalledProcessError, OSError):
        print("FAIL: Rust source inventory is unavailable")
        return 1
    for path in paths:
        rel = path.relative_to(ROOT).as_posix()
        if rel in ALLOWED:
            continue
        try:
            text = (subprocess.run(["git", "show", f":{rel}"], cwd=ROOT,
                                   capture_output=True, text=True, check=True).stdout
                    if args.staged else path.read_text(encoding="utf-8"))
        except (UnicodeDecodeError, OSError, subprocess.CalledProcessError):
            problems.append(f"{rel}: source is unreadable")
            continue
        try:
            unsafe_lines, weakening = scan(text)
        except LexError as error:
            problems.append(f"{rel}:{error.line}: {error.message}")
            continue
        if rel.startswith(VENDOR_ROOT):
            package = rel[len(VENDOR_ROOT):].split("/", 1)[0]
            vendored[package] = vendored.get(package, 0) + len(unsafe_lines)
            continue
        checked += 1
        for line in unsafe_lines:
            problems.append(f"{rel}:{line}: `unsafe` outside the allow-list")
        for line in weakening:
            problems.append(
                f"{rel}:{line}: `unsafe_code` lowered by an allow, expect "
                f"or warn attribute")

    for package in sorted(set(vendored) | set(VENDORED_UNSAFE)):
        found = vendored.get(package)
        recorded = VENDORED_UNSAFE.get(package)
        if recorded is None:
            problems.append(
                f"vendored package {package} has {found} `unsafe` construct(s) "
                f"and no recorded count. A vendored package is a dependency "
                f"this repository ships, so its audit surface is recorded in "
                f"VENDORED_UNSAFE and in docs/SOURCE-POLICY.md rather than "
                f"being excluded silently")
        elif found is None:
            problems.append(
                f"vendored package {package} is recorded with {recorded} "
                f"`unsafe` construct(s) and is not in the tree. A record "
                f"nothing reads is not a ratchet")
        elif found != recorded:
            problems.append(
                f"vendored package {package} has {found} `unsafe` "
                f"construct(s), recorded {recorded}. Re-audit it, update "
                f"docs/SOURCE-POLICY.md, and move the number in the same diff")

    if problems:
        print("FAIL: `unsafe` outside the allow-list (HLD section 27.2 R5)")
        for problem in problems:
            print(f"  {problem}")
        print("\nThe allow-list is:")
        for path in sorted(ALLOWED):
            print(f"  {path}")
        print("\nAdding to it is a design-plan decision with a recorded")
        print("rationale, not an edit to this script made to get a build green.")
        return 1

    audit = ", ".join(f"{package} {count}"
                      for package, count in sorted(vendored.items()))
    print(f"OK: no unsafe outside the allow-list ({checked} files checked, "
          f"{len(ALLOWED)} permitted)"
          + (f", vendored audit surface unchanged: {audit}" if vendored else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
