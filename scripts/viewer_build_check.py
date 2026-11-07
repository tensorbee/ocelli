#!/usr/bin/env python3
"""The example viewer builds with the wasm package absent and with it present.

**The viewer's build output depends on whether `crates/ocelli-wasm/pkg` exists
when `vite build` runs**, and this check builds both states so neither can
regress unseen. `examples/viewer-react/vite.config.ts` folds the worker's
dynamic import into a constant `/@fs` URL. Vite 7's Rollup worker sub-build
resolves a constant specifier and loads the file, so a clean clone, which has
no pkg, failed `npm run build` with ENOENT after F-217 moved Vite 8 to 7. The
config now marks that URL external when the pkg is absent.

The CI frontend job has no Rust toolchain and never has a pkg, so the real
tree only ever exercises the absent state there, and a developer machine with
a built pkg only ever exercises the present state. Neither run proves the
other. This script copies the viewer into a temporary tree shaped like the
repository, once with no pkg and once with a stand-in pkg, builds each with
the repository's own Vite, and reads what was emitted.

Absent: the build passes, no wasm chunk or asset is emitted, and the worker
chunk keeps the runtime `import()` of the pkg URL and the `fetch()` probe in
front of it, which is the path that reports the core as not built.

Present: the build passes and the wasm glue and the `.wasm` are emitted as
their own files, and the worker no longer imports the `/@fs` URL at runtime.

The stand-in pkg is two files, a glue module that references its `.wasm`
through `new URL(..., import.meta.url)` as `wasm-pack --target web` output
does, and a valid module padded past Vite's inline limit. It is built here
rather than taken from `bin/ocelli.sh wasm`, because that needs a Rust
toolchain the frontend job does not install. It proves what Vite does with a pkg that exists, not that
the real pkg instantiates.

Run after `npm run build`, which `bin/ocelli.sh gate packages` does through
`scripts/package_check.py` first. The two library packages must have a
`dist` for the copied viewer to resolve them.

Usage:
  python3 scripts/viewer_build_check.py
"""

from __future__ import annotations

import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VIEWER = Path("examples") / "viewer-react"
PKG = Path("crates") / "ocelli-wasm" / "pkg"
# What the viewer's build reads. `tsconfig.json` extends the root base config,
# so that is copied too, at the same relative place, and it references the two
# library packages, which esbuild follows, so `packages` is linked beside it.
VIEWER_FILES = ("index.html", "package.json", "tsconfig.json",
                "vite.config.ts", "src")
LIBRARY_ENTRIES = ("packages/core/dist/index.js",
                   "packages/react/dist/index.js")

STUB_GLUE = """\
// A stand-in for wasm-pack --target web output, written by
// scripts/viewer_build_check.py. It references its module the same way.
export default async function init(input) {
  const url = input ?? new URL("ocelli_wasm_bg.wasm", import.meta.url);
  const { instance } = await WebAssembly.instantiateStreaming(fetch(url), {});
  return instance.exports;
}
"""


def _stub_wasm() -> bytes:
    """A valid module, the header and one 8 KiB custom section.

    The padding is load-bearing. Vite inlines an asset under
    `build.assetsInlineLimit`, 4096 bytes by default, as a data URL, so an
    eight-byte module would never be emitted as a file and the present-state
    check would refuse a correct build. A real pkg is far above the limit.
    """
    name = b"pad"
    content = bytes([len(name)]) + name + bytes(8192)
    size, leb = len(content), bytearray()
    while True:
        byte, size = size & 0x7F, size >> 7
        leb.append(byte | (0x80 if size else 0))
        if not size:
            break
    return b"\x00asm\x01\x00\x00\x00" + b"\x00" + bytes(leb) + content


STUB_WASM = _stub_wasm()


def wasm_url(base: Path) -> str:
    """The URL the config defines for a tree rooted at `base`."""
    return f"/@fs{(base / PKG / 'ocelli_wasm.js').as_posix()}"


def make_tree(base: Path, with_pkg: bool) -> Path:
    viewer = base / VIEWER
    viewer.mkdir(parents=True)
    for name in VIEWER_FILES:
        source = ROOT / VIEWER / name
        if source.is_dir():
            shutil.copytree(source, viewer / name)
        else:
            shutil.copy2(source, viewer / name)
    shutil.copy2(ROOT / "tsconfig.base.json", base / "tsconfig.base.json")
    (base / "node_modules").symlink_to(ROOT / "node_modules")
    (base / "packages").symlink_to(ROOT / "packages")
    if with_pkg:
        pkg = base / PKG
        pkg.mkdir(parents=True)
        (pkg / "ocelli_wasm.js").write_text(STUB_GLUE, encoding="utf-8")
        (pkg / "ocelli_wasm_bg.wasm").write_bytes(STUB_WASM)
    return viewer


