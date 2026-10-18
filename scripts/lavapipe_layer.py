#!/usr/bin/env python3
"""F-X002 layer 2 record.

A missing ``ci/lavapipe-layer.json`` means the probe is required. The only
other accepted document is ``{"status":"dropped","reason":"<non-empty>"}``.
``pass``, ``skip`` and an empty reason are refused. A drop prints
``layer 2 dropped: <reason>`` and does not run the probe.

Exit 0 is the drop. Exit 10 means the caller should run the probe. Exit 1
is a refused document. This script does not print a pass.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_RECORD = ROOT / "ci" / "lavapipe-layer.json"


def decide(path: Path) -> str:
    """``probe``, ``dropped`` or ``refused``."""
    if not path.is_file():
        return "probe"
    try:
        document = json.loads(path.read_text())
    except json.JSONDecodeError:
        return "refused"
    if not isinstance(document, dict):
        return "refused"
    status = document.get("status")
    reason = document.get("reason")
    if status != "dropped" or not isinstance(reason, str):
        return "refused"
    if reason.strip() == "":
        return "refused"
    return "dropped"


def drop_line(path: Path) -> str:
    """The one line a drop is allowed to print."""
    document = json.loads(path.read_text())
    reason = document.get("reason")
    if not isinstance(reason, str):
        return ""
    flat = " ".join(reason.split())
    return f"layer 2 dropped: {flat}"


def main(argv: list[str]) -> int:
    path = Path(argv[1]) if len(argv) > 1 else DEFAULT_RECORD
    choice = decide(path)
    if choice == "dropped":
        print(drop_line(path))
        return 0
    if choice == "probe":
        return 10
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
