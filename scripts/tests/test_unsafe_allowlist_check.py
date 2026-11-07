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


class StagedIndex(unittest.TestCase):
    """A disposable index with one clean crate and one clean vendored file."""

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


class IndexAudit(StagedIndex):
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

    def test_string_continuation_does_not_blank_following_code(self):
        # A Rust string continuation is a backslash before a newline inside a
        # string literal. While the STRING pattern could not cross it, the
        # match at the literal's opening quote failed and the regex re-paired
        # from that literal's closing quote to the next literal's opening
        # quote, so the `unsafe` between the two was blanked as string and
        # the guard returned 0. The trailing literal is what makes the
        # re-pairing possible, and this test was red on that pattern.
        self.write(
            "crates/example/src/lib.rs",
            'pub const FIRST: &str = "one \\\n'
            'two";\n'
            "pub unsafe fn planted() {}\n"
            'pub const LAST: &str = "three";\n')
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:3", output)

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


class Tokenizer(StagedIndex):
    """The left-to-right pass F-232 replaced the three regexes with.

    Each fixture is one of the five triggers the F-229 review planted against
    the string regex, or a shape the tokenizer must NOT report. The planted
    `unsafe` sits between the trigger and a trailing literal, because the
    trailing literal is what gives the old regex a place to close its false
    string: without it the old scanner happens to report the block, and the
    fixture would prove nothing. Expected line numbers are the planted
    block's own line, counted by hand in each fixture.
    """

    def test_unsafe_after_quote_char_literal(self):
        # AC1. `'"'` is a char literal holding a double quote. The STRING
        # regex opened a false match at that quote and ran to the next
        # literal's opening quote, blanking the block between them.
        self.write(
            "crates/example/src/lib.rs",
            "pub const QUOTE: char = '\"';\n"        # line 1, the trigger
            "pub unsafe fn planted() {}\n"           # line 2, refused
            'pub const AFTER: &str = "next";\n')     # line 3, false close
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)

    def test_unsafe_after_escaped_quote_char_literal(self):
        # AC2. The escaped spelling of the same trigger. The backslash sits
        # before the opening quote of the false match, so it changed nothing
        # about where the regex blanked.
        self.write(
            "crates/example/src/lib.rs",
            "pub const ESCAPED: char = '\\\"';\n"    # line 1, the trigger
            "pub unsafe fn planted() {}\n"           # line 2, refused
            'pub const AFTER: &str = "next";\n')     # line 3, false close
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)

    def test_unsafe_after_raw_string_ending_in_backslash(self):
        # AC3. `r"a\"` is a COMPLETE raw string holding `a\`. A rule that
        # treats the backslash as an escape inside a raw string runs past the
        # closing quote and swallows the block after it.
        self.write(
            "crates/example/src/lib.rs",
            "pub const RAW: &str = r\"a\\\";\n"       # line 1, r"a\"
            "pub unsafe fn planted() {}\n"           # line 2, refused
            'pub const AFTER: &str = "next";\n')     # line 3, false close
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)

    def test_unsafe_after_raw_string_holding_a_quote(self):
        # AC4. A raw string's hash count is part of its delimiter, in both
        # the plain and the byte form. A rule with a fixed hash count
        # mis-closes each of these and blanks the block that follows.
        self.write(
            "crates/example/src/lib.rs",
            'pub const RAW: &str = r#"a " b"#;\n'    # line 1, holds a quote
            "pub unsafe fn planted() {}\n"           # line 2, refused
            'pub const AFTER: &str = "next";\n')     # line 3, false close
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)
        self.write(
            "crates/example/src/lib.rs",
            'pub const BYTES: &[u8] = br##"x "# y"##;\n'  # line 1
            "pub unsafe fn planted() {}\n"                # line 2, refused
            'pub const AFTER: &str = "next";\n')          # line 3
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)

    def test_unsafe_after_line_comment_with_a_quote(self):
        # AC5. A quote inside a line comment or a doc comment is comment
        # text. The regexes stripped strings BEFORE line comments, so the
        # unpaired quote opened a false string into the code that follows.
        for comment in ('// say "', '/// say "'):
            self.write(
                "crates/example/src/lib.rs",
                comment + "\n"                       # line 1, the trigger
                "pub unsafe fn planted() {}\n"       # line 2, refused
                'pub const AFTER: &str = "next";\n')  # line 3, false close
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("crates/example/src/lib.rs:2", output)

    def test_no_keyword_inside_literals_or_comments(self):
        # AC7. None of these is the `unsafe` token. The nested block comment
        # is ONE comment: the old regex stopped at the inner `*/` and
        # reported the second half as code, so this fixture is also red on
        # the scanner it replaced, in the false-positive direction.
        self.write(
            "crates/example/src/lib.rs",
            'pub const IN_A_STRING: &str = "unsafe";\n'
            'pub const IN_A_RAW_STRING: &str = r#"unsafe"#;\n'
            'pub const IN_A_BYTE_STRING: &[u8] = b"unsafe";\n'
            "pub const CHAR_CASE: char = 'u';\n"
            'pub const AFTER_A_CHAR: &str = "unsafe";\n'
            "// unsafe inside a line comment\n"
            "/// unsafe inside a doc comment\n"
            # N2, review pass 2. `////` is a divider comment, which
            # rustdoc does not read as documentation, so an `unsafe {` on
            # one is still not a finding.
            "//// unsafe { 0 }\n"
            "/* /* unsafe */ unsafe */\n"
            "pub const IN_AN_IDENTIFIER: usize = is_unsafe;\n"
            "pub fn r#unsafe() {}\n")
        self.stage()
        self.assertEqual(self.run_guard()[0], 0)

    def test_lifetime_is_not_a_char_literal(self):
        # AC7. `'a` is a lifetime. Reading it as an unterminated char
        # literal blanks the line that follows, which is where the planted
        # block sits, so the tokenizer must end the token at the identifier.
        self.write(
            "crates/example/src/lib.rs",
            "pub fn borrowed<'a>(value: &'a u8) {}\n"  # line 1
            "pub unsafe fn planted() {}\n")            # line 2, refused
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:2", output)

    def test_unterminated_literal_is_refused(self):
        # AC8. An unterminated string, raw string or block comment is a
        # refusal naming the file and the line the literal starts on. The
        # regexes answered one by blanking to the end of the file.
        for source in (
                'pub const NEVER: &str = "no terminator;\n',
                'pub const RAW: &str = r"no terminator;\n',
                "/* never closed\npub fn f() {}\n"):
            self.write("crates/example/src/lib.rs", source)
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("crates/example/src/lib.rs:1", output)
            self.assertIn("unterminated", output)

    def test_weakening_attribute_is_refused(self):
        # AC14. allow, expect and warn of `unsafe_code`, inner or outer, are
        # refused in any tracked Rust file. The path is a trybuild one,
        # because those files are compiled by trybuild and no lint table
        # reaches them, which is why the scanner has to.
        for attribute in ("#![allow(unsafe_code)]", "#[allow(unsafe_code)]",
                          "#![expect(unsafe_code)]", "#[warn(unsafe_code)]"):
            self.write("crates/example/tests/ui/planted.rs",
                       attribute + "\npub fn clean() {}\n")
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("unsafe_code` lowered", output)

    def test_unsafe_attribute_is_reported(self):
        # D1, review pass 1. `#[unsafe(no_mangle)]` is edition 2024's only
        # accepted spelling and rustc refuses it with "declaration of a
        # `no_mangle` function", so the attribute body is code and not a
        # suppression. The old scanner dropped every token between `#[` and
        # `]`, which also blinded the vendored count to the same shape.
        for source in ('#[unsafe(no_mangle)]\npub fn planted() -> i32 { 0 }\n',
                       '#[unsafe(export_name = "planted")]\npub fn f() {}\n',
                       '#[cfg_attr(all(), unsafe(no_mangle))]\n'
                       'pub fn planted() -> i32 { 0 }\n'):
            self.write("crates/example/tests/ui/planted.rs", source)
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("crates/example/tests/ui/planted.rs:1", output)

    def test_unterminated_attribute_is_refused(self):
        # D1, review pass 1. An attribute still open at end of file is a
        # lexically broken file. The old scanner returned without a refusal,
        # so every `unsafe` after an unclosed `#[` was hidden.
        for source in ("#[cfg(any())\npub unsafe fn planted() {}\n",
                       "#[derive(\npub fn g() { unsafe { 1 } }\n"):
            self.write("crates/example/tests/ui/planted.rs", source)
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("crates/example/tests/ui/planted.rs:1", output)
            self.assertIn("unterminated attribute", output)

    def test_weakening_attribute_with_space_or_comment_is_refused(self):
        # S1, review pass 1. `#`, an optional `!` and `[` are three tokens,
        # and Rust allows whitespace and comments between any two of them.
        # rustc 1.97.1 compiled `# [allow(unsafe_code)] mod m` holding an
        # unsafe block with warnings only, so the refusal must find the
        # spaced spellings and not only the tight one.
        for attribute in ("# [allow(unsafe_code)]\npub fn clean() {}\n",
                          "#! [allow(unsafe_code)]\npub fn clean() {}\n",
                          "#/**/[allow(unsafe_code)]\npub fn clean() {}\n"):
            self.write("crates/example/tests/ui/planted.rs", attribute)
            self.stage()
            code, output = self.run_guard()
            self.assertEqual(code, 1)
            self.assertIn("unsafe_code` lowered", output)

    def test_doc_text_is_comment_text(self):
        # Lead ruling after review pass 3. A doctest is refused by rustc,
        # through `#![doc(test(attr(forbid(unsafe_code))))]` at each member
        # library root, which scripts/lint_policy_check.py checks present.
        # The scanner does not read doc text, so `unsafe` in a doc comment
        # or a doc attribute's string, fenced or not, is not a finding here.
        self.write(
            "crates/example/src/lib.rs",
            "/// ```\n"
            "/// let _x = unsafe { core::ptr::read(&0u8) };\n"
            "/// ```\n"
            "//! unsafe fn f() {}\n"
            "/** ```\n"
            " * unsafe { 0 }\n"
            " * ``` */\n"
            "/*! ```\n"
            "unsafe { 0 }\n"
            "``` */\n"
            '#[doc = "```\\nlet _x = unsafe { 0 };\\n```"]\n'
            '#[doc = r#"unsafe impl Send for X {}"#]\n'
            "pub fn clean() {}\n")
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 0, output)

    def test_code_after_doc_text_is_still_read(self):
        # Doc comments are skipped whole, as comments, so the code after
        # one is code. A doc string holding a quote does not open a string
        # that blanks the planted line, and neither does a block doc
        # comment holding one.
        self.write(
            "crates/example/src/lib.rs",
            '/// say "\n'                                  # line 1
            '/** say " */\n'                              # line 2
            '#[doc = "say \\""]\n'                       # line 3
            "pub fn planted() { unsafe { 0 }; }\n"        # line 4, refused
            'pub const S: &str = "x";\n')
        self.stage()
        code, output = self.run_guard()
        self.assertEqual(code, 1)
        self.assertIn("crates/example/src/lib.rs:4", output)

if __name__ == "__main__":
    unittest.main()
