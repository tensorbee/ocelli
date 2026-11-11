#!/usr/bin/env python3
"""The release-time comparison between a cold build of the tag and the tarball.

docs/RELEASE.md item 6. `npm pack` runs `@ocelli/wasm`'s prepack script,
which rebuilds both modules and hashes what it staged, so a comparison made
on the pack's own build compares the build with itself and cannot differ.
This script builds the tagged source a second time, in a temporary checkout
with an empty cargo target directory, and refuses unless the packed build id
names the revision's SHA and its tree, records a clean tree, and carries
digests equal to the cold build's and to the tarball's own bytes. It then
copies the verified `.tgz` to the output directory and prints it, and
`npm publish <file>` runs no prepack, so the published bytes are the compared
bytes.

The wasm build is byte-reproducible for a given SHA on one toolchain, which
is what makes the comparison meaningful. Measured on 2026-11-07: two cold
builds of one tree and a warm rebuild agree on every byte of both modules. A
release on a toolchain where that stops holding refuses here rather than
shipping an uncompared tarball.

Usage:
  python3 scripts/release_wasm.py vX.Y.Z                 # the tag, once it exists
  python3 scripts/release_wasm.py HEAD --out /tmp/ocelli-release
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MODULE_PACKAGE = ROOT / "packages" / "wasm"

# The two shipped modules and the build id, at their tarball paths. These are
# the same paths `scripts/package_check.py` refuses a tarball without.
BUILD_ID_PATH = "dist/build-id.json"
MODULES = ("dist/ocelli_wasm_bg.wasm", "dist/decode/ocelli_wasm_bg.wasm")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def compare_problems(build_id: dict, tag_sha: str, tag_tree: str,
                     cold: dict[str, str]) -> list[str]:
    """The packed build id against the release revision and a cold build of it.

    `build_id` is the parsed dist/build-id.json the pack staged, `tag_sha` is
    the commit the release publishes, `tag_tree` is the tree of that
    commit, and `cold` maps each module's tarball path to the digest a second,
    cold build of that commit produced. A mismatch on any of the four recorded
    facts means the tarball is not the bytes the tagged source builds. The
    tree is what refuses a staged index: an index that differs from the tag's
    commit is not a clean checkout of it, even where the SHA matches.
    """
    problems: list[str] = []
    if build_id.get("dirty") is not False:
        problems.append(
            "the packed build id does not record a clean tree: dirty is "
            f"{build_id.get('dirty')!r}. The modules may have been built "
            "from uncommitted changes that the recorded SHA does not name.")
    recorded_sha = build_id.get("git_sha")
    if recorded_sha != tag_sha:
        problems.append(
            f"the packed build id records git SHA {recorded_sha!r} and the "
            f"release revision is {tag_sha!r}. The tarball is not built "
            f"from the tagged source.")
    recorded_tree = build_id.get("tree")
    if recorded_tree != tag_tree:
        problems.append(
            f"the packed build id records tree {recorded_tree!r} and the "
            f"release revision's tree is {tag_tree!r}. The index "
            f"the tarball was staged from is not the tagged source.")
    recorded = build_id.get("modules")
    if not isinstance(recorded, dict):
        problems.append("the packed build id records no modules object "
                        "with digests")
        recorded = {}
    for name in MODULES:
        expected = cold.get(name)
        found = recorded.get(name)
        if expected is None:
            problems.append(f"the cold build produced no digest for {name}")
        elif found != expected:
            problems.append(
                f"the packed build id records digest {found!r} for {name} "
                f"and a cold build of the tagged source hashes to "
                f"{expected!r}. The staged modules are not the ones the tag "
                f"builds.")
    return problems


def tarball_digests(tarball: Path) -> dict[str, str]:
    """The SHA-256 of each shipped module inside the packed tarball."""
    digests: dict[str, str] = {}
    with tarfile.open(tarball) as archive:
        for member in archive.getmembers():
            name = member.name.removeprefix("package/")
            if name in MODULES:
                extracted = archive.extractfile(member)
                if extracted is None:
                    raise RuntimeError(f"{tarball} carries {name} as "
                                       f"something other than a file")
                digests[name] = hashlib.sha256(extracted.read()).hexdigest()
    return digests


def tarball_build_id(tarball: Path) -> dict:
    """The parsed build id inside the packed tarball."""
    with tarfile.open(tarball) as archive:
        for member in archive.getmembers():
            if member.name.removeprefix("package/") == BUILD_ID_PATH:
                extracted = archive.extractfile(member)
                if extracted is None:
                    break
                return json.loads(extracted.read())
    raise RuntimeError(f"{tarball} carries no {BUILD_ID_PATH}")


def verify(tarball: Path, tag_sha: str, tag_tree: str,
           cold: dict[str, str]) -> list[str]:
    """The packed tarball against the revision and a cold build of it.

    Both halves are needed. The build id against the cold build is the
    comparison docs/RELEASE.md item 6 states, and the tarball's own module
    bytes against the cold build is what makes the published bytes the
    compared bytes when `npm publish <file>` runs no prepack.
    """
    problems = compare_problems(tarball_build_id(tarball), tag_sha, tag_tree,
                                cold)
    packed = tarball_digests(tarball)
    for name in MODULES:
        if packed.get(name) != cold.get(name):
            problems.append(
                f"{name} in the packed tarball hashes to {packed.get(name)!r} "
                f"and a cold build of the tagged source hashes to "
                f"{cold.get(name)!r}. Publishing the tarball would not be "
                f"publishing the compared bytes.")
    return problems


def cold_digests(tag_sha: str, work: Path) -> dict[str, str]:
    """Build the tagged source cold and hash both modules.

    `git archive` carries the tracked files only, which is the tagged source
    and nothing else, and the build's cargo target directory lives under
    `work`, so the build shares nothing with this repository's `target`. The
    build goes through bin/ocelli.sh, which is the one definition of the
    wasm-pack build that CI and a developer both get.
    """
    source = work / "checkout"
    source.mkdir()
    archive = subprocess.run(["git", "archive", tag_sha], cwd=ROOT,
                             capture_output=True)
    if archive.returncode != 0:
        raise RuntimeError(
            f"git archive {tag_sha} failed: "
            f"{archive.stderr.decode(errors='replace').strip()}")
    extract = subprocess.run(["tar", "-x"], cwd=source,
                             input=archive.stdout, capture_output=True)
    if extract.returncode != 0:
        raise RuntimeError(
            "untarring the tagged source failed: "
            f"{extract.stderr.decode(errors='replace').strip()}")
    for command in (["bin/ocelli.sh", "wasm"], ["bin/ocelli.sh", "wasm-decode"]):
        built = subprocess.run(command, cwd=source, env=dict(
            os.environ, CARGO_TARGET_DIR=str(work / "target")))
        if built.returncode != 0:
            raise RuntimeError(f"{' '.join(command)} failed in the "
                               f"temporary checkout")
    digests: dict[str, str] = {}
    for name, relative in (
            ("dist/ocelli_wasm_bg.wasm",
             "crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm"),
            ("dist/decode/ocelli_wasm_bg.wasm",
             "crates/ocelli-wasm/pkg-decode/ocelli_wasm_bg.wasm")):
        built_module = source / relative
        if not built_module.is_file():
            raise RuntimeError(f"the cold build produced no {relative}")
        digests[name] = sha256(built_module)
    return digests


def pack(work: Path) -> Path:
    """Pack `@ocelli/wasm` into `work`, prepack included.

    This is the real pack: the prepack script rebuilds both modules, stages
    them and writes dist/build-id.json, so the tarball and its identity are
    the ones a publish would produce.
    """
    destination = work / "pack"
    destination.mkdir()
    result = subprocess.run(
        ["npm", "pack", "--pack-destination", str(destination)],
        cwd=MODULE_PACKAGE, capture_output=True, text=True)
    if result.returncode != 0:
        raise RuntimeError(f"npm pack failed:\n{result.stderr.strip()}")
    tarballs = list(destination.glob("*.tgz"))
    if len(tarballs) != 1:
        raise RuntimeError(f"expected one tarball in {destination}, "
                           f"found {len(tarballs)}")
    return tarballs[0]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Compare a cold build of the release revision with the "
                    "packed @ocelli/wasm tarball, and print the verified "
                    ".tgz to publish.")
    parser.add_argument("revision",
                        help="the release tag, or the commit it will be "
                             "created at")
    parser.add_argument("--out", type=Path,
                        default=ROOT / "target" / "release-wasm",
                        help="where the verified tarball is copied")
    args = parser.parse_args()

    resolved = subprocess.run(
        ["git", "rev-parse", args.revision + "^{commit}"], cwd=ROOT,
        capture_output=True, text=True)
    if resolved.returncode != 0:
        print("FAIL: the wasm release comparison")
        print(f"  cannot resolve {args.revision!r} to a commit: "
              f"{resolved.stderr.strip()}")
        return 1
    tag_sha = resolved.stdout.strip()
    tag_tree = subprocess.run(
        ["git", "rev-parse", tag_sha + "^{tree}"], cwd=ROOT,
        capture_output=True, text=True)
    if tag_tree.returncode != 0:
        print("FAIL: the wasm release comparison")
        print(f"  cannot read the tree of {args.revision!r}: "
              f"{tag_tree.stderr.strip()}")
        return 1

    try:
        with tempfile.TemporaryDirectory(
                prefix="ocelli-release-wasm-") as temporary:
            work = Path(temporary)
            cold = cold_digests(tag_sha, work)
            tarball = pack(work)
            problems = verify(tarball, tag_sha, tag_tree.stdout.strip(), cold)
            if not problems:
                # Inside the block, because the pack directory dies with it
                # and the verified tarball is the one thing that must survive.
                args.out.mkdir(parents=True, exist_ok=True)
                verified = args.out / tarball.name
                shutil.copy2(tarball, verified)
    except RuntimeError as error:
        print("FAIL: the wasm release comparison")
        print(f"  {error}")
        return 1

    if problems:
        print("FAIL: the wasm release comparison")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: the packed @ocelli/wasm build id names {tag_sha} and the "
          f"tree of that commit, records no unstaged tracked changes, and "
          f"digests equal to a cold build of that source, and the packed "
          f"module bytes hash to the same digests")
    print(f"publish this exact file: npm publish {verified}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
