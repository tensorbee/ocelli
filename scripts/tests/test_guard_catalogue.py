#!/usr/bin/env python3
"""Tests for the guard catalogue, the discovery scan and the sandbox. F-X009.

    python3 -m unittest discover -s scripts/tests -p 'test_guard_catalogue.py'

Needs nothing but the standard library, and nothing here builds a sandbox: the
sandbox's own refusals are covered by `scripts/guard_probe.py --self-test`,
which the `guards` gate runs, and building one per test would make this file
slower than the harness it tests.

**These are assertions about the DECLARATION.** That every entry is well
formed, that its citation resolves to a real section or a real deviation row,
that its profile agrees with what it needs, and that the discovery scan finds
the shapes a refusal takes here. Whether each guard actually refuses is the
probe runner's job and not this file's.
"""

from __future__ import annotations

import re
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

from guards import census, discover, sandbox  # noqa: E402
from guards.catalogue import (CONSTANTS, DEFECTS, DICOM_FIXTURE,  # noqa: E402
                              GUARDS)


class CatalogueIsWellFormed(unittest.TestCase):
    def test_every_entry_has_a_unique_id(self) -> None:
        ids = [g.id for g in GUARDS]
        self.assertEqual(len(ids), len(set(ids)), sorted(ids))

    def test_every_probe_has_a_unique_id(self) -> None:
        ids = [p.id for g in GUARDS for p in g.probes]
        self.assertEqual(len(ids), len(set(ids)), sorted(ids))

    def test_every_entry_names_a_tracked_file(self) -> None:
        for guard in GUARDS:
            with self.subTest(guard.id):
                if "*" in guard.file:
                    continue
                self.assertTrue((ROOT / guard.file).is_file(), guard.file)

    def test_every_entry_carries_a_citation_and_a_refuses_sentence(
            self) -> None:
        """`spec` and `refuses` are what make this satisfy HLD 27.2 R2.

        A probe derived from the guard's source asserts the current regex and
        passes forever once the regex has been weakened to match. These two
        fields are where a reviewer checks that it was not.
        """
        for guard in GUARDS:
            with self.subTest(guard.id):
                self.assertTrue(guard.spec.strip(), guard.id)
                self.assertTrue(guard.refuses.strip(), guard.id)
                self.assertGreater(len(guard.refuses), 25, guard.id)

    def test_a_declared_out_of_scope_entry_carries_its_reason(self) -> None:
        for guard in GUARDS:
            if guard.kind == "guard":
                continue
            with self.subTest(guard.id):
                self.assertGreater(len(guard.reason), 60, guard.id)

    def test_every_probe_declares_a_message_fragment_or_declares_silence(
            self) -> None:
        """`expect` proves the run went red for the DECLARED reason.

        A run that failed for another reason proves nothing about the guard it
        aimed at, which is the argument tools/oracle/src/faults.mjs makes for
        its own fragments. The one exception is a refusal with no words, and
        that has to be declared rather than left to look like an oversight.
        """
        silent = {g.id for g in GUARDS if g.silent}
        for guard in GUARDS:
            for probe in guard.probes:
                with self.subTest(probe.id):
                    if guard.id in silent:
                        continue
                    self.assertTrue(probe.expect.strip(), probe.id)

    def test_a_known_defect_names_a_declared_defect(self) -> None:
        named = {p.defect for g in GUARDS for p in g.probes if p.defect}
        self.assertEqual(named, set(DEFECTS))
        for key, text in DEFECTS.items():
            with self.subTest(key):
                self.assertGreater(len(text), 100, key)

    def test_polarity_is_one_of_two_values(self) -> None:
        for guard in GUARDS:
            for probe in guard.probes:
                with self.subTest(probe.id):
                    self.assertIn(probe.polarity, {"refuse", "accept"})


class ProfileAgreesWithNeeds(unittest.TestCase):
    """`.claude/WORKFLOW.md`'s floor definition, as a mechanism."""

    def test_the_real_catalogue_agrees(self) -> None:
        rows = [(p.id, p.needs, p.profile) for g in GUARDS for p in g.probes]
        self.assertEqual(census.profile_problems(rows), [])

    def test_a_floor_entry_needing_a_gpu_is_refused(self) -> None:
        problems = census.profile_problems([("probe", "gpu", "floor")])
        self.assertTrue(any("may not be in the floor" in p for p in problems))

    def test_a_floor_entry_needing_cargo_is_refused(self) -> None:
        problems = census.profile_problems([("probe", "cargo", "floor")])
        self.assertTrue(any("may not be in the floor" in p for p in problems))

    def test_a_deep_entry_needing_a_browser_is_allowed(self) -> None:
        self.assertEqual(census.profile_problems([("p", "browser", "deep")]),
                         [])


