#!/usr/bin/env python3
"""Tests for scripts/gen_sprint_plan.py.

Covers the protected write mode, the milestone summary lines, and the
sprint-table and goal-line refusals `--check` prints.
"""

from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import gen_sprint_plan  # noqa: E402


class ProtectedWriteMode(unittest.TestCase):
    def paths(self, directory: str) -> tuple[Path, Path]:
        allocation = Path(directory) / "allocation.json"
        plan = Path(directory) / "SPRINT_PLAN.md"
        allocation.write_text('{"stories": []}\n', encoding="utf-8")
        return allocation, plan

    def invoke(self, argv: list[str], allocation: Path,
               plan: Path) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = gen_sprint_plan.main(
                argv, allocation_path=allocation, plan_path=plan)
        return status, output.getvalue()

    def test_bare_write_refuses_an_existing_plan_without_changing_it(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            original = b"# Hand-curated sprint plan\n\nKeep this paragraph.\n"
            plan.write_bytes(original)

            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, output = self.invoke([], allocation, plan)

            self.assertEqual(status, 1)
            self.assertEqual(plan.read_bytes(), original)
            self.assertIn("refuses to overwrite", output)
            self.assertIn("--force", output)
            self.assertIn("--check", output)

    def test_force_is_the_explicit_replacement_path(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            plan.write_text("hand curated\n", encoding="utf-8")
            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, _ = self.invoke(["--force"], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_text(encoding="utf-8"), "generated\n")

    def test_bare_write_still_bootstraps_an_absent_plan(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            with mock.patch.object(gen_sprint_plan, "render",
                                   return_value="generated\n"):
                status, _ = self.invoke([], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_text(encoding="utf-8"), "generated\n")

    def test_check_reads_explicit_paths_without_rewriting_the_plan(self):
        with tempfile.TemporaryDirectory() as directory:
            allocation, plan = self.paths(directory)
            allocation.write_bytes(
                (ROOT / "docs/sprints/allocation.json").read_bytes())
            original = (ROOT / "docs/sprints/SPRINT_PLAN.md").read_bytes()
            plan.write_bytes(original)

            status, _ = self.invoke(["--check"], allocation, plan)

            self.assertEqual(status, 0)
            self.assertEqual(plan.read_bytes(), original)


class PlanFixture:
    """The hand-written two-story plan the checker tests share.

    `write_plan_check`, `shared_sprint` and `shared_plan` used to live on
    `MilestoneSummary`. A second test class that inherited that case would
    run its tests twice, so the helpers sit on a mixin instead. `story` sits
    with them because `shared_sprint` calls it.
    """

    def story(self, fid: str, sprint: str, milestone: str,
              weeks: int) -> dict:
        return {"fid": fid, "sprint": sprint, "milestone": milestone,
                "weeks": weeks}

    def write_plan_check(self, data: dict, plan_text: str) -> tuple[int, str]:
        with tempfile.TemporaryDirectory() as directory:
            allocation = Path(directory) / "allocation.json"
            plan = Path(directory) / "SPRINT_PLAN.md"
            allocation.write_text(json.dumps(data), encoding="utf-8")
            plan.write_text(plan_text, encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = gen_sprint_plan.main(
                    ["--check"], allocation_path=allocation, plan_path=plan)
            return status, output.getvalue()

    def shared_sprint(self) -> dict:
        # Both milestones' only stories sit in S1, so they share one span.
        return {
            "milestones": [{"key": "M1", "title": "One", "goal": "g"},
                           {"key": "M2", "title": "Two", "goal": "g"}],
            "stories": [
                {**self.story("F-001", "S1", "M1", 3), "eid": "E1.1",
                 "story": "First", "layer": "Rust"},
                {**self.story("F-002", "S1", "M2", 3), "eid": "E2.1",
                 "story": "Second", "layer": "Rust"},
            ]}

    def shared_plan(self, second_summary: bool) -> str:
        lines = ["### M1, One", "", "g", "",
                 "_S1 to S1, 1 stories, 3 engineer-weeks._", "",
                 "### M2, Two", "", "g", ""]
        if second_summary:
            lines += ["_S1 to S1, 1 stories, 3 engineer-weeks._", ""]
        lines += ["#### Sprint S1", "", "**Goal**: First, Second.", "",
                  "| F-ID | Epic ref | Story | Layer | Est |",
                  "|------|----------|-------|-------|-----|",
                  "| F-001 | E1.1 | First | Rust | 3w |",
                  "| F-002 | E2.1 | Second | Rust | 3w |", ""]
        return "\n".join(lines)


class MilestoneSummary(PlanFixture, unittest.TestCase):
    def test_a_shared_sprint_counts_each_story_under_its_own_milestone(self):
        # S2 holds one M2 story first and one M1 story. Counting the whole of
        # S2 under its first story's milestone gave M1 one story over S1 and
        # M2 three stories and 14 weeks, which is the drift the S46 review
        # found.
        data = {"stories": [
            self.story("F-001", "S1", "M1", 3),
            self.story("F-002", "S2", "M2", 5),
            self.story("F-003", "S2", "M1", 2),
            self.story("F-004", "S3", "M2", 7),
        ]}
        groups = gen_sprint_plan.sprint_groups(data)

        self.assertEqual(gen_sprint_plan.milestone_summary(groups, "M1"),
                         ("S1", "S2", 2, 5))
        self.assertEqual(gen_sprint_plan.milestone_summary(groups, "M2"),
                         ("S2", "S3", 2, 12))
        self.assertEqual(gen_sprint_plan.rendered_milestones(data),
                         [("M1", "S1", "S2", 2, 5), ("M2", "S2", "S3", 2, 12)])

    def test_a_milestone_that_never_leads_a_sprint_keeps_its_line(self):
        # M2 has a story only in S1, behind an M1 story. A walk over each
        # sprint's first story returned M1 alone and the check stayed green.
        data = {"stories": [
            self.story("F-001", "S1", "M1", 3),
            self.story("F-002", "S1", "M2", 4),
            self.story("F-003", "S2", "M1", 2),
        ]}
        self.assertEqual(gen_sprint_plan.rendered_milestones(data),
                         [("M1", "S1", "S2", 2, 5), ("M2", "S1", "S1", 1, 4)])

    def test_a_shared_span_keeps_one_line_per_milestone(self):
        status, _ = self.write_plan_check(self.shared_sprint(),
                                          self.shared_plan(True))
        self.assertEqual(status, 0)

    def test_render_writes_a_heading_for_every_milestone(self):
        with mock.patch.object(gen_sprint_plan, "HOOKS", []):
            text = gen_sprint_plan.render(self.shared_sprint())
        self.assertEqual(text.count("_S1 to S1, 1 stories, 3 engineer-weeks._"), 2)
        self.assertLess(text.index("### M1, One"), text.index("### M2, Two"))
        self.assertLess(text.index("### M2, Two"), text.index("#### Sprint S1"))
        status, _ = self.write_plan_check(
            self.shared_sprint(), text.split("## Goals per sprint", 1)[-1])
        self.assertEqual(status, 0)

    def test_a_duplicated_line_is_refused(self):
        text = self.shared_plan(True).replace(
            "### M2, Two\n\ng\n\n_S1 to S1, 1 stories, 3 engineer-weeks._\n",
            "### M2, Two\n\ng\n\n_S1 to S1, 1 stories, 3 engineer-weeks._\n"
            "_S1 to S1, 1 stories, 3 engineer-weeks._\n", 1)
        status, output = self.write_plan_check(self.shared_sprint(), text)
        self.assertEqual(status, 1)
        self.assertIn("M2 carries 2 summary lines", output)

    def test_a_milestone_the_allocation_does_not_plan_is_refused(self):
        text = self.shared_plan(True) + (
            "\n### M9, Nine\n\ng\n\n_S1 to S1, 1 stories, 3 engineer-weeks._\n")
        status, output = self.write_plan_check(self.shared_sprint(), text)
        self.assertEqual(status, 1)
        self.assertIn("summary line under M9", output)

    def test_a_deleted_line_in_a_shared_span_is_refused(self):
        status, output = self.write_plan_check(self.shared_sprint(),
                                               self.shared_plan(False))
        self.assertEqual(status, 1)
        self.assertIn("M2", output)


# One fragment per refusal that names a table row or a goal line. Each test
# owns one and asserts the other five are absent, which is what tells the
# six refusals apart. `**Goal** lines` is the duplicate-line refusal. The
# tests do not build that state. They only prove their own line does not
# collide with it.
ROW_AND_GOAL_FRAGMENTS = (
    "appears in no SPRINT_PLAN.md sprint table",
    "carries no sprint in BACKLOG.md",
    "carries no **Goal** line",
    "its order or its punctuation has drifted",
    "**Goal** lines",
    "does not carry the story title it is built from",
)


class RowAndGoalRefusals(PlanFixture, unittest.TestCase):
    def assert_lone(self, output: str, fragment: str) -> None:
        problems = [line for line in output.splitlines()
                    if line.startswith("  - ")]
        self.assertEqual(len(problems), 1, output)
        self.assertIn(fragment, problems[0])
        for other in ROW_AND_GOAL_FRAGMENTS:
            if other != fragment:
                self.assertNotIn(other, output)

    def test_a_deleted_story_row_is_absent_from_every_table(self) -> None:
        # R1. The allocation still plans F-002. The goal line still names
        # Second, so the missing-title refusal stays quiet.
        data = self.shared_sprint()
        plan = self.shared_plan(True)
        status, _ = self.write_plan_check(data, plan)
        self.assertEqual(status, 0)
        row = "| F-002 | E2.1 | Second | Rust | 3w |\n"
        self.assertEqual(plan.count(row), 1)
        status, output = self.write_plan_check(data, plan.replace(row, "", 1))
        self.assertEqual(status, 1)
        self.assert_lone(output, "appears in no SPRINT_PLAN.md sprint table")

    def test_an_unplanned_row_carries_no_sprint_in_the_backlog(self) -> None:
        # R2. F-009 is not in the allocation. The goal line is unchanged,
        # so the goal refusals stay quiet.
        data = self.shared_sprint()
        plan = self.shared_plan(True)
        status, _ = self.write_plan_check(data, plan)
        self.assertEqual(status, 0)
        anchor = "| F-002 | E2.1 | Second | Rust | 3w |"
        extra = "| F-009 | E9.9 | Extra | Rust | 1w |"
        self.assertEqual(plan.count(anchor), 1)
        mutated = plan.replace(anchor, anchor + "\n" + extra, 1)
        status, output = self.write_plan_check(data, mutated)
        self.assertEqual(status, 1)
        self.assert_lone(output, "carries no sprint in BACKLOG.md")

    def test_a_deleted_goal_line_is_the_only_problem(self) -> None:
        # R3. The sprint heading and both table rows stay.
        data = self.shared_sprint()
        plan = self.shared_plan(True)
        status, _ = self.write_plan_check(data, plan)
        self.assertEqual(status, 0)
        goal = "**Goal**: First, Second.\n"
        self.assertEqual(plan.count(goal), 1)
        status, output = self.write_plan_check(data, plan.replace(goal, "", 1))
        self.assertEqual(status, 1)
        self.assert_lone(output, "carries no **Goal** line")

    def test_a_swapped_goal_line_is_order_drift(self) -> None:
        # R4. Both titles remain, in the other order.
        data = self.shared_sprint()
        plan = self.shared_plan(True)
        status, _ = self.write_plan_check(data, plan)
        self.assertEqual(status, 0)
        mutated = plan.replace(
            "**Goal**: First, Second.", "**Goal**: Second, First.", 1)
        status, output = self.write_plan_check(data, mutated)
        self.assertEqual(status, 1)
        self.assert_lone(output, "its order or its punctuation has drifted")

    def test_a_missing_title_is_the_only_problem(self) -> None:
        # R6. Second is gone from the goal line and Other is not a story.
        data = self.shared_sprint()
        plan = self.shared_plan(True)
        status, _ = self.write_plan_check(data, plan)
        self.assertEqual(status, 0)
        mutated = plan.replace(
            "**Goal**: First, Second.", "**Goal**: First, Other.", 1)
        status, output = self.write_plan_check(data, mutated)
        self.assertEqual(status, 1)
        self.assert_lone(
            output, "does not carry the story title it is built from")


if __name__ == "__main__":
    unittest.main()
