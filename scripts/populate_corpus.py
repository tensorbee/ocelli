#!/usr/bin/env python3
"""Populate the ignored corpus/data directory and verify every manifest row.

Synthetic cases are regenerated from the committed generator. Public real
series are acquired from TCIA. An optional trusted seed directory can avoid
the downloads, and files are copied from it only after their SHA-256 digest
matches the manifest.
"""

from __future__ import annotations

import argparse
import hashlib
import shutil
import subprocess
import sys
import tempfile
import urllib.error
import urllib.parse
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "corpus" / "data"
MANIFEST = ROOT / "corpus" / "manifest.tsv"
VENV_PYTHON = ROOT / ".venv" / "bin" / "python"
TCIA_API = "https://services.cancerimagingarchive.net/nbia-api/services/v1"
REAL_SERIES = {
    "ct_cmb_mml": "1.3.6.1.4.1.14519.5.2.1.108975852603347259500108190173730050021",
    "mr_eay131": "1.3.6.1.4.1.14519.5.2.1.1620.1226.229417808443818737599259533657",
    "dx_varepop": "1.3.6.1.4.1.14519.5.2.1.111496736574540772816177955707250560822",
    "us_cmb_crc": "1.3.6.1.4.1.14519.5.2.1.1.56314755871495081827678310314743171188",
}


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def manifest_rows() -> list[tuple[str, str]]:
    lines = MANIFEST.read_text(encoding="utf-8").splitlines()
    if not lines:
        raise RuntimeError("corpus/manifest.tsv is empty")
    header = lines[0].split("\t")
    path_index = header.index("path")
    digest_index = header.index("sha256")
    return [
        (fields[path_index], fields[digest_index])
        for line in lines[1:]
        if line.strip()
        for fields in [line.split("\t")]
    ]


def copy_seed(seed: Path) -> int:
    if not seed.is_dir():
        print(f"FAIL: seed directory is absent: {seed}")
        return 1
    if seed.resolve() == DATA.resolve():
        print("FAIL: --seed must not be corpus/data itself")
        return 1

    copied = present = absent = 0
    for relative, expected in manifest_rows():
        source = seed / relative
        if not source.is_file():
            absent += 1
            continue
        if digest(source) != expected:
            print(f"FAIL: seed digest does not match for {relative}")
            return 1

        target = DATA / relative
        if target.is_file():
            if digest(target) != expected:
                print(f"FAIL: refusing to overwrite mismatched {relative}")
                return 1
            present += 1
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
        copied += 1

    print(f"seed: {copied} copied, {present} already present, "
          f"{absent} unavailable")
    return 0


def download_real_series(rows: list[tuple[str, str]], offline: bool) -> int:
    for name, uid in REAL_SERIES.items():
        prefix = f"real/{name}/"
        expected = [(relative, expected_digest)
                    for relative, expected_digest in rows
                    if relative.startswith(prefix)]
        if not expected:
            print(f"FAIL: manifest has no rows for real/{name}")
            return 1
        if all((DATA / relative).is_file()
               and digest(DATA / relative) == expected_digest
               for relative, expected_digest in expected):
            print(f"real/{name}: already present")
            continue
        if offline:
            print(f"FAIL: real/{name} is incomplete and --offline forbids "
                  "acquisition")
            return 1

        query = urllib.parse.urlencode({"SeriesInstanceUID": uid})
        url = f"{TCIA_API}/getImage?{query}"
        print(f"real/{name}: downloading public TCIA series")
        try:
            with tempfile.TemporaryDirectory(prefix="ocelli-corpus-") as tmp:
                archive = Path(tmp) / f"{name}.zip"
                with urllib.request.urlopen(url, timeout=120) as response:
                    with archive.open("wb") as output:
                        shutil.copyfileobj(response, output)
                destination = DATA / "real" / name
                destination.mkdir(parents=True, exist_ok=True)
                with zipfile.ZipFile(archive) as bundle:
                    for member in bundle.infolist():
                        target = (destination / member.filename).resolve()
                        if not target.is_relative_to(destination.resolve()):
                            print(f"FAIL: archive path escapes real/{name}")
                            return 1
                    bundle.extractall(destination)
        except (OSError, urllib.error.URLError, zipfile.BadZipFile) as error:
            print(f"FAIL: real/{name} acquisition failed, "
                  f"{type(error).__name__}")
            return 1
    return 0


def run(*arguments: str) -> int:
    return subprocess.run(arguments, cwd=ROOT).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--seed",
        type=Path,
        help="trusted corpus root used for manifest rows with no direct URL",
    )
    parser.add_argument(
        "--offline",
        action="store_true",
        help="do not download missing public real series",
    )
    args = parser.parse_args()

    if not VENV_PYTHON.is_file():
        print("FAIL: .venv is absent. Create it with:")
        print("  uv sync --locked")
        return 1

    DATA.mkdir(parents=True, exist_ok=True)
    if args.seed is not None and copy_seed(args.seed.expanduser()) != 0:
        return 1
    rows = manifest_rows()
    if download_real_series(rows, args.offline) != 0:
        return 1

    commands = (
        (str(VENV_PYTHON), "scripts/corpus_synth.py", "--tool-versions"),
        (str(VENV_PYTHON), "scripts/corpus_synth.py"),
        (str(VENV_PYTHON), "scripts/corpus_check.py", "--fetch"),
        ("bin/ocelli.sh", "gate", "corpus"),
    )
    for command in commands:
        if run(*command) != 0:
            return 1

    print("OK: corpus/data is populated and verified")
    return 0


if __name__ == "__main__":
    sys.exit(main())
