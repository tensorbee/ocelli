#!/usr/bin/env python3
"""Tests for scripts/simd_variant_check.py.

    python3 -B -m unittest discover -s scripts/tests \
      -p 'test_simd_variant_check.py'

Needs nothing but the standard library. Every case plants its rlibs in a
temporary directory and calls `check` directly, so nothing here builds anything
or reads a real target directory.

**Expected values are from F-233's design plan, decision 3, and not from the
implementation.** The check refuses a listed crate whose release rlib is
byte-identical between the plain and the SIMD target directories, and one that
is missing from either side, and it passes when every listed crate differs.
The fixture layout is the one `bin/ocelli.sh native` steps 7 and 10 produce:
`<target-dir>/wasm32-unknown-unknown/release/deps/lib<crate>-<hash>.rlib`,
because both verification examples build with `--target
wasm32-unknown-unknown --release`.
"""

from __future__ import annotations

import sys
import os
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import simd_variant_check  # noqa: E402

DEPS = Path("wasm32-unknown-unknown") / "release" / "deps"


class SimdVariantCheck(unittest.TestCase):

    def plant(self, root: Path, crate: str, digest: str, body: bytes) -> None:
        deps = root / DEPS
        deps.mkdir(parents=True, exist_ok=True)
        (deps / f"lib{crate}-{digest}.rlib").write_bytes(body)

    def check(self, plain: Path, simd: Path,
              crates: list[str]) -> list[str]:
        return simd_variant_check.check(plain, simd, crates)

    def test_an_identical_rlib_is_refused_and_named(self) -> None:
        # The F-229 review's measured state: the "+simd128" build held
        # dependency rlibs byte-identical to the plain build's, because
        # `cargo rustc --` passed the flag to the example crate only.
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            body = b"plain wasm32 release rlib bytes"
            self.plant(plain, "probe_codec", "abc123", body)
            self.plant(simd, "probe_codec", "abc123", body)
            problems = self.check(plain, simd, ["probe_codec"])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("probe_codec", problems[0])
        self.assertIn("byte-identical", problems[0])
        self.assertIn("libprobe_codec-abc123.rlib", problems[0])

    def test_a_missing_side_is_refused_and_named(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            self.plant(plain, "probe_codec", "abc123", b"only here")
            problems = self.check(plain, simd, ["probe_codec"])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("probe_codec", problems[0])
        self.assertIn("simd", problems[0])
        self.assertNotIn("byte-identical", problems[0])

    def test_a_missing_deps_directory_is_refused(self) -> None:
        # A target directory that never held a release wasm32 build must
        # refuse rather than read as an empty pass over zero crates.
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            plain.mkdir()
            simd.mkdir()
            problems = self.check(plain, simd, ["probe_codec"])
        self.assertEqual(len(problems), 2, problems)
        self.assertIn("plain", problems[0])
        self.assertIn("simd", problems[1])

    def test_differing_rlibs_pass(self) -> None:
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            self.plant(plain, "probe_codec", "aaa111", b"plain bytes")
            self.plant(simd, "probe_codec", "bbb222", b"simd bytes")
            self.plant(plain, "probe_other", "ccc333", b"plain two")
            self.plant(simd, "probe_other", "ddd444", b"simd two")
            problems = self.check(plain, simd,
                                  ["probe_codec", "probe_other"])
        self.assertEqual(problems, [])

    def test_a_crate_outside_the_closure_is_refused(self) -> None:
        # A listed crate that neither build produced is missing from both
        # sides, which is a refusal and not a silent pass over an empty set.
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            self.plant(plain, "probe_codec", "abc123", b"plain bytes")
            self.plant(simd, "probe_codec", "def456", b"simd bytes")
            problems = self.check(plain, simd, ["probe_absent"])
        self.assertEqual(len(problems), 2, problems)

    def test_a_stale_rlib_does_not_mask_a_fresh_identical_one(self) -> None:
        # The regression the newest-rlib rule exists for: a later SIMD build
        # that lost the flag writes plain's own rlib beside the stale one a
        # flag change left behind. Comparing every rlib would see two
        # different names on the SIMD side and pass. The stale file is made
        # older by mtime, which is what distinguishes it on a real
        # filesystem, because cargo writes the fresh build after it.
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            body = b"the rlib a no-flag build writes"
            self.plant(plain, "probe_codec", "abc123", body)
            self.plant(simd, "probe_codec", "abc123", body)
            self.plant(simd, "probe_codec", "zzz789", b"a stale differing rlib")
            old, new = 1_000_000_000, 1_500_000_000
            os.utime(simd / DEPS / "libprobe_codec-abc123.rlib", ns=(new, new))
            os.utime(simd / DEPS / "libprobe_codec-zzz789.rlib", ns=(old, old))
            problems = self.check(plain, simd, ["probe_codec"])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("byte-identical", problems[0])

    def test_a_stale_rlib_does_not_force_a_refusal(self) -> None:
        # The state the native arm's own SIMD directory is in after F-233
        # changed the command: the pre-F-233 rlib, byte-identical to plain's,
        # sits beside the fresh flag-built one. The newest file decides.
        with tempfile.TemporaryDirectory() as name:
            base = Path(name)
            plain, simd = base / "plain", base / "simd"
            self.plant(plain, "probe_codec", "abc123", b"plain bytes")
            self.plant(simd, "probe_codec", "abc123", b"plain bytes")
            self.plant(simd, "probe_codec", "bbb222", b"fresh simd bytes")
            old, new = 1_000_000_000, 1_500_000_000
            os.utime(simd / DEPS / "libprobe_codec-abc123.rlib", ns=(old, old))
            os.utime(simd / DEPS / "libprobe_codec-bbb222.rlib", ns=(new, new))
            problems = self.check(plain, simd, ["probe_codec"])
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
