#!/usr/bin/env python3
"""One allowlist, read three ways.

deny.toml is the allowlist. cargo-deny 0.19.0 reads it for Rust. This script
reads the same list for npm and Python, checks the third-party notices and
the copyright holder, generates the shipped-binary licence file, and scans
installed wheels when asked.

    python3 scripts/licence_check.py [rust|npm|python|notices|holder|binaries]...
    python3 scripts/licence_check.py --write binaries
    python3 scripts/licence_check.py verify-upstream
    python3 scripts/licence_check.py installed [--python PATH]

With no section, the floor checks run. installed is the wheel scan and
verify-upstream re-fetches pins. Neither is part of that default. The
binaries gate reads the pinned files and does not fetch.
"""

from __future__ import annotations

import argparse
import email
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
import urllib.error
import urllib.request
from dataclasses import dataclass
from email import policy
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

CARGO_DENY_VERSION = "0.19.0"
CARGO_DENY_INSTALL = "cargo install cargo-deny --version 0.19.0 --locked"

# Markers that mean a bundled text is copyleft. A file that contains one is
# refused unless ci/licences.toml records that exact file.
COPYLEFT_MARKERS = (
    "GNU General Public License",
    "GNU Lesser General Public License",
    "GNU Affero General Public License",
    "GPL-2.0",
    "GPL-3.0",
    "LGPL-2.0",
    "LGPL-2.1",
    "LGPL-3.0",
    "AGPL-3.0",
    "GPLv2",
    "GPLv3",
)

EXCEPTION_FIELDS = ("licence", "reason", "scope", "decided", "decided_by")
EXCEPTION_SCOPES = ("dev-only", "tool", "build-only")
# An empty bundled file states no licence. This value records that fact and
# is accepted only together with empty = true.
PLACEHOLDER_LICENCE = "no-licence-text"

# Pre-PEP 639 wheels keep the text at the dist-info root. "licen" matches
# licence and license, so a second prefix is not part of the rule.
_DIST_INFO_LICENCE_PREFIXES = ("licen", "copying", "notice")
_TOOLING_LOCK_PREFIXES = ("tools/", "examples/")

SPDX_EXPRESSION = "MIT OR Apache-2.0"
# Content hash of LICENSE-APACHE. The git blob is d645695673349e3947e8e5ae42332d0ac3164cd7.
APACHE_SHA256 = (
    "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
)
HOLDER_LINE = "Copyright (c) 2026 Tensorbee Limited"
NOTICE_TEXT = "Ocelli\nCopyright 2026 Tensorbee Limited\n"

OPENJPH_BEGIN = b"-----BEGIN OPENJPH LICENSE-----\n"
OPENJPH_END = b"-----END OPENJPH LICENSE-----\n"
RITK_BEGIN = b"-----BEGIN RITK-CODECS LICENSE-MIT-----\n"
RITK_END = b"-----END RITK-CODECS LICENSE-MIT-----\n"

VIRTUAL_PYTHON = "ocelli-tooling"
FLOOR_SECTIONS = ("rust", "npm", "python", "notices", "holder", "binaries")

# A file in a crate root counts when its name begins with one of these.
# The match is case-insensitive. LICENSES/ is a directory, matched the same way.
_LICENCE_NAME_PREFIXES = (
    "license", "licence", "copying", "notice", "unlicense", "copyright",
)
_NO_FILE_SENTENCE = (
    "The published crate carries no licence file. "
    "The text below is the standard text of the licence it declares."
)
_UPSTREAM_SENTENCE = (
    "The published crate carries no licence file. "
    "The text below is the licence file from its repository."
)
# A repository statement that names no holder and holds no terms. The
# notice reproduces that statement, then the elected template.
# Apache-2.0 stays the published text, so the template sentence above
# stays true for it. fill_copyright never rewrites that file.
# The sentence names the repository and the commit from the pin row.
# It is not a fixed string, because two crates can share one text and
# still refer to different commits.
#
# One attribution paragraph in the header. The adapted MIT text appears
# in more than one text group, and CC BY 3.0 section 4(b) asks for
# attribution reasonable to the medium on every copy of the file.
# The Zlib template follows madler/zlib tag v1.3.1 file LICENSE. That
# file is the zlib grant. Its terms keep the two spaces after
# "warranty." that choosealicense.com file _licenses/zlib.txt does not,
# so the Zlib template is not CC BY content and this paragraph does not
# credit it.
_CC_BY_ATTRIBUTION = (
    "Attribution. choosealicense.com file _licenses/mit.txt "
    "(https://github.com/github/choosealicense.com/blob/gh-pages/"
    "_licenses/mit.txt), published by GitHub, is licensed under the "
    "Creative Commons Attribution 3.0 Unported licence "
    "(https://creativecommons.org/licenses/by/3.0/). "
    "The text was adapted into a template whose copyright line is "
    "filled per crate."
)
_CC_BY_REQUIRED = (
    "choosealicense.com",
    "_licenses/mit.txt",
    "Creative Commons Attribution 3.0 Unported",
    "https://creativecommons.org/licenses/by/3.0/",
    "adapted into a template whose copyright line is filled per crate",
)
# The only copyright line a MIT or Zlib template may carry. The generator
# replaces the marker. Apache-2.0 stays exactly as published. Its appendix
# line is instructions to a licensor, and the generator never fills it.
_COPYRIGHT_MARKER = "Copyright (c) <copyright-holder>"
_APACHE_BOILERPLATE = "Copyright [yyyy] [name of copyright owner]"
_RAW_COMMIT = re.compile(
    r"^https://raw\.githubusercontent\.com/[^/]+/[^/]+/"
    r"([0-9a-fA-F]{7,40})(?:/|$)")
_RAW_PIN = re.compile(
    r"^https://raw\.githubusercontent\.com/"
    r"(?P<owner>[^/]+)/(?P<repo>[^/]+)/"
    r"(?P<commit>[0-9a-fA-F]{7,40})/(?P<path>.+)$")
_COPYRIGHT_LINE = re.compile(r"(?i)^\s*(?:copyright\b|\(c\)\b|©)")
_EMAIL = re.compile(r"[\w.+-]+@[\w.-]+\.\w+")
# Placeholders this generator must not leave in its own output. The Apache
# appendix line is not one of them. Third-party Apache texts and the standard
# library notice ship that line as published.
_UNFILLED = ("<copyright-holder>", "[year]", "[fullname]")
_STD_BEGIN = b"-----BEGIN RUST STANDARD LIBRARY NOTICE-----\n"
_STD_END = b"-----END RUST STANDARD LIBRARY NOTICE-----"
_SOURCES = ROOT / "ci" / "notices" / "sources.toml"
_LICENCES = ROOT / "THIRD_PARTY_LICENSES"
# Bytes already read for a pin URL, or None when the path was absent.
# verify-upstream fills this. The binaries gate does not.
_RAW_PIN_CACHE: dict[str, bytes | None] = {}
_SPDX_OPERATOR = re.compile(r"\b(?:OR|AND|WITH)\b")


# ---------------------------------------------------------------------------
# SPDX 2.3 Annex D. Operators are case-sensitive. Identifiers are not.
# Precedence, low to high, is OR, then AND, then WITH, then +.
# A WITH compound is allowed only as a whole allowlisted compound.
# ---------------------------------------------------------------------------

class ExpressionError(Exception):
    """The text is not an SPDX licence expression."""


class _Scan:
    def __init__(self, text: str) -> None:
        self.text = text
        self.n = len(text)
        self.i = 0

    def _skip(self) -> None:
        while self.i < self.n and self.text[self.i].isspace():
            self.i += 1

    def _boundary(self, end: int) -> bool:
        if end >= self.n:
            return True
        char = self.text[end]
        return not (char.isalnum() or char in ".-")

    def take(self, word: str) -> bool:
        self._skip()
        end = self.i + len(word)
        if self.text.startswith(word, self.i) and self._boundary(end):
            self.i = end
            return True
        return False

    def identifier(self) -> str | None:
        self._skip()
        if self.i >= self.n:
            return None
        char = self.text[self.i]
        # SPDX ids may start with a digit. 0BSD is one.
        if not (char.isalnum() or char == "_"):
            return None
        start = self.i
        self.i += 1
        while self.i < self.n and (self.text[self.i].isalnum() or self.text[self.i] in ".-"):
            self.i += 1
        return self.text[start:self.i]

    def char(self, wanted: str) -> bool:
        self._skip()
        if self.i < self.n and self.text[self.i] == wanted:
            self.i += 1
            return True
        return False

    def finished(self) -> bool:
        self._skip()
        return self.i >= self.n


def _parse_or(scan: _Scan):
    left = _parse_and(scan)
    while scan.take("OR"):
        left = ("or", left, _parse_and(scan))
    return left


def _parse_and(scan: _Scan):
    left = _parse_with(scan)
    while scan.take("AND"):
        left = ("and", left, _parse_with(scan))
    return left


def _parse_with(scan: _Scan):
    left = _parse_plus(scan)
    if scan.take("WITH"):
        exception = scan.identifier()
        if exception is None or left[0] not in {"id", "plus"}:
            raise ExpressionError("WITH has no licence exception")
        return ("with", left, exception)
    return left


def _parse_plus(scan: _Scan):
    atom = _parse_atom(scan)
    if scan.char("+"):
        if atom[0] != "id":
            raise ExpressionError("+ does not follow a licence id")
        return ("plus", atom)
    return atom


def _parse_atom(scan: _Scan):
    if scan.char("("):
        node = _parse_or(scan)
        if not scan.char(")"):
            raise ExpressionError("unclosed parenthesis")
        return node
    ident = scan.identifier()
    if ident is None:
        raise ExpressionError("expected a licence id")
    return ("id", ident)


def parse_expression(text: str):
    """Parse one SPDX expression. Raise ExpressionError when it is not one."""
    if text.strip() == "":
        raise ExpressionError("empty licence expression")
    scan = _Scan(text)
    node = _parse_or(scan)
    if not scan.finished():
        raise ExpressionError("trailing text after the licence expression")
    return node


def _compound(node) -> str:
    if node[0] == "id":
        return node[1]
    if node[0] == "plus":
        return node[1][1] + "+"
    if node[0] == "with":
        return f"{_compound(node[1])} WITH {node[2]}"
    raise ExpressionError("a compound is an id, a plus or a WITH")


def satisfiable(node, allow: set[str]) -> bool:
    """True when some disjunct has every conjunct on the allowlist."""
    kind = node[0]
    if kind == "or":
        return satisfiable(node[1], allow) or satisfiable(node[2], allow)
    if kind == "and":
        return satisfiable(node[1], allow) and satisfiable(node[2], allow)
    return _compound(node).casefold() in allow


