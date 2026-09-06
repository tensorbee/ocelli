#!/usr/bin/env python3
"""Adversarial tests for the marked skill example checker. F-X015."""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import skill_examples_check as checker  # noqa: E402


def stdout_document(example_id: str = "prints-ok",
                    code: str = 'print("ok")', output: str = "ok") -> str:
    return (
        f"<!-- ocelli-example: id={example_id} interpreter=python3 "
        "mode=stdout -->\n"
        f"```python\n{code}\n```\n```text\n{output}\n```\n"
        "<!-- /ocelli-example -->\n"
    )


def assert_document(example_id: str = "asserts-ok",
                    code: str = "assert 2 + 2 == 4") -> str:
    return (
        f"<!-- ocelli-example: id={example_id} interpreter=python3 "
        "mode=assert -->\n"
        f"```python\n{code}\n```\n<!-- /ocelli-example -->\n"
    )


class SkillExampleTest(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.root = Path(self.scratch.name)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def write(self, name: str, text: str) -> Path:
        path = self.root / name
        path.write_text(text, encoding="utf-8")
        return path

    def test_stdout_and_assert_modes_parse(self) -> None:
        path = self.write("SKILL.md", stdout_document() + assert_document())
        examples = checker.parse_all([path])
        self.assertEqual([item.mode for item in examples], ["stdout", "assert"])
        self.assertEqual(examples[0].expected_stdout, "ok\n")
        self.assertIsNone(examples[1].expected_stdout)

    def test_marker_must_start_at_column_zero(self) -> None:
        path = self.write("SKILL.md", " " + stdout_document())
        with self.assertRaisesRegex(checker.ExampleError, "column-zero"):
            checker.parse_all([path])

    def test_unknown_interpreter_is_refused(self) -> None:
        text = stdout_document().replace("interpreter=python3", "interpreter=bash")
        path = self.write("SKILL.md", text)
        with self.assertRaisesRegex(checker.ExampleError, "exact column-zero"):
            checker.parse_all([path])

    def test_unknown_mode_is_refused(self) -> None:
        text = stdout_document().replace("mode=stdout", "mode=print")
        path = self.write("SKILL.md", text)
        with self.assertRaisesRegex(checker.ExampleError, "exact column-zero"):
            checker.parse_all([path])

    def test_marker_text_inside_an_unmarked_fence_is_ignored(self) -> None:
        inert = (
            "```text\n"
            "<!-- ocelli-example: id=not-live interpreter=bash mode=no -->\n"
            "<!-- /ocelli-example -->\n"
            "```\n"
        )
        path = self.write("SKILL.md", inert + assert_document())
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["asserts-ok"])

    def test_marker_text_inside_a_blockquoted_fence_is_ignored(self) -> None:
        inert = (
            "> ```text\n"
            "> <!-- ocelli-example: id=not-live interpreter=bash mode=no -->\n"
            "> <!-- /ocelli-example -->\n"
            "> ```\n"
        )
        path = self.write("SKILL.md", inert + assert_document())
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["asserts-ok"])

    def test_marker_text_inside_a_list_nested_fence_is_ignored(self) -> None:
        inert = (
            "- Quoted protocol\n"
            "    ```text\n"
            "    <!-- ocelli-example: id=not-live interpreter=bash mode=no -->\n"
            "    <!-- /ocelli-example -->\n"
            "    ```\n"
        )
        path = self.write("SKILL.md", inert + assert_document())
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["asserts-ok"])

    def test_marker_text_inside_an_ordered_list_fence_is_ignored(self) -> None:
        inert = (
            "10. Quoted protocol\n"
            "    ```text\n"
            "    <!-- ocelli-example: id=not-live interpreter=bash mode=no -->\n"
            "    <!-- /ocelli-example -->\n"
            "    ```\n"
        )
        path = self.write("SKILL.md", inert + assert_document())
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["asserts-ok"])

    def test_column_zero_marker_after_a_blockquote_is_live(self) -> None:
        quoted = "> ```text\n> inert documentation\n"
        path = self.write("SKILL.md", quoted + assert_document())
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["asserts-ok"])

    def test_column_zero_marker_after_a_list_is_live(self) -> None:
        for indent in (2, 3, 4):
            with self.subTest(indent=indent):
                spaces = " " * indent
                nested = (f"- Documentation\n{spaces}```text\n"
                          f"{spaces}inert documentation\n")
                path = self.write(f"list-{indent}.md",
                                  nested + assert_document())
                self.assertEqual(
                    [item.id for item in checker.parse_all([path])],
                    ["asserts-ok"],
                )

    def test_backtick_in_backtick_fence_info_cannot_hide_example(self) -> None:
        document = (
            "````text`\n"
            + assert_document("must-run", "assert False")
            + assert_document("visible-pass")
        )
        path = self.write("SKILL.md", document)
        self.assertEqual([item.id for item in checker.parse_all([path])],
                         ["must-run", "visible-pass"])
        with self.assertRaisesRegex(checker.ExampleError, "exited 1"):
            checker.check_paths([path])

    def test_backtick_in_tilde_fence_info_remains_inert(self) -> None:
        document = (
            "~~~~text`\n"
            + assert_document("inert-failure", "assert False")
            + "~~~~\n"
            + assert_document("visible-pass")
        )
        path = self.write("SKILL.md", document)
        examples = checker.check_paths([path])
        self.assertEqual([item.id for item in examples], ["visible-pass"])

    def test_unmatched_end_marker_is_refused(self) -> None:
        path = self.write("SKILL.md", checker.END + "\n")
        with self.assertRaisesRegex(checker.ExampleError, "has no start"):
            checker.parse_all([path])

    def test_missing_end_marker_is_refused(self) -> None:
        path = self.write("SKILL.md", stdout_document().replace(checker.END, ""))
        with self.assertRaisesRegex(checker.ExampleError, "must end"):
            checker.parse_all([path])

    def test_missing_output_fence_is_refused(self) -> None:
        text = stdout_document().replace("```text\nok\n```\n", "")
        path = self.write("SKILL.md", text)
        with self.assertRaisesRegex(checker.ExampleError, "following text fence"):
            checker.parse_all([path])

    def test_empty_code_is_refused(self) -> None:
        path = self.write("SKILL.md", assert_document(code=""))
        with self.assertRaisesRegex(checker.ExampleError, "code is empty"):
            checker.parse_all([path])

    def test_duplicate_id_across_skills_is_refused(self) -> None:
        first = self.write("first.md", assert_document("same-id"))
        second = self.write("second.md", stdout_document("same-id"))
        with self.assertRaisesRegex(checker.ExampleError, "duplicate example id"):
            checker.parse_all([first, second])

    def test_empty_selection_is_refused(self) -> None:
        with self.assertRaisesRegex(checker.ExampleError, "no marked"):
            checker.parse_all([])

    def test_invalid_utf8_is_refused(self) -> None:
        path = self.root / "SKILL.md"
        path.write_bytes(b"\xff\xfe")
        with self.assertRaisesRegex(checker.ExampleError, "invalid UTF-8"):
            checker.parse_all([path])

    def test_child_skill_symlink_may_not_escape(self) -> None:
        repository = self.root / "repository"
        skills = repository / ".claude" / "skills"
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "SKILL.md").write_text(assert_document(), encoding="utf-8")
        skills.mkdir(parents=True)
        (skills / "escaped").symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(checker.ExampleError, "symlink escapes"):
            checker.canonical_skill_paths(skills, repository)

    def test_canonical_skills_root_may_not_be_a_symlink(self) -> None:
        repository = self.root / "repository"
        canonical_parent = repository / ".claude"
        outside = self.root / "outside"
        (outside / "skill").mkdir(parents=True)
        (outside / "skill" / "SKILL.md").write_text(
            assert_document(), encoding="utf-8")
        canonical_parent.mkdir(parents=True)
        skills = canonical_parent / "skills"
        skills.symlink_to(outside, target_is_directory=True)
        with self.assertRaisesRegex(checker.ExampleError, "root is a symlink"):
            checker.canonical_skill_paths(skills, repository)

    def test_noncanonical_skills_root_is_refused(self) -> None:
        repository = self.root / "repository"
        repository.mkdir()
        other = self.root / "other-skills"
        other.mkdir()
        with self.assertRaisesRegex(checker.ExampleError,
                                    "not this repository's skills root"):
            checker.canonical_skill_paths(other, repository)

    def test_missing_canonical_skills_root_is_refused(self) -> None:
        repository = self.root / "repository"
        repository.mkdir()
        skills = repository / ".claude" / "skills"
        with self.assertRaisesRegex(checker.ExampleError,
                                    "cannot resolve canonical skills root"):
            checker.canonical_skill_paths(skills, repository)

    def test_symlinked_parent_may_not_move_the_root_outside_repository(
            self) -> None:
        repository = self.root / "repository"
        outside = self.root / "outside"
        (outside / "skills").mkdir(parents=True)
        repository.mkdir()
        (repository / ".claude").symlink_to(outside, target_is_directory=True)
        skills = repository / ".claude" / "skills"
        with self.assertRaisesRegex(checker.ExampleError,
                                    "skills root escapes repository"):
            checker.canonical_skill_paths(skills, repository)

    def test_every_file_parses_before_any_example_runs(self) -> None:
        valid = self.write("a.md", assert_document())
        malformed = self.write("z.md", " <!-- ocelli-example -->\n")
        ran: list[str] = []
        with self.assertRaises(checker.ExampleError):
            checker.check_paths([valid, malformed], lambda item: ran.append(item.id))
        self.assertEqual(ran, [])

    def test_stdout_difference_is_refused(self) -> None:
        example = checker.parse_all([
            self.write("SKILL.md", stdout_document(output="different"))
        ])[0]
        with self.assertRaisesRegex(checker.ExampleError, "stdout differs"):
            checker.execute(example)

    def test_nonzero_example_is_refused_with_bounded_diagnostics(self) -> None:
        code = 'raise RuntimeError("x" * 10000)'
        example = checker.parse_all([
            self.write("SKILL.md", assert_document(code=code))
        ])[0]
        with self.assertRaises(checker.ExampleError) as caught:
            checker.execute(example)
        message = str(caught.exception)
        self.assertIn("exited 1", message)
        self.assertIn("characters omitted", message)
        self.assertLess(len(message), checker.DIAGNOSTIC_LIMIT + 500)

    def test_timeout_is_refused(self) -> None:
        example = checker.parse_all([
            self.write("SKILL.md", assert_document(code="while True: pass"))
        ])[0]
        with mock.patch.object(checker, "TIMEOUT_SECONDS", 0.05):
            with self.assertRaisesRegex(checker.ExampleError, "timed out"):
                checker.execute(example)

    def test_each_example_gets_an_isolated_cwd_and_environment(self) -> None:
        code = (
            "import os\nfrom pathlib import Path\n"
            "assert Path.cwd().resolve() == Path(os.environ['HOME']).resolve()\n"
            "assert Path.cwd().resolve() == Path(os.environ['TMPDIR']).resolve()\n"
            "assert 'OCELLI_SKILL_EXAMPLE_SECRET' not in os.environ\n"
        )
        example = checker.parse_all([
            self.write("SKILL.md", assert_document(code=code))
        ])[0]
        with mock.patch.dict(os.environ,
                             {"OCELLI_SKILL_EXAMPLE_SECRET": "must-not-pass"}):
            checker.execute(example)

    def test_execution_uses_the_fixed_argv_stdin_and_no_shell(self) -> None:
        example = checker.parse_all([
            self.write("SKILL.md", assert_document())
        ])[0]
        completed = mock.Mock(returncode=0, stdout=b"", stderr=b"")
        with mock.patch.object(checker.subprocess, "run",
                               return_value=completed) as run:
            checker.execute(example)
        argv = run.call_args.args[0]
        options = run.call_args.kwargs
        self.assertEqual(argv, ["python3", "-I", "-B", "-"])
        self.assertEqual(options["input"], example.code.encode("utf-8"))
        self.assertNotIn("shell", options)
        self.assertEqual(set(options["env"]),
                         {"HOME", "LANG", "LC_ALL", "PATH", "TMPDIR"})

    def test_assert_mode_must_not_hide_stdout(self) -> None:
        example = checker.parse_all([
            self.write("SKILL.md", assert_document(code='print("surprise")'))
        ])[0]
        with self.assertRaisesRegex(checker.ExampleError,
                                    "assert mode wrote stdout"):
            checker.execute(example)

    def test_cli_refuses_an_arbitrary_selector(self) -> None:
        with mock.patch("sys.stderr"):
            self.assertEqual(checker.main(["--only", "prints-ok"]), 2)


if __name__ == "__main__":
    unittest.main()
