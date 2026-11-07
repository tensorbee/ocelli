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

Every line of doc text is scanned as code, fail closed: the content of
`///` and `//!` lines, of `/** ... */` and `/*! ... */` comments, and of
a `#[doc = ...]` or `#![doc = ...]` attribute's string. Outside a fence
the word is reported only where it is followed by `{`, `fn`, `impl`,
`trait` or `extern`, the forms that compile, and prose that only names
the word is not a finding. Inside a fence every `unsafe` keyword token
is reported, and a fence is skipped only when its info string carries a
word rustdoc does not recognise as a Rust attribute and no `rust` word,
the rule cited at `_fence_is_rust`. Backtick and tilde fences are both
read, and a fence still open when the doc text ends runs to its end, as
markdown closes it.

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


# rustdoc's own documentation for the pinned toolchain, the page the fence
# rule below is copied from. The recognised attribute words are its
# Attributes, Ignoring targets and Custom CSS classes sections, with
# `test_harness` and the four digit error codes taken from the same
# release's LangString parse in src/librustdoc/html/markdown.rs.
RUSTDOC_DOC_PAGE = ("https://doc.rust-lang.org/1.97.1/rustdoc/"
                    "write-documentation/documentation-tests.html")
RUSTDOC_WORDS = ("rust", "should_panic", "no_run", "ignore", "test_harness",
                 "compile_fail", "standalone_crate", "custom")


def _info_words(info: str) -> list[str]:
    """The attribute words of a fence info string.

    A `{...}` group is a CSS class attribute, the Custom CSS classes form
    on the page RUSTDOC_DOC_PAGE cites, and is never an unknown word, so
    brace groups are dropped before the split on whitespace and commas.
    """
    outside: list[str] = []
    depth = 0
    for ch in info:
        if ch == "{":
            depth += 1
        elif ch == "}" and depth:
            depth -= 1
        elif depth == 0:
            outside.append(ch)
    return "".join(outside).replace(",", " ").split()


def _attribute_word(token: str) -> str:
    """A token's word: its quoted value, minus a `(...)` suffix.

    rustdoc's tag iterator reads a quoted segment as one word and skips a
    parenthesised suffix, so `should_panic(the message)` and `"not rust"`
    carry the words `should_panic` and `not rust`.
    """
    if len(token) >= 2 and token[0] == '"' and token[-1] == '"':
        token = token[1:-1]
    if token.endswith(")") and "(" in token:
        token = token[:token.index("(")]
    return token


def _is_error_code(word: str) -> bool:
    """An `E` followed by exactly four digits, the error code form."""
    return (len(word) == 5 and word[0] == "E" and word[1:].isascii()
            and word[1:].isdigit())


def _fence_is_rust(info: str) -> bool:
    """Whether rustdoc would compile a fence carrying this info string.

    The rule is rustdoc's own, at RUSTDOC_DOC_PAGE for the pinned
    toolchain: unknown attributes are treated as markers for code samples
    written in other programming languages, unless the info string also
    carries a `rust` word. A fence is skipped only when its info string
    carries a word outside the recognised list and no `rust` word.
    `ignore` is a recognised word, so an `ignore` fence is still scanned:
    rustdoc reads it as a Rust code block it chooses not to compile, and
    the fail-closed direction reports it.
    """
    words = [_attribute_word(token) for token in _info_words(info)]
    return "rust" in words or all(
        word in RUSTDOC_WORDS
        or word.startswith("ignore-")
        or word.startswith("edition")
        or _is_error_code(word)
        for word in words)


def _fence_open(body: str) -> tuple[int, str, str] | None:
    """The (fence length, fence character, info string) a line opens.

    A fence opens on three or more backticks or tildes, and its info
    string is what follows them. A backtick fence's info string cannot
    hold a backtick: such a line is inline code and not a fence at all,
    so it is left for the prose scan.
    """
    char = body[:1]
    if char not in ("`", "~"):
        return None
    ticks = len(body) - len(body.lstrip(char))
    if ticks < 3:
        return None
    info = body[ticks:].strip()
    if char == "`" and "`" in info:
        return None
    return ticks, char, info


def _fence_closes(body: str, char: str, ticks: int) -> bool:
    """Whether a fence body line closes a fence of `ticks` `char` marks.

    A closing fence is at least as long as its opener and holds nothing
    else, and backticks never close a tilde fence nor the reverse.
    """
    seen = len(body) - len(body.lstrip(char))
    return seen >= ticks and body[seen:].strip() == ""


