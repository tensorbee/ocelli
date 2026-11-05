"""Advisory threshold, exception rows and the consumer proof.

The expected refusals are the plan's fragment table. The reports are npm's
documented audit report version 2, built here, not copied from a live run.
"""

from __future__ import annotations

import importlib.util
import io
import json
import os
import stat
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))


def load(name: str):
    spec = importlib.util.spec_from_file_location(
        name + "_under_test", ROOT / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


A = load("advisory_check")
P = load("package_check")

HIGH = "GHSA-1111-2222-3333"
OTHER = "GHSA-9999-8888-7777"
CLOSURE = {"adm-zip", "dcmjs", "pako"}
VERSIONS = {
    "node_modules/adm-zip": "0.5.17",
    "node_modules/dcmjs": "0.52.2",
    "node_modules/pako": "2.1.0",
}


def row(**overrides) -> dict:
    base = {
        "ecosystem": "npm",
        "package": "adm-zip",
        "version": "0.5.17",
        "advisories": [HIGH],
        "scope": "unreachable",
        "proof": "packages-consumer-trace",
        "reason": "the shipped files do not load it",
        "decided": "2026-11-05",
        "decided_by": "lead",
    }
    base.update(overrides)
    return base


def via(ghsa: str, severity: str) -> dict:
    return {
        "url": f"https://github.com/advisories/{ghsa}",
        "severity": severity,
    }


def report_for(package: str, entries: list, nodes: list[str] | None = None,
               version: int | str = 2) -> dict:
    return {
        "auditReportVersion": version,
        "vulnerabilities": {
            package: {
                "via": entries,
                "nodes": nodes if nodes is not None else [f"node_modules/{package}"],
            },
        },
    }


def run_main(argv: list[str]) -> tuple[int, str]:
    buffer = io.StringIO()
    with redirect_stdout(buffer):
        code = A.main(argv)
    return code, buffer.getvalue()


class ExceptionShape(unittest.TestCase):
    def test_a_complete_row_is_accepted(self) -> None:
        self.assertEqual(A.exception_problems(
            [row()], CLOSURE, VERSIONS, set()), [])

    def test_each_required_field_emptied_is_missing(self) -> None:
        for field in A.FIELDS:
            with self.subTest(field=field):
                broken = row()
                broken[field] = [] if field == "advisories" else ""
                problems = A.exception_problems(
                    [broken], CLOSURE, VERSIONS, set())
                self.assertTrue(
                    any(f"is missing {field}" in item for item in problems),
                    problems)
                self.assertFalse(
                    any("is not unreachable" in item for item in problems))
                self.assertFalse(
                    any("names no proof" in item for item in problems))

    def test_a_missing_key_is_the_same_refusal(self) -> None:
        broken = row()
        del broken["reason"]
        problems = A.exception_problems([broken], CLOSURE, VERSIONS, set())
        self.assertIn("is missing reason", "\n".join(problems))

    def test_dev_only_scope_is_not_unreachable(self) -> None:
        problems = A.exception_problems(
            [row(scope="dev-only")], CLOSURE, VERSIONS, set())
        self.assertTrue(any("is not unreachable" in item for item in problems))
        self.assertFalse(any("is missing scope" in item for item in problems))

    def test_a_padded_scope_is_not_unreachable(self) -> None:
        padded = "unreachable "
        problems = A.exception_problems(
            [row(scope=padded)], CLOSURE, VERSIONS, set())
        self.assertTrue(any("is not unreachable" in item for item in problems))
        self.assertFalse(any("is missing scope" in item for item in problems))
        self.assertFalse(A.scope_is_unreachable(padded))
        self.assertFalse(A.scope_is_unreachable(" unreachable"))
        self.assertTrue(A.scope_is_unreachable("unreachable"))
        # The consumer proof reads the same function, so a padded row is not
        # a row it checks, even when the installed version disagrees.
        self.assertEqual(P.consumer_version_problems(
            [row(scope=padded)],
            {"packages": {"node_modules/adm-zip": {"version": "0.6.1"}}}),
            [])

    def test_an_unknown_proof_is_refused(self) -> None:
        problems = A.exception_problems(
            [row(proof="npm-ls")], CLOSURE, VERSIONS, set())
        self.assertTrue(
            any("names no proof the packages gate runs" in item
                for item in problems),
            problems)

    def test_a_version_the_closure_does_not_install_is_stale(self) -> None:
        problems = A.exception_problems(
            [row(version="0.5.16")], CLOSURE, VERSIONS, set())
        self.assertTrue(
            any("stale advisory exception" in item for item in problems))

    def test_a_renamed_package_is_stale(self) -> None:
        problems = A.exception_problems(
            [row(package="adm-zip-old")], CLOSURE, VERSIONS, set())
        self.assertTrue(
            any("stale advisory exception" in item for item in problems))

    def test_an_install_script_is_refused(self) -> None:
        problems = A.exception_problems(
            [row()], CLOSURE, VERSIONS, {"adm-zip"})
        self.assertTrue(
            any("has an install script" in item for item in problems))


class Overrides(unittest.TestCase):
    def test_a_top_level_closure_package_is_refused(self) -> None:
        problems = A.override_problems({"adm-zip": "0.6.1"}, CLOSURE)
        self.assertEqual(len(problems), 1)
        self.assertIn("overrides do not reach a consumer", problems[0])
        self.assertIn("adm-zip", problems[0])

    def test_a_nested_override_names_both_packages(self) -> None:
        problems = A.override_problems(
            {"dcmjs": {"adm-zip": "0.6.1"}}, CLOSURE)
        text = "\n".join(problems)
        self.assertIn("overrides do not reach a consumer", text)
        self.assertIn("dcmjs", text)
        self.assertIn("adm-zip", text)

    def test_a_deeper_nesting_still_matches(self) -> None:
        problems = A.override_problems(
            {"dcmjs": {"pako": {"adm-zip": "0.6.1"}}}, CLOSURE)
        self.assertEqual(len(problems), 3)

    def test_a_version_selector_names_the_package(self) -> None:
        problems = A.override_problems({"adm-zip@0.5.17": "0.6.1"}, CLOSURE)
        self.assertIn("adm-zip", problems[0])
        scoped = A.override_problems(
            {"@babel/runtime-corejs3@7.29.2": "7.29.2"},
            {"@babel/runtime-corejs3"})
        self.assertIn("@babel/runtime-corejs3", scoped[0])

    def test_react_overrides_are_outside_the_closure(self) -> None:
        self.assertEqual(A.override_problems(
            {"react": "19.2.8", "react-dom": "19.2.8"}, CLOSURE), [])

    def test_a_dot_key_does_not_name_a_package(self) -> None:
        self.assertEqual(A.override_problems(
            {"left-pad": {".": "1.0.0"}}, CLOSURE), [])


class AuditReport(unittest.TestCase):
    def test_an_unexcepted_high_names_the_advisory(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("pako", [via(HIGH, "high")]),
            CLOSURE, VERSIONS, [])
        self.assertEqual(warnings, [])
        self.assertTrue(any("has no exception" in item and HIGH in item
                            for item in problems), problems)

    def test_critical_is_refused_the_same_way(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("pako", [via(HIGH, "critical")]),
            CLOSURE, VERSIONS, [])
        self.assertTrue(any("has no exception" in item and HIGH in item
                            for item in problems), problems)

    def test_a_new_high_on_an_excepted_package_is_refused(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("adm-zip", [via(HIGH, "high"), via(OTHER, "high")]),
            CLOSURE, VERSIONS, [row(advisories=[HIGH])])
        self.assertTrue(any(OTHER in item and "has no exception" in item
                            for item in problems), problems)
        self.assertFalse(any(HIGH in item and "has no exception" in item
                             for item in problems), problems)

    def test_a_listed_advisory_the_report_dropped_is_stale(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("adm-zip", [via(HIGH, "high")]),
            CLOSURE, VERSIONS, [row(advisories=[HIGH, OTHER])])
        self.assertTrue(any("no longer reports" in item and OTHER in item
                            for item in problems), problems)
        self.assertFalse(any(HIGH in item and "no longer reports" in item
                             for item in problems))

    def test_a_high_outside_the_closure_is_accepted(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("vite", [via(HIGH, "high")], ["node_modules/vite"]),
            CLOSURE, VERSIONS, [])
        self.assertEqual(problems, [])
        self.assertEqual(warnings, [])

    def test_a_versions_entry_outside_the_closure_is_not_shipped(self) -> None:
        """The outside-closure probe records vite's node and not its name.

        Membership is not "any key of versions". The directory name has to
        be in the closure as well, which is the alias rule the other way
        round: `node_modules/vite` with closure `{left-pad}` stays out.
        """
        problems, warnings = A.audit_problems(
            report_for("vite", [via(HIGH, "high")], ["node_modules/vite"]),
            {"left-pad"}, {"node_modules/vite": "7.0.0"}, [])
        self.assertEqual(problems, [])
        self.assertEqual(warnings, [])

    def test_an_npm_alias_is_in_the_closure_by_its_node(self) -> None:
        """The audit names adm-zip. The closure key is the alias zipper.

        npm's report for a lock entry `node_modules/zipper` whose `name` is
        adm-zip uses that node list and the real package name. `versions`
        is keyed by the node path, which is how the closure records the
        alias. With no row the high advisory is refused.
        """
        closure = {"zipper", "dcmjs"}
        versions = {
            "node_modules/zipper": "0.5.17",
            "node_modules/dcmjs": "0.52.2",
        }
        document = report_for(
            "adm-zip", [via(HIGH, "high")], ["node_modules/zipper"])
        problems, warnings = A.audit_problems(document, closure, versions, [])
        self.assertEqual(warnings, [])
        self.assertTrue(
            any("adm-zip@0.5.17" in item and "has no exception" in item
                for item in problems), problems)
        covered, _warned = A.audit_problems(
            document, closure, versions, [row()])
        self.assertFalse(
            any("has no exception" in item for item in covered), covered)

    def test_an_unexcepted_moderate_warns_and_does_not_fail(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("pako", [via(HIGH, "moderate")]),
            CLOSURE, VERSIONS, [])
        self.assertEqual(problems, [])
        self.assertTrue(any(HIGH in item and "moderate" in item
                            for item in warnings), warnings)

    def test_low_and_info_warn(self) -> None:
        for severity in ("low", "info"):
            with self.subTest(severity=severity):
                problems, warnings = A.audit_problems(
                    report_for("pako", [via(HIGH, severity)]),
                    CLOSURE, VERSIONS, [])
                self.assertEqual(problems, [])
                self.assertEqual(len(warnings), 1)

    def test_a_listed_moderate_still_warns(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("adm-zip", [via(HIGH, "moderate")]),
            CLOSURE, VERSIONS, [row(advisories=[HIGH])])
        self.assertEqual(problems, [])
        self.assertEqual(len(warnings), 1)

    def test_a_listed_high_does_not_warn(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("adm-zip", [via(HIGH, "high")]),
            CLOSURE, VERSIONS, [row()])
        self.assertEqual(problems, [])
        self.assertEqual(warnings, [])

    def test_a_string_via_is_attribution_and_not_an_advisory(self) -> None:
        problems, warnings = A.audit_problems(
            report_for("dcmjs", ["adm-zip"]),
            CLOSURE, VERSIONS, [])
        self.assertEqual(problems, [])
        self.assertEqual(warnings, [])

    def test_two_versions_are_judged_separately(self) -> None:
        versions = dict(VERSIONS)
        versions["node_modules/dcmjs/node_modules/adm-zip"] = "0.5.16"
        document = report_for(
            "adm-zip", [via(HIGH, "high")],
            ["node_modules/adm-zip", "node_modules/dcmjs/node_modules/adm-zip"])
        problems, _warnings = A.audit_problems(
            document, CLOSURE, versions, [row()])
        self.assertTrue(any("0.5.16" in item and "has no exception" in item
                            for item in problems), problems)
        self.assertFalse(any("0.5.17" in item and "has no exception" in item
                             for item in problems))

    def test_an_empty_report_did_not_produce_a_report(self) -> None:
        for document in ({}, None, {"error": {"code": "EAI_AGAIN"}}):
            with self.subTest(document=document):
                problems, warnings = A.audit_problems(
                    document, CLOSURE, VERSIONS, [])
                self.assertEqual(warnings, [])
                self.assertTrue(
                    any("did not produce a report" in item for item in problems))

    def test_report_version_1_is_refused(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("pako", [], version=1), CLOSURE, VERSIONS, [])
        self.assertTrue(any("report version" in item for item in problems),
                        problems)
        self.assertFalse(any("did not produce a report" in item
                             for item in problems))

    def test_a_string_version_is_not_version_2(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("pako", [], version="2"), CLOSURE, VERSIONS, [])
        self.assertTrue(any("report version" in item for item in problems))

    def test_an_unknown_severity_fails_closed(self) -> None:
        problems, _warnings = A.audit_problems(
            report_for("pako", [via(HIGH, "severe")]),
            CLOSURE, VERSIONS, [])
        self.assertTrue(any("unknown severity" in item and "severe" in item
                            for item in problems), problems)


class Runner(unittest.TestCase):
    def _stub(self, directory: Path, text: str) -> None:
        binary = directory / "npm"
        binary.write_text("#!/bin/sh\n" + text + "\n", encoding="utf-8")
        binary.chmod(binary.stat().st_mode | stat.S_IEXEC)

    def test_a_stub_npm_with_no_json_is_refused(self) -> None:
        directory = Path(tempfile.mkdtemp())
        self._stub(directory, "echo not-json\nexit 1")
        with patch.dict(os.environ, {"PATH": str(directory)}):
            self.assertIsNone(A.load_audit(None))
            code, output = run_main(["npm"])
        self.assertEqual(code, 1)
        self.assertIn("did not produce a report", output)

    def test_a_stub_npm_with_an_error_key_is_refused(self) -> None:
        directory = Path(tempfile.mkdtemp())
        self._stub(directory, "echo '{\"error\":{\"code\":\"EAI_AGAIN\"}}'\nexit 1")
        with patch.dict(os.environ, {"PATH": str(directory)}):
            code, output = run_main(["npm"])
        self.assertEqual(code, 1)
        self.assertIn("did not produce a report", output)

    def test_a_report_file_is_read_instead_of_calling_npm(self) -> None:
        directory = Path(tempfile.mkdtemp())
        report = directory / "audit.json"
        report.write_text(json.dumps({
            "auditReportVersion": 2,
            "vulnerabilities": {},
        }), encoding="utf-8")
        with patch.dict(os.environ, {"PATH": str(directory)}):
            code, output = run_main(["npm", "--report", str(report)])
        self.assertEqual(code, 1)
        self.assertIn("no longer reports", output)
        self.assertNotIn("did not produce a report", output)

    def test_unparseable_report_text_is_refused(self) -> None:
        directory = Path(tempfile.mkdtemp())
        report = directory / "audit.json"
        report.write_text("not json", encoding="utf-8")
        code, output = run_main(["npm", "--report", str(report)])
        self.assertEqual(code, 1)
        self.assertIn("did not produce a report", output)

    def test_the_recorded_exception_section_passes(self) -> None:
        code, output = run_main(["exceptions"])
        self.assertEqual(code, 0, output)
        self.assertIn("adm-zip@0.5.17", output)
        self.assertIn("OK:", output)


class ConsumerProof(unittest.TestCase):
    def test_a_different_consumer_version_is_refused(self) -> None:
        problems = P.consumer_version_problems(
            [row()],
            {"packages": {"node_modules/adm-zip": {"version": "0.6.1"}}})
        self.assertTrue(any("the consumer resolves" in item for item in problems))
        self.assertIn("0.6.1", problems[0])

    def test_the_recorded_consumer_version_is_accepted(self) -> None:
        self.assertEqual(P.consumer_version_problems(
            [row()],
            {"packages": {"node_modules/adm-zip": {"version": "0.5.17"}}}),
            [])

    def test_a_missing_consumer_package_is_refused(self) -> None:
        problems = P.consumer_version_problems([row()], {"packages": {}})
        self.assertTrue(any("the consumer resolves" in item for item in problems))

    def test_a_resolved_url_under_the_package_is_refused(self) -> None:
        problems = P.resolved_package_problems(
            ["file:///tmp/consumer/node_modules/adm-zip/adm-zip.js"],
            {"adm-zip"})
        self.assertTrue(
            any("loads an unreachable package" in item for item in problems))

    def test_other_resolved_urls_are_accepted(self) -> None:
        self.assertEqual(P.resolved_package_problems(
            ["file:///tmp/consumer/node_modules/dcmjs/build/dcmjs.es.js"],
            {"adm-zip"}), [])

    def test_a_quoted_name_in_another_package_is_refused(self) -> None:
        problems = P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js": 'import "adm-zip"\n'},
            {"adm-zip"})
        self.assertTrue(
            any("names an unreachable package" in item for item in problems))
        single = P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js": "import 'adm-zip'\n"},
            {"adm-zip"})
        self.assertEqual(len(single), 1)

    def test_the_packages_own_files_and_bare_mentions_are_accepted(self) -> None:
        self.assertEqual(P.specifier_problems(
            {"node_modules/adm-zip/adm-zip.js": 'module.exports = "adm-zip"\n'},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js": "var admZip = 1\n"},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js":
             '"node_modules/adm-zip/index.js"\n'},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js":
             'require(["adm","zip"].join("-"));\n'},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js": '"adm-zipper"\n'},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js":
             'require("../../adm-zipper");\n'},
            {"adm-zip"}), [])
        self.assertEqual(P.specifier_problems(
            {"node_modules/dcmjs/build/dcmjs.es.js":
             'require("foo/../../adm-zip");\n'},
            {"adm-zip"}), [])

    def test_a_specifier_in_any_quote_or_subpath_is_refused(self) -> None:
        forms = [
            'import "adm-zip";\n',
            "import 'adm-zip';\n",
            "import `adm-zip`;\n",
            'require("adm-zip/adm-zip.js");\n',
            "require('adm-zip/foo');\n",
            "require(`adm-zip/adm-zip.js`);\n",
            'import "./node_modules/adm-zip";\n',
            'import "./node_modules/adm-zip/adm-zip.js";\n',
            'import "../node_modules/adm-zip";\n',
            'import "../../node_modules/adm-zip/index.js";\n',
            'import "../adm-zip";\n',
            "import '../adm-zip/adm-zip.js';\n",
            "import `../../adm-zip/adm-zip.js`;\n",
            'require("../../adm-zip");\n',
            'require("../../../adm-zip/adm-zip.js");\n',
        ]
        for text in forms:
            with self.subTest(text=text):
                problems = P.specifier_problems(
                    {"node_modules/dcmjs/build/dcmjs.es.js": text,
                     "node_modules/dcmjs/build/dcmjs.js": text},
                    {"adm-zip"})
                self.assertEqual(len(problems), 2, problems)
                self.assertTrue(all(
                    "names an unreachable package" in item for item in problems))

    def test_a_trace_without_the_dcmjs_entry_is_refused(self) -> None:
        only_cjs = P.trace_entry_problems([
            "file:///tmp/consumer/node_modules/dcmjs/build/dcmjs.js"])
        self.assertEqual(only_cjs, [
            "the resolve trace does not record dcmjs/build/dcmjs.es.js"])
        only_es = P.trace_entry_problems([
            "file:///tmp/c/node_modules/dcmjs/build/dcmjs.es.js"])
        self.assertEqual(only_es, [
            "the resolve trace does not record dcmjs/build/dcmjs.js"])
        self.assertEqual(P.trace_entry_problems([]), [
            "the resolve trace does not record dcmjs/build/dcmjs.es.js",
            "the resolve trace does not record dcmjs/build/dcmjs.js",
        ])
        self.assertEqual(P.trace_entry_problems([
            "file:///tmp/c/node_modules/dcmjs/build/dcmjs.es.js",
            "file:///tmp/c/node_modules/dcmjs/build/dcmjs.js",
        ]), [])

    def test_the_smoke_script_requires_dcmjs_and_the_hook_registers(self) -> None:
        self.assertEqual(
            P.smoke_wiring_problems(P.SMOKE_SCRIPT, P.TRACE_REGISTER), [])
        self.assertIn('createRequire(import.meta.url)("dcmjs")', P.SMOKE_SCRIPT)
        self.assertIn("registerHooks(", P.TRACE_REGISTER)
        missing = P.smoke_wiring_problems("import 'x'\n", P.TRACE_REGISTER)
        self.assertTrue(any("does not require dcmjs" in item for item in missing))
        plain = P.smoke_wiring_problems(
            P.SMOKE_SCRIPT, 'import { register } from "node:module";\n')
        self.assertTrue(
            any("does not use registerHooks" in item for item in plain))

    def test_unreachable_problems_reads_a_consumer_directory(self) -> None:
        directory = Path(tempfile.mkdtemp())
        consumer = directory / "consumer"
        (consumer / "node_modules" / "dcmjs" / "build").mkdir(parents=True)
        (consumer / "node_modules" / "dcmjs" / "build" / "dcmjs.es.js").write_text(
            "import `adm-zip`;\n", encoding="utf-8")
        (consumer / "package-lock.json").write_text(json.dumps({
            "packages": {"node_modules/adm-zip": {"version": "0.5.17"}},
        }), encoding="utf-8")
        (consumer / "trace-urls.txt").write_text(
            "file:///tmp/c/node_modules/other.js\n", encoding="utf-8")
        problems = P.unreachable_problems(consumer, [row()])
        text = "\n".join(problems)
        self.assertIn("does not record dcmjs/build/dcmjs.es.js", text)
        self.assertIn("names an unreachable package", text)
        self.assertNotIn("the consumer resolves", text)


if __name__ == "__main__":
    unittest.main()
