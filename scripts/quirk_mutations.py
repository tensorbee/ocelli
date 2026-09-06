#!/usr/bin/env python3
"""Execute the checker-owned F-014 mutations in a disposable repository."""

from __future__ import annotations

import ast
import json
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import corpus_tests  # noqa: E402
import quirk_check  # noqa: E402
from guards.sandbox import Sandbox, SandboxError, sandbox  # noqa: E402


@dataclass(frozen=True)
class Mutation:
    kind: str
    path: str
    old: str
    new: str
    argv: tuple[str, ...]
    failure_signature: str
    replacement_value: object | None = None
    target_symbol: str | None = None


PYTHON_BOUNDARY = (quirk_check.GENERATOR_MUTATION_SCRIPT,)
FIXTURE_MODULE = Path(quirk_check.FIXTURE_SCRIPT).stem
FIXTURE_DIRECTORY = Path(quirk_check.FIXTURE_SCRIPT).parent.as_posix()
FIXTURE_BOUNDARY = (
    "-c",
    f"import sys; sys.path.insert(0, {FIXTURE_DIRECTORY!r}); "
    f"from {FIXTURE_MODULE} import {quirk_check.EXPECTATION_BOUNDARY}; "
    f"{quirk_check.EXPECTATION_BOUNDARY}()",
)
RUST_BOUNDARY = tuple(quirk_check.REGRESSION_COMMAND.split())


def mutations(python: Path) -> tuple[Mutation, ...]:
    """Fixed edits and invocations. No executable field comes from JSON."""
    return (
        Mutation(
            kind="generator-voi-function",
            path=quirk_check.GENERATOR_SCRIPT,
            old='"voiLutFunction": "SIGMOID",',
            new='"voiLutFunction": "LINEAR",',
            argv=(str(python), *PYTHON_BOUNDARY),
            failure_signature="expected SIGMOID, found LINEAR",
            replacement_value="LINEAR",
        ),
        Mutation(
            kind="generator-window-width",
            path=quirk_check.GENERATOR_SCRIPT,
            old='"windowWidth": "0.5",',
            new='"windowWidth": "1",',
            argv=(str(python), *PYTHON_BOUNDARY),
            failure_signature="expected width 0.5, found 1.0",
            replacement_value=1.0,
        ),
        Mutation(
            kind="disable-reference-attribution",
            path=quirk_check.ATTRIBUTION_SCRIPT,
            old=(
                "        let attributed = compare_view(\n"
                "            &Context {\n"
                "                reference: &reference_run,\n"
                "                candidate: &candidate_run,\n"
                "                register: &register,\n"
                "                low_information: &empty,"
            ),
            new=(
                "        let attributed = compare_view(\n"
                "            &Context {\n"
                "                reference: &reference_run,\n"
                "                candidate: &candidate_run,\n"
                "                register: &Register::default(),\n"
                "                low_information: &empty,"
            ),
            argv=RUST_BOUNDARY,
            failure_signature="left: Fail",
        ),
    )


def fixture_binding_mutations(python: Path) -> tuple[Mutation, ...]:
    """One fixed mutation for every literal the fixture record names."""
    specifications = (
        (
            "fixture-modality-values",
            "SIGMOID_MODALITY_VALUES",
            "SIGMOID_MODALITY_VALUES = (39.5, 39.75, 40.0, 40.25, 40.5)",
            "SIGMOID_MODALITY_VALUES = (39.0, 39.75, 40.0, 40.25, 40.5)",
        ),
        ("fixture-centre", "SIGMOID_CENTRE",
         "SIGMOID_CENTRE = 40.0", "SIGMOID_CENTRE = 41.0"),
        ("fixture-width", "SIGMOID_WIDTH",
         "SIGMOID_WIDTH = 0.5", "SIGMOID_WIDTH = 0.75"),
        (
            "fixture-display-minimum",
            "SIGMOID_DISPLAY_MINIMUM",
            "SIGMOID_DISPLAY_MINIMUM = 0.0",
            "SIGMOID_DISPLAY_MINIMUM = 1.0",
        ),
        (
            "fixture-display-maximum",
            "SIGMOID_DISPLAY_MAXIMUM",
            "SIGMOID_DISPLAY_MAXIMUM = 255.0",
            "SIGMOID_DISPLAY_MAXIMUM = 254.0",
        ),
        (
            "fixture-expected-values",
            "SIGMOID_DISPLAY_VALUES",
            "SIGMOID_DISPLAY_VALUES = (\n"
            "    4.586483540333347,\n"
            "    30.396745115639977,\n"
            "    127.5,\n"
            "    224.60325488436,\n"
            "    250.41351645966665,\n"
            ")",
            "SIGMOID_DISPLAY_VALUES = (\n"
            "    0.0,\n"
            "    30.396745115639977,\n"
            "    127.5,\n"
            "    224.60325488436,\n"
            "    250.41351645966665,\n"
            ")",
        ),
    )
    return tuple(
        Mutation(
            kind=kind,
            path=quirk_check.FIXTURE_SCRIPT,
            old=old,
            new=new,
            argv=(str(python), *FIXTURE_BOUNDARY),
            failure_signature="SIGMOID expectation fixture mismatch",
            target_symbol=symbol,
        )
        for kind, symbol, old, new in specifications
    )


