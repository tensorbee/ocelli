#!/usr/bin/env python3
"""Verify the golden corpus against corpus/manifest.tsv.

The corpus is not in git (see `corpus/README.md`). The manifest is, and it is
what makes a local corpus either the corpus the tolerance policy was written
against or provably not it.

    HLD section 25.1: "Write it down once and hold it. Tuning tolerance per
    failure is how a suite stops meaning anything."

A silently different corpus does the same damage from the other direction: the
tolerance holds and the thing it is measured against moved.

Every row carries a licence and a licence URL, because a DICOM corpus assembled
from public collections carries per-collection terms and a citation
requirement, and the answer to "may we redistribute this" should not require
archaeology.

Usage:
  python3 scripts/corpus_check.py                      # verify presence + digests
  python3 scripts/corpus_check.py --manifest-only      # shape only, no data needed
  python3 scripts/corpus_check.py --fetch              # download rows carrying a url
  python3 scripts/corpus_check.py --add FILE --modality CT --category stack-window
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "corpus" / "manifest.tsv"
COLUMNS = ["path", "modality", "transfer_syntax", "category", "source",
           "licence", "licence_url", "sha256", "url"]
HEADER = "\t".join(COLUMNS)


def corpus_dir() -> Path:
    override = os.environ.get("OCELLI_CORPUS_DIR")
    return Path(override).expanduser() if override else ROOT / "corpus" / "data"


def load() -> list[dict[str, str]]:
    if not MANIFEST.exists():
        sys.exit(f"missing {MANIFEST.relative_to(ROOT)}")
    lines = MANIFEST.read_text(encoding="utf-8").splitlines()
    if not lines or lines[0] != HEADER:
        sys.exit(f"manifest header must be: {HEADER}")

    rows, seen = [], set()
    for number, line in enumerate(lines[1:], start=2):
        if not line.strip():
            continue
        fields = line.split("\t")
        if len(fields) != len(COLUMNS):
            sys.exit(f"manifest line {number} has {len(fields)} fields, "
                     f"expected {len(COLUMNS)}")
        row = dict(zip(COLUMNS, fields))
        if Path(row["path"]).is_absolute() or ".." in Path(row["path"]).parts:
            sys.exit(f"manifest line {number}: path must be relative and "
                     f"must not escape the corpus directory")
        if row["path"] in seen:
            sys.exit(f"manifest line {number}: duplicate path {row['path']}")
        seen.add(row["path"])
        if len(row["sha256"]) != 64 or not all(
                c in "0123456789abcdef" for c in row["sha256"]):
            sys.exit(f"manifest line {number}: sha256 is not a 64-char "
                     f"lowercase hex digest")
        for required in ("modality", "category", "source", "licence",
                         "licence_url"):
            if not row[required].strip():
                sys.exit(f"manifest line {number}: {required} is empty. "
                         f"A case whose licence is unrecorded cannot be "
                         f"redistributed or cited, so it is not a fixture.")
        rows.append(row)
    return rows


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def verify(rows: list[dict[str, str]], fetch: bool) -> int:
    base = corpus_dir()
    missing, wrong, ok = [], [], 0

    for row in rows:
        target = base / row["path"]
        if not target.exists() and fetch and row["url"]:
            target.parent.mkdir(parents=True, exist_ok=True)
            print(f"  fetching {row['path']}")
            urllib.request.urlretrieve(row["url"], target)
        if not target.exists():
            missing.append(row["path"])
            continue
        actual = digest(target)
        if actual != row["sha256"]:
            wrong.append(f"{row['path']}: expected {row['sha256'][:12]}..., "
                         f"got {actual[:12]}...")
        else:
            ok += 1

    print(f"corpus directory: {base}")
    print(f"  {ok} verified, {len(missing)} missing, {len(wrong)} mismatched, "
          f"{len(rows)} in manifest")

    if wrong:
        print("\nFAIL: a corpus case does not match its manifest digest.")
        print("This is not a stale-checksum problem to fix by updating the")
        print("manifest. It means the thing the tolerance policy is measured")
        print("against has moved, and the diff results were not comparable.")
        for problem in wrong:
            print(f"  {problem}")
        return 1

    if missing:
        print("\nFAIL: corpus cases are absent.")
        print("Set OCELLI_CORPUS_DIR, or run with --fetch for rows that")
        print("carry a url. See corpus/README.md.")
        for path in missing[:20]:
            print(f"  {path}")
        if len(missing) > 20:
            print(f"  ... and {len(missing) - 20} more")
        return 1

    if not rows:
        print("\nThe manifest is empty. The corpus is acquired by F-009")
        print("(E2.1) and the oracle cannot gate anything until it exists.")
    return 0


def add(args: argparse.Namespace) -> int:
    source = Path(args.add).expanduser().resolve()
    if not source.is_file():
        sys.exit(f"not a file: {source}")
    base = corpus_dir().resolve()
    try:
        relative = source.relative_to(base).as_posix()
    except ValueError:
        sys.exit(f"{source} is not inside the corpus directory {base}. "
                 f"Copy it there first, then add it.")

    rows = load()
    if any(r["path"] == relative for r in rows):
        sys.exit(f"{relative} is already in the manifest")

    row = "\t".join([
        relative, args.modality, args.transfer_syntax, args.category,
        args.source, args.licence, args.licence_url, digest(source), args.url,
    ])
    with MANIFEST.open("a", encoding="utf-8") as handle:
        handle.write(row + "\n")
    print(f"added {relative}")
    print("Commit the manifest row. Never commit the file itself, "
          ".githooks/pre-commit refuses it.")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest-only", action="store_true")
    parser.add_argument("--fetch", action="store_true")
    parser.add_argument("--add", metavar="FILE")
    parser.add_argument("--modality", default="")
    parser.add_argument("--transfer-syntax", dest="transfer_syntax", default="")
    parser.add_argument("--category", default="")
    parser.add_argument("--source", default="")
    parser.add_argument("--licence", default="")
    parser.add_argument("--licence-url", dest="licence_url", default="")
    parser.add_argument("--url", default="")
    args = parser.parse_args()

    if args.add:
        return add(args)

    rows = load()
    if args.manifest_only:
        print(f"OK: manifest shape valid, {len(rows)} rows")
        return 0
    return verify(rows, args.fetch)


if __name__ == "__main__":
    sys.exit(main())
