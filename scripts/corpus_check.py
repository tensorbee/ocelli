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
  python3 scripts/corpus_check.py --coverage           # what the corpus is MISSING
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

# Every transfer syntax the codec registry will claim. The UIDs and their names
# are PS3.5 Annex A, which is the authority for them. HLD section 21
# (docs/hld/18-codec-registry.md) specifies the registry that claims them and
# names the two open Appendix A gates, A1 for HTJ2K and A2 for JPEG-LS, but it
# carries no list of syntaxes, so this list is not a transcription of it.
#
# Sixteen of them. Condition 4 of F-009 is therefore sixteen rows at minimum
# rather than a gesture at the common ones, and neither open gate has anything
# to be answered against until its syntaxes have cases.
REGISTRY_TRANSFER_SYNTAXES = (
    "1.2.840.10008.1.2",          # Implicit VR Little Endian
    "1.2.840.10008.1.2.1",        # Explicit VR Little Endian
    "1.2.840.10008.1.2.1.99",     # Deflated Explicit VR Little Endian
    "1.2.840.10008.1.2.2",        # Explicit VR Big Endian, retired
    "1.2.840.10008.1.2.5",        # RLE Lossless
    "1.2.840.10008.1.2.4.50",     # JPEG Baseline, process 1
    "1.2.840.10008.1.2.4.51",     # JPEG Extended, process 2 and 4
    "1.2.840.10008.1.2.4.57",     # JPEG Lossless, process 14
    "1.2.840.10008.1.2.4.70",     # JPEG Lossless, process 14 SV1
    "1.2.840.10008.1.2.4.80",     # JPEG-LS Lossless
    "1.2.840.10008.1.2.4.81",     # JPEG-LS Near-Lossless
    "1.2.840.10008.1.2.4.90",     # JPEG 2000 Lossless Only
    "1.2.840.10008.1.2.4.91",     # JPEG 2000
    "1.2.840.10008.1.2.4.201",    # HTJ2K Lossless Only
    "1.2.840.10008.1.2.4.202",    # HTJ2K Lossless Only, RPCL
    "1.2.840.10008.1.2.4.203",    # HTJ2K
)

# The `category` column is a comma-separated token list. Two facts are read out
# of it, and both are properties the manifest can be asked about with no corpus
# present, which is what makes them survivable under deviation D-04.
LAYER_TOKENS = ("synthetic", "real")

# HLD section 25.1 sets one tolerance for monochrome 16-bit and a different one
# for colour and ultrasound. An untested class has an untested tolerance, so
# the corpus is not done until both are present.
MONOCHROME_16_BIT = "mono16"
COLOUR_OR_ULTRASOUND = ("colour", "us")
CLASS_TOKENS = (MONOCHROME_16_BIT,) + COLOUR_OR_ULTRASOUND

# Documentary tokens that a later edit to the SAME row can falsify. Unlike
# `burned-in-unchecked`, which only story E22.3 can settle, `chroma-untested`
# is settled the moment someone adds a `colour` token beside it, and a stale
# one would read as a live gap forever.
CONTRADICTORY_TOKENS = (("colour", "chroma-untested"),)


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


def tokens(row: dict[str, str]) -> set[str]:
    return {part.strip() for part in row["category"].split(",") if part.strip()}


