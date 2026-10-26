#!/usr/bin/env python3
"""Handoff grammar and verification-attempt invalidation. F-X014, F-X023."""

from __future__ import annotations

import sys
import argparse
import io
import json
import subprocess
import tempfile
from contextlib import redirect_stdout
from unittest.mock import patch
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import backlog_check  # noqa: E402
import sprint_workflow  # noqa: E402
from sprint_workflow import carried_forward_reasons, handoff_field  # noqa: E402


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


class CarryForwardGrammar(unittest.TestCase):
    def test_reads_only_the_named_sprint_section(self) -> None:
        text = (
            "## Carried forward from S03\n\n"
            "- **F-001** old reason\n\n"
            "## Carried forward from S04\n\n"
            "- **F-012** candidate renderer does not exist\n"
            "- **F-X011** second physical machine is unavailable\n\n"
            "## Next section\n\n"
            "- **F-999** not a carry-forward row\n"
        )
        self.assertEqual(
            carried_forward_reasons(text, "S04"),
            {
                "F-012": "candidate renderer does not exist",
                "F-X011": "second physical machine is unavailable",
            },
        )

    def test_empty_or_missing_reason_is_not_a_record(self) -> None:
        text = "## Carried forward from S04\n\n- **F-012**\n"
        self.assertEqual(carried_forward_reasons(text, "S04"), {})


class VerificationAttempt(unittest.TestCase):
    def test_new_attempt_removes_only_its_prior_receipt(self):
        import verify_ledger
        with tempfile.TemporaryDirectory() as directory:
            ledger = Path(directory) / "ledger.json"
            ledger.write_text('{"old-tree": {"corpus": "pass"}, '
                              '"other-tree": {"corpus": "pass"}}')
            args = argparse.Namespace(tree="old-tree", require_corpus=False,
                                      require_comparison=False)
            with patch.object(verify_ledger, "LEDGER", ledger), \
                 redirect_stdout(io.StringIO()) as out:
                self.assertEqual(verify_ledger.cmd_assert(args), 0)
                self.assertEqual(verify_ledger.cmd_invalidate(args), 0)
                self.assertIn("invalidated verification for tree old-tree", out.getvalue())
                self.assertEqual(verify_ledger.cmd_assert(args), 1)
                self.assertEqual(verify_ledger.entry_for("other-tree"), {"corpus": "pass"})
                out.truncate(0)
                out.seek(0)
                self.assertEqual(verify_ledger.cmd_invalidate(args), 0)
                self.assertIn("no verification receipt for tree old-tree", out.getvalue())
                self.assertIn("nothing to invalidate", out.getvalue())


TREE = "a" * 40


def _feature(state: str = "completed") -> dict:
    return {
        "state": state,
        "reviews": [{"pass": 1, "defects": 0, "smells": 0, "nitpicks": 0}],
    }


