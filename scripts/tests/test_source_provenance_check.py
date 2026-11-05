"""Manifest name extraction for the source-provenance check.

The blocked token is built in the test. The whole word is not written here.
Expected hits come from the prefix rule, not from a run of the checker.
"""

import importlib.util
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BLOCKED = "gro" + "k"


def load():
    spec = importlib.util.spec_from_file_location(
        "provenance_names", ROOT / "source_provenance_check.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


P = load()


class Prefix(unittest.TestCase):
    def test_prefix_and_the_lib_prefix(self) -> None:
        self.assertEqual(P.blocked_in_name(BLOCKED + "j2k"), BLOCKED)
        self.assertEqual(P.blocked_in_name("lib" + BLOCKED), BLOCKED)
        self.assertEqual(P.blocked_in_name("lib" + BLOCKED + "j2k"), BLOCKED)
        self.assertIsNone(P.blocked_in_name("liberty"))
        self.assertIsNone(P.blocked_in_name("a" + BLOCKED))

    def test_final_segment_only(self) -> None:
        self.assertEqual(P.blocked_in_name("@scope/" + BLOCKED + "j2k"), BLOCKED)
        self.assertIsNone(P.blocked_in_name(BLOCKED + "j2k/liberty"))


class Grammars(unittest.TestCase):
    def test_cargo_toml_dependency_key(self) -> None:
        text = "[dependencies]\n" + BLOCKED + 'j2k = "1"\n'
        self.assertIn(BLOCKED + "j2k", P.dependency_names("Cargo.toml", text))
        self.assertTrue(self._depends("Cargo.toml", text))

    def test_workspace_and_target_tables(self) -> None:
        text = (
            "[workspace.dependencies]\n"
            + BLOCKED + ' = "1"\n'
            + "[target.'cfg(unix)'.dependencies]\nliberty = \"1\"\n"
        )
        names = P.dependency_names("Cargo.toml", text)
        self.assertIn(BLOCKED, names)
        self.assertIn("liberty", names)
        self.assertTrue(self._depends("Cargo.toml", text))

    def test_cargo_lock_name(self) -> None:
        text = 'name = "lib' + BLOCKED + '"\n'
        self.assertTrue(self._depends("Cargo.lock", text))

    def test_uv_lock_name(self) -> None:
        text = 'name = "lib' + BLOCKED + 'j2k"\n'
        self.assertTrue(self._depends("uv.lock", text))

    def test_pep508_name_stops_at_a_marker(self) -> None:
        text = (
            "[project]\ndependencies = [\n"
            '  "' + BLOCKED + 'j2k>=1",\n'
            '  "liberty[extra]==1",\n'
            "]\n"
        )
        names = P.dependency_names("pyproject.toml", text)
        self.assertEqual(names, [BLOCKED + "j2k", "liberty[extra]==1"][:1]
                         + ["liberty"])
        self.assertNotIn("liberty[extra]==1", names)
        self.assertTrue(self._depends("pyproject.toml", text))

    def test_package_json_keys_and_bundled_list(self) -> None:
        text = (
            '{"dependencies": {"' + BLOCKED + '": "1"},'
            ' "bundledDependencies": ["liberty"]}'
        )
        names = P.dependency_names("package.json", text)
        self.assertIn(BLOCKED, names)
        self.assertIn("liberty", names)
        self.assertTrue(self._depends("package.json", text))

    def test_package_lock_final_segment(self) -> None:
        text = (
            '{"packages": {'
            '"": {"name": "root"},'
            '"node_modules/lib' + BLOCKED + 'j2k": {"version": "1"}'
            "}}"
        )
        names = P.dependency_names("package-lock.json", text)
        self.assertIn("lib" + BLOCKED + "j2k", names)
        self.assertNotIn("root", names)
        self.assertTrue(self._depends("package-lock.json", text))

    def test_markdown_is_not_a_manifest_hit(self) -> None:
        text = "The name " + BLOCKED + "j2k is not a dependency.\n"
        self.assertEqual(P.check_text("notes.md", text), [])

    def test_agent_trailer_with_a_negation_stays_clean(self) -> None:
        text = (
            "Ocelli-Generated-By: " + BLOCKED + "\n"
            "This names the agent. It is not a read-blocked project.\n"
        )
        self.assertEqual(P.check_text("review.md", text), [])

    def test_agent_trailer_without_a_negation_is_refused(self) -> None:
        text = "Ocelli-Generated-By: " + BLOCKED + "\nThis names the agent.\n"
        problems = P.check_text("review.md", text)
        self.assertTrue(any("out of bounds" in problem for problem in problems))

    def _depends(self, rel: str, text: str) -> bool:
        return any(
            "depends on " + BLOCKED in problem
            for problem in P.check_text(rel, text))


class Tree(unittest.TestCase):
    def test_the_tracked_tree_stays_clean(self) -> None:
        completed = subprocess.run(
            ["python3", "scripts/source_provenance_check.py"],
            cwd=ROOT.parent, capture_output=True, text=True, check=False)
        self.assertEqual(completed.returncode, 0, completed.stdout + completed.stderr)
        self.assertIn("OK: source-provenance policy clean", completed.stdout)
