#!/usr/bin/env python3
"""The F-103 catalogue matches the pinned cornerstone3D 5.8.2 enums.

Without `--require-cornerstone` this reads only the committed catalogue and
checks its own counts, the pinned version and the two file digests recorded
in `packages/core/src/events.ts`.

With `--require-cornerstone` it also reads the installed enum files and
refuses a missing file, a version other than 5.8.2, a digest mismatch, or a
name present on only one side. A missing install is a refusal, never a skip.

Usage:
  python3 scripts/event_catalogue_check.py
  python3 scripts/event_catalogue_check.py --require-cornerstone
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOGUE = ROOT / "packages" / "core" / "src" / "events.ts"
CORE_EVENTS = (
    ROOT
    / "tools"
    / "oracle"
    / "node_modules"
    / "@cornerstonejs"
    / "core"
    / "dist"
    / "esm"
    / "enums"
    / "Events.js"
)
TOOLS_EVENTS = (
    ROOT
    / "tools"
    / "oracle"
    / "node_modules"
    / "@cornerstonejs"
    / "tools"
    / "dist"
    / "esm"
    / "enums"
    / "Events.js"
)
PINNED_VERSION = "5.8.2"
MEASURED_CORE = 50
MEASURED_TOOL = 53

ROW = re.compile(
    r'\{ cornerstone: "([A-Z0-9_]+)", source: "(core|tool)", '
    r'ocelli: (?:"([A-Za-z.]+)"|null), delivery: "(ring|shell|unavailable)"'
)
ENUM_KEY = re.compile(r'Events\["([A-Z0-9_]+)"\]')
DIGEST = re.compile(r"(core|tools) Events\.js sha256 ([0-9a-f]{64})")


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)
    sys.exit(1)


def catalogue_rows(text: str) -> list[tuple[str, str]]:
    rows = ROW.findall(text)
    if len(rows) != MEASURED_CORE + MEASURED_TOOL:
        fail(
            f"catalogue has {len(rows)} rows, "
            f"want {MEASURED_CORE + MEASURED_TOOL}"
        )
    names = [name for name, _source, _ocelli, _delivery in rows]
    if len(names) != len(set(names)):
        fail("catalogue repeats a cornerstone name")
    return [(name, source) for name, source, _ocelli, _delivery in rows]


def enum_names(path: Path) -> set[str]:
    if not path.is_file():
        fail(f"missing pinned enum file {path}")
    return set(ENUM_KEY.findall(path.read_text()))


def package_version(events_js: Path) -> str:
    package = events_js.parents[3] / "package.json"
    if not package.is_file():
        fail(f"missing package.json beside {events_js}")
    version = json.loads(package.read_text()).get("version")
    if version != PINNED_VERSION:
        fail(f"{package} is {version}, want {PINNED_VERSION}")
    return str(version)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--require-cornerstone", action="store_true")
    args = parser.parse_args()
    if not CATALOGUE.is_file():
        fail(f"missing {CATALOGUE}")
    text = CATALOGUE.read_text()
    if PINNED_VERSION not in text:
        fail(f"catalogue does not name {PINNED_VERSION}")
    digests = dict(DIGEST.findall(text))
    if set(digests) != {"core", "tools"}:
        fail("catalogue must record core and tools Events.js digests")
    rows = catalogue_rows(text)
    core = {name for name, source in rows if source == "core"}
    tool = {name for name, source in rows if source == "tool"}
    if len(core) != MEASURED_CORE or len(tool) != MEASURED_TOOL:
        fail(f"catalogue counts are {len(core)} core and {len(tool)} tool")
    if not args.require_cornerstone:
        print(
            f"OK: catalogue holds {len(core)} core and {len(tool)} tool "
            f"names at {PINNED_VERSION}"
        )
        return 0
    installed = {
        "core": enum_names(CORE_EVENTS),
        "tools": enum_names(TOOLS_EVENTS),
    }
    package_version(CORE_EVENTS)
    package_version(TOOLS_EVENTS)
    for label, path, names in (
        ("core", CORE_EVENTS, installed["core"]),
        ("tools", TOOLS_EVENTS, installed["tools"]),
    ):
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        if digest != digests[label]:
            fail(f"{label} Events.js sha256 is {digest}, catalogue says {digests[label]}")
        expected = core if label == "core" else tool
        missing = sorted(names - expected)
        extra = sorted(expected - names)
        if missing or extra:
            fail(f"{label} mismatch missing={missing} extra={extra}")
    print(
        f"OK: catalogue matches {PINNED_VERSION} "
        f"({len(core)} core, {len(tool)} tool)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
