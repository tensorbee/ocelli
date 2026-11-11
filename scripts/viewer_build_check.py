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
chunk keeps the runtime `import()` of the encoded pkg URL. The core is
reported not built because that runtime import fails. F-266 removed the
worker's preflight `fetch()` of that URL, so the check no longer expects one:
Vite's dev and preview servers answer a missing path with the HTML fallback
and a 200, so a fetch never decided anything, and a plain static host answers
404 only for a reach the build should not emit.

Present: the build passes and the wasm glue and the `.wasm` are emitted as
their own files, and the worker references its own emitted chunk. Any reach
at the build machine's absolute `/@fs` pkg path is refused, in a fetch, an
import or any other position, including the spellings where one path segment
is decoded and the rest stays encoded. A spelling of the real checkout root
is refused the same way, because the temporary tree links node_modules and
packages into it. The intentional absent-state external
encoded import is the one exemption.

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


# The temporary tree's directory prefix. The space is deliberate. The config
# decodes its URL so that Rollup can load a pkg under a path with a space, and
# an encoded URL there is ENOENT, so every run of the gate builds under a path
# that would need encoding.
TREE_PREFIX = "viewer build-"


def wasm_url(base: Path) -> str:
    """The encoded browser URL, `'/@fs' + wasmPackage.pathname`.

    `URL.pathname` encodes exactly once, so a real space is `%20`, a literal
    percent is `%25` and a unicode name is its UTF-8 percent form. The
    browser re-encodes nothing: the value is already the encoded form. The
    check reads the emitted worker for that exact string. The encoding is
    Node's own, computed by the same engine that builds the viewer, not a
    Python approximation: `+`, `!`, `$`, `&` and other punctuation encode
    differently, and guessing here would desynchronize the expected string
    from what the build emits.
    """
    return f"/@fs{_node_url_pathname(base / PKG / 'ocelli_wasm.js')}"


def _node_url_pathname(path: Path) -> str:
    """`require('node:url').pathToFileURL(path).pathname`, via `node -e`.

    `pathToFileURL` is the single source of truth for the encoding Node and
    the browser agree on. Running it through `node -e` keeps the check's
    expectation produced by the same runtime that runs Vite; `node` is
    already a hard requirement of every build this gate performs. Only the
    pathname is printed, with no trailing newline, so the caller receives
    the exact emitted form.
    """
    return subprocess.run(
        ["node", "-e",
         "process.stdout.write("
         "require('node:url').pathToFileURL(process.argv[1]).pathname)",
         str(path)],
        check=True, capture_output=True, text=True).stdout



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


def emitted(viewer: Path) -> tuple[list[str], dict[str, bytes]]:
    """Every emitted file under dist/, and each file's raw bytes.

    The names are the map's keys: dist-relative, sorted, recursive. The map
    is the raw-byte view the contract names; nothing is decoded here. The
    worker chunk is resolved by basename against the keys, and its text is
    decoded only when a checker actually reads it.
    """
    dist = viewer / "dist"
    if not dist.is_dir():
        return [], {}
    raw = {p.relative_to(dist).as_posix(): p.read_bytes()
           for p in sorted(dist.rglob("*")) if p.is_file()}
    return sorted(raw), raw


def _text(raw: dict[str, bytes], name: str) -> str | None:
    """One emitted file's decoded text, for a worker-role read.

    Returns None for a file that is not valid UTF-8. A valid WASM binary is
    carried raw and is never decoded; a `.map` is normally JSON text and
    decodes the same bounded way. No blanket non-UTF-8 refusal and no claim
    about obfuscated JS lives here.
    """
    blob = raw.get(name)
    if blob is None:
        return None
    try:
        return blob.decode("utf-8")
    except UnicodeDecodeError:
        return None


def _calls(text: str, callee: str, url: str) -> bool:
    quoted = re.escape(url)
    pattern = rf"\b{callee}\(\s*([\"'`]){quoted}\1\s*\)"
    return re.search(pattern, text) is not None


def _workers(names: list[str]) -> list[str]:
    """The worker chunks, matched by basename against dist-relative keys."""
    return [n for n in names
            if n.rsplit("/", 1)[-1].startswith("core-worker-")
            and n.rsplit("/", 1)[-1].endswith(".js")]



