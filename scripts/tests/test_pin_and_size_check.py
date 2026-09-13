#!/usr/bin/env python3
"""Tests for the licence half of scripts/pin_and_size_check.py."""

from __future__ import annotations

import hashlib
import sys
import shutil
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


class RitkVendor(unittest.TestCase):
    def check(self, vendor: Path = pin_and_size_check.RITK_VENDOR,
              trees: tuple[str, ...] = ("ritk-codecs v0.6.0\nanyhow v1.0.104",)):
        return pin_and_size_check.check_ritk_vendor(
            vendor=vendor,
            workspace=ROOT / "Cargo.toml",
            dependency_trees=trees,
        )

    def copy_vendor(self, directory: str) -> Path:
        copied = Path(directory) / "ritk-codecs-0.6.0"
        shutil.copytree(pin_and_size_check.RITK_VENDOR, copied)
        return copied

    def test_exact_vendor_inventory_provenance_and_no_rayon_are_accepted(self):
        self.assertEqual(self.check(), [])

    def test_a_removed_published_file_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            (vendor / "src" / "lib.rs").unlink()
            problems = self.check(vendor)
        self.assertTrue(any("published file is absent: src/lib.rs" in problem
                            for problem in problems), problems)

    def test_a_non_manifest_source_change_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            (vendor / "src" / "lib.rs").write_text("changed\n")
            problems = self.check(vendor)
        self.assertTrue(any("published file changed: src/lib.rs" in problem
                            for problem in problems), problems)

    def test_coordinated_source_and_inventory_tamper_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            source = vendor / "src" / "lib.rs"
            source.write_bytes(source.read_bytes() + b"// coordinated tamper\n")
            inventory = vendor / "PACKAGE-INVENTORY.sha256"
            rows = inventory.read_text().splitlines()
            replacement = hashlib.sha256(source.read_bytes()).hexdigest()
            inventory.write_text("\n".join(
                f"{replacement}  ./src/lib.rs" if row.endswith("  ./src/lib.rs") else row
                for row in rows
            ) + "\n")
            problems = self.check(vendor)
        self.assertTrue(any("inventory digest" in problem for problem in problems),
                        problems)

    def test_unrelated_edits_to_either_patched_manifest_are_refused(self):
        for manifest_name in ("Cargo.toml", "Cargo.toml.orig"):
            with self.subTest(manifest=manifest_name), \
                 tempfile.TemporaryDirectory() as directory:
                vendor = self.copy_vendor(directory)
                manifest = vendor / manifest_name
                manifest.write_bytes(manifest.read_bytes() + b"\n# unrelated edit\n")
                problems = self.check(vendor)
            self.assertTrue(any(
                manifest_name in problem and "undeclared changes" in problem
                for problem in problems
            ), problems)

    def test_false_append_to_patch_provenance_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            provenance = vendor / "PATCH-PROVENANCE.md"
            provenance.write_bytes(
                provenance.read_bytes() + b"\nThe Rust source was changed locally.\n"
            )
            problems = self.check(vendor)
        self.assertTrue(any(
            "PATCH-PROVENANCE.md" in problem and "undeclared changes" in problem
            for problem in problems
        ), problems)

    def test_rayon_in_either_target_graph_is_refused(self):
        problems = self.check(trees=(
            "ritk-codecs v0.6.0\nrayon v1.11.0",
            "ritk-codecs v0.6.0\nrayon-core v1.13.0",
        ))
        self.assertEqual(sum("Rayon" in problem for problem in problems), 2,
                         problems)

    def test_both_manifest_patches_are_required(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            original = (vendor / "Cargo.toml.orig").read_text()
            (vendor / "Cargo.toml.orig").write_text(original.replace(
                "jpeg-decoder = { workspace = true, default-features = false }",
                "jpeg-decoder = { workspace = true }"))
            problems = self.check(vendor)
        self.assertTrue(any("Cargo.toml.orig" in problem and
                            "default-features" in problem
                            for problem in problems), problems)


if __name__ == "__main__":
    unittest.main()