def every_term_allowed(node, allow: set[str]) -> bool:
    """True when every licence the expression names is on the allowlist.

    Unlike `satisfiable`, an OR does not pass on one allowlisted side. A
    waiver records what a file's text offers, and text that offers a
    copyleft licence as an alternative is the operator's to waive.
    """
    kind = node[0]
    if kind in ("or", "and"):
        return every_term_allowed(node[1], allow) and every_term_allowed(
            node[2], allow)
    return _compound(node).casefold() in allow


def expression_allowed(text: str, allow: set[str]) -> bool:
    try:
        node = parse_expression(text)
    except ExpressionError:
        return False
    return satisfiable(node, allow)


def allowlist() -> set[str]:
    data = tomllib.loads((ROOT / "deny.toml").read_text(encoding="utf-8"))
    raw = data.get("licenses", {}).get("allow", [])
    if not isinstance(raw, list) or not raw:
        sys.exit("FAIL: deny.toml [licenses] allow is empty")
    return {str(item).casefold() for item in raw}


# ---------------------------------------------------------------------------
# Tables
# ---------------------------------------------------------------------------

def _table() -> dict:
    path = ROOT / "ci" / "licences.toml"
    if not path.is_file():
        return {}
    return tomllib.loads(path.read_text(encoding="utf-8"))


def _norm(name: str) -> str:
    return re.sub(r"[-_.]+", "-", name).casefold()


def exception_rows() -> list[dict]:
    rows = _table().get("exception", [])
    return rows if isinstance(rows, list) else []


def python_rows() -> list[dict]:
    rows = _table().get("python", [])
    return rows if isinstance(rows, list) else []


def licence_file_rows() -> list[dict]:
    rows = _table().get("licence_file", [])
    return rows if isinstance(rows, list) else []


def bundle_lists() -> dict[str, set[str]]:
    """platform -> set of site-packages-relative library paths."""
    found: dict[str, set[str]] = {}
    bundles = _table().get("bundle", {})
    if not isinstance(bundles, dict):
        return found
    for platform, packages in bundles.items():
        paths: set[str] = set()
        if isinstance(packages, dict):
            for spec in packages.values():
                if isinstance(spec, dict):
                    libs = spec.get("libs", [])
                    if isinstance(libs, list):
                        paths.update(str(item) for item in libs)
        found[str(platform)] = paths
    return found


def required_libraries() -> list[dict]:
    rows = _table().get("required_library", [])
    return rows if isinstance(rows, list) else []


def _scope_tokens(scope: str) -> list[str] | None:
    """One scope, or several separated by commas. None when any token is unknown."""
    tokens = [part.strip() for part in scope.split(",")]
    if not tokens or any(not part for part in tokens):
        return None
    if any(part not in EXCEPTION_SCOPES for part in tokens):
        return None
    return tokens


def _waiver_authorised(record: dict | None) -> bool:
    """A marker or empty-file waiver counts only with a recorded decision."""
    if not isinstance(record, dict):
        return False
    if any(not str(record.get(field, "")).strip() for field in EXCEPTION_FIELDS):
        return False
    return _scope_tokens(str(record.get("scope", ""))) is not None


def exception_problems(rows: list[dict] | None = None) -> list[str]:
    """Missing fields and unknown scopes. Stale and shipped are separate."""
    problems: list[str] = []
    for row in exception_rows() if rows is None else rows:
        ecosystem = str(row.get("ecosystem", ""))
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        label = f"{ecosystem} {package}@{version}".strip()
        for field in EXCEPTION_FIELDS:
            if not str(row.get(field, "")).strip():
                problems.append(f"exception {label} is missing {field}")
        scope = str(row.get("scope", ""))
        if scope and _scope_tokens(scope) is None:
            problems.append(
                f"exception {package} scope {scope!r} is not dev-only, tool "
                f"or build-only")
        if not ecosystem or not package or not version:
            problems.append(
                f"exception {label} is missing ecosystem, package or version")
    return problems


def licence_file_waiver_problems(rows: list[dict] | None = None) -> list[str]:
    """A row that waives a refusal needs the same fields as an exception.

    A lead decision may name an expression whose every term is on the
    allowlist, under AND and under OR alike, or the placeholder on an empty
    file. Text that offers a copyleft licence even as an alternative is the
    operator's to waive. An operator decision may name any parseable
    expression, because the operator decided it, or the same placeholder.
    """
    problems: list[str] = []
    source = licence_file_rows() if rows is None else rows
    allow: set[str] | None = None
    for row in source:
        waives = row.get("empty") is True or bool(row.get("markers"))
        if not waives:
            continue
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        path = str(row.get("path", ""))
        label = f"{package}@{version} {path}".strip()
        for field in EXCEPTION_FIELDS:
            if not str(row.get(field, "")).strip():
                problems.append(f"licence file {label} is missing {field}")
        scope = str(row.get("scope", ""))
        if scope and _scope_tokens(scope) is None:
            problems.append(
                f"licence file {package} scope {scope!r} is not dev-only, tool "
                f"or build-only")
        licence = str(row.get("licence", "")).strip()
        empty = row.get("empty") is True
        # The placeholder is not a grant. It pairs with an empty file and
        # with nothing else, so a row cannot claim a licence the file does
        # not state.
        if (licence == PLACEHOLDER_LICENCE) != empty:
            problems.append(
                f"licence file {label} licence {licence!r} is valid only "
                f"on an empty file together with {PLACEHOLDER_LICENCE}")
        elif licence and licence != PLACEHOLDER_LICENCE:
            # Only an operator may name a licence outside the allowlist.
            # Any other decider, including the lead, is held to the list.
            decided_by = str(row.get("decided_by", "")).strip()
            if decided_by == "operator":
                try:
                    parse_expression(licence)
                except ExpressionError:
                    problems.append(
                        f"licence file {label} licence {licence!r} is not a "
                        f"parseable SPDX expression")
            else:
                if allow is None:
                    allow = allowlist()
                try:
                    named = every_term_allowed(parse_expression(licence), allow)
                except ExpressionError:
                    named = False
                if not named:
                    problems.append(
                        f"licence file {label} licence {licence!r} is not an "
                        f"allowlisted expression a lead may waive")
    return problems


def _lock_versions(kind: str) -> set[tuple[str, str]]:
    """(package, version) pairs present in the tracked lock for one ecosystem."""
    found: set[tuple[str, str]] = set()
    if kind == "python":
        data = tomllib.loads((ROOT / "uv.lock").read_text(encoding="utf-8"))
        for package in data.get("package", []):
            name = str(package.get("name", ""))
            if name == VIRTUAL_PYTHON:
                continue
            found.add((name, str(package.get("version", ""))))
        return found
    if kind == "npm":
        for path in _tracked("*package-lock.json"):
            packages = json.loads(path.read_text(encoding="utf-8")).get("packages", {})
            for key, entry in packages.items():
                if not isinstance(entry, dict):
                    continue
                name, version = _npm_identity(path, key, entry)
                if name and version:
                    found.add((name, version))
        return found
    if kind == "rust":
        text = (ROOT / "Cargo.lock").read_text(encoding="utf-8")
        # Cargo.lock pairs name and version as neighbouring keys. Read both.
        current_name = ""
        for line in text.splitlines():
            name_match = re.match(r'^name = "([^"]+)"$', line.strip())
            version_match = re.match(r'^version = "([^"]+)"$', line.strip())
            if name_match:
                current_name = name_match.group(1)
            elif version_match and current_name:
                found.add((current_name, version_match.group(1)))
                current_name = ""
        return found
    return found


def _is_tooling_lock(rel: str) -> bool:
    return rel.startswith(_TOOLING_LOCK_PREFIXES)


def _scope_holds_for_lock(token: str, rel: str, entry: dict) -> bool:
    """Whether one scope token is true for one lockfile entry.

    A lockfile under tools/ or examples/ is tooling scope, because those
    trees are never published. In the root lockfile, dev-only holds only
    when the entry's dev flag is true.
    """
    if token == "tool":
        return _is_tooling_lock(rel)
    if token == "dev-only":
        if _is_tooling_lock(rel) or rel != "package-lock.json":
            return False
        return entry.get("dev") is True
    return False


def _npm_occurrences() -> dict[tuple[str, str], list[tuple[str, dict]]]:
    found: dict[tuple[str, str], list[tuple[str, dict]]] = {}
    for path in _tracked("*package-lock.json"):
        try:
            packages = json.loads(path.read_text(encoding="utf-8")).get("packages", {})
        except json.JSONDecodeError:
            continue
        rel = path.relative_to(ROOT).as_posix()
        for key, entry in packages.items():
            if key == "" or not isinstance(entry, dict):
                continue
            name, version = _npm_identity(path, key, entry)
            if name and version:
                found.setdefault((name, version), []).append((rel, entry))
    return found


def scope_problems() -> list[str]:
    """Refuse an npm exception whose scope is false for a lockfile it covers."""
    problems: list[str] = []
    occurrences = _npm_occurrences()
    for row in exception_rows():
        if str(row.get("ecosystem", "")) != "npm":
            continue
        scope = str(row.get("scope", ""))
        tokens = _scope_tokens(scope)
        if not tokens:
            continue
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        covered = occurrences.get((package, version), [])
        if not covered:
            continue
        held = {token: False for token in tokens}
        for rel, entry in covered:
            if not any(_scope_holds_for_lock(token, rel, entry) for token in tokens):
                problems.append(
                    f"exception npm {package}@{version} scope {scope!r} "
                    f"is false for {rel}")
            for token in tokens:
                if _scope_holds_for_lock(token, rel, entry):
                    held[token] = True
        for token, ok in held.items():
            if not ok:
                problems.append(
                    f"exception npm {package}@{version} scope {token!r} "
                    f"holds for no lockfile it covers")
    return problems


def stale_problems() -> list[str]:
    problems: list[str] = []
    cached: dict[str, set[tuple[str, str]]] = {}
    for row in exception_rows():
        ecosystem = str(row.get("ecosystem", ""))
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        if ecosystem not in cached:
            cached[ecosystem] = _lock_versions(ecosystem)
        if (package, version) not in cached[ecosystem]:
            problems.append(
                f"stale exception {ecosystem} {package}@{version} is not in "
                f"the lock")
    # A file waiver names one package version. The dist-info path embeds
    # that version for some rows and the imagecodecs/licenses paths do not,
    # so the row's own version is what goes stale when uv.lock moves.
    if "python" not in cached:
        cached["python"] = _lock_versions("python")
    for row in licence_file_rows():
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        path = str(row.get("path", ""))
        if (package, version) not in cached["python"]:
            problems.append(
                f"stale licence file {package}@{version} {path} is not in "
                f"the lock")
    return problems


# ---------------------------------------------------------------------------
# npm
# ---------------------------------------------------------------------------

