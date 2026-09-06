#!/usr/bin/env python3
"""Focused tests for the integration handoff field grammar. F-X014."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

from sprint_workflow import handoff_field  # noqa: E402


class HandoffFieldGrammar(unittest.TestCase):
    def test_plain_text_and_one_complete_code_span_are_accepted(self) -> None:
        self.assertEqual(handoff_field("**Head**: 012345\n", "Head"),
                         "012345")
        self.assertEqual(handoff_field("**Head**: `012345`\n", "Head"),
                         "012345")

    def test_absent_field_is_distinct_from_a_malformed_value(self) -> None:
        self.assertIsNone(handoff_field("**Base**: main\n", "Head"))

    def test_malformed_spans_are_refused(self) -> None:
        malformed = (
            "",
            "``",
            "`012345",
            "012345`",
            "01`23`45",
            "`012345` `forged`",
            "`012345`forged",
            "forged`012345`",
        )
        for value in malformed:
            with self.subTest(value=value):
                with self.assertRaisesRegex(ValueError,
                                            "exactly one Markdown code span"):
                    handoff_field(f"**Head**: {value}\n", "Head")

    def test_duplicate_required_field_is_refused(self) -> None:
        with self.assertRaisesRegex(ValueError,
                                    r"duplicate \*\*Head\*\* fields"):
            handoff_field("**Head**: first\n**Head**: second\n", "Head")


if __name__ == "__main__":
    unittest.main()
