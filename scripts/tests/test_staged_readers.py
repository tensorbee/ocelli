"""Staged guards read the index, including a path that contains a space.

A missing git listing is a failure. A reverted or deleted worktree file does
not hide the blob that the commit would record.
"""
import contextlib
import importlib.util
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
BLOCKED = "gro" + "k"
EM = "\u2014"


def load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


CONTENT = load("content_guard", "staged_content_check.py")
PROVENANCE = load("provenance_guard", "source_provenance_check.py")
PROSE = load("prose_guard", "prose_check.py")


class StagedReaders(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        for module in (CONTENT, PROVENANCE, PROSE):
            patched = patch.object(module, "ROOT", self.root)
            patched.start()
            self.addCleanup(patched.stop)

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(content, bytes):
            target.write_bytes(content)
        else:
            target.write_text(content)

    def stage(self):
        subprocess.run(["git", "add", "-A"], cwd=self.root, check=True)

    def commit_tree(self):
        subprocess.run(
            ["git", "config", "user.email", "dev@example.com"],
            cwd=self.root,
            check=True,
        )
        subprocess.run(
            ["git", "config", "user.name", "Dev"],
            cwd=self.root,
            check=True,
        )
        self.stage()
        subprocess.run(
            ["git", "commit", "-q", "--no-verify", "-m", "base"],
            cwd=self.root,
            check=True,
        )

    def replace_symlink(self, path, content):
        link = self.root / path
        link.unlink()
        self.write(path, content)
        self.stage()

    def invoke(self, module, argv):
        output = io.StringIO()
        with patch("sys.argv", [module.__name__, *argv]), contextlib.redirect_stdout(output):
            result = module.main()
        return result, output.getvalue()

    def test_a_spaced_dicom_name_is_one_path(self):
        self.write("my file.dcm", b"not a real header\n")
        self.stage()
        code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("my file.dcm", output)

    def test_a_reverted_worktree_cannot_hide_staged_dicom(self):
        self.write("anon001", b"\x00" * 128 + b"DICM" + b"payload")
        self.stage()
        self.write("anon001", b"clean text\n")
        code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("anon001", output)
        self.assertIn("this is DICOM", output)

    def test_a_deleted_worktree_file_cannot_hide_staged_dicom(self):
        self.write("sample.dcm", b"not really\n")
        self.stage()
        (self.root / "sample.dcm").unlink()
        code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("sample.dcm", output)

    def test_tracked_mode_reads_the_worktree_copy(self):
        self.write("anon001", b"clean text\n")
        self.stage()
        self.write("anon001", b"\x00" * 128 + b"DICM" + b"payload")
        code, output = self.invoke(CONTENT, ["--tracked"])
        self.assertEqual(code, 1)
        self.assertIn("this is DICOM", output)

    def test_staged_mode_ignores_an_unstaged_worktree_dicom(self):
        self.write("anon001", b"clean text\n")
        self.stage()
        self.write("anon001", b"\x00" * 128 + b"DICM" + b"payload")
        self.assertEqual(self.invoke(CONTENT, [])[0], 0)

    def test_a_missing_content_inventory_is_not_a_pass(self):
        with patch.object(CONTENT.subprocess, "run", side_effect=FileNotFoundError):
            code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("inventory is unavailable", output)

    def test_an_unreadable_content_blob_is_not_a_pass(self):
        self.write("probe.bin", b"x")
        self.stage()
        real = CONTENT.subprocess.run

        def refuse_show(argv, **kwargs):
            if argv[:2] == ["git", "show"]:
                raise subprocess.CalledProcessError(1, argv)
            return real(argv, **kwargs)

        with patch.object(CONTENT.subprocess, "run", side_effect=refuse_show):
            code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("unreadable", output)

    def test_a_reverted_worktree_cannot_hide_a_staged_mention(self):
        self.write("docs/lld/note.md", f"Use {BLOCKED} for the pixels.\n")
        self.stage()
        self.write("docs/lld/note.md", "A clean note.\n")
        code, output = self.invoke(PROVENANCE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/note.md", output)
        self.assertIn("out of bounds", output)

    def test_a_spaced_provenance_path_is_one_file(self):
        self.write("docs/lld/my note.md", f"Use {BLOCKED} for the pixels.\n")
        self.stage()
        code, output = self.invoke(PROVENANCE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/my note.md", output)

    def test_a_missing_provenance_inventory_is_not_a_pass(self):
        with patch.object(PROVENANCE.subprocess, "run", side_effect=FileNotFoundError):
            code, output = self.invoke(PROVENANCE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("inventory is unavailable", output)
        self.assertNotIn("clean over 0", output)

    def test_a_reverted_worktree_cannot_hide_staged_prose(self):
        self.write("docs/lld/voice.md", f"The value is one{EM} two.\n")
        self.stage()
        self.write("docs/lld/voice.md", "The value is one, two.\n")
        code, output = self.invoke(PROSE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/voice.md", output)
        self.assertIn("em-dash", output)

    def test_a_spaced_prose_path_is_one_file(self):
        self.write("docs/lld/my voice.md", "The value is one; two.\n")
        self.stage()
        code, output = self.invoke(PROSE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/my voice.md", output)

    def test_a_missing_prose_inventory_is_not_a_pass(self):
        with patch.object(PROSE.subprocess, "run", side_effect=FileNotFoundError):
            code, output = self.invoke(PROSE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("inventory is unavailable", output)
        self.assertNotIn("clean over 0", output)

    def test_an_unreadable_prose_blob_is_not_a_pass(self):
        self.write("docs/lld/voice.md", "A clean sentence.\n")
        self.stage()
        real = PROSE.subprocess.run

        def refuse_show(argv, **kwargs):
            if argv[:2] == ["git", "show"]:
                raise subprocess.CalledProcessError(1, argv)
            return real(argv, **kwargs)

        with patch.object(PROSE.subprocess, "run", side_effect=refuse_show):
            code, output = self.invoke(PROSE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("unreadable", output)

    def test_a_type_change_cannot_hide_staged_dicom(self):
        (self.root / "anon001").symlink_to("elsewhere")
        self.commit_tree()
        self.replace_symlink("anon001", b"\x00" * 128 + b"DICM" + b"payload")
        code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("anon001", output)
        self.assertIn("this is DICOM", output)

    def test_a_stage_prefix_cannot_hide_staged_dicom(self):
        self.write("anon001", b"clean text\n")
        self.write("0:anon001", b"\x00" * 128 + b"DICM" + b"payload")
        self.stage()
        code, output = self.invoke(CONTENT, [])
        self.assertEqual(code, 1)
        self.assertIn("0:anon001", output)
        self.assertIn("this is DICOM", output)

    def test_a_type_change_cannot_hide_a_staged_mention(self):
        (self.root / "docs" / "lld").mkdir(parents=True)
        (self.root / "docs" / "lld" / "note.md").symlink_to("elsewhere")
        self.commit_tree()
        self.replace_symlink(
            "docs/lld/note.md",
            f"Use {BLOCKED} for the pixels.\n",
        )
        code, output = self.invoke(PROVENANCE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/note.md", output)
        self.assertIn("out of bounds", output)

    def test_a_stage_prefix_cannot_hide_a_staged_mention(self):
        self.write("note.md", "A clean note.\n")
        self.write("0:note.md", f"Use {BLOCKED} for the pixels.\n")
        self.stage()
        code, output = self.invoke(PROVENANCE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("0:note.md", output)
        self.assertIn("out of bounds", output)

    def test_a_type_change_cannot_hide_staged_prose(self):
        (self.root / "docs" / "lld").mkdir(parents=True)
        (self.root / "docs" / "lld" / "voice.md").symlink_to("elsewhere")
        self.commit_tree()
        self.replace_symlink("docs/lld/voice.md", f"The value is one{EM} two.\n")
        code, output = self.invoke(PROSE, ["--staged"])
        self.assertEqual(code, 1)
        self.assertIn("docs/lld/voice.md", output)
        self.assertIn("em-dash", output)


if __name__ == "__main__":
    unittest.main()