def _tracked(pattern: str) -> list[Path]:
    completed = subprocess.run(
        ["git", "ls-files", "-z", "--", pattern],
        cwd=ROOT, capture_output=True, text=True, check=True)
    return [ROOT / name for name in completed.stdout.split("\0") if name]


def _npm_identity(lock: Path, key: str, entry: dict) -> tuple[str, str]:
    if key == "":
        return "", ""
    if entry.get("link") is True:
        resolved = str(entry.get("resolved", ""))
        manifest_path = lock.parent / resolved / "package.json"
        if not manifest_path.is_file():
            return "", ""
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        return str(manifest.get("name", "")), str(manifest.get("version", ""))
    name = str(entry.get("name", ""))
    if not name:
        parts = [part for part in key.split("/") if part and part != "node_modules"]
        if len(parts) >= 2 and parts[-2].startswith("@"):
            name = "/".join(parts[-2:])
        elif parts:
            name = parts[-1]
    return name, str(entry.get("version", ""))


def _npm_licence(lock: Path, entry: dict) -> str:
    if entry.get("link") is True:
        resolved = str(entry.get("resolved", ""))
        manifest_path = lock.parent / resolved / "package.json"
        if manifest_path.is_file():
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            value = manifest.get("license", "")
            return value if isinstance(value, str) else ""
        return ""
    value = entry.get("license", "")
    return value if isinstance(value, str) else ""


def npm_runtime_closure() -> set[str]:
    """Package names reachable from the published packages' dependencies."""
    lock_path = ROOT / "package-lock.json"
    packages = json.loads(lock_path.read_text(encoding="utf-8")).get("packages", {})

    def find(name: str) -> dict | None:
        suffix = f"node_modules/{name}"
        hits = [key for key in packages if key == suffix or key.endswith("/" + suffix)]
        if not hits:
            return None
        return packages[sorted(hits, key=len)[0]]

    seen: set[str] = set()

    def walk(name: str) -> None:
        if name in seen:
            return
        seen.add(name)
        entry = find(name)
        if not isinstance(entry, dict):
            return
        if entry.get("link") is True:
            manifest_path = ROOT / str(entry.get("resolved", "")) / "package.json"
            if not manifest_path.is_file():
                return
            children = json.loads(manifest_path.read_text(encoding="utf-8")).get("dependencies", {})
        else:
            children = entry.get("dependencies", {})
        if isinstance(children, dict):
            for child in children:
                walk(str(child))

    for folder in ("packages/core", "packages/react"):
        manifest = json.loads((ROOT / folder / "package.json").read_text(encoding="utf-8"))
        for name in manifest.get("dependencies", {}):
            walk(str(name))
    return seen


def npm_problems(allow: set[str]) -> list[str]:
    problems: list[str] = []
    closure = npm_runtime_closure()
    excepted = {
        (str(row.get("package", "")), str(row.get("version", ""))): row
        for row in exception_rows() if str(row.get("ecosystem", "")) == "npm"
    }
    for package, version in excepted:
        if package in closure:
            problems.append(
                f"exception npm {package}@{version} is in the published "
                f"runtime closure")
    for lock in _tracked("*package-lock.json"):
        try:
            packages = json.loads(lock.read_text(encoding="utf-8")).get("packages", {})
        except json.JSONDecodeError:
            problems.append(f"{lock.relative_to(ROOT)} is not valid JSON")
            continue
        rel = lock.relative_to(ROOT).as_posix()
        for key, entry in packages.items():
            if key == "" or not isinstance(entry, dict):
                continue
            name, version = _npm_identity(lock, key, entry)
            if not name:
                problems.append(f"{rel}: {key} has no package name")
                continue
            row = excepted.get((name, version))
            if row is not None:
                continue
            licence = _npm_licence(lock, entry).strip()
            if not licence:
                problems.append(
                    f"{rel}: {name}@{version} has no licence and no exception")
                continue
            if not expression_allowed(licence, allow):
                problems.append(
                    f"{rel}: {name}@{version} licence {licence!r} is not allowed")
    return problems


# ---------------------------------------------------------------------------
# Python table
# ---------------------------------------------------------------------------

def uv_packages() -> list[tuple[str, str]]:
    data = tomllib.loads((ROOT / "uv.lock").read_text(encoding="utf-8"))
    found = []
    for package in data.get("package", []):
        name = str(package.get("name", ""))
        if name == VIRTUAL_PYTHON:
            continue
        found.append((name, str(package.get("version", ""))))
    return found


def python_problems(allow: set[str]) -> list[str]:
    problems: list[str] = []
    locked = uv_packages()
    locked_set = set(locked)
    rows = {(str(row.get("package", "")), str(row.get("version", ""))): row
            for row in python_rows()}
    for name, version in locked:
        row = rows.get((name, version))
        if row is None:
            problems.append(f"python {name}@{version} has no licence row")
            continue
        spdx = str(row.get("spdx", "")).strip()
        if not spdx or not expression_allowed(spdx, allow):
            problems.append(
                f"python {name}@{version} licence {spdx!r} is not allowed")
    for key in rows:
        if key not in locked_set:
            problems.append(
                f"python {key[0]}@{key[1]} is not in uv.lock")
    return problems


def metadata_string(message: email.message.Message) -> str:
    """License-Expression, else License, else the License classifiers."""
    expression = str(message.get("License-Expression") or "").strip()
    if expression:
        return expression
    licence = str(message.get("License") or "").strip()
    if licence:
        return licence
    classifiers = [
        str(item).strip() for item in (message.get_all("Classifier") or [])
        if str(item).lower().startswith("license")
    ]
    return "\n".join(classifiers)


def licence_bytes_problems(relpath: str, data: bytes, record: dict | None) -> list[str]:
    """Empty files and copyleft markers. A waiver counts only when authorised."""
    problems: list[str] = []
    authorised = _waiver_authorised(record)
    if len(data) == 0:
        if not (authorised and record is not None and record.get("empty") is True):
            problems.append(f"empty licence file {relpath}")
        return problems
    found = [marker for marker in COPYLEFT_MARKERS if marker.encode("utf-8") in data]
    if not found:
        return problems
    allowed = set(record.get("markers", [])) if authorised and record else set()
    extra = [marker for marker in found if marker not in allowed]
    if extra:
        problems.append(
            f"copyleft marker in {relpath}: {', '.join(extra)}")
        return problems
    if "GPLv2" in allowed:
        # licence and reason are the fields the waiver rule requires.
        # Reading spdx and note would let a second copy disagree with the
        # record the gate checks.
        licence = str(record.get("licence", "") if record else "")
        reason = str(record.get("reason", "") if record else "")
        if licence != "BSD-2-Clause" or "not taken" not in reason:
            problems.append(
                f"{relpath} records a GPLv2 marker without BSD-2-Clause and "
                f"the statement that the alternative is not taken")
    return problems


def bundle_list_problems(found: list[str], platform: str,
                         recorded: set[str] | None) -> list[str]:
    problems: list[str] = []
    if recorded is None:
        problems.append(f"no recorded bundled-library list for {platform}")
        return problems
    for path in sorted(found):
        if path not in recorded:
            problems.append(
                f"bundled library {path} is not on the recorded list for "
                f"{platform}")
    return problems


def _dist_message(dist: Path) -> email.message.Message:
    return email.message_from_string(
        (dist / "METADATA").read_text(encoding="utf-8", errors="replace"),
        policy=policy.compat32)


def _shared_libraries(site: Path) -> list[str]:
    found: list[str] = []
    if not site.is_dir():
        return found
    for child in site.iterdir():
        if child.is_dir() and child.name.endswith(".libs"):
            for path in child.rglob("*"):
                if path.is_file():
                    found.append(path.relative_to(site).as_posix())
        elif child.is_dir() and not child.name.endswith(".dist-info"):
            dylibs = child / ".dylibs"
            if dylibs.is_dir():
                for path in dylibs.rglob("*"):
                    if path.is_file():
                        found.append(path.relative_to(site).as_posix())
    return found


def _licence_files(site: Path) -> list[Path]:
    found: list[Path] = []
    if not site.is_dir():
        return found
    for dist in site.glob("*.dist-info"):
        folder = dist / "licenses"
        if folder.is_dir():
            found.extend(path for path in folder.rglob("*") if path.is_file())
        # Pre-PEP 639 wheels leave the text beside METADATA. licenses/ is
        # already walked above, so only the dist-info root is added here.
        for path in dist.iterdir():
            if path.is_file() and path.name.casefold().startswith(
                    _DIST_INFO_LICENCE_PREFIXES):
                found.append(path)
    for child in site.iterdir():
        if not child.is_dir() or child.name.endswith(".dist-info"):
            continue
        folder = child / "licenses"
        if folder.is_dir():
            found.extend(path for path in folder.rglob("*") if path.is_file())
    return found


def _matching_waiver(files: dict[tuple[str, str, str], dict], relpath: str,
                     installed_version: dict[str, str]) -> dict | None:
    """The waiver for this file at the installed package version.

    A row for any other version does not apply. An upgrade would otherwise
    carry a waiver onto text nobody has re-read.
    """
    owner = Path(relpath).parts[0]
    if owner.endswith(".dist-info"):
        owner_name = re.sub(r"-\d.*$", "", owner)
    else:
        owner_name = owner
    version = installed_version.get(_norm(owner_name), "")
    fallback = None
    for (norm_name, row_version, recorded_path), row in files.items():
        if recorded_path != relpath or row_version != version:
            continue
        if norm_name == _norm(owner_name):
            return row
        # The dist-info directory can spell the name with an underscore.
        if fallback is None:
            fallback = row
    return fallback


