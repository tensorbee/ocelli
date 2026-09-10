#!/usr/bin/env python3
"""Tests for scripts/bench_check.py.

    python3 -m unittest discover -s scripts/tests -p 'test_bench_check.py'

Needs nothing but the standard library. Every case builds its inputs in memory
and calls `check` directly, so nothing here reads the real registry, the real
allocation or the real baseline.

**These are the negative cases.** A guard is only worth having if it goes red,
and F-X009's whole subject is that a guard with no standing test is watched by
nothing. The positive case is the last test in the file, and it exists so that a
`check` which returned a problem for everything would not pass.

Written in the shape used in this repository today, deliberately.
`docs/sprints/allocation.json` puts F-X009 after F-006, and F-X009 converts this
file into its standing-guard shape rather than F-006 guessing at a shape that
does not exist yet.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import bench_check  # noqa: E402


HOST_CLASS = {
    "platform": "darwin",
    "release": "25.6.0",
    "arch": "arm64",
    "cpu_model": "Apple M5 Max",
    "cpu_count": 18,
    "memory_bytes": 137438953472,
}

LANDED = {
    "id": "wasm.cold_start",
    "title": "a subject that exists",
    "definition_hld": "A-spike-gates.md gate A4",
    "definition": "starts here, stops there",
    "unit": "ms",
    "dimensions": ["browser"],
    "tiers": ["n/a"],
    "subject_story": None,
    "feeds": ["A4"],
}

PENDING = {
    "id": "decode.frame",
    "title": "a subject that does not exist yet",
    "definition_hld": "18-codec-registry.md section 21",
    "definition": "one call to Decoder::decode",
    "unit": "ms",
    "dimensions": ["transfer_syntax"],
    "tiers": ["n/a"],
    "subject_story": "F-023",
    "feeds": ["A1"],
}

ALLOCATION = {"stories": [{"fid": "F-001"}, {"fid": "F-023"}]}

BACKLOG = """# Backlog

## Recorded defects in the imported backlog

| F-ID | Epic ref | Defect |
| F-145 | E35.3 | declared dependency E4.9 does not exist |

### M1, Foundations and the differential oracle

