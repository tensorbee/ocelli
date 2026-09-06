#!/usr/bin/env python3
"""Validate the permanent field-quirk evidence graph. F-014.

The registry is data only. This checker never executes a command or imports the
DICOM generator named by a record. It parses the generator and fixture source,
then connects each declared path to the committed manifest.
"""

from __future__ import annotations

import ast
import csv
import json
import re
import subprocess
import sys
from io import StringIO
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "corpus" / "quirks.json"
MANIFEST = ROOT / "corpus" / "manifest.tsv"

GENERATOR_SCRIPT = "scripts/corpus_synth.py"
GENERATOR_SOURCE = f"Ocelli synthetic, {GENERATOR_SCRIPT}"
FIXTURE_SCRIPT = "scripts/tests/test_corpus_synth.py"
EXPECTATION_BOUNDARY = "sigmoid_expectation_boundary"
GENERATOR_MUTATION_SCRIPT = "scripts/quirk_mutation_boundaries.py"
GENERATOR_MUTATION_BOUNDARY = (
    "quirk_mutation_boundaries.sigmoid_generator_boundary"
)
ATTRIBUTION_SCRIPT = "tools/oracle/src/attribution.rs"
ATTRIBUTION_BOUNDARY = (
    "attribution::tests::"
    "an_inverted_sigmoid_reference_is_attributed_only_when_pixels_differ"
)
REGRESSION_COMMAND = (
    f"cargo test -p ocelli-oracle {ATTRIBUTION_BOUNDARY} -- --exact"
)
REGRESSION_CONTRACTS = {
    "sigmoid-width-below-one": {
        "kind": "oracle-comparator-test",
        "command": REGRESSION_COMMAND,
        "failureSignature": "left: Fail",
    },
}
MUTATION_KINDS = {
    "generator-voi-function",
    "generator-window-width",
    "disable-reference-attribution",
}
FIXTURE_INPUT_SYMBOLS = {
    "modalityValues": "SIGMOID_MODALITY_VALUES",
    "centre": "SIGMOID_CENTRE",
    "width": "SIGMOID_WIDTH",
    "displayMinimum": "SIGMOID_DISPLAY_MINIMUM",
    "displayMaximum": "SIGMOID_DISPLAY_MAXIMUM",
}
EXPECTED_VALUES_SYMBOL = "SIGMOID_DISPLAY_VALUES"
MUTATION_CONTRACTS = {
    "sigmoid-width-below-one": {
        "generator-voi-function": {
            "value": "LINEAR",
            "boundary": GENERATOR_MUTATION_BOUNDARY,
            "failureSignature": "expected SIGMOID, found LINEAR",
        },
        "generator-window-width": {
            "value": 1.0,
            "boundary": GENERATOR_MUTATION_BOUNDARY,
            "failureSignature": "expected width 0.5, found 1.0",
        },
        "disable-reference-attribution": {
            "boundary": ATTRIBUTION_BOUNDARY,
            "failureSignature": "left: Fail",
        },
    },
}
AUTHORITY_KINDS = {"dicom-standard", "hand-calculation"}
ID = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
PART = re.compile(r"^PS3\.\d+$")


def nonempty(value: object) -> bool:
    return isinstance(value, str) and bool(value.strip())


def objects(value: object) -> list[dict[str, Any]]:
    if not isinstance(value, list):
        return []
    return [item for item in value if isinstance(item, dict)]


def unknown_fields(
    value: object, allowed: set[str], prefix: str
) -> list[str]:
    """Name every property outside one declared object vocabulary."""
    if not isinstance(value, dict):
        return []
    return [
        f"{prefix}: unknown field {field!r}"
        for field in sorted(set(value) - allowed)
    ]


def source_symbols(path: Path) -> tuple[set[str], set[str]]:
    """Return top-level functions and dotted unittest method names."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError):
        return set(), set()
    functions = {
        node.name for node in tree.body if isinstance(node, ast.FunctionDef)
    }
    tests: set[str] = set()
    for node in tree.body:
        if not isinstance(node, ast.ClassDef):
            continue
        for child in node.body:
            if isinstance(child, ast.FunctionDef) and child.name.startswith("test_"):
                tests.add(f"{node.name}.{child.name}")
    return functions, tests


def literal_assignments(path: Path) -> dict[str, Any]:
    """Return top-level assignments whose right side is an AST literal."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError):
        return {}
    values: dict[str, Any] = {}
    for node in tree.body:
        if not isinstance(node, (ast.Assign, ast.AnnAssign)):
            continue
        targets = node.targets if isinstance(node, ast.Assign) else [node.target]
        if len(targets) != 1 or not isinstance(targets[0], ast.Name):
            continue
        try:
            values[targets[0].id] = ast.literal_eval(node.value)
        except (ValueError, TypeError):
            continue
    return values


