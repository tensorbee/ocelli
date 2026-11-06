#!/usr/bin/env python3
"""The SIMD verification variant's dependency rlibs differ from the plain ones.

F-233. The story row: "the SIMD codec verification variants pass `+simd128` to
every crate through a target-scoped RUSTFLAGS, a native arm step refuses a
SIMD variant whose dependency rlibs match the plain build".

The defect this rejects is F-229's review finding D2: `cargo rustc -- -C
target-feature=+simd128` hands the flag to the final crate only, the example,
and compiles every dependency without it, so the "+simd128" build held rlibs
that were byte-identical to the plain build's. The native arm now builds the
SIMD variant with `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS` and runs this
check after each build, against the crate list F-233 records: `ocelli_codec`
and `openjph_core`, plus `jxl_grid`, `jxl_render` and `jxl_modular` now that
F-229 has landed.

Usage:
  python3 scripts/simd_variant_check.py <plain-target-dir> \
      <simd-target-dir> <crate-name>...

Both verification examples build with `--target wasm32-unknown-unknown` and
`--release`, so each `--target-dir` lays rlibs out at DEPS below it.
"""

from __future__ import annotations

import hashlib
import sys
from pathlib import Path

DEPS = Path("wasm32-unknown-unknown") / "release" / "deps"


def newest_rlib(deps: Path, crate: str) -> tuple[str, str] | None:
    """The newest `lib<crate>-*.rlib` under `deps`, name and SHA-256.

    The newest, and not the only one, because cargo leaves a superseded rlib
    in a reused target directory after any change to the flags it was built
    with. The native arm's SIMD directory has held the pre-F-233 rlibs beside
    the current ones ever since the command changed, and a comparison over
    every rlib would keep answering from that history: a later SIMD build
    that lost the flag would write plain's own rlib beside a stale differing
    one and the two sets would still disagree, so the regression would pass.
    The newest file is the one the build just wrote. `None` is a real answer:
    the caller refuses a crate found on neither side, naming the side and the
    path it looked under.
    """
    if not deps.is_dir():
        return None
    candidates = sorted(deps.glob(f"lib{crate}-*.rlib"))
    if not candidates:
        return None
    path = max(candidates, key=lambda item: item.stat().st_mtime)
    return path.name, hashlib.sha256(path.read_bytes()).hexdigest()


def check(plain: Path, simd: Path, crates: list[str]) -> list[str]:
    """One refusal per listed crate whose rlibs are identical or missing.

    Cargo derives an rlib's file name from the crate's metadata hash, so the
    same name with the same digest on both sides is the byte-identical state.
    A name on one side only means that side never compiled the crate.
    """
    problems: list[str] = []
    for crate in crates:
        sides = {"plain": newest_rlib(plain / DEPS, crate),
                 "simd": newest_rlib(simd / DEPS, crate)}
        missing = [label for label, found in sides.items() if found is None]
        for label in missing:
            root = plain if label == "plain" else simd
            problems.append(
                f"{crate} has no lib{crate}-*.rlib under the {label} variant "
                f"at {root / DEPS}. Either that build did not run, the crate "
                f"is not in the example's dependency closure, or the "
                f"directory is not the release wasm32 target directory the "
                f"native arm builds.")
        if missing:
            continue
        if sides["plain"] == sides["simd"]:
            problems.append(
                f"{crate}'s release rlib is byte-identical in the plain and "
                f"SIMD variants: {sides['plain'][0]}. The SIMD build did not "
                f"reach this crate, which is what `cargo rustc -- -C "
                f"target-feature=+simd128` does to every dependency.")
    return problems


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print("usage: simd_variant_check.py <plain-target-dir> "
              "<simd-target-dir> <crate-name>...", file=sys.stderr)
        return 2
    problems = check(Path(argv[0]), Path(argv[1]), argv[2:])
    if problems:
        print("FAIL: simd variant rlibs")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: simd variant rlibs differ for {', '.join(argv[2:])}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