def installed_problems(site: Path, platform: str, machine: str) -> list[str]:
    problems: list[str] = []
    platform_key = f"{platform}-{machine}"
    locked = {_norm(name): (name, version) for name, version in uv_packages()}
    rows = {
        (_norm(str(row.get("package", ""))), str(row.get("version", ""))): row
        for row in python_rows()
    }
    files = {
        (_norm(str(row.get("package", ""))), str(row.get("version", "")),
         str(row.get("path", ""))): row
        for row in licence_file_rows()
    }
    seen: set[str] = set()
    installed_version: dict[str, str] = {}
    for dist in sorted(site.glob("*.dist-info")):
        if not (dist / "METADATA").is_file():
            problems.append(f"{dist.name} has no METADATA")
            continue
        message = _dist_message(dist)
        name = str(message.get("Name") or "")
        version = str(message.get("Version") or "")
        if name and version:
            installed_version[_norm(name)] = version
        key = (_norm(name), version)
        seen.add(_norm(name))
        if _norm(name) not in locked:
            problems.append(
                f"installed {name}@{version} is not in uv.lock")
            continue
        row = rows.get(key)
        if row is None:
            problems.append(f"python {name}@{version} has no licence row")
            continue
        actual = metadata_string(message)
        expected = str(row.get("metadata", ""))
        if actual != expected:
            problems.append(
                f"python {name}@{version} metadata {actual!r} does not match "
                f"the recorded {expected!r}")
    for norm_name, (name, version) in locked.items():
        if norm_name not in seen:
            problems.append(f"python {name}@{version} is not installed")

    for path in _licence_files(site):
        relpath = path.relative_to(site).as_posix()
        record = _matching_waiver(files, relpath, installed_version)
        problems.extend(licence_bytes_problems(relpath, path.read_bytes(), record))

    libraries = _shared_libraries(site)
    recorded = bundle_lists()
    problems.extend(bundle_list_problems(
        libraries, platform_key, recorded.get(platform_key)))

    excepted_files = {
        str(row.get("file", ""))
        for row in exception_rows()
        if str(row.get("ecosystem", "")) == "python"
        and str(row.get("platform", platform_key)) == platform_key
    }
    for row in required_libraries():
        if str(row.get("platform", "")) != platform_key:
            continue
        relpath = str(row.get("file", ""))
        if not (site / relpath).is_file():
            problems.append(
                f"required bundled library {relpath} is absent on "
                f"{platform_key}")
        elif relpath not in excepted_files:
            problems.append(
                f"required bundled library {relpath} has no tool exception")
    return problems


def resolve_python(explicit: str | None) -> Path | None:
    """The interpreter corpus_tests uses, or the one --python names."""
    if explicit:
        path = Path(explicit).expanduser()
        if not path.is_file():
            print(f"FAIL: interpreter {path} is not present")
            return None
        return path
    sys.path.insert(0, str(ROOT / "scripts"))
    import corpus_tests
    path, origin = corpus_tests.resolve()
    if path is None:
        print("FAIL: no interpreter can be resolved for the wheel scan")
        print(origin)
        return None
    return path


def installed_from_interpreter(explicit: str | None) -> list[str]:
    interpreter = resolve_python(explicit)
    if interpreter is None:
        return ["the wheel scan has no interpreter"]
    program = (
        "import json, platform, sysconfig\n"
        "print(json.dumps({"
        "'purelib': sysconfig.get_paths()['purelib'],"
        "'platform': __import__('sys').platform,"
        "'machine': platform.machine()}))\n"
    )
    completed = subprocess.run(
        [str(interpreter), "-c", program],
        capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        return [f"{interpreter} could not report its site-packages"]
    try:
        info = json.loads(completed.stdout)
    except json.JSONDecodeError:
        return [f"{interpreter} site-packages report is not JSON"]
    site = Path(str(info.get("purelib", "")))
    if not site.is_dir():
        return [f"site-packages {site} is not a directory"]
    return installed_problems(site, str(info.get("platform", "")),
                              str(info.get("machine", "")))


def synthetic_installed_problems(kind: str) -> list[str]:
    """One planted wheel problem, from installed_problems on a temporary tree.

    kind is marker, library, platform or clean. The tree holds one package so
    the real lockfile cannot report every other package as missing.
    """
    global ROOT
    previous = ROOT
    try:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            ROOT = root
            (root / "uv.lock").write_text(
                'version = 1\n[[package]]\nname = "pyyaml"\nversion = "6.0.3"\n',
                encoding="utf-8")
            licences = (
                '[[python]]\n'
                'package = "pyyaml"\n'
                'version = "6.0.3"\n'
                'spdx = "MIT"\n'
                'metadata = "MIT"\n'
            )
            if kind != "platform":
                licences += "\n[bundle.darwin-arm64.pyyaml]\nlibs = []\n"
            ci = root / "ci"
            ci.mkdir()
            (ci / "licences.toml").write_text(licences, encoding="utf-8")
            site = root / "site-packages"
            dist = site / "pyyaml-6.0.3.dist-info"
            folder = dist / "licenses"
            folder.mkdir(parents=True)
            (dist / "METADATA").write_text(
                "Metadata-Version: 2.1\nName: pyyaml\nVersion: 6.0.3\n"
                "License: MIT\n",
                encoding="utf-8")
            licence = ("GNU General Public License\n" if kind == "marker"
                       else "MIT License\n")
            (folder / "LICENSE").write_text(licence, encoding="utf-8")
            if kind == "library":
                libs = site / "pyyaml.libs"
                libs.mkdir()
                (libs / "libextra.so").write_bytes(b"\0")
            if kind == "platform":
                return installed_problems(site, "freebsd", "x86_64")
            return installed_problems(site, "darwin", "arm64")
    finally:
        ROOT = previous


# ---------------------------------------------------------------------------
# Rust
# ---------------------------------------------------------------------------

def _cargo_deny_version() -> tuple[str | None, list[str]]:
    cargo = shutil.which("cargo")
    if cargo is None:
        return None, [
            f"cargo-deny is not installed. Install it with `{CARGO_DENY_INSTALL}`"
        ]
    completed = subprocess.run(
        [cargo, "deny", "--version"],
        cwd=ROOT, capture_output=True, text=True, check=False)
    text = (completed.stdout or "") + (completed.stderr or "")
    if completed.returncode != 0 or "cargo-deny" not in text:
        return None, [
            f"cargo-deny is not installed. Install it with `{CARGO_DENY_INSTALL}`"
        ]
    match = re.search(r"cargo-deny\s+(\S+)", text)
    found = match.group(1) if match else ""
    if found != CARGO_DENY_VERSION:
        return found, [
            f"cargo-deny {found or 'unknown'} is not {CARGO_DENY_VERSION}. "
            f"Install it with `{CARGO_DENY_INSTALL}`"
        ]
    return found, []


def _diagnostic_names(node) -> list[str]:
    names: list[str] = []
    if isinstance(node, dict):
        krate = node.get("Krate")
        if isinstance(krate, dict) and krate.get("name"):
            names.append(str(krate["name"]))
        for value in node.values():
            names.extend(_diagnostic_names(value))
    elif isinstance(node, list):
        for item in node:
            names.extend(_diagnostic_names(item))
    return names


def rust_problems() -> list[str]:
    _version, problems = _cargo_deny_version()
    if problems:
        return problems
    completed = subprocess.run(
        ["cargo", "deny", "--locked", "--format", "json", "check", "licenses"],
        cwd=ROOT, capture_output=True, text=True, check=False)
    named = False
    for line in (completed.stderr or "").splitlines():
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if item.get("type") != "diagnostic":
            continue
        fields = item.get("fields", {})
        if fields.get("severity") != "error":
            continue
        names = _diagnostic_names(fields.get("graphs", []))
        crate = names[0] if names else "unknown"
        span = ""
        labels = fields.get("labels") or []
        if labels and isinstance(labels[0], dict):
            span = str(labels[0].get("span", ""))
        problems.append(
            f"rust: {crate} failed to satisfy license requirements ({span})")
        named = True
    if completed.returncode != 0 and not named:
        problems.append(f"cargo deny exited {completed.returncode}")
    problems.extend(_rust_exception_problems())
    return problems


class ShippedClosureError(RuntimeError):
    """cargo metadata could not produce the shipped closure."""


def _cargo_metadata(extra: list[str]) -> dict:
    """One locked offline metadata document.

    A missing registry cache is a failure. This call does not reach the network
    and it does not skip the closure.
    """
    completed = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline",
         *extra],
        cwd=ROOT, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        detail = ""
        for line in (completed.stderr or completed.stdout or "").splitlines():
            if line.strip():
                detail = line.strip()
                break
        raise ShippedClosureError(
            "cargo metadata --locked --offline failed. "
            "Run `cargo fetch --locked` and retry. "
            f"{detail}")
    return json.loads(completed.stdout)


def _walk_closure(data: dict, root_name: str) -> dict[str, dict]:
    """Package id to package record, following normal and build edges."""
    packages = {item["id"]: item for item in data.get("packages", [])}
    nodes = {item["id"]: item for item in data.get("resolve", {}).get("nodes", [])}
    roots = [item["id"] for item in data.get("packages", [])
             if item.get("name") == root_name]
    found: dict[str, dict] = {}
    stack = list(roots)
    seen: set[str] = set()
    while stack:
        ident = stack.pop()
        if ident in seen:
            continue
        seen.add(ident)
        package = packages.get(ident)
        if package and package.get("name"):
            found[ident] = package
        node = nodes.get(ident) or {}
        for dep in node.get("deps", []):
            kinds = dep.get("dep_kinds") or []
            keep = False
            for kind in kinds:
                value = kind.get("kind") if isinstance(kind, dict) else None
                if value in (None, "build"):
                    keep = True
            if keep and dep.get("pkg"):
                stack.append(dep["pkg"])
    return found


def _rust_shipped() -> tuple[dict[str, dict], set[str]]:
    """Records reachable from the two entry points, and the workspace ids.

    The wasm entry point is filtered to wasm32-unknown-unknown. The native
    entry point has no platform filter, so the result is the same on every
    host. Workspace members stay in the record map. Callers that want the
    third-party set drop those ids. Dev edges are not followed.
    """
    queries = (
        ("ocelli-wasm", ["--filter-platform", "wasm32-unknown-unknown"]),
        ("ocelli-native", []),
    )
    found: dict[str, dict] = {}
    members: set[str] = set()
    for root_name, extra in queries:
        data = _cargo_metadata(extra)
        found.update(_walk_closure(data, root_name))
        for ident in data.get("workspace_members", []):
            members.add(str(ident))
    return found, members


def _rust_exception_problems() -> list[str]:
    rows = [row for row in exception_rows()
            if str(row.get("ecosystem", "")) == "rust"]
    if not rows:
        return []
    try:
        shipped, _members = _rust_shipped()
    except (ShippedClosureError, RuntimeError, json.JSONDecodeError, OSError) as exc:
        return [f"rust shipped-closure walk failed: {exc}"]
    names = {str(package.get("name")) for package in shipped.values()}
    problems = []
    for row in rows:
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        if package in names:
            problems.append(
                f"exception rust {package}@{version} is in the published "
                f"runtime closure")
    return problems


# ---------------------------------------------------------------------------
# Notices and holder
# ---------------------------------------------------------------------------

def _between(blob: bytes, begin: bytes, end: bytes) -> bytes | None:
    start = blob.find(begin)
    stop = blob.find(end)
    if start < 0 or stop < 0 or stop < start:
        return None
    return blob[start + len(begin):stop]


