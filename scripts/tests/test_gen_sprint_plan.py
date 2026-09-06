#!/usr/bin/env python3
"""Tests for the protected write mode of scripts/gen_sprint_plan.py."""

from __future__ import annotations

import contextlib
import io
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import gen_sprint_plan  # noqa: E402


class ProtectedWriteMode(unittest.TestCase):
    def paths(self, directory: str) -> tuple[Path, Path]:
        allocation = Path(directory) / "allocation.json"
        plan = Path(directory) / "SPRINT_PLAN.md"
        allocation.write_text('{"stories": []}\n', encoding="utf-8")
        return allocation, plan

    def invoke(self, argv: list[str], allocation: Path,
               plan: Path) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = gen_sprint_plan.main(
                argv, allocation_path=allocation, plan_path=plan)
        return status, output.getvalue()

    def test_bare_write_refuses_an_existing_plan_without_changing_it(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            original = b"# Hand-curated sprint plan\n\nKeep this paragraph.\n"
            plan.write_bytes(original)

            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, output = self.invoke([], allocation, plan)

            self.assertEqual(status, 1)
            self.assertEqual(plan.read_bytes(), original)
            self.assertIn("refuses to overwrite", output)
            self.assertIn("--force", output)
            self.assertIn("--check", output)

    def test_force_is_the_explicit_replacement_path(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            plan.write_text("hand curated\n", encoding="utf-8")
            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, _ = self.invoke(["--force"], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_text(encoding="utf-8"), "generated\n")

    def test_bare_write_still_bootstraps_an_absent_plan(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, _ = self.invoke([], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_text(encoding="utf-8"), "generated\n")

    def test_check_reads_explicit_paths_without_rewriting_the_plan(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            allocation.write_bytes(
                (ROOT / "docs/sprints/allocation.json").read_bytes())
            original = (ROOT / "docs/sprints/SPRINT_PLAN.md").read_bytes()
            plan.write_bytes(original)

            status, _ = self.invoke(["--check"], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
