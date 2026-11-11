"""The viewer build check's URL matches the config's, under a path with a space.

`examples/viewer-react/vite.config.ts` defines its wasm URL as `/@fs` and the
decoded path of the pkg, because Rollup loads that specifier as a literal path
when the pkg is present and `%20` there is ENOENT. The check reads the emitted
worker for that exact string. The expected strings are the plain path, written
from that sentence and not from the check's code.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import viewer_build_check as v  # noqa: E402


class WasmUrl(unittest.TestCase):
    def test_a_space_stays_a_space(self) -> None:
        self.assertEqual(
            v.wasm_url(Path("/a b/c")),
            "/@fs/a b/c/crates/ocelli-wasm/pkg/ocelli_wasm.js")

    def test_the_gate_builds_under_a_path_with_a_space(self) -> None:
        self.assertIn(" ", v.TREE_PREFIX)

    def test_the_url_with_a_space_is_what_the_check_finds(self) -> None:
        url = v.wasm_url(Path("/a b/c"))
        worker = f'fetch("{url}");import("{url}")'
        self.assertEqual(
            v.absent_problems(["core-worker-a1.js"], worker, url), [])

    def test_an_encoded_url_in_the_worker_is_refused(self) -> None:
        url = v.wasm_url(Path("/a b/c"))
        encoded = url.replace(" ", "%20")
        worker = f'fetch("{encoded}");import("{encoded}")'
        self.assertNotEqual(
            v.absent_problems(["core-worker-a1.js"], worker, url), [])


if __name__ == "__main__":
    unittest.main()
