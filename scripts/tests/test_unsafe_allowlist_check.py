"""The index, rather than a changed-name subset or the worktree, is audited."""
import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location(
    "unsafe_guard", Path(__file__).resolve().parents[1] / "unsafe_allowlist_check.py")
guard = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(guard)


class IndexAudit(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        self.root_patch = patch.object(guard, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)
        vendor_patch = patch.object(guard, "VENDORED_UNSAFE", {"test-package": 0})
        vendor_patch.start()
        self.addCleanup(vendor_patch.stop)
        self.write("vendor/test-package/src/lib.rs", "pub fn dependency() {}\n")
        self.write("crates/example/src/lib.rs", "pub fn application() {}\n")
        self.stage()

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def stage(self):
        subprocess.run(["git", "add", "."], cwd=self.root, check=True)

    def run_guard(self):
        output = io.StringIO()
        with patch("sys.argv", ["unsafe_allowlist_check.py", "--staged"]), contextlib.redirect_stdout(output):
            result = guard.main()
        return result, output.getvalue()

    def test_unchanged_vendor_is_part_of_the_index(self):
        subprocess.run(["git", "-c", "user.name=Test", "-c", "user.email=test@example.invalid",
                        "commit", "-qm", "base"], cwd=self.root, check=True)
        self.write("crates/example/src/lib.rs", "pub fn changed() {}\n")
        self.stage()
        self.assertEqual(self.run_guard()[0], 0)

    def test_staged_unsafe_cannot_be_hidden_by_a_clean_worktree(self):
        self.write("crates/example/src/lib.rs", "pub unsafe fn planted() {}\n")
        self.stage()
        self.write("crates/example/src/lib.rs", "pub fn clean() {}\n")
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:1", output)

    def test_unstaged_unsafe_is_not_attributed_to_the_index(self):
        self.write("crates/example/src/lib.rs", "pub unsafe fn local() {}\n")
        self.assertEqual(self.run_guard()[0], 0)

    def test_unreadable_index_blob_is_not_a_pass(self):
        run = guard.subprocess.run
        def read_or_refuse(argv, **kwargs):
            if argv[:2] == ["git", "show"]:
                raise subprocess.CalledProcessError(1, argv)
            return run(argv, **kwargs)
        with patch.object(guard.subprocess, "run", side_effect=read_or_refuse):
            code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("source is unreadable", output)

    def test_missing_git_inventory_is_not_a_pass(self):
        with patch.object(guard.subprocess, "run", side_effect=FileNotFoundError):
            code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("inventory", output)


if __name__ == "__main__":
    unittest.main()