class SpecCitationsResolve(unittest.TestCase):
    DEVIATIONS = ROOT / "docs" / "hld" / "DEVIATIONS.md"

    def test_every_cited_deviation_is_a_declared_row(self) -> None:
        """A catalogue citing an undeclared deviation is the failure mode
        scripts/deviation_check.py exists for, arriving from a directory that
        checker does not scan."""
        declared = set(re.findall(r"^\|\s*D-(\d{2})\s*\|",
                                  self.DEVIATIONS.read_text(encoding="utf-8"),
                                  re.M))
        for guard in GUARDS:
            for token in re.findall(r"\bD-(\d{2})\b", guard.spec):
                with self.subTest(f"{guard.id}:D-{token}"):
                    self.assertIn(token, declared)

    def test_every_cited_repository_path_exists(self) -> None:
        for guard in GUARDS:
            for path in re.findall(r"`([a-z][\w./-]+\.(?:md|py|mjs|sh|json|"
                                   r"toml|js))`", guard.spec):
                with self.subTest(f"{guard.id}:{path}"):
                    self.assertTrue((ROOT / path).exists(), path)


class DiscoveryFindsTheShapes(unittest.TestCase):
    """The scan is what makes the census mechanical rather than remembered."""

    def test_it_finds_a_collected_python_problem(self) -> None:
        sites = discover._sites_in(
            "probe.py",
            "def check(problems):\n"
            "    problems.append('a refusal nobody declared')\n")
        self.assertEqual(len(sites), 1)
        self.assertIn("a refusal nobody declared", sites[0].message)

    def test_it_finds_a_printed_python_refusal(self) -> None:
        sites = discover._sites_in(
            "probe.py", 'def main():\n    print("FAIL: it is wrong")\n')
        self.assertEqual([s.shape for s in sites], ["py-print-fail"])

    def test_it_finds_a_raised_refusal(self) -> None:
        sites = discover._sites_in(
            "probe.py", 'raise SystemExit("it cannot run")\n')
        self.assertEqual([s.shape for s in sites], ["py-raise"])

    def test_it_finds_a_javascript_throw(self) -> None:
        sites = discover._sites_in(
            "probe.mjs", 'throw new Error("the frame never presented");\n')
        self.assertEqual([s.shape for s in sites], ["js-throw"])

    def test_it_finds_a_shell_refusal(self) -> None:
        sites = discover._sites_in(
            "probe.sh", 'echo "FAIL: $name reaches it"\nexit 1\n')
        self.assertEqual(sorted(s.shape for s in sites),
                         ["sh-exit", "sh-fail"])

    def test_a_site_is_identified_by_its_words_and_not_its_line(self) -> None:
        """Moving a refusal within a file must not churn the catalogue, and
        rewording one must force a re-read of what it is for."""
        body = "problems.append('the same words')\n"
        first = discover._sites_in("probe.py", body)[0]
        second = discover._sites_in("probe.py", "\n\n\n" + body)[0]
        self.assertEqual(first.key, second.key)
        self.assertNotEqual(
            first.key,
            discover._sites_in("probe.py",
                               "problems.append('other words')\n")[0].key)

    def test_it_does_not_scan_test_suites(self) -> None:
        """A test's own assertions are not guards, and scanning them would
        make every throw in a suite a site needing a catalogue entry."""
        for name in discover.tracked_sources():
            self.assertFalse(name.startswith(discover.SCAN_EXCLUDE), name)

    def test_it_does_not_scan_crates(self) -> None:
        """Decision 7 of the design plan: a runtime refusal inside a crate is
        that crate's story's test."""
        for name in discover.tracked_sources():
            self.assertFalse(name.startswith("crates/"), name)


class TheEnvironmentScrub(unittest.TestCase):
    def test_it_removes_every_inherited_git_variable(self) -> None:
        import os
        os.environ["GIT_INDEX_FILE"] = "/somewhere/else"
        try:
            env = sandbox.scrubbed_env()
        finally:
            del os.environ["GIT_INDEX_FILE"]
        self.assertNotIn("GIT_INDEX_FILE", env)
        self.assertEqual(env["GIT_CONFIG_GLOBAL"], os.devnull)
        self.assertEqual(env["GIT_CONFIG_SYSTEM"], os.devnull)

    def test_repo_read_refuses_a_write_verb(self) -> None:
        """The one function that names the real repository is read-only, and
        a write can only arrive by editing it."""
        for verb in ("add", "commit", "checkout", "reset", "clean",
                     "write-tree", "update-index"):
            with self.subTest(verb):
                with self.assertRaises(sandbox.SandboxError):
                    sandbox.repo_read(verb, "--help")


