#!/usr/bin/env python3
"""The npm packaging and publish pipeline. Story E1.3.

`tsc --build` proves the TypeScript compiles. It proves nothing about what a
consumer receives, and the gap between those two is where packaging defects
live. Every check here is about the second thing.

## The defect this exists for

A package whose `exports` map names a path the tarball does not contain
installs cleanly and fails at the consumer's first import. `npm publish` does
not catch it, `tsc --build` does not catch it, and the repository's own path
mapping hides it, because inside the workspace `@ocelli/core` resolves through
a symlink to `src/` and never consults the tarball at all.

So the tarballs are built, installed OUTSIDE the workspace, and imported.

## Why there is no bundler

`@ocelli/core` has no runtime dependency and emits ESM with declarations
through `tsc --build`. A bundler over a dependency-free ESM package produces
the same modules with an extra tool in the path. `AGENTS.md` asks whether a
construct reduces the cases a reader must consider or increases the places
they must look, and it increases them.

What the story needed from "bundling" is that a consumer's resolver handles the
published tarball. That is a property of the tarball, not of a build step here,
and it is what the consumer proof below actually measures.

Usage:
  python3 scripts/package_check.py
  python3 scripts/package_check.py --no-publish-dry-run   # skip the registry
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = ["core", "react"]

# Present in the tarball or the package is not publishable. The two licence
# files matter as much as the code: the manifest says "MIT OR Apache-2.0" and a
# tarball that carries neither text makes that a claim rather than a grant.
REQUIRED = ["package.json", "README.md", "LICENSE-MIT", "LICENSE-APACHE"]

# Never in the tarball. `src` and the build info are noise a consumer pays to
# download, and `.tsbuildinfo` additionally leaks absolute paths from the
# machine that built it.
FORBIDDEN_PREFIXES = ["src/", "node_modules/"]
FORBIDDEN_SUFFIXES = [".tsbuildinfo"]


def run(cmd: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)


def workspace_version() -> str:
    """The Rust workspace version. One number across both toolchains."""
    for line in (ROOT / "Cargo.toml").read_text().splitlines():
        stripped = line.strip()
        if stripped.startswith("version =") and '"' in stripped:
            return stripped.split('"')[1]
    raise RuntimeError("Cargo.toml [workspace.package] has no version")


def tarball_paths(directory: Path) -> tuple[Path, list[str]]:
    """Build the real tarball and list what is inside it."""
    result = run(["npm", "pack", "--pack-destination", str(directory)],
                 ROOT / "packages" / directory.name)
    if result.returncode != 0:
        raise RuntimeError(f"npm pack failed:\n{result.stderr.strip()}")
    tarballs = list(directory.glob("*.tgz"))
    if len(tarballs) != 1:
        raise RuntimeError(f"expected one tarball in {directory}, "
                           f"found {len(tarballs)}")
    with tarfile.open(tarballs[0]) as archive:
        # Every path inside an npm tarball is prefixed with "package/".
        names = [n.removeprefix("package/") for n in archive.getnames()
                 if n != "package"]
    return tarballs[0], names


def check_manifest(name: str, manifest: dict, version: str) -> list[str]:
    problems = []
    if manifest.get("version") != version:
        problems.append(
            f"@ocelli/{name} is version {manifest.get('version')!r} and the "
            f"Rust workspace is {version!r}. docs/RELEASE.md says the crates "
            f"and the packages version together and that a skew between them "
            f"is not a supported configuration.")

    dependency = manifest.get("dependencies", {}).get("@ocelli/core")
    if name == "react":
        if dependency != version:
            problems.append(
                f"@ocelli/react depends on @ocelli/core {dependency!r}, which "
                f"is not the version @ocelli/core publishes, {version!r}. "
                f"That resolves to whatever the registry has.")
    return problems


def check_tarball(name: str, manifest: dict, names: list[str]) -> list[str]:
    problems = []
    present = set(names)

    for required in REQUIRED:
        if required not in present:
            problems.append(f"@ocelli/{name} tarball has no {required}")

    for path in names:
        for prefix in FORBIDDEN_PREFIXES:
            if path.startswith(prefix):
                problems.append(
                    f"@ocelli/{name} tarball carries {path}, which a consumer "
                    f"downloads and never uses")
        for suffix in FORBIDDEN_SUFFIXES:
            if path.endswith(suffix):
                problems.append(
                    f"@ocelli/{name} tarball carries {path}. It is build "
                    f"state, and it embeds absolute paths from the machine "
                    f"that produced it")

    # THE CHECK THIS FILE EXISTS FOR. Every path the manifest advertises has to
    # be in the tarball. npm does not verify this and a consumer finds out at
    # their first import.
    advertised: list[tuple[str, str]] = []
    for field in ("main", "types", "module"):
        if field in manifest:
            advertised.append((field, manifest[field]))

    def walk(node, trail: str) -> None:
        if isinstance(node, str):
            advertised.append((trail, node))
        elif isinstance(node, dict):
            for key, value in node.items():
                walk(value, f"{trail}.{key}")

    walk(manifest.get("exports", {}), "exports")

    for field, target in advertised:
        relative = target.removeprefix("./")
        if relative not in present:
            problems.append(
                f"@ocelli/{name} advertises {field} = {target!r} and the "
                f"tarball has no {relative}. This installs cleanly and fails "
                f"at the consumer's first import.")
    return problems


def check_consumer(work: Path, tarballs: list[Path]) -> list[str]:
    """Install the real tarballs outside the workspace and import them.

    Outside is the point. An install inside the npm workspace resolves
    @ocelli/core through the workspace link to `src/` and proves nothing about
    the tarball.
    """
    consumer = work / "consumer"
    consumer.mkdir()
    (consumer / "package.json").write_text(json.dumps({
        "name": "ocelli-packaging-consumer",
        "private": True,
        "version": "0.0.0",
        "type": "module",
    }, indent=2) + "\n")

    install = run(["npm", "install", "--no-audit", "--no-fund",
                   *[str(t) for t in tarballs]], consumer)
    if install.returncode != 0:
        return [f"a consumer outside the workspace could not install the "
                f"tarballs:\n{install.stderr.strip()}"]

    problems = []

    # 1. It executes under plain node as ESM.
    (consumer / "smoke.mjs").write_text(
        'import { VERSION, coreAvailable } from "@ocelli/core";\n'
        'if (typeof VERSION !== "string") { throw new Error("VERSION"); }\n'
        'if (coreAvailable() !== false) { throw new Error("coreAvailable"); }\n'
        'console.log("ok", VERSION);\n')
    smoke = run(["node", "smoke.mjs"], consumer)
    if smoke.returncode != 0:
        problems.append(
            f"the published @ocelli/core does not import under node:\n"
            f"{smoke.stderr.strip()}")

    # 2. It type-checks under BOTH resolution modes. `bundler` is what a Vite
    #    or webpack consumer uses and `node16` is what a plain tsc consumer
    #    uses, and an exports map can satisfy one and not the other.
    (consumer / "check.ts").write_text(
        'import { VERSION, coreAvailable } from "@ocelli/core";\n'
        'import { OcelliViewport } from "@ocelli/react";\n'
        'export const version: string = VERSION;\n'
        'export const available: boolean = coreAvailable();\n'
        'export const component = OcelliViewport;\n')
    typescript = ROOT / "node_modules" / "typescript" / "bin" / "tsc"
    if not typescript.exists():
        problems.append("typescript is not installed, run npm ci")
        return problems

    for resolution in ("bundler", "node16"):
        module = "esnext" if resolution == "bundler" else "node16"
        (consumer / f"tsconfig.{resolution}.json").write_text(json.dumps({
            "compilerOptions": {
                "target": "ES2022",
                "lib": ["ES2022", "DOM"],
                "module": module,
                "moduleResolution": resolution,
                "strict": True,
                "noEmit": True,
                "jsx": "react-jsx",
                "skipLibCheck": True,
            },
            "files": ["check.ts"],
        }, indent=2) + "\n")
        checked = run(["node", str(typescript), "-p",
                       f"tsconfig.{resolution}.json"], consumer)
        if checked.returncode != 0:
            problems.append(
                f"the published packages do not type-check under "
                f"moduleResolution {resolution!r}:\n"
                f"{checked.stdout.strip() or checked.stderr.strip()}")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-publish-dry-run", action="store_true")
    args = parser.parse_args()

    if not (ROOT / "node_modules").is_dir():
        print("SKIPPED: node_modules is absent, run npm ci")
        return 3

    version = workspace_version()
    problems: list[str] = []

    build = run(["npm", "run", "build"], ROOT)
    if build.returncode != 0:
        print("FAIL: npm run build")
        print(build.stdout.strip() or build.stderr.strip())
        return 1

    with tempfile.TemporaryDirectory() as temporary:
        work = Path(temporary)
        tarballs = []
        for name in PACKAGES:
            manifest = json.loads(
                (ROOT / "packages" / name / "package.json").read_text())
            problems += check_manifest(name, manifest, version)

            directory = work / name
            directory.mkdir()
            try:
                tarball, names = tarball_paths(directory)
            except RuntimeError as exc:
                problems.append(f"@ocelli/{name}: {exc}")
                continue
            tarballs.append(tarball)
            problems += check_tarball(name, manifest, names)

        if len(tarballs) == len(PACKAGES) and not problems:
            problems += check_consumer(work, tarballs)

    # `npm publish --dry-run` does everything a publish does except publish.
    # It is the only step that exercises the registry-facing path at all,
    # including the `publishConfig.access` setting both packages carry.
    #
    # An earlier draft refused to run when NPM_TOKEN or an authenticated
    # .npmrc was present, on the theory that a future edit might remove the
    # --dry-run. That guard was removed: a developer logged into npm for any
    # unrelated project would have had this gate fail on them, and a gate that
    # fails for reasons nobody can act on is a gate that gets disabled, which
    # AGENTS.md makes a WORKFLOW.md change. Publishing is /release's, and the
    # protection against publishing here is that this command cannot.
    if not args.no_publish_dry_run and not problems:
        for name in PACKAGES:
            dry = run(["npm", "publish", "--dry-run"],
                      ROOT / "packages" / name)
            if dry.returncode != 0:
                problems.append(
                    f"@ocelli/{name} publish dry run failed:\n"
                    f"{dry.stderr.strip()}")

    if problems:
        print("FAIL: npm packaging")
        for problem in problems:
            print(f"  {problem}")
        return 1

    print(f"OK: {len(PACKAGES)} package(s) at {version}, tarball contents and "
          f"exports verified, and a consumer outside the workspace resolves "
          f"them under bundler and node16")
    return 0


if __name__ == "__main__":
    sys.exit(main())
