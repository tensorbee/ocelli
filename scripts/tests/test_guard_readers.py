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

import re
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


MARKER = re.compile(r"\bM\d\b")


def bash_runs_in_the_arm(body: str) -> set[str]:
    """The markers bash RUNS when `body` is one arm of a `case`.

    **This is the arm reader's oracle, and it is bash rather than a reading of
    POSIX.** The S03 review's twelfth pass asked whether bash can be asked for
    the arm bodies of `run_gate` instead of a hand-written model, and
    `ci_floor_check`'s tokenizer header records why the READER does not ask it:
    `declare -f` needs the file executed, and its output format differs between
    the two bash versions on this machine. Neither objection touches a TEST,
    which runs a SYNTHETIC arm of `echo` markers and reads what bash prints.

    So every case in `WhereAnArmEnds` below states its claim twice: what bash
    really runs, and what the scanner attributes to the arm. A shape where
    those two disagree is the fail-open class this file has been rewritten for
    five passes running.

    A second arm is present and must never run, because an arm that ends early
    runs the NEXT arm's body in this parser's reading and not in bash's.
    """
    script = f"case g in\n  g) {body} ;;\n  g2) echo M9 ;;\nesac\n"
    return set(MARKER.findall(bash_says(script)))


def scanner_keeps_in_the_arm(body: str) -> set[str]:
    """The markers `ci_floor_check`'s arm reader attributes to the same arm.

    The same region the check itself reads, through the same two functions:
    `shell_source` for comments and continuations, `_arm_end` for the
    terminator. Reading it any other way would test a path the guard does not
    take.
    """
    region = f"case g in\n  g) {body} ;;\n  g2) echo M9 ;;\nesac\n"
    text = ci_floor_check.shell_source(region)
    start = text.index("g)") + 2
    end, reason = ci_floor_check._arm_end(text, start)
    if reason:
        raise AssertionError(f"the scanner refused the arm: {reason}")
    return set(MARKER.findall(text[start:end]))


def scanner_runs_in_the_arm(body: str) -> set[str]:
    """The markers the guard's CONSUMERS of the arm resolve to a statement.

    **`scanner_keeps_in_the_arm` above tests where the arm ENDS, and that is
    where this suite stopped until the S03 review's thirteenth pass.** Every
    marker in every shape below sits INSIDE the arm's extent, so the extent
    test stayed green while a consumer one function later merged two statements
    into one and dropped the work in the second. That is exactly what happened:
    `STATEMENT_BREAK` carried `&&` and no `&`, `A & B` came out as one
    statement, and a head that is a `COMMAND_PREFIXES` prefix or a
    `SHELL_NOISE` builtin took `B` out of `unseen_commands` with it. Measured
    at exit 0 with six node suites out of CI.

    The shell reader is one tokenizer and four hand-written productions on top
    of it, and this is the oracle for the one the extent test cannot reach. The
    consumer question is not "what text is in the arm" but "does the reader
    resolve each command to its OWN statement head", because that is the
    question `unseen_commands` and `invoked_gates` both ask. So a marker counts
    here when it is the ONLY marker of a statement whose head, after the same
    policy `unseen_commands` applies, is the command that prints it. Two
    markers in one statement is a merge, and a merge is the defect.

    The policy is the guard's, not a paraphrase: `SHELL_INTRODUCERS` heads are
    stripped and the remainder re-scanned, `command ...` goes through
    `command_builtin_runs`, and what is left is the head. The three fail-opens
    of the sixth, seventh and eighth passes all lived in those two consumers,
    and nothing in this file reached them until now.
    """
    region = f"case g in\n  g) {body} ;;\n  g2) echo M9 ;;\nesac\n"
    text = ci_floor_check.shell_source(region)
    start = text.index("g)") + 2
    end, reason = ci_floor_check._arm_end(text, start)
    if reason:
        raise AssertionError(f"the scanner refused the arm: {reason}")
    resolved: set[str] = set()
    for raw in ci_floor_check._split_statements(text[start:end]):
        statement = re.sub(r"\s+", " ", raw).strip()
        while statement.split(" ", 1)[0] in ci_floor_check.SHELL_INTRODUCERS:
            statement = statement.partition(" ")[2].strip()
        if statement.split(" ", 1)[0] == "command":
            statement = ci_floor_check.command_builtin_runs(statement)
        markers = MARKER.findall(statement)
        if len(markers) == 1 and statement.split(" ", 1)[0] == "echo":
            resolved.add(markers[0])
    return resolved


class WhereAShellCommentBegins(unittest.TestCase):
    """`shell_source`, against bash rather than against itself."""

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
            ci_floor_check.shell_source("echo A$(printf x)#no && x"),
            "echo A$(printf x)#no && x")

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_hash_after_an_operator_is_a_comment(self) -> None:
        """The direction the fix must not undo. A `case` pattern ends with a
        real operator `)` and a `#` after one opens a comment, which is why
        `)` stays in `WORD_BREAK`."""
        self.assertEqual(bash_says("true;#;;\necho AFTER"), "AFTER\n")
        self.assertEqual(
            ci_floor_check.shell_source("true;#;;\necho AFTER"),
            "true;\necho AFTER")

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_hash_after_a_closing_quote_is_not_a_comment(self) -> None:
        self.assertEqual(bash_says('echo "a"#b'), "a#b\n")
        self.assertEqual(ci_floor_check.shell_source('echo "a"#b'),
                         'echo "a"#b')

    def test_a_hash_inside_a_span_is_not_a_comment(self) -> None:
        """Quote, backtick, `${ }` and a here-document body, one rule each."""
        for text in ('echo "a # b" c',
                     "echo `printf '# b'` c",
                     "echo ${x#y} c",
                     "cat <<EOF\n# kept\nEOF\nc"):
            with self.subTest(text):
                self.assertEqual(ci_floor_check.shell_source(text),
                                 text)

    def test_an_unclosed_span_keeps_its_text(self) -> None:
        """A span that never closes must not delete the rest of the input.

        The refusal belongs to `_arm_end`, which can say what the loss would
        have cost. Dropping the text here would take every later arm with it
        in silence, which is the failure shape this whole file is about.
        """
        text = "a && echo 'never closed\nb ;;"
        self.assertEqual(ci_floor_check.shell_source(text), text)
        end, reason = ci_floor_check._arm_end(text, 0)
        self.assertEqual(end, -1)
        self.assertIn("opened and never closed", reason)