def subscript_key(node: ast.AST, key: object) -> bool:
    return (
        isinstance(node, ast.Subscript)
        and isinstance(node.slice, ast.Constant)
        and node.slice.value == key
    )


def function_binds_quirk_path(path: Path, function: str) -> bool:
    """Prove the named case takes its write path from its QUIRK_CASES row."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError):
        return False
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name == function:
            case_names: set[str] = set()
            path_names: set[str] = set()
            for child in ast.walk(node):
                if not isinstance(child, ast.Assign) or len(child.targets) != 1:
                    continue
                target = child.targets[0]
                if not isinstance(target, ast.Name):
                    continue
                value = child.value
                if (
                    subscript_key(value, function)
                    and isinstance(value.value, ast.Name)
                    and value.value.id == "QUIRK_CASES"
                ):
                    case_names.add(target.id)
                    continue
                if (
                    isinstance(value, ast.Call)
                    and isinstance(value.func, ast.Name)
                    and value.func.id == "Path"
                    and len(value.args) == 1
                    and subscript_key(value.args[0], 0)
                    and subscript_key(value.args[0].value, "paths")
                    and isinstance(value.args[0].value.value, ast.Name)
                    and value.args[0].value.value.id in case_names
                ):
                    path_names.add(target.id)
            writes = [
                child for child in ast.walk(node)
                if isinstance(child, ast.Call)
                and isinstance(child.func, ast.Name)
                and child.func.id == "write"
            ]
            return (
                len(writes) == 1
                and any(
                    isinstance(argument, ast.BinOp)
                    and isinstance(argument.op, ast.Div)
                    and isinstance(argument.right, ast.Name)
                    and argument.right.id in path_names
                    for argument in writes[0].args
                )
            )
    return False


def test_calls_boundary(path: Path, dotted_test: str, boundary: str) -> bool:
    """Whether one named unittest method consists of the fixed boundary call."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError):
        return False
    class_name, separator, test_name = dotted_test.partition(".")
    if not separator:
        return False
    for node in tree.body:
        if not isinstance(node, ast.ClassDef) or node.name != class_name:
            continue
        for child in node.body:
            if isinstance(child, ast.FunctionDef) and child.name == test_name:
                if len(child.body) != 1 or not isinstance(child.body[0], ast.Expr):
                    return False
                expression = child.body[0].value
                return (
                    isinstance(expression, ast.Call)
                    and isinstance(expression.func, ast.Name)
                    and expression.func.id == boundary
                    and not expression.args
                    and not expression.keywords
                )
    return False


def json_shape(value: Any) -> Any:
    """Normalise tuple literals to the array shape JSON can represent."""
    if isinstance(value, (list, tuple)):
        return [json_shape(item) for item in value]
    if isinstance(value, dict):
        return {key: json_shape(item) for key, item in value.items()}
    return value


def called_functions(path: Path, caller: str) -> set[str]:
    """Return direct function calls made inside one top-level function."""
    try:
        tree = ast.parse(path.read_text(encoding="utf-8"), filename=str(path))
    except (OSError, SyntaxError):
        return set()
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name == caller:
            return {
                child.func.id
                for child in ast.walk(node)
                if isinstance(child, ast.Call)
                and isinstance(child.func, ast.Name)
            }
    return set()


def manifest_rows(text: str) -> dict[str, list[dict[str, str]]]:
    rows: dict[str, list[dict[str, str]]] = {}
    for row in csv.DictReader(StringIO(text), delimiter="\t"):
        rows.setdefault(row.get("path", ""), []).append(row)
    return rows


