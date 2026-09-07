#!/usr/bin/env python3
"""Fixtures for the F-014 quirk-capture evidence checker."""

from __future__ import annotations

import copy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import quirk_check  # noqa: E402


def valid_record() -> dict[str, object]:
    """One complete record whose expected values come from PS3.3."""
    return {
        "schemaVersion": 1,
        "quirks": [
            {
                "id": "sigmoid-width-below-one",
                "symptom": "A legal SIGMOID width below one is treated as LINEAR.",
                "intake": {
                    "kind": "synthetic-reconstruction",
                    "patientDataRetained": False,
                },
                "generator": {
                    "script": "scripts/corpus_synth.py",
                    "case": "case_sigmoid_width_half",
                    "paths": ["synthetic/ct_sigmoid_width_half.dcm"],
                },
                "expectation": {
                    "authority": {
                        "kind": "dicom-standard",
                        "part": "PS3.3",
                        "section": "C.11.2.1.3.1",
                        "edition": "2026a",
                    },
                    "fixture": {
                        "path": "scripts/tests/test_corpus_synth.py",
                        "test": (
                            "SigmoidFixture."
                            "test_selected_display_values_follow_the_sigmoid_formula"
                        ),
                        "boundary": "sigmoid_expectation_boundary",
                        "symbols": {
                            "modalityValues": "SIGMOID_MODALITY_VALUES",
                            "centre": "SIGMOID_CENTRE",
                            "width": "SIGMOID_WIDTH",
                            "displayMinimum": "SIGMOID_DISPLAY_MINIMUM",
                            "displayMaximum": "SIGMOID_DISPLAY_MAXIMUM",
                            "expectedDisplayValues": "SIGMOID_DISPLAY_VALUES",
                        },
                    },
                    "inputs": {
                        "modalityValues": [39.5, 39.75, 40.0, 40.25, 40.5],
                        "centre": 40.0,
                        "width": 0.5,
                        "displayMinimum": 0.0,
                        "displayMaximum": 255.0,
                    },
                    "expectedDisplayValues": [
                        4.586483540333347,
                        30.396745115639977,
                        127.5,
                        224.60325488436,
                        250.41351645966665,
                    ],
                },
                "regression": {
                    "kind": "oracle-comparator-test",
                    "command": (
                        "cargo test -p ocelli-oracle "
                        "attribution::tests::"
                        "an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ "
                        "-- --exact"
                    ),
                    "failureSignature": "left: Fail",
                },
                "mutations": [
                    {
                        "kind": "generator-voi-function",
                        "value": "LINEAR",
                        "boundary": "quirk_mutation_boundaries.sigmoid_generator_boundary",
                        "failureSignature": "expected SIGMOID, found LINEAR",
                    },
                    {
                        "kind": "generator-window-width",
                        "value": 1.0,
                        "boundary": "quirk_mutation_boundaries.sigmoid_generator_boundary",
                        "failureSignature": "expected width 0.5, found 1.0",
                    },
                    {
                        "kind": "disable-reference-attribution",
                        "boundary": (
                            "attribution::tests::"
                            "an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ"
                        ),
                        "failureSignature": "left: Fail",
                    },
                ],
            }
        ],
    }