def _fence_code(
    lines: list[tuple[int, str]],
) -> tuple[list[list[tuple[int, str]]], list[tuple[int, str]]]:
    """Every Rust fence's (file line, code line) pairs, and the prose.

    `lines` pairs each content line of the file's doc text with its
    number in the file. Backtick and tilde fences are both read, and a
    fence's Rust-ness is `_fence_is_rust` of its info string: a Rust
    fence's lines are returned as one block for the scanner to read as
    code, a non-Rust fence's lines are dropped, and a line outside any
    fence is returned as prose for the fail-closed doc text rule. The
    opener line is the fence marker and its info string, never code, so
    the content starts on the next line. A fence still open when the doc
    text ends runs to its end, as markdown closes it.
    """
    blocks: list[list[tuple[int, str]]] = []
    prose: list[tuple[int, str]] = []
    open_fence: tuple[int, str, bool, list[tuple[int, str]]] | None = None
    for abs_line, content in lines:
        body = content.lstrip()
        if open_fence is None:
            fence = _fence_open(body)
            if fence is None:
                prose.append((abs_line, content))
            else:
                ticks, char, info = fence
                open_fence = (ticks, char, _fence_is_rust(info), [])
        else:
            ticks, char, is_rust, buffer_ = open_fence
            if _fence_closes(body, char, ticks):
                if is_rust and buffer_:
                    blocks.append(buffer_)
                open_fence = None
            else:
                buffer_.append((abs_line, content))
    if open_fence is not None:
        _ticks, _char, is_rust, buffer_ = open_fence
        if is_rust and buffer_:
            blocks.append(buffer_)
    return blocks, prose


def _fence_hits(
    blocks: list[list[tuple[int, str]]],
) -> tuple[list[int], list[int]]:
    """Scan each Rust fence's code and map its lines back onto the file's."""
    unsafe: list[int] = []
    weakening: list[int] = []
    for block in blocks:
        code = "\n".join(content for _line, content in block)
        try:
            more_unsafe, more_weakening = scan(code)
        except LexError as error:
            raise LexError(block[error.line - 1][0], error.message) from None
        unsafe.extend(block[hit - 1][0] for hit in more_unsafe)
        weakening.extend(block[hit - 1][0] for hit in more_weakening)
    return unsafe, weakening


def _prose_unsafe(content: str) -> bool:
    """Whether a doc text line holds `unsafe` in a form that compiles.

    The fail-closed rule from F-232's plan: prose is not code, so the word
    alone is not a finding, but `unsafe` followed by `{`, `fn`, `impl`,
    `trait` or `extern` is the shape of an item or block rustdoc would
    compile, and is reported. Only spaces and tabs are skipped between the
    word and its form, and the word must be a whole word.
    """
    i = 0
    while True:
        i = content.find("unsafe", i)
        if i == -1:
            return False
        end = i + len("unsafe")
        bounded = (i == 0 or not (content[i - 1].isalnum()
                                  or content[i - 1] == "_")) \
            and (end == len(content) or not (content[end].isalnum()
                                             or content[end] == "_"))
        if bounded:
            j = end
            while j < len(content) and content[j] in " \t":
                j += 1
            if j < len(content):
                if content[j] == "{":
                    return True
                k = _ident_at(content, j)
                if k > j and content[j:k] in ("fn", "impl", "trait",
                                              "extern"):
                    return True
        i = end


def _doc_line_text(text: str, i: int, line: int,
                   doc: list[tuple[int, str]]) -> tuple[int, int]:
    """Collect one block of `///` or `//!` doc lines as doc text.

    `text[i]` is the first `/` of a `///` or `//!` line. The block is the
    consecutive doc comment lines from here, which rustdoc reads as one
    document. Each line's content and file line are appended to `doc` for
    the fence rule and the fail-closed prose scan. Returns the index and
    line just past the block.
    """
    n = len(text)
    while True:
        end = text.find("\n", i)
        doc.append((line, text[i + 3:] if end == -1 else text[i + 3:end]))
        if end == -1:
            return n, line
        i = end + 1
        line += 1
        j = i
        while j < n and text[j] in " \t\r":
            j += 1
        if not _is_doc_line(text, j):
            return i, line


def _doc_block_text(text: str, i: int, line: int,
                    doc: list[tuple[int, str]]) -> tuple[int, int]:
    """Collect one `/** ... */` or `/*! ... */` comment as doc text.

    `text[i]` is the opening `/`. The comment is skipped with nesting, and
    its body lines get the `* ` unindent rustdoc applies to block doc
    lines. Each body line's content and file line are appended to `doc`.
    Returns the index and line just past the comment.
    """
    start_line = line
    i, line, body = _block_comment(text, i, line)
    for offset, raw in enumerate(body.split("\n")):
        content = raw.lstrip()
        if content.startswith("*"):
            content = content[1:]
        doc.append((start_line + offset, content))
    return i, line