def validate_authority(authority: object, prefix: str) -> list[str]:
    if not isinstance(authority, dict):
        return [f"{prefix}: missing expectation authority"]
    kind = authority.get("kind")
    if kind not in AUTHORITY_KINDS:
        return [
            f"{prefix}: expectation authority kind {kind!r} is not independent. "
            "Use dicom-standard or hand-calculation"
        ]
    errors: list[str] = []
    if kind == "dicom-standard":
        errors.extend(unknown_fields(
            authority, {"kind", "part", "section", "edition"}, prefix
        ))
        if not nonempty(authority.get("part")) or not PART.fullmatch(
                str(authority.get("part", ""))):
            errors.append(f"{prefix}: DICOM authority needs a PS3.x part")
        for field in ("section", "edition"):
            if not nonempty(authority.get(field)):
                errors.append(f"{prefix}: DICOM authority needs {field}")
    else:
        errors.extend(unknown_fields(
            authority, {"kind", "inputs", "steps"}, prefix
        ))
        if not isinstance(authority.get("inputs"), dict) or not authority["inputs"]:
            errors.append(f"{prefix}: hand calculation needs named inputs")
        steps = authority.get("steps")
        if not isinstance(steps, list) or not steps or not all(
                nonempty(step) for step in steps):
            errors.append(f"{prefix}: hand calculation needs literal steps")
    return errors


