#!/usr/bin/env python3
"""Tests for scripts/error_code_check.py.

    python3 -m unittest discover -s scripts/tests -p 'test_error_code_check.py'

Needs nothing but the standard library. Every case builds its three inputs in
memory and calls `check` directly, so nothing here reads the real registry, the
real Rust module or the real TypeScript module.

**These are the negative cases.** A guard is only worth having if it goes red,
and F-X009's whole subject is that a guard with no standing test is watched by
nothing. The positive case is the fourth test at the bottom, and it exists so
that a `check` which returned a problem for everything would not pass this file.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import error_code_check  # noqa: E402


REGISTRY = {
    "ranges": [
        {"crate": "ocelli-core", "first": 1, "last": 99, "covers": "x"},
        {"crate": "ocelli-compute", "first": 700, "last": 799, "covers": "y"},
    ],
    "codes": [
        {"name": "Panicked", "number": 1, "crate": "ocelli-core",
         "producer": "p", "summary": "s"},
        {"name": "Unavailable", "number": 700, "crate": "ocelli-compute",
         "producer": "p", "summary": "s"},
    ],
}

RUST = """
#[repr(u16)]
pub enum ErrorCode {
    Panicked = 1,
    Unavailable = 700,
}
impl ErrorCode {
    pub const fn number(self) -> u16 {
        match self {
            Self::Panicked => 1,
            Self::Unavailable => 700,
        }
    }
}
"""

TYPESCRIPT = """
export const ERROR_CODE = {
  Panicked: 1,
  Unavailable: 700,
} as const;

const MESSAGES: Readonly<Record<number, string>> = {
  1: "the core stopped",
  700: "unavailable on this tier",
};
"""


def registry(**changes: object) -> dict:
    """A deep-enough copy of REGISTRY with `codes` or `ranges` replaced."""
    return {
        "ranges": changes.get("ranges", [dict(r) for r in REGISTRY["ranges"]]),
        "codes": changes.get("codes", [dict(c) for c in REGISTRY["codes"]]),
    }


class ErrorCodeCheck(unittest.TestCase):

    def problems(self, reg: dict, rust: str = RUST,
                 typescript: str = TYPESCRIPT) -> list[str]:
        return error_code_check.check(reg, rust, typescript)

    # 1. A renumbering, which HLD section 23 forbids and which nothing else
    #    prevents. This is the defect the registry exists for.
    def test_a_renumbered_code_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes[0]["number"] = 2
        problems = self.problems(registry(codes=codes))
        self.assertTrue(problems)
        self.assertTrue(any("Panicked" in p for p in problems), problems)

    # 2. The worst version of a renumbering: an old shell decodes a new code
    #    as something plausible.
    def test_a_reused_number_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes.append({"name": "Workgroup", "number": 700,
                      "crate": "ocelli-compute", "producer": "p",
                      "summary": "s"})
        problems = self.problems(registry(codes=codes))
        self.assertTrue(any("700" in p and "reuse" in p.lower()
                            for p in problems), problems)

    def test_a_reused_name_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes.append({"name": "Panicked", "number": 42,
                      "crate": "ocelli-core", "producer": "p",
                      "summary": "s"})
        problems = self.problems(registry(codes=codes))
        self.assertTrue(any("Panicked" in p and "reuse" in p.lower()
                            for p in problems), problems)

    # 3. A Rust variant the registry does not carry, and the mirror image.
    def test_a_rust_variant_missing_from_the_registry_is_refused(self):
        rust = RUST.replace("    Unavailable = 700,\n",
                            "    Unavailable = 700,\n    Workgroup = 701,\n")
        problems = self.problems(registry(), rust=rust)
        self.assertTrue(any("Workgroup" in p for p in problems), problems)

    def test_a_registry_entry_with_no_rust_variant_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes.append({"name": "Workgroup", "number": 701,
                      "crate": "ocelli-compute", "producer": "p",
                      "summary": "s"})
        problems = self.problems(registry(codes=codes))
        self.assertTrue(any("Workgroup" in p for p in problems), problems)

    # 4. A code the shell cannot describe reaches a user as a number.
    def test_a_registry_entry_with_no_typescript_entry_is_refused(self):
        typescript = TYPESCRIPT.replace("  Unavailable: 700,\n", "")
        problems = self.problems(registry(), typescript=typescript)
        self.assertTrue(any("Unavailable" in p for p in problems), problems)

    def test_a_registry_entry_with_no_typescript_message_is_refused(self):
        typescript = TYPESCRIPT.replace(
            '  700: "unavailable on this tier",\n', "")
        problems = self.problems(registry(), typescript=typescript)
        self.assertTrue(any("700" in p for p in problems), problems)

    def test_a_typescript_entry_with_no_registry_entry_is_refused(self):
        typescript = TYPESCRIPT.replace(
            "  Unavailable: 700,\n", "  Unavailable: 700,\n  Invented: 900,\n")
        problems = self.problems(registry(), typescript=typescript)
        self.assertTrue(any("Invented" in p for p in problems), problems)

    def test_a_typescript_number_that_disagrees_with_the_registry_is_refused(self):
        typescript = TYPESCRIPT.replace("  Unavailable: 700,",
                                        "  Unavailable: 701,")
        problems = self.problems(registry(), typescript=typescript)
        self.assertTrue(any("Unavailable" in p for p in problems), problems)

    # 5. A second crate quietly picking an overlapping block.
    def test_a_code_outside_its_declared_range_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes[1]["crate"] = "ocelli-core"
        problems = self.problems(registry(codes=codes))
        self.assertTrue(any("range" in p.lower() for p in problems), problems)

    def test_a_code_naming_no_declared_range_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes[0]["crate"] = "ocelli-invented"
        problems = self.problems(registry(codes=codes))
        self.assertTrue(any("ocelli-invented" in p for p in problems),
                        problems)

    def test_overlapping_declared_ranges_are_refused(self):
        ranges = [dict(r) for r in REGISTRY["ranges"]]
        ranges[1]["first"] = 50
        problems = self.problems(registry(ranges=ranges))
        self.assertTrue(any("overlap" in p.lower() for p in problems),
                        problems)

    # 6. Zero is never a valid code, which is what makes a zeroed payload
    #    undecodable. crates/ocelli-core/src/error.rs asserts the same rule.
    def test_code_zero_is_refused(self):
        codes = [dict(c) for c in REGISTRY["codes"]]
        codes[0]["number"] = 0
        problems = self.problems(registry(codes=codes))
        # "code 0" and not "0", which also matches 700 and would make this
        # assertion pass on any problem at all.
        self.assertTrue(any("code 0" in p for p in problems), problems)

    # 7. The positive case. Without it a `check` that returned a problem for
    #    every input would satisfy every test above.
    def test_the_agreeing_set_is_accepted(self):
        self.assertEqual(self.problems(registry()), [])

    # 8. And the real files agree, which is what the gate actually runs.
    def test_the_repository_agrees_with_itself(self):
        self.assertEqual(error_code_check.check(
            error_code_check.load_registry(),
            error_code_check.REGISTRY_RUST.read_text(encoding="utf-8"),
            error_code_check.REGISTRY_TS.read_text(encoding="utf-8"),
        ), [])


if __name__ == "__main__":
    unittest.main()