def _plain_doc_lines(text: str, start: int,
                     line: int) -> list[tuple[int, str]]:
    """The doc text lines of a `#[doc = "..."]` string value.

    `text[start]` is the opening quote. A `\n` escape in the value is the
    line break rustdoc sees, so the value is split the same way, and every
    line is attributed to the line the string starts on. Other escapes are
    left as written: no rule reads them.
    """
    n = len(text)
    i = start + 1
    value: list[str] = []
    while i < n and text[i] != '"':
        if text[i] == "\\" and i + 1 < n:
            if text[i + 1] == "n":
                value.append("\n")
            else:
                value.append(text[i:i + 2])
            i += 2
        else:
            value.append(text[i])
            i += 1
    return [(line, part) for part in "".join(value).split("\n")]


def _raw_doc_lines(text: str, quote: int, hashes: int,
                   line: int) -> list[tuple[int, str]]:
    """The doc text lines of a `#[doc = r#"..."#]` raw string value.

    `text[quote]` is the opening quote and `hashes` the count in its
    delimiter. A raw string's value is its literal body, so a real newline
    in it is a real line break and each body line keeps its own file line.
    """
    n = len(text)
    lines: list[tuple[int, str]] = []
    start = quote + 1
    i = start
    while i < n:
        if text[i] == '"' and text[i + 1:i + 1 + hashes] == "#" * hashes:
            break
        if text[i] == "\n":
            lines.append((line, text[start:i]))
            line += 1
            start = i + 1
        i += 1
    lines.append((line, text[start:i]))
    return lines


def scan(text: str) -> tuple[list[int], list[int]]:
    """One left-to-right pass over Rust source.

    Returns the line numbers of `unsafe` keyword tokens and of attributes
    weakening `unsafe_code`. Raises `LexError` for an unterminated literal,
    comment or attribute. Comments and literals are skipped whole rather
    than blanked, so nothing after one is ever read as code or the reverse.

    `unsafe` is reported inside an attribute too: `#[unsafe(no_mangle)]` is
    edition 2024's spelling and rustc refuses it. Every line of doc text is
    read as code under the rule the module docstring states, so a doctest
    holding `unsafe` is reported wherever its fence sits and however the
    fence is written, because no lint table reaches one.
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
    # Every doc text line in source order, as (file line, content): the
    # content of `///` and `//!` lines, of block doc comments, and of a
    # `#[doc = ...]` or `#![doc = ...]` attribute's string.
    doc: list[tuple[int, str]] = []
    while i < n:
        i, line = _skip_trivia(text, i, line)
        if i >= n:
            break
        c = text[i]
        if c == "/":
            # The only comments left here are doc comments, which
            # _skip_trivia stops on. Their lines are collected as doc
            # text, which _fence_code splits into fences and prose at
            # the end.
            nxt = text[i + 1] if i + 1 < n else ""
            if nxt == "/":
                i, line = _doc_line_text(text, i, line, doc)
            elif nxt == "*":
                i, line = _doc_block_text(text, i, line, doc)
            else:
                i += 1  # a lone slash is punctuation, not a comment
        elif c == '"':
            # The string of a doc attribute is doc text. Only the plain
            # `#[doc = "..."]` and `#![doc = "..."]` spellings are, where
            # `doc` is the attribute's first and only name so far: an
            # `alias` value or a `cfg_attr` wrapper's payload is not.
            if attr_depth == 1 and attr_names == ["doc"]:
                doc.extend(_plain_doc_lines(text, i, line))
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
                    if attr_depth == 1 and attr_names == ["doc"]:
                        doc.extend(_raw_doc_lines(text, hashes_end,
                                                  hashes_end - end, line))
                    i, line = _scan_raw_string(
                        text, hashes_end, hashes_end - end, line)
                else:
                    # `r#name` is a raw identifier. Its name is never a
                    # keyword, so the whole token is consumed here.
                    i = _ident_at(text, hashes_end)
            elif word in STRING_PREFIXES and nxt == '"':
                if word.endswith("r"):
                    if attr_depth == 1 and attr_names == ["doc"]:
                        doc.extend(_raw_doc_lines(text, end, 0, line))
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
    blocks, prose = _fence_code(doc)
    fence_unsafe, fence_weakening = _fence_hits(blocks)
    unsafe_lines.extend(fence_unsafe)
    weakening.extend(fence_weakening)
    for prose_line, content in prose:
        if _prose_unsafe(content):
            unsafe_lines.append(prose_line)
    return sorted(unsafe_lines), sorted(weakening)


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
