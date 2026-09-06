#!/usr/bin/env python3
"""Tests for the two readers that stopped being regexes. F-X009, S03 pass 11.

    python3 -m unittest discover -s scripts/tests -p 'test_guard_readers.py'

**Every case here comes from the GRAMMAR, not from the reader.** That is the
whole point of the change these tests arrived with. Ten review passes patched a
regex over TOML and a regex over POSIX shell one spelling at a time, and each
pass closed the spelling that pass happened to think of, because the tests were
written from what the regex matched. So:

- the shell cases are checked against `bash` itself where a behaviour is
  disputable, which is the only authority this repository has for what a shell
  does, and
- the TOML cases are the five spellings TOML 1.0 gives one table row, each
  measured under the pinned 1.97.1 toolchain to be the same row to cargo.

Needs nothing but the standard library. The bash cross-checks skip when there
is no bash, and a skip is not a pass: `bin/ocelli.sh` is a bash script, so a
machine that runs the gate has one.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import ci_floor_check  # noqa: E402
import lint_policy_check  # noqa: E402

BASH = shutil.which("bash")


def bash_says(script: str) -> str:
    """What bash prints for `script`, as the authority on shell grammar."""
    done = subprocess.run([BASH, "-c", script], capture_output=True, text=True)
    if done.returncode != 0:
        raise AssertionError(
            f"bash refused the probe script itself ({done.returncode}): "
            f"{done.stderr.strip()}. A test whose input the shell will not "
            f"accept proves nothing about the shell.")
    return done.stdout


class WhereAShellCommentBegins(unittest.TestCase):
    """`_strip_shell_comments`, against bash rather than against itself."""

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_hash_after_a_substitution_is_not_a_comment(self) -> None:
        """The eleventh pass's regression, stated as bash states it.

        `)` begins a word, so a `#` after an operator `)` opens a comment, and
        the `)` that closes a `$( ... )` is not an operator. bash prints both
        lines here, so the `#no` is part of the word.
        """
        self.assertEqual(
            bash_says("echo A$(printf x)#no && echo RAN_SECOND"),
            "Ax#no\nRAN_SECOND\n")
        self.assertEqual(
            ci_floor_check._strip_shell_comments("echo A$(printf x)#no && x"),
            "echo A$(printf x)#no && x")

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_hash_after_an_operator_is_a_comment(self) -> None:
        """The direction the fix must not undo. A `case` pattern ends with a
        real operator `)` and a `#` after one opens a comment, which is why
        `)` stays in `COMMENT_WORD_START`."""
        self.assertEqual(bash_says("true;#;;\necho AFTER"), "AFTER\n")
        self.assertEqual(
            ci_floor_check._strip_shell_comments("true;#;;\necho AFTER"),
            "true;\necho AFTER")

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_hash_after_a_closing_quote_is_not_a_comment(self) -> None:
        self.assertEqual(bash_says('echo "a"#b'), "a#b\n")
        self.assertEqual(ci_floor_check._strip_shell_comments('echo "a"#b'),
                         'echo "a"#b')

    def test_a_hash_inside_a_span_is_not_a_comment(self) -> None:
        """Quote, backtick, `${ }` and a here-document body, one rule each."""
        for text in ('echo "a # b" c',
                     "echo `printf '# b'` c",
                     "echo ${x#y} c",
                     "cat <<EOF\n# kept\nEOF\nc"):
            with self.subTest(text):
                self.assertEqual(ci_floor_check._strip_shell_comments(text),
                                 text)

    def test_an_unclosed_span_keeps_its_text(self) -> None:
        """A span that never closes must not delete the rest of the input.

        The refusal belongs to `_arm_end`, which can say what the loss would
        have cost. Dropping the text here would take every later arm with it
        in silence, which is the failure shape this whole file is about.
        """
        text = "a && echo 'never closed\nb ;;"
        self.assertEqual(ci_floor_check._strip_shell_comments(text), text)
        end, reason = ci_floor_check._arm_end(text, 0)
        self.assertEqual(end, -1)
        self.assertIn("opened and never closed", reason)


class WhereAnArmEnds(unittest.TestCase):
    def test_a_terminator_inside_a_substitution_does_not_end_an_arm(self
                                                                   ) -> None:
        text = "x $(printf 'a ;; b') && z ;; tail"
        end, reason = ci_floor_check._arm_end(text, 0)
        self.assertEqual(reason, "")
        self.assertEqual(text[end:], ";; tail")

    def test_a_case_pattern_inside_a_substitution_closes_it_early(self
                                                                 ) -> None:
        """The declared limit, asserted where it is rather than assumed away.

        `$(case y in *) ... esac)` has an unbalanced `)` by this scanner's
        arithmetic, because bash's own `case` grammar is not modelled here.
        The span closes at the pattern's `)` and the arm then ends at the
        inner `;;`. That is FAIL-CLOSED and not a hole: the truncated body
        still carries `$(case `, and `NESTED_CASE` refuses a nested `case` in
        an arm, which `ci-floor.nested-case-in-a-backtick` and its siblings
        watch. This test exists so that the day someone teaches the scanner
        `case`, the refusal that was carrying the weight is visible.
        """
        text = "x $(case y in *) : ;; esac) && z ;; tail"
        end, reason = ci_floor_check._arm_end(text, 0)
        self.assertEqual(reason, "")
        self.assertEqual(text[end:], ";; esac) && z ;; tail")
        self.assertIsNotNone(ci_floor_check.NESTED_CASE.search(text[:end]))

    def test_a_nested_substitution_closes_at_the_outer_paren(self) -> None:
        text = "x $(printf $(printf y)) ;; tail"
        end, _ = ci_floor_check._arm_end(text, 0)
        self.assertEqual(text[end:], ";; tail")

    def test_a_statement_split_keeps_a_substitution_whole(self) -> None:
        self.assertEqual(
            ci_floor_check._split_statements("test -n $(printf x) && cargo z"),
            ["test -n $(printf x) ", " cargo z"])


class TheGatesArrayHasOneReader(unittest.TestCase):
    """The Python reader has to agree with bash, not with another regex."""

    RUNNER = ROOT / "bin" / "ocelli.sh"

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_the_entry_count_matches_bash(self) -> None:
        """`bin/ocelli.sh` is the authority on its own array.

        This is the check that was missing. Two Python copies of one row regex
        agreed with each other and not with `IFS='|' read -r name gpu desc`,
        which imposes no character class on a gate name, and a gate named
        `prose2` was therefore counted by bash and not by either of them.
        """
        counted = bash_says(
            f'source {self.RUNNER} >/dev/null 2>&1 || true;'
            f' printf "%s\\n" "${{#GATES[@]}}"')
        self.assertEqual(
            len(ci_floor_check.gate_entries(self.RUNNER.read_text())),
            int(counted.strip()))

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_the_names_match_bash(self) -> None:
        listed = bash_says(
            f'source {self.RUNNER} >/dev/null 2>&1 || true;'
            f' for e in "${{GATES[@]}}"; do'
            f' IFS="|" read -r name gpu desc <<<"$e";'
            f' printf "%s\\n" "$name"; done')
        self.assertEqual(
            ci_floor_check.declared_gates(self.RUNNER.read_text()),
            listed.split())

    def test_an_entry_outside_the_name_class_is_refused(self) -> None:
        """Refused, and not dropped. Dropping it was the defect."""
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", 'GATES=(\n  "prose2|no|a second pass"\n', 1)
        self.assertIn("prose2", ci_floor_check.declared_gates(runner))
        self.assertEqual(ci_floor_check.gate_row_problems(runner), [])

    def test_an_entry_with_two_fields_is_refused(self) -> None:
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", 'GATES=(\n  "half|no"\n', 1)
        problems = ci_floor_check.gate_row_problems(runner)
        self.assertTrue(any("name|needs_gpu|description" in p
                            for p in problems), problems)

    def test_an_unknown_gpu_column_is_refused(self) -> None:
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", 'GATES=(\n  "odd|maybe|a third value"\n', 1)
        problems = ci_floor_check.gate_row_problems(runner)
        self.assertTrue(any("GPU column" in p for p in problems), problems)

    def test_a_comment_and_a_blank_line_are_not_entries(self) -> None:
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", "GATES=(\n  # grouped\n\n", 1)
        original = self.RUNNER.read_text()
        self.assertEqual(ci_floor_check.declared_gates(runner),
                         ci_floor_check.declared_gates(original))
        self.assertEqual(ci_floor_check.gate_row_problems(runner), [])

    def test_a_description_keeps_its_own_pipes(self) -> None:
        """`read -r name gpu desc` puts every later `|` in the description."""
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", 'GATES=(\n  "piped|no|a|b|c"\n', 1)
        rows = dict((name, desc)
                    for name, _, desc in ci_floor_check.gate_rows(runner))
        self.assertEqual(rows["piped"], "a|b|c")


class OneLintTableRowFiveWays(unittest.TestCase):
    """The five TOML spellings of one row, each measured against cargo.

    Every level below is what `cargo clippy --workspace --all-targets --
    -D warnings` was measured to do under the pinned 1.97.1 toolchain on a
    minimal workspace carrying `cast_possible_truncation = "deny"` and one
    `x as i32`, baseline exit 101:

        "cast_possible_truncation" = "deny"          101, still denied
        cast_possible_truncation.level = "deny"      101, still denied
        pedantic = { level = "allow", priority = 1 }   0, group wins
        pedantic.level/.priority                       0, group wins
        "pedantic" = { level = "allow", priority = 1 } 0, group wins
    """

    HEAD = '[workspace.lints.clippy]\ncast_possible_truncation = "deny"\n'

    def levels(self, rows: str) -> dict[str, str]:
        tables, error = lint_policy_check.workspace_lints(self.HEAD + rows)
        self.assertEqual(error, "")
        return lint_policy_check.lint_levels(tables["clippy"])

    def test_a_required_row_is_the_same_row_in_three_spellings(self) -> None:
        self.assertEqual(self.levels(""),
                         {"cast_possible_truncation": "deny"})
        for spelling in ('"cast_possible_truncation" = "deny"',
                         'cast_possible_truncation.level = "deny"'):
            with self.subTest(spelling):
                tables, error = lint_policy_check.workspace_lints(
                    f"[workspace.lints.clippy]\n{spelling}\n")
                self.assertEqual(error, "")
                self.assertEqual(
                    lint_policy_check.lint_levels(tables["clippy"]),
                    {"cast_possible_truncation": "deny"})

    def test_a_group_row_is_seen_in_every_spelling(self) -> None:
        for rows in ('pedantic = { level = "allow", priority = 1 }',
                     'pedantic.level = "allow"\npedantic.priority = 1',
                     '"pedantic" = { level = "allow", priority = 1 }',
                     'pedantic = { level = "allow", priority = 1 } # noise',
                     '\n\npedantic = { level = "allow", priority = 1 }'):
            with self.subTest(rows):
                self.assertEqual(self.levels(rows + "\n")["pedantic"], "allow")

    def test_a_row_whose_level_cannot_be_read_is_weaker_than_allow(self
                                                                   ) -> None:
        """The empty string is below every level in `STRENGTH`, so it is
        refused rather than skipped."""
        levels = self.levels("pedantic = { priority = 1 }\n")
        self.assertEqual(levels["pedantic"], "")
        self.assertLess(lint_policy_check.STRENGTH.get(levels["pedantic"], 0),
                        lint_policy_check.STRENGTH["deny"])

    def test_a_multi_line_inline_table_is_a_parse_error(self) -> None:
        """cargo accepts it and TOML 1.0 does not, MEASURED at cargo exit 0.

        The reader reports the error and `main` refuses the document, which is
        the fail-closed direction. The regex it replaced returned an empty
        table for the same input.
        """
        tables, error = lint_policy_check.workspace_lints(
            self.HEAD + 'pedantic = { level = "allow",\n  priority = 1 }\n')
        self.assertNotEqual(error, "")
        self.assertEqual(tables, {})
        self.assertIsNone(lint_policy_check.workspace_lints_rows(
            self.HEAD + 'pedantic = { level = "allow",\n  priority = 1 }\n'))

    def test_the_recorded_rows_do_not_depend_on_the_spelling(self) -> None:
        """What the declared-constant ratchet records.

        Two documents that are the same document to cargo have to record the
        same value, or the ratchet asks to be re-recorded for a reformat and
        trains the next author to re-record on sight. Two that differ have to
        differ, including in `priority` alone, which is what decides whether a
        group row outranks a named lint.
        """
        one = self.HEAD + 'pedantic = { level = "allow", priority = 1 }\n'
        same = self.HEAD + '"pedantic" = { priority = 1, level = "allow" }\n'
        other = self.HEAD + 'pedantic = { level = "allow", priority = 2 }\n'
        self.assertEqual(lint_policy_check.workspace_lints_rows(one),
                         lint_policy_check.workspace_lints_rows(same))
        self.assertNotEqual(lint_policy_check.workspace_lints_rows(one),
                            lint_policy_check.workspace_lints_rows(other))


class WhichMembersInheritTheTable(unittest.TestCase):
    """`lints.workspace = true`, as a key path rather than as two regexes."""

    def test_both_spellings_inherit(self) -> None:
        for manifest in ('[package]\nname = "a"\n\n[lints]\n'
                         'workspace = true\n',
                         'lints.workspace = true\n\n[package]\nname = "a"\n',
                         '[package]\nname = "a"\n\n[lints]\n'
                         '"workspace" = true  # HLD 27.1\n'):
            with self.subTest(manifest):
                self.assertEqual(
                    lint_policy_check.inherits_workspace_lints(manifest),
                    (True, ""))

    def test_the_key_under_package_does_not_inherit(self) -> None:
        """MEASURED under the pinned 1.97.1 toolchain: cargo prints
        `unused manifest key: package.lints` and clippy exits 0, so refusing
        this spelling is right."""
        inherits, error = lint_policy_check.inherits_workspace_lints(
            '[package]\nname = "a"\nlints.workspace = true\n')
        self.assertEqual((inherits, error), (False, ""))

    def test_a_manifest_that_is_not_toml_reports_the_error(self) -> None:
        inherits, error = lint_policy_check.inherits_workspace_lints(
            "[package]\nname = = \n")
        self.assertFalse(inherits)
        self.assertNotEqual(error, "")


if __name__ == "__main__":  # pragma: no cover
    unittest.main()
