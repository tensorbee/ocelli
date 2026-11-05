"""SPDX Annex D and the licence readers.

The expressions are the ones Annex D's precedence decides. They are not
copied from a run of this checker.
"""

import importlib.util
import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
ALLOW = {
    "mit", "apache-2.0", "bsd-2-clause", "bsd-3-clause",
    "apache-2.0 with llvm-exception",
}


def load():
    spec = importlib.util.spec_from_file_location(
        "licence_check_under_test", ROOT / "licence_check.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


L = load()


class Spdx(unittest.TestCase):
    def test_and_binds_before_or(self) -> None:
        node = L.parse_expression("MIT OR Apache-2.0 AND GPL-3.0-only")
        self.assertEqual(node[0], "or")
        self.assertEqual(node[2][0], "and")
        self.assertTrue(L.expression_allowed(
            "MIT OR Apache-2.0 AND GPL-3.0-only", ALLOW))

    def test_parentheses_force_the_other_grouping(self) -> None:
        node = L.parse_expression("(MIT OR Apache-2.0) AND GPL-3.0-only")
        self.assertEqual(node[0], "and")
        self.assertEqual(node[1][0], "or")
        self.assertFalse(L.expression_allowed(
            "(MIT OR Apache-2.0) AND GPL-3.0-only", ALLOW))

    def test_with_is_a_whole_compound(self) -> None:
        self.assertTrue(L.expression_allowed(
            "Apache-2.0 WITH LLVM-exception", ALLOW))
        self.assertFalse(L.expression_allowed(
            "GPL-2.0-only WITH Classpath-exception-2.0", ALLOW))

    def test_identifiers_are_case_insensitive_and_operators_are_not(self) -> None:
        self.assertTrue(L.expression_allowed("mit", ALLOW))
        self.assertFalse(L.expression_allowed("MIT or Apache-2.0", ALLOW))

    def test_non_expressions_are_refused(self) -> None:
        for text in ("LicenseRef-x", "UNLICENSED", "SEE LICENSE IN x", ""):
            with self.subTest(text=text):
                self.assertFalse(L.expression_allowed(text, ALLOW))

    def test_a_disjunction_is_satisfied_by_one_permissive_term(self) -> None:
        self.assertTrue(L.expression_allowed(
            "MIT OR Apache-2.0 OR LGPL-2.1-or-later", ALLOW))

    def test_an_identifier_may_start_with_a_digit(self) -> None:
        allow = ALLOW | {"0bsd", "zlib", "cc0-1.0"}
        self.assertTrue(L.expression_allowed(
            "BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0", allow))


class CargoDenyPin(unittest.TestCase):
    def test_a_different_version_names_the_install_command(self) -> None:
        self._with_stub("#!/bin/sh\necho 'cargo-deny 0.1.0'\n", "0.19.0")

    def test_a_missing_binary_names_the_install_command(self) -> None:
        empty = tempfile.mkdtemp()
        with patch.dict(os.environ, {"PATH": empty}):
            _found, problems = L._cargo_deny_version()
        self.assertTrue(any(L.CARGO_DENY_INSTALL in problem for problem in problems))

    def _with_stub(self, script: str, needle: str) -> None:
        directory = tempfile.mkdtemp()
        cargo = Path(directory) / "cargo"
        cargo.write_text(script)
        cargo.chmod(0o755)
        with patch.dict(os.environ, {"PATH": directory}):
            _found, problems = L._cargo_deny_version()
        self.assertTrue(problems)
        self.assertTrue(any(needle in problem for problem in problems))
        self.assertTrue(any(L.CARGO_DENY_INSTALL in problem for problem in problems))


class Tables(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self._patch = patch.object(L, "ROOT", self.root)
        self._patch.start()
        self.addCleanup(self._patch.stop)
        (self.root / "deny.toml").write_text(
            '[licenses]\nallow = ["MIT", "Apache-2.0", "BSD-3-Clause"]\n')
        (self.root / "ci").mkdir()
        (self.root / "uv.lock").write_text(textwrap.dedent("""\
            version = 1
            [[package]]
            name = "pyyaml"
            version = "6.0.3"
            [[package]]
            name = "ocelli-tooling"
            version = "0.0.0"
            """))
        (self.root / "ci" / "licences.toml").write_text(textwrap.dedent("""\
            [[python]]
            package = "pyyaml"
            version = "6.0.3"
            spdx = "MIT"
            metadata = "MIT"

            [[exception]]
            ecosystem = "npm"
            package = "caniuse-lite"
            version = "1.0.0"
            licence = "CC-BY-4.0"
            reason = "data"
            scope = "dev-only"
            decided = "2026-11-05"
            decided_by = "operator"
            """))
        for rel in ("package-lock.json", "tools/oracle/package-lock.json",
                    "tools/bench/package-lock.json"):
            path = self.root / rel
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(json.dumps({"packages": {
                "": {"name": "root"},
                "node_modules/left-pad": {"version": "1.0.0", "license": "MIT"},
            }}))
        for folder, name in (("packages/core", "@ocelli/core"),
                             ("packages/react", "@ocelli/react")):
            path = self.root / folder
            path.mkdir(parents=True)
            (path / "package.json").write_text(
                '{"name": "' + name + '", "dependencies": {}}')
        subprocess.run(["git", "init", "-q"], cwd=self.root, check=True)
        subprocess.run(["git", "add", "-A"], cwd=self.root, check=True)

    def test_a_gpl_entry_is_refused_in_each_lockfile(self) -> None:
        for rel in ("package-lock.json", "tools/oracle/package-lock.json",
                    "tools/bench/package-lock.json"):
            path = self.root / rel
            data = json.loads(path.read_text())
            data["packages"]["node_modules/probe-copyleft"] = {
                "version": "0.0.1", "license": "GPL-3.0-only"}
            path.write_text(json.dumps(data))
        problems = L.npm_problems(L.allowlist())
        for rel in ("package-lock.json", "tools/oracle/package-lock.json",
                    "tools/bench/package-lock.json"):
            self.assertTrue(
                any(rel in problem and "GPL-3.0-only" in problem
                    for problem in problems),
                problems)

    def test_a_missing_licence_is_refused(self) -> None:
        path = self.root / "package-lock.json"
        data = json.loads(path.read_text())
        data["packages"]["node_modules/rechoir"] = {"version": "0.6.2"}
        path.write_text(json.dumps(data))
        problems = L.npm_problems(L.allowlist())
        self.assertTrue(any("has no licence and no exception" in p for p in problems))

    def test_an_exception_needs_every_field_and_a_known_scope(self) -> None:
        rows = [{"ecosystem": "npm", "package": "x", "version": "1",
                 "licence": "MIT", "scope": "dev-only"}]
        problems = L.exception_problems(rows)
        self.assertTrue(any("missing reason" in p for p in problems))
        self.assertTrue(any("missing decided" in p for p in problems))
        self.assertTrue(any("missing decided_by" in p for p in problems))

    def test_a_stale_exception_is_refused(self) -> None:
        problems = L.stale_problems()
        self.assertTrue(any("caniuse-lite@1.0.0" in p for p in problems))

    def _waiver(self, version: str) -> None:
        path = self.root / "ci" / "licences.toml"
        path.write_text(path.read_text() + textwrap.dedent(f"""
            [[licence_file]]
            package = "pyyaml"
            version = "{version}"
            path = "pyyaml/licenses/LICENSE"
            licence = "MIT"
            reason = "probe"
            scope = "tool"
            decided = "2026-11-05"
            decided_by = "lead"
            markers = ["GPLv2"]
            """))

    def test_a_waiver_version_absent_from_the_lock_is_stale(self) -> None:
        self._waiver("1999.0.0")
        problems = L.stale_problems()
        self.assertTrue(any(
            "stale licence file pyyaml@1999.0.0" in problem
            for problem in problems), problems)

    def test_a_lock_upgrade_makes_the_waiver_stale(self) -> None:
        self._waiver("6.0.3")
        self.assertFalse(any(
            "stale licence file" in problem for problem in L.stale_problems()))
        lock = self.root / "uv.lock"
        lock.write_text(lock.read_text().replace(
            'version = "6.0.3"', 'version = "6.0.4"', 1))
        problems = L.stale_problems()
        self.assertTrue(any(
            "stale licence file pyyaml@6.0.3" in problem
            for problem in problems), problems)

    def test_a_shipped_exception_is_refused(self) -> None:
        (self.root / "packages" / "core" / "package.json").write_text(
            '{"name": "@ocelli/core", "dependencies": {"dcmjs": "0.52.2"}}')
        (self.root / "packages" / "react" / "package.json").write_text(
            '{"name": "@ocelli/react", "dependencies": {"@ocelli/core": "0.1.0"}}')
        data = json.loads((self.root / "package-lock.json").read_text())
        data["packages"]["node_modules/dcmjs"] = {
            "version": "0.52.2", "license": "MIT", "dependencies": {}}
        data["packages"]["node_modules/@ocelli/core"] = {
            "version": "0.1.0", "license": "MIT", "link": True,
            "resolved": "packages/core"}
        (self.root / "package-lock.json").write_text(json.dumps(data))
        (self.root / "ci" / "licences.toml").write_text(textwrap.dedent("""\
            [[python]]
            package = "pyyaml"
            version = "6.0.3"
            spdx = "MIT"
            metadata = "MIT"

            [[exception]]
            ecosystem = "npm"
            package = "dcmjs"
            version = "0.52.2"
            licence = "MIT"
            reason = "probe"
            scope = "dev-only"
            decided = "2026-11-05"
            decided_by = "probe"
            """))
        subprocess.run(["git", "add", "-A"], cwd=self.root, check=True)
        problems = L.npm_problems(L.allowlist())
        self.assertTrue(any("published runtime closure" in p for p in problems))

    def _plant_caniuse(self, root_dev: bool, oracle: bool) -> None:
        root_lock = self.root / "package-lock.json"
        data = json.loads(root_lock.read_text())
        entry: dict = {"version": "1.0.0", "license": "CC-BY-4.0"}
        if root_dev:
            entry["dev"] = True
        data["packages"]["node_modules/caniuse-lite"] = entry
        root_lock.write_text(json.dumps(data))
        if oracle:
            path = self.root / "tools/oracle/package-lock.json"
            data = json.loads(path.read_text())
            data["packages"]["node_modules/caniuse-lite"] = {
                "version": "1.0.0", "license": "CC-BY-4.0"}
            path.write_text(json.dumps(data))

    def test_a_false_scope_is_refused(self) -> None:
        self._plant_caniuse(root_dev=True, oracle=True)
        problems = L.scope_problems()
        self.assertTrue(any(
            "tools/oracle/package-lock.json" in problem and "false" in problem
            for problem in problems), problems)
        text = (self.root / "ci" / "licences.toml").read_text().replace(
            'scope = "dev-only"', 'scope = "dev-only, tool"', 1)
        (self.root / "ci" / "licences.toml").write_text(text)
        self.assertEqual(L.scope_problems(), [])

    def test_root_dev_only_must_match_the_dev_flag(self) -> None:
        self._plant_caniuse(root_dev=False, oracle=False)
        problems = L.scope_problems()
        self.assertTrue(any(
            problem.endswith("is false for package-lock.json")
            for problem in problems), problems)
        self.assertFalse(any("tools/" in problem for problem in problems))

    def test_a_tools_lock_is_tooling_scope(self) -> None:
        (self.root / "ci" / "licences.toml").write_text(textwrap.dedent("""\
            [[python]]
            package = "pyyaml"
            version = "6.0.3"
            spdx = "MIT"
            metadata = "MIT"

            [[exception]]
            ecosystem = "npm"
            package = "rechoir"
            version = "0.6.2"
            licence = "MIT"
            reason = "legacy"
            scope = "dev-only"
            decided = "2026-11-05"
            decided_by = "operator"
            """))
        path = self.root / "tools/oracle/package-lock.json"
        data = json.loads(path.read_text())
        data["packages"]["node_modules/rechoir"] = {
            "version": "0.6.2", "license": "MIT"}
        path.write_text(json.dumps(data))
        problems = L.scope_problems()
        self.assertTrue(any(
            "rechoir" in problem and "false" in problem for problem in problems),
            problems)
        text = (self.root / "ci" / "licences.toml").read_text().replace(
            'scope = "dev-only"', 'scope = "tool"')
        (self.root / "ci" / "licences.toml").write_text(text)
        self.assertEqual(L.scope_problems(), [])

    def test_python_rows_must_cover_the_lock_and_be_allowed(self) -> None:
        problems = L.python_problems(L.allowlist())
        self.assertEqual(problems, [])
        text = (self.root / "ci" / "licences.toml").read_text().replace(
            'spdx = "MIT"', 'spdx = "GPL-3.0-only"', 1)
        (self.root / "ci" / "licences.toml").write_text(text)
        problems = L.python_problems(L.allowlist())
        self.assertTrue(any("GPL-3.0-only" in p for p in problems))
        (self.root / "ci" / "licences.toml").write_text("")
        problems = L.python_problems(L.allowlist())
        self.assertTrue(any("has no licence row" in p for p in problems))
        self.assertFalse(any("ocelli-tooling" in p for p in problems))


def _authorised(**extra: object) -> dict:
    row = {
        "licence": "MIT",
        "reason": "recorded",
        "scope": "tool",
        "decided": "2026-11-05",
        "decided_by": "lead",
    }
    row.update(extra)
    return row


class WheelBytes(unittest.TestCase):
    def test_empty_marker_and_library_rules(self) -> None:
        self.assertTrue(L.licence_bytes_problems("a/LICENSE", b"", None))
        self.assertEqual(
            L.licence_bytes_problems(
                "a/LICENSE", b"", _authorised(empty=True)), [])
        marked = L.licence_bytes_problems(
            "a/LICENSE", b"see GPLv2\n", None)
        self.assertTrue(any("copyleft marker" in p for p in marked))
        recorded = L.licence_bytes_problems(
            "a/LICENSE-lzf", b"see GPLv2\n",
            _authorised(markers=["GPLv2"], licence="BSD-2-Clause",
                        reason="the alternative is not taken",
                        spdx="GPL-3.0-only", note="unrelated"))
        self.assertEqual(recorded, [])
        # spdx and note can match while licence and reason do not. The check
        # reads the fields the waiver rule requires, so the matching pair
        # does not rescue the row.
        diverged = L.licence_bytes_problems(
            "a/LICENSE-lzf", b"see GPLv2\n",
            _authorised(markers=["GPLv2"], spdx="BSD-2-Clause",
                        note="the alternative is not taken",
                        licence="MIT",
                        reason="the alternative is not taken"))
        self.assertTrue(any("GPLv2" in problem for problem in diverged), diverged)
        silent_reason = L.licence_bytes_problems(
            "a/LICENSE-lzf", b"see GPLv2\n",
            _authorised(markers=["GPLv2"], licence="BSD-2-Clause",
                        reason="recorded", spdx="BSD-2-Clause",
                        note="the alternative is not taken"))
        self.assertTrue(any("GPLv2" in problem for problem in silent_reason),
                        silent_reason)

    def test_the_placeholder_licence_pairs_only_with_an_empty_file(self) -> None:
        empty = _authorised(
            package="imagecodecs", version="2026.10.10",
            path="imagecodecs/licenses/LICENSE-heif",
            empty=True, licence=L.PLACEHOLDER_LICENCE)
        self.assertEqual(L.licence_file_waiver_problems([empty]), [])
        claimed = dict(empty, licence="BSD-3-Clause")
        problems = L.licence_file_waiver_problems([claimed])
        self.assertTrue(any(
            "is valid only on an empty file" in problem for problem in problems),
            problems)
        marker = _authorised(
            package="imagecodecs", version="2026.10.10",
            path="imagecodecs/licenses/LICENSE-lzf",
            markers=["GPLv2"], licence=L.PLACEHOLDER_LICENCE)
        problems = L.licence_file_waiver_problems([marker])
        self.assertTrue(any(
            "is valid only on an empty file" in problem for problem in problems),
            problems)

    def test_a_lead_waiver_names_an_allowlisted_expression(self) -> None:
        base = _authorised(
            package="et-xmlfile", version="2.0.0",
            path="et_xmlfile-2.0.0.dist-info/LICENCE.python",
            markers=["GNU General Public License"], licence="Python-2.0")
        self.assertEqual(L.licence_file_waiver_problems([base]), [])
        copyleft = dict(base, licence="GPL-3.0-only")
        problems = L.licence_file_waiver_problems([copyleft])
        self.assertTrue(any(
            "allowlisted expression a lead may waive" in problem
            for problem in problems), problems)
        self.assertFalse(any(
            "parseable SPDX" in problem for problem in problems), problems)
        conjunction = dict(base, licence="MIT AND GPL-3.0-only")
        problems = L.licence_file_waiver_problems([conjunction])
        self.assertTrue(any(
            "allowlisted expression a lead may waive" in problem
            for problem in problems), problems)
        disjunction = dict(base, licence="MIT OR GPL-3.0-only")
        self.assertEqual(L.licence_file_waiver_problems([disjunction]), [])
        grouped = dict(base, licence="(MIT OR Apache-2.0) AND GPL-3.0-only")
        problems = L.licence_file_waiver_problems([grouped])
        self.assertTrue(any(
            "allowlisted expression a lead may waive" in problem
            for problem in problems), problems)
        other = dict(base, decided_by="reviewer", licence="GPL-3.0-only")
        problems = L.licence_file_waiver_problems([other])
        self.assertTrue(any(
            "allowlisted expression a lead may waive" in problem
            for problem in problems), problems)
        placeholder = _authorised(
            package="imagecodecs", version="2026.10.10",
            path="imagecodecs/licenses/LICENSE-heif",
            empty=True, licence=L.PLACEHOLDER_LICENCE)
        self.assertEqual(L.licence_file_waiver_problems([placeholder]), [])

    def test_an_operator_waiver_may_name_any_parseable_expression(self) -> None:
        base = _authorised(
            package="imagecodecs", version="2026.10.10",
            path="imagecodecs/licenses/LICENSE-lzf",
            markers=["GPLv2"], licence="GPL-3.0-only",
            decided_by="operator",
            reason="the alternative is not taken")
        self.assertEqual(L.licence_file_waiver_problems([base]), [])
        compound = dict(base, licence="GPL-3.0-or-later WITH GCC-exception-3.1")
        self.assertEqual(L.licence_file_waiver_problems([compound]), [])
        disjunction = dict(base, licence="MIT OR GPL-3.0-only")
        self.assertEqual(L.licence_file_waiver_problems([disjunction]), [])
        broken = dict(base, licence="SEE LICENSE IN FILE")
        problems = L.licence_file_waiver_problems([broken])
        self.assertTrue(any(
            "is not a parseable SPDX expression" in problem
            for problem in problems), problems)
        self.assertFalse(any(
            "a lead may waive" in problem for problem in problems), problems)
        lowercase = dict(base, licence="MIT or Apache-2.0")
        problems = L.licence_file_waiver_problems([lowercase])
        self.assertTrue(any(
            "is not a parseable SPDX expression" in problem
            for problem in problems), problems)
        placeholder = dict(
            base, markers=[], empty=True, licence=L.PLACEHOLDER_LICENCE)
        self.assertEqual(L.licence_file_waiver_problems([placeholder]), [])

    def test_a_bare_waiver_is_refused(self) -> None:
        row = {
            "package": "pydicom",
            "version": "3.0.2",
            "path": "pydicom-3.0.2.dist-info/licenses/LICENSE",
            "markers": ["GNU General Public License"],
        }
        problems = L.licence_file_waiver_problems([row])
        for field in L.EXCEPTION_FIELDS:
            self.assertTrue(
                any(f"missing {field}" in problem for problem in problems),
                problems)
        scanned = L.licence_bytes_problems(
            row["path"], b"GNU General Public License\n", row)
        self.assertTrue(any("copyleft marker" in problem for problem in scanned),
                        scanned)
        extra = L.bundle_list_problems(
            ["pkg/.dylibs/extra.dylib"], "darwin-arm64", set())
        self.assertTrue(any("not on the recorded list" in p for p in extra))
        self.assertEqual(
            L.bundle_list_problems(
                ["pkg/.dylibs/libz.dylib"], "darwin-arm64",
                {"pkg/.dylibs/libz.dylib"}),
            [])
        missing = L.bundle_list_problems(["pkg.libs/libz.so"], "freebsd-x86_64", None)
        self.assertTrue(any("no recorded bundled-library list" in p for p in missing))


class SyntheticWheel(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self._patch = patch.object(L, "ROOT", self.root)
        self._patch.start()
        self.addCleanup(self._patch.stop)
        (self.root / "uv.lock").write_text(textwrap.dedent("""\
            version = 1
            [[package]]
            name = "pyyaml"
            version = "6.0.3"
            """))
        (self.root / "ci").mkdir()
        (self.root / "ci" / "licences.toml").write_text(textwrap.dedent("""\
            [[python]]
            package = "pyyaml"
            version = "6.0.3"
            spdx = "MIT"
            metadata = "MIT"

            [bundle.darwin-arm64.pyyaml]
            libs = []
            """))
        self.site = self.root / "site-packages"
        self.dist = self.site / "pyyaml-6.0.3.dist-info"
        folder = self.dist / "licenses"
        folder.mkdir(parents=True)
        (self.dist / "METADATA").write_text(
            "Metadata-Version: 2.1\nName: pyyaml\nVersion: 6.0.3\nLicense: MIT\n")
        (folder / "LICENSE").write_text("MIT License\n")

    def test_a_clean_tree_is_quiet(self) -> None:
        self.assertEqual(L.installed_problems(self.site, "darwin", "arm64"), [])

    def test_a_marker_and_a_bundled_library_are_refused(self) -> None:
        (self.dist / "licenses" / "LICENSE").write_text(
            "GNU General Public License\n")
        libs = self.site / "pyyaml.libs"
        libs.mkdir()
        (libs / "libextra.so").write_bytes(b"x")
        problems = L.installed_problems(self.site, "darwin", "arm64")
        self.assertTrue(any("copyleft marker" in p for p in problems), problems)
        self.assertTrue(
            any("not on the recorded list" in p for p in problems), problems)

    def test_a_waiver_applies_only_at_its_own_version(self) -> None:
        (self.dist / "licenses" / "LICENSE").write_text("see GPLv2\n")
        (self.root / "ci" / "licences.toml").write_text(textwrap.dedent("""\
            [[python]]
            package = "pyyaml"
            version = "6.0.3"
            spdx = "MIT"
            metadata = "MIT"

            [[licence_file]]
            package = "pyyaml"
            version = "6.0.3"
            path = "pyyaml-6.0.3.dist-info/licenses/LICENSE"
            markers = ["GPLv2"]
            spdx = "BSD-2-Clause"
            licence = "BSD-2-Clause"
            reason = "the alternative is not taken"
            scope = "tool"
            decided = "2026-11-05"
            decided_by = "lead"
            note = "a different sentence"

            [bundle.darwin-arm64.pyyaml]
            libs = []
            """))
        self.assertEqual(L.installed_problems(self.site, "darwin", "arm64"), [])
        text = (self.root / "ci" / "licences.toml").read_text().replace(
            'version = "6.0.3"\npath =', 'version = "1999.0.0"\npath =', 1)
        (self.root / "ci" / "licences.toml").write_text(text)
        problems = L.installed_problems(self.site, "darwin", "arm64")
        self.assertTrue(any("copyleft marker" in problem for problem in problems),
                        problems)

    def test_a_dist_info_root_licence_is_scanned(self) -> None:
        (self.dist / "LICENCE.rst").write_text(
            "GNU General Public License version 3\n")
        problems = L.installed_problems(self.site, "darwin", "arm64")
        self.assertTrue(any(
            "LICENCE.rst" in problem and "copyleft marker" in problem
            for problem in problems), problems)


class WorkspaceLicence(unittest.TestCase):
    def test_a_comment_does_not_inherit_the_workspace_licence(self) -> None:
        self.assertFalse(L.inherits_workspace_licence(
            '[package]\nlicense = "MIT" # was license.workspace = true\n'))
        self.assertTrue(L.inherits_workspace_licence(
            "[package]\nlicense.workspace = true\n"))


class RealTree(unittest.TestCase):
    def test_notices_and_holder_match_the_tree(self) -> None:
        self.assertEqual(L.notices_problems(), [])
        self.assertEqual(L.holder_problems(), [])
        self.assertEqual(L.scope_problems(), [])
        self.assertEqual(L.licence_file_waiver_problems(), [])
        self.assertEqual(L.stale_problems(), [])
        self.assertEqual(L.python_problems(L.allowlist()), [])
        locked = dict(L.uv_packages())
        for row in L.licence_file_rows():
            self.assertEqual(
                locked.get(str(row.get("package"))), str(row.get("version")),
                row)
