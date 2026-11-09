#!/usr/bin/env python3
"""F-110. The decode build does not link wgpu or Naga.

The render build still does. wasm-opt stays a flag list. On wasm32 the
wgpu feature set is the one D-27 records.
"""

from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WASM_CRATE = ROOT / "crates" / "ocelli-wasm" / "Cargo.toml"
RENDER_CRATE = ROOT / "crates" / "ocelli-render" / "Cargo.toml"
COMPUTE_CRATE = ROOT / "crates" / "ocelli-compute" / "Cargo.toml"

WASM_FEATURES = {"std", "parking_lot", "wgsl", "webgpu", "webgl"}
NATIVE_FEATURES = {
    "std", "parking_lot", "dx12", "metal", "gles", "vulkan", "wgsl", "webgpu",
}
# D-33, F-238. What the SHIPPED native table must keep: the D-27 native set
# minus `metal`, which moves to a forwarding feature so the shipped archive
# can build without it. `gles` stays, measured: on Apple the `gles` chain
# never reaches wgpu-hal's own `gles`, so no objc2 crate follows it.
SHIPPED_NATIVE_FEATURES = NATIVE_FEATURES - {"metal"}
NATIVE_CFG = 'cfg(not(target_arch = "wasm32"))'
WASM_CFG = 'cfg(target_arch = "wasm32")'
PACKAGE = re.compile(r"(?:^|\s)(wgpu|naga|ocelli-render) v")


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text())


def target_wgpu(manifest: dict, cfg: str) -> object:
    try:
        return manifest["target"][cfg]["dependencies"]["wgpu"]
    except (KeyError, TypeError):
        return None


def check_wasm_opt(manifest: dict) -> list[str]:
    try:
        flags = manifest["package"]["metadata"]["wasm-pack"]["profile"]["release"]["wasm-opt"]
    except (KeyError, TypeError):
        return ["wasm-opt is absent from the ocelli-wasm release profile"]
    if flags is False or not isinstance(flags, list) or not flags:
        return ["wasm-opt must stay a non-empty flag list"]
    if any(not isinstance(flag, str) for flag in flags):
        return ["wasm-opt flags must be strings"]
    return []


def check_features(manifest: dict) -> list[str]:
    features = manifest.get("features")
    if not isinstance(features, dict):
        return ["ocelli-wasm declares no features"]
    problems = []
    default = features.get("default")
    if default != ["render-worker"]:
        problems.append("the default feature must be render-worker only")
    if "decode-worker" not in features:
        problems.append("decode-worker is not a feature")
    elif features["decode-worker"] not in ([], None):
        problems.append("decode-worker must not enable a dependency")
    render = features.get("render-worker")
    if not isinstance(render, list):
        problems.append("render-worker is not a feature list")
    else:
        needed = {"dep:ocelli-render", "dep:ocelli-viewport", "dep:wgpu"}
        if set(render) != needed:
            problems.append(
                "render-worker must enable ocelli-render, ocelli-viewport and wgpu")
    return problems


def check_trimmed(name: str, manifest: dict, native_webgl: bool,
                  shipped: bool = False,
                  forward_metal: bool | None = None) -> list[str]:
    problems = []
    # `forward_metal` defaults to `shipped`: the crate judged against the
    # shipped table is the one whose `metal` forwarding is required. A crate
    # outside the shipped graph can pass `forward_metal=False` to be judged
    # against the same table without the forwarding.
    if forward_metal is None:
        forward_metal = shipped
    wasm = target_wgpu(manifest, WASM_CFG)
    native = target_wgpu(manifest, NATIVE_CFG)
    if not isinstance(wasm, dict):
        problems.append(f"{name} has no wasm32 wgpu dependency")
    else:
        if wasm.get("default-features") is not False:
            problems.append(f"{name} wasm32 wgpu still uses default features")
        if set(wasm.get("features") or []) != WASM_FEATURES:
            problems.append(f"{name} wasm32 wgpu features are not the D-27 set")
    if not isinstance(native, dict):
        problems.append(f"{name} has no native wgpu dependency")
    else:
        native_features = set(native.get("features") or [])
        # D-27 keeps `metal` in the table for the crates whose native graph is
        # development-only. D-33 moves it to a forwarding feature on the
        # renderer, which the shipped archive is built from, and this half
        # asserts both halves of that: the table keeps every shipped backend,
        # and the manifest still forwards `metal` to wgpu so development can
        # restore the D-27 set. The shipped set is a floor, not the exact
        # table: a table that drops one of its backends is refused, and one
        # that names more is not. `ocelli-compute` is judged against that
        # floor without being in the shipped graph, and names no forwarding
        # feature, because nothing depends on it there and a flag with no
        # named user is forbidden.
        required = SHIPPED_NATIVE_FEATURES if shipped else NATIVE_FEATURES
        if not required <= native_features:
            problems.append(f"{name} native wgpu drops a default backend")
        if native_webgl and "webgl" not in native_features:
            problems.append(f"{name} native wgpu does not enable webgl")
    if forward_metal:
        forwarded = (manifest.get("features") or {}).get("metal")
        if forwarded != ["wgpu/metal"]:
            problems.append(
                f"{name} does not forward metal to wgpu, so the D-27 native "
                f"set is unreachable in development")
    return problems