def coverage(rows: list[dict[str, str]]) -> int:
    """Answer the two coverage conditions from the manifest alone.

    This mode reads no DICOM and needs no corpus directory, deliberately.
    Deviation D-04 means CI has neither a GPU nor the corpus, so coverage is
    the part of F-009 that CI can still see. A manifest that has stopped
    covering the codec registry then fails on the pull request rather than at
    the moment someone tries to answer gate A1.

    That is a claim about a file, so here is the file: it runs in the `guards`
    job of `.github/workflows/ci.yml`. The `corpus` gate stays out of
    `--floor` because its other half verifies digests against a corpus CI
    does not have.

    Everything reported here names what is absent. "Coverage is incomplete" is
    a sentence nobody can act on.
    """
    problems: list[str] = []

    # A blank transfer syntax is named per path rather than discarded. `load()`
    # does not require the column to be non-empty and `--add` defaults the flag
    # to the empty string, so such a row is reachable through the documented
    # path, and silently dropping it would leave it counted for nothing and
    # reported as nothing. Condition 4 of this story is "at least one case per
    # transfer syntax", and a row claiming none is exactly as much of a hole as
    # a row claiming no tolerance class.
    for row in rows:
        if not row["transfer_syntax"].strip():
            problems.append(
                f"{row['path']}: declares no transfer syntax. The column is "
                f"what condition 4 is counted from, so a blank one is a case "
                f"that covers nothing. `--add` defaults --transfer-syntax to "
                f"empty, so read it out of the file rather than omitting it.")

    present = {row["transfer_syntax"].strip() for row in rows} - {""}
    missing = [uid for uid in REGISTRY_TRANSFER_SYNTAXES if uid not in present]
    if missing:
        problems.append(
            "no case for these transfer syntaxes the codec registry claims:")
        problems += [f"    {uid}" for uid in missing]

    unknown = sorted(present - set(REGISTRY_TRANSFER_SYNTAXES))
    if unknown:
        problems.append(
            "these transfer syntaxes are in the manifest and not in the "
            "registry, so either the registry list or the row is wrong:")
        problems += [f"    {uid}" for uid in unknown]

    for row in rows:
        found = tokens(row)
        layers = found & set(LAYER_TOKENS)
        if len(layers) != 1:
            problems.append(
                f"{row['path']}: category declares {len(layers)} layer tokens, "
                f"needs exactly one of {', '.join(LAYER_TOKENS)}. A case is "
                f"generated by this repository or it is not, and 'at least one "
                f"row is not synthetic' is otherwise unanswerable.")
        if not found & set(CLASS_TOKENS):
            problems.append(
                f"{row['path']}: category declares no tolerance class. One of "
                f"{', '.join(CLASS_TOKENS)} is required by HLD section 25.1, "
                f"and a row in neither class is a hole rather than a case.")
        for claim, gap in CONTRADICTORY_TOKENS:
            if {claim, gap} <= found:
                problems.append(
                    f"{row['path']}: carries both '{claim}' and '{gap}', which "
                    f"contradict. The gap this row recorded has been closed, "
                    f"so drop '{gap}' rather than leaving it to read as a live "
                    f"gap that nothing checks.")

    def satisfies(subset: list[dict[str, str]], wanted: tuple[str, ...]) -> bool:
        return any(tokens(row) & set(wanted) for row in subset)

    real = [row for row in rows if "real" in tokens(row)]
    for label, subset, why in (
            ("the corpus", rows, "an untested class has an untested tolerance"),
            ("the real layer", real,
             "a class only ever exercised against bytes this repository "
             "generated has never seen a vendor's padding or odd-length "
             "values")):
        if not satisfies(subset, (MONOCHROME_16_BIT,)):
            problems.append(f"{label} has no monochrome 16-bit case "
                            f"(HLD 25.1 class one), and {why}")
        if not satisfies(subset, COLOUR_OR_ULTRASOUND):
            problems.append(f"{label} has no colour or ultrasound case "
                            f"(HLD 25.1 class two), and {why}")

    if not real:
        problems.append(
            "every row is synthetic. At least one row must be not synthetic, "
            "because a corpus built only from generated cases has never seen "
            "a real vendor's padding, private blocks or odd-length values.")

    print(f"coverage over {len(rows)} manifest rows, "
          f"{len(real)} of them real")
    print(f"  transfer syntaxes: {len(REGISTRY_TRANSFER_SYNTAXES) - len(missing)}"
          f" of {len(REGISTRY_TRANSFER_SYNTAXES)}")
    print(f"  monochrome 16-bit rows: "
          f"{sum(1 for r in rows if MONOCHROME_16_BIT in tokens(r))}")
    class_two = [r for r in rows if tokens(r) & set(COLOUR_OR_ULTRASOUND)]
    real_class_two = [r for r in class_two if "real" in tokens(r)]
    real_chroma = [r for r in real_class_two if "colour" in tokens(r)]
    print(f"  colour or ultrasound rows: {len(class_two)} "
          f"({len(real_class_two)} real, of which {len(real_chroma)} carry "
          f"chroma)")

    # Not a failure, and deliberately not one. HLD 25.1's class two is
    # "Colour and ultrasound", and a greyscale ultrasound satisfies that as
    # written, so failing here would be the check disagreeing with the policy
    # it implements. But 25.1 gives the REASON for the class as "chroma
    # subsampling and YBR conversion legitimately differ", and a greyscale
    # ultrasound exercises neither. Printing `colour or ultrasound rows: 6`
    # and stopping would let a reader conclude the class is covered against
    # real data when no real case has any chroma in it at all.
    if real_class_two and not real_chroma:
        print("  NOTE: every real class-two case is greyscale, so no real "
              "case exercises chroma")
        print("        subsampling or YBR conversion, which HLD 25.1 gives as "
              "the reason for")
        print("        the class. The only chroma in the corpus is generated "
              "by this repository.")
        print("        See corpus/README.md, the real-layer table.")

    if problems:
        print("\nFAIL: corpus coverage")
        for problem in problems:
            print(f"  {problem}")
        print("\nSee corpus/README.md for what each condition is for. Do not")
        print("close the gap by relaxing this check.")
        return 1
    print("OK: coverage complete")
    return 0


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
    parser.add_argument("--coverage", action="store_true")
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
    if args.coverage:
        return coverage(rows)
    if args.manifest_only:
        print(f"OK: manifest shape valid, {len(rows)} rows")
        return 0
    return verify(rows, args.fetch)


if __name__ == "__main__":
    sys.exit(main())