def notices_problems() -> list[str]:
    problems: list[str] = []
    path = ROOT / "THIRD_PARTY_NOTICES"
    if not path.is_file():
        problems.append("THIRD_PARTY_NOTICES is absent")
        return problems
    blob = path.read_bytes()
    if b"Tensorbee" in blob:
        problems.append("THIRD_PARTY_NOTICES names Tensorbee")
    vendor = ROOT / "vendor" / "openjph-core-0.1.0" / "LICENSE"
    if not vendor.is_file():
        problems.append("vendor/openjph-core-0.1.0/LICENSE is absent")
    else:
        body = _between(blob, OPENJPH_BEGIN, OPENJPH_END)
        vendor_bytes = vendor.read_bytes()
        digest = hashlib.sha256(vendor_bytes).hexdigest()
        sys.path.insert(0, str(ROOT / "scripts"))
        import pin_and_size_check
        if digest != pin_and_size_check.OPENJPH_LICENCE_SHA256:
            problems.append(
                f"vendor/openjph-core-0.1.0/LICENSE sha256 is {digest}")
        if body != vendor_bytes:
            problems.append(
                "THIRD_PARTY_NOTICES OpenJPH body does not match "
                "vendor/openjph-core-0.1.0/LICENSE")
    ritk = (ROOT / "vendor" / "ritk-codecs-0.6.0" / "LICENSE-MIT").read_bytes()
    ritk_body = _between(blob, RITK_BEGIN, RITK_END)
    if ritk_body != ritk:
        problems.append(
            "THIRD_PARTY_NOTICES ritk-codecs body does not match "
            "vendor/ritk-codecs-0.6.0/LICENSE-MIT")
    text = blob.decode("utf-8")
    if "Michael Knopke" not in text or "openjph-core 0.1.0" not in text:
        problems.append(
            "THIRD_PARTY_NOTICES does not name the openjph-core port")
    if "LICENSE-APACHE" not in text:
        problems.append(
            "THIRD_PARTY_NOTICES does not point at the ritk-codecs Apache grant")
    return problems


def _member_manifests() -> list[Path]:
    root = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    members = root.get("workspace", {}).get("members", [])
    found: list[Path] = []
    for pattern in members:
        if "*" in pattern:
            for path in sorted(ROOT.glob(pattern)):
                manifest = path / "Cargo.toml"
                if manifest.is_file():
                    found.append(manifest)
        else:
            manifest = ROOT / pattern / "Cargo.toml"
            if manifest.is_file():
                found.append(manifest)
    return found


def inherits_workspace_licence(text: str) -> bool:
    """True when [package].license is the workspace key, not a string or a comment."""
    try:
        data = tomllib.loads(text)
    except tomllib.TOMLDecodeError:
        return False
    package = data.get("package")
    if not isinstance(package, dict):
        return False
    licence = package.get("license")
    return isinstance(licence, dict) and licence.get("workspace") is True


def holder_problems() -> list[str]:
    problems: list[str] = []
    for path in _tracked("*LICENSE-MIT"):
        relative = path.relative_to(ROOT).as_posix()
        # vendor/ and ci/notices/ hold other projects' grants. Their bytes
        # are pinned separately and are not this project's copyright.
        if relative.startswith(("vendor/", "ci/notices/")):
            continue
        text = path.read_text(encoding="utf-8")
        if HOLDER_LINE not in text:
            problems.append(
                f"{path.relative_to(ROOT).as_posix()} does not name "
                f"Tensorbee Limited")
    for relative in ("NOTICE", "packages/core/NOTICE", "packages/react/NOTICE"):
        path = ROOT / relative
        if not path.is_file() or path.read_text(encoding="utf-8") != NOTICE_TEXT:
            problems.append(f"{relative} does not name Tensorbee Limited")
    licence = (ROOT / "LICENSE").read_text(encoding="utf-8")
    if "Tensorbee Limited" not in licence:
        problems.append("LICENSE does not name Tensorbee Limited")
    apache = (ROOT / "LICENSE-APACHE").read_bytes()
    digest = hashlib.sha256(apache).hexdigest()
    if digest != APACHE_SHA256:
        problems.append(
            f"LICENSE-APACHE sha256 is {digest}, expected the unmodified "
            f"Apache text")
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    value = workspace.get("workspace", {}).get("package", {}).get("license")
    if value != SPDX_EXPRESSION:
        problems.append(
            f"Cargo.toml licence is {value!r}, expected {SPDX_EXPRESSION}")
    for manifest in _member_manifests():
        text = manifest.read_text(encoding="utf-8")
        if not inherits_workspace_licence(text):
            problems.append(
                f"{manifest.relative_to(ROOT).as_posix()} licence is not "
                f"{SPDX_EXPRESSION}")
    for path in _tracked("*package.json"):
        if "node_modules" in path.parts:
            continue
        manifest = json.loads(path.read_text(encoding="utf-8"))
        if manifest.get("license") != SPDX_EXPRESSION:
            problems.append(
                f"{path.relative_to(ROOT).as_posix()} licence is "
                f"{manifest.get('license')!r}, expected {SPDX_EXPRESSION}")
    project = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    project_licence = project.get("project", {}).get("license")
    if project_licence != SPDX_EXPRESSION:
        problems.append(
            f"pyproject.toml licence is {project_licence!r}, expected "
            f"{SPDX_EXPRESSION}")
    return problems


# ---------------------------------------------------------------------------
# Shipped binary notices
# ---------------------------------------------------------------------------

class BinariesError(Exception):
    """The notice file cannot be generated from the inputs."""


@dataclass(frozen=True)
class CrateNotice:
    """One third-party crate, already reduced to the fields the file carries."""

    name: str
    version: str
    source: str
    expression: str
    authors: tuple[str, ...]
    files: tuple[tuple[str, bytes], ...]
    repository: str = ""
    # packaged, upstream, template, or statement. Empty files are a template.
    # statement reproduces a repository statement, then the elected template.
    notice: str = "packaged"
    earlier: tuple[tuple[str, bytes], ...] = ()
    # The pin URL's repository and commit. The shipped sentence uses these,
    # not the package repository, which is the copyright holder line.
    pin_repository: str = ""
    commit: str = ""
    earlier_repository: str = ""
    earlier_commit: str = ""


def normalise_newlines(data: bytes) -> bytes:
    """CRLF and a lone CR become LF. No other byte changes."""
    return data.replace(b"\r\n", b"\n").replace(b"\r", b"\n")


def legacy_or(expression: str) -> str:
    """Cargo's deprecated `/` separator, read as OR when no operator is present."""
    if "/" in expression and _SPDX_OPERATOR.search(expression) is None:
        return expression.replace("/", " OR ")
    return expression


def _disjuncts(node) -> list:
    if node[0] == "or":
        return _disjuncts(node[1]) + _disjuncts(node[2])
    return [node]


def _terms(node) -> list[str]:
    if node[0] == "and":
        return _terms(node[1]) + _terms(node[2])
    return [_compound(node)]


def elect_terms(expression: str, allow: set[str]) -> list[str]:
    """The left-most disjunct whose every term is allowlisted."""
    prepared = legacy_or(expression)
    try:
        node = parse_expression(prepared)
    except ExpressionError as exc:
        raise BinariesError(
            f"licence expression is not SPDX: {exc}") from exc
    for disjunct in _disjuncts(node):
        terms = _terms(disjunct)
        if terms and all(term.casefold() in allow for term in terms):
            return terms
    raise BinariesError(f"has no allowlisted licence in {expression}")


def collect_licence_files(
        crate_root: Path, license_file: str | None) -> list[tuple[str, bytes]]:
    """Licence files in one crate root.

    Root files whose names begin with a licence prefix, every file under a
    LICENSES directory, and the manifest licence-file when it names one more.
    """
    found: dict[str, Path] = {}
    if crate_root.is_dir():
        for path in crate_root.iterdir():
            if path.is_file() and path.name.casefold().startswith(
                    _LICENCE_NAME_PREFIXES):
                found[path.name] = path
            elif path.is_dir() and path.name.casefold() == "licenses":
                for child in path.rglob("*"):
                    if child.is_file():
                        found[child.relative_to(crate_root).as_posix()] = child
    if license_file:
        relative = Path(license_file)
        if relative.is_absolute() or ".." in relative.parts:
            raise BinariesError(
                f"licence file {license_file} escapes the crate root")
        key = relative.as_posix()
        if key not in found:
            candidate = crate_root / relative
            if not candidate.is_file():
                raise BinariesError(f"licence file {key} is absent")
            found[key] = candidate
    return [(name, found[name].read_bytes()) for name in sorted(found)]


def source_label(package: dict) -> str:
    """`crates.io`, or a path relative to this repository. Never a local path."""
    name = str(package.get("name") or "unknown")
    source = package.get("source")
    manifest = str(package.get("manifest_path") or "")
    if isinstance(source, str) and "crates.io" in source:
        return "crates.io"
    if manifest:
        try:
            relative = Path(manifest).resolve().parent.relative_to(ROOT.resolve())
        except ValueError:
            relative = None
        if relative is not None:
            text = relative.as_posix()
            return text or "."
    if isinstance(source, str) and source.startswith("git+"):
        base = source.split("#", 1)[0]
        if "/Users/" in base or "/home/" in base or ".cargo" in base:
            raise BinariesError(f"{name} source would embed a local path")
        return base
    if isinstance(source, str) and source.startswith("registry+"):
        if "/Users/" in source or "/home/" in source or ".cargo/registry" in source:
            raise BinariesError(f"{name} source would embed a local path")
        return source
    raise BinariesError(f"{name} manifest is outside the repository")


def _load_sources() -> dict:
    if not _SOURCES.is_file():
        raise BinariesError("ci/notices/sources.toml is absent")
    return tomllib.loads(_SOURCES.read_text(encoding="utf-8"))


def _table_rows(data: dict, key: str, required: bool) -> list[dict]:
    rows = data.get(key, [])
    if not rows:
        if required:
            raise BinariesError(f"ci/notices/sources.toml has no {key} rows")
        return []
    if not isinstance(rows, list):
        raise BinariesError(f"ci/notices/sources.toml {key} rows are not a list")
    return [row for row in rows if isinstance(row, dict)]


def notice_source_rows() -> list[dict]:
    return _table_rows(_load_sources(), "file", required=True)


def crate_pin_rows() -> list[dict]:
    return _table_rows(_load_sources(), "crate", required=False)


def fallback_reasons() -> dict[tuple[str, str], str]:
    found: dict[tuple[str, str], str] = {}
    for row in _table_rows(_load_sources(), "fallback", required=False):
        name = str(row.get("name") or "")
        version = str(row.get("version") or "")
        reason = str(row.get("reason") or "")
        if name and version and reason:
            found[(name, version)] = reason
    return found


def digest_rows(data: dict | None = None) -> list[dict]:
    """File rows and crate pins. Both are checked by sha256."""
    if data is None:
        loaded = _load_sources()
    else:
        loaded = data
    rows = _table_rows(loaded, "file", required=True)
    rows.extend(_table_rows(loaded, "crate", required=False))
    rows.extend(_table_rows(loaded, "earlier", required=False))
    return rows


