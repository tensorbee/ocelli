#!/usr/bin/env python3
"""Tests for scripts/shipped_native_check.py.

    python3 -B -m unittest discover -s scripts/tests \
      -p 'test_shipped_native_check.py'

Needs nothing but the standard library. Every case hands `graph_problems` a
return code and captured output strings, so nothing here runs cargo.

**Expected values are from F-238's acceptance rows, not from the
implementation.** AC1: the shipped build's resolved graph has no `objc2*`
package for `aarch64-apple-darwin` and `x86_64-apple-darwin`, and the check
refuses any objc2 name. AC2: the refusal reads the resolved graph rather than
a feature list, so an `objc2` line in `cargo tree` output is the trigger and
a manifest is never parsed. The package line shapes come from `cargo tree
--prefix none --format {p}`: `name vVersion`, `name vVersion (/path)` for a
path dependency, and a trailing ` (*)` on a subtree printed before.
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import shipped_native_check as check  # noqa: E402

TARGET = "aarch64-apple-darwin"


def tree(*packages: str) -> str:
    return "".join(line + "\n" for line in packages)


class GraphProblems(unittest.TestCase):
    def test_an_objc2_line_is_refused_with_name_version_and_target(self) -> None:
        problems = check.graph_problems(
            TARGET, 0, tree("wgpu v30.0.1", "objc2 v0.6.4"), "")
        self.assertEqual(problems, [
            "objc2 v0.6.4 reaches the shipped native graph for "
            + TARGET,
        ])

    def test_every_hyphenated_objc2_crate_is_refused(self) -> None:
        problems = check.graph_problems(
            TARGET, 0,
            tree("objc2-metal v0.3.2", "objc2-encode v4.1.0"), "")
        self.assertEqual(len(problems), 2)
        self.assertIn("objc2-metal v0.3.2", problems[0])
        self.assertIn("objc2-encode v4.1.0", problems[1])

    def test_a_name_containing_objc2_is_not_the_family(self) -> None:
        self.assertEqual(
            check.graph_problems(TARGET, 0, tree("notobjc2 v1.0.0"), ""), [])

    def test_a_path_suffix_and_a_repeat_marker_do_not_hide_a_crate(
            self) -> None:
        problems = check.graph_problems(
            TARGET, 0,
            tree("objc2-foundation v0.3.2 (/somewhere/objc2-foundation)",
                 "wgpu v30.0.1 (*)"),
            "")
        self.assertEqual(len(problems), 1)
        self.assertIn("objc2-foundation v0.3.2", problems[0])

    def test_a_graph_without_objc2_is_clean(self) -> None:
        self.assertEqual(
            check.graph_problems(
                TARGET, 0,
                tree("ocelli-native v0.1.0 (/repo/crates/ocelli-native)",
                     "ocelli-render v0.1.0 (/repo/crates/ocelli-render)",
                     "wgpu v30.0.1", "ash v0.38.0+1.3.281"),
                ""),
            [])

    def test_a_failed_resolve_is_refused_and_names_the_target(self) -> None:
        problems = check.graph_problems(
            TARGET, 101, "", "error: the lock file needs to be updated")
        self.assertEqual(len(problems), 1)
        self.assertIn(TARGET, problems[0])
        self.assertIn("did not resolve", problems[0])
        self.assertIn("the lock file needs to be updated", problems[0])

    def test_the_target_pair_is_both_apple_triples(self) -> None:
        # AC1 names exactly these two. A third target or a missing one would
        # change what the check claims to prove.
        self.assertEqual(
            check.APPLE_TARGETS,
            ("aarch64-apple-darwin", "x86_64-apple-darwin"))

    def test_the_walk_reads_normal_and_build_edges(self) -> None:
        # A build-dependency is compiled when the archive is built, so the
        # story row's "compiles no objc2 crate" covers it, and the licence
        # walk reads the same kinds. A walk of normal edges alone stayed
        # green when objc2 was planted as a build-dependency of
        # `ocelli-native`, which is the F-238 review's S1.
        self.assertIn("-e", check.COMMAND)
        kinds = check.COMMAND[check.COMMAND.index("-e") + 1]
        self.assertEqual(kinds, "normal,build")


if __name__ == "__main__":
    unittest.main()
