#!/usr/bin/env python3
"""One allowlist, read three ways.

deny.toml is the allowlist. cargo-deny 0.19.0 reads it for Rust. This script
reads the same list for npm and Python, checks the third-party notices and
the copyright holder, and scans installed wheels when asked.

    python3 scripts/licence_check.py [rust|npm|python|notices|holder]...
    python3 scripts/licence_check.py installed [--python PATH]

With no section, the five floor checks run. installed is the wheel scan and
is not part of that default.
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
FLOOR_SECTIONS = ("rust", "npm", "python", "notices", "holder")


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
    """A row that waives a refusal needs the same fields as an exception."""
    problems: list[str] = []
    source = licence_file_rows() if rows is None else rows
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
        note = str(record.get("note", "") if record else "")
        spdx = str(record.get("spdx", "") if record else "")
        if spdx != "BSD-2-Clause" or "not taken" not in note:
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


def _rust_shipped() -> set[str]:
    """Crate names reachable from the two entry points on normal and build edges."""
    names: set[str] = set()
    queries = (
        ("ocelli-wasm", ["--filter-platform", "wasm32-unknown-unknown"]),
        ("ocelli-native", []),
    )
    for root_name, extra in queries:
        completed = subprocess.run(
            ["cargo", "metadata", "--format-version", "1", "--locked", *extra],
            cwd=ROOT, capture_output=True, text=True, check=False)
        if completed.returncode != 0:
            raise RuntimeError(root_name)
        data = json.loads(completed.stdout)
        packages = {item["id"]: item for item in data.get("packages", [])}
        nodes = {item["id"]: item for item in data.get("resolve", {}).get("nodes", [])}
        roots = [item["id"] for item in data.get("packages", [])
                 if item.get("name") == root_name]
        stack = list(roots)
        seen: set[str] = set()
        while stack:
            ident = stack.pop()
            if ident in seen:
                continue
            seen.add(ident)
            package = packages.get(ident)
            if package and package.get("name"):
                names.add(str(package["name"]))
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
    return names


def _rust_exception_problems() -> list[str]:
    rows = [row for row in exception_rows()
            if str(row.get("ecosystem", "")) == "rust"]
    if not rows:
        return []
    try:
        shipped = _rust_shipped()
    except (RuntimeError, json.JSONDecodeError, OSError):
        return ["rust shipped-closure walk failed"]
    problems = []
    for row in rows:
        package = str(row.get("package", ""))
        version = str(row.get("version", ""))
        if package in shipped:
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
        # vendor/ritk-codecs-0.6.0/LICENSE-MIT is Ryan Clanton's grant. Its
        # bytes are pinned separately and are not this project's copyright.
        if relative.startswith("vendor/"):
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
    return [f"unknown section {name}"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sections", nargs="*",
                        choices=[*FLOOR_SECTIONS, "installed"])
    parser.add_argument("--python", default=None)
    args = parser.parse_args(argv)
    selected = list(args.sections) or list(FLOOR_SECTIONS)

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