class TheTripwire(unittest.TestCase):
    # The tripwire is the mechanism behind `sandbox.py`'s opening claim that
    # nothing here may write inside the real repository. Every other test in
    # this class iterates over `TRIPWIRE_READS`, so with that tuple emptied
    # they all pass vacuously and the claim is watched by nothing. The S03
    # sprint review's second pass proved exactly that: `TRIPWIRE_READS = ()`
    # left thirty tests and the self test green.
    #
    # So the SET is asserted here, by name, from the outside. Adding a read is
    # a deliberate change to this list. Removing one fails here rather than
    # quietly shrinking what the tripwire watches.
    EXPECTED_READS = {"HEAD", "unstaged", "staged", "untracked", "hooksPath"}

    def test_it_watches_exactly_the_declared_set(self) -> None:
        labels = {label for label, _ in sandbox.TRIPWIRE_READS}
        self.assertEqual(
            labels,
            self.EXPECTED_READS,
            "the tripwire's coverage changed. Each of these is a way the "
            "developer's repository could be disturbed: the commit it is on, "
            "its unstaged and staged content, its untracked files, and whether "
            "its hooks are enabled. Removing one narrows what a probe run is "
            "allowed to disturb without saying so.",
        )

    def test_each_declared_read_actually_runs_a_git_command(self) -> None:
        for label, args in sandbox.TRIPWIRE_READS:
            with self.subTest(label):
                self.assertTrue(args, f"{label} names no git command")
                self.assertIn(
                    args[0],
                    {"rev-parse", "diff", "ls-files", "config", "status"},
                    f"{label} does not read anything git knows about",
                )

    def test_it_reports_a_planted_change_to_each_thing_it_captures(
            self) -> None:
        before = {label: "before" for label, _ in sandbox.TRIPWIRE_READS}
        for label, _ in sandbox.TRIPWIRE_READS:
            after = dict(before)
            after[label] = "after"
            with self.subTest(label):
                changed = sandbox.tripwire_compare(before, after)
                self.assertEqual(len(changed), 1)
                self.assertIn(label, changed[0])

    def test_it_is_quiet_when_nothing_moved(self) -> None:
        before = {label: "same" for label, _ in sandbox.TRIPWIRE_READS}
        self.assertEqual(sandbox.tripwire_compare(before, dict(before)), [])


class TheDeclaredConstantRatchet(unittest.TestCase):
    def test_every_declared_constant_can_be_read(self) -> None:
        for constant in CONSTANTS:
            with self.subTest(f"{constant.file}:{constant.name}"):
                self.assertIsNotNone(
                    census.constant_value(constant, ROOT),
                    f"{constant.name} cannot be read from {constant.file}, so "
                    f"the ratchet cannot see the value that decides how "
                    f"strict the guard is")

    def test_a_changed_value_changes_its_digest(self) -> None:
        first = census.constant_digest('{".dcm", ".dicom", ".ima"}')
        second = census.constant_digest('{".dcm"}')
        self.assertNotEqual(first, second)

    def test_a_tunable_constant_is_still_recorded(self) -> None:
        """A tunable value is still in the ratchet, because the point is that
        the change appears in a diff and not that it is forbidden."""
        tunable = [c for c in CONSTANTS if c.tunable]
        self.assertTrue(tunable)
        for constant in tunable:
            with self.subTest(constant.name):
                self.assertTrue(constant.why.strip())


class TheDicomFixture(unittest.TestCase):
    """PS3.10: 128 preamble bytes then `DICM`. Synthesised, never a corpus row.

    A real row would put patient data in a temporary directory for no gain,
    and the corpus is absent in CI anyway.
    """

    def test_it_is_the_shape_the_guard_refuses(self) -> None:
        self.assertEqual(len(DICOM_FIXTURE), 132)
        self.assertEqual(DICOM_FIXTURE[128:132], b"DICM")
        self.assertEqual(set(DICOM_FIXTURE[:128]), {0})


if __name__ == "__main__":
    unittest.main()
