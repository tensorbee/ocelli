#!/usr/bin/env python3
"""The exact dependency pins and the wasm size budget. HLD 27.2 R4 and E1.2.

Two checks that share a file because they share a failure mode: both are
about a number nobody looks at until it is wrong.

**The wgpu pin.** HLD section 15.2 pins `wgpu = "=30.0.1"` and says why:

    "The exact wgpu pin is not fussiness. Agents reliably emit wgpu 0.19-era
     pipeline code; a caret range lets that compile against something subtly
     different from what the shader expects."

R4 adds: "treat GPU code that compiles first try with suspicion". A caret or
tilde range on wgpu re-opens exactly the gap the pin closes, so the range form
is refused, not just a wrong version.

**`=` is not the pin. `=` and a full version is.** The S03 sprint review's
third pass measured the residue: this check tested only that the spec starts
with `=`, and `wgpu = "=30"` passed. Cargo reads a partial version after `=`
as a whole band, which is demonstrable rather than a reading of the
documentation:

    wgpu = "=30.0.0"   cargo metadata  -> Downgrading wgpu v30.0.1 -> v30.0.0
    wgpu = "=30"       cargo metadata  -> resolves against 30.0.1, no change
    wgpu = "=30"       cargo update -p wgpu --precise 30.0.0  -> exit 0
    wgpu = "=30.0.1"   cargo update -p wgpu --precise 30.0.0  -> exit 101,
                       "failed to select a version for the requirement"

A spec that both resolves 30.0.1 and permits 30.0.0 is `=30.*`, and two
versions is a range. So an exact pin is `=` followed by a major, a minor and a
patch, and `=30` and `=30.0` are refused with the rest.

**The wasm-bindgen pin.** Added by F-002 and not in section 15.2's list, which
predates the build pipeline story. `wasm-pack` runs a wasm-bindgen CLI whose
version must match the crate version, so a range lets the two drift and the
mismatch reads as a build break rather than as a resolution change.

**How the entry is read, and why it is not a regex.** Cargo accepts two forms,
`wgpu = "=30.0.1"` and `wgpu = { version = "=30.0.1", features = [...] }`.
The first version of this script took the FIRST quoted string in the entry as
the version, which is positional rather than structural, and the S03 sprint
review measured what that costs:

    wgpu = { default-features = false, features = ["=noop"], version = "30" }

printed `OK: wgpu pinned exactly`. The version read was `=noop`, which starts
with `=`, and the real version was a caret range nobody looked at. A hard rule
in CLAUDE.md, "wgpu is pinned exactly", was defeated by the ORDER of the keys.
So the file is parsed with `tomllib` and the version is taken from the
`version` key. A table that declares no version at all is refused rather than
guessed at.

**The published package licences.** The workspace declares `MIT OR
Apache-2.0`, so the generated wasm package must carry both grants. The crate
uses relative symlinks to the repository originals, and this check compares
the generated package bytes with those originals. A copied or stale legal text
is refused rather than treated as equivalent.

**The size budget.** Story E1.2 is "wasm-pack build pipeline with a hard size
budget gate", and Appendix A gate A4 asks whether binary size and cold start
land within budget at all, estimating 3 to 8 MB uncompressed before tuning with
Naga dominating. Unmeasured today, by the HLD's own admission.

So the budget starts as a RECORDED MEASUREMENT rather than a guess. The first
run writes the observed size to `ci/wasm-size-budget.json` and passes. After
that a regression beyond the tolerance fails. A budget invented before the
first measurement would be either meaningless or immediately wrong.

Usage:
  python3 scripts/pin_and_size_check.py                 # pin only
  python3 scripts/pin_and_size_check.py --with-size     # pin + size
  python3 scripts/pin_and_size_check.py --accept-size   # re-baseline
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CARGO = ROOT / "Cargo.toml"
BUDGET = ROOT / "ci" / "wasm-size-budget.json"
PKG = ROOT / "crates" / "ocelli-wasm" / "pkg"
DECODE_BUDGET = ROOT / "ci" / "wasm-decode-size.json"
DECODE_PKG = ROOT / "crates" / "ocelli-wasm" / "pkg-decode"
# The decode bindings. Canvas exports belong to the render worker.
DECODE_EXPORTS = (
    "install_panic_hook",
    "ocelli_version",
    "panic_record_len",
    "panic_record_ptr",
)
DECODE_ABSENT = ("attachOffscreenCanvas",)
RITK_VENDOR = ROOT / "vendor" / "ritk-codecs-0.6.0"

RITK_ARCHIVE_SHA256 = "5fb65755a819c6ba38bf8aaf146f4ccc23d2fe217c8161a61bf00517c9d1cce5"
RITK_VCS = "33497ccd55b44e004c0b8314a1bcc2e0fc9cb3ed"
RITK_ADDITIONS = {
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "PACKAGE-INVENTORY.sha256",
    "PATCH-PROVENANCE.md",
}
RITK_INVENTORY_SHA256 = "066d80ffd6b74bd9410df63f1d694c209431be8e3efde37b7c427f71da0cb351"
# The reviewed patch record, rewritten when a patch changes. F-239 added the
# three-component colour decode to the published decoder, F-243 the
# JPEG-LS run-mode, end-of-scan and default-threshold patch, and F-269 a
# comment-only audit whose corrections landed in four jpeg_ls files.
RITK_PROVENANCE_SHA256 = "073cddf7486560e5e8ef049ffaf5caeeebf97f48d00d639ac8836bcfcf57e958"
# Every published source file this repository has patched, with the exact
# bytes of the reviewed patch. Anything else must stay byte-identical to the
# archive.
RITK_PATCHED_HASHES = {
    # F-239 and D-31. The colour decode: three-component LRCP packet walking,
    # the inverse component transform before the level shift, and the exports.
    "src/jpeg_2000/image.rs": "8c7a1c8b1147369c60b11fdd3132339cf798b2f4c40327ac00ffd34ca3f30e44",
    "src/jpeg_2000/mod.rs": "a3e14bca44ec66090df7a1d3a69da08e74f01f2b8cbcd769c9073c300bf7d52a",
    "src/jpeg_2000/packet/reader.rs": "2ed1bf22d0607915048fcb807f6ccaa599c05eb29057a2b19c95ef73e611efc1",
    "src/lib.rs": "accfac2f2cfe750a0b8df2f3409e00a46c98ed2eeeef338469a84ad997ed7def",
    # F-243 and D-20. ITU-T T.87 A.7.1 run mode, the context RESET, the end of
    # scan held to CharLS 2.4.4 and C.2.4.1.1.1 default thresholds, with the
    # encoder's code segment A.16 mirror and one published test edited by the
    # lead's ruling. F-269's comment audit also binds context/mod.rs,
    # encoder.rs and scan.rs, so those three digests carry F-269 comment
    # bytes too.
    "src/jpeg_ls/bitstream.rs": "49367ccb7467dd91b267a2dfe5a1f70f70d872bf28243fdbbf817e9a34a68cbb",
    "src/jpeg_ls/context/mod.rs": "24af96f8abb4e5aae0b94428426c949f41ad85438daae0ff1cfa7a3289986421",
    # F-269. The comment-only audit against ITU-T T.87. First patched under
    # F-269, which edited only this file's test annotations.
    "src/jpeg_ls/context/tests.rs": "b62259de352e6353fc4decc2e9a2e05700246a7d8b87dcfa46935bde66f57f95",
    "src/jpeg_ls/decoder.rs": "d898917107a8d2575a04cc47ba53748785f04c1b661235f283ede6744f8a4b11",
    "src/jpeg_ls/encoder.rs": "8ad43406e62dc7c01ced6744f838b68121ba8f74c99fb96e24c842155abaad75",
    "src/jpeg_ls/scan.rs": "29b9eac0579b3279d883304a2ac739d8034e7e68bd0a6c60289d6e627dbb659f",
    "src/jpeg_ls/tests/conformance.rs": "aee89d4a0a49b21d8f6ebc4229e4458cc4c037300dad36babcb17f08f4acc512",
}
# The manifest-only patch D-20 records, checked as TOML as well as by hash.
RITK_MANIFEST_PATCHED_HASHES = {
    "Cargo.toml": "a4832241153ecc35ddb1de5b246f0a244e2d4d301852944aa09952aba7b306d2",
    "Cargo.toml.orig": "527ba797ed392fed09da5f771c84c6462b59100741ac2177c3223f43c6c176ae",
}
RITK_PATCHED = set(RITK_PATCHED_HASHES) | set(RITK_MANIFEST_PATCHED_HASHES)
RITK_LICENCE_HASHES = {
    "LICENSE-MIT": "8ff8d4621b3a1aa9df7006e0eaeacd5dfb686dcaa328e86c692374fbf2c4874f",
    "LICENSE-APACHE": "b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1",
}

# F-244 and D-18. The published dicom-object 0.10.0 package with ONE source
# patch, the `first = false;` line in collector.rs that PS3.5 A.4's empty
# Basic Offset Table needs. The patched file is pinned by its whole digest, so
# a second change beside the first is an undeclared change.
DICOM_OBJECT_VENDOR = ROOT / "vendor" / "dicom-object-0.10.0"
DICOM_OBJECT_ARCHIVE_SHA256 = (
    "0504b87987f0e7624cf7ee063fe7ffdb438c66c092620817fb9bb0f130686b85"
)
DICOM_OBJECT_VCS = "39603cef65da125e47daffbb32310c6826f2f62b"
DICOM_OBJECT_TAG_COMMIT = "2941c54cae434dd63a54d0a5546b3b0df42e9f6c"
DICOM_OBJECT_PUBLISHED_FILES = 13
DICOM_OBJECT_ADDITIONS = {
    "LICENSE-MIT",
    "LICENSE-APACHE",
    "PACKAGE-INVENTORY.sha256",
    "PATCH-PROVENANCE.md",
}
DICOM_OBJECT_INVENTORY_SHA256 = (
    "9097f089c5ad85896529eec52cf5a8edcb45fc57b85ed6bf019894c25c7d9c1d"
)
DICOM_OBJECT_PROVENANCE_SHA256 = (
    "51532f1ee265afc5ae20ef0d4ad61eefca5696c63a1a33c551507c2cbdadf508"
)
DICOM_OBJECT_PATCHED_HASHES = {
    "src/collector.rs":
        "69f08d0ebf59e3d4fe377912d8e0729d79f980d266500d378d49558413620656",
}
DICOM_OBJECT_LICENCE_HASHES = {
    "LICENSE-MIT": "77fbf925c55b05c210868fa639de038b4f01d9a19b9309f7db46f32ce0d5661f",
    "LICENSE-APACHE": "ae8a8e2f408f02b92bbaa5a4b482209288ec1d9dac85b5722b001040f1f13a9d",
}

OPENJPH_VENDOR = ROOT / "vendor" / "openjph-core-0.1.0"
OPENJPH_ARCHIVE_SHA256 = "c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d"
OPENJPH_VCS = "7ed6d6d110d994ec740aacaa90a78b2e807c4c24"
OPENJPH_INVENTORY_SHA256 = (
    "f6aef8e7211a9ab5b562414d673f5df5db01f99a3cf926ee90d01d0e65a14772"
)
OPENJPH_PUBLISHED_FILES = 50
# Upstream OpenJPH LICENSE at tag 0.26.3. The redistribution check pins these
# bytes. crates.io still reports no repository URL, so the file is fetched
# from the OpenJPH tag the port names.
OPENJPH_LICENCE_SHA256 = "5ddf5177863dfc9ab65fa129d587db651241f00e21ed2427b218bea997591f98"
OPENJPH_NOTICE_FILES = ("LICENSE", "LICENCE", "COPYING", "NOTICE")
# What this repository may add beside the published archive. The notice
# filenames stay permitted so a later notice is not an unrecorded file, and
# F-239's D-31 patch adds its own provenance record.
OPENJPH_ADDITIONS = {
    "PACKAGE-INVENTORY.sha256",
    "PATCH-PROVENANCE.md",
    *OPENJPH_NOTICE_FILES,
}
# The reviewed patch record, hashed like its ritk counterpart.
OPENJPH_PROVENANCE_SHA256 = "8619d0c0d8710d7b5562c844dba2eab1a99f2ff27999d5e8f204b8b8d6433d8b"
# The published files D-31's patch changes, with the exact bytes of the
# reviewed patch. The decode path runs the codestream's own inverse colour
# transform before the DC level shift and its clamp, which the published
# package never does, and the inverse coefficients are pinned to Annex G
# equation G-6's literals. Every other published file must stay
# byte-identical.
OPENJPH_PATCHED_HASHES = {
    "src/transform/colour.rs": "8d13e0db444ac9b9049b70dc5a65fe42b69568a3aeb717156fe12f7234cda829",
    "src/codestream/tile.rs": "551f0f6f174c38e663bbff2f68573e602e7a7eff64b8195d64290e9853b59b0a",
    "src/codestream/tile_comp.rs": "0585e14229f26b83c16fb8e1c7ab1aefb1edef42bdea0f31299cd01cd98c469c",
}

PACKAGE_LICENCES = ("LICENSE-MIT", "LICENSE-APACHE")

# Crates whose version must be an EXACT `=` pin, with the reason a range is
# refused. The reason is printed on failure, because "pin it exactly" without
# the reason is the kind of rule that gets relaxed by the next person in a
# hurry.
EXACT_PINNED = {
    "wgpu":
        "HLD section 15.2 requires `=`. Agents reliably emit wgpu 0.19-era "
        "pipeline code and a range lets that compile against something "
        "subtly different from what the shader expects.",
    # F-002 (E1.2). Not in HLD section 15.2's list, which predates the build
    # pipeline story.
    "wasm-bindgen":
        "`wasm-pack` runs a wasm-bindgen CLI whose version must match the "
        "crate version. A range lets the two drift, and the resulting "
        "version-mismatch reads as a build break rather than as a resolution "
        "change.",
}

# Growth tolerated before the gate fails, as a fraction of the baseline.
# A binary that grows 5% in one story is a story that should say why.
TOLERANCE = 0.05

# An exact pin: `=`, optional whitespace Cargo allows after the operator, then
# all three of major, minor and patch, then the optional pre-release and build
# metadata semver permits. One version and no band. `=30` and `=30.0` are
# comparators over a whole minor or patch band and are refused here.
EXACT_PIN = re.compile(
    r"^=\s*\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$")


def declared_version(entry: object) -> str | None:
    """The version a `[workspace.dependencies]` entry declares.

    A bare string IS the version. A table declares it under the `version`
    key, and the rest of the table is features, a path or a
    `default-features` flag, any of which may hold a string that starts with
    `=`. `None` means the entry declares no version this check can read, and
    that is refused rather than assumed to be exact.
    """
    if isinstance(entry, str):
        return entry
    if isinstance(entry, dict):
        version = entry.get("version")
        return version if isinstance(version, str) else None
    return None


def check_pins() -> list[str]:
    try:
        cargo = tomllib.loads(CARGO.read_text())
    except tomllib.TOMLDecodeError as error:
        return [f"Cargo.toml does not parse as TOML: {error}"]
    declared = cargo.get("workspace", {}).get("dependencies")
    if not isinstance(declared, dict):
        return ["Cargo.toml has no [workspace.dependencies] section"]

    problems = []
    for crate, reason in sorted(EXACT_PINNED.items()):
        if crate not in declared:
            problems.append(
                f"{crate} is not declared in [workspace.dependencies], "
                f"and it must be, pinned exactly. {reason}")
            continue
        spec = declared_version(declared[crate])
        if spec is None:
            problems.append(
                f"{crate}: cannot read a version from "
                f"{declared[crate]!r}. An entry with no `version` key is "
                f"not an exact pin, whatever else the table holds. {reason}")
            continue
        if not spec.strip().startswith("="):
            problems.append(
                f"{crate} = \"{spec}\" is a RANGE, not an exact pin. {reason}")
            continue
        if "," in spec:
            problems.append(
                f"{crate} = \"{spec}\" declares MORE THAN ONE comparator, "
                f"which is a range whatever the first one says. An exact pin "
                f"is one `=` and one full major.minor.patch. {reason}")
            continue
        if not EXACT_PIN.match(spec.strip()):
            problems.append(
                f"{crate} = \"{spec}\" pins with `=` and a PARTIAL version, "
                f"which Cargo reads as a whole band. Measured on wgpu: `=30` "
                f"resolves against 30.0.1, and under it "
                f"`cargo update -p wgpu --precise 30.0.0` exits 0 where "
                f"`=30.0.1` refuses that version. Two versions is a range. "
                f"Write `=` and a full major.minor.patch. {reason}")
    return problems


def file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def dependency_tree_outputs() -> tuple[list[str], list[str]]:
    outputs = []
    problems = []
    for target in (None, "wasm32-unknown-unknown"):
        command = ["cargo", "tree", "-p", "ocelli-codec", "--edges",
                   "normal,build", "--prefix", "none"]
        label = "native"
        if target is not None:
            command.extend(["--target", target])
            label = target
        result = subprocess.run(command, cwd=ROOT, text=True,
                                capture_output=True, check=False)
        if result.returncode != 0:
            problems.append(
                f"cannot prove the {label} dependency graph: cargo tree "
                f"exited {result.returncode}: {result.stderr.strip()}")
        else:
            outputs.append(result.stdout)
    return outputs, problems


def check_ritk_vendor(
    vendor: Path = RITK_VENDOR,
    workspace: Path = CARGO,
    dependency_trees: tuple[str, ...] | None = None,
) -> list[str]:
    """Prove the exact published package, local patch, licences and graphs."""
    problems = []
    inventory_path = vendor / "PACKAGE-INVENTORY.sha256"
    if not inventory_path.is_file():
        return ["ritk-codecs vendor has no PACKAGE-INVENTORY.sha256"]
    if file_sha256(inventory_path) != RITK_INVENTORY_SHA256:
        problems.append(
            "ritk-codecs published-package inventory digest does not match "
            "the immutable crates.io archive inventory")
    inventory = {}
    for line in inventory_path.read_text().splitlines():
        digest, separator, name = line.partition("  ./")
        if not separator or not re.fullmatch(r"[0-9a-f]{64}", digest):
            problems.append(f"ritk-codecs inventory row is invalid: {line!r}")
            continue
        if name in inventory:
            problems.append(f"ritk-codecs inventory repeats {name}")
        inventory[name] = digest
    if len(inventory) != 74:
        problems.append(
            f"ritk-codecs inventory has {len(inventory)} published files, "
            f"expected the archive's exact 74")

    actual = {
        str(path.relative_to(vendor)) for path in vendor.rglob("*")
        if path.is_file()
    }
    expected = set(inventory) | RITK_ADDITIONS
    for name in sorted(set(inventory) - actual):
        problems.append(f"ritk-codecs published file is absent: {name}")
    for name in sorted(actual - expected):
        problems.append(f"ritk-codecs carries an unrecorded file: {name}")
    for name, digest in sorted(inventory.items()):
        path = vendor / name
        if path.is_file() and name not in RITK_PATCHED and file_sha256(path) != digest:
            problems.append(f"ritk-codecs published file changed: {name}")

    try:
        vcs = json.loads((vendor / ".cargo_vcs_info.json").read_text())["git"]["sha1"]
    except (FileNotFoundError, KeyError, json.JSONDecodeError):
        vcs = None
    if vcs != RITK_VCS:
        problems.append(f"ritk-codecs VCS revision is {vcs!r}, expected {RITK_VCS}")

    for name, digest in RITK_LICENCE_HASHES.items():
        path = vendor / name
        if not path.is_file() or file_sha256(path) != digest:
            problems.append(f"ritk-codecs {name} is absent or not the exact upstream text")
    provenance = (vendor / "PATCH-PROVENANCE.md")
    if not provenance.is_file() or file_sha256(provenance) != RITK_PROVENANCE_SHA256:
        problems.append(
            "ritk-codecs PATCH-PROVENANCE.md has undeclared changes from "
            "the exact reviewed patch record")
    provenance_text = provenance.read_text() if provenance.is_file() else ""
    for fact in (RITK_ARCHIVE_SHA256, RITK_VCS, "MIT alternative"):
        if fact not in provenance_text:
            problems.append(f"ritk-codecs provenance does not record {fact}")

    for name, digest in sorted(RITK_PATCHED_HASHES.items()):
        path = vendor / name
        if path.is_file() and file_sha256(path) != digest:
            problems.append(
                f"ritk-codecs {name} has undeclared changes beyond its "
                "recorded patch")
    for manifest_name in sorted(RITK_MANIFEST_PATCHED_HASHES):
        path = vendor / manifest_name
        if path.is_file() and file_sha256(path) != RITK_MANIFEST_PATCHED_HASHES[manifest_name]:
            problems.append(
                f"ritk-codecs {manifest_name} has undeclared changes beyond "
                "the exact jpeg-decoder default-features patch")
        try:
            manifest = tomllib.loads(path.read_text())
            jpeg = manifest["dependencies"]["jpeg-decoder"]
        except (FileNotFoundError, KeyError, tomllib.TOMLDecodeError):
            jpeg = None
        if not isinstance(jpeg, dict) or jpeg.get("default-features") is not False:
            problems.append(
                f"ritk-codecs {manifest_name} does not set jpeg-decoder "
                f"default-features = false")

    try:
        root = tomllib.loads(workspace.read_text())
        exclude = root["workspace"]["exclude"]
        dependency = root["workspace"]["dependencies"]["ritk-codecs"]
    except (FileNotFoundError, KeyError, tomllib.TOMLDecodeError):
        exclude, dependency = [], None
    if "vendor/ritk-codecs-0.6.0" not in exclude:
        problems.append("workspace does not exclude vendor/ritk-codecs-0.6.0")
    if not isinstance(dependency, dict) or dependency.get("version") != "=0.6.0" \
            or dependency.get("path") != "vendor/ritk-codecs-0.6.0":
        problems.append("workspace ritk-codecs dependency is not the exact path and version")

    trees = dependency_trees
    if trees is None:
        generated, tree_problems = dependency_tree_outputs()
        trees = tuple(generated)
        problems.extend(tree_problems)
    for index, tree in enumerate(trees):
        rayon = sorted({line.split()[0] for line in tree.splitlines()
                        if line.split() and line.split()[0] in {"rayon", "rayon-core"}})
        if rayon:
            problems.append(
                f"Rayon is present in dependency graph {index + 1}: "
                f"{', '.join(rayon)}")
    return problems


def check_dicom_object_vendor(
    vendor: Path = DICOM_OBJECT_VENDOR,
    workspace: Path = CARGO,
) -> list[str]:
    """Prove the exact published package, its one patch and its licences.

    F-244 and deviation D-18. Every published file but the patched one must
    match the archive inventory byte for byte, the patched one must match its
    recorded digest, and the inventory and provenance digests are rooted here,
    outside the vendor tree, so a coordinated edit of a file and its record
    still fails.
    """
    label = "dicom-object"
    problems = []
    inventory_path = vendor / "PACKAGE-INVENTORY.sha256"
    if not inventory_path.is_file():
        return [f"{label} vendor has no PACKAGE-INVENTORY.sha256"]
    if file_sha256(inventory_path) != DICOM_OBJECT_INVENTORY_SHA256:
        problems.append(
            f"{label} published-package inventory digest does not match "
            "the immutable crates.io archive inventory")
    inventory = {}
    for line in inventory_path.read_text().splitlines():
        digest, separator, name = line.partition("  ./")
        if not separator or not re.fullmatch(r"[0-9a-f]{64}", digest):
            problems.append(f"{label} inventory row is invalid: {line!r}")
            continue
        if name in inventory:
            problems.append(f"{label} inventory repeats {name}")
        inventory[name] = digest
    if len(inventory) != DICOM_OBJECT_PUBLISHED_FILES:
        problems.append(
            f"{label} inventory has {len(inventory)} published files, "
            f"expected the archive's exact {DICOM_OBJECT_PUBLISHED_FILES}")

    actual = {
        str(path.relative_to(vendor)) for path in vendor.rglob("*")
        if path.is_file()
    }
    expected = set(inventory) | DICOM_OBJECT_ADDITIONS
    for name in sorted(set(inventory) - actual):
        problems.append(f"{label} published file is absent: {name}")
    for name in sorted(actual - expected):
        problems.append(f"{label} carries an unrecorded file: {name}")
    for name, digest in sorted(inventory.items()):
        path = vendor / name
        if path.is_file() and name not in DICOM_OBJECT_PATCHED_HASHES \
                and file_sha256(path) != digest:
            problems.append(f"{label} published file changed: {name}")
    for name, digest in sorted(DICOM_OBJECT_PATCHED_HASHES.items()):
        path = vendor / name
        if path.is_file() and file_sha256(path) != digest:
            problems.append(
                f"{label} {name} has undeclared changes beyond the exact "
                "empty Basic Offset Table patch")

    try:
        vcs = json.loads((vendor / ".cargo_vcs_info.json").read_text())["git"]["sha1"]
    except (FileNotFoundError, KeyError, json.JSONDecodeError):
        vcs = None
    if vcs != DICOM_OBJECT_VCS:
        problems.append(
            f"{label} VCS revision is {vcs!r}, expected {DICOM_OBJECT_VCS}")

    for name, digest in DICOM_OBJECT_LICENCE_HASHES.items():
        path = vendor / name
        if not path.is_file() or file_sha256(path) != digest:
            problems.append(
                f"{label} {name} is absent or not the exact upstream text")
    provenance = vendor / "PATCH-PROVENANCE.md"
    if not provenance.is_file() \
            or file_sha256(provenance) != DICOM_OBJECT_PROVENANCE_SHA256:
        problems.append(
            f"{label} PATCH-PROVENANCE.md has undeclared changes from "
            "the exact reviewed patch record")
    provenance_text = provenance.read_text() if provenance.is_file() else ""
    for fact in (DICOM_OBJECT_ARCHIVE_SHA256, DICOM_OBJECT_VCS,
                 DICOM_OBJECT_TAG_COMMIT, "PS3.5 A.4", "MIT alternative",
                 *DICOM_OBJECT_PATCHED_HASHES.values()):
        if fact not in provenance_text:
            problems.append(f"{label} provenance does not record {fact}")

    try:
        root = tomllib.loads(workspace.read_text())
        exclude = root["workspace"]["exclude"]
        dependency = root["workspace"]["dependencies"]["dicom-object"]
    except (FileNotFoundError, KeyError, tomllib.TOMLDecodeError):
        exclude, dependency = [], None
    if "vendor/dicom-object-0.10.0" not in exclude:
        problems.append("workspace does not exclude vendor/dicom-object-0.10.0")
    if not isinstance(dependency, dict) or dependency.get("version") != "=0.10.0" \
            or dependency.get("path") != "vendor/dicom-object-0.10.0":
        problems.append(
            "workspace dicom-object dependency is not the exact path and version")
    elif dependency.get("default-features") is not False:
        # D-18 keeps every dicom-rs component's defaults off.
        problems.append("workspace dicom-object dependency enables default features")
    return problems


def wasm_bytes(pkg: Path = PKG) -> int | None:
    if not pkg.is_dir():
        return None
    modules = sorted(pkg.glob("*.wasm"))
    if not modules:
        return None
    return max(m.stat().st_size for m in modules)


def check_decode_exports(pkg: Path) -> list[str]:
    """The decode bindings export the panic surface and not the canvas."""
    dts = pkg / "ocelli_wasm.d.ts"
    if not dts.is_file():
        return [f"{dts} is absent. The decode pack did not emit bindings."]
    text = dts.read_text()
    problems = []
    for name in DECODE_EXPORTS:
        if f"export function {name}" not in text:
            problems.append(f"decode bindings do not export {name}")
    for name in DECODE_ABSENT:
        if name in text:
            problems.append(f"decode bindings still name {name}")
    return problems


def check_decode_size() -> list[str]:
    """The F-110 decode module, measured, not guessed.

    The render budget stays the shipping module. This file is the other
    artefact. A missing record is a failure, not an invitation to write one.
    """
    size = wasm_bytes(DECODE_PKG)
    if size is None:
        return [f"no .wasm found under {DECODE_PKG.relative_to(ROOT)}. "
                f"Run `bin/ocelli.sh wasm-decode` first."]
    if not DECODE_BUDGET.exists():
        return ["ci/wasm-decode-size.json is absent. Record the measured "
                "decode module. Do not invent the byte count."]
    recorded = json.loads(DECODE_BUDGET.read_text())
    baseline = recorded["bytes"]
    tolerance = recorded["tolerance"]
    if tolerance != TOLERANCE:
        return [f"decode size tolerance is {tolerance}, not {TOLERANCE}."]
    ceiling = int(baseline * (1 + tolerance))
    print(f"  decode wasm size {size:,} bytes, baseline {baseline:,}, "
          f"ceiling {ceiling:,}")
    problems = []
    if size > ceiling:
        problems.append(
            f"decode wasm module is {size:,} bytes, over the "
            f"{ceiling:,} byte ceiling ({tolerance:.0%} above the "
            f"{baseline:,} byte baseline).")
    problems.extend(check_decode_exports(DECODE_PKG))
    return problems


def check_package_licences(pkg: Path = PKG,
                           source_root: Path = ROOT) -> list[str]:
    """Require both generated grants to match the repository originals."""
    problems = []
    for name in PACKAGE_LICENCES:
        source = source_root / name
        packaged = pkg / name
        if not source.is_file():
            problems.append(
                f"repository licence {name} is absent, so the generated "
                f"package cannot be checked against its legal source")
            continue
        if packaged.is_symlink():
            problems.append(
                f"generated package licence {name} is a symlink, not a "
                f"regular file under {pkg}. A published package must contain "
                f"both grants rather than links to bytes outside it.")
            continue
        if not packaged.is_file():
            problems.append(
                f"generated package licence {name} is absent under "
                f"{pkg}. The package declares MIT OR Apache-2.0 and must "
                f"ship both grants.")
            continue
        if packaged.read_bytes() != source.read_bytes():
            problems.append(
                f"generated package licence {name} is not byte-identical to "
                f"the repository original. Rebuild from the package-local "
                f"relative symlink rather than copying legal text.")
    return problems


def check_size(accept: bool) -> list[str]:
    size = wasm_bytes()
    if size is None:
        return [f"no .wasm found under {PKG.relative_to(ROOT)}. "
                f"Run `bin/ocelli.sh wasm` first."]

    if not BUDGET.exists() or accept:
        BUDGET.parent.mkdir(parents=True, exist_ok=True)
        BUDGET.write_text(json.dumps({
            "bytes": size,
            "tolerance": TOLERANCE,
            "note": "Recorded measurement, not a guess. HLD Appendix A gate "
                    "A4 estimates 3-8 MB uncompressed before tuning, with "
                    "Naga dominating, and says it is unmeasured. Re-baseline "
                    "deliberately with --accept-size and say why in the "
                    "design plan.",
        }, indent=2) + "\n")
        verb = "re-baselined" if accept else "baselined"
        print(f"  wasm size {verb} at {size:,} bytes "
              f"({size / 1_048_576:.2f} MiB)")
        if accept:
            return []
        return check_decode_size()

    baseline = json.loads(BUDGET.read_text())["bytes"]
    ceiling = int(baseline * (1 + TOLERANCE))
    print(f"  wasm size {size:,} bytes, baseline {baseline:,}, "
          f"ceiling {ceiling:,}")
    problems = []
    if size > ceiling:
        problems.append(
            f"wasm module is {size:,} bytes, over the "
            f"{ceiling:,} byte ceiling ({TOLERANCE:.0%} above the "
            f"{baseline:,} byte baseline). Either reduce it or "
            f"re-baseline with --accept-size and record why.")
    if not accept:
        problems.extend(check_decode_size())
    return problems



def check_openjph_vendor(
    vendor: Path = OPENJPH_VENDOR,
    workspace: Path = CARGO,
) -> list[str]:
    """Prove the exact published package and its workspace wiring.

    Deviation D-22 with the D-31 patch recorded beside it: every published
    file must match the archive byte for byte except the two patched source
    files, which are held to their own reviewed digests. The additions this
    repository writes are the inventory, the patch provenance and the
    licence notice.
    """
    problems = []
    inventory_path = vendor / "PACKAGE-INVENTORY.sha256"
    if not inventory_path.is_file():
        return ["openjph-core vendor has no PACKAGE-INVENTORY.sha256"]
    if file_sha256(inventory_path) != OPENJPH_INVENTORY_SHA256:
        problems.append(
            "openjph-core published-package inventory digest does not match "
            "the immutable crates.io archive inventory")
    inventory = {}
    for line in inventory_path.read_text().splitlines():
        digest, separator, name = line.partition("  ")
        if not separator or not re.fullmatch(r"[0-9a-f]{64}", digest):
            problems.append(f"openjph-core inventory row is invalid: {line!r}")
            continue
        if name in inventory:
            problems.append(f"openjph-core inventory repeats {name}")
        inventory[name] = digest
    if len(inventory) != OPENJPH_PUBLISHED_FILES:
        problems.append(
            f"openjph-core inventory has {len(inventory)} published files, "
            f"expected the archive's exact {OPENJPH_PUBLISHED_FILES}")

    actual = {
        str(path.relative_to(vendor)) for path in vendor.rglob("*")
        if path.is_file()
    }
    expected = set(inventory) | OPENJPH_ADDITIONS
    for name in sorted(set(inventory) - actual):
        problems.append(f"openjph-core published file is absent: {name}")
    for name in sorted(actual - expected):
        problems.append(f"openjph-core carries an unrecorded file: {name}")
    for name, digest in sorted(inventory.items()):
        path = vendor / name
        if path.is_file() and name not in OPENJPH_PATCHED_HASHES \
                and file_sha256(path) != digest:
            problems.append(
                f"openjph-core published file changed: {name}. The recorded "
                f"patch covers only {sorted(OPENJPH_PATCHED_HASHES)}")

    try:
        vcs = json.loads((vendor / ".cargo_vcs_info.json").read_text())["git"]["sha1"]
    except (FileNotFoundError, KeyError, json.JSONDecodeError):
        vcs = None
    if vcs != OPENJPH_VCS:
        problems.append(f"openjph-core VCS revision is {vcs!r}, expected {OPENJPH_VCS}")

    provenance = vendor / "PATCH-PROVENANCE.md"
    if not provenance.is_file() or file_sha256(provenance) != OPENJPH_PROVENANCE_SHA256:
        problems.append(
            "openjph-core PATCH-PROVENANCE.md is absent or does not match "
            "the exact reviewed patch record")
    for name, digest in sorted(OPENJPH_PATCHED_HASHES.items()):
        path = vendor / name
        if path.is_file() and file_sha256(path) != digest:
            problems.append(
                f"openjph-core {name} has undeclared changes beyond the "
                "exact D-31 colour-transform patch")

    try:
        root = tomllib.loads(workspace.read_text())
        exclude = root["workspace"]["exclude"]
        dependency = root["workspace"]["dependencies"]["openjph-core"]
    except (FileNotFoundError, KeyError, tomllib.TOMLDecodeError):
        exclude, dependency = [], None
    if "vendor/openjph-core-0.1.0" not in exclude:
        problems.append("workspace does not exclude vendor/openjph-core-0.1.0")
    if not isinstance(dependency, dict) or dependency.get("version") != "=0.1.0" \
            or dependency.get("path") != "vendor/openjph-core-0.1.0":
        problems.append(
            "workspace openjph-core dependency is not the exact path and version")
    return problems


def check_openjph_redistribution(vendor: Path = OPENJPH_VENDOR) -> list[str]:
    """Refuse to publish while the BSD notice is absent or its bytes differ.

    `openjph-core` 0.1.0's published archive carries no notice. The file at
    `vendor/openjph-core-0.1.0/LICENSE` is the upstream OpenJPH text at tag
    0.26.3, and `OPENJPH_LICENCE_SHA256` pins those bytes. A missing LICENSE
    keeps the original refusal, including when some other notice name is
    present. A present LICENSE whose bytes differ is refused on the hash.

    It is NOT in the `pins` gate and therefore not in `--floor`, `--sprint` or
    `--all`. `/release` invokes this flag. The probes `pins.openjph-notice`
    and `pins.openjph-notice-hash` prove both refusals.
    """
    licence = vendor / "LICENSE"
    if not licence.is_file():
        return [
            "openjph-core 0.1.0 carries no BSD notice or copyright material, so it "
            "cannot be redistributed. Its crates.io archive has no "
            f"{', '.join(OPENJPH_NOTICE_FILES)} and the registry reports no "
            "repository URL to obtain one from. BSD-2-Clause permits "
            "redistribution only with its notice conditions retained. Obtain the "
            f"complete notice, place it at {vendor.name}/LICENSE, record its hash "
            "beside the other vendored licence hashes, and see D-22 in "
            "docs/hld/DEVIATIONS.md."
        ]
    digest = file_sha256(licence)
    if digest != OPENJPH_LICENCE_SHA256:
        return [
            "openjph-core 0.1.0 LICENSE sha256 is "
            f"{digest}, expected {OPENJPH_LICENCE_SHA256}"
        ]
    return []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--with-size", action="store_true")
    parser.add_argument("--accept-size", action="store_true")
    parser.add_argument(
        "--require-redistribution", action="store_true",
        help="refuse a vendored package whose licence notice is absent. "
             "/release passes this. No development profile does.")
    args = parser.parse_args(argv)

    problems = check_pins()
    problems.extend(check_ritk_vendor())
    problems.extend(check_dicom_object_vendor())
    problems.extend(check_openjph_vendor())
    if args.require_redistribution:
        problems.extend(check_openjph_redistribution())
    if args.with_size or args.accept_size:
        problems += check_size(args.accept_size)
        problems += check_package_licences()

    if problems:
        print("FAIL: pin or size gate")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {', '.join(sorted(EXACT_PINNED))} pinned exactly" +
          (", wasm size within budget, package licences match"
           if args.with_size else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main())