class CloseInventory(unittest.TestCase):
    def close(
        self,
        features: dict,
        *,
        allocated: tuple[str, ...] = ("F-045", "F-046"),
        backlog_fids: tuple[str, ...] | None = None,
        statuses: dict[str, str] | None = None,
        carry: str = "",
        tree: str = TREE,
        head_tree: str | None = None,
        dirty: str = "",
        verifications: list[dict] | None = None,
        sprint_reviews: list[dict] | None = None,
    ) -> tuple[int, str]:
        statuses = statuses or {fid: "done" for fid in allocated}
        data = {
            "sprint": "S99",
            "features": features,
            "sprint_reviews": sprint_reviews if sprint_reviews is not None else [{
                "pass": 1,
                "defects": 0,
                "smells": 0,
                "nitpicks": 0,
                "tree": tree,
            }],
            "verifications": verifications if verifications is not None else [{
                "profile": "sprint",
                "result": "pass",
                "gates": "test",
                "corpus": "pass",
                "tree": tree,
            }],
        }
        head = tree if head_tree is None else head_tree

        def fake_run(args, cwd=None, capture_output=None, text=None, check=False):
            completed = subprocess.CompletedProcess(args, 0, stdout="", stderr="")
            if args[:2] == ["git", "status"]:
                completed.stdout = dirty
            elif args[:2] == ["git", "rev-parse"]:
                completed.stdout = head + "\n"
            elif args[:2] == ["git", "write-tree"]:
                completed.stdout = tree + "\n"
            elif args[:3] == ["git", "worktree", "list"]:
                completed.stdout = f"worktree {sprint_workflow.ROOT}\n"
            else:
                raise AssertionError(args)
            return completed

        with tempfile.TemporaryDirectory() as directory:
            sprints = Path(directory) / "sprints"
            handoffs = Path(directory) / "handoffs"
            sprints.mkdir()
            handoffs.mkdir()
            listed = allocated if backlog_fids is None else backlog_fids
            rows = [
                f"| {fid} | E0.0 | S99 | Story | Rust | 1w | - | {statuses.get(fid, 'pending')} |"
                for fid in listed
            ]
            (sprints / "BACKLOG.md").write_text(
                "### M9, fixture\n\n"
                "| F-ID | Epic | Sprint | Story | Layer | Est | Depends | Status |\n"
                "|------|------|--------|-------|-------|-----|---------|--------|\n"
                + "\n".join(rows) + "\n"
            )
            (sprints / "allocation.json").write_text(json.dumps({
                "stories": [{"fid": fid, "sprint": "S99"} for fid in allocated],
            }))
            (sprints / "CURRENT_SPRINT.md").write_text(
                "# Current sprint, S99\n\n" + carry
            )
            buffer = io.StringIO()
            with patch.object(sprint_workflow, "SPRINTS", sprints), \
                 patch.object(sprint_workflow, "ALLOCATION", sprints / "allocation.json"), \
                 patch.object(sprint_workflow, "HANDOFFS", handoffs), \
                 patch.object(backlog_check, "SPRINTS", sprints), \
                 patch.object(sprint_workflow.subprocess, "run", fake_run), \
                 patch.object(sprint_workflow, "load", lambda sprint=None: data), \
                 redirect_stdout(buffer):
                code = sprint_workflow.cmd_close_preflight(
                    argparse.Namespace(sprint="S99"))
            return code, buffer.getvalue()

    def test_exact_inventory_and_done_backlog_pass(self) -> None:
        code, output = self.close({
            "F-045": _feature(),
            "F-046": _feature(),
        })
        self.assertEqual(code, 0, output)
        self.assertIn("OK: S99 is ready to close", output)

    def test_empty_missing_and_extra_inventories_fail(self) -> None:
        cases = {
            "empty": ({}, {}),
            "missing": ({"F-045": _feature()}, {}),
        }
        for name, (features, kwargs) in cases.items():
            with self.subTest(name=name):
                code, output = self.close(features, **kwargs)
                self.assertEqual(code, 1, output)
                self.assertIn(
                    "feature inventory differs from allocation: missing=",
                    output,
                )
        code, output = self.close(
            {
                "F-045": _feature(),
                "F-046": _feature(),
                "F-099": _feature(),
            },
            backlog_fids=("F-045", "F-046", "F-099"),
            statuses={"F-045": "done", "F-046": "done", "F-099": "done"},
        )
        self.assertEqual(code, 1, output)
        self.assertIn("feature inventory differs from allocation: missing=", output)
        self.assertIn("extra=", output)

    def test_completed_pending_backlog_fails(self) -> None:
        code, output = self.close(
            {"F-045": _feature(), "F-046": _feature()},
            statuses={"F-045": "pending", "F-046": "done"},
        )
        self.assertEqual(code, 1, output)
        self.assertIn(
            "F-045 is completed in run state but backlog status is pending",
            output,
        )

    def test_missing_backlog_row_fails(self) -> None:
        code, output = self.close(
            {"F-045": _feature()},
            allocated=("F-045",),
            backlog_fids=(),
        )
        self.assertEqual(code, 1, output)
        self.assertIn(
            "F-045 is completed in run state but backlog status is missing",
            output,
        )

    def test_empty_allocation_fails(self) -> None:
        code, output = self.close({}, allocated=())
        self.assertEqual(code, 1, output)
        self.assertIn("feature inventory differs from allocation: missing=", output)

    def test_explained_carry_passes_without_a_done_backlog(self) -> None:
        carry = (
            "## Carried forward from S99\n\n"
            "- **F-045** second machine is unavailable\n"
            "- **F-046** candidate renderer does not exist\n"
        )
        code, output = self.close(
            {"F-045": _feature("carried"), "F-046": _feature("carried")},
            statuses={"F-045": "pending", "F-046": "pending"},
            carry=carry,
        )
        self.assertEqual(code, 0, output)
        self.assertIn("OK: S99 is ready to close", output)

    def test_unrecorded_carry_keeps_its_reason_refusal(self) -> None:
        code, output = self.close(
            {"F-045": _feature("carried"), "F-046": _feature()},
        )
        self.assertEqual(code, 1, output)
        self.assertIn("is carried but has no recorded carry-forward reason", output)

    def test_recorded_carry_with_a_completed_neighbour_passes(self) -> None:
        carry = (
            "## Carried forward from S99\n\n"
            "- **F-045** second machine is unavailable\n"
        )
        code, output = self.close(
            {"F-045": _feature("carried"), "F-046": _feature()},
            carry=carry,
        )
        self.assertEqual(code, 0, output)

    def test_stale_failed_and_dirty_conditions_stay_refused(self) -> None:
        features = {"F-045": _feature(), "F-046": _feature()}
        stale_code, stale_output = self.close(
            features,
            verifications=[{
                "profile": "sprint",
                "result": "pass",
                "tree": "0" * 40,
            }],
        )
        self.assertEqual(stale_code, 1, stale_output)
        self.assertIn("is stale for current tree", stale_output)

        failed_code, failed_output = self.close(
            features,
            verifications=[
                {"profile": "sprint", "result": "pass", "tree": TREE},
                {"profile": "sprint", "result": "fail", "tree": TREE},
            ],
        )
        self.assertEqual(failed_code, 1, failed_output)
        self.assertIn(
            "latest sprint-profile verification did not pass",
            failed_output,
        )

        dirty_code, dirty_output = self.close(features, dirty=" M docs/sprints/BACKLOG.md\n")
        self.assertEqual(dirty_code, 1, dirty_output)
        self.assertIn("working tree is not clean", dirty_output)


if __name__ == "__main__":
    unittest.main()