def rust_std_row() -> dict:
    for row in notice_source_rows():
        if row.get("identifier") == "rust-std":
            return row
    raise BinariesError("ci/notices/sources.toml has no rust-std row")


def pinned_digest_problems(
        overrides: dict[str, bytes] | None = None,
        rows: list[dict] | None = None) -> list[str]:
    """Every pinned notice file, checked against the digest in sources.toml."""
    problems: list[str] = []
    overrides = overrides or {}
    for row in (rows if rows is not None else digest_rows()):
        relative = str(row.get("path") or "")
        expected = str(row.get("sha256") or "")
        if not relative or not expected:
            problems.append("ci/notices/sources.toml has a row with no path or sha256")
            continue
        if relative in overrides:
            blob = overrides[relative]
        else:
            path = ROOT / relative
            if not path.is_file():
                problems.append(f"{relative} is absent")
                continue
            blob = path.read_bytes()
        found = hashlib.sha256(blob).hexdigest()
        if found != expected:
            problems.append(
                f"{relative} sha256 is {found}, expected {expected}")
    return problems


def toolchain_channel() -> str:
    data = tomllib.loads((ROOT / "rust-toolchain.toml").read_text(encoding="utf-8"))
    channel = data.get("toolchain", {}).get("channel")
    if not isinstance(channel, str) or channel == "":
        raise BinariesError("rust-toolchain.toml has no channel")
    return channel


def toolchain_mismatch_message(channel: str, recorded: str) -> str:
    return (
        f"rust-toolchain.toml channel is {channel} and the pinned standard "
        f"library notice records {recorded}. Install rust-docs with "
        f"`rustup component add rust-docs` and regenerate with "
        f"`python3 scripts/licence_check.py --write binaries`."
    )


def toolchain_problems() -> list[str]:
    recorded = str(rust_std_row().get("toolchain") or "")
    channel = toolchain_channel()
    if recorded == channel:
        return []
    return [toolchain_mismatch_message(channel, recorded)]


def _authors_line(authors: tuple[str, ...]) -> str:
    cleaned = [
        author.replace("\r\n", " ").replace("\r", " ").replace("\n", " ")
        for author in authors
    ]
    return "authors: " + ", ".join(cleaned)


def copyright_holder(name: str, authors: tuple[str, ...], repository: str) -> str:
    """The copyright line for a template. No year is invented."""
    cleaned = [
        author.replace("\r\n", " ").replace("\r", " ").replace("\n", " ").strip()
        for author in authors
    ]
    cleaned = [author for author in cleaned if author]
    if cleaned:
        return ", ".join(cleaned)
    repo = repository.strip()
    if repo:
        return f"the {name} authors ({repo})"
    return f"the {name} authors"


def fill_copyright(blob: bytes, holder: str) -> bytes:
    """Fill a MIT or Zlib marker.

    Apache-2.0 stays exactly as published. The appendix line is
    instructions to a licensor, and this function returns those bytes
    unchanged.
    """
    if _APACHE_BOILERPLATE.encode() in blob:
        return blob
    text = blob.decode("utf-8")
    text = text.replace("<copyright-holder>", holder)
    return text.encode("utf-8")


def template_holder_problem(identifier: str, blob: bytes) -> str | None:
    """A MIT or Zlib template may carry the marker and nothing else.

    Apache-2.0 is the published text and is not checked here.
    """
    if identifier.casefold() not in {"mit", "zlib"}:
        return None
    try:
        text = blob.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise BinariesError(
            f"pinned text {identifier} is not valid UTF-8") from exc
    lines = [line.strip() for line in text.splitlines()
             if _COPYRIGHT_LINE.match(line)]
    if lines != [_COPYRIGHT_MARKER] or _EMAIL.search(text):
        return f"pinned template {identifier} names another project's holder"
    return None


def unfilled_placeholder_problem(blob: bytes) -> str | None:
    try:
        text = blob.decode("utf-8")
    except UnicodeDecodeError:
        return None
    if any(marker in text for marker in _UNFILLED):
        return "unfilled placeholder in the output"
    return None


def _pinned_files(rows: list[dict], name: str, version: str) -> list[tuple[str, bytes]]:
    found: list[tuple[str, bytes]] = []
    for row in rows:
        if str(row.get("name") or "") != name:
            continue
        if str(row.get("version") or "") != version:
            continue
        relative = str(row.get("path") or "")
        if not relative:
            raise BinariesError(f"{name} {version} pin has no path")
        path = ROOT / relative
        if not path.is_file():
            raise BinariesError(f"{relative} is absent")
        found.append((Path(relative).name, path.read_bytes()))
    found.sort(key=lambda item: item[0])
    return found


def _row_for(rows: list[dict], name: str, version: str) -> dict | None:
    for row in rows:
        if str(row.get("name") or "") != name:
            continue
        if str(row.get("version") or "") != version:
            continue
        return row
    return None


def upstream_files(name: str, version: str) -> list[tuple[str, bytes]]:
    return _pinned_files(crate_pin_rows(), name, version)


def earlier_files(name: str, version: str) -> list[tuple[str, bytes]]:
    rows = _table_rows(_load_sources(), "earlier", required=False)
    return _pinned_files(rows, name, version)


def url_commit(url: str) -> str | None:
    """The commit named by a raw.githubusercontent.com URL, or None."""
    match = _RAW_COMMIT.match(url.strip())
    if match is None:
        return None
    return match.group(1).lower()


def _registry_roots() -> list[Path]:
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    src = cargo_home / "registry" / "src"
    if not src.is_dir():
        return []
    return sorted(path for path in src.iterdir() if path.is_dir())


def published_sha1(name: str, version: str) -> str | None:
    """The git sha1 in the package's .cargo_vcs_info.json, read offline."""
    folder = f"{name}-{version}"
    for root in _registry_roots():
        path = root / folder / ".cargo_vcs_info.json"
        if not path.is_file():
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return None
        git = data.get("git")
        if not isinstance(git, dict):
            return None
        sha = git.get("sha1")
        if isinstance(sha, str) and re.fullmatch(r"[0-9a-fA-F]{40}", sha):
            return sha.lower()
        return None
    return None


def _same_commit(url_sha: str, published: str) -> bool:
    if url_sha == published:
        return True
    return len(url_sha) >= 7 and published.startswith(url_sha)


def published_commit_problems(
        rows: list[dict] | None = None,
        sha1_of=None) -> list[str]:
    """Refuse a crate pin whose URL names a commit other than the published one.

    The published commit is read from .cargo_vcs_info.json in the registry
    cache. sha1_of, when given, maps a name and version to that sha1 and
    the cache is not read. The check does not use the network.
    """
    problems: list[str] = []
    chosen = crate_pin_rows() if rows is None else rows
    lookup = published_sha1 if sha1_of is None else sha1_of
    for row in chosen:
        name = str(row.get("name") or "")
        version = str(row.get("version") or "")
        url = str(row.get("url") or "")
        label = f"{name} {version}".strip() or "crate pin"
        found = url_commit(url)
        published = lookup(name, version) if found is not None else None
        shown_url = found if found is not None else "none"
        shown_published = published if published else "absent"
        if (found is None or not published
                or not _same_commit(found, published)):
            problems.append(
                f"{label} pin URL commit {shown_url} differs from the "
                f"published commit {shown_published}")
    return problems


def raw_pin(url: str) -> tuple[str, str, str] | None:
    """`(https://github.com/owner/repo, commit, path)` from a raw pin URL."""
    match = _RAW_PIN.match(url.strip())
    if match is None:
        return None
    owner = match.group("owner")
    repo = match.group("repo")
    commit = match.group("commit").lower()
    path = match.group("path")
    return (f"https://github.com/{owner}/{repo}", commit, path)


def pin_repository_and_commit(row: dict) -> tuple[str, str]:
    """The repository and commit a shipped sentence names, taken from the row."""
    name = str(row.get("name") or "")
    version = str(row.get("version") or "")
    label = f"{name} {version}".strip() or "pin"
    parsed = raw_pin(str(row.get("url") or ""))
    if parsed is None:
        raise BinariesError(f"{label} pin URL names no commit")
    repository, url_commit, _path = parsed
    claimed = str(row.get("commit") or "").strip().lower()
    if claimed != "":
        return repository, claimed
    return repository, url_commit


def statement_sentence(
        repository: str, commit: str, filename: str,
        earlier_repository: str = "", earlier_commit: str = "",
        earlier_filename: str = "") -> str:
    """The no-licence-text sentence. The commit and repository come from the row."""
    sentence = (
        "Neither the published package nor its repository "
        f"{repository} at commit {commit} carries licence text. "
        f"The text below is {filename} from commit {commit} of "
        f"{repository}, then the standard text of the licence the "
        "crate declares"
    )
    if earlier_filename:
        sentence += (
            f", then the repository root {earlier_filename} from "
            f"commit {earlier_commit} of {earlier_repository}"
        )
    return sentence + "."


def attribution_problems(blob: bytes) -> list[str]:
    """The shipped file must carry the CC BY credit for the MIT template."""
    try:
        text = blob.decode("utf-8")
    except UnicodeDecodeError:
        return ["CC BY attribution paragraph is absent"]
    if all(phrase in text for phrase in _CC_BY_REQUIRED):
        return []
    return ["CC BY attribution paragraph is absent"]


def fetch_raw_pin(url: str) -> bytes | None:
    """Bytes at a raw pin URL, or None when that path is absent at the commit.

    A transport failure raises BinariesError. Failing to read the commit
    is a refusal, not a skip. The result is cached for this process.
    verify-upstream is the caller. The binaries gate does not call this.
    """
    cached = _RAW_PIN_CACHE.get(url)
    if cached is not None or url in _RAW_PIN_CACHE:
        return cached
    request = urllib.request.Request(
        url, headers={"User-Agent": "ocelli-licence-check"})
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            blob = response.read()
    except urllib.error.HTTPError as exc:
        if exc.code == 404:
            _RAW_PIN_CACHE[url] = None
            return None
        raise BinariesError(
            f"pin URL {url} could not be read: HTTP {exc.code}") from exc
    except urllib.error.URLError as exc:
        raise BinariesError(
            f"pin URL {url} could not be read: {exc.reason}") from exc
    except OSError as exc:
        raise BinariesError(
            f"pin URL {url} could not be read: {exc}") from exc
    _RAW_PIN_CACHE[url] = blob
    return blob