class WhatBashRunsInAnArm(unittest.TestCase):
    """Every span-table row and the here-document production, against bash.

    **These are the six mutations the S03 review's twelfth pass found the suite
    could not see, plus the two fail-opens it measured.** Each case runs the
    shape under bash, reads the markers bash printed, and requires the scanner
    to attribute the same set to the arm. A table of shell facts nothing
    re-derives is how the last four passes each shipped one more wrong row.
    """

    def check(self, body: str, expected: set[str]) -> None:
        self.assertEqual(bash_runs_in_the_arm(body), expected,
                         "bash does not do what this case claims")
        self.assertEqual(scanner_keeps_in_the_arm(body), expected)
        # The consumers, and this line is the thirteenth pass's. Every shape
        # above was already inside the arm's extent, so the two assertions
        # before this one agreed while a statement was being lost one function
        # later. See `scanner_runs_in_the_arm`.
        self.assertEqual(scanner_runs_in_the_arm(body), expected)

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_backslash_quoted_heredoc_delimiter(self) -> None:
        """`<<\\EOF`, the third quoting mechanism beside `'` and `"`.

        The first of the twelfth pass's two measured fail-opens. `HEREDOC` was
        `<<(-?)(?!<)[ \\t]*(['\\"]?)([A-Za-z_]\\w*)\\2`, which knows two of the
        three, so the redirection was not recognised, the body was scanned as
        CODE and its `;;` ended the arm. MEASURED in the `prose` arm of a real
        clone with a real `python3 scripts/prose_check.py --probe-extra` after
        it: `bash -n` green, `scripts/ci_floor_check.py` exit 0, that command
        dropped, and bash really runs it.
        """
        self.check("echo M1 &&\n: <<\\EOF\ntrue ;;\nEOF\necho M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_quoted_heredoc_delimiter_holding_a_hyphen(self) -> None:
        """`<<'EOF-1'`, where `\\w*` stops at the hyphen.

        The second fail-open, and it is the one that shows why a character
        class was the wrong shape rather than the wrong class: `\\w*` matched
        `EOF`, the closing `'` then failed to match `-`, and the whole
        redirection went unrecognised. Same measurement as above, exit 0 with
        the command dropped.
        """
        self.check("echo M1 &&\n: <<'EOF-1'\ntrue ;;\nEOF-1\necho M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_an_unquoted_heredoc_delimiter_holding_a_hyphen(self) -> None:
        """`<<EOF-1`, the near miss, which used to refuse for the wrong reason.

        The delimiter is a WORD and a hyphen is in it, so this terminates and
        the arm reads to its own `;;`. The old class stopped at `EOF`, never
        met a terminator, swallowed the rest of the region as one span, and
        then could not even report that: `shell_pieces` cleared `pending`
        before its own `if pending:` test, so the refusal said "the arm reaches
        the end of the region with no `;;` terminator" about an arm whose `;;`
        was right there.
        """
        self.check("echo M1 &&\n: <<EOF-1\ntrue ;;\nEOF-1\necho M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_heredoc_delimiter_ends_at_an_operator(self) -> None:
        """The word ends at a blank, a newline or an operator, not at a class.

        Each of these is a shape bash runs, and the delimiter in each is `EOF`
        with the operator after it belonging to the command line.
        """
        for body in ("echo M1 &&\n: <<EOF; echo M2\ntrue ;;\nEOF\n:",
                     "echo M1 &&\n: <<EOF>/dev/null\ntrue ;;\nEOF\necho M2",
                     "echo M1 &&\n: <<- EOF\n\ttrue ;;\n\tEOF\necho M2"):
            with self.subTest(body):
                self.check(body, {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_comment_ending_in_a_backslash_continues_nothing(self) -> None:
        """A backslash ending a COMMENT line, which bash does not join.

        `arm_bodies` ran `CONTINUATION.sub(" ", region)` BEFORE the tokenizer,
        which is the one thing the tokenizer's own header says no pass may do.
        MEASURED: the pre-pass joined the next line into the comment, the
        planted `python3 scripts/prose_check.py --probe-extra` vanished
        entirely, `arms['prose']` came back holding the `content` gate's
        command, and the check refused while naming two gates neither of which
        was the one edited.
        """
        self.check("echo M1 &&\n# a note ending in a backslash \\\necho M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_continuation_inside_a_command_joins_it(self) -> None:
        """The direction the fix must not undo, and the reason `JOIN` is a
        piece rather than a deletion in the scanner.

        The marker set alone cannot see this: a backslash left in the text
        does not move a `;;`. What it moves is `gate_commands`, whose
        extraction class stops at the backslash, so `-p <suite>` falls off the
        end of every multi-line arm and the arm command CI runs verbatim reads
        as absent. So the join is asserted on the TEXT as well.
        """
        self.check("echo M1 &&\necho \\\nM2", {"M1", "M2"})
        self.assertEqual(
            ci_floor_check.shell_source("python3 x.py \\\n  --flag"),
            "python3 x.py   --flag")

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_single_quote_takes_no_backslash_escape(self) -> None:
        """`Span("'", escapes=False)`, and the suite could not see it flipped.

        MEASURED: `echo 'a\\'` prints `a\\`, so the quote closes at the second
        `'` and the backslash is literal. With `escapes` True the span would
        run past it to the next quote in the region.
        """
        self.assertEqual(bash_says("echo 'a\\'"), "a\\\n")
        self.check("echo M1 && echo 'a\\' && echo M2", {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_double_quote_does_take_one(self) -> None:
        self.assertEqual(bash_says('echo "a\\" ;; b"'), 'a" ;; b\n')
        self.check('echo M1 && echo "a\\" ;; b" && echo M2', {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_backtick_takes_one_too(self) -> None:
        """An escaped backtick does not close a backtick span.

        With `escapes` False the span would end at the first `` \\` ``, the
        rest of the substitution would be read as code and the next backtick
        would open a span that never closes, which refuses. bash runs both
        markers here, so refusing is wrong and the escape is load-bearing.
        """
        self.check("echo M1 && test -n `printf '%s' a\\`:\\`c` && echo M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_backtick_opens_no_span_inside_itself(self) -> None:
        """**A fail-open in the span table, and the twelfth pass's mutation
        list is what found it.**

        The backtick's `opens` was `True`, and flipping it to `False` left all
        24 tests green. `False` is bash's answer. MEASURED:
        `x=`printf '%s' 'a`b'`` is an ERROR to bash, "unexpected EOF while
        looking for matching `''", so the quote inside did NOT hide the closing
        backtick: a backtick runs to the next unescaped backtick and nothing
        opens inside it. The scanner said the whole thing was one closed span
        and accepted a file bash refuses.
        """
        done = subprocess.run([BASH, "-c", "x=`printf '%s' 'a`b'`"],
                              capture_output=True, text=True)
        self.assertIn("unexpected EOF", done.stderr)
        end, reason = ci_floor_check._arm_end("x=`printf '%s' 'a`b'` ;; t", 0)
        self.assertEqual(end, -1)
        self.assertIn("opened and never closed", reason)

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_double_quote_opens_the_three_substitutions(self) -> None:
        """The declared limit until the twelfth pass, closed and measured.

        `"` ran to its closer, so a `"` inside a `$( )` inside a `"` ended the
        outer quote early. All three of these print through, so all three open
        inside a double quote and carry their own nesting.
        """
        for script, printed in (
                ('printf "[%s]" "a$(printf "%s" X)b"', "[aXb]"),
                ('printf "[%s]" "a`printf b`c"', "[abc]"),
                ('u=;printf "[%s]" "${u:-"}"}"', "[}]")):
            with self.subTest(script):
                self.assertEqual(bash_says(script), printed)
        self.check('echo M1 && echo "a$(printf ";;")b" && echo M2',
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_double_quote_opens_neither_quote_form(self) -> None:
        """The direction that must not widen with it. Inside a double quote a
        `'` is literal and `$'...'` is a `$` beside a literal quote, so opening
        either would run the span to some later quote in the region."""
        self.assertEqual(bash_says("""printf "[%s]" "a'b\""""), "[a'b]")
        self.assertEqual(bash_says("""printf "[%s]" "$'a'\""""), "[$'a']")
        self.check("""echo M1 && echo "a'b" && echo M2""", {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_bare_paren_deepens_a_substitution(self) -> None:
        """`Span.nests`, reached at last.

        `test_a_nested_substitution_closes_at_the_outer_paren` never touched
        the counter, because the inner `$(` was re-opened as a span whatever
        the counter said, so the twelfth pass could set the depth wrong and
        watch 24 tests stay green. A SUBSHELL is the shape that needs it.
        MEASURED: `echo A$( (printf x) )#b && echo RAN_SECOND` prints `Ax#b`
        and `RAN_SECOND`, so bash balanced the bare parentheses and the `#`
        after the closing one is not a comment. Without the counter the span
        ends at the inner `)`, the next `)` is then an operator, a `#` after an
        operator IS a comment and the rest of the line goes with it.
        """
        self.assertEqual(bash_says("echo A$( (printf x) )#b && echo RAN"),
                         "Ax#b\nRAN\n")
        self.check("echo A$( (printf x) )#b && echo M1 && echo M2",
                   {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_bare_brace_deepens_an_expansion(self) -> None:
        """The same field one span along, and the marker set alone cannot see
        it: a `${ }` that ends early is followed by a `}` that is not a word
        break, so no `;;` moves. What moves is `_split_statements`, where `{`
        and `}` ARE separators, so an expansion read as ending early is split
        into two statements and the residue is reported as a command CI does
        not run.

        MEASURED: `${u:-{a}}` prints `{a}`, so bash balanced the bare braces
        rather than closing at the first `}`. `${u:-'}'}` prints `}` and
        `${u:-a\\}b}` prints `a}b`, so a quote and a backslash each hide one
        too.
        """
        self.assertEqual(bash_says("u=;printf '[%s]' ${u:-{a}}"), "[{a}]")
        self.assertEqual(bash_says("u=;printf '[%s]' ${u:-'}'}"), "[}]")
        self.assertEqual(bash_says("u=;printf '[%s]' ${u:-a\\}b}"), "[a}b]")
        self.check("u=; echo M1 ${u:-{a}} && echo M2", {"M1", "M2"})
        self.assertEqual(
            ci_floor_check._split_statements("echo M1 ${u:-{a}} && echo M2"),
            ["echo M1 ${u:-{a}} ", " echo M2"])

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_bare_ampersand_ends_a_statement(self) -> None:
        """**The fourteenth route, and it is passes 6, 7 and 8 one operator
        along.**

        bash's `&` terminates a list exactly as `;` does. `STATEMENT_BREAK`
        carried `&&` and no `&`, so `A & B` was ONE statement whose head is
        `A`, and `unseen_commands` drops a statement whose head is a
        `COMMAND_PREFIXES` prefix or a `SHELL_NOISE` builtin. MEASURED in a
        real clone: the `&&` before `node --test` in the `bench` arm rewritten
        as `&` on one line, with the `- run: bin/ocelli.sh gate bench` step
        replaced by the arm's two extractable `python3` commands, gave
        `bash -n` 0 and `scripts/ci_floor_check.py` exit 0 printing "every
        command in each gate's arm", `bench` gone from the named-only list and
        six node suites out of CI.

        The extent test could not see it. `M2` is inside the arm either way,
        which is why `scanner_runs_in_the_arm` exists.
        """
        self.assertEqual(bash_says("echo M1 & echo M2\nwait"), "M2\nM1\n")
        self.check("echo M1 & echo M2", {"M1", "M2"})
        self.assertEqual(ci_floor_check._split_statements("echo M1 & echo M2"),
                         ["echo M1 ", " echo M2"])

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_redirection_ampersand_is_not_a_statement_break(self) -> None:
        """The direction the fix must not widen into, and it was measured open.

        `&` is also the second character of `>&` and `<&` and the first of `&>`
        and `&>>`, and none of those separates anything. MEASURED with the
        naive alternative `r"[\\n;{}()]|&&|\\|\\||\\||&"` over the real runner:
        `unseen_commands` grew `unseen['panic'] = ['2', ...]`, a file descriptor
        reported as a command CI does not run, out of the `panic` arm's real
        `echo "wasm-pack is not installed. ..." >&2`. It cost no exit code only
        because `panic` already holds unseen commands and CI names the gate, so
        an arm whose only unextractable text was a redirection would have
        refused a legitimate state.

        bash is the authority here too: `echo M1 2>&1` prints `M1` and exits 0,
        where the naive reading `echo M1 2 & 1` backgrounds one command and
        then fails to find `1`, which `bash_says` would refuse outright.
        """
        self.assertEqual(bash_says("echo M1 2>&1"), "M1\n")
        for text in ("echo M1 2>&1", "echo M1 >&2", "echo M1 &> /dev/null",
                     "echo M1 &>> /dev/null", "echo M1 <&0"):
            with self.subTest(text):
                self.assertEqual(ci_floor_check._split_statements(text), [text])

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_dollar_single_quote_is_a_span(self) -> None:
        """The other half of the limit the eleventh pass declared.

        `$'...'` was not an opener, so the `$` was text and the `'` opened an
        ordinary single quote, which closes at the escaped quote inside and
        leaves the rest of the region reading as quoted. MEASURED: `$'a\\'b'`
        is the one string `a'b` to bash. Its EXTENT is modelled now. Its C
        escapes are not decoded, which `shell_words` declares.
        """
        self.assertEqual(bash_says("printf '[%s]' $'a\\'b'"), "[a'b]")
        self.check("echo M1 && echo $'a\\';;b' && echo M2", {"M1", "M2"})

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_heredoc_body_is_not_a_command(self) -> None:
        """A legitimate here-document note in an arm, which used to refuse.

        MEASURED before the fix, with `: <<'EOF-1'` and a two-word note in the
        `prose` arm of a real clone: `scripts/ci_floor_check.py` exited 1
        reporting that the arm "runs 'a note', 'EOF-1'". Two lines of English
        demanded of CI as commands is a guard refusing a legitimate state,
        which is the runbook's own sentence. The body is a piece of its own
        kind now, and `_split_statements` drops it.
        """
        arm = "python3 x.py &&\n: <<'EOF-1'\na note\nEOF-1\n"
        text = ci_floor_check.shell_source(f"case g in\n  g) {arm} ;;\nesac\n")
        start = text.index("g)") + 2
        end, reason = ci_floor_check._arm_end(text, start)
        self.assertEqual(reason, "")
        statements = ci_floor_check._split_statements(text[start:end])
        self.assertEqual([s for s in statements if "a note" in s], [])
        self.assertEqual([s for s in statements if "EOF-1" in s
                          and "<<" not in s], [])

    def test_a_heredoc_body_that_never_closes_is_named(self) -> None:
        """The refusal `_heredoc_end`'s docstring claimed for two passes.

        It said an unterminated body "reports the whole remainder so the
        caller's unclosed-span refusal is what fires", and `shell_pieces`
        cleared `pending` before its own `if pending:` test, so `unclosed` came
        back empty and the arm was refused for having no `;;` at all. bash
        refuses this file too, with a here-document warning, so the direction
        was never in doubt. What it said was.
        """
        end, reason = ci_floor_check._arm_end(
            "echo a &&\n: <<EOF\nbody ;;\necho b ;;", 0)
        self.assertEqual(end, -1)
        self.assertIn("here-document `<<EOF`", reason)
        self.assertIn("never meets its delimiter", reason)


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


def workflow_with(step: str) -> str:
    """One job, one step, the smallest workflow `run_commands` will read."""
    return ("on: [push]\njobs:\n  j:\n    runs-on: ubuntu-latest\n"
            "    steps:\n" + step)


class WhatCiRunsInAStepBody(unittest.TestCase):
    """The SECOND consumer of the tokenizer, and it had no oracle at all.

    `run_commands` reads `.github/workflows/ci.yml`'s `run:` bodies, and until
    the S03 review's thirteenth pass it read them ONE LINE AT A TIME while the
    workflow itself was parsed. Cross-line shell state was discarded between
    the line that establishes it and the lines it governs, which is one line of
    code producing a fail-open and a false refusal at once. bash is the
    authority for both, exactly as it is for the arm reader above.
    """

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_heredoc_body_is_not_an_invocation(self) -> None:
        """Route 4a of the twelfth pass, in the spelling that pass left open.

        A gate name in a here-document BODY is data. MEASURED in a real clone
        with the `- run: bin/ocelli.sh gate panic` step rewritten as this
        body: `scripts/ci_floor_check.py` exit 0 with the runner never called,
        on HLD section 23's wasm panic-hook proof, the one property no native
        test can observe.
        """
        self.assertEqual(
            bash_says("cat <<'EOF'\nbin/ocelli.sh gate panic\nEOF"),
            "bin/ocelli.sh gate panic\n")
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: |\n"
            "          cat <<'EOF'\n"
            "          bin/ocelli.sh gate panic\n"
            "          EOF\n"))
        self.assertNotIn("panic", ci_floor_check.invoked_gates(commands))

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_continuation_hides_no_invocation_either(self) -> None:
        """The same fail-open through the other cross-line production."""
        self.assertEqual(bash_says("echo not \\\nbin/ocelli.sh gate panic"),
                         "not bin/ocelli.sh gate panic\n")
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: |\n"
            "          echo not \\\n"
            "          bin/ocelli.sh gate panic\n"))
        self.assertNotIn("panic", ci_floor_check.invoked_gates(commands))

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_a_continued_command_is_one_command(self) -> None:
        """The false refusal, which is the direction the fix must not undo.

        MEASURED before the fix, with the real `content` step continued onto a
        second line: the check exited 1 saying nothing in the workflow ran the
        gate, while bash runs it as one command.
        """
        self.assertEqual(
            bash_says("echo python3 scripts/staged_content_check.py \\\n"
                      "  --tracked"),
            "python3 scripts/staged_content_check.py --tracked\n")
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: |\n"
            "          python3 scripts/staged_content_check.py \\\n"
            "            --tracked\n"))
        self.assertTrue(ci_floor_check.runs_command(
            "python3 scripts/staged_content_check.py --tracked", commands))

    def test_two_commands_on_one_line_are_two_commands(self) -> None:
        """A statement and not a line is the unit now, so a step chaining two
        of a gate's arm commands covers both. The line reader matched neither,
        because `a && b` is not either argv."""
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: python3 scripts/a.py && python3 scripts/b.py\n"))
        self.assertTrue(ci_floor_check.runs_command("python3 scripts/a.py",
                                                    commands))
        self.assertTrue(ci_floor_check.runs_command("python3 scripts/b.py",
                                                    commands))

    def test_continue_on_error_takes_the_step_away(self) -> None:
        """`continue-on-error` was in the parsed tree and nothing read it.

        Three plants, each valid YAML, each leaving the check at exit 0 with
        the `guards` gate reported covered: the key on the step, the key on the
        job, and `|| true` appended to the run.
        """
        step = ci_floor_check.run_commands(workflow_with(
            "      - continue-on-error: true\n"
            "        run: bin/ocelli.sh gate guards\n"))
        self.assertTrue(all(c.tolerated for c in step), step)
        self.assertEqual(
            [c for c in step if c.runs_on({"push"})], [])

        job = ci_floor_check.run_commands(
            "on: [push]\njobs:\n  j:\n    runs-on: ubuntu-latest\n"
            "    continue-on-error: true\n    steps:\n"
            "      - run: bin/ocelli.sh gate guards\n")
        self.assertTrue(all(c.tolerated for c in job), job)

        swallowed = ci_floor_check.run_commands(workflow_with(
            "      - run: bin/ocelli.sh gate guards || true\n"))
        runner = [c for c in swallowed if "gate guards" in c.text]
        self.assertEqual(len(runner), 1, swallowed)
        self.assertTrue(runner[0].tolerated, runner)

    def test_continue_on_error_false_takes_nothing_away(self) -> None:
        """The accept direction. `false` is a value GitHub accepts and it
        tolerates nothing, so reading the key must not turn writing it down
        explicitly into a refusal."""
        for value in ("false", "False", "no", "off", "0"):
            with self.subTest(value):
                commands = ci_floor_check.run_commands(workflow_with(
                    f"      - continue-on-error: {value}\n"
                    "        run: bin/ocelli.sh gate guards\n"))
                self.assertEqual([c.tolerated for c in commands], [""])

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_which_failures_bash_e_actually_reports(self) -> None:
        """**The obvious reading of the errexit paragraph is wrong, and this is
        the table that says so.** GitHub runs a `run:` body as `bash -e`, and
        every row of `_tolerated_statements` is one of these measurements
        rather than a reading. The first version of that function asserted that
        a failing left side of `&&` fails the step, and rows two and three say
        it does not: the short circuit means the command after the final `&&`
        never runs, so nothing fires errexit and the list status is discarded.

        The two `true || false` rows are the S03 review's fourteenth pass and
        they are about the OTHER side of that operator. A right-hand side runs
        only when the left one FAILED, so with a succeeding left side it never
        runs at all, and `true || bin/ocelli.sh gate guards` satisfied the
        `guards` gate at exit 0 with the runner never called. That is a gate
        counted as invoked rather than a failure discarded, and it reaches the
        same place.
        """
        for script, status in (("false\necho AFTER", 1),
                               ("false && true\necho AFTER", 0),
                               ("false && false\necho AFTER", 0),
                               ("false && true", 1),
                               ("true && false\necho AFTER", 1),
                               ("false || false\necho AFTER", 1),
                               ("false || true\necho AFTER", 0),
                               ("true || false\necho AFTER", 0),
                               ("true || false", 0),
                               ("true | false\necho AFTER", 1),
                               ("false | cat\necho AFTER", 0),
                               ("false &\necho AFTER\nwait", 0)):
            with self.subTest(script):
                done = subprocess.run([BASH, "-ec", script],
                                      capture_output=True, text=True)
                self.assertEqual(done.returncode, status)

    def test_the_toleration_rule_matches_that_table(self) -> None:
        """The same shapes, asked of the reader rather than of bash.

        The set is the statement indices whose failure the shell discards, read
        straight off the measurements above.

        The three `||` rows gained a member in the fourteenth pass, from the
        two `true || false` measurements added above rather than to make
        anything pass: index 1 of `x || y` runs only if `x` failed, so with
        `x` succeeding its own status never reaches the step.
        """
        for body, tolerated in (("x\ny", set()),
                                ("x && y\nz", {0}),
                                ("x && y", set()),
                                ("x || y\nz", {0, 1}),
                                ("x || y", {0, 1}),
                                ("x | y\nz", {0}),
                                ("x | y", {0}),
                                ("x &\ny", {0}),
                                ("x && y || z", {0, 1, 2}),
                                ("x && y && z", set())):
            with self.subTest(body):
                self.assertEqual(
                    set(ci_floor_check._tolerated_statements(body)), tolerated)

    def test_a_final_and_list_keeps_both_halves(self) -> None:
        """The accept direction the rule must not lose. A step whose whole body
        is `a && b` reports `a`'s failure, because the last list in the body
        decides the script's status, so reading `&&` as a swallow everywhere
        would refuse a legitimate arrangement."""
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: bin/ocelli.sh gate guards && python3 scripts/a.py\n"))
        self.assertEqual([c.tolerated for c in commands], ["", ""])

    def test_an_unclosed_quote_in_a_body_is_named(self) -> None:
        """A refusal that names the quote rather than a gate.

        The swallowed text can only HIDE commands, so the outcome without this
        is a refusal naming whichever gate went missing, which sends its reader
        to `bin/ocelli.sh` for a defect in `ci.yml`.
        """
        with self.assertRaises(RuntimeError) as caught:
            ci_floor_check.run_commands(workflow_with(
                "      - run: |\n"
                "          echo 'never closed\n"
                "          bin/ocelli.sh gate guards\n"))
        self.assertIn("opened and never closed", str(caught.exception))


# One `run:` body shape per row, with `{0}` and `{1}` where a command goes.
#
# The GRAMMAR the oracle below generates from. The point of generating rather
# than listing is that `_tolerated_statements` carried a hand-written table of
# ten measured examples and passed it, while four whole CONTEXTS were missing
# from the enumeration and each one put `bin/ocelli.sh gate guards`, the gate
# that watches every other gate, into a step whose failure could not fail the
# run. A table of examples can only contain what its author thought of.
STEP_BODY_SHAPES = (
    "{0}\n", "{0}\necho tail\n",
    "{0} && {1}\n", "{0} && {1}\necho tail\n",
    "{0} || {1}\n", "{0} || {1}\necho tail\n",
    "{0} | {1}\n", "{0} | {1}\necho tail\n",
    "{0} &\nwait\n", "{0} &\necho tail\nwait\n",
    "{0}; {1}\n", "{0}\n{1}\n",
    "if {0}; then echo T; fi\n", "if {0}; then echo T; fi\necho tail\n",
    "if {0} && {1}; then echo T; fi\n",
    "if {0} || {1}; then echo T; fi\n",
    "if true; then {0}; fi\n", "if true; then {0}; fi\necho tail\n",
    "if false; then echo A; else {0}; fi\n",
    "if true; then echo A; else {0}; fi\necho tail\n",
    "if false; then echo A; elif {0}; then echo T; fi\n",
    "while {0}; do break; done\n", "while {0}; do break; done\necho tail\n",
    "until {0}; do break; done\n",
    "while true; do {0}; break; done\n",
    "until true; do {0}; done\necho tail\n",
    "for x in a; do {0}; done\n",
    "for f in $NOTHING; do {0}; done\necho tail\n",
    "! {0}\n", "! {0}\necho tail\n", "! {0} && {1}\n",
    "set +e\n{0}\necho done\n", "set +e\n{0}\n",
    "set +o errexit\n{0}\necho done\n", "set +o errexit\n{0}\n",
    "set +e\nset -e\n{0}\n", "set +e\nset -e\n{0}\necho tail\n",
    "set +eu\n{0}\necho done\n", "set +o pipefail\n{0}\necho done\n",
    "shopt -uo errexit\n{0}\necho done\n",
    "set +e\n( set -e )\n{0}\necho done\n",
    "{0} && {1} || echo fallback\n",
    "if {0}; then {1}; fi\n",
    "while {0}; do {1}; break; done\n",
    "set +e\nif {0}; then echo T; fi\n{1}\n",
    "guards_step() {{ {0}; }}\necho tail\n",
    'case "$X" in Windows) {0} ;; esac\necho tail\n',
    "exit 0\n{0}\n",
    "{{ {0}; }}\necho tail\n",
    # NESTED shapes, added by the sixteenth pass. Every one of these walks
    # past a reader that tests only a statement's FIRST WORD, and two of them
    # measured a defect in the reader the fifteenth pass added: `do case ...`
    # hides the `case` behind the `do`, and an `elif` chain has two `then` and
    # one `fi`.
    'for i in 1; do case "$X" in z) {0} ;; esac; done\necho tail\n',
    "if true; then if false; then {0}; fi; fi\necho tail\n",
    'case "$X" in a) case "$Y" in b) {0} ;; esac ;; esac\necho tail\n',
    "for i in 1; do for j in 2; do {0}; done; done\necho tail\n",
    'while true; do case x in x) {0} ;; esac; break; done\necho tail\n',
    "if true; then echo a; elif false; then echo b; fi\n{0}\n",
    "if true; then echo a; elif false; then echo b; else echo c; fi\n{0}\n",
    "if false; then echo a; elif false; then echo b; else {0}; fi\n"
    "echo tail\n",
    'case "$X" in a|b) {0} ;; *) : ;; esac\necho tail\n',
    'case "$X" in (a) {0} ;; esac\necho tail\n',
    'case "$X" in *) echo "arm ) paren" ;; esac\n{0}\n',
    'echo "case x in )"\n{0}\n',
    "cat <<EOF\ncase x in\nEOF\n{0}\n",
    "( exit 0 )\n{0}\n",
    "f() {{ exit 0; }}\n{0}\n",
    "f() {{ if true; then {0}; fi; }}\necho tail\n",
    "( {{ {0}; }} )\necho tail\n",
    "x=$(echo hi)\n{0}\n",
    "if [ \"$X\" = \"y\" ]; then :; fi\n{0}\n",
)


def bash_fails(body: str) -> bool | None:
    """Does `body` exit non-zero under `bash -ec`, or None if bash refuses it.

    GitHub runs a step body as `bash -e {0}`, so a command's failure reaches
    the step exactly when the script exits non-zero, and a command that never
    RUNS cannot make it do so. Both halves of what this file has to decide are
    therefore one measurement.
    """
    if subprocess.run([BASH, "-n"], input=body, text=True,
                      capture_output=True).returncode != 0:
        return None
    try:
        done = subprocess.run([BASH, "-ec", body], capture_output=True,
                              text=True, timeout=10)
    except subprocess.TimeoutExpired:  # pragma: no cover, a runaway shape
        return None
    return done.returncode != 0


def _slot_count(shape: str) -> int:
    return len(set(re.findall(r"\{(\d)\}", shape)))


def _body_and_index(shape: str, target: int) -> tuple[str, int] | None:
    """`shape` with `target` failing and every other command succeeding."""
    count = _slot_count(shape)
    words = [("false" if i == target else "true") + f" M{i}"
             for i in range(count)]
    body = shape.format(*words)
    pairs = ci_floor_check._statement_separators(
        ci_floor_check.shell_source(body))
    found = [i for i, (statement, _) in enumerate(pairs)
             if f"M{target}" in statement]
    return (body, found[0]) if len(found) == 1 else None


class WhoseFailureBashDiscards(unittest.TestCase):
    """`_tolerated_statements` against bash over GENERATED bodies.

    The fourth and last consumer of the tokenizer to get an oracle. It carried
    a hand-written table of ten measured `bash -ec` exit codes until the S03
    review's fourteenth pass. The table was correct about every row in it and
    was missing four whole contexts, because a table of examples can only
    contain what its author thought of.

    Generating the input has now found six more that no reviewer named, in two
    rounds. The fifteenth pass added `case`, a function definition, `exit`, a
    never-taken `else`, an empty `for` word list and an `until true` body, and
    the first three are TOTAL BYPASSES: bash never runs the command at all,
    and the check reported every floor gate invoked. Each was measured at exit
    0 with the real `bin/ocelli.sh gate guards` step replaced.

    **So the question this file asks is not "does the failure reach the step".
    It is "is the shell GUARANTEED to run this command and report its
    failure".** The first is what `bash -e` decides. The second is what "CI
    runs the floor" means, and it is strictly stronger. The two tests below
    are the two halves of that: no shape may be counted when bash discards it,
    and the shapes counted MORE strictly than bash are declared exactly.
    """

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_no_generated_shape_is_a_fail_open(self) -> None:
        """If bash exits 0, the statement must NOT count as an invocation.

        This is the direction that matters and it is asserted with no
        exceptions. A shape bash exits 0 on either discarded the failure or
        never ran the command, and in both cases nothing here says CI runs the
        gate.
        """
        checked = 0
        for shape in STEP_BODY_SHAPES:
            for target in range(_slot_count(shape)):
                made = _body_and_index(shape, target)
                self.assertIsNotNone(made, f"ambiguous marker in {shape!r}")
                body, index = made
                reaches = bash_fails(body)
                self.assertIsNotNone(reaches, f"bash refused {body!r}")
                tolerated = index in ci_floor_check._tolerated_statements(
                    ci_floor_check.shell_source(body))
                checked += 1
                if not reaches:
                    self.assertTrue(
                        tolerated,
                        f"FAIL-OPEN: bash exits 0 for statement {index} of "
                        f"{body!r}, so its failure cannot fail the step, and "
                        f"this file counts it as CI running the gate")
        self.assertGreaterEqual(checked, 83)

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_the_shapes_refused_more_strictly_than_bash_are_declared(
            self) -> None:
        """The DECLARED cost, asserted as an exact set.

        Every member is a compound BODY that does run, so bash reports its
        failure and this file declines to count it anyway, because whether the
        body is reached depends on a condition, a word list or a caller that
        this scanner cannot evaluate. Each therefore costs a REFUSAL naming
        the gate rather than a pass, which is the safe direction.

        None of these appears around a gate in `.github/workflows/ci.yml`, and
        `python3 scripts/ci_floor_check.py` measures 0 compound statements in
        its `run:` bodies, so the cost today is zero. A shape LEAVING this set
        is a fail-open and the test above catches it. A shape joining it is a
        new false refusal and this test catches that.
        """
        conservative = set()
        for shape in STEP_BODY_SHAPES:
            for target in range(_slot_count(shape)):
                body, index = _body_and_index(shape, target)
                if not bash_fails(body):
                    continue
                if index in ci_floor_check._tolerated_statements(
                        ci_floor_check.shell_source(body)):
                    conservative.add(body)
        self.assertEqual(conservative, {
            "( { false M0; } )\necho tail\n",
            "for i in 1; do for j in 2; do false M0; done; done\necho tail\n",
            "for x in a; do false M0; done\n",
            "if false; then echo A; else false M0; fi\n",
            "if false; then echo a; elif false; then echo b; else false M0; "
            "fi\necho tail\n",
            "if true M0; then false M1; fi\n",
            "if true; then false M0; fi\n",
            "if true; then false M0; fi\necho tail\n",
            "while true M0; do false M1; break; done\n",
            "while true; do case x in x) false M0 ;; esac; break; done\n"
            "echo tail\n",
            "while true; do false M0; break; done\n",
            "{ false M0; }\necho tail\n",
        })

    @unittest.skipUnless(BASH, "no bash on this machine")
    def test_the_right_hand_side_of_and_is_the_remaining_residue(self) -> None:
        """F-X019, reduced to one shape by the fifteenth pass.

        `a && GATE` runs the gate only when `a` SUCCEEDS, so it is not a
        guaranteed invocation either, and this asserts that it is STILL
        COUNTED, which is a fail-open held open on purpose. It is not closed
        here because `invoked_gates`' own docstring documents `cd x &&
        bin/ocelli.sh gate y` as a legitimate invocation, so refusing it
        contradicts a declared behaviour rather than repairing a reader. That
        is a decision with an owner.

        The test above cannot see this shape: it fails ONE slot and succeeds
        every other, so it never constructs the failing left side that makes
        the right side unreachable. **This test is the record that the hole is
        known**, and it goes red when F-X019 closes it, which is the signal to
        delete it rather than a regression.
        """
        body = "false M0 && false M1\necho tail\n"
        self.assertFalse(bash_fails(body), "bash must exit 0 for this body")
        source = ci_floor_check.shell_source(body)
        pairs = ci_floor_check._statement_separators(source)
        index = next(i for i, (s, _) in enumerate(pairs) if "M1" in s)
        self.assertNotIn(index, ci_floor_check._tolerated_statements(source),
                         "F-X019 appears to be closed. Delete this test.")


class WhichShellTheStepRunsUnder(unittest.TestCase):
    """A `run:` body is read as `bash -e`, so any other shell is refused.

    GitHub's default on Linux with no `shell:` is `bash -e {0}`. A custom
    template is the dangerous value and it is named nowhere in the workflow
    syntax as a hazard: `shell: bash {0}` is still bash and has NO `-e`.
    """

    def test_a_custom_template_on_a_step_is_refused_by_name(self) -> None:
        with self.assertRaises(RuntimeError) as raised:
            ci_floor_check.run_commands(workflow_with(
                "      - shell: bash {0}\n"
                "        run: bin/ocelli.sh gate guards\n"))
        self.assertIn("shell:", str(raised.exception))
        self.assertIn("bash {0}", str(raised.exception))

    def test_a_workflow_level_default_is_read_too(self) -> None:
        """The spelling that touches no step and takes every `run:` with it."""
        with self.assertRaises(RuntimeError) as raised:
            ci_floor_check.run_commands(
                "on: [push]\ndefaults:\n  run:\n    shell: bash {0}\n"
                "jobs:\n  j:\n    runs-on: ubuntu-latest\n    steps:\n"
                "      - run: bin/ocelli.sh gate guards\n")
        self.assertIn("defaults.run.shell:", str(raised.exception))

    def test_a_job_level_default_overrides_the_workflow_one(self) -> None:
        """A job saying `bash` is measured, even under a workflow that is not.

        This is the case that makes the resolution ORDER load-bearing rather
        than decorative, and reading only the workflow level would refuse it.
        """
        commands = ci_floor_check.run_commands(
            "on: [push]\ndefaults:\n  run:\n    shell: bash {0}\n"
            "jobs:\n  j:\n    runs-on: ubuntu-latest\n"
            "    defaults:\n      run:\n        shell: bash\n"
            "    steps:\n      - run: bin/ocelli.sh gate guards\n")
        self.assertIn("guards", ci_floor_check.invoked_gates(commands))

    def test_the_measured_shells_are_accepted(self) -> None:
        for shell in sorted(ci_floor_check.MEASURED_SHELLS):
            with self.subTest(shell):
                commands = ci_floor_check.run_commands(workflow_with(
                    f"      - shell: {shell}\n"
                    f"        run: bin/ocelli.sh gate guards\n"))
                self.assertIn("guards",
                              ci_floor_check.invoked_gates(commands))

    def test_a_step_with_no_shell_is_still_read(self) -> None:
        commands = ci_floor_check.run_commands(workflow_with(
            "      - run: bin/ocelli.sh gate guards\n"))
        self.assertIn("guards", ci_floor_check.invoked_gates(commands))


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
        """Refused, and not dropped. Dropping it was the defect.

        **This test planted `prose2` and asserted `gate_row_problems` was
        EMPTY until the S03 review's twelfth pass**, with a docstring
        saying "Refused, and not dropped". `prose2` is INSIDE
        `[A-Za-z0-9_-]+`, so it asserted the opposite of its own name: that a
        digit in a name is counted, which is true, which is
        `test_a_digit_in_a_name_is_counted` below, and which says nothing
        about the refusal. MEASURED: with the `if not GATE_NAME.match(name)`
        branch disabled, all 24 tests, both unit suites, the census and all 54
        `ci-floor` probes stayed at their unmutated status. The name-class
        refusal was watched by nothing at all.

        A dot is the shape that matters, because the refusal's own sentence is
        that a gate name reaches `re.escape`-free patterns in this file, in
        `scripts/guards/census.py` and in the catalogue's probe builders, and
        a dot in a name is a regex wildcard in every one of them.
        """
        runner = self.RUNNER.read_text().replace(
            "GATES=(\n", 'GATES=(\n  "pro.se|no|a dotted name"\n', 1)
        problems = ci_floor_check.gate_row_problems(runner)
        self.assertTrue(any("whose name is outside" in p for p in problems),
                        problems)
        self.assertIn("pro.se", ci_floor_check.declared_gates(runner))

    def test_a_digit_in_a_name_is_counted(self) -> None:
        """The direction the class must not narrow into.

        `bin/ocelli.sh` reads an entry with `IFS='|' read -r name gpu desc`
        and imposes no class, and the two Python copies of the row regex both
        spelled it `[a-z-]+`, so a gate named `prose2` was counted by bash and
        by neither of them. It is counted here and it is not a problem row.
        """
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


class BothLintTablesAreRead(unittest.TestCase):
    """`LINT_TABLES`, which the twelfth pass could narrow to `("clippy",)`
    with 24 tests staying green.

    MEASURED under the pinned 1.97.1 toolchain, one workspace declaring
    `[workspace.lints.clippy] cast_possible_truncation = "deny"` and
    `[workspace.lints.rust] unused_variables = "deny"`, one member inheriting
    with `[lints] workspace = true`, one `x as i32` and one unused binding.
    `cargo clippy --workspace --all-targets` exits 101 reporting BOTH, as
    errors: `casting `i64` to `i32` may truncate the value` and `unused
    variable: y`. So the `rust` table is enforced exactly as the `clippy` one
    is, and HLD 27.1's `unsafe_code` row lives in it.
    """

    DOCUMENT = ('[workspace.lints.clippy]\n'
                'cast_possible_truncation = "deny"\n\n'
                '[workspace.lints.rust]\n'
                'unsafe_code = "forbid"\n')

    def test_the_rust_table_is_read_and_not_only_clippy(self) -> None:
        tables, error = lint_policy_check.workspace_lints(self.DOCUMENT)
        self.assertEqual(error, "")
        self.assertEqual(sorted(tables), ["clippy", "rust"])
        self.assertEqual(lint_policy_check.lint_levels(tables["rust"]),
                         {"unsafe_code": "forbid"})

    def test_the_recorded_rows_carry_both_tables(self) -> None:
        """The ratchet records what the guard reads, so narrowing the tables
        has to move the recorded value rather than leaving it stale."""
        rows = lint_policy_check.workspace_lints_rows(self.DOCUMENT)
        self.assertIn("clippy.cast_possible_truncation", rows)
        self.assertIn("rust.unsafe_code", rows)


class WhichMemberGlobsTheWorkspaceDeclares(unittest.TestCase):
    """`member_patterns`, the third hand-rolled reader of this document.

    Every spelling below was MEASURED to be one workspace to cargo 1.97.1:
    `cargo metadata --no-deps --offline` exits 0 on each. The regex this
    replaced required a literal `[workspace]` header on its own line and each
    entry in DOUBLE quotes, so it read no members at all from two of them, and
    `main` then refuses a workspace whose member list it could not resolve.
    """

    def test_every_spelling_of_one_member_list(self) -> None:
        for document in ('[workspace]\nmembers = ["crates/*"]\n',
                         'workspace.members = ["crates/*"]\n',
                         '[workspace]\n"members" = [\'crates/*\']\n',
                         '[workspace]\nmembers = [\n  "crates/*",\n]\n'):
            with self.subTest(document):
                self.assertEqual(lint_policy_check.member_patterns(document),
                                 (["crates/*"], []))

    def test_the_exclude_list_is_read_the_same_way(self) -> None:
        self.assertEqual(
            lint_policy_check.member_patterns(
                'workspace.members = ["crates/*"]\n'
                'workspace.exclude = ["crates/skip"]\n'),
            (["crates/*"], ["crates/skip"]))


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

    def test_a_value_that_is_not_the_boolean_true_does_not_inherit(self
                                                                    ) -> None:
        """`is True`, and the twelfth pass could weaken it to `is not None`
        with the suite staying green.

        MEASURED under the pinned 1.97.1 toolchain on a workspace carrying
        `cast_possible_truncation = "deny"` and `unused_variables = "deny"`,
        against a member with one `x as i32` and one unused binding:

            [lints] workspace = true      101, both lints fire
            [lints] workspace = false     101, `workspace` cannot be false
            [lints] workspace = "yes"     101, invalid type: string

        so neither of the last two is a manifest cargo will build, let alone
        one that inherits the table. `is not None` reports both as inheriting,
        which is a positive assertion of a false thing about a manifest that
        does not compile.
        """
        for spelling in ("false", '"yes"', "1"):
            with self.subTest(spelling):
                inherits, error = lint_policy_check.inherits_workspace_lints(
                    f'[package]\nname = "a"\n\n[lints]\n'
                    f'workspace = {spelling}\n')
                self.assertEqual((inherits, error), (False, ""))

    def test_a_manifest_that_is_not_toml_reports_the_error(self) -> None:
        inherits, error = lint_policy_check.inherits_workspace_lints(
            "[package]\nname = = \n")
        self.assertFalse(inherits)
        self.assertNotEqual(error, "")


if __name__ == "__main__":  # pragma: no cover
    unittest.main()
