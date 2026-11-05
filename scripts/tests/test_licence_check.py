"""SPDX Annex D and the licence readers.

The expressions are the ones Annex D's precedence decides. They are not
copied from a run of this checker.
"""

import hashlib
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
REPO = ROOT.parent
ALLOW = {
    "mit", "apache-2.0", "bsd-2-clause", "bsd-3-clause",
    "apache-2.0 with llvm-exception",
}


def load():
    spec = importlib.util.spec_from_file_location(
        "licence_check_under_test", ROOT / "licence_check.py")
    module = importlib.util.module_from_spec(spec)
    # dataclass looks the class module up in sys.modules while the class
    # body runs. Register the module before executing it.
    sys.modules[spec.name] = module
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
        # A lead may not record text that offers a copyleft licence even as
        # an alternative. That is the operator's decision, as lzf is.
        disjunction = dict(base, licence="MIT OR GPL-3.0-only")
        problems = L.licence_file_waiver_problems([disjunction])
        self.assertTrue(any(
            "allowlisted expression a lead may waive" in problem
            for problem in problems), problems)
        permissive = dict(base, licence="MIT OR Apache-2.0")
        self.assertEqual(L.licence_file_waiver_problems([permissive]), [])
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


# The four licence names and the two excluded names are the plan's file rule.
# The elected texts are the plan's left-most allowlisted disjunct, not a run
# of the generator.
_PIN_TEXTS = {
    "mit": b"MIT-STANDARD\nCopyright (c) <copyright-holder>\n",
    "apache-2.0": (
        b"APACHE-STANDARD\nCopyright [yyyy] [name of copyright owner]\n"),
    "zlib": b"ZLIB-STANDARD\nCopyright (c) <copyright-holder>\n",
}
_PIN_ALLOW = {"mit", "apache-2.0", "zlib", "bsd-3-clause"}
_NO_FILE = (
    "The published crate carries no licence file. "
    "The text below is the standard text of the licence it declares."
)
_STD_BEGIN = b"-----BEGIN RUST STANDARD LIBRARY NOTICE-----\n"
_STD_END = b"-----END RUST STANDARD LIBRARY NOTICE-----"
_STDLIB_SHA256 = (
    "0a65bb747c49c7bb816cbc7188319bd6e4e8d08091c1190b8a3c0971c47968ed"
)


def _notice(name, version, expression, files=(), authors=("Ann Author <ann@example.com>",),
           repository="", notice="packaged", earlier=(), commit="",
           pin_repository="", earlier_commit="", earlier_repository=""):
    return L.CrateNotice(
        name=name,
        version=version,
        source="crates.io",
        expression=expression,
        authors=authors,
        files=files,
        repository=repository,
        notice=notice,
        earlier=earlier,
        commit=commit,
        pin_repository=pin_repository,
        earlier_commit=earlier_commit,
        earlier_repository=earlier_repository,
    )


def _render(crates, std=b"<std-notice>"):
    return L.render_notices(list(crates), _PIN_TEXTS, std, "1.97.1", _PIN_ALLOW)


