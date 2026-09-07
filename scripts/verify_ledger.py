#!/usr/bin/env python3
"""Verification evidence, recorded against a TREE and carried in a trailer.

This file is the mechanism behind two things at once.

**HLD section 27.2 R6, the provenance trailer.** "Cheap now, a retrofit across
sixty thousand lines is not, and a device pathway may require it." Under
agent-assisted development this is what lets you answer a diligence question
about what was generated and how it was verified.

**Deviation D-04**, `docs/hld/DEVIATIONS.md`. CI runs no GPU, so the corpus
renders locally. That moves the gate onto a human remembering to run it, and
this is what stops it being a memory problem.

## Why the TREE hash and not the commit

A trailer cannot name the commit it is part of, and a ledger keyed on a commit
sha is written after the fact by definition. The tree hash is available BEFORE
the commit exists, is identical for an amend that changes only the message, and
changes the instant one byte of content changes.

So the chain is:

    /verify runs the gates  ->  records the result against `git write-tree`
    .githooks/pre-commit    ->  looks up the staged tree, refuses if absent,
                                appends the trailer it found there
    CI (no GPU)             ->  re-reads the trailer on the pushed head

The trailer cannot be hand-written to satisfy CI, because the hook only emits
one it found in a ledger entry that a real gate run wrote. That matters more
than it looks: a check the process CLAIMS to run and does not is worth LESS
than no check at all, because the records then read as though it ran and
nobody goes looking.

The ledger is per-clone evidence and is gitignored. The TRAILER is the shared,
committed artefact.

Usage:
  python3 scripts/verify_ledger.py record --gates fmt,clippy --corpus pass
  python3 scripts/verify_ledger.py assert                 # staged tree is green
  python3 scripts/verify_ledger.py trailer                # emit trailer lines
  python3 scripts/verify_ledger.py check-commit HEAD      # CI side, no GPU
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / ".claude" / "verify-ledger.json"
REPORT_CONTRACT_PATH = ROOT / "tools" / "oracle" / "report-contract.json"

TRAILER_VERIFY = "Ocelli-Verify"
TRAILER_AGENT = "Ocelli-Generated-By"
CORPUS_STATES = {"pass", "fail", "absent", "skipped"}


class DuplicateJsonKey(ValueError):
    pass


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKey(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _invalid_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON number {value}")


def _contract_object(value: object, label: str, keys: set[str]) -> dict:
    if not isinstance(value, dict) or set(value) != keys:
        actual = set(value) if isinstance(value, dict) else set()
        raise ValueError(
            f"{label} has invalid keys, missing={sorted(keys - actual)}, "
            f"unknown={sorted(actual - keys)}"
        )
    return value


def _contract_string_array(value: object, label: str) -> list[str]:
    if (not isinstance(value, list)
            or any(not isinstance(item, str) or not item for item in value)
            or len(set(value)) != len(value)):
        raise ValueError(f"{label} is not a unique non-empty string array")
    return value


def _load_report_contract() -> dict:
    root_keys = {
        "version", "schemas", "vocabularies", "semantics",
        "hashAlgorithms", "greenReport",
    }
    schema_keys = {
        "report", "coverage", "record", "statistics", "channel",
        "parameterDivergence", "geometryDivergence", "renderHashes",
        "greenUnmeasuredState",
    }
    vocabulary_keys = {
        "kinds", "toleranceClasses", "outcomes", "sides", "rungs",
        "qualifiers",
    }
    semantic_keys = {
        "channelCountByClass", "greenUnmeasuredQualifiers",
        "greenUnmeasuredStates",
        "monochromeWithinOneLsbFraction", "monochromeMaxAbsDiff",
        "monochromeSignedMeanBias", "informativeFractionFloor",
    }
    try:
        contract = json.loads(
            REPORT_CONTRACT_PATH.read_bytes(),
            object_pairs_hook=_unique_object,
            parse_constant=_invalid_constant,
        )
        contract = _contract_object(contract, "root", root_keys)
        if isinstance(contract["version"], bool) or contract["version"] != 1:
            raise ValueError("version is not the integer 1")
        schemas = _contract_object(contract["schemas"], "schemas", schema_keys)
        vocabularies = _contract_object(
            contract["vocabularies"], "vocabularies", vocabulary_keys
        )
        semantics = _contract_object(
            contract["semantics"], "semantics", semantic_keys
        )
        algorithms = _contract_object(
            contract["hashAlgorithms"], "hashAlgorithms", {"view", "run"}
        )
        if any(not isinstance(value, str) or not value
               for value in algorithms.values()):
            raise ValueError("hashAlgorithms values are not non-empty strings")
        if not isinstance(contract["greenReport"], dict):
            raise ValueError("greenReport is not an object")
        for name, value in schemas.items():
            _contract_string_array(value, f"schemas.{name}")
        for name, value in vocabularies.items():
            _contract_string_array(value, f"vocabularies.{name}")
        classes = _contract_object(
            semantics["channelCountByClass"],
            "semantics.channelCountByClass",
            set(vocabularies["toleranceClasses"]),
        )
        if any(isinstance(value, bool) or not isinstance(value, int) or value <= 0
               for value in classes.values()):
            raise ValueError("channelCountByClass values are not positive integers")
        green_qualifiers = _contract_string_array(
            semantics["greenUnmeasuredQualifiers"],
            "semantics.greenUnmeasuredQualifiers",
        )
        if (not green_qualifiers
                or any(item not in vocabularies["qualifiers"]
                       for item in green_qualifiers)):
            raise ValueError("greenUnmeasuredQualifiers contains an invalid value")
        for name in (
            "monochromeWithinOneLsbFraction", "monochromeMaxAbsDiff",
            "monochromeSignedMeanBias", "informativeFractionFloor",
        ):
            value = semantics[name]
            if (isinstance(value, bool) or not isinstance(value, (int, float))
                    or not math.isfinite(value) or value < 0):
                raise ValueError(f"semantics.{name} is not a finite non-negative number")
        states = semantics["greenUnmeasuredStates"]
        if not isinstance(states, list) or not states:
            raise ValueError("semantics.greenUnmeasuredStates is not an array")
        state_keys = set(schemas["greenUnmeasuredState"])
        qualifier_order = vocabularies["qualifiers"]
        seen_states = set()
        for index, raw_state in enumerate(states):
            state = _contract_object(
                raw_state,
                f"semantics.greenUnmeasuredStates[{index}]",
                state_keys,
            )
            tolerance_class = state["toleranceClass"]
            qualifiers = _contract_string_array(
                state["qualifiers"],
                f"semantics.greenUnmeasuredStates[{index}].qualifiers",
            )
            rung = state["rung"]
            if tolerance_class not in vocabularies["toleranceClasses"]:
                raise ValueError("green unmeasured state has an invalid class")
            if (not qualifiers
                    or any(item not in semantics["greenUnmeasuredQualifiers"]
                           for item in qualifiers)
                    or qualifiers != sorted(qualifiers, key=qualifier_order.index)):
                raise ValueError("green unmeasured state has invalid qualifiers")
            if rung not in vocabularies["rungs"]:
                raise ValueError("green unmeasured state has an invalid rung")
            identity = (tolerance_class, tuple(qualifiers))
            if identity in seen_states:
                raise ValueError("green unmeasured states contain a duplicate")
            seen_states.add(identity)
        return contract
    except (OSError, json.JSONDecodeError, DuplicateJsonKey, ValueError) as error:
        sys.exit(f"report contract is invalid: {REPORT_CONTRACT_PATH}: {error}")


REPORT_CONTRACT = _load_report_contract()
REPORT_SCHEMAS = REPORT_CONTRACT["schemas"]
REPORT_VOCABULARIES = REPORT_CONTRACT["vocabularies"]
REPORT_SEMANTICS = REPORT_CONTRACT["semantics"]
REPORT_HASH_ALGORITHMS = REPORT_CONTRACT["hashAlgorithms"]

REPORT_KEYS = set(REPORT_SCHEMAS["report"])
COVERAGE_KEYS = set(REPORT_SCHEMAS["coverage"])
RECORD_KEYS = set(REPORT_SCHEMAS["record"])
STATISTICS_KEYS = set(REPORT_SCHEMAS["statistics"])
CHANNEL_KEYS = set(REPORT_SCHEMAS["channel"])
PARAMETER_DIVERGENCE_KEYS = set(REPORT_SCHEMAS["parameterDivergence"])
GEOMETRY_DIVERGENCE_KEYS = set(REPORT_SCHEMAS["geometryDivergence"])
RENDER_HASH_KEYS = set(REPORT_SCHEMAS["renderHashes"])
HASH_ALGORITHM = REPORT_HASH_ALGORITHMS["view"]
RUN_HASH_ALGORITHM = REPORT_HASH_ALGORITHMS["run"]
HASH_PATTERN = re.compile(r"[0-9a-f]{64}")
KINDS = {label: index for index, label in enumerate(REPORT_VOCABULARIES["kinds"])}
CLASSES = REPORT_SEMANTICS["channelCountByClass"]
OUTCOMES = set(REPORT_VOCABULARIES["outcomes"])
SIDES = set(REPORT_VOCABULARIES["sides"])
RUNGS = set(REPORT_VOCABULARIES["rungs"])
QUALIFIERS = tuple(REPORT_VOCABULARIES["qualifiers"])
GREEN_UNMEASURED_QUALIFIERS = set(
    REPORT_SEMANTICS["greenUnmeasuredQualifiers"]
)
GREEN_UNMEASURED_STATES = {
    (state["toleranceClass"], tuple(state["qualifiers"])): state["rung"]
    for state in REPORT_SEMANTICS["greenUnmeasuredStates"]
}
MONOCHROME_WITHIN_ONE_LSB_FRACTION = REPORT_SEMANTICS[
    "monochromeWithinOneLsbFraction"
]
MONOCHROME_MAX_ABS_DIFF = REPORT_SEMANTICS["monochromeMaxAbsDiff"]
MONOCHROME_SIGNED_MEAN_BIAS = REPORT_SEMANTICS["monochromeSignedMeanBias"]
INFORMATIVE_FRACTION_FLOOR = REPORT_SEMANTICS["informativeFractionFloor"]
U64_MAX = (1 << 64) - 1
U32_MAX = (1 << 32) - 1
I32_MIN = -(1 << 31)
I32_MAX = (1 << 31) - 1


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True,
                          text=True, check=True).stdout.strip()


def staged_tree() -> str:
    """Tree hash of the index. Available before the commit exists."""
    return git("write-tree")


def load() -> dict:
    if not LEDGER.exists():
        return {}
    try:
        return json.loads(LEDGER.read_text())
    except json.JSONDecodeError:
        return {}


def save(data: dict) -> None:
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(data, indent=1, sort_keys=True) + "\n")


def _schema(value: object, label: str, keys: set[str]) -> dict:
    if not isinstance(value, dict):
        sys.exit(f"comparison report {label} is not an object")
    missing = sorted(keys - value.keys())
    unknown = sorted(value.keys() - keys)
    if missing or unknown:
        sys.exit(
            f"comparison report {label} has invalid keys, "
            f"missing={missing}, unknown={unknown}"
        )
    return value


def _integer(value: object, label: str, *, maximum: int = U64_MAX) -> int:
    if (isinstance(value, bool) or not isinstance(value, int)
            or value < 0 or value > maximum):
        sys.exit(f"comparison report has invalid {label}")
    return value


def _number(value: object, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        sys.exit(f"comparison report has invalid {label}")
    try:
        finite = math.isfinite(value)
    except OverflowError:
        finite = False
    if not finite:
        sys.exit(f"comparison report has invalid {label}")
    return float(value)


def _derived_number(value: object, label: str) -> float:
    number = _number(value, label)
    if number == 0.0 and math.copysign(1.0, number) < 0.0:
        sys.exit(f"comparison report {label} is negative zero")
    return number


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        sys.exit(f"comparison report has invalid {label}")
    return value


def _array(value: object, label: str) -> list:
    if not isinstance(value, list):
        sys.exit(f"comparison report has no {label} array")
    return value


def _hash(value: object, label: str) -> str:
    if not isinstance(value, str) or HASH_PATTERN.fullmatch(value) is None:
        sys.exit(f"comparison report has invalid {label} render hash")
    return value


def _signed_histogram(value: object, label: str) -> tuple[list[int], int, int]:
    entries = _array(value, label)
    if not entries:
        sys.exit(f"comparison report {label} is empty")
    absolute = [0] * 256
    total = 0
    signed_sum = 0
    previous = -256
    for index, entry in enumerate(entries):
        entry_label = f"{label}[{index}]"
        if not isinstance(entry, list) or len(entry) != 2:
            sys.exit(f"comparison report has invalid {entry_label}")
        difference, raw_count = entry
        if (isinstance(difference, bool) or not isinstance(difference, int)
                or not -255 <= difference <= 255 or difference <= previous):
            sys.exit(f"comparison report has invalid {entry_label} difference")
        count = _integer(raw_count, f"{entry_label} count", maximum=U32_MAX)
        if count == 0:
            sys.exit(f"comparison report {entry_label} count is zero")
        previous = difference
        absolute[abs(difference)] += count
        total += count
        signed_sum += difference * count
    return absolute, total, signed_sum


def _channel_report(value: object, label: str) -> dict:
    channel = _schema(value, label, CHANNEL_KEYS)
    pixels = _integer(channel["pixels"], f"{label}.pixels", maximum=U32_MAX)
    histogram, histogram_pixels, signed_sum = _signed_histogram(
        channel["signedHistogram"], f"{label}.signedHistogram"
    )
    if histogram_pixels != pixels:
        sys.exit(f"comparison report {label} signed histogram does not total pixels")
    counts = [
        _integer(channel[key], f"{label}.{key}", maximum=U32_MAX)
        for key in ("countAtZero", "countAtOne", "countAtTwo", "countOverTwo")
    ]
    if sum(counts) != pixels:
        sys.exit(f"comparison report {label} channel counts do not total pixels")
    expected_counts = [histogram[0], histogram[1], histogram[2], sum(histogram[3:])]
    if counts != expected_counts:
        sys.exit(f"comparison report {label} channel counts contradict signed histogram")
    maximum = _integer(channel["maxAbsDiff"], f"{label}.maxAbsDiff", maximum=255)
    percentile = _integer(
        channel["percentile999AbsDiff"],
        f"{label}.percentile999AbsDiff",
        maximum=255,
    )
    within = _derived_number(channel["fractionWithinOneLsb"],
                             f"{label}.fractionWithinOneLsb")
    differing = _derived_number(channel["differingFraction"],
                                f"{label}.differingFraction")
    signed_mean = _derived_number(
        channel["signedMeanDiff"], f"{label}.signedMeanDiff"
    )
    if pixels == 0 or not 0.0 <= within <= 1.0 or not 0.0 <= differing <= 1.0:
        sys.exit(f"comparison report has invalid {label} channel fractions")
    expected_within = (counts[0] + counts[1]) / pixels
    expected_differing = (pixels - counts[0]) / pixels
    if within != expected_within or differing != expected_differing:
        sys.exit(f"comparison report {label} channel fractions contradict counts")
    expected_maximum = next(
        difference for difference in range(255, -1, -1)
        if histogram[difference] > 0
    )
    if maximum != expected_maximum:
        sys.exit(f"comparison report {label} maximum contradicts signed histogram")
    cumulative = 0
    expected_percentile = 255
    for difference, count in enumerate(histogram):
        cumulative += count
        if cumulative / pixels >= MONOCHROME_WITHIN_ONE_LSB_FRACTION:
            expected_percentile = difference
            break
    if percentile != expected_percentile:
        sys.exit(f"comparison report {label} percentile contradicts signed histogram")
    if not I32_MIN <= signed_sum <= I32_MAX:
        sys.exit(f"comparison report {label} signed sum exceeds producer range")
    if signed_mean != signed_sum / pixels:
        sys.exit(f"comparison report {label} signed mean contradicts signed histogram")
    return channel


def _statistics(value: object, record_id: str, tolerance_class: str) -> dict:
    label = f"record {record_id!r} statistics"
    statistics = _schema(value, label, STATISTICS_KEYS)
    channels = _integer(statistics["channels"], f"{label}.channels")
    if channels != CLASSES[tolerance_class]:
        sys.exit(f"comparison report {label} channels contradict tolerance class")

    regions = {}
    for region in ("full", "image", "background", "informative"):
        raw = _array(statistics[region], f"{label}.{region}")
        if region in ("full", "image") and len(raw) != channels:
            sys.exit(f"comparison report {label}.{region} has the wrong channel count")
        if region in ("background", "informative") and len(raw) not in (0, channels):
            sys.exit(f"comparison report {label}.{region} has the wrong channel count")
        regions[region] = [
            _channel_report(entry, f"{label}.{region}[{index}]")
            for index, entry in enumerate(raw)
        ]

    rows = _integer(statistics["rowsTouched"], f"{label}.rowsTouched",
                    maximum=U32_MAX)
    columns = _integer(statistics["columnsTouched"],
                       f"{label}.columnsTouched", maximum=U32_MAX)
    frame_rows = _integer(
        statistics["frameRows"], f"{label}.frameRows", maximum=U32_MAX
    )
    frame_columns = _integer(
        statistics["frameColumns"], f"{label}.frameColumns", maximum=U32_MAX
    )
    image_rows = _integer(
        statistics["imageRows"], f"{label}.imageRows", maximum=U32_MAX
    )
    image_columns = _integer(
        statistics["imageColumns"], f"{label}.imageColumns", maximum=U32_MAX
    )
    image_pixels = _integer(
        statistics["imagePixels"], f"{label}.imagePixels", maximum=U32_MAX
    )
    informative_pixels = _integer(
        statistics["informativePixels"],
        f"{label}.informativePixels",
        maximum=U32_MAX,
    )
    informative_fraction = _derived_number(
        statistics["informativeFraction"], f"{label}.informativeFraction")
    if image_pixels == 0 or informative_pixels > image_pixels:
        sys.exit(f"comparison report {label} has invalid region pixel counts")
    if informative_fraction != informative_pixels / image_pixels:
        sys.exit(f"comparison report {label} informative fraction contradicts counts")
    if not isinstance(statistics["predicatePasses"], bool):
        sys.exit(f"comparison report has invalid {label}.predicatePasses")
    if not isinstance(statistics["biasPasses"], bool):
        sys.exit(f"comparison report has invalid {label}.biasPasses")
    signed_mean = _derived_number(
        statistics["signedMeanDiff"], f"{label}.signedMeanDiff"
    )

    full_pixels = {entry["pixels"] for entry in regions["full"]}
    image_region_pixels = {entry["pixels"] for entry in regions["image"]}
    background_pixels = {entry["pixels"] for entry in regions["background"]}
    informative_region_pixels = {entry["pixels"] for entry in regions["informative"]}
    if len(full_pixels) != 1 or image_region_pixels != {image_pixels}:
        sys.exit(f"comparison report {label} region totals contradict channel reports")
    full_pixel_count = next(iter(full_pixels))
    if (frame_rows == 0 or frame_columns == 0
            or image_rows == 0 or image_columns == 0
            or frame_rows * frame_columns != full_pixel_count
            or image_rows * image_columns != image_pixels
            or image_rows > frame_rows or image_columns > frame_columns):
        sys.exit(f"comparison report {label} frame and image dimensions contradict regions")
    expected_background = full_pixel_count - image_pixels
    if expected_background < 0:
        sys.exit(f"comparison report {label} image exceeds the full frame")
    if background_pixels not in (set(), {expected_background}):
        sys.exit(f"comparison report {label} background total is inconsistent")
    if expected_background > 0 and not background_pixels:
        sys.exit(f"comparison report {label} omits non-empty background statistics")
    expected_informative = set() if informative_pixels == 0 else {informative_pixels}
    if informative_region_pixels != expected_informative:
        sys.exit(f"comparison report {label} informative total is inconsistent")
    if rows > frame_rows or columns > frame_columns:
        sys.exit(f"comparison report {label} touched counts exceed frame dimensions")
    if regions["background"]:
        for channel_index in range(channels):
            full = regions["full"][channel_index]
            image = regions["image"][channel_index]
            background = regions["background"][channel_index]
            combined_histogram: dict[int, int] = {}
            for entry in image["signedHistogram"] + background["signedHistogram"]:
                difference, count = entry
                combined_histogram[difference] = (
                    combined_histogram.get(difference, 0) + count
                )
            expected_histogram = [
                [difference, count]
                for difference, count in sorted(combined_histogram.items())
            ]
            if full["signedHistogram"] != expected_histogram:
                sys.exit(
                    f"comparison report {label} full signed histogram is not image plus background"
                )
    else:
        for channel_index in range(channels):
            full = regions["full"][channel_index]
            image = regions["image"][channel_index]
            if full != image:
                sys.exit(
                    f"comparison report {label} full region is not the whole image"
                )
    for channel_index in range(channels):
        image = regions["image"][channel_index]
        informative = (
            regions["informative"][channel_index]
            if regions["informative"] else None
        )
        image_histogram = dict(image["signedHistogram"])
        informative_histogram = (
            dict(informative["signedHistogram"]) if informative else {}
        )
        if informative:
            if any(
                count > image_histogram.get(difference, 0)
                for difference, count in informative["signedHistogram"]
            ):
                sys.exit(
                    f"comparison report {label} informative signed histogram exceeds image"
                )
        if any(
            difference != 0
            and informative_histogram.get(difference, 0) != count
            for difference, count in image["signedHistogram"]
        ):
            sys.exit(
                f"comparison report {label} informative signed histogram omits image differences"
            )
    any_difference = any(
        channel["countAtZero"] != channel["pixels"] for channel in regions["full"]
    )
    if any_difference != (rows > 0 and columns > 0):
        sys.exit(f"comparison report {label} touched counts contradict differences")
    differing_counts = [
        channel["pixels"] - channel["countAtZero"] for channel in regions["full"]
    ]
    if (rows > sum(differing_counts) or columns > sum(differing_counts)
            or rows * columns < max(differing_counts)):
        sys.exit(f"comparison report {label} touched counts contradict differing pixels")

    # A lane difference can occupy at most one distinct pixel, while differences
    # from different lanes may share one. Summing the lane counts is therefore a
    # safe upper bound on distinct differing pixels in each region. Keep the
    # regions separate here: pooling them loses the fact that an image confined
    # to one row cannot touch two rows when the background is unchanged.
    image_difference_samples = sum(
        channel["pixels"] - channel["countAtZero"]
        for channel in regions["image"]
    )
    background_difference_samples = sum(
        channel["pixels"] - channel["countAtZero"]
        for channel in regions["background"]
    )
    image_differing_pixels_upper = min(image_pixels, image_difference_samples)
    background_differing_pixels_upper = min(
        expected_background, background_difference_samples
    )

    image_rows_upper = min(image_rows, image_differing_pixels_upper)
    image_columns_upper = min(image_columns, image_differing_pixels_upper)
    background_rows_available = (
        frame_rows if image_columns < frame_columns else frame_rows - image_rows
    )
    background_columns_available = (
        frame_columns
        if image_rows < frame_rows
        else frame_columns - image_columns
    )
    background_rows_upper = min(
        background_rows_available, background_differing_pixels_upper
    )
    background_columns_upper = min(
        background_columns_available, background_differing_pixels_upper
    )
    if (rows > min(frame_rows, image_rows_upper + background_rows_upper)
            or columns > min(
                frame_columns, image_columns_upper + background_columns_upper
            )):
        sys.exit(
            f"comparison report {label} touched counts contradict region geometry"
        )

    if tolerance_class == "mono16":
        full = regions["full"][0]
        expected_predicate = (
            full["fractionWithinOneLsb"] >= MONOCHROME_WITHIN_ONE_LSB_FRACTION
            and full["countOverTwo"] == 0
        )
        if regions["informative"]:
            expected_signed_mean = regions["informative"][0]["signedMeanDiff"]
            expected_bias = abs(expected_signed_mean) <= MONOCHROME_SIGNED_MEAN_BIAS
        else:
            expected_signed_mean = 0.0
            expected_bias = True
    else:
        expected_predicate = True
        expected_bias = True
        expected_signed_mean = regions["image"][0]["signedMeanDiff"]
    if statistics["predicatePasses"] != expected_predicate:
        sys.exit(f"comparison report {label} predicate contradicts statistics")
    if statistics["biasPasses"] != expected_bias:
        sys.exit(f"comparison report {label} bias verdict contradicts statistics")
    if signed_mean != expected_signed_mean:
        sys.exit(f"comparison report {label} signed mean contradicts its source region")
    return statistics


def _parameter_divergence(value: object, label: str) -> None:
    divergence = _schema(value, label, PARAMETER_DIVERGENCE_KEYS)
    _text(divergence["field"], f"{label}.field")
    if (not isinstance(divergence["attributedTo"], str)
            or divergence["attributedTo"] not in SIDES):
        sys.exit(f"comparison report has invalid {label}.attributedTo")
    _text(divergence["why"], f"{label}.why")


def _geometry_divergence(value: object, label: str) -> None:
    divergence = _schema(value, label, GEOMETRY_DIVERGENCE_KEYS)
    _text(divergence["field"], f"{label}.field")
    for key in ("reference", "candidate", "difference", "bound"):
        number = _number(divergence[key], f"{label}.{key}")
        if key in ("difference", "bound") and number < 0:
            sys.exit(f"comparison report has invalid {label}.{key}")


def _record(value: object, index: int) -> dict:
    label = f"records[{index}]"
    record = _schema(value, label, RECORD_KEYS)
    record_id = _text(record["id"], f"{label}.id")
    kind = record["kind"]
    tolerance_class = record["toleranceClass"]
    outcome = record["outcome"]
    if not isinstance(kind, str) or kind not in KINDS:
        sys.exit(f"comparison report has invalid {label}.kind")
    if not isinstance(tolerance_class, str) or tolerance_class not in CLASSES:
        sys.exit(f"comparison report has invalid {label}.toleranceClass")
    if not isinstance(outcome, str) or outcome not in OUTCOMES:
        sys.exit(f"comparison report has invalid {label}.outcome")
    if (not isinstance(record["attributedTo"], str)
            or record["attributedTo"] not in SIDES):
        sys.exit(f"comparison report has invalid {label}.attributedTo")
    if not isinstance(record["rung"], str) or record["rung"] not in RUNGS:
        sys.exit(f"comparison report has invalid {label}.rung")
    if not isinstance(record["monochromeFrame"], bool):
        sys.exit(f"comparison report has invalid {label}.monochromeFrame")

    qualifiers = _array(record["qualifiers"], f"{label}.qualifiers")
    if (any(not isinstance(item, str) or item not in QUALIFIERS
            for item in qualifiers)
            or len(set(qualifiers)) != len(qualifiers)
            or qualifiers != sorted(qualifiers, key=QUALIFIERS.index)):
        sys.exit(f"comparison report has invalid {label}.qualifiers")
    notes = _array(record["notes"], f"{label}.notes")
    if any(not isinstance(note, str) or not note for note in notes):
        sys.exit(f"comparison report has invalid {label}.notes")

    parameters = _array(record["parameterDivergences"],
                        f"{label}.parameterDivergences")
    for item_index, item in enumerate(parameters):
        _parameter_divergence(item, f"{label}.parameterDivergences[{item_index}]")
    geometry = _array(record["geometryDivergences"],
                      f"{label}.geometryDivergences")
    for item_index, item in enumerate(geometry):
        _geometry_divergence(item, f"{label}.geometryDivergences[{item_index}]")
    register_entry = record["referenceDivergenceEntry"]
    if register_entry is not None:
        _text(register_entry, f"{label}.referenceDivergenceEntry")

    hashes = _schema(record["renderHashes"], f"{label}.renderHashes",
                     RENDER_HASH_KEYS)
    if hashes["algorithm"] != HASH_ALGORITHM:
        sys.exit(f"comparison report has invalid {label} render hash algorithm")
    _hash(hashes["reference"], f"{label} reference")
    _hash(hashes["candidate"], f"{label} candidate")
    _statistics(record["statistics"], record_id, tolerance_class)

    if not record["statistics"]["predicatePasses"] or not record["statistics"]["biasPasses"]:
        sys.exit(f"comparison report green record {record_id!r} has a failed predicate")
    if parameters or geometry or register_entry is not None:
        sys.exit(f"comparison report green record {record_id!r} has divergence details")
    if tolerance_class == "mono16" and not record["monochromeFrame"]:
        sys.exit(f"comparison report mono16 record {record_id!r} is not monochrome")
    if outcome == "pass":
        if (tolerance_class != "mono16" or qualifiers
                or record["attributedTo"] != "none"
                or record["rung"] != "pixels" or notes
                or record["statistics"]["informativePixels"] == 0):
            sys.exit(f"comparison report pass record {record_id!r} is inconsistent")
    elif outcome == "unmeasured":
        if (not qualifiers or not set(qualifiers) <= GREEN_UNMEASURED_QUALIFIERS
                or record["attributedTo"] != "none" or not notes):
            sys.exit(f"comparison report unmeasured record {record_id!r} is inconsistent")
        expected_rung = GREEN_UNMEASURED_STATES.get(
            (tolerance_class, tuple(qualifiers))
        )
        if expected_rung is None:
            sys.exit(
                f"comparison report unmeasured record {record_id!r} contradicts its class"
            )
        if record["rung"] != expected_rung:
            sys.exit(f"comparison report unmeasured record {record_id!r} has the wrong rung")
        weak = "weak" in qualifiers
        below_floor = record["statistics"]["informativeFraction"] < INFORMATIVE_FRACTION_FLOOR
        if weak and not below_floor:
            sys.exit(f"comparison report weak record {record_id!r} is not low-information")
    else:
        sys.exit(f"comparison report green record {record_id!r} has outcome {outcome}")
    return record


def _run_hash(records: list[dict], side: str) -> str:
    entries = sorted(
        records,
        key=lambda record: (
            KINDS[record["kind"]], record["id"], record["renderHashes"][side]
        ),
    )
    digest = hashlib.sha256()
    digest.update(RUN_HASH_ALGORITHM.encode() + b"\0")
    digest.update(len(entries).to_bytes(8, "little"))
    for record in entries:
        for field in (record["kind"], record["id"], record["renderHashes"][side]):
            encoded = field.encode()
            digest.update(len(encoded).to_bytes(8, "little"))
            digest.update(encoded)
    return digest.hexdigest()


def _resolved_directory(value: object, label: str) -> Path:
    text = _text(value, label)
    source = Path(text)
    if not source.is_absolute():
        source = ROOT / source
    try:
        resolved = source.resolve(strict=True)
    except OSError as error:
        sys.exit(f"comparison report {label} cannot be resolved: {error}")
    if not resolved.is_dir():
        sys.exit(f"comparison report {label} is not a directory")
    return resolved


def comparison_evidence(path: str) -> dict:
    report_path = Path(path)
    try:
        encoded = report_path.read_bytes()
    except OSError as error:
        sys.exit(f"comparison report cannot be read: {report_path}: {error}")
    try:
        report = json.loads(
            encoded,
            object_pairs_hook=_unique_object,
            parse_constant=_invalid_constant,
        )
    except (json.JSONDecodeError, DuplicateJsonKey, ValueError) as error:
        sys.exit(f"comparison report is not valid JSON: {report_path}: {error}")
    report = _schema(report, "root", REPORT_KEYS)
    if report.get("operation") != "gate":
        sys.exit("comparison report was not produced by the explicit candidate gate")
    if report.get("gateVerdict") != "pass" or report.get("green") is not True:
        sys.exit("comparison report is not green")

    if report["story"] != "F-011, F-012, F-015":
        sys.exit("comparison report has an invalid story set")
    reference = _resolved_directory(report["reference"], "reference directory")
    candidate = _resolved_directory(report["candidate"], "candidate directory")
    if reference == candidate:
        sys.exit("comparison report reference and candidate directories are equal")

    claimed = _integer(report["claimedVerdictViews"], "claimed verdict count")
    if claimed <= 0:
        sys.exit("comparison report judged zero views or has an invalid count")
    summaries = {
        outcome: _integer(report[outcome], f"{outcome} count")
        for outcome in ("pass", "fail", "unmeasured", "absent")
    }

    empty_arrays = {
        "problems": "problems",
        "coverageProblems": "coverage problems",
        "absorbedDivergences": "absorbed divergences",
    }
    for field, label in empty_arrays.items():
        value = report.get(field)
        if not isinstance(value, list):
            sys.exit(f"comparison report has no {label} array")
        if value:
            sys.exit(f"comparison report is green but has {label}")

    if summaries["absent"] != 0:
        sys.exit("comparison report has absent views")
    if summaries["fail"] != 0:
        sys.exit("comparison report is green but has failed views")

    coverage = _schema(report["coverage"], "coverage", COVERAGE_KEYS)
    coverage_counts = {
        field: _integer(coverage[field], f"coverage count for {field}")
        for field in COVERAGE_KEYS
    }
    if coverage_counts["absent"] != 0:
        sys.exit("comparison report has absent views")
    if summaries["unmeasured"] != coverage_counts["unmeasured"]:
        sys.exit("comparison report unmeasured count disagrees with coverage")

    records_raw = _array(report["records"], "records")
    if not records_raw:
        sys.exit("comparison report records array is empty")
    records = [_record(value, index) for index, value in enumerate(records_raw)]
    identifiers = [record["id"] for record in records]
    if len(set(identifiers)) != len(identifiers):
        sys.exit("comparison report has duplicate record identifiers")
    if identifiers != sorted(identifiers):
        sys.exit("comparison report records are not in serializer order")

    derived = collections.Counter(record["outcome"] for record in records)
    views = _integer(report["views"], "views count")
    if views != len(records):
        sys.exit("comparison report views count is not the record count")
    for outcome in ("pass", "fail", "unmeasured", "absent"):
        if summaries[outcome] != derived[outcome]:
            sys.exit(f"comparison report {outcome} count contradicts records")
    if summaries["pass"] + summaries["fail"] != claimed:
        sys.exit("comparison report claimed count is not pass plus fail")
    qualifiers = report["qualifiers"]
    if not isinstance(qualifiers, dict):
        sys.exit("comparison report qualifiers is not an object")
    if any(key not in QUALIFIERS for key in qualifiers):
        sys.exit("comparison report qualifiers has an unknown key")
    for key, value in qualifiers.items():
        if _integer(value, f"qualifier count for {key}") <= 0:
            sys.exit(f"comparison report has invalid qualifier count for {key}")
    derived_qualifiers = collections.Counter(
        qualifier for record in records for qualifier in record["qualifiers"]
    )
    if qualifiers != dict(derived_qualifiers):
        sys.exit("comparison report qualifier histogram contradicts records")

    hashes = _schema(report["renderHashes"], "renderHashes", RENDER_HASH_KEYS)
    if hashes["algorithm"] != HASH_ALGORITHM:
        sys.exit("comparison report has invalid aggregate render hash algorithm")
    for side in ("reference", "candidate"):
        aggregate = _hash(hashes[side], f"aggregate {side}")
        if aggregate != _run_hash(records, side):
            sys.exit(f"comparison report aggregate {side} render hash contradicts records")

    return {
        "reportSha256": hashlib.sha256(encoded).hexdigest(),
        "claimedVerdictViews": claimed,
        "verdict": "pass",
    }


def valid_comparison(entry: dict) -> bool:
    comparison = entry.get("comparison")
    if not isinstance(comparison, dict):
        return False
    digest = comparison.get("reportSha256")
    claimed = comparison.get("claimedVerdictViews")
    return (isinstance(digest, str)
            and re.fullmatch(r"[0-9a-f]{64}", digest) is not None
            and isinstance(claimed, int) and not isinstance(claimed, bool)
            and claimed > 0
            and comparison.get("verdict") == "pass")


def cmd_record(args: argparse.Namespace) -> int:
    if args.corpus not in CORPUS_STATES:
        sys.exit(f"--corpus must be one of {sorted(CORPUS_STATES)}")
    tree = args.tree or staged_tree()
    data = load()
    entry = {
        "gates": sorted(set(filter(None, args.gates.split(",")))),
        "corpus": args.corpus,
        "profile": args.profile,
        "agent": args.agent or os.environ.get("OCELLI_AGENT", "unknown"),
    }
    if args.comparison_report:
        entry["comparison"] = comparison_evidence(args.comparison_report)
    data[tree] = entry
    save(data)
    print(f"recorded tree {tree[:12]} corpus={args.corpus} "
          f"profile={args.profile}")
    return 0


def entry_for(tree: str) -> dict | None:
    return load().get(tree)


def cmd_assert(args: argparse.Namespace) -> int:
    tree = args.tree or staged_tree()
    entry = entry_for(tree)
    if entry is None:
        print(f"FAIL: no verification recorded for the staged tree "
              f"{tree[:12]}.")
        print("Run `/verify` (or `bin/ocelli.sh gate --all`) and commit the")
        print("same tree. A record for an ancestor commit does not count,")
        print("because it is a claim about different content.")
        return 1
    if entry["corpus"] == "fail":
        print(f"FAIL: the corpus is RED for tree {tree[:12]}.")
        return 1
    if args.require_corpus and entry["corpus"] != "pass":
        print(f"FAIL: corpus is '{entry['corpus']}' for tree {tree[:12]}, "
              f"and this gate requires 'pass'.")
        print("The corpus is the mechanism that makes generated Rust safe to")
        print("merge at volume (HLD decision D7). Acquire it, see")
        print("corpus/README.md.")
        return 1
    if args.require_comparison and not valid_comparison(entry):
        print(f"FAIL: comparison evidence is required for tree {tree[:12]}.")
        print("Run the explicit candidate gate and record its compare.json.")
        return 1
    print(f"OK: tree {tree[:12]} verified, corpus={entry['corpus']}")
    return 0


def cmd_trailer(args: argparse.Namespace) -> int:
    tree = args.tree or staged_tree()
    entry = entry_for(tree)
    if entry is None:
        return 1
    comparison = entry.get("comparison")
    suffix = ""
    if valid_comparison(entry):
        suffix = (f" comparison={comparison['reportSha256']}"
                  f" comparison-views={comparison['claimedVerdictViews']}"
                  f" comparison-verdict={comparison['verdict']}")
    print(f"{TRAILER_VERIFY}: profile={entry['profile']} "
          f"gates={','.join(entry['gates'])} corpus={entry['corpus']} "
          f"tree={tree[:12]}{suffix}")
    print(f"{TRAILER_AGENT}: {entry['agent']}")
    return 0


def cmd_check_commit(args: argparse.Namespace) -> int:
    """CI side. Needs no GPU, no corpus and no ledger."""
    message = git("log", "-1", "--format=%B", args.rev)
    tree = git("rev-parse", f"{args.rev}^{{tree}}")

    verify_line = next(
        (l for l in message.splitlines() if l.startswith(f"{TRAILER_VERIFY}:")),
        None)
    if verify_line is None:
        print(f"FAIL: {args.rev} carries no {TRAILER_VERIFY} trailer.")
        print("Every commit records how it was verified (HLD 27.2 R6), and")
        print("with no GPU in CI this trailer is the only evidence CI has")
        print("that the corpus ran at all (DEVIATIONS.md D-04).")
        return 1

    fields = dict(
        part.split("=", 1) for part in verify_line.split(":", 1)[1].split()
        if "=" in part)

    if fields.get("tree") != tree[:12]:
        print(f"FAIL: {args.rev} trailer names tree {fields.get('tree')} "
              f"but the commit's tree is {tree[:12]}.")
        print("The trailer was carried over from a different tree, so it is")
        print("evidence about content that is not in this commit.")
        return 1

    corpus = fields.get("corpus")
    if corpus == "fail":
        print(f"FAIL: {args.rev} records a RED corpus.")
        return 1
    if args.require_corpus and corpus != "pass":
        print(f"FAIL: {args.rev} records corpus={corpus}, 'pass' required.")
        return 1

    comparison_fields = {
        key: fields.get(key)
        for key in ("comparison", "comparison-views", "comparison-verdict")
    }
    has_comparison_field = any(value is not None
                               for value in comparison_fields.values())
    comparison_valid = (
        re.fullmatch(r"[0-9a-f]{64}", comparison_fields["comparison"] or "")
        is not None
        and re.fullmatch(r"[1-9][0-9]*",
                         comparison_fields["comparison-views"] or "")
        is not None
        and comparison_fields["comparison-verdict"] == "pass"
    )
    if has_comparison_field and not comparison_valid:
        print(f"FAIL: {args.rev} carries malformed comparison evidence.")
        return 1
    if args.require_comparison and not comparison_valid:
        print(f"FAIL: {args.rev} comparison evidence is required.")
        return 1

    print(f"OK: {args.rev} verified, corpus={corpus}, tree matches")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("record")
    p.add_argument("--tree")
    p.add_argument("--gates", default="")
    p.add_argument("--corpus", default="absent")
    p.add_argument("--profile", default="feature")
    p.add_argument("--agent", default="")
    p.add_argument("--comparison-report", default="")
    p.set_defaults(func=cmd_record)

    p = sub.add_parser("assert")
    p.add_argument("--tree")
    p.add_argument("--require-corpus", action="store_true")
    p.add_argument("--require-comparison", action="store_true")
    p.set_defaults(func=cmd_assert)

    p = sub.add_parser("trailer")
    p.add_argument("--tree")
    p.set_defaults(func=cmd_trailer)

    p = sub.add_parser("check-commit")
    p.add_argument("rev", nargs="?", default="HEAD")
    p.add_argument("--require-corpus", action="store_true")
    p.add_argument("--require-comparison", action="store_true")
    p.set_defaults(func=cmd_check_commit)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