def earlier_commit_problems(
        rows: list[dict] | None = None,
        blob_at=None) -> list[str]:
    """Refuse an earlier pin whose URL is not the commit the row claims.

    The pinned file must exist locally and its digest must be the row's
    sha256. The gate calls this with no blob_at and does not fetch. The
    recorded commit is what the shipped sentence names. blob_at, when
    given, maps a URL to the bytes at that commit, or to None when the
    path is absent. A missing path is a refusal, not a match.
    verify-upstream passes the network read as blob_at.
    """
    problems: list[str] = []
    if rows is None:
        chosen = _table_rows(_load_sources(), "earlier", required=False)
    else:
        chosen = rows
    seen: dict[str, bytes | None] = {}
    for row in chosen:
        name = str(row.get("name") or "")
        version = str(row.get("version") or "")
        url = str(row.get("url") or "")
        relative = str(row.get("path") or "")
        expected = str(row.get("sha256") or "").lower()
        label = f"{name} {version}".strip() or "earlier pin"
        if not relative or not expected:
            problems.append(
                "ci/notices/sources.toml has a row with no path or sha256")
            continue
        parsed = raw_pin(url)
        found = parsed[1] if parsed is not None else None
        claimed = str(row.get("commit") or "").strip().lower()
        shown_url = found if found is not None else "none"
        shown_claimed = claimed if claimed else "absent"
        if (found is None or claimed == ""
                or not _same_commit(found, claimed)):
            problems.append(
                f"{label} earlier pin URL commit {shown_url} differs from "
                f"the commit the row claims {shown_claimed}")
            continue
        filename = parsed[2] if parsed is not None else Path(relative).name
        if blob_at is not None:
            if url not in seen:
                seen[url] = blob_at(url)
            blob = seen[url]
            if blob is None:
                problems.append(
                    f"{label} {filename} is absent at commit {claimed}")
                continue
            found_digest = hashlib.sha256(blob).hexdigest()
            if found_digest != expected:
                problems.append(
                    f"{label} {filename} at commit {claimed} sha256 is "
                    f"{found_digest}, expected {expected}")
                continue
        path = ROOT / relative
        if not path.is_file():
            problems.append(f"{relative} is absent")
            continue
        local_digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if local_digest != expected:
            problems.append(
                f"{relative} sha256 is {local_digest}, expected {expected}")
    return problems


def upstream_pin_rows() -> list[dict]:
    """Crate pins and earlier pins, each carrying the commit its URL names.

    A crate row records that commit in the URL. An earlier row records it
    in the commit field. The copy is shallow, so the loaded table stays as
    it was read.
    """
    rows: list[dict] = []
    for row in crate_pin_rows():
        copied = dict(row)
        if str(copied.get("commit") or "").strip() == "":
            parsed = raw_pin(str(copied.get("url") or ""))
            if parsed is not None:
                copied["commit"] = parsed[1]
        rows.append(copied)
    rows.extend(_table_rows(_load_sources(), "earlier", required=False))
    return rows


def verify_upstream_problems(blob_at=None) -> list[str]:
    """Re-fetch every crate pin and every earlier pin, and compare bytes.

    The comparison is against the pinned file's recorded sha256. blob_at,
    when given, stands in for the network and returns None when the path
    is absent at that commit. A difference is a refusal, and so is a
    missing file. The floor gate does not call this.
    """
    fetch = fetch_raw_pin if blob_at is None else blob_at
    try:
        return earlier_commit_problems(upstream_pin_rows(), blob_at=fetch)
    except BinariesError as exc:
        return [str(exc)]


def _text_chunk(raw: bytes) -> bytes:
    text = normalise_newlines(raw)
    if text.endswith(b"\n"):
        text = text[:-1]
    return text


def render_notices(
        crates: list[CrateNotice],
        pins: dict[str, bytes],
        standard_library: bytes,
        toolchain: str,
        allow: set[str],
        skip_names: set[str] | None = None) -> bytes:
    """The notice file. Crates are sorted by name, then version, then source."""
    for ident, blob in pins.items():
        problem = template_holder_problem(ident, blob)
        if problem:
            raise BinariesError(problem)
    skipped = skip_names or set()
    chosen = [crate for crate in crates if crate.name not in skipped]
    chosen.sort(key=lambda crate: (crate.name, crate.version, crate.source))

    # (name, version, file, authors, kind, sentence, normalised text).
    # kind is packaged, upstream, template, statement, or statement-earlier.
    # sentence is empty except for the two statement kinds.
    carriers: list[tuple[str, str, str, tuple[str, ...], str, str, bytes]] = []
    for crate in chosen:
        if crate.files and crate.notice == "statement":
            if crate.expression == "":
                raise BinariesError(
                    f"{crate.name} {crate.version} has no licence file and "
                    f"no licence expression")
            try:
                terms = elect_terms(crate.expression, allow)
            except BinariesError as exc:
                raise BinariesError(
                    f"{crate.name} {crate.version} has no allowlisted "
                    f"licence in {crate.expression}") from exc
            holder = copyright_holder(
                crate.name, crate.authors, crate.repository)
            chunks: list[bytes] = []
            for relative, raw in crate.files:
                try:
                    raw.decode("utf-8")
                except UnicodeDecodeError as exc:
                    raise BinariesError(
                        f"{crate.name} {crate.version} {relative} "
                        f"is not valid UTF-8") from exc
                chunks.append(_text_chunk(raw))
            for term in terms:
                blob = pins.get(term.casefold())
                if blob is None:
                    raise BinariesError(
                        f"{crate.name} {crate.version} has no pinned text "
                        f"for {term}")
                try:
                    filled = fill_copyright(blob, holder)
                except UnicodeDecodeError as exc:
                    raise BinariesError(
                        f"pinned text {term} is not valid UTF-8") from exc
                chunks.append(_text_chunk(filled))
            if crate.pin_repository == "" or crate.commit == "":
                raise BinariesError(
                    f"{crate.name} {crate.version} statement names no "
                    f"repository or commit")
            filename = crate.files[0][0]
            earlier_filename = ""
            if crate.earlier:
                if (crate.earlier_repository == ""
                        or crate.earlier_commit == ""):
                    raise BinariesError(
                        f"{crate.name} {crate.version} earlier notice names "
                        f"no repository or commit")
                earlier_filename = crate.earlier[0][0]
                for relative, raw in crate.earlier:
                    try:
                        raw.decode("utf-8")
                    except UnicodeDecodeError as exc:
                        raise BinariesError(
                            f"{crate.name} {crate.version} {relative} "
                            f"is not valid UTF-8") from exc
                    chunks.append(_text_chunk(raw))
            sentence = statement_sentence(
                crate.pin_repository, crate.commit, filename,
                crate.earlier_repository, crate.earlier_commit,
                earlier_filename)
            kind = "statement-earlier" if crate.earlier else "statement"
            body = b"\n\n".join(chunks) + b"\n"
            label = filename if len(crate.files) == 1 else "NOTICE"
            carriers.append((
                crate.name, crate.version, label, crate.authors,
                kind, sentence, body))
            continue
        if crate.files:
            kind = "upstream" if crate.notice == "upstream" else "packaged"
            for relative, raw in crate.files:
                try:
                    raw.decode("utf-8")
                except UnicodeDecodeError as exc:
                    raise BinariesError(
                        f"{crate.name} {crate.version} {relative} "
                        f"is not valid UTF-8") from exc
                carriers.append((
                    crate.name, crate.version, relative, crate.authors,
                    kind, "", normalise_newlines(raw)))
            continue
        if crate.expression == "":
            raise BinariesError(
                f"{crate.name} {crate.version} has no licence file and "
                f"no licence expression")
        try:
            terms = elect_terms(crate.expression, allow)
        except BinariesError as exc:
            raise BinariesError(
                f"{crate.name} {crate.version} {exc}") from exc
        holder = copyright_holder(crate.name, crate.authors, crate.repository)
        for term in terms:
            blob = pins.get(term.casefold())
            if blob is None:
                raise BinariesError(
                    f"{crate.name} {crate.version} has no pinned text for {term}")
            try:
                filled = fill_copyright(blob, holder)
            except UnicodeDecodeError as exc:
                raise BinariesError(
                    f"pinned text {term} is not valid UTF-8") from exc
            carriers.append((
                crate.name, crate.version, term, crate.authors,
                "template", "", normalise_newlines(filled)))

    groups: dict[str, list[tuple[str, str, str, tuple[str, ...], str, str]]] = {}
    texts: dict[str, bytes] = {}
    order: list[str] = []
    for name, version, file, authors, kind, sentence, text in carriers:
        digest = hashlib.sha256(text).hexdigest()
        if digest not in groups:
            order.append(digest)
            groups[digest] = []
            texts[digest] = text
        groups[digest].append((name, version, file, authors, kind, sentence))

    parts: list[bytes] = []
    parts.append((
        f"THIRD_PARTY_LICENSES\n"
        "\n"
        "Generated by `python3 scripts/licence_check.py --write binaries`.\n"
        "Do not edit.\n"
        "Regenerate this file after a change to Cargo.lock or to the channel "
        "in rust-toolchain.toml.\n"
        f"{_CC_BY_ATTRIBUTION}\n"
        "Closure: third-party crates reachable on a normal or build edge from "
        "the wasm entry point for wasm32-unknown-unknown, and from the native "
        "entry point with no platform filter.\n"
        "Workspace members are excluded.\n"
        "A build-dependency crate and a proc-macro crate stay in the list, "
        "because this is the closure the licence gate checks.\n"
        "\n"
        "## Crate index\n"
        "\n"
    ).encode())
    for crate in chosen:
        expression = crate.expression if crate.expression else "unspecified"
        parts.append((
            f"name: {crate.name}\n"
            f"version: {crate.version}\n"
            f"source: {crate.source}\n"
            f"expression: {expression}\n"
            "\n"
        ).encode())
    parts.append(b"## Licence texts\n\n")
    for digest in order:
        parts.append(f"sha256: {digest}\n".encode())
        for name, version, file, authors, kind, sentence in groups[digest]:
            parts.append(f"carrier: {name} {version} {file}\n".encode())
            if kind == "upstream":
                parts.append((_authors_line(authors) + "\n").encode())
                parts.append((_UPSTREAM_SENTENCE + "\n").encode())
            elif kind == "template":
                parts.append((_authors_line(authors) + "\n").encode())
                parts.append((_NO_FILE_SENTENCE + "\n").encode())
            elif kind in {"statement", "statement-earlier"}:
                parts.append((_authors_line(authors) + "\n").encode())
                parts.append((sentence + "\n").encode())
        parts.append(b"\n")
        parts.append(texts[digest])
        if not texts[digest].endswith(b"\n"):
            parts.append(b"\n")
        parts.append(b"-----END LICENCE TEXT-----\n\n")
    parts.append(b"## Rust standard library\n\n")
    parts.append(f"Toolchain {toolchain}.\n".encode())
    parts.append(
        b"The bytes between the markers are the pinned toolchain notice.\n\n")
    parts.append(_STD_BEGIN)
    parts.append(standard_library)
    parts.append(_STD_END)
    parts.append(b"\n")
    blob = b"".join(parts)
    problem = unfilled_placeholder_problem(blob)
    if problem:
        raise BinariesError(problem)
    return blob