class QuirkDocumentValidation(unittest.TestCase):
    def errors(self, document: dict[str, object]) -> list[str]:
        return quirk_check.validate_document(
            document,
            root=ROOT,
            manifest_text=(ROOT / "corpus" / "manifest.tsv").read_text(
                encoding="utf-8"
            ),
            tracked_paths=set(),
        )

    def test_unknown_fields_are_refused_at_every_schema_object(self) -> None:
        paths = (
            (),
            ("quirks", 0),
            ("quirks", 0, "intake"),
            ("quirks", 0, "generator"),
            ("quirks", 0, "expectation"),
            ("quirks", 0, "expectation", "authority"),
            ("quirks", 0, "expectation", "fixture"),
        )
        for path in paths:
            with self.subTest(path=path):
                document = valid_record()
                target: object = document
                for component in path:
                    target = target[component]  # type: ignore[index]
                self.assertIsInstance(target, dict)
                target["unexpected"] = "not in the schema"  # type: ignore[index]
                self.assertTrue(
                    any("unknown field 'unexpected'" in error
                        for error in self.errors(document))
                )

    def test_a_complete_capture_record_passes(self) -> None:
        self.assertEqual(self.errors(valid_record()), [])

    def test_missing_expectation_authority_is_refused(self) -> None:
        record = valid_record()
        del record["quirks"][0]["expectation"]["authority"]
        self.assertTrue(any("expectation authority" in e for e in self.errors(record)))

    def test_an_ocelli_derived_expectation_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["authority"] = {
            "kind": "ocelli-output"
        }
        self.assertTrue(any("authority kind" in e for e in self.errors(record)))

    def test_a_reference_derived_expectation_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["authority"] = {
            "kind": "cornerstone3d-output"
        }
        self.assertTrue(any("authority kind" in e for e in self.errors(record)))

    def test_missing_generator_case_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["generator"]["case"] = "case_not_present"
        self.assertTrue(any("callable generator case" in e for e in self.errors(record)))

    def test_an_unregistered_manifest_path_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["generator"]["paths"] = ["synthetic/absent.dcm"]
        self.assertTrue(any("manifest row" in e for e in self.errors(record)))

    def test_a_path_owned_by_another_generator_case_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["generator"]["paths"] = [
            "synthetic/cr_monochrome1.dcm"
        ]
        self.assertTrue(any("QUIRK_CASES" in e for e in self.errors(record)))

    def test_the_generator_write_must_consume_the_declared_path(self) -> None:
        source = (ROOT / "scripts" / "corpus_synth.py").read_text()
        source = source.replace("write(ds, out / path)", "write(ds, out / 'other.dcm')")
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "corpus_synth.py"
            path.write_text(source)
            self.assertFalse(
                quirk_check.function_binds_quirk_path(
                    path, "case_sigmoid_width_half"
                )
            )

    def test_an_extra_generator_write_is_refused(self) -> None:
        source = (ROOT / "scripts" / "corpus_synth.py").read_text()
        source = source.replace(
            "write(ds, out / path)",
            "write(ds, out / path)\n    write(ds, out / 'other.dcm')",
        )
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "corpus_synth.py"
            path.write_text(source)
            self.assertFalse(
                quirk_check.function_binds_quirk_path(
                    path, "case_sigmoid_width_half"
                )
            )

    def test_extra_generator_paths_are_refused_when_both_records_agree(self) -> None:
        record = valid_record()
        extra = "synthetic/cr_monochrome1.dcm"
        record["quirks"][0]["generator"]["paths"].append(extra)
        original = quirk_check.literal_assignments

        def literals(path: Path) -> dict[str, object]:
            values = original(path)
            if path.name == "corpus_synth.py":
                values["QUIRK_CASES"]["case_sigmoid_width_half"]["paths"] = (
                    "synthetic/ct_sigmoid_width_half.dcm",
                    extra,
                )
            return values

        with patch.object(quirk_check, "literal_assignments", side_effect=literals):
            self.assertTrue(
                any("exactly one declared path" in error
                    for error in self.errors(record))
            )

    def test_a_manifest_row_from_another_source_is_refused(self) -> None:
        manifest = (
            "path\tmodality\ttransfer_syntax\tcategories\tsource\tlicence\t"
            "licence_url\tsha256\turl\n"
            "synthetic/ct_sigmoid_width_half.dcm\tCT\t1.2.3\tsynthetic\t"
            "somewhere else\tMIT\thttps://example.invalid\tdeadbeef\t\n"
        )
        errors = quirk_check.validate_document(
            valid_record(), root=ROOT, manifest_text=manifest, tracked_paths=set()
        )
        self.assertTrue(any("generator source" in e for e in errors))

    def test_a_tracked_generated_dicom_is_refused(self) -> None:
        record = valid_record()
        errors = quirk_check.validate_document(
            record,
            root=ROOT,
            manifest_text=(ROOT / "corpus" / "manifest.tsv").read_text(
                encoding="utf-8"
            ),
            tracked_paths={"corpus/data/synthetic/ct_sigmoid_width_half.dcm"},
        )
        self.assertTrue(any("tracked DICOM" in e for e in errors))

    def test_missing_mutation_evidence_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["mutations"] = []
        self.assertTrue(any("mutation evidence" in e for e in self.errors(record)))

    def test_changed_expected_values_are_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["expectedDisplayValues"] = [0.0]
        self.assertTrue(any("fixture literals" in e for e in self.errors(record)))

    def test_changed_expectation_inputs_are_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["inputs"] = {"unrelated": 999}
        self.assertTrue(any("fixture literals" in e for e in self.errors(record)))

    def test_an_unrelated_fixture_test_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["fixture"]["test"] = (
            "SigmoidFixture.test_stored_values_cross_the_window_centre"
        )
        self.assertTrue(any("declared boundary" in e for e in self.errors(record)))

    def test_irrelevant_literal_reads_do_not_count_as_a_fixture_test(self) -> None:
        source = """
class Fixture:
    def test_values(self):
        observed = INPUTS, EXPECTED
        self.assertIsNotNone(observed)
"""
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixture.py"
            path.write_text(source)
            self.assertFalse(
                quirk_check.test_calls_boundary(
                    path, "Fixture.test_values", "expectation_boundary"
                )
            )

    def test_the_registry_cannot_redirect_the_fixture_boundary(self) -> None:
        record = valid_record()
        record["quirks"][0]["expectation"]["fixture"]["boundary"] = (
            "vacuous_expectation_boundary"
        )
        self.assertTrue(any("declared boundary" in e for e in self.errors(record)))

    def test_an_invented_mutation_contract_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["mutations"] = [
            {
                "kind": "generator-window-width",
                "value": "not-a-width",
                "boundary": "SigmoidFixture.test_stored_values_cross_the_window_centre",
                "failureSignature": "invented",
            }
        ]
        self.assertTrue(any("checker-owned contract" in e for e in self.errors(record)))

    def test_each_required_mutation_is_refused_when_removed(self) -> None:
        for kind in quirk_check.MUTATION_KINDS:
            with self.subTest(kind=kind):
                record = valid_record()
                record["quirks"][0]["mutations"] = [
                    mutation
                    for mutation in record["quirks"][0]["mutations"]
                    if mutation["kind"] != kind
                ]
                self.assertTrue(
                    any(
                        f"required mutation {kind} is missing" in error
                        for error in self.errors(record)
                    )
                )

    def test_an_unknown_regression_kind_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["regression"]["kind"] = "shell-from-json"
        self.assertTrue(any("checker-owned contract" in e for e in self.errors(record)))

    def test_regression_signature_drift_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["regression"]["failureSignature"] = "invented"
        self.assertTrue(any("checker-owned contract" in e for e in self.errors(record)))

    def test_an_unknown_mutation_kind_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["mutations"][0]["kind"] = "arbitrary-edit"
        self.assertTrue(any("mutation kind" in e for e in self.errors(record)))

    def test_patient_data_retention_is_refused(self) -> None:
        record = valid_record()
        record["quirks"][0]["intake"]["patientDataRetained"] = True
        self.assertTrue(any("patient data" in e for e in self.errors(record)))

    def test_duplicate_ids_are_refused(self) -> None:
        record = valid_record()
        record["quirks"].append(copy.deepcopy(record["quirks"][0]))
        self.assertTrue(any("duplicate quirk id" in e for e in self.errors(record)))


if __name__ == "__main__":
    unittest.main()
