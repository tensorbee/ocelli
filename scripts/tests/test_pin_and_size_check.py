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
            # An unpatched published source file. src/lib.rs is one of the
            # recorded D-31 patches, so its bytes are checked against the
            # patch record instead of the archive inventory.
            (vendor / "src" / "jpeg_2000" / "codestream.rs").write_text("changed\n")
            problems = self.check(vendor)
        self.assertTrue(any("published file changed" in problem
                            for problem in problems), problems)

    def test_a_planted_byte_in_each_patched_source_file_is_refused(self):
        # A patched file is exempt from the archive inventory, so it is held
        # to its own recorded bytes instead, and the refusal names it. F-239
        # records the D-31 colour-decode files, F-243 the D-20 JPEG-LS
        # files, and F-269 the JPEG-LS context test file, and a planted
        # byte in each is refused by name.
        self.assertEqual(sorted(pin_and_size_check.RITK_PATCHED_HASHES), [
            "src/jpeg_2000/image.rs",
            "src/jpeg_2000/mod.rs",
            "src/jpeg_2000/packet/reader.rs",
            "src/jpeg_ls/bitstream.rs",
            "src/jpeg_ls/context/mod.rs",
            "src/jpeg_ls/context/tests.rs",
            "src/jpeg_ls/decoder.rs",
            "src/jpeg_ls/encoder.rs",
            "src/jpeg_ls/scan.rs",
            "src/jpeg_ls/tests/conformance.rs",
            "src/lib.rs",
        ])
        for name in sorted(pin_and_size_check.RITK_PATCHED_HASHES):
            with self.subTest(source=name), \
                 tempfile.TemporaryDirectory() as directory:
                vendor = self.copy_vendor(directory)
                source = vendor / name
                source.write_bytes(source.read_bytes() + b" ")
                problems = self.check(vendor)
            self.assertEqual(problems, [
                f"ritk-codecs {name} has undeclared changes beyond its "
                "recorded patch"
            ])

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


class DicomObjectVendor(unittest.TestCase):
    """F-244 and D-18. The published package, one recorded patch, and no other change."""

    def check(self, vendor: Path = pin_and_size_check.DICOM_OBJECT_VENDOR,
              workspace: Path = ROOT / "Cargo.toml"):
        return pin_and_size_check.check_dicom_object_vendor(
            vendor=vendor, workspace=workspace)

    def copy_vendor(self, directory: str) -> Path:
        copied = Path(directory) / "dicom-object-0.10.0"
        shutil.copytree(pin_and_size_check.DICOM_OBJECT_VENDOR, copied)
        return copied

    def test_the_exact_vendor_inventory_patch_and_provenance_are_accepted(self):
        self.assertEqual(self.check(), [])

    def test_a_planted_byte_in_any_vendored_file_is_refused_by_name(self):
        names = sorted(
            str(path.relative_to(pin_and_size_check.DICOM_OBJECT_VENDOR))
            for path in pin_and_size_check.DICOM_OBJECT_VENDOR.rglob("*")
            if path.is_file())
        self.assertEqual(len(names), pin_and_size_check.DICOM_OBJECT_PUBLISHED_FILES
                         + len(pin_and_size_check.DICOM_OBJECT_ADDITIONS))
        for name in names:
            with self.subTest(file=name), \
                 tempfile.TemporaryDirectory() as directory:
                vendor = self.copy_vendor(directory)
                path = vendor / name
                path.write_bytes(path.read_bytes() + b"\n")
                problems = self.check(vendor)
            self.assertTrue(any(name in problem or "inventory digest" in problem
                                for problem in problems), problems)

    def test_a_second_patch_beside_the_first_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            collector = vendor / "src" / "collector.rs"
            text = collector.read_text()
            self.assertIn("first = false;", text)
            collector.write_text(text.replace(
                "LazyDataToken::ItemStart { len: _ } => { /* no-op */ }",
                "LazyDataToken::ItemStart { len: _ } => { first = true; }"))
            problems = self.check(vendor)
        self.assertTrue(any("src/collector.rs has undeclared changes" in problem
                            for problem in problems), problems)

    def test_removing_the_patch_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            collector = vendor / "src" / "collector.rs"
            collector.write_text(collector.read_text().replace(
                "                    first = false;\n                }\n"
                "                LazyDataToken::ItemStart",
                "                }\n                LazyDataToken::ItemStart"))
            problems = self.check(vendor)
        self.assertTrue(any("src/collector.rs has undeclared changes" in problem
                            for problem in problems), problems)

    def test_coordinated_source_and_inventory_tamper_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            source = vendor / "src" / "mem.rs"
            source.write_bytes(source.read_bytes() + b"// coordinated tamper\n")
            inventory = vendor / "PACKAGE-INVENTORY.sha256"
            replacement = hashlib.sha256(source.read_bytes()).hexdigest()
            inventory.write_text("\n".join(
                f"{replacement}  ./src/mem.rs" if row.endswith("  ./src/mem.rs")
                else row for row in inventory.read_text().splitlines()) + "\n")
            problems = self.check(vendor)
        self.assertTrue(any("inventory digest" in problem for problem in problems),
                        problems)

    def test_an_unrecorded_file_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            vendor = self.copy_vendor(directory)
            (vendor / "src" / "extra.rs").write_text("// extra\n")
            problems = self.check(vendor)
        self.assertTrue(any("carries an unrecorded file: src/extra.rs" in problem
                            for problem in problems), problems)

    def test_registry_wiring_is_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            workspace = Path(directory) / "Cargo.toml"
            workspace.write_text((ROOT / "Cargo.toml").read_text().replace(
                'dicom-object = { version = "=0.10.0", '
                'path = "vendor/dicom-object-0.10.0", ',
                'dicom-object = { version = "0.10", '))
            problems = self.check(workspace=workspace)
        self.assertIn(
            "workspace dicom-object dependency is not the exact path and version",
            problems)


class DecodeExports(unittest.TestCase):
    def bindings(self, directory: str, text: str) -> Path:
        package = Path(directory) / "pkg-decode"
        package.mkdir()
        (package / "ocelli_wasm.d.ts").write_text(text)
        return package

    def test_the_panic_surface_is_accepted(self):
        text = "\n".join(
            f"export function {name}(): void;"
            for name in pin_and_size_check.DECODE_EXPORTS
        ) + "\n"
        with tempfile.TemporaryDirectory() as directory:
            package = self.bindings(directory, text)
            self.assertEqual(
                pin_and_size_check.check_decode_exports(package), [])

    def test_a_canvas_name_is_refused(self):
        text = "\n".join(
            f"export function {name}(): void;"
            for name in pin_and_size_check.DECODE_EXPORTS
        ) + "\nexport function attachOffscreenCanvas(): void;\n"
        with tempfile.TemporaryDirectory() as directory:
            package = self.bindings(directory, text)
            problems = pin_and_size_check.check_decode_exports(package)
        self.assertTrue(
            any("attachOffscreenCanvas" in problem for problem in problems),
            problems)

    def test_a_missing_panic_export_is_refused(self):
        names = [
            name for name in pin_and_size_check.DECODE_EXPORTS
            if name != "ocelli_version"
        ]
        text = "\n".join(f"export function {name}(): void;" for name in names)
        with tempfile.TemporaryDirectory() as directory:
            package = self.bindings(directory, text)
            problems = pin_and_size_check.check_decode_exports(package)
        self.assertTrue(
            any("ocelli_version" in problem for problem in problems),
            problems)


if __name__ == "__main__":
    unittest.main()