def validate_document(
    document: object,
    *,
    root: Path,
    manifest_text: str,
    tracked_paths: set[str],
) -> list[str]:
    """Return every refusal in a registry without executing registry data."""
    if not isinstance(document, dict):
        return ["registry: top level must be an object"]
    errors = unknown_fields(document, {"schemaVersion", "quirks"}, "registry")
    if document.get("schemaVersion") != 1:
        errors.append("registry: schemaVersion must be 1")
    quirks_value = document.get("quirks")
    quirks = objects(quirks_value)
    if not isinstance(quirks_value, list) or len(quirks) != len(quirks_value):
        errors.append("registry: quirks must be an array of objects")
        return errors
    if not quirks:
        errors.append("registry: at least one quirk is required")
        return errors

    rows = manifest_rows(manifest_text)
    generator_path = root / GENERATOR_SCRIPT
    generator_functions, _ = source_symbols(generator_path)
    generated_cases = called_functions(generator_path, "generate")
    generator_literals = literal_assignments(generator_path)
    quirk_cases = generator_literals.get("QUIRK_CASES", {})
    seen: set[str] = set()

    for index, quirk in enumerate(quirks):
        quirk_id = quirk.get("id")
        label = quirk_id if nonempty(quirk_id) else f"quirks[{index}]"
        prefix = f"quirk {label}"
        errors.extend(unknown_fields(
            quirk,
            {"id", "symptom", "intake", "generator", "expectation",
             "regression", "mutations"},
            prefix,
        ))
        if not nonempty(quirk_id) or not ID.fullmatch(str(quirk_id)):
            errors.append(f"{prefix}: id must be lowercase words joined by hyphens")
        elif quirk_id in seen:
            errors.append(f"{prefix}: duplicate quirk id")
        else:
            seen.add(str(quirk_id))
        if not nonempty(quirk.get("symptom")):
            errors.append(f"{prefix}: symptom is required")

        intake = quirk.get("intake")
        if not isinstance(intake, dict):
            errors.append(f"{prefix}: synthetic intake record is required")
        else:
            errors.extend(unknown_fields(
                intake, {"kind", "patientDataRetained"}, f"{prefix} intake"
            ))
            if intake.get("kind") != "synthetic-reconstruction":
                errors.append(f"{prefix}: intake must be a synthetic reconstruction")
            if intake.get("patientDataRetained") is not False:
                errors.append(f"{prefix}: patient data may not be retained")

        declared_case: dict[str, Any] | None = None
        generator = quirk.get("generator")
        if not isinstance(generator, dict):
            errors.append(f"{prefix}: generator record is required")
        else:
            errors.extend(unknown_fields(
                generator, {"script", "case", "paths"}, f"{prefix} generator"
            ))
            if generator.get("script") != GENERATOR_SCRIPT:
                errors.append(f"{prefix}: generator script must be {GENERATOR_SCRIPT}")
            case = generator.get("case")
            if (
                not nonempty(case)
                or case not in generator_functions
                or case not in generated_cases
            ):
                errors.append(
                    f"{prefix}: {case!r} is not a callable generator case reached by generate"
                )
            declared_case = (
                quirk_cases.get(case) if isinstance(quirk_cases, dict) else None
            )
            if not isinstance(declared_case, dict) or not function_binds_quirk_path(
                    generator_path, str(case)):
                errors.append(
                    f"{prefix}: generator case {case!r} is not bound to QUIRK_CASES"
                )
            paths = generator.get("paths")
            if not isinstance(paths, list) or not paths or not all(
                    nonempty(path) for path in paths):
                errors.append(f"{prefix}: generator paths must be a non-empty array")
                paths = []
            declared_paths = (
                json_shape(declared_case.get("paths"))
                if isinstance(declared_case, dict)
                else None
            )
            if paths != declared_paths:
                errors.append(
                    f"{prefix}: generator paths do not match QUIRK_CASES for {case!r}"
                )
            if isinstance(declared_paths, list) and len(declared_paths) != 1:
                errors.append(
                    f"{prefix}: generator case binding writes exactly one declared path"
                )
            for manifest_path in paths:
                manifest_path = str(manifest_path)
                matching = rows.get(manifest_path, [])
                if len(matching) != 1:
                    errors.append(
                        f"{prefix}: {manifest_path} needs exactly one manifest row"
                    )
                elif matching[0].get("source") != GENERATOR_SOURCE:
                    errors.append(
                        f"{prefix}: {manifest_path} manifest row has the wrong generator source"
                    )
                tracked = f"corpus/data/{manifest_path}"
                if tracked in tracked_paths or manifest_path in tracked_paths:
                    errors.append(f"{prefix}: generated path is a tracked DICOM: {tracked}")

        expectation = quirk.get("expectation")
        if not isinstance(expectation, dict):
            errors.append(f"{prefix}: expectation record is required")
        else:
            errors.extend(unknown_fields(
                expectation,
                {"authority", "fixture", "inputs", "expectedDisplayValues"},
                f"{prefix} expectation",
            ))
            errors.extend(validate_authority(expectation.get("authority"), prefix))
            fixture = expectation.get("fixture")
            if not isinstance(fixture, dict):
                errors.append(f"{prefix}: expectation fixture is required")
            else:
                errors.extend(unknown_fields(
                    fixture,
                    {"path", "test", "boundary", "symbols"},
                    f"{prefix} fixture",
                ))
                fixture_path = fixture.get("path")
                fixture_test = fixture.get("test")
                if fixture_path != FIXTURE_SCRIPT:
                    errors.append(f"{prefix}: fixture path must be {FIXTURE_SCRIPT}")
                else:
                    _, test_names = source_symbols(root / str(fixture_path))
                    if not nonempty(fixture_test) or fixture_test not in test_names:
                        errors.append(
                            f"{prefix}: fixture test {fixture_test!r} is not present"
                        )
                    fixture_literals = literal_assignments(root / str(fixture_path))
                    fixture_functions, _ = source_symbols(root / str(fixture_path))
                    fixture_boundary = fixture.get("boundary")
                    symbols = fixture.get("symbols")
                    expected_symbols = {
                        **FIXTURE_INPUT_SYMBOLS,
                        "expectedDisplayValues": EXPECTED_VALUES_SYMBOL,
                    }
                    if symbols != expected_symbols:
                        errors.append(
                            f"{prefix}: fixture symbols must name the independent literals"
                        )
                    else:
                        if (
                            fixture_boundary != EXPECTATION_BOUNDARY
                            or fixture_boundary not in fixture_functions
                            or not test_calls_boundary(
                                root / str(fixture_path),
                                str(fixture_test),
                                str(fixture_boundary),
                            )
                        ):
                            errors.append(
                                f"{prefix}: fixture test must execute its declared boundary"
                            )
                        literal_inputs = {
                            key: json_shape(fixture_literals.get(symbol))
                            for key, symbol in FIXTURE_INPUT_SYMBOLS.items()
                        }
                        if expectation.get("inputs") != literal_inputs:
                            errors.append(
                                f"{prefix}: expectation inputs differ from fixture literals"
                            )
                        literal_values = json_shape(
                            fixture_literals.get(EXPECTED_VALUES_SYMBOL)
                        )
                        if expectation.get("expectedDisplayValues") != literal_values:
                            errors.append(
                                f"{prefix}: expected display values differ from fixture literals"
                            )
            if not isinstance(expectation.get("inputs"), dict) or not expectation["inputs"]:
                errors.append(f"{prefix}: independent expectation inputs are required")
            elif isinstance(declared_case, dict):
                inputs = expectation["inputs"]
                try:
                    generator_centre = float(declared_case["windowCentre"])
                    generator_width = float(declared_case["windowWidth"])
                except (KeyError, TypeError, ValueError):
                    errors.append(
                        f"{prefix}: QUIRK_CASES needs numeric window coordinates"
                    )
                else:
                    if (
                        inputs.get("centre") != generator_centre
                        or inputs.get("width") != generator_width
                    ):
                        errors.append(
                            f"{prefix}: expectation window differs from generator recipe"
                        )
            values = expectation.get("expectedDisplayValues")
            if not isinstance(values, list) or not values or not all(
                    isinstance(value, (int, float)) and not isinstance(value, bool)
                    for value in values):
                errors.append(f"{prefix}: literal expected display values are required")

        regression = quirk.get("regression")
        if not isinstance(regression, dict):
            errors.append(f"{prefix}: regression record is required")
        else:
            regression_contract = REGRESSION_CONTRACTS.get(str(quirk_id))
            if regression_contract is None or regression != regression_contract:
                errors.append(
                    f"{prefix}: regression differs from its checker-owned contract"
                )

        mutations_value = quirk.get("mutations")
        mutations = objects(mutations_value)
        if (
            not isinstance(mutations_value, list)
            or not mutations
            or len(mutations) != len(mutations_value)
        ):
            errors.append(f"{prefix}: mutation evidence must be a non-empty array")
        contract = MUTATION_CONTRACTS.get(str(quirk_id))
        if contract is None:
            errors.append(f"{prefix}: no checker-owned mutation contract exists")
            contract = {}
        seen_mutations: set[str] = set()
        for mutation in mutations:
            kind = mutation.get("kind")
            if kind not in MUTATION_KINDS:
                errors.append(f"{prefix}: mutation kind {kind!r} is not understood")
                continue
            if kind in seen_mutations:
                errors.append(f"{prefix}: mutation {kind} is duplicated")
            seen_mutations.add(str(kind))
            expected_mutation = contract.get(str(kind))
            if mutation != ({"kind": kind, **expected_mutation}
                            if expected_mutation is not None else None):
                errors.append(
                    f"{prefix}: mutation {kind} differs from its checker-owned contract"
                )
                continue
            boundary = mutation.get("boundary")
            signature = mutation.get("failureSignature")
            if not nonempty(boundary) or not nonempty(signature):
                errors.append(
                    f"{prefix}: mutation {kind} needs a boundary and failure signature"
                )
                continue
            if kind in {"generator-voi-function", "generator-window-width"}:
                functions, _ = source_symbols(
                    root / GENERATOR_MUTATION_SCRIPT
                )
                boundary_function = str(boundary).rsplit(".", maxsplit=1)[-1]
                if boundary_function not in functions:
                    errors.append(f"{prefix}: mutation boundary {boundary!r} is not present")
                if "value" not in mutation:
                    errors.append(f"{prefix}: mutation {kind} needs a replacement value")
            else:
                rust_test = str(boundary).rsplit("::", maxsplit=1)[-1]
                try:
                    attribution = (root / ATTRIBUTION_SCRIPT).read_text(
                        encoding="utf-8"
                    )
                except OSError:
                    attribution = ""
                if f"fn {rust_test}(" not in attribution:
                    errors.append(f"{prefix}: mutation boundary {boundary!r} is not present")
        missing_mutations = set(contract) - seen_mutations
        for kind in sorted(missing_mutations):
            errors.append(f"{prefix}: required mutation {kind} is missing")
        unexpected_mutations = seen_mutations - set(contract)
        for kind in sorted(unexpected_mutations):
            errors.append(f"{prefix}: mutation {kind} has no contract")
    return errors


def git_tracked_paths(root: Path) -> set[str]:
    result = subprocess.run(
        ["git", "ls-files"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or "git ls-files failed")
    return set(result.stdout.splitlines())


def main() -> int:
    try:
        document = json.loads(REGISTRY.read_text(encoding="utf-8"))
        manifest = MANIFEST.read_text(encoding="utf-8")
        tracked = git_tracked_paths(ROOT)
    except (OSError, json.JSONDecodeError, RuntimeError) as error:
        print(f"quirk check refused: {error}", file=sys.stderr)
        return 1
    errors = validate_document(
        document,
        root=ROOT,
        manifest_text=manifest,
        tracked_paths=tracked,
    )
    if errors:
        for error in errors:
            print(f"quirk check refused: {error}", file=sys.stderr)
        return 1
    print(f"OK: {len(document['quirks'])} permanent quirk capture(s) are complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