| F-ID | Epic ref | Sprint | Story | Layer | Est | Depends on | Status |
|------|----------|--------|-------|-------|-----|------------|--------|
| F-001 | E1.1 | S01 | a landed thing | Build | 2w | - | done |
| F-023 | E4.1 | S07 | a pending thing | Rust | 2w | F-016 | pending |
"""

BENCH_PKG = {"devDependencies": {"playwright": "1.62.1"}}
ORACLE_PKG = {"devDependencies": {"esbuild": "0.28.2",
                                  "playwright": "1.62.1"}}


def baseline(subjects: dict | None = None) -> dict:
    return {
        "host_classes": {
            "darwin|25.6.0|arm64|Apple_M5_Max|18|137438953472": {
                "host_class": dict(HOST_CLASS),
                "subjects": subjects if subjects is not None else {},
            },
        },
    }


def entry(**changes) -> dict:
    recorded = {
        "value": 2.4,
        "unit": "ms",
        "tolerance": 0.25,
        "tolerance_provenance": "derived from the observed spread",
        "provenance": "measured",
        "recorded": "2026-09-05",
        "story": "F-006",
        "why": "the first recorded cold start",
        "host_class": dict(HOST_CLASS),
        "instrument": {"node": "v24.16.0", "playwright": "1.62.1"},
    }
    recorded.update(changes)
    return recorded


def backlog_with_f023_status(status: str) -> str:
    old = "| F-023 | E4.1 | S07 | a pending thing | Rust | 2w | F-016 | pending |"
    new = f"| F-023 | E4.1 | S07 | a pending thing | Rust | 2w | F-016 | {status} |"
    changed = BACKLOG.replace(old, new)
    if changed == BACKLOG:
        raise AssertionError("the F-023 backlog fixture row changed shape")
    return changed


class BenchCheck(unittest.TestCase):

    def problems(self, subjects=None, runners=None, base=None,
                 bench_pkg=None, oracle_pkg=None, allocation=None,
                 backlog=BACKLOG):
        return bench_check.check(
            {"subjects": subjects if subjects is not None
             else [dict(LANDED), dict(PENDING)]},
            allocation if allocation is not None else ALLOCATION,
            backlog,
            runners if runners is not None else {"wasm_cold_start"},
            base if base is not None else baseline(),
            bench_pkg if bench_pkg is not None else BENCH_PKG,
            oracle_pkg if oracle_pkg is not None else ORACLE_PKG,
        )

    # --- the anti-fabrication rule -----------------------------------------

    def test_a_runner_for_a_pending_subject_is_refused(self):
        problems = self.problems(
            runners={"wasm_cold_start", "decode_frame"})
        self.assertTrue(
            any("decode.frame has a runner" in p for p in problems), problems)
        self.assertTrue(any("can only be timing a stub" in p
                            for p in problems), problems)

    def test_a_baseline_entry_for_a_pending_subject_is_refused(self):
        problems = self.problems(
            base=baseline({"decode.frame": entry()}))
        self.assertTrue(
            any("records a number for decode.frame" in p for p in problems),
            problems)

    def test_a_landed_subject_with_no_runner_is_refused(self):
        problems = self.problems(runners=set())
        self.assertTrue(
            any("has a subject today and no runner" in p for p in problems),
            problems)

    def test_a_runner_with_no_registry_row_is_refused(self):
        problems = self.problems(
            runners={"wasm_cold_start", "render_first_frame"})
        self.assertTrue(
            any("render_first_frame.mjs matches no row" in p
                for p in problems), problems)

    def test_an_in_progress_story_may_have_a_runner_and_baseline_entry(self):
        backlog = backlog_with_f023_status("in-progress")
        problems = self.problems(
            runners={"wasm_cold_start", "decode_frame"},
            base=baseline({"decode.frame": entry()}),
            backlog=backlog)
        self.assertEqual(problems, [])

    def test_a_done_story_may_have_a_runner_and_a_baseline_entry(self):
        problems = self.problems(
            runners={"wasm_cold_start", "decode_frame"},
            base=baseline({"decode.frame": entry()}),
            backlog=backlog_with_f023_status("done"))
        self.assertEqual(problems, [])

    def test_a_done_story_with_no_runner_is_refused(self):
        problems = self.problems(
            backlog=backlog_with_f023_status("done"))
        self.assertTrue(
            any("decode.frame is done and has no runner" in p
                for p in problems), problems)

    def test_python_policy_matches_the_runtime_status_table(self):
        expected = {
            "pending": False,
            "in-progress": True,
            "done": True,
            "archived": False,
            "superseded": False,
        }
        self.assertEqual(bench_check.MEASURABLE_STATUS, expected)

    # --- the registry ------------------------------------------------------

    def test_an_unknown_f_id_is_refused(self):
        subject = dict(PENDING)
        subject["subject_story"] = "F-999"
        problems = self.problems(subjects=[dict(LANDED), subject])
        self.assertTrue(any("not an F-ID in" in p for p in problems), problems)

    def test_an_f_id_with_no_backlog_row_is_refused(self):
        allocation = {"stories": [{"fid": "F-001"}, {"fid": "F-023"},
                                  {"fid": "F-500"}]}
        subject = dict(PENDING)
        subject["subject_story"] = "F-500"
        problems = self.problems(subjects=[dict(LANDED), subject],
                                 allocation=allocation)
        self.assertTrue(any("no status row" in p for p in problems), problems)

    def test_every_field_is_required(self):
        # `id` is refused by its own message, because a row with no id cannot be
        # named in the message that would report the others.
        for field in bench_check.REQUIRED_FIELDS:
            subject = dict(LANDED)
            del subject[field]
            wanted = ("a subject has a missing or non-string id"
                      if field == "id" else f"missing the {field} field")
            with self.subTest(field=field):
                problems = self.problems(subjects=[subject, dict(PENDING)])
                self.assertTrue(any(wanted in p for p in problems), problems)

    def test_a_row_with_no_tier_is_refused_and_n_a_is_accepted(self):
        subject = dict(LANDED)
        subject["tiers"] = []
        self.assertTrue(any("declares no tiers" in p for p in
                            self.problems(subjects=[subject, dict(PENDING)])))
        subject["tiers"] = ["D"]
        self.assertTrue(any("not one of" in p for p in
                            self.problems(subjects=[subject, dict(PENDING)])))

    def test_a_duplicate_id_is_refused(self):
        problems = self.problems(
            subjects=[dict(LANDED), dict(LANDED), dict(PENDING)])
        self.assertTrue(any("appears twice" in p for p in problems), problems)

    def test_a_recorded_defect_row_is_not_read_as_a_status(self):
        statuses = bench_check.backlog_statuses(BACKLOG)
        self.assertNotIn("F-145", statuses)
        self.assertEqual(statuses["F-001"], "done")
        self.assertEqual(statuses["F-023"], "pending")

    # --- the baseline ------------------------------------------------------

    def test_a_baseline_entry_needs_a_host_class(self):
        base = baseline({"wasm.cold_start": entry()})
        block = next(iter(base["host_classes"].values()))
        del block["host_class"]["cpu_model"]
        problems = self.problems(base=base)
        self.assertTrue(any("records no cpu_model" in p for p in problems),
                        problems)

    def test_a_baseline_entry_needs_a_tolerance_in_range(self):
        for bad in (None, 0, 1.5, -0.1, "0.25", True):
            with self.subTest(tolerance=bad):
                problems = self.problems(
                    base=baseline({"wasm.cold_start": entry(tolerance=bad)}))
                self.assertTrue(
                    any("has tolerance" in p for p in problems), problems)

    def test_a_tolerance_with_no_provenance_is_refused(self):
        problems = self.problems(base=baseline(
            {"wasm.cold_start": entry(tolerance_provenance="")}))
        self.assertTrue(
            any("nothing saying where it came from" in p for p in problems),
            problems)

    def test_a_null_host_class_or_instrument_is_refused(self):
        # Presence is not enough. A null passes a `not in` test, names no
        # machine and no tools, and leaves an entry that can never be compared
        # against anything while the gate calls it fine.
        for field in ("host_class", "instrument"):
            with self.subTest(field=field):
                problems = self.problems(
                    base=baseline({"wasm.cold_start": entry(**{field: None})}))
                self.assertTrue(
                    any(f"records {field} as None rather than a block" in p
                        for p in problems), problems)

    def test_a_non_list_dimension_or_feed_is_refused(self):
        # The same rule tools/bench/src/registry.mjs applies. A rule on one
        # side only lets a registry the driver refuses land in the tree.
        for field in ("dimensions", "feeds"):
            subject = dict(LANDED)
            subject[field] = "browser"
            with self.subTest(field=field):
                problems = self.problems(subjects=[subject, dict(PENDING)])
                self.assertTrue(
                    any(f"has a non-list {field}" in p for p in problems),
                    problems)

    def test_a_baseline_entry_for_an_unknown_subject_is_refused(self):
        problems = self.problems(
            base=baseline({"not.a.subject": entry()}))
        self.assertTrue(
            any("no such row" in p for p in problems), problems)

    def test_a_derived_or_estimated_figure_is_refused(self):
        problems = self.problems(base=baseline(
            {"wasm.cold_start": entry(provenance="estimated")}))
        self.assertTrue(any("has provenance" in p for p in problems), problems)

    def test_an_entry_filed_under_the_wrong_machine_is_refused(self):
        other = dict(HOST_CLASS)
        other["cpu_model"] = "Apple M4 Pro"
        problems = self.problems(
            base=baseline({"wasm.cold_start": entry(host_class=other)}))
        self.assertTrue(
            any("differs from its block" in p for p in problems), problems)

    def test_a_scalar_baseline_is_refused(self):
        problems = self.problems(base={"wasm.cold_start": 2.4})
        self.assertTrue(
            any("no `host_classes` map" in p for p in problems), problems)

    # --- the pins ----------------------------------------------------------

    def test_a_drifted_playwright_pin_is_refused(self):
        problems = self.problems(
            bench_pkg={"devDependencies": {"playwright": "1.63.0"}})
        self.assertTrue(
            any("tools/bench pins playwright 1.63.0" in p for p in problems),
            problems)

    def test_a_ranged_playwright_pin_is_refused(self):
        problems = self.problems(
            bench_pkg={"devDependencies": {"playwright": "^1.62.1"}},
            oracle_pkg={"devDependencies": {"playwright": "^1.62.1"}})
        self.assertTrue(
            any("no caret and no tilde" in p for p in problems), problems)

    def test_an_absent_playwright_pin_is_refused(self):
        problems = self.problems(bench_pkg={"devDependencies": {}})
        self.assertTrue(
            any("pins no playwright" in p for p in problems), problems)

    # --- the positive case -------------------------------------------------

    def test_the_shipped_shape_passes(self):
        problems = self.problems(
            base=baseline({"wasm.cold_start": entry()}))
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