def build(viewer: Path) -> subprocess.CompletedProcess[str]:
    vite = ROOT / "node_modules" / "vite" / "bin" / "vite.js"
    return subprocess.run(["node", str(vite), "build", "--logLevel", "warn"],
                          cwd=viewer, capture_output=True, text=True,
                          check=False)


def emitted(viewer: Path) -> tuple[list[str], str]:
    """The emitted asset names, and the worker chunk's text."""
    assets = viewer / "dist" / "assets"
    names = sorted(p.name for p in assets.iterdir()) if assets.is_dir() else []
    workers = _workers(names)
    text = ""
    if len(workers) == 1:
        text = (assets / workers[0]).read_text(encoding="utf-8")
    return names, text


def _calls(text: str, callee: str, url: str) -> bool:
    quoted = re.escape(url)
    pattern = rf"\b{callee}\(\s*([\"'`]){quoted}\1\s*\)"
    return re.search(pattern, text) is not None


def _workers(names: list[str]) -> list[str]:
    return [n for n in names
            if n.startswith("core-worker-") and n.endswith(".js")]


def absent_problems(names: list[str], worker: str, url: str) -> list[str]:
    """What is wrong with a build made with no pkg present."""
    problems: list[str] = []
    if len(_workers(names)) != 1:
        problems.append(
            f"with the pkg absent the build emitted "
            f"{len(_workers(names))} core-worker chunk(s), expected one")
        return problems
    wasm = [n for n in names if n.startswith("ocelli_wasm")]
    if wasm:
        problems.append(
            f"with the pkg absent the build emitted {', '.join(wasm)}, "
            f"which has no source to come from")
    if not _calls(worker, "import", url):
        problems.append(
            f"with the pkg absent the worker chunk does not keep a runtime "
            f"import() of {url}, so it cannot report the core as not built")
    if not _calls(worker, "fetch", url):
        problems.append(
            f"with the pkg absent the worker chunk does not probe {url} with "
            f"fetch() before importing it")
    return problems


def present_problems(names: list[str], worker: str, url: str) -> list[str]:
    """What is wrong with a build made with the pkg present."""
    problems: list[str] = []
    if len(_workers(names)) != 1:
        problems.append(
            f"with the pkg present the build emitted "
            f"{len(_workers(names))} core-worker chunk(s), expected one")
        return problems
    if not any(re.fullmatch(r"ocelli_wasm-[\w-]+\.js", n) for n in names):
        problems.append(
            "with the pkg present the build emitted no ocelli_wasm chunk, so "
            "the wasm glue was not bundled")
    if not any(re.fullmatch(r"ocelli_wasm_bg-[\w-]+\.wasm", n)
               for n in names):
        problems.append(
            "with the pkg present the build emitted no ocelli_wasm_bg .wasm "
            "asset")
    if _calls(worker, "import", url):
        problems.append(
            f"with the pkg present the worker chunk still imports {url} at "
            f"runtime, so the pkg was left external")
    return problems


def main() -> int:
    if not (ROOT / "node_modules" / "vite").is_dir():
        print("SKIPPED: node_modules is absent, run npm ci")
        return 3
    missing = [e for e in LIBRARY_ENTRIES if not (ROOT / e).is_file()]
    if missing:
        print(f"FAIL: {', '.join(missing)} is absent, run npm run build "
              f"first")
        return 1

    problems: list[str] = []
    with tempfile.TemporaryDirectory(prefix="viewer-build-") as temporary:
        work = Path(temporary).resolve()
        for with_pkg in (False, True):
            state = "present" if with_pkg else "absent"
            base = work / state
            viewer = make_tree(base, with_pkg)
            built = build(viewer)
            if built.returncode != 0:
                problems.append(
                    f"vite build failed with the pkg {state}:\n"
                    f"{built.stderr.strip() or built.stdout.strip()}")
                continue
            names, worker = emitted(viewer)
            check = present_problems if with_pkg else absent_problems
            problems += check(names, worker, wasm_url(base))

    if problems:
        print("FAIL: the example viewer build")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: the example viewer builds with the wasm pkg absent, keeping "
          "the runtime probe, and with it present, emitting the wasm chunk")
    return 0


if __name__ == "__main__":
    sys.exit(main())