def absent_problems(
        names: list[str], raw: dict[str, bytes], url: str,
        base: Path) -> list[str]:
    """What is wrong with a build made with no pkg present.

    F-266 drops the old fetch-shape expectation deliberately: the worker's
    preflight `fetch()` of the package URL before its import was removed with
    the import-only loading change, so no absence refusal names a fetch. The
    intentional external encoded `/@fs` import stays, and is the one shape
    exempt from the pkg-present absolute-path ban: the absence refusal never
    bans the legitimate external import, only a wrong chunk set or a missing
    runtime import. `base` is the actual temporary build root, kept explicit
    so no hidden canonical ROOT exists.
    """
    problems: list[str] = []
    if len(_workers(names)) != 1:
        problems.append(
            f"with the pkg absent the build emitted "
            f"{len(_workers(names))} core-worker chunk(s), expected one")
        return problems
    wasm = [n for n in names
            if n.rsplit("/", 1)[-1].startswith("ocelli_wasm")]
    if wasm:
        problems.append(
            f"with the pkg absent the build emitted {', '.join(wasm)}, "
            f"which has no source to come from")
    worker = _worker_text(raw)
    if worker is None or not _calls(worker, "import", url):
        problems.append(
            f"with the pkg absent the worker chunk does not keep a runtime "
            f"import() of {url}, so it cannot report the core as not built")
    return problems


def _worker_text(raw: dict[str, bytes]) -> str | None:
    """The single worker chunk's decoded text, or None.

    Resolution is by basename against the dist-relative keys, per the
    approved contract. A dist with zero or several worker chunks has no one
    text; the worker-count refusals name that before this matters.
    """
    workers = _workers(list(raw))
    if len(workers) != 1:
        return None
    return _text(raw, workers[0])


def _known_roots(base: Path) -> list[tuple[bytes, str]]:
    """The known-root byte patterns, each with the label its refusal names.

    Two roots are known. `base` is the actual temporary build root. The
    real checkout `ROOT` is known too, because `make_tree` symlinks
    `node_modules` and `packages` into it, so a build that resolves those
    symlinks spells the checkout path rather than the base. Each root
    contributes its raw UTF-8 bytes and its once-URL-encoded form, exactly
    what `URL.pathname` yields per `wasm_url`, computed by Node's
    `pathToFileURL` for the same reason `wasm_url` uses it: Python's
    `quote` would disagree with the build's own encoding on `+` and other
    punctuation. They are separate patterns, not one concatenated with
    `/@fs`.
    """
    patterns: list[tuple[bytes, str]] = []
    for path, noun in ((base.resolve(), "root"), (ROOT, "checkout root")):
        patterns.append((path.as_posix().encode("utf-8"), f"raw {noun}"))
        patterns.append((_node_url_pathname(path).encode("utf-8"),
                         f"encoded {noun}"))
    return patterns


