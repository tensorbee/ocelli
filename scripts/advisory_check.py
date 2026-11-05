#!/usr/bin/env python3
"""High and critical advisories in the published npm runtime.

`npm audit` reports what is installed. A root `overrides` entry changes this
repository's lockfile and never reaches a consumer of `@ocelli/core`, so the
gate refuses that shape and accepts one recorded exception instead. The
exception stands only while `gate packages` checks that no shipped entry
point loads the package.

    python3 scripts/advisory_check.py [exceptions|npm]...
    python3 scripts/advisory_check.py npm --report PATH

With no section, both run. `exceptions` is offline. `npm` asks the registry
unless `--report` names a saved audit document. npm's own exit status is
ignored. A missing, unreadable or erroring report fails closed.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from pathlib import Path

import licence_check

ROOT = Path(__file__).resolve().parent.parent

# npm's five severities. High and critical fail unless a row lists the
# advisory. The other three warn. Anything else fails closed. Membership,
# not alphabetical order, because "critical" sorts before "high".
REFUSED = ("critical", "high")
KNOWN = ("critical", "high", "moderate", "low", "info")
FIELDS = (
    "ecosystem",
    "package",
    "version",
    "advisories",
    "scope",
    "proof",
    "reason",
    "decided",
    "decided_by",
)
SCOPES = ("unreachable",)
PROOFS = ("packages-consumer-trace",)


def scope_is_unreachable(value: object) -> bool:
    """True only when `value` is exactly a scope in `SCOPES`.

    Padding and any other spelling are not a match. `package_check` reads
    this function for the same decision, so the advisory gate and the
    consumer proof cannot accept different rows.
    """
    return isinstance(value, str) and value in SCOPES
SECTIONS = ("exceptions", "npm")
REPORT_VERSION = 2


def _path_package(path: str) -> str:
    """The package name a lock node path installs, or empty."""
    marker = "node_modules/"
    if marker not in path:
        return ""
    return path.rsplit(marker, 1)[1]


def _field_present(row: dict, field: str) -> bool:
    value = row.get(field)
    if field == "advisories":
        return (
            isinstance(value, list)
            and len(value) > 0
            and all(isinstance(item, str) and item.strip() for item in value)
        )
    if not isinstance(value, str):
        return False
    return bool(value.strip())


def _label(row: dict) -> str:
    ecosystem = str(row.get("ecosystem", "")).strip()
    package = str(row.get("package", "")).strip()
    version = str(row.get("version", "")).strip()
    return f"{ecosystem} {package}@{version}".strip()


def _pair_present(package: str, version: str, closure: set[str],
                  versions: dict[str, str]) -> bool:
    """True when the published closure installs this package at this version."""
    if package not in closure:
        return False
    for path, found in versions.items():
        if found == version and _path_package(path) == package:
            return True
    return False


def exception_problems(rows: list[dict], closure: set[str],
                       versions: dict[str, str],
                       install_scripts: set[str]) -> list[str]:
    """Row shape, the one scope, the proof name, staleness, install scripts."""
    problems: list[str] = []
    for row in rows:
        if not isinstance(row, dict):
            problems.append("exception is missing ecosystem")
            continue
        label = _label(row)
        for field in FIELDS:
            if not _field_present(row, field):
                problems.append(f"{label} is missing {field}")
        scope = row.get("scope")
        # Blank is `is missing`. Any other spelling is compared exactly, by
        # the same function `package_check` uses to choose the rows it checks.
        if (isinstance(scope, str) and scope.strip()
                and not scope_is_unreachable(scope)):
            problems.append(f"{label} scope {scope!r} is not unreachable")
        proof = row.get("proof")
        if isinstance(proof, str) and proof.strip() and proof.strip() not in PROOFS:
            problems.append(f"{label} proof {proof!r} names no proof the packages gate runs")
        package = str(row.get("package", "")).strip()
        version = str(row.get("version", "")).strip()
        if package and version and not _pair_present(package, version, closure, versions):
            problems.append(f"{label} is a stale advisory exception")
        if package and package in install_scripts:
            problems.append(f"{label} has an install script")
    return problems


def _override_name(key: str) -> str:
    """The package an overrides key names.

    npm writes `name` and `name@version`. A scoped name already starts with
    `@`, so its version is the second `@`. `.` selects the parent package
    and names nothing of its own.
    """
    if not key or key == ".":
        return ""
    if key.startswith("@"):
        body = key[1:]
        if "@" in body:
            return "@" + body.split("@", 1)[0]
        return key
    if "@" in key:
        return key.split("@", 1)[0]
    return key


def override_problems(overrides: dict, closure: set[str]) -> list[str]:
    """Any key, at any depth, that names a package in the published closure."""
    problems: list[str] = []
    seen: set[str] = set()

    def walk(node: object) -> None:
        if not isinstance(node, dict):
            return
        for key, value in node.items():
            name = _override_name(str(key))
            if name in closure and name not in seen:
                seen.add(name)
                problems.append(f"override {name} overrides do not reach a consumer")
            walk(value)

    walk(overrides)
    return problems


def _ghsa(via: dict) -> str:
    """The advisory id, which is the last path segment of the via URL."""
    url = via.get("url")
    if not isinstance(url, str) or not url.strip():
        return ""
    return url.rstrip("/").rsplit("/", 1)[-1]


def _report_unusable(report: object) -> bool:
    if not isinstance(report, dict) or not report:
        return True
    if "error" in report:
        return True
    if "auditReportVersion" not in report:
        return True
    return not isinstance(report.get("vulnerabilities"), dict)


def _row_covers(rows: list[dict], package: str, version: str | None,
                ghsa: str) -> bool:
    if not ghsa or not isinstance(version, str):
        return False
    for row in rows:
        if not isinstance(row, dict):
            continue
        if str(row.get("package", "")) != package:
            continue
        if str(row.get("version", "")) != version:
            continue
        listed = row.get("advisories")
        if isinstance(listed, list) and ghsa in listed:
            return True
    return False


def _carried(report: dict, package: str, version: str,
             versions: dict[str, str]) -> set[str]:
    """Advisory ids the report attributes to this package at this version."""
    vuln = report["vulnerabilities"].get(package)
    if not isinstance(vuln, dict):
        return set()
    nodes = vuln.get("nodes")
    if not isinstance(nodes, list):
        return set()
    matched = False
    for node in nodes:
        if isinstance(node, str) and versions.get(node) == version:
            matched = True
            break
    if not matched:
        return set()
    found: set[str] = set()
    vias = vuln.get("via")
    if not isinstance(vias, list):
        return set()
    for via in vias:
        if isinstance(via, dict):
            ghsa = _ghsa(via)
            if ghsa:
                found.add(ghsa)
    return found


def _closure_node(node: object, closure: set[str],
                  versions: dict[str, str]) -> bool:
    """True when this lock node is an install the published closure ships.

    `versions` is keyed by node path. `closure` is keyed by the dependency
    name. An npm alias installs `adm-zip` at `node_modules/zipper`, so the
    audit names `adm-zip` and the closure names `zipper`. The path counts
    when `versions` carries it and that directory name is in the closure.
    A versions entry whose directory is outside the closure is not shipped.
    """
    return (isinstance(node, str)
            and node in versions
            and _path_package(node) in closure)


def _in_shipped_closure(package: object, vuln: dict, closure: set[str],
                        versions: dict[str, str]) -> bool:
    """True when the published closure installs this vulnerability."""
    if isinstance(package, str) and package in closure:
        return True
    nodes = vuln.get("nodes")
    if not isinstance(nodes, list):
        return False
    return any(_closure_node(node, closure, versions) for node in nodes)


def audit_problems(report: object, closure: set[str],
                   versions: dict[str, str],
                   rows: list[dict]) -> tuple[list[str], list[str]]:
    """Problems and warnings for one audit report.

    String `via` entries are attribution. dcmjs names adm-zip that way and
    has no advisory of its own, so those entries are skipped. Every node
    path of a closure package is judged, so two versions of one name cannot
    hide behind the version a row happens to name. An alias is in the
    closure when any of its nodes is a `versions` key whose directory name
    is in the closure.
    """
    problems: list[str] = []
    warnings: list[str] = []
    if not isinstance(report, dict) or _report_unusable(report):
        problems.append("npm audit did not produce a report")
        return problems, warnings
    if report.get("auditReportVersion") != REPORT_VERSION:
        problems.append(f"npm audit report version {report.get('auditReportVersion')!r} is not 2")
        return problems, warnings

    judged: set[tuple[str, str | None, str, str]] = set()
    vulnerabilities = report["vulnerabilities"]
    for package, vuln in vulnerabilities.items():
        if not isinstance(vuln, dict) or not _in_shipped_closure(
                package, vuln, closure, versions):
            continue
        nodes = vuln.get("nodes")
        if not isinstance(nodes, list) or not nodes:
            nodes = [None]
        elif not (isinstance(package, str) and package in closure):
            # Entered by a node path. Judge the installs the closure records
            # and not a node the lock does not carry.
            nodes = [node for node in nodes
                     if _closure_node(node, closure, versions)]
        vias = vuln.get("via")
        if not isinstance(vias, list):
            continue
        for node in nodes:
            version = versions.get(node) if isinstance(node, str) else None
            for via in vias:
                if isinstance(via, str):
                    continue
                severity = via.get("severity") if isinstance(via, dict) else None
                ghsa = _ghsa(via) if isinstance(via, dict) else ""
                if not isinstance(via, dict) or severity not in KNOWN:
                    key = (str(package), version, ghsa, "unknown")
                    if key not in judged:
                        judged.add(key)
                        problems.append(f"{package}@{version} {ghsa} has unknown severity {severity!r}")
                    continue
                if severity in REFUSED:
                    if not _row_covers(rows, str(package), version, ghsa):
                        key = (str(package), version, ghsa, "refused")
                        if key not in judged:
                            judged.add(key)
                            problems.append(f"{package}@{version} {ghsa} has no exception")
                else:
                    key = (str(package), version, ghsa, str(severity))
                    if key not in judged:
                        judged.add(key)
                        warnings.append(f"{ghsa} on {package}@{version} is {severity}")

    for row in rows:
        if not isinstance(row, dict):
            continue
        package = str(row.get("package", "")).strip()
        version = str(row.get("version", "")).strip()
        if not package or not version:
            continue
        if not _pair_present(package, version, closure, versions):
            continue
        listed = row.get("advisories")
        if not isinstance(listed, list):
            continue
        carried = _carried(report, package, version, versions)
        stated: set[str] = set()
        for ghsa in listed:
            if not isinstance(ghsa, str) or ghsa in stated:
                continue
            stated.add(ghsa)
            if ghsa not in carried:
                problems.append(f"{package}@{version} no longer reports {ghsa}")
    return problems, warnings


def read_rows() -> list[dict]:
    path = ROOT / "ci" / "advisories.toml"
    if not path.is_file():
        return []
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    rows = data.get("exception", [])
    if not isinstance(rows, list):
        return []
    return [row for row in rows if isinstance(row, dict)]


def closure_facts() -> tuple[set[str], dict[str, str], set[str]]:
    """Closure names, their lock node versions, and install-script names.

    Versions are every root-lock node whose package name the published
    closure reaches. Dev-only names are absent, so a high advisory on a
    tool is not this gate's refusal.
    """
    closure = licence_check.npm_runtime_closure()
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    packages = lock.get("packages", {})
    versions: dict[str, str] = {}
    scripts: set[str] = set()
    if isinstance(packages, dict):
        for key, entry in packages.items():
            if not isinstance(key, str) or not isinstance(entry, dict):
                continue
            name = _path_package(key)
            if name not in closure:
                continue
            version = entry.get("version")
            if isinstance(version, str) and version:
                versions[key] = version
            if entry.get("hasInstallScript") is True:
                scripts.add(name)
    return closure, versions, scripts


def tracked_overrides() -> list[dict]:
    """`overrides` objects from every tracked package.json."""
    completed = subprocess.run(
        ["git", "ls-files", "-z"],
        cwd=ROOT, capture_output=True, text=True, check=True)
    found: list[dict] = []
    for name in completed.stdout.split("\0"):
        if not name.endswith("package.json"):
            continue
        manifest = json.loads((ROOT / name).read_text(encoding="utf-8"))
        overrides = manifest.get("overrides")
        if isinstance(overrides, dict):
            found.append(overrides)
    return found


def load_audit(report_path: str | None) -> object:
    """The audit document, or None when it cannot be read.

    npm exits 1 when it finds an advisory. That status is not the verdict.
    """
    if report_path is not None:
        path = Path(report_path)
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            return None
    try:
        completed = subprocess.run(
            ["npm", "audit", "--omit=dev", "--package-lock-only", "--json"],
            cwd=ROOT, capture_output=True, text=True)
    except OSError:
        return None
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError:
        return None


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("sections", nargs="*", choices=SECTIONS)
    parser.add_argument("--report", default=None)
    args = parser.parse_args(argv)
    selected = list(args.sections) or list(SECTIONS)

    closure, versions, scripts = closure_facts()
    rows = read_rows()
    problems: list[str] = []
    warnings: list[str] = []

    if "exceptions" in selected:
        problems += exception_problems(rows, closure, versions, scripts)
        for overrides in tracked_overrides():
            problems += override_problems(overrides, closure)
    if "npm" in selected:
        found, warned = audit_problems(
            load_audit(args.report), closure, versions, rows)
        problems += found
        warnings += warned

    for warning in warnings:
        print(f"WARN: {warning}")
    if problems:
        print("FAIL: advisory check")
        for problem in problems:
            print(f"  {problem}")
        return 1
    if rows:
        named = ", ".join(
            f"{row.get('package')}@{row.get('version')}" for row in rows)
        print(f"OK: advisory check, exception {named}")
    else:
        print("OK: advisory check, no exception")
    return 0


if __name__ == "__main__":
    sys.exit(main())
