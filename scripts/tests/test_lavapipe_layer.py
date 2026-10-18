#!/usr/bin/env python3
"""F-X002. The layer 2 record is either absent or an explicit drop."""

from __future__ import annotations

import io
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import lavapipe_layer  # noqa: E402


class LayerRecord(unittest.TestCase):
    def run_main(self, path: Path) -> tuple[int, str]:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            code = lavapipe_layer.main(["lavapipe_layer.py", str(path)])
        return code, stdout.getvalue()

    def test_a_missing_file_requires_the_probe(self):
        missing = Path(tempfile.mkdtemp()) / "lavapipe-layer.json"
        self.assertEqual(lavapipe_layer.decide(missing), "probe")

    def test_a_dropped_record_with_a_reason_is_the_drop(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lavapipe-layer.json"
            path.write_text('{"status":"dropped","reason":"no ICD on ubuntu-latest"}\n')
            self.assertEqual(lavapipe_layer.decide(path), "dropped")
            self.assertEqual(
                lavapipe_layer.drop_line(path),
                "layer 2 dropped: no ICD on ubuntu-latest",
            )

    def test_an_empty_reason_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lavapipe-layer.json"
            path.write_text('{"status":"dropped","reason":""}\n')
            self.assertEqual(lavapipe_layer.decide(path), "refused")

    def test_skip_and_pass_are_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lavapipe-layer.json"
            path.write_text('{"status":"skip","reason":"not installed"}\n')
            self.assertEqual(lavapipe_layer.decide(path), "refused")
            path.write_text('{"status":"pass","reason":"assumed"}\n')
            self.assertEqual(lavapipe_layer.decide(path), "refused")

    def test_a_document_that_is_not_an_object_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lavapipe-layer.json"
            path.write_text("[]\n")
            self.assertEqual(lavapipe_layer.decide(path), "refused")
            path.write_text("not json\n")
            self.assertEqual(lavapipe_layer.decide(path), "refused")

    def test_main_exit_codes_are_what_the_workflow_branches_on(self):
        missing = Path(tempfile.mkdtemp()) / "lavapipe-layer.json"
        code, text = self.run_main(missing)
        self.assertEqual(code, 10)
        self.assertEqual(text, "")

        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "lavapipe-layer.json"
            path.write_text(
                '{"status":"dropped","reason":"no ICD on ubuntu-latest"}\n'
            )
            code, text = self.run_main(path)
            self.assertEqual(code, 0)
            self.assertEqual(text, "layer 2 dropped: no ICD on ubuntu-latest\n")

            refused = [
                '{"status":"skip","reason":"not installed"}\n',
                '{"status":"pass","reason":"assumed"}\n',
                '{"status":"dropped","reason":""}\n',
                '{"status":"dropped","reason":"   "}\n',
                "[]\n",
                "not json\n",
            ]
            for body in refused:
                path.write_text(body)
                code, text = self.run_main(path)
                self.assertEqual(code, 1, body)
                self.assertEqual(text, "", body)


if __name__ == "__main__":
    unittest.main()
