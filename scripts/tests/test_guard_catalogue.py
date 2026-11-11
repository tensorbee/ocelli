#!/usr/bin/env python3
"""Tests for the guard catalogue, the discovery scan and the sandbox. F-X009.

    python3 -m unittest discover -s scripts/tests -p 'test_guard_catalogue.py'

Needs nothing but the standard library. One close-state regression builds a
real disposable sandbox to distinguish inherited fixture handoffs from a
handoff planted after setup. A launcher regression observes actual child PID
and URL propagation in a disposable fixture. The sandbox's own refusals are covered by
`scripts/guard_probe.py --self-test`, which the `guards` gate runs.

**These are assertions about the DECLARATION.** That every entry is well
formed, that its citation resolves to a real section or a real deviation row,
that its profile agrees with what it needs, and that the discovery scan finds
the shapes a refusal takes here. Whether each guard actually refuses is the
probe runner's job and not this file's.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
import tempfile
import tomllib
import unittest
from unittest import mock
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

from guards import catalogue, census, discover, sandbox  # noqa: E402
from guards.catalogue import (CONSTANTS, DEFECTS, DICOM_FIXTURE,  # noqa: E402
                              GUARDS, _close_preflight, _run_docs_gate,
                              _write_docs_fixture,
                              _example_dev_reaches_a_url, _write_close_state,
                              _workspace_manifest_with_exclusion)


class SandboxCopyPreservesTrackedShape(unittest.TestCase):
    def test_a_relative_symlink_stays_a_relative_symlink(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = root / "repository"
            destination_root = root / "sandbox"
            repository.mkdir()
            destination_root.mkdir()
            (repository / "LICENSE-MIT").write_text("grant\n", encoding="utf-8")
            source = repository / "crates" / "ocelli-wasm" / "LICENSE-MIT"
            source.parent.mkdir(parents=True)
            source.symlink_to("../../LICENSE-MIT")
            destination = destination_root / "crates" / "ocelli-wasm" / "LICENSE-MIT"

            sandbox.copy_tracked_path(
                source,
                destination,
                source_root=repository,
                destination_root=destination_root,
            )

            self.assertTrue(destination.is_symlink())
            self.assertEqual(destination.readlink(), Path("../../LICENSE-MIT"))

    def test_an_absolute_symlink_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = root / "repository"
            destination_root = root / "sandbox"
            source = repository / "absolute-link"
            repository.mkdir()
            destination_root.mkdir()
            source.symlink_to(root / "outside")

            with self.assertRaisesRegex(
                    sandbox.SandboxError, "absolute symlink"):
                sandbox.copy_tracked_path(
                    source,
                    destination_root / "absolute-link",
                    source_root=repository,
                    destination_root=destination_root,
                )

    def test_a_relative_symlink_escaping_only_the_source_root_is_refused(
            self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = root / "repository"
            destination_root = root / "sandbox"
            destination_root.mkdir()
            source = repository / "nested" / "escaping-link"
            source.parent.mkdir(parents=True)
            source.symlink_to("../../outside")

            with self.assertRaisesRegex(
                    sandbox.SandboxError, "escapes"):
                sandbox.copy_tracked_path(
                    source,
                    destination_root / "nested" / "deeper" / "escaping-link",
                    source_root=repository,
                    destination_root=destination_root,
                )

    def test_a_relative_symlink_escaping_only_the_destination_root_is_refused(
            self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repository = root / "repository"
            destination_root = root / "sandbox"
            destination_root.mkdir()
            source = repository / "nested" / "deeper" / "escaping-link"
            source.parent.mkdir(parents=True)
            source.symlink_to("../../outside")

            with self.assertRaisesRegex(
                    sandbox.SandboxError, "escapes"):
                sandbox.copy_tracked_path(
                    source,
                    destination_root / "nested" / "escaping-link",
                    source_root=repository,
                    destination_root=destination_root,
                )

    def test_an_unrepresentable_shape_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "tracked-directory"
            source.mkdir()

            with self.assertRaisesRegex(
                    sandbox.SandboxError,
                    "neither a regular file nor a symlink"):
                sandbox.copy_tracked_path(
                    source,
                    root / "copy",
                    source_root=root,
                    destination_root=root,
                )



class CloseStateHandoffIsolation(unittest.TestCase):
    def test_copied_and_planted_handoffs_are_distinct(self) -> None:
        source_handoffs = ROOT / ".claude" / "handoffs"
        before = {p.name: p.read_bytes()
                  for p in source_handoffs.glob("*-ready.md")}

        def commit_handoff(box: sandbox.Sandbox, name: str) -> str:
            relative = f".claude/handoffs/{name}-ready.md"
            box.write(relative, "Own synthetic handoff for close-state control.\n")
            box.git("add", "--", relative)
            box.git("commit", "-q", "-m", "probe inherited handoff", "--", relative)
            return relative

        def ready(box: sandbox.Sandbox) -> None:
            result = _close_preflight(box)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn("is ready to close", result.stdout)
            self.assertEqual(box.git("status", "--porcelain").stdout, "")

        try:
            with sandbox.sandbox() as box:
                inherited = commit_handoff(box, "probe-inherited")
                # Exercise the actual lazy control, which prepares state once.
                ready(box)
                self.assertFalse((box.path / inherited).exists())
                self.assertEqual(list((box.path / ".claude/handoffs").glob("*-ready.md")), [])

                # The second setup has an already-done backlog and ignored
                # run state. It must not force that old state into the index.
                again = commit_handoff(box, "probe-inherited-again")
                _write_close_state(box)
                self.assertFalse((box.path / again).exists())
                ready(box)

                planted = commit_handoff(box, "probe-unconsumed")
                sprint = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)",
                                   box.read("docs/sprints/CURRENT_SPRINT.md"), re.M)
                self.assertIsNotNone(sprint)
                state_path = f".claude/scratch/{sprint.group(1)}-run.json"
                data = json.loads(box.read(state_path))
                tree = box.git("write-tree").stdout.strip()
                data["sprint_reviews"][-1]["tree"] = tree
                data["verifications"][-1]["tree"] = tree
                box.write(state_path, json.dumps(data) + "\n")
                result = _close_preflight(box)
                self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
                self.assertIn("handoffs remain: probe-unconsumed-ready.md", result.stdout)
                self.assertNotIn("stale", result.stdout)
                self.assertNotIn("working tree is not clean", result.stdout)
                self.assertNotIn("is not HEAD tree", result.stdout)
                self.assertTrue((box.path / planted).is_file())
        finally:
            after = {p.name: p.read_bytes()
                     for p in source_handoffs.glob("*-ready.md")}
            self.assertEqual(after, before, "source handoffs remain byte-identical")

class WorkspaceExcludeProbeBuilder(unittest.TestCase):
    def test_it_extends_an_existing_exclusion_without_a_duplicate_key(
            self) -> None:
        manifest = (
            '[workspace]\n'
            'resolver = "3"\n'
            'members = ["crates/*", "tools/oracle"]\n'
            'exclude = ["vendor/ritk-codecs-0.6.0"]\n\n'
            '[workspace.package]\n'
            'edition = "2024"\n'
        )

        updated = _workspace_manifest_with_exclusion(manifest, "tools/oracle")

        workspace = tomllib.loads(updated)["workspace"]
        self.assertEqual(
            workspace["exclude"],
            ["vendor/ritk-codecs-0.6.0", "tools/oracle"],
        )
        self.assertEqual(updated.count("\nexclude = "), 1)

    def test_multiline_exclusions_comments_and_quoted_brackets_stay_valid(self) -> None:
        manifest = (            '[workspace]\nresolver = "3"\nmembers = ["crates/*"]\n'
            'exclude = [\n "vendor/a]", # a comment containing ]\n'
            ' "vendor/b",\n] # closing comment\n'
            '[workspace.package]\nversion = "0.1.0"\n')
        updated = _workspace_manifest_with_exclusion(manifest, "tools/oracle")
        before = tomllib.loads(manifest)
        after = tomllib.loads(updated)
        before["workspace"]["exclude"].append("tools/oracle")
        self.assertEqual(after, before)
        self.assertEqual(_workspace_manifest_with_exclusion(updated, "tools/oracle"), updated)

    def test_missing_empty_and_single_line_lists_preserve_other_tables(self) -> None:
        for row in ["", "exclude = []\n", 'exclude = ["vendor/a"] # comment\n']:
            with self.subTest(row=row):
                manifest = ('[workspace]\nmembers = ["member"]\n' + row +
                            '[package]\nname = "other"\nexclude = ["leave-alone"]\n')
                expected = tomllib.loads(manifest)
                expected["workspace"].setdefault("exclude", []).append("tools/oracle")
                self.assertEqual(tomllib.loads(_workspace_manifest_with_exclusion(
                    manifest, "tools/oracle")), expected)

    def test_current_workspace_multiline_list_is_retained(self) -> None:
        manifest = (ROOT / "Cargo.toml").read_text()
        expected = tomllib.loads(manifest)
        excluded = expected["workspace"].setdefault("exclude", [])
        if "tools/oracle" not in excluded:
            excluded.append("tools/oracle")
        self.assertEqual(tomllib.loads(_workspace_manifest_with_exclusion(
            manifest, "tools/oracle")), expected)

    def test_unsupported_fixture_layouts_fail_before_a_duplicate_key(self) -> None:
        cases = [
            ('workspace = { members = [] }\n', r'no \[workspace\] table'),
            ('[workspace]\n"exclude" = []\n', 'unsupported spelling'),
            ('[workspace]\nexclude = ["""vendor/\n[fixture]\n"""]\n',
             'could not be delimited'),
        ]
        for manifest, message in cases:
            with self.subTest(manifest=manifest):
                tomllib.loads(manifest)  # Valid TOML can still be an unsupported fixture layout.
                with self.assertRaisesRegex(AssertionError, message):
                    _workspace_manifest_with_exclusion(manifest, "tools/oracle")


class LintModuleProbeWitness(unittest.TestCase):
    def test_exact_increment_is_independent_of_repository_module_count(self) -> None:
        from guards.catalogue import _one_added_lint_path, _record_lint_path_baseline
        import subprocess
        for baseline in [0, 3, 41]:
            box = mock.Mock()
            text = f"OK: {baseline} `#[path]` module(s) followed"
            box.run.return_value = subprocess.CompletedProcess([], 0, text, "")
            _record_lint_path_baseline(box)
            box.write.assert_called_once_with("probe-shared/baseline-path-count.json", str(baseline))
            box.read.return_value = str(baseline)
            for offset in [0, 1, 2]:
                box.run.return_value = subprocess.CompletedProcess(
                    [], 0, f"OK: {baseline + offset} `#[path]` module(s) followed", "")
                result = _one_added_lint_path(box)
                self.assertEqual(result.returncode, 0 if offset == 1 else 1)
                self.assertEqual("one additional `#[path]` module followed" in result.stdout,
                                 offset == 1)

    def test_guard_failure_propagates_and_countless_success_has_no_witness(self) -> None:
        from guards.catalogue import _one_added_lint_path, _record_lint_path_baseline
        import subprocess
        box = mock.Mock()
        failure = subprocess.CompletedProcess([], 1, "actual guard refusal", "detail")
        box.run.return_value = failure
        self.assertIs(_one_added_lint_path(box), failure)
        with self.assertRaisesRegex(AssertionError, "baseline guard failed"):
            _record_lint_path_baseline(box)
        box.run.return_value = subprocess.CompletedProcess([], 0, "OK: no count", "")
        box.read.return_value = "0"
        with self.assertRaisesRegex(AssertionError, "did not report followed"):
            _one_added_lint_path(box)


class ExampleDevControlKeepsBackgroundChild(unittest.TestCase):
    def test_the_control_observes_its_background_pid_and_exported_url(self) -> None:
        launcher = (ROOT / "bin/example-dev.sh").read_bytes()
        with tempfile.TemporaryDirectory() as directory:
            box = sandbox.Sandbox(path=Path(directory))
            target = box.write("bin/example-dev.sh", launcher)
            target.chmod(0o755)
            original_run = box.run

            def observed_run(argv, **kwargs):
                # The witness follows both assertions. An EXIT trap may mask a
                # shell failure's status, so status alone cannot prove these ran.
                box.append(
                    "bin/example-dev.sh",
                    '\ntest -n "${pid-}" || exit 71\n'
                    'test "${VITE_EXAMPLE_SERVER-}" = http://127.0.0.1:9 || exit 72\n'
                    'printf "%s\\n" "fixture: child PID and URL observed"\n')
                return original_run(argv, **kwargs)

            with mock.patch.object(box, "run", side_effect=observed_run):
                done = _example_dev_reaches_a_url(box)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertIn("example helper http://127.0.0.1:9", done.stderr)
            self.assertIn("fixture: child PID and URL observed", done.stdout)
        self.assertEqual((ROOT / "bin/example-dev.sh").read_bytes(), launcher)


class DocProbeHarnessPreconditions(unittest.TestCase):
    """Harness preconditions: helper input refusal and no-op transport rejection."""

    def test_write_docs_fixture_rejects_unknown_fault(self) -> None:
        with tempfile.TemporaryDirectory() as tempdirectory:
            box = sandbox.Sandbox(path=Path(tempdirectory))
            with self.assertRaises(ValueError) as caught:
                _write_docs_fixture(box, 'unsupported-arm')
        self.assertEqual(str(caught.exception), "unknown fault: 'unsupported-arm'")

    def test_run_docs_gate_rejects_noop_transport(self) -> None:
        with tempfile.TemporaryDirectory() as tempdirectory:
            box = sandbox.Sandbox(path=Path(tempdirectory))
            with mock.patch.object(box, 'run', return_value=subprocess.CompletedProcess([], 0, '', '')) as fake_run:
                with self.assertRaises(AssertionError) as caught:
                    _run_docs_gate(box, 'healthy')
            fake_run.assert_called_once()
        self.assertEqual(str(caught.exception), 'docs gate passed without real rendered docs')


class UnextractableGateProbeSelection(unittest.TestCase):
    def test_selector_picks_joined_arm(self):
        with tempfile.TemporaryDirectory() as directory:
            box = sandbox.Sandbox(path=Path(directory))
            box.write(
                "scripts/ci_floor_check.py",
                "NOT_IN_FLOOR = {}\n",
            )
            box.write(
                ".github/workflows/ci.yml",
                "jobs:\n"
                "  synth:\n"
                "    steps:\n"
                "      - run: bin/ocelli.sh gate wrapped\n"
                "      - run: bin/ocelli.sh gate inline\n"
                "      - run: bin/ocelli.sh gate joined\n",
            )
            box.write(
                "bin/ocelli.sh",
                "GATES=(\n"
                '  "wrapped|no|fixture"\n'
                '  "inline|no|fixture"\n'
                '  "joined|no|fixture"\n'
                ")\n"
                "run_gate() { local name=$1; case \"$name\" in\n"
                "  wrapped) env FLAG=x cargo doc --no-deps ;;\n"
                "  inline) env FLAG=x cargo doc --no-deps &&\n"
                "    python3 later.py ;;\n"
                "  joined) python3 first.py &&\n"
                "    python3 second.py &&\n"
                "    node --test synthetic.mjs ;;\n"
                "esac; }\n"
                "skip() { return 3; }\n",
            )
            runner_before = box.read('bin/ocelli.sh')
            selected = catalogue._a_gate_with_an_unextractable_arm_command(box)
            self.assertEqual(selected, "joined")
            self.assertEqual(box.read('bin/ocelli.sh'), runner_before)


class CatalogueIsWellFormed(unittest.TestCase):
    def test_the_skills_gate_names_its_checker_and_suite_exactly(self) -> None:
        """The named CI gate must not lose either executable check."""
        runner = (ROOT / "bin" / "ocelli.sh").read_text(encoding="utf-8")
        arm = runner[runner.index("    skills)"):
                     runner.index("    lint)")]
        self.assertEqual(
            arm,
            "    skills)      python3 scripts/sync_agent_skills.py --check &&\n"
            "                 python3 scripts/skill_examples_check.py &&\n"
            "                 python3 -B -m unittest discover -s scripts/tests \\\n"
            "                   -p test_skill_examples_check.py ;;\n",
        )

    def test_the_guards_gate_names_its_python_suites_exactly(self) -> None:
        """A focused suite must not exist only as a manual invocation."""
        runner = (ROOT / "bin" / "ocelli.sh").read_text(encoding="utf-8")
        arm = runner[runner.index("    guards)"):
                     runner.index("    guards-deep)")]
        suites = re.findall(
            r"python3 -B -m unittest discover -s scripts/tests \\\n"
            r"\s+-p ([\w.]+)",
            arm,
        )
        self.assertEqual(
            suites,
            [
                "test_guard_catalogue.py",
                "test_sprint_workflow.py",
                "test_gen_sprint_plan.py",
                "test_guard_readers.py",
                "test_staged_readers.py",
                "test_release_wasm.py",
            ],
        )

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

    # The two floor branches share the words "may not be in the floor", and
    # the second subsumes the first, so both of these and the
    # `census.floor-needing-a-gpu` probe went on passing with the GPU rule
    # deleted. Each now asserts the sentence its own branch writes, and that
    # the OTHER branch's sentence is absent.
    GPU_SENTENCE = "no GPU, no browser and no corpus"
    TOOLCHAIN_SENTENCE = "no cargo, no npm, no wasm-pack"

    def test_a_floor_entry_needing_a_gpu_is_refused(self) -> None:
        problems = census.profile_problems([("probe", "gpu", "floor")])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn(self.GPU_SENTENCE, problems[0])
        self.assertNotIn(self.TOOLCHAIN_SENTENCE, problems[0])

    def test_a_floor_entry_needing_a_browser_or_the_corpus_is_refused(
            self) -> None:
        for needs in ("browser", "corpus"):
            with self.subTest(needs):
                problems = census.profile_problems([("p", needs, "floor")])
                self.assertEqual(len(problems), 1, problems)
                self.assertIn(self.GPU_SENTENCE, problems[0])

    def test_a_floor_entry_needing_cargo_is_refused(self) -> None:
        problems = census.profile_problems([("probe", "cargo", "floor")])
        self.assertEqual(len(problems), 1, problems)
        self.assertIn(self.TOOLCHAIN_SENTENCE, problems[0])
        self.assertNotIn(self.GPU_SENTENCE, problems[0])

    def test_a_deep_entry_needing_a_browser_is_allowed(self) -> None:
        self.assertEqual(census.profile_problems([("p", "browser", "deep")]),
                         [])


class KindAndGateAreValidated(unittest.TestCase):
    """Two fields that decide a bucket, and nothing checked either of them.

    `kind` decides whether an entry's refusals sit in the uncovered ratchet or
    in "declared out of scope", so a typo such as `not_a_guard` moves them out
    of the count and nothing anywhere said so. `gate` decides nothing at all in
    this harness, which is the point: a probe invokes the guard's own script,
    so deleting a row from `bin/ocelli.sh`'s GATES array shrank `--floor`,
    `--sprint` and `--all` and left every count here unchanged.
    """

    # Named from the outside, in the shape `TheTripwire.EXPECTED_READS` uses.
    # Asserting `guard.kind in census.KINDS` alone would pass with KINDS
    # widened to hold the typo.
    EXPECTED_KINDS = {"guard", "not-a-guard"}

    def test_the_declared_set_is_exactly_two(self) -> None:
        self.assertEqual(census.KINDS, self.EXPECTED_KINDS)

    def test_every_entry_declares_one_of_them(self) -> None:
        for guard in GUARDS:
            with self.subTest(guard.id):
                self.assertIn(guard.kind, self.EXPECTED_KINDS, guard.id)

    def test_every_entry_names_a_declared_gate_or_the_sentinel(self) -> None:
        declared = set(census.gates_declared())
        self.assertTrue(declared, "bin/ocelli.sh's GATES array parsed empty, "
                                  "so this test proved nothing")
        for guard in GUARDS:
            for name in guard.gate.split():
                with self.subTest(f"{guard.id}:{name}"):
                    self.assertTrue(name == "-" or name in declared, name)


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

    def test_every_cited_section_of_a_cited_document_exists(self) -> None:
        """`spec` is the normative citation and the first field to read.

        Checking only that the FILE exists let `section 7a` stand in a plan
        with sections 0 to 10 and decisions 1 to 9, which the S03 review's
        second pass found. A citation nobody can follow is not a citation.
        """
        cited = re.compile(r"`([\w./-]+\.md)`\s+(section|decision)\s+"
                           r"(\d+[a-z]?)")
        found = 0
        for guard in GUARDS:
            for path, kind, number in cited.findall(guard.spec):
                with self.subTest(f"{guard.id}:{path} {kind} {number}"):
                    self.assertTrue((ROOT / path).is_file(), path)
                    text = (ROOT / path).read_text(encoding="utf-8")
                    anchor = (rf"^#{{2,4}} {re.escape(number)}\."
                              if kind == "section"
                              else rf"^\*\*{re.escape(number)}\.")
                    self.assertRegex(text, re.compile(anchor, re.M))
                    found += 1
        self.assertGreater(found, 0, "no spec cites a numbered section, so "
                                     "this test proved nothing")


class CoveredByNamesATestThatOpensTheFile(unittest.TestCase):
    """Check f of the census, asserted here as well as run there.

    `bench.runner` claimed `scripts/tests/test_bench_check.py (the 7 argument
    refusals and the 4 run-time refusals)` and that suite never opens
    `tools/bench/run.mjs`, so nine refusals were counted as watched by a test
    that cannot reach them. Nothing verified the claim, in either place.
    """

    def test_the_real_catalogue_agrees(self) -> None:
        self.assertEqual(census.covered_by_problems(), [])

    def test_a_test_that_does_not_open_the_file_is_refused(self) -> None:
        from guards.catalogue import Guard
        unrelated = Guard(
            id="probe", file="tools/bench/run.mjs", gate="-", spec="none",
            refuses="A sentence long enough to satisfy the well-formed test.",
            claims=("*",),
            covered_by=("scripts/tests/test_corpus_check.py",))
        problems = self._problems_for(unrelated)
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("no test it names opens that file", problems[0])

    # These two used to assert the opposite: that a directory holding a file
    # which reaches the guarded file was an acceptable claim, with
    # `scripts/guards/` as the worked example. The S03 review's fourth pass
    # measured what that example actually proves. `scripts/guards/` holds
    # `catalogue.py`, which writes every guarded path as
    # `file="tools/bench/run.mjs"`, so that directory reaches EVERY entry and
    # so does any directory whose walk contains the catalogue. The reviewer got
    # zero check-f problems from `("scripts/",)`, `("scripts/guards/",)` and
    # `("docs/",)`, and one line moved `bench.runner`'s nine unwatched refusals
    # into the covered bucket. So the route is gone and the test is now that a
    # directory claim is refused, in both the shape that never reached the file
    # and the shape that did.

    def test_a_directory_whose_files_do_not_open_it_is_refused(self) -> None:
        from guards.catalogue import Guard
        hatch = Guard(
            id="probe", file="tools/bench/run.mjs", gate="-", spec="none",
            refuses="A sentence long enough to satisfy the well-formed test.",
            claims=("*",),
            covered_by=("tools/bench/tests/",))
        problems = self._problems_for(hatch)
        # Two: the directory claim itself, and the entry left with no claim
        # that reaches its file, which is what makes it uncovered.
        self.assertEqual(len(problems), 2, problems)
        self.assertIn("covered by the directory tools/bench/tests/",
                      problems[0])
        self.assertIn("no test it names opens that file", problems[1])

    def test_a_directory_holding_the_catalogue_is_refused_too(self) -> None:
        """The measured bypass, asserted where it was asserted the other way.

        `ci/check-device-ownership.sh` is named inside
        `scripts/guards/catalogue.py`, so under the pass 3 rule
        `covered_by=("scripts/guards/",)` was a valid claim of coverage for
        it, and for every other entry, since the catalogue names them all.
        """
        from guards.catalogue import Guard
        real = Guard(
            id="probe", file="ci/check-device-ownership.sh", gate="-",
            spec="none",
            refuses="A sentence long enough to satisfy the well-formed test.",
            claims=("*",),
            covered_by=("scripts/guards/",))
        problems = self._problems_for(real)
        self.assertEqual(len(problems), 2, problems)
        self.assertIn("covered by the directory scripts/guards/", problems[0])
        self.assertIn("no test it names opens that file", problems[1])

    def test_the_catalogue_names_no_directory(self) -> None:
        """Removing the route was honest only because nothing used it.

        If an entry ever needs one, the answer is to name the file rather than
        to put the route back, and this fails at the moment somebody tries.
        """
        from guards.catalogue import GUARDS as REAL
        for guard in REAL:
            for claim in guard.covered_by:
                match = census.COVERED_PATH.search(claim)
                if match is None:
                    continue
                with self.subTest(f"{guard.id}:{match.group(1)}"):
                    self.assertFalse(
                        (ROOT / match.group(1)).is_dir(),
                        f"{guard.id} claims the directory {match.group(1)}")

    def test_a_named_test_that_is_gone_is_refused(self) -> None:
        from guards.catalogue import Guard
        missing = Guard(
            id="probe", file="tools/bench/run.mjs", gate="-", spec="none",
            refuses="A sentence long enough to satisfy the well-formed test.",
            claims=("*",),
            covered_by=("scripts/tests/test_no_such_file.py",))
        problems = self._problems_for(missing)
        self.assertTrue(any("not in this repository" in p for p in problems))

    @staticmethod
    def _problems_for(guard: object) -> list[str]:
        import guards.census as module
        original = module.GUARDS
        module.GUARDS = (guard,)
        try:
            return module.covered_by_problems()
        finally:
            module.GUARDS = original


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

    # Named from the outside and not from `SCAN_EXCLUDE`. The earlier test
    # asserted `not name.startswith(discover.SCAN_EXCLUDE)` using the very
    # constant it was testing, and `str.startswith(())` is always False, so
    # `SCAN_EXCLUDE = ()` left it green with the scanner reading every test
    # suite in the repository.
    TEST_SUITES = (
        "scripts/tests/test_guard_catalogue.py",
        "scripts/tests/test_bench_check.py",
        "tools/oracle/tests/faults.mjs",
        "tools/bench/tests/registry_test.mjs",
    )

    def test_it_does_not_scan_test_suites(self) -> None:
        """A test's own assertions are not guards, and scanning them would
        make every throw in a suite a site needing a catalogue entry."""
        scanned = set(discover.tracked_sources())
        self.assertTrue(scanned)
        for name in self.TEST_SUITES:
            with self.subTest(name):
                self.assertTrue((ROOT / name).is_file(),
                                f"{name} is gone, so this test now names "
                                f"nothing and proves nothing")
                self.assertNotIn(name, scanned)

    def test_a_docstring_shape_table_is_not_a_refusal(self) -> None:
        """Five rows of this module's own shape table were counted as five
        refusals, so deleting the documentation turned the census red."""
        body = ('"""A module.\n\n'
                '| `problems.append(` | Python | a collected problem |\n'
                '| `throw new Error(` | JavaScript | a refusal |\n'
                '"""\n'
                "# print(\"FAIL: this is a comment\")\n"
                "def check(problems):\n"
                "    problems.append('the only real refusal here')\n")
        sites = discover._sites_in("probe.py", body)
        self.assertEqual([s.message for s in sites],
                         ["the only real refusal here"])

    def test_a_coloured_printed_refusal_is_a_site(self) -> None:
        """`guard_probe.py` prints `print(f"{RED}FAIL{OFF}: ...")`, so the
        census that refuses an unclaimed refusal could not see the top-level
        refusal of the file that runs it."""
        sites = discover._sites_in(
            "probe.py", 'print(f"{RED}FAIL{OFF}: the guard harness")\n')
        self.assertEqual([s.shape for s in sites], ["py-print-fail"])

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

    def test_repo_read_refuses_the_write_forms_of_config(self) -> None:
        """`config` is a read or a write depending on its next argument.

        `git config core.hooksPath X` inside REPO_ROOT rewrites the
        developer's clone, and the verb was not in the forbidden list at all
        because the tripwire needs `config --get`.
        """
        for args in (("config", "core.hooksPath", ".githooks"),
                     ("config", "--unset", "core.hooksPath"),
                     ("config", "--global", "user.name", "x"),
                     ("config",)):
            with self.subTest(" ".join(args)):
                with self.assertRaises(sandbox.SandboxError):
                    sandbox.repo_read(*args)

    def test_repo_read_still_allows_the_read_forms_of_config(self) -> None:
        """A tripwire that cannot read `core.hooksPath` watches nothing."""
        self.assertIn("core.", sandbox.repo_read("config", "--list"))


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
        # Two arbitrary strings. They used to be a third copy of another
        # guard's source line, which is the duplication the ratchet exists to
        # remove rather than to spread.
        self.assertNotEqual(census.constant_digest("a strict value"),
                            census.constant_digest("a weaker value"))

    def test_no_declared_constant_swallows_another(self) -> None:
        """`constant_value` compiles with `re.S`, so `(.*)$` is greedy across
        newlines and captures the rest of the file.

        Measured on five patterns in S03: `TOLERANCE`'s recorded value was
        3,426 characters of `pin_and_size_check.py` and its digest moved
        whenever any line below it changed, which is not a ratchet on the
        tolerance at all. A capture that reaches another declared constant is
        the detectable form of that mistake.
        """
        names = {c.name for c in CONSTANTS}
        for constant in CONSTANTS:
            value = census.constant_value(constant, ROOT) or ""
            for other in sorted(names - {constant.name}):
                with self.subTest(f"{constant.name} < {other}"):
                    self.assertNotIn(
                        f"{other} = ", value,
                        f"{constant.file}:{constant.name} captured the "
                        f"declaration of {other}, so its recorded digest "
                        f"moves when unrelated lines change and stands still "
                        f"for nothing")

    def test_no_declared_constant_captures_greedily(self) -> None:
        """The shape of the same mistake, asserted where it is written.

        The test above catches a greedy capture only when it happens to reach
        another declared constant, and `TOLERANCE` was the last one in its
        file, so it swallowed 3,426 characters and no value test could see it.
        `(.*)` under `re.S` is the defect itself, so it is refused here.
        """
        for constant in CONSTANTS:
            with self.subTest(f"{constant.file}:{constant.name}"):
                self.assertNotIn(
                    "(.*)", constant.pattern,
                    f"{constant.name}'s pattern captures greedily and "
                    f"census.constant_value compiles with re.S, so `.` "
                    f"matches newlines and the capture runs to the last line "
                    f"that can close it. Write `(.*?)` or a bounded class.")

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