def manifests() -> list[str]:
    wasm = load(WASM_CRATE)
    problems = check_wasm_opt(wasm)
    problems += check_features(wasm)
    problems += check_trimmed("ocelli-wasm", wasm, False)
    # D-33. The renderer is the crate the shipped native archive is built
    # from, so its table is judged against the shipped set and its `metal`
    # forwarding is required. The compute crate is not in the shipped graph,
    # but it shares the renderer's device, so its table is held to the
    # shipped set as a floor for the day it joins one. It names no `metal`
    # feature: nothing depends on it there, and a flag with no named user is
    # forbidden.
    problems += check_trimmed("ocelli-render", load(RENDER_CRATE), True,
                              shipped=True)
    problems += check_trimmed("ocelli-compute", load(COMPUTE_CRATE), False,
                              shipped=True, forward_metal=False)
    return problems


def cargo_tree(extra: list[str]) -> tuple[int, str]:
    command = [
        "cargo", "tree", "-p", "ocelli-wasm",
        "--target", "wasm32-unknown-unknown",
        "-e", "normal", *extra,
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    text = result.stdout + result.stderr
    return result.returncode, text


def wgpu_features(target: str | None) -> tuple[int, set[str]]:
    """Features Cargo resolved for wgpu on one target.

    This is the workspace resolve, not the `ocelli-wasm` package graph.
    A shared dependency outside a target table shows up here and not in
    `cargo tree -p ocelli-wasm`.
    """
    command = [
        "cargo", "tree", "-e", "normal",
        "--prefix", "none", "--format", "{p}|{f}",
    ]
    if target is not None:
        command.extend(["--target", target])
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    features: set[str] = set()
    for raw in result.stdout.splitlines():
        line = raw.strip()
        if line.endswith("(*)"):
            line = line[: -len("(*)")].rstrip()
        if not line.startswith("wgpu "):
            continue
        _package, _, field = line.partition("|")
        features.update(part.strip() for part in field.split(",") if part.strip())
    return result.returncode, features


def classify_workspace(wasm_code: int, wasm: set[str],
                       host_code: int, host: set[str]) -> list[str]:
    """wasm32 must not resolve a native backend. The host must keep them."""
    problems = []
    if wasm_code != 0:
        problems.append("wasm32 workspace resolve did not run")
    else:
        banned = sorted({"dx12", "metal", "gles", "vulkan", "glsl", "spirv"} & wasm)
        if banned:
            problems.append(
                "wasm32 workspace resolve enables " + ", ".join(banned))
        missing = sorted(WASM_FEATURES - wasm)
        if missing:
            problems.append(
                "wasm32 workspace resolve drops " + ", ".join(missing))
    if host_code != 0:
        problems.append("host workspace resolve did not run")
    else:
        missing = sorted((NATIVE_FEATURES | {"webgl"}) - host)
        if missing:
            problems.append(
                "host workspace resolve drops " + ", ".join(missing))
        banned = sorted({"glsl", "spirv"} & host)
        if banned:
            problems.append(
                "host workspace resolve enables " + ", ".join(banned))
    return problems


def workspace() -> list[str]:
    wasm_code, wasm = wgpu_features("wasm32-unknown-unknown")
    host_code, host = wgpu_features(None)
    return classify_workspace(wasm_code, wasm, host_code, host)


def classify_graph(decode_code: int, decode: str,
                   render_code: int, render: str) -> list[str]:
    problems = []
    if decode_code != 0:
        problems.append("decode-worker graph did not resolve")
    else:
        hit = sorted(set(PACKAGE.findall(decode)))
        if hit:
            problems.append(
                "decode-worker still links " + ", ".join(hit))
    if render_code != 0:
        problems.append("render-worker graph did not resolve")
    elif "wgpu v" not in render:
        problems.append("the default wasm graph does not link wgpu")
    return problems


def graphs() -> list[str]:
    decode_code, decode = cargo_tree([
        "--no-default-features", "--features", "decode-worker",
    ])
    render_code, render = cargo_tree([])
    return classify_graph(decode_code, decode, render_code, render)


def main() -> int:
    problems = manifests()
    if not problems:
        problems = graphs()
    if not problems:
        problems = workspace()
    if problems:
        print("FAIL: decode graph")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print("OK: decode build has no wgpu or naga, render build keeps wgpu, wasm-opt stays on")
    return 0


if __name__ == "__main__":
    sys.exit(main())