# The filename continuation bytes. A root with one of these glued to
# either side is part of a longer name, which is a different path.
_NAME_BYTES = frozenset(
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-")


def _is_name_byte(byte: int) -> bool:
    """Whether one byte continues a filename across a root boundary.

    The continuation set is the ASCII name characters `[A-Za-z0-9._-]`
    plus every non-ASCII byte, so a non-ASCII character glues the same
    way an ASCII one does.
    """
    return byte in _NAME_BYTES or byte >= 0x80


def _dot_segment_before(blob: bytes, start: int) -> bool:
    """Whether relative dots end immediately before `start`.

    The dots immediately before the position must run back to the
    start of the content or to a `/`, the byte-level form of the `.`
    and `..` segments a sourcemap's relative sources spell, as in
    `../../w/x.js` for a checkout at `/w` through the symlinked
    packages. A dot glued into a longer name, as in `pkg./w`, is not
    a relative segment.
    """
    index = start - 1
    if index < 0 or blob[index] != ord("."):
        return False
    while index >= 0 and blob[index] == ord("."):
        index -= 1
    return index < 0 or blob[index] == ord("/")


def _spells_root(blob: bytes, root: bytes) -> bool:
    """Whether the content spells the root as a path prefix.

    The root counts when it stands alone: the byte before it starts the
    content, is not a name byte, or ends relative dots that run back
    to the content start or a `/`, and the byte after it ends the
    content or is not one. A root glued into a longer name is not a
    root spelling: a checkout at `/ocelli` matches neither a healthy
    worker's `import("./ocelli_wasm-a1.js")` nor a URL that merely
    carries the name. Reaches a bare substring would catch stay caught
    on the right: a root followed by a quote, an apostrophe, a
    backtick, an encoded separator, a query mark, a fragment mark, a
    backslash, whitespace or the end of the content is a reach, in a
    JSON field or a const binding as well as an import. On the left the
    boundary names one acceptance. A host-prefixed spelling,
    `http://localhost:5173/w/...` or `file://localhost/w/...`, is glued
    to the host's last letter or digit and is not a reach, because
    bytes alone cannot tell that host from the github URL name a
    refusal must not fire on.
    """
    start = blob.find(root)
    while start != -1:
        end = start + len(root)
        before = (start == 0
                  or not _is_name_byte(blob[start - 1])
                  or _dot_segment_before(blob, start))
        after = end == len(blob) or not _is_name_byte(blob[end])
        if before and after:
            return True
        start = blob.find(root, start + 1)
    return False


def _absolute_reach(raw: dict[str, bytes], base: Path) -> str | None:
    """The first file reaching a known root or the /@fs prefix, named.

    Scans every emitted file's raw bytes, binaries included. The known
    roots are the build `base` and the real checkout `ROOT`, each as raw
    UTF-8 bytes and as its once-URL-encoded form, per `_known_roots`,
    matched only as a path prefix with a boundary by `_spells_root`, and
    the standalone `/@fs` loader prefix is its own bare pattern. A valid
    WASM binary is never decoded. The gate builds only with the stand-in
    stub, so the real wasm is never scanned here, and no other absolute
    path is banned.
    """
    roots = _known_roots(base)
    for name in sorted(raw):
        blob = raw[name]
        for pattern, label in roots:
            if _spells_root(blob, pattern):
                return f"{name} ({label})"
        if b"/@fs" in blob:
            return f"{name} (/@fs)"
    return None



def present_problems(
        names: list[str], raw: dict[str, bytes], url: str,
        base: Path) -> list[str]:
    """What is wrong with a build made with the pkg present.

    The worker must reference the emitted glue chunk, and no emitted file
    may reach the build machine's absolute path, in any position, across
    every recursive output. The pkg-ABSENT build's external encoded import
    is the deliberate exemption checked by `absent_problems`, and the
    decoded build specifier that Rollup itself rewrites is not a runtime
    reach: what lands in the emitted worker is the rewritten chunk
    specifier. `base` is the actual temporary build root the caller built.
    """
    problems: list[str] = []
    if len(_workers(names)) != 1:
        problems.append(
            f"with the pkg present the build emitted "
            f"{len(_workers(names))} core-worker chunk(s), expected one")
        return problems
    if not any(re.fullmatch(r"ocelli_wasm-[\w-]+\.js", n.rsplit("/", 1)[-1])
               for n in names):
        problems.append(
            "with the pkg present the build emitted no ocelli_wasm chunk, so "
            "the wasm glue was not bundled")
    glue = [n.rsplit("/", 1)[-1] for n in names
            if re.fullmatch(r"ocelli_wasm-[\w-]+\.js", n.rsplit("/", 1)[-1])]
    worker_text = _worker_text(raw)
    if worker_text is None or not any(
            _calls(worker_text, "import", f"./{g}") for g in glue):
        problems.append(
            "with the pkg present the emitted worker does not import the "
            "emitted ocelli_wasm glue chunk, so the wasm glue would not load")
    if not any(re.fullmatch(r"ocelli_wasm_bg-[\w-]+\.wasm",
                            n.rsplit("/", 1)[-1]) for n in names):
        problems.append(
            "with the pkg present the build emitted no ocelli_wasm_bg .wasm "
            "asset")
    reach = _absolute_reach(raw, base)
    if reach is not None:
        problems.append(
            f"with the pkg present the emitted file {reach} reaches the "
            f"build machine's absolute path, so the dist depends on the "
            f"machine that built it")
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
    with tempfile.TemporaryDirectory(prefix=TREE_PREFIX) as temporary:
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
            names, raw = emitted(viewer)
            check = present_problems if with_pkg else absent_problems
            problems += check(names, raw, wasm_url(base), base)

    if problems:
        print("FAIL: the example viewer build")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: the example viewer builds with the wasm pkg absent, keeping "
          "the runtime import, and with a stand-in pkg present, emitting the "
          "wasm chunk")
    return 0


if __name__ == "__main__":
    sys.exit(main())
