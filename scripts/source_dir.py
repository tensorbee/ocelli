#!/usr/bin/env python3
"""Resolve where the private source documents live.

They are outside the repository on purpose, so their location is a property of
this CLONE rather than of the project. Three sources, first match wins:

  1. $OCELLI_SOURCE_DIR                  per-invocation override
  2. .ocelli-source-path                 per-clone, gitignored, survives a move
  3. ~/Desktop/ocelli/source-documents   the bootstrap default

Recording it in a file rather than relying on an exported variable matters:
an env var has to be remembered every session, and forgetting it makes the
docs gate SKIP rather than fail, which is quiet. A wrong path should be fixed
once, not re-exported forever.

Usage:
  python3 scripts/source_dir.py               # print the resolved path
  python3 scripts/source_dir.py --set PATH    # record it for this clone
  python3 scripts/source_dir.py --check       # resolve and verify contents
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CONFIG = ROOT / ".ocelli-source-path"
DEFAULT = "~/Desktop/ocelli/source-documents"
EXPECTED = ["Ocelli-HLD.docx", "Rust-WASM-Imaging-Backlog.xlsx"]


def resolve() -> tuple[Path, str]:
    """Return (path, where it came from)."""
    env = os.environ.get("OCELLI_SOURCE_DIR")
    if env:
        return Path(env).expanduser(), "OCELLI_SOURCE_DIR"
    if CONFIG.exists():
        recorded = CONFIG.read_text().strip()
        if recorded:
            return Path(recorded).expanduser(), CONFIG.name
    return Path(DEFAULT).expanduser(), "default"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--set", metavar="PATH")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if args.set:
        target = Path(args.set).expanduser()
        missing = [f for f in EXPECTED if not (target / f).exists()]
        if missing:
            print(f"FAIL: {target} does not contain {', '.join(missing)}")
            print("Point --set at the directory holding the source documents,")
            print("not at its parent.")
            return 1
        CONFIG.write_text(str(target) + "\n")
        print(f"recorded {target} in {CONFIG.name}")
        return 0

    path, origin = resolve()
    if not args.check:
        print(path)
        return 0

    print(f"source directory: {path}   (from {origin})")
    if not path.exists():
        print("  MISSING. Set it with:")
        print("    python3 scripts/source_dir.py --set /new/path")
        return 1
    missing = [f for f in EXPECTED if not (path / f).exists()]
    if missing:
        print(f"  present but incomplete, missing: {', '.join(missing)}")
        return 1
    print(f"  OK, {len(EXPECTED)} documents present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
