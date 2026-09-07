#!/usr/bin/env python3
"""Unit boundaries for the executable F-014 mutation harness."""

from __future__ import annotations

import ast
import io
import json
import subprocess
import sys
import unittest
from contextlib import redirect_stdout
from dataclasses import replace
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import quirk_mutations  # noqa: E402


class FakeSandbox:
    def __init__(self, result: subprocess.CompletedProcess[str]) -> None:
        self.result = result
        self.edits: list[tuple[str, str, str]] = []

    def substitute(self, path: str, old: str, new: str) -> None:
        self.edits.append((path, old, new))

    def run(
        self, argv: list[str], timeout: int = 300
    ) -> subprocess.CompletedProcess[str]:
        del argv, timeout
        return self.result


class MutationHarness(unittest.TestCase):
    def mutation(self) -> quirk_mutations.Mutation:
        return quirk_mutations.Mutation(
            kind="probe",
            path="probe.py",
            old="good",
            new="bad",
            argv=("probe",),
            failure_signature="expected failure",
        )

    def test_a_green_mutation_is_refused(self) -> None:
        box = FakeSandbox(subprocess.CompletedProcess([], 0, "", ""))
        with self.assertRaisesRegex(RuntimeError, "stayed green"):
            quirk_mutations.require_red(box, self.mutation())

    def test_a_red_mutation_with_the_wrong_reason_is_refused(self) -> None:
        box = FakeSandbox(subprocess.CompletedProcess([], 1, "other", ""))
        with self.assertRaisesRegex(RuntimeError, "wrong reason"):
            quirk_mutations.require_red(box, self.mutation())

    def test_a_red_mutation_with_the_declared_reason_passes(self) -> None:
        box = FakeSandbox(
            subprocess.CompletedProcess([], 1, "expected failure", "")
        )
        with redirect_stdout(io.StringIO()):
            quirk_mutations.require_red(box, self.mutation())
        self.assertEqual(box.edits, [("probe.py", "good", "bad")])

    def test_registry_drift_is_refused(self) -> None:
        document = json.loads((ROOT / "corpus" / "quirks.json").read_text())
        document["quirks"][0]["mutations"][0]["failureSignature"] = "invented"
        declared = quirk_mutations.mutations(Path(sys.executable))
        with self.assertRaisesRegex(RuntimeError, "executable contracts"):
            quirk_mutations.require_registry_contract(document, declared)

    def test_regression_is_the_executed_attribution_mutation(self) -> None:
        document = json.loads((ROOT / "corpus" / "quirks.json").read_text())
        declared = quirk_mutations.mutations(Path(sys.executable))
        regression = document["quirks"][0]["regression"]
        self.assertEqual(regression["command"], " ".join(declared[2].argv))
        self.assertEqual(
            regression["failureSignature"], declared[2].failure_signature
        )
        document["quirks"][0]["regression"]["command"] = "unrelated"
        with self.assertRaisesRegex(RuntimeError, "executable attribution"):
            quirk_mutations.require_registry_contract(document, declared)

    def test_the_focused_boundary_remains_named_and_owned(self) -> None:
        path = ROOT / "scripts" / "quirk_mutation_boundaries.py"
        tree = ast.parse(path.read_text(), filename=str(path))
        functions = {
            node.name for node in tree.body if isinstance(node, ast.FunctionDef)
        }
        self.assertIn("sigmoid_generator_boundary", functions)
        source = path.read_text()
        self.assertIn("expected SIGMOID, found", source)
        self.assertIn("expected width 0.5, found", source)

    def test_every_fixture_literal_has_an_executable_mutation(self) -> None:
        bindings = quirk_mutations.fixture_binding_mutations(Path(sys.executable))
        self.assertEqual(
            {binding.kind for binding in bindings},
            {
                "fixture-modality-values",
                "fixture-centre",
                "fixture-width",
                "fixture-display-minimum",
                "fixture-display-maximum",
                "fixture-expected-values",
            },
        )
        for changed in (
            replace(bindings[0], target_symbol="OTHER"),
            replace(bindings[0], old="OTHER = 1"),
            replace(bindings[0], argv=(str(sys.executable), "unrelated.py")),
        ):
            with self.subTest(changed=changed):
                drifted = (changed, *bindings[1:])
                with self.assertRaisesRegex(RuntimeError, "checker symbol contract"):
                    quirk_mutations.require_fixture_binding_contract(drifted)

    def test_the_live_harness_uses_the_checker_owned_fixture_boundary(self) -> None:
        self.assertIn(
            quirk_mutations.quirk_check.EXPECTATION_BOUNDARY,
            quirk_mutations.FIXTURE_BOUNDARY[1],
        )
        self.assertEqual(
            quirk_mutations.fixture_binding_mutations(Path(sys.executable))[0].path,
            quirk_mutations.quirk_check.FIXTURE_SCRIPT,
        )

    def test_executable_mutation_drift_is_refused(self) -> None:
        document = json.loads((ROOT / "corpus" / "quirks.json").read_text())
        declared = list(quirk_mutations.mutations(Path(sys.executable)))
        for changed in (
            replace(declared[0], failure_signature="invented"),
            replace(declared[0], replacement_value="OTHER"),
        ):
            with self.subTest(changed=changed):
                candidate = (changed, *declared[1:])
                with self.assertRaisesRegex(RuntimeError, "executable mutation"):
                    quirk_mutations.require_registry_contract(document, candidate)


if __name__ == "__main__":
    unittest.main()