class Binaries(unittest.TestCase):
    def test_binaries_collects_licence_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            crate = Path(tmp)
            (crate / "LICENSE-MIT").write_bytes(b"mit\n")
            (crate / "license-apache-2.0").write_bytes(b"apache\n")
            (crate / "NOTICE").write_bytes(b"notice\n")
            (crate / "LICENSES").mkdir()
            (crate / "LICENSES" / "Zlib.txt").write_bytes(b"zlib\n")
            (crate / "README.md").write_text("readme\n", encoding="utf-8")
            (crate / "src").mkdir()
            (crate / "src" / "lib.rs").write_text("fn main() {}\n", encoding="utf-8")
            found = [name for name, _blob in L.collect_licence_files(crate, None)]
        self.assertEqual(
            set(found),
            {"LICENSE-MIT", "license-apache-2.0", "NOTICE", "LICENSES/Zlib.txt"},
        )
        self.assertNotIn("README.md", found)
        self.assertNotIn("src/lib.rs", found)

    def test_binaries_elects_the_left_most_allowlisted_disjunct(self) -> None:
        zlib = _render([_notice("widget", "1.2.3", "Zlib OR Apache-2.0 OR MIT")])
        self.assertIn(b"ZLIB-STANDARD\n", zlib)
        self.assertNotIn(b"APACHE-STANDARD", zlib)
        self.assertNotIn(b"MIT-STANDARD", zlib)
        self.assertIn(b"authors: Ann Author <ann@example.com>", zlib)
        self.assertIn(_NO_FILE.encode(), zlib)

        legacy = _render([_notice("widget", "1.2.3", "MIT/Apache-2.0")])
        self.assertIn(b"MIT-STANDARD\n", legacy)
        self.assertNotIn(b"APACHE-STANDARD", legacy)

        chosen = _render([_notice("widget", "1.2.3", "GPL-3.0-only OR MIT")])
        self.assertIn(b"MIT-STANDARD\n", chosen)
        self.assertIn(b"expression: GPL-3.0-only OR MIT\n", chosen)
        self.assertNotIn(b"carrier: widget 1.2.3 GPL-3.0-only", chosen)

    def test_binaries_refuses_an_identifier_with_no_pinned_text(self) -> None:
        with self.assertRaises(L.BinariesError) as caught:
            _render([_notice("widget", "1.2.3", "BSD-3-Clause")])
        message = str(caught.exception)
        self.assertIn("widget", message)
        self.assertIn("1.2.3", message)
        self.assertIn("BSD-3-Clause", message)

    def test_a_shipped_file_is_kept_and_line_endings_become_lf(self) -> None:
        # CRLF and a lone CR each become one LF. No other byte changes.
        raw = b"line-a\r\nline-b\rline-c\n"
        blob = _render([_notice(
            "widget", "1.2.3", "MIT", files=(("LICENSE", raw),))])
        self.assertIn(b"line-a\nline-b\nline-c\n", blob)
        self.assertNotIn(b"\r", blob)
        self.assertNotIn(b"MIT-STANDARD", blob)
        self.assertNotIn(_NO_FILE.encode(), blob)

    def test_identical_texts_are_emitted_once_in_index_order(self) -> None:
        later = _notice("b", "1.0.0", "MIT", files=(("LICENSE", b"same\r\n"),))
        earlier = _notice("a", "1.0.0", "MIT", files=(("LICENSE", b"same\n"),))
        blob = _render([later, earlier])
        self.assertEqual(blob.count(b"same\n"), 1)
        self.assertLess(blob.find(b"name: a\n"), blob.find(b"name: b\n"))
        self.assertLess(blob.find(b"carrier: a 1.0.0 LICENSE"),
                        blob.find(b"carrier: b 1.0.0 LICENSE"))

    def test_a_version_bump_changes_the_output(self) -> None:
        first = _render([_notice("widget", "1.2.3", "MIT")])
        second = _render([_notice("widget", "1.2.4", "MIT")])
        self.assertNotEqual(first, second)

    def test_two_renders_match_and_leak_no_local_path(self) -> None:
        first = _render([_notice("widget", "1.2.3", "MIT")])
        second = _render([_notice("widget", "1.2.3", "MIT")])
        self.assertEqual(first, second)
        self.assertNotIn(b"/Users/", first)
        self.assertNotIn(b"/home/", first)
        self.assertNotIn(b".cargo/registry", first)
        self.assertNotIn(b"\r", first)

    def test_invalid_utf8_is_refused_naming_the_file(self) -> None:
        with self.assertRaises(L.BinariesError) as caught:
            _render([_notice(
                "widget", "1.2.3", "MIT", files=(("LICENSE", b"\xff"),))])
        self.assertIn("LICENSE", str(caught.exception))

    def test_a_workspace_member_is_absent_from_the_render(self) -> None:
        member = _notice("ocelli-core", "0.1.0", "MIT")
        other = _notice("widget", "1.2.3", "MIT")
        blob = L.render_notices(
            [member, other], _PIN_TEXTS, b"<std-notice>", "1.97.1",
            _PIN_ALLOW, skip_names={"ocelli-core"})
        text = blob.decode("utf-8")
        self.assertNotIn("ocelli-core", text)
        self.assertNotIn("Tensorbee", text)
        self.assertIn("name: widget\n", text)

    def test_the_standard_library_section_is_the_pinned_bytes(self) -> None:
        pinned = (REPO / "ci/notices/rust-std/COPYRIGHT-library.html").read_bytes()
        self.assertEqual(hashlib.sha256(pinned).hexdigest(), _STDLIB_SHA256)
        blob = _render([_notice("widget", "1.2.3", "MIT")], std=pinned)
        start = blob.find(_STD_BEGIN)
        stop = blob.find(_STD_END)
        self.assertGreaterEqual(start, 0)
        self.assertGreater(stop, start)
        self.assertEqual(blob[start + len(_STD_BEGIN):stop], pinned)

    def test_a_crates_io_source_drops_the_registry_path(self) -> None:
        self.assertEqual(L.source_label({
            "name": "adler2",
            "source": "registry+https://github.com/rust-lang/crates.io-index",
            "manifest_path": "/Users/someone/.cargo/registry/src/adler2-1.0.0/Cargo.toml",
        }), "crates.io")

    def test_a_path_inside_the_repository_is_relative(self) -> None:
        manifest = REPO / "vendor/openjph-core-0.1.0/Cargo.toml"
        self.assertEqual(L.source_label({
            "name": "openjph-core",
            "source": None,
            "manifest_path": str(manifest),
        }), "vendor/openjph-core-0.1.0")

    def test_a_path_outside_the_repository_is_refused(self) -> None:
        with self.assertRaises(L.BinariesError) as caught:
            L.source_label({
                "name": "elsewhere",
                "source": None,
                "manifest_path": "/tmp/elsewhere/Cargo.toml",
            })
        self.assertIn("elsewhere", str(caught.exception))

    def test_closure_lists_each_shipped_crate_once(self) -> None:
        records, members = L._rust_shipped()
        third = [package for ident, package in records.items()
                 if ident not in members]
        self.assertEqual(len(third), 129)
        blob = L.generate_third_party_licenses()
        text = blob.decode("utf-8")
        names = []
        versions = []
        for line in text.splitlines():
            if line.startswith("name: "):
                names.append(line.removeprefix("name: "))
            elif line.startswith("version: "):
                versions.append(line.removeprefix("version: "))
        got = list(zip(names, versions, strict=True))
        expected = sorted(
            (str(package["name"]), str(package["version"]))
            for package in third)
        self.assertEqual(got, expected)
        self.assertNotIn("proptest", names)
        self.assertNotIn("Tensorbee", text)
        # cargo metadata lists every workspace member. The closure record map
        # holds only the crates the two entry points reach.
        workspace = {
            str(records[ident]["name"])
            for ident in members if ident in records
        }
        self.assertTrue(workspace)
        for name in workspace:
            self.assertNotIn(f"name: {name}\n", text)
        self.assertFalse(any(name.startswith("ocelli-") for name in names))
        self.assertNotIn(b"/Users/", blob)
        self.assertNotIn(b"/home/", blob)
        self.assertNotIn(b".cargo/registry", blob)
        self.assertNotIn(b"\r", blob)

    def test_tracked_file_matches_regeneration(self) -> None:
        self.assertEqual(L.binaries_problems(), [])

    def test_a_changed_pin_byte_is_refused_by_the_binaries_gate(self) -> None:
        # The binaries entry point, not pinned_digest_problems on its own.
        # A template the closure does not embed still fails here.
        path = REPO / "ci/notices/spdx/MIT.txt"
        original = path.read_bytes()
        try:
            path.write_bytes(original + b"\n")
            problems = L.binaries_problems()
        finally:
            path.write_bytes(original)
        self.assertTrue(any(
            "ci/notices/spdx/MIT.txt" in problem and "sha256" in problem
            for problem in problems), problems)
        self.assertEqual(L.pinned_digest_problems(), [])

    def test_a_template_that_names_another_holder_is_refused(self) -> None:
        foreign = (
            b"Copyright (C) 1995-2022 Jean-loup Gailly and Mark Adler\n"
            b"jloup@gzip.org\n"
            b"Permission is granted.\n"
        )
        pins = dict(_PIN_TEXTS)
        pins["zlib"] = foreign
        with self.assertRaises(L.BinariesError) as caught:
            L.render_notices(
                [_notice("widget", "1.2.3", "Zlib")], pins, b"<std>",
                "1.97.1", _PIN_ALLOW)
        self.assertIn("names another project's holder", str(caught.exception))
        self.assertIn("zlib", str(caught.exception))

    def test_an_unfilled_placeholder_in_the_output_is_refused(self) -> None:
        pins = dict(_PIN_TEXTS)
        pins["mit"] = (
            b"Copyright (c) <copyright-holder>\n"
            b"See [year] [fullname] below.\n"
        )
        with self.assertRaises(L.BinariesError) as caught:
            L.render_notices(
                [_notice("widget", "1.2.3", "MIT")], pins, b"<std>",
                "1.97.1", _PIN_ALLOW)
        self.assertIn("unfilled placeholder", str(caught.exception))

    def test_a_template_fallback_names_declared_authors_or_the_crate(self) -> None:
        named = _render([_notice("widget", "1.2.3", "MIT")])
        self.assertIn(b"Copyright (c) Ann Author <ann@example.com>", named)
        self.assertNotIn(b"<copyright-holder>", named)
        self.assertNotIn(b"[year]", named)
        self.assertNotIn(b"[fullname]", named)
        apache = _render([_notice("widget", "1.2.3", "Apache-2.0")])
        self.assertIn(b"authors: Ann Author <ann@example.com>", apache)
        self.assertIn(b"Copyright [yyyy] [name of copyright owner]", apache)
        self.assertNotIn(b"Copyright (c) Ann Author", apache)
        self.assertIn(_NO_FILE.encode(), apache)
        published = (REPO / "ci/notices/spdx/Apache-2.0.txt").read_bytes()
        self.assertEqual(
            L.fill_copyright(published, "Ann Author <ann@example.com>"),
            published)
        empty = _render([_notice(
            "widget", "1.2.3", "Zlib", authors=(),
            repository="https://example.com/widget")])
        self.assertIn(
            b"Copyright (c) the widget authors (https://example.com/widget)",
            empty)
        self.assertNotIn(b"<copyright-holder>", empty)

    def test_a_no_file_crate_without_a_pin_or_fallback_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text(
                "[package]\nname = \"widget-not-pinned\"\n", encoding="utf-8")
            with self.assertRaises(L.BinariesError) as caught:
                L.crate_from_package({
                    "name": "widget-not-pinned",
                    "version": "9.9.9",
                    "license": "MIT",
                    "authors": [],
                    "repository": "https://example.com/widget",
                    "manifest_path": str(root / "Cargo.toml"),
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                })
        self.assertIn("widget-not-pinned", str(caught.exception))
        self.assertIn("no pinned upstream notice", str(caught.exception))
        self.assertIn("no recorded fallback", str(caught.exception))

    def test_a_recorded_fallback_uses_the_template(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text("x\n", encoding="utf-8")
            package = {
                "name": "widget-fallback",
                "version": "1.2.3",
                "license": "MIT",
                "authors": [],
                "repository": "https://example.com/widget",
                "manifest_path": str(root / "Cargo.toml"),
                "source": "registry+https://github.com/rust-lang/crates.io-index",
            }
            with patch.object(
                    L, "fallback_reasons",
                    return_value={("widget-fallback", "1.2.3"):
                                  "upstream repository carries no licence file"}):
                notice = L.crate_from_package(package)
        self.assertEqual(notice.notice, "template")
        self.assertEqual(notice.files, ())
        blob = L.render_notices(
            [notice], _PIN_TEXTS, b"<std>", "1.97.1", _PIN_ALLOW)
        self.assertIn(
            b"the widget-fallback authors (https://example.com/widget)", blob)
        self.assertIn(L._NO_FILE_SENTENCE.encode(), blob)

    def test_upstream_notice_is_the_pinned_file(self) -> None:
        notice = _notice(
            "widget", "1.2.3", "MIT",
            files=(("LICENSE-MIT.txt", b"Copyright 2026 Mads Marquart\n"),),
            authors=(), notice="upstream")
        blob = _render([notice])
        self.assertIn(b"Copyright 2026 Mads Marquart\n", blob)
        self.assertIn(L._UPSTREAM_SENTENCE.encode(), blob)
        self.assertNotIn(b"MIT-STANDARD", blob)
        self.assertNotIn(b"Jean-loup", blob)

    def test_a_statement_notice_is_the_file_then_the_template(self) -> None:
        repo = "https://github.com/acme/widget"
        published = "a" * 40
        plain = _notice(
            "widget", "1.2.3", "Zlib",
            files=(("LICENSE.md", b"statement only\n"),),
            authors=("Mads Marquart <mads@marquart.dk>",),
            notice="statement",
            pin_repository=repo,
            commit=published)
        plain_blob = _render([plain]).decode("utf-8")
        plain_sentence = L.statement_sentence(repo, published, "LICENSE.md")
        self.assertIn(plain_sentence, plain_blob)
        self.assertIn(repo, plain_blob)
        self.assertIn(published, plain_blob)
        self.assertNotIn("at that commit", plain_blob)
        self.assertNotIn("licence file from its repository", plain_blob)
        self.assertLess(
            plain_blob.find("statement only"),
            plain_blob.find("Copyright (c) Mads Marquart <mads@marquart.dk>"))
        earlier_commit = "b" * 40
        earlier = _notice(
            "widget", "1.2.3", "MIT",
            files=(("LICENSE.md", b"statement only\n"),),
            authors=(),
            repository="https://example.com/widget",
            notice="statement",
            pin_repository=repo,
            commit=published,
            earlier=(("LICENSE.txt", b"Copyright (c) Steven Sheldon\n"),),
            earlier_repository=repo,
            earlier_commit=earlier_commit)
        text = _render([earlier]).decode("utf-8")
        earlier_sentence = L.statement_sentence(
            repo, published, "LICENSE.md", repo, earlier_commit, "LICENSE.txt")
        self.assertIn(earlier_sentence, text)
        self.assertIn(earlier_commit, text)
        self.assertNotIn("cfb19922", text)
        self.assertNotIn("at that commit", text)
        holder = text.find(
            "Copyright (c) the widget authors (https://example.com/widget)")
        self.assertLess(text.find("statement only"), holder)
        self.assertLess(holder, text.find("Copyright (c) Steven Sheldon"))
        self.assertNotIn("Copyright 2026", text)
        self.assertNotIn("licence file from its repository", text)
        with self.assertRaises(L.BinariesError) as caught:
            _render([_notice(
                "widget", "1.2.3", "MIT",
                files=(("LICENSE.md", b"statement only\n"),),
                notice="statement")])
        self.assertIn("statement names no repository or commit",
                      str(caught.exception))

    def test_objc2_crates_name_their_holder(self) -> None:
        self.assertNotIn("not a grant",
                         (REPO / "ci/notices/sources.toml").read_text(
                             encoding="utf-8"))
        reasons = L.fallback_reasons()
        objc2 = (
            ("block2", "0.6.2", True),
            ("objc2", "0.6.4", True),
            ("objc2-encode", "4.1.0", True),
            ("objc2-foundation", "0.3.2", True),
            ("dispatch2", "0.3.1", False),
            ("objc2-core-foundation", "0.3.2", False),
            ("objc2-core-graphics", "0.3.2", False),
            ("objc2-io-surface", "0.3.2", False),
            ("objc2-metal", "0.3.2", False),
            ("objc2-quartz-core", "0.3.2", False),
        )
        for name, version, _mit in objc2:
            self.assertIn("no licence text", reasons[(name, version)])
        earlier = {
            (str(row["name"]), str(row["version"]))
            for row in L._table_rows(L._load_sources(), "earlier", required=False)
        }
        pins = {
            (str(row["name"]), str(row["version"])): row
            for row in L.crate_pin_rows()
        }
        earlier_pins = {
            (str(row["name"]), str(row["version"])): row
            for row in L._table_rows(
                L._load_sources(), "earlier", required=False)
        }
        blob = L.generate_third_party_licenses()
        text = blob.decode("utf-8")
        lines = text.splitlines()
        self.assertNotIn("cfb19922", text)
        self.assertNotIn("at that commit", text)
        for name, version, mit in objc2:
            needle = f"carrier: {name} {version} LICENSE.md"
            self.assertIn(needle, lines)
            window = "\n".join(lines[lines.index(needle):lines.index(needle) + 4])
            repo, commit, _path = L.raw_pin(str(pins[(name, version)]["url"]))
            self.assertIn(repo, window)
            self.assertIn(commit, window)
            self.assertNotIn("licence file from its repository", window)
            self.assertNotIn("at that commit", window)
            if mit:
                self.assertIn((name, version), earlier)
                earlier_row = earlier_pins[(name, version)]
                earlier_repo, earlier_commit, _earlier_path = L.raw_pin(
                    str(earlier_row["url"]))
                self.assertEqual(
                    earlier_commit, str(earlier_row["commit"]).lower())
                self.assertIn(earlier_repo, window)
                self.assertIn(earlier_commit, window)
                self.assertIn("LICENSE.txt", window)
            else:
                self.assertNotIn((name, version), earlier)
                self.assertNotIn("LICENSE.txt", window)
        self.assertIn("derived from Apple SDKs", text)
        self.assertIn(
            "Copyright (c) Mads Marquart <mads@marquart.dk>", text)
        self.assertIn(
            "Copyright (c) Mads Marquart <mads@marquart.dk>, "
            "Mary <mary@mary.zone>", text)
        self.assertIn(
            "Copyright (c) the objc2-foundation authors "
            "(https://github.com/madsmtm/objc2)", text)
        self.assertIn(
            "Copyright (c) the objc2-metal authors "
            "(https://github.com/madsmtm/objc2)", text)
        self.assertIn("Copyright (c) Steven Sheldon", text)
        self.assertNotIn("Copyright 2026 Mads Marquart", text)
        self.assertNotIn("Jean-loup Gailly", text)
        self.assertNotIn("[fullname]", text)
        self.assertNotIn("[year]", text)
        self.assertNotIn("<copyright-holder>", text)

    def test_a_pin_from_another_commit_is_refused(self) -> None:
        published = "a" * 40
        other = "b" * 40
        rows = [{
            "name": "widget",
            "version": "1.0.0",
            "url": f"https://raw.githubusercontent.com/acme/widget/{other}/LICENSE",
        }]
        problems = L.published_commit_problems(
            rows, sha1_of=lambda _name, _version: published)
        self.assertEqual(problems, [
            "widget 1.0.0 pin URL commit "
            f"{other} differs from the published commit {published}"])
        self.assertEqual(L.published_commit_problems(), [])

    def test_a_missing_published_commit_is_refused_offline(self) -> None:
        rows = [{
            "name": "widget",
            "version": "1.0.0",
            "url": "https://raw.githubusercontent.com/acme/widget/"
                   + ("c" * 40) + "/LICENSE",
        }]
        with patch.object(L, "published_sha1", return_value=None):
            problems = L.published_commit_problems(rows)
        self.assertEqual(len(problems), 1)
        self.assertIn("differs from the published commit absent", problems[0])

    def test_binaries_gate_reads_the_published_commit(self) -> None:
        message = (
            "widget 1.0.0 pin URL commit " + ("b" * 40)
            + " differs from the published commit " + ("a" * 40))
        with patch.object(
                L, "published_commit_problems", return_value=[message]):
            problems = L.binaries_problems()
        self.assertTrue(
            any("differs from the published commit" in item for item in problems),
            problems)

    def test_the_attribution_paragraph_is_in_the_header(self) -> None:
        blob = _render([_notice("widget", "1.2.3", "MIT")])
        text = blob.decode("utf-8")
        paragraph = next(
            line for line in text.splitlines()
            if "Creative Commons Attribution 3.0 Unported" in line)
        for phrase in L._CC_BY_REQUIRED:
            self.assertIn(phrase, paragraph)
        self.assertNotIn("zlib", paragraph.casefold())
        self.assertLess(
            text.find(paragraph), text.find("## Crate index"))
        self.assertEqual(L.attribution_problems(blob), [])
        stripped = blob.replace(L._CC_BY_ATTRIBUTION.encode(), b"")
        self.assertEqual(
            L.attribution_problems(stripped),
            ["CC BY attribution paragraph is absent"])
        with patch.object(L, "render_closure", return_value=stripped):
            problems = L.binaries_problems()
        self.assertTrue(
            any("attribution paragraph is absent" in item for item in problems),
            problems)

    def test_an_earlier_pin_must_match_the_commit_it_names(self) -> None:
        claimed = "c" * 40
        other = "d" * 40
        digest = "ab" * 32
        row = {
            "name": "widget",
            "version": "1.0.0",
            "path": "ci/notices/crates/objc2-earlier/LICENSE.txt",
            "url": "https://raw.githubusercontent.com/acme/widget/"
                   + other + "/LICENSE.txt",
            "sha256": digest,
            "commit": claimed,
        }
        problems = L.earlier_commit_problems(
            [row], blob_at=lambda _url: (_ for _ in ()).throw(
                AssertionError("fetched a commit the row does not claim")))
        self.assertEqual(problems, [
            "widget 1.0.0 earlier pin URL commit "
            f"{other} differs from the commit the row claims {claimed}"])

        absent = dict(row, url=(
            "https://raw.githubusercontent.com/acme/widget/"
            + claimed + "/LICENSE.txt"), commit=claimed)
        problems = L.earlier_commit_problems(
            [absent], blob_at=lambda _url: None)
        self.assertEqual(problems, [
            f"widget 1.0.0 LICENSE.txt is absent at commit {claimed}"])

        problems = L.earlier_commit_problems(
            [absent], blob_at=lambda _url: b"not-the-pin\n")
        self.assertEqual(len(problems), 1)
        self.assertIn("sha256 is", problems[0])
        self.assertIn(claimed, problems[0])
        self.assertIn(digest, problems[0])

        pinned = (REPO / "ci/notices/crates/objc2-earlier/LICENSE.txt").read_bytes()
        real_digest = hashlib.sha256(pinned).hexdigest()
        good = dict(absent, sha256=real_digest)
        self.assertEqual(
            L.earlier_commit_problems([good], blob_at=lambda _url: pinned),
            [])

    def test_binaries_gate_reads_the_earlier_commit(self) -> None:
        message = "block2 0.6.2 LICENSE.txt is absent at commit " + ("e" * 40)
        with patch.object(L, "earlier_commit_problems", return_value=[message]):
            problems = L.binaries_problems()
        self.assertTrue(
            any("is absent at commit" in item for item in problems),
            problems)

    def test_the_binaries_gate_does_not_fetch(self) -> None:
        def explode(*_args, **_kwargs):
            raise AssertionError("the binaries gate fetched a pin")

        self.assertNotIn("verify-upstream", L.FLOOR_SECTIONS)
        with patch.object(L, "fetch_raw_pin", side_effect=explode), \
                patch.object(L.urllib.request, "urlopen", side_effect=explode):
            self.assertEqual(L.binaries_problems(), [])

    def test_verify_upstream_refuses_a_mismatched_pin(self) -> None:
        def local(url: str) -> bytes:
            tables = list(L.crate_pin_rows())
            tables.extend(L._table_rows(
                L._load_sources(), "earlier", required=False))
            for row in tables:
                if str(row.get("url") or "") == url:
                    return (L.ROOT / str(row["path"])).read_bytes()
            raise AssertionError(f"no pin for {url}")

        with patch.object(L, "fetch_raw_pin", side_effect=local):
            self.assertEqual(L.verify_upstream_problems(), [])

        def mismatched(url: str):
            if url.endswith("/LICENSE.txt"):
                return None
            return b"not-the-pin\n"

        with patch.object(L, "fetch_raw_pin", side_effect=mismatched):
            buffer = io.StringIO()
            with patch("sys.stdout", buffer):
                code = L.main(["verify-upstream"])
        text = buffer.getvalue()
        self.assertEqual(code, 1)
        self.assertIn("FAIL: licence check", text)
        self.assertIn("sha256 is", text)
        self.assertIn("LICENSE.txt is absent at commit", text)

    def test_rust_std_pin_rewrite_keeps_every_file_row(self) -> None:
        original = (REPO / "ci/notices/sources.toml").read_text(encoding="utf-8")
        rewritten = L.rewrite_rust_std_pin(original, "1.93.0", "ab" * 32)
        data = tomllib.loads(rewritten)
        self.assertNotIn("identifier", data)
        idents = [row["identifier"] for row in data["file"]]
        self.assertEqual(idents, ["MIT", "Apache-2.0", "Zlib", "rust-std"])
        self.assertEqual(data["file"][-1]["toolchain"], "1.93.0")
        self.assertEqual(data["file"][-1]["sha256"], "ab" * 32)
        before = tomllib.loads(original)
        self.assertEqual(
            [row["sha256"] for row in data["crate"]],
            [row["sha256"] for row in before["crate"]])
        self.assertEqual(data["file"][0]["sha256"], before["file"][0]["sha256"])
        self.assertEqual(rewritten.count("[[file]]"), original.count("[[file]]"))
        self.assertEqual(rewritten.count("[[crate]]"), original.count("[[crate]]"))
        self.assertEqual(
            rewritten.count("[[fallback]]"), original.count("[[fallback]]"))
        self.assertEqual(
            rewritten.count("[[earlier]]"), original.count("[[earlier]]"))

    def test_write_after_a_channel_change_round_trips_sources(self) -> None:
        # The documented route: change the channel, then --write binaries.
        with tempfile.TemporaryDirectory(prefix="ocelli-f220-") as tmp:
            dest = Path(tmp)
            _copy_worktree(dest)
            sources = dest / "ci/notices/sources.toml"
            html = dest / "ci/notices/rust-std/COPYRIGHT-library.html"
            licences = dest / "THIRD_PARTY_LICENSES"
            before = (sources.read_bytes(), html.read_bytes(), licences.read_bytes())
            toolchain = dest / "rust-toolchain.toml"
            toolchain.write_text(
                toolchain.read_text(encoding="utf-8").replace(
                    'channel = "1.97.1"', 'channel = "1.93.0"', 1),
                encoding="utf-8")
            victim = dest / "ci/notices/crates/block2-0.6.2/LICENSE.md"
            victim.unlink()
            env = os.environ.copy()
            env["CARGO_TARGET_DIR"] = str(dest / "target")
            failed = _run_licence(dest, ["--write", "binaries"], env)
            self.assertNotEqual(failed.returncode, 0, failed.stdout + failed.stderr)
            self.assertIn("is absent", failed.stdout + failed.stderr)
            self.assertEqual(sources.read_bytes(), before[0])
            self.assertEqual(html.read_bytes(), before[1])
            self.assertEqual(licences.read_bytes(), before[2])
            shutil.copy2(
                REPO / "ci/notices/crates/block2-0.6.2/LICENSE.md", victim)
            written = _run_licence(dest, ["--write", "binaries"], env)
            self.assertEqual(written.returncode, 0, written.stdout + written.stderr)
            data = tomllib.loads(sources.read_text(encoding="utf-8"))
            self.assertNotIn("identifier", data)
            self.assertEqual(
                [row["identifier"] for row in data["file"]],
                ["MIT", "Apache-2.0", "Zlib", "rust-std"])
            self.assertEqual(data["file"][-1]["toolchain"], "1.93.0")
            self.assertEqual(
                data["file"][-1]["sha256"],
                hashlib.sha256(html.read_bytes()).hexdigest())
            self.assertEqual(len(data["crate"]), 16)
            checked = _run_licence(dest, ["binaries"], env)
            self.assertEqual(checked.returncode, 0, checked.stdout + checked.stderr)

    def test_an_allowlisted_gap_is_refused(self) -> None:
        with self.assertRaises(L.BinariesError) as caught:
            L.elect_terms("GPL-3.0-only", {"mit"})
        self.assertIn("has no allowlisted licence", str(caught.exception))

    def test_a_licence_path_that_escapes_the_crate_root_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(L.BinariesError) as caught:
                L.collect_licence_files(Path(tmp), "../secret")
        self.assertIn("escapes the crate root", str(caught.exception))

    def test_an_absent_licence_file_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(L.BinariesError) as caught:
                L.collect_licence_files(Path(tmp), "MISSING-LICENCE.txt")
        self.assertIn("is absent", str(caught.exception))

    def test_a_crate_with_no_file_and_no_expression_is_refused(self) -> None:
        with self.assertRaises(L.BinariesError) as caught:
            _render([_notice("widget", "1.2.3", "")])
        self.assertIn("no licence file and no licence expression",
                      str(caught.exception))

    def test_write_refuses_a_section_other_than_binaries(self) -> None:
        buffer = io.StringIO()
        with patch("sys.stdout", buffer):
            code = L.main(["--write", "holder"])
        self.assertEqual(code, 1)
        self.assertIn(
            "--write regenerates the binaries section only", buffer.getvalue())

    def test_a_toolchain_mismatch_names_the_regenerate_instruction(self) -> None:
        message = L.toolchain_mismatch_message("1.98.0", "1.97.1")
        self.assertIn("1.98.0", message)
        self.assertIn("rustup component add rust-docs", message)

    def test_offline_empty_cargo_home_names_fetch(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            env = os.environ.copy()
            env["CARGO_HOME"] = tmp
            result = subprocess.run(
                [os.environ.get("PYTHON", "python3"),
                 str(REPO / "scripts/licence_check.py"), "binaries"],
                cwd=REPO, env=env, capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cargo fetch --locked", result.stdout + result.stderr)


def _copy_worktree(dest: Path) -> None:
    """Copy the tracked tree, then the notice pins, into a scratch directory."""
    listed = subprocess.run(
        ["git", "ls-files", "-z"], cwd=REPO, check=True, capture_output=True)
    for item in listed.stdout.split(b"\0"):
        if not item:
            continue
        relative = item.decode()
        source = REPO / relative
        if not source.is_file():
            continue
        target = dest / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)
    notices = REPO / "ci/notices"
    for source in notices.rglob("*"):
        if not source.is_file():
            continue
        target = dest / source.relative_to(REPO)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, target)


def _run_licence(dest: Path, args: list[str], env: dict):
    return subprocess.run(
        [sys.executable, "scripts/licence_check.py", *args],
        cwd=dest, env=env, capture_output=True, text=True)