def _standard_texts() -> dict[str, bytes]:
    found: dict[str, bytes] = {}
    for row in notice_source_rows():
        if row.get("identifier") == "rust-std":
            continue
        ident = str(row.get("identifier") or "")
        relative = str(row.get("path") or "")
        if not ident or not relative:
            continue
        found[ident.casefold()] = (ROOT / relative).read_bytes()
    return found


def crate_from_package(package: dict) -> CrateNotice:
    name = str(package.get("name") or "")
    version = str(package.get("version") or "")
    expression = package.get("license")
    if not isinstance(expression, str):
        expression = ""
    authors = package.get("authors") or []
    if not isinstance(authors, list):
        authors = []
    repository = package.get("repository")
    if not isinstance(repository, str):
        repository = ""
    license_file = package.get("license_file")
    if not isinstance(license_file, str) or license_file == "":
        license_file = None
    manifest = str(package.get("manifest_path") or "")
    files: tuple[tuple[str, bytes], ...] = ()
    if manifest:
        files = tuple(collect_licence_files(Path(manifest).parent, license_file))
    notice = "packaged"
    earlier: tuple[tuple[str, bytes], ...] = ()
    pin_repository = ""
    commit = ""
    earlier_repository = ""
    earlier_commit = ""
    if not files:
        pinned = tuple(upstream_files(name, version))
        recorded = (name, version) in fallback_reasons()
        if pinned and recorded:
            files = pinned
            notice = "statement"
            earlier = tuple(earlier_files(name, version))
            row = _row_for(crate_pin_rows(), name, version)
            if row is None:
                raise BinariesError(
                    f"{name} {version} has no pinned upstream notice")
            pin_repository, commit = pin_repository_and_commit(row)
            if earlier:
                earlier_row = _row_for(
                    _table_rows(_load_sources(), "earlier", required=False),
                    name, version)
                if earlier_row is None:
                    raise BinariesError(
                        f"{name} {version} earlier notice has no row")
                earlier_repository, earlier_commit = (
                    pin_repository_and_commit(earlier_row))
        elif pinned:
            files = pinned
            notice = "upstream"
        elif recorded:
            notice = "template"
        else:
            raise BinariesError(
                f"{name} {version} has no pinned upstream notice "
                f"and no recorded fallback")
    return CrateNotice(
        name=name,
        version=version,
        source=source_label(package),
        expression=expression,
        authors=tuple(str(item) for item in authors),
        files=files,
        repository=repository,
        notice=notice,
        earlier=earlier,
        pin_repository=pin_repository,
        commit=commit,
        earlier_repository=earlier_repository,
        earlier_commit=earlier_commit,
    )


def render_closure(
        std_bytes: bytes | None = None,
        toolchain: str | None = None) -> bytes:
    records, members = _rust_shipped()
    commit_problems = published_commit_problems()
    if commit_problems:
        detail = "\n".join(commit_problems)
        raise BinariesError(
            f"pin URL commit differs from the published commit\n{detail}")
    # Offline. Local bytes and the recorded commit. verify-upstream fetches.
    earlier_problems = earlier_commit_problems()
    if earlier_problems:
        detail = "\n".join(earlier_problems)
        raise BinariesError(
            f"earlier pin does not match the commit it names\n{detail}")
    crates = [
        crate_from_package(package)
        for ident, package in records.items()
        if ident not in members
    ]
    row = rust_std_row()
    standard = (std_bytes if std_bytes is not None
                else (ROOT / str(row["path"])).read_bytes())
    channel = toolchain if toolchain is not None else toolchain_channel()
    return render_notices(
        crates, _standard_texts(), standard, channel, allowlist())


def generate_third_party_licenses() -> bytes:
    problems = pinned_digest_problems()
    problems.extend(toolchain_problems())
    if problems:
        raise BinariesError("\n".join(problems))
    return render_closure()


def binaries_problems() -> list[str]:
    problems = pinned_digest_problems()
    problems.extend(toolchain_problems())
    if problems:
        return problems
    try:
        blob = render_closure()
    except BinariesError as exc:
        problems.append(f"shipped binary notices: {exc}")
        return problems
    except ShippedClosureError as exc:
        problems.append(f"shipped binary notices: {exc}")
        return problems
    problems.extend(attribution_problems(blob))
    if not _LICENCES.is_file():
        problems.append("THIRD_PARTY_LICENSES is absent")
        return problems
    if _LICENCES.read_bytes() != blob:
        problems.append("THIRD_PARTY_LICENSES does not match the lockfile")
    return problems


def _rust_docs_notice() -> Path | None:
    completed = subprocess.run(
        ["rustc", "--print", "sysroot"],
        cwd=ROOT, capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        return None
    path = (Path(completed.stdout.strip()) / "share" / "doc" / "rust"
            / "COPYRIGHT-library.html")
    if path.is_file():
        return path
    return None


def rewrite_rust_std_pin(text: str, toolchain: str, digest: str) -> str:
    """Return sources.toml with the rust-std toolchain and digest replaced.

    The first `[[file]]` header stays. Crate rows that follow the file rows
    stay. Nothing is written.
    """
    parts = text.split("[[file]]")
    if len(parts) < 2:
        raise BinariesError("ci/notices/sources.toml has no file rows")
    head, *blocks = parts
    rewritten: list[str] = []
    found = False
    for block in blocks:
        if 'identifier = "rust-std"' in block:
            found = True
            block, tool_count = re.subn(
                r'toolchain = "[^"]*"',
                f'toolchain = "{toolchain}"', block, count=1)
            block, hash_count = re.subn(
                r'sha256 = "[0-9a-fA-F]+"',
                f'sha256 = "{digest}"', block, count=1)
            if tool_count != 1 or hash_count != 1:
                raise BinariesError(
                    "ci/notices/sources.toml rust-std row has no toolchain "
                    "or sha256")
        rewritten.append(block)
    if not found:
        raise BinariesError("ci/notices/sources.toml has no rust-std row")
    return "[[file]]".join([head, *rewritten])


def _atomic_write(path: Path, data: bytes) -> None:
    """Replace path with data. A failed write leaves the previous file."""
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(
        dir=path.parent, prefix=f".{path.name}.", suffix=".tmp")
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    except BaseException:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise


def write_binaries() -> list[str]:
    """Regenerate THIRD_PARTY_LICENSES. Write nothing if generation fails.

    A channel change copies the installed rust-docs notice into memory, checks
    the rewritten sources, and writes the notice, sources.toml and the
    generated file only after that check succeeds.
    """
    try:
        channel = toolchain_channel()
        row = rust_std_row()
        recorded = str(row.get("toolchain") or "")
        std_path = ROOT / str(row["path"])
        old_html = std_path.read_bytes()
        old_sources = _SOURCES.read_text(encoding="utf-8")
        pending_html: bytes | None = None
        pending_sources: str | None = None
        std_bytes = old_html
        render_channel: str | None = None
        if recorded != channel:
            docs = _rust_docs_notice()
            if docs is None:
                return [toolchain_mismatch_message(channel, recorded)]
            pending_html = docs.read_bytes()
            std_bytes = pending_html
            render_channel = channel
            pending_sources = rewrite_rust_std_pin(
                old_sources, channel, hashlib.sha256(pending_html).hexdigest())
            loaded = tomllib.loads(pending_sources)
            problems = pinned_digest_problems(
                overrides={str(row["path"]): pending_html},
                rows=digest_rows(loaded))
            if problems:
                return problems
        else:
            problems = pinned_digest_problems()
            problems.extend(toolchain_problems())
            if problems:
                return problems
        blob = render_closure(std_bytes=std_bytes, toolchain=render_channel)
    except BinariesError as exc:
        return [f"shipped binary notices: {exc}"]
    except ShippedClosureError as exc:
        return [f"shipped binary notices: {exc}"]
    except tomllib.TOMLDecodeError as exc:
        return [f"shipped binary notices: sources.toml {exc}"]

    wrote_html = False
    wrote_sources = False
    try:
        if pending_html is not None and pending_sources is not None:
            _atomic_write(std_path, pending_html)
            wrote_html = True
            _atomic_write(_SOURCES, pending_sources.encode("utf-8"))
            wrote_sources = True
        _atomic_write(_LICENCES, blob)
    except OSError as exc:
        if wrote_sources:
            _atomic_write(_SOURCES, old_sources.encode("utf-8"))
        if wrote_html:
            _atomic_write(std_path, old_html)
        return [f"shipped binary notices: {exc}"]
    return []


# ---------------------------------------------------------------------------
# Entry
# ---------------------------------------------------------------------------

def section_problems(name: str, allow: set[str]) -> list[str]:
    if name == "rust":
        return rust_problems()
    if name == "npm":
        return npm_problems(allow)
    if name == "python":
        return python_problems(allow)
    if name == "notices":
        return notices_problems()
    if name == "holder":
        return holder_problems()
    if name == "binaries":
        return binaries_problems()
    return [f"unknown section {name}"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sections", nargs="*",
                        choices=[*FLOOR_SECTIONS, "installed",
                                 "verify-upstream"])
    parser.add_argument("--python", default=None)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args(argv)
    selected = list(args.sections) or list(FLOOR_SECTIONS)
    if args.write:
        if selected != ["binaries"]:
            print("FAIL: licence check")
            print("  --write regenerates the binaries section only")
            return 1
        problems = write_binaries()
        if problems:
            print("FAIL: licence check")
            for problem in problems:
                print(f"  {problem}")
            return 1
        print("OK: licence check (binaries)")
        return 0
    if selected == ["verify-upstream"]:
        problems = verify_upstream_problems()
        if problems:
            print("FAIL: licence check")
            for problem in problems:
                print(f"  {problem}")
            return 1
        print("OK: licence check (verify-upstream)")
        return 0
    if "verify-upstream" in selected:
        print("FAIL: licence check")
        print("  verify-upstream runs alone")
        return 1

    problems: list[str] = []
    # Exception shape is shared. A broken row fails every floor section and
    # the wheel scan, because an exception that cannot be read is not an
    # approval.
    if any(name in selected for name in ("rust", "npm", "python", "installed")):
        problems.extend(exception_problems())
        problems.extend(stale_problems())
        problems.extend(licence_file_waiver_problems())
        problems.extend(scope_problems())

    allow = allowlist()
    for name in selected:
        if name == "installed":
            problems.extend(installed_from_interpreter(args.python))
        else:
            problems.extend(section_problems(name, allow))

    if problems:
        print("FAIL: licence check")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: licence check (" + ", ".join(selected) + ")")
    return 0


if __name__ == "__main__":
    sys.exit(main())