def require_fixture_binding_contract(declared: tuple[Mutation, ...]) -> None:
    expected = set(quirk_check.FIXTURE_INPUT_SYMBOLS.values()) | {
        quirk_check.EXPECTED_VALUES_SYMBOL
    }
    actual = {mutation.target_symbol for mutation in declared}

    def assigned_name(source: str) -> str | None:
        try:
            statement = ast.parse(source).body
        except SyntaxError:
            return None
        if (
            len(statement) != 1
            or not isinstance(statement[0], ast.Assign)
            or len(statement[0].targets) != 1
            or not isinstance(statement[0].targets[0], ast.Name)
        ):
            return None
        return statement[0].targets[0].id

    coordinates_match = all(
        mutation.path == quirk_check.FIXTURE_SCRIPT
        and mutation.argv[1:] == FIXTURE_BOUNDARY
        and mutation.failure_signature == "SIGMOID expectation fixture mismatch"
        and assigned_name(mutation.old) == mutation.target_symbol
        and assigned_name(mutation.new) == mutation.target_symbol
        for mutation in declared
    )
    if actual != expected or None in actual or not coordinates_match:
        raise RuntimeError(
            "fixture mutation targets differ from the checker symbol contract"
        )


def require_registry_contract(
    document: dict[str, object], declared: tuple[Mutation, ...]
) -> None:
    record = next(
        item for item in document["quirks"]
        if item.get("id") == "sigmoid-width-below-one"
    )
    contract = quirk_check.MUTATION_CONTRACTS["sigmoid-width-below-one"]
    executable = {mutation.kind: mutation for mutation in declared}
    expected = {
        kind: {"kind": kind, **specification}
        for kind, specification in contract.items()
    }
    if set(executable) != set(contract):
        raise RuntimeError("executable mutation kinds differ from the checker contract")
    for kind, specification in contract.items():
        mutation = executable[kind]
        if mutation.failure_signature != specification["failureSignature"]:
            raise RuntimeError(
                f"executable mutation {kind} has a different failure signature"
            )
        if mutation.replacement_value != specification.get("value"):
            raise RuntimeError(
                f"executable mutation {kind} has a different replacement value"
            )
    actual = {item.get("kind"): item for item in record["mutations"]}
    if actual != expected:
        raise RuntimeError(
            "registry mutation evidence differs from the executable contracts"
        )
    attribution = executable["disable-reference-attribution"]
    executable_regression = {
        "kind": "oracle-comparator-test",
        "command": " ".join(attribution.argv),
        "failureSignature": attribution.failure_signature,
    }
    if record.get("regression") != executable_regression:
        raise RuntimeError(
            "registry regression differs from the executable attribution mutation"
        )


def require_green(box: Sandbox, argv: tuple[str, ...], label: str) -> None:
    result = box.run(list(argv), timeout=900)
    if result.returncode != 0:
        output = result.stdout + result.stderr
        raise RuntimeError(f"{label} control was red:\n{output}")


def require_red(box: Sandbox, mutation: Mutation) -> None:
    box.substitute(mutation.path, mutation.old, mutation.new)
    result = box.run(list(mutation.argv), timeout=900)
    output = result.stdout + result.stderr
    if result.returncode == 0:
        raise RuntimeError(f"{mutation.kind} mutation stayed green")
    if mutation.failure_signature not in output:
        raise RuntimeError(
            f"{mutation.kind} failed for the wrong reason. Expected "
            f"{mutation.failure_signature!r}:\n{output}"
        )
    print(f"RED: {mutation.kind}: {mutation.failure_signature}")


def main() -> int:
    python, reason = corpus_tests.resolve()
    if python is None:
        print(f"quirk mutations failed: no DICOM interpreter\n{reason}", file=sys.stderr)
        return 1
    if shutil.which("cargo") is None:
        print("quirk mutations failed: cargo is absent", file=sys.stderr)
        return 1

    declared = mutations(python)
    fixture_bindings = fixture_binding_mutations(python)
    try:
        document = json.loads((ROOT / "corpus" / "quirks.json").read_text())
        require_registry_contract(document, declared)
        require_fixture_binding_contract(fixture_bindings)
        with sandbox() as box:
            require_green(box, declared[0].argv, "SIGMOID generator")
            require_red(box, declared[0])
            box.reset()
            require_red(box, declared[1])
            box.reset()
            require_green(box, fixture_bindings[0].argv, "expectation fixture")
            for binding in fixture_bindings:
                require_red(box, binding)
                box.reset()
            require_green(box, declared[2].argv, "reference attribution")
            require_red(box, declared[2])
    except (OSError, RuntimeError, SandboxError) as error:
        print(f"quirk mutations failed: {error}", file=sys.stderr)
        return 1
    print(
        f"OK: {len(declared)} controlled mutations and "
        f"{len(fixture_bindings)} fixture-binding mutations went red"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
