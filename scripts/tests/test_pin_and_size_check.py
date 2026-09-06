#!/usr/bin/env python3
"""Tests for the licence half of scripts/pin_and_size_check.py."""

from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import pin_and_size_check  # noqa: E402


class PackageLicences(unittest.TestCase):
    def fixture(self, directory: str) -> tuple[Path, Path]:
        root = Path(directory) / "root"
        package = Path(directory) / "pkg"
        root.mkdir()
        package.mkdir()
        (root / "LICENSE-MIT").write_bytes(b"the MIT grant\n")
        (root / "LICENSE-APACHE").write_bytes(b"the Apache grant\n")
        return root, package

    def test_both_packaged_licences_are_required_and_byte_identical(self):
        with tempfile.TemporaryDirectory() as directory:
            root, package = self.fixture(directory)
            for name in ("LICENSE-MIT", "LICENSE-APACHE"):
                (package / name).write_bytes((root / name).read_bytes())

            self.assertEqual(
                pin_and_size_check.check_package_licences(package, root), [])

    def test_an_absent_packaged_licence_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            root, package = self.fixture(directory)
            (package / "LICENSE-MIT").write_bytes(
                (root / "LICENSE-MIT").read_bytes())

            problems = pin_and_size_check.check_package_licences(package, root)

            self.assertEqual(len(problems), 1, problems)
            self.assertIn("LICENSE-APACHE", problems[0])
            self.assertIn("absent", problems[0])

    def test_an_absent_repository_licence_is_refused_by_name(self):
        with tempfile.TemporaryDirectory() as directory:
            root, package = self.fixture(directory)
            for name in ("LICENSE-MIT", "LICENSE-APACHE"):
                (package / name).write_bytes((root / name).read_bytes())
            (root / "LICENSE-APACHE").unlink()

            problems = pin_and_size_check.check_package_licences(package, root)

            self.assertEqual(len(problems), 1, problems)
            self.assertIn("repository licence LICENSE-APACHE", problems[0])
            self.assertIn("absent", problems[0])

    def test_packaged_licences_must_be_regular_files(self):
        with tempfile.TemporaryDirectory() as directory:
            root, package = self.fixture(directory)
            for name in ("LICENSE-MIT", "LICENSE-APACHE"):
                (package / name).symlink_to(root / name)

            problems = pin_and_size_check.check_package_licences(package, root)

            self.assertEqual(len(problems), 2, problems)
            for name in ("LICENSE-MIT", "LICENSE-APACHE"):
                self.assertTrue(any(name in problem and "regular file" in problem
                                    for problem in problems), problems)

    def test_a_changed_packaged_licence_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            root, package = self.fixture(directory)
            (package / "LICENSE-MIT").write_bytes(b"changed grant\n")
            (package / "LICENSE-APACHE").write_bytes(
                (root / "LICENSE-APACHE").read_bytes())

            problems = pin_and_size_check.check_package_licences(package, root)

            self.assertEqual(len(problems), 1, problems)
            self.assertIn("LICENSE-MIT", problems[0])
            self.assertIn("byte-identical", problems[0])

    def test_with_size_runs_the_package_licence_check(self):
        with mock.patch.object(pin_and_size_check, "check_pins", return_value=[]), \
             mock.patch.object(pin_and_size_check, "check_size", return_value=[]), \
             mock.patch.object(pin_and_size_check, "check_package_licences",
                               return_value=[]) as licences:
            status = pin_and_size_check.main(["--with-size"])

        self.assertEqual(status, 0)
        licences.assert_called_once_with()


if __name__ == "__main__":
    unittest.main()
