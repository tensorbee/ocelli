#!/usr/bin/env python3
"""Drive every declared guard red, in a disposable repository, and prove it.

This is the mechanism `docs/hld/24-agent-code-standards.md` section 27.3's
third bullet describes, made standing for the one class where it can be
scripted:

    "That a new test would actually fail if the code were wrong. Mutate one
     constant, re-run, confirm it goes red."

`docs/sprints/allocation.json` says why it has to be standing rather than a
procedure somebody runs: round 12 of F-010's review "mutated its guards in the
same command that added them, which proves the guard fired once and leaves
nothing watching it afterwards".

## The two things that make a green run here mean something

**Inverted success.** A probe whose guard exits 0 is a FAILURE OF THE HARNESS,
not a pass. A probe builder that silently stops mutating anything therefore
turns this red rather than green. The message is the one
`tools/oracle/tests/faults.mjs` already uses.

**The mandatory control.** Every distinct CONTROL is run against the UNMUTATED
sandbox and must exit with the status its probe declares, which is 0 for all
but two of them. `split_hld` and `corpus-tests` have no healthy state a sandbox
can build, so their control is the DIFFERENT refusal a healthy repository
gives, declared as `control_status` and `control_expect`, and this sentence
said "every invoke must exit 0" while the code beside it read
`probe.control_status`. It is the thing whose absence made F-010's
round 12 worthless: its harness was broken, so every "all refusals red" result
had a red baseline and proved nothing. The control does two more jobs. It
proves the sandbox is a faithful copy, because a guard that refuses an
unmutated copy means the copy is wrong. And it is the runbook's own sentence,
that a guard which fails on everything is as useless as one that fails on
nothing.

Usage:
  python3 scripts/guard_probe.py --profile floor
  python3 scripts/guard_probe.py --profile deep
  python3 scripts/guard_probe.py --only content.dicom-magic
  python3 scripts/guard_probe.py --list          # every probe, both profiles
  python3 scripts/guard_probe.py --self-test

A RUN defaults to `--profile floor`, which is the set CI's `guards` gate is
about. A LIST defaults to every probe, because an inventory that omits a third
of them is not an inventory, and four files send a reader here to check
something about the deep set.
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import time
from pathlib import Path

# Before the first import of `guards.*`, and that ordering is the point.
# `scrubbed_env` sets PYTHONDONTWRITEBYTECODE for CHILDREN. The parent is the
# process that imports these modules and builds every probe's rejected state
# from them, so without this it wrote `scripts/guards/__pycache__` inside the
# developer's repository on every run. `.gitignore` covers it, so the tripwire
# could not see it either, and a stale `.pyc` is capable of making the harness
# build a rejected state that does not match its source.
sys.dont_write_bytecode = True

sys.path.insert(0, str(Path(__file__).resolve().parent))

from guards import sandbox as sb  # noqa: E402
from guards.catalogue import DEFECTS, GUARDS, Probe  # noqa: E402
from guards.census import (BUDGET, BUDGET_NOTE, ROOT,  # noqa: E402
                           load_budget)

RED = "\033[31m"
GREEN = "\033[32m"
DIM = "\033[2m"
OFF = "\033[0m"


def selected(profile: str, only: list[str]) -> list[tuple[str, Probe]]:
    pairs = [(g.id, p) for g in GUARDS for p in g.probes]
    if only:
        known = {p.id for _, p in pairs}
        unknown = [name for name in only if name not in known]
        if unknown:
            raise SystemExit(
                f"FAIL: --only names {', '.join(unknown)}, which no catalogue "
                f"entry declares. A filter that selects nothing would leave "
                f"the problem list empty and report every probe caught having "
                f"caught none, which is the shape this whole file exists to "
                f"refuse.")
        return [(gid, p) for gid, p in pairs if p.id in only]
    if profile == "deep":
        return pairs
    return [(gid, p) for gid, p in pairs if p.profile == "floor"]


def combined(done: "subprocess.CompletedProcess[str]") -> str:
    return f"{done.stdout or ''}{done.stderr or ''}"


def run_controls(box: sb.Sandbox, probes: list[tuple[str, Probe]],
                 problems: list[str]) -> dict[str, bool]:
    """One control per distinct invoke, on the unmutated sandbox.

    Mandatory, and it is the thing whose absence made F-010's round 12
    worthless: its harness was broken, so every earlier all-refusals-red result
    had a red baseline and proved nothing.
    """
    seen: dict[str, bool] = {}
    for _, probe in probes:
        control = probe.control or probe.invoke
        key = f"{control.key}|{probe.control_status}|{probe.control_expect}"
        if key in seen:
            continue
        box.reset()
        _prepare_control(box, probe)
        done = control.run(box)
        output = combined(done)
        detail = None
        if done.returncode != probe.control_status:
            detail = (f"exited {done.returncode} and a healthy repository "
                      f"exits {probe.control_status}")
        elif probe.control_expect and probe.control_expect not in output:
            detail = (f"exited {done.returncode} as declared and nothing in "
                      f"its output contains {probe.control_expect!r}")
        elif probe.control_status != 0 and probe.expect \
                and probe.expect in output:
            detail = (f"refused for the SAME reason as the probe. The control "
                      f"already contains {probe.expect!r}, so the probe proves "
                      f"nothing about what the guard discriminates")
        seen[key] = detail is None
        if detail is not None:
            problems.append(
                f"control `{control.key}` {detail} on an UNMUTATED sandbox. "
                f"Either the sandbox is not a faithful copy of the "
                f"repository, or the guard fails on everything, and a guard "
                f"that fails on everything is as useless as one that fails on "
                f"nothing. Nothing that uses this control proves anything "
                f"until it is green.\n"
                f"      {output.strip()[:400]}")
    box.reset()
    return seen


def _prepare_control(box: sb.Sandbox, probe: Probe) -> None:
    """The minimum state an invoke needs to be green when nothing is broken.

    Most invokes with a branch here refuse a freshly built sandbox, because
    what a healthy repository has at that point is produced by an earlier step
    rather than tracked: hooks enabled, a benign change staged, a verify-ledger
    record keyed on the tree the index holds, a built wasm artefact, a present
    corpus. Each branch builds exactly that and no more. Recorded here rather
    than hidden inside each probe, because a control that quietly does the
    probe's job is the false green this file exists to refuse. An earlier
    version of this sentence said "three invokes" and the branch list has never
    been three.

    One branch deliberately builds nothing. `split_hld` has no healthy state in
    a repository with no private source, so its control is the DIFFERENT
    refusal it gives with no source configured at all, declared through
    `control_status` and `control_expect` rather than constructed here.
    `corpus-tests` is the other probe whose control is a refusal, and its
    invoke needs nothing prepared, so it has no branch.
    """
    def write_green_comparison_report() -> None:
        box.write(".claude/probe-comparison.json", json.dumps({
            "operation": "gate",
            "pass": 1,
            "fail": 0,
            "claimedVerdictViews": 1,
            "gateVerdict": "pass",
            "green": True,
            "coverage": {
                "unmeasured": 0,
                "absent": 0,
                "unsupportedSourceRows": 0,
                "declaredVolumeRefusals": 0,
            },
            "unmeasured": 0,
            "absent": 0,
            "problems": [],
            "coverageProblems": [],
            "absorbedDivergences": [],
        }) + "\n")

    key = (probe.control or probe.invoke).key
    if key.startswith("git commit:"):
        # A healthy commit: hooks enabled and one benign change staged. The
        # ONLY difference between this and the probe is the thing under probe,
        # which for the pre-commit hook is the staged content and for the
        # commit-msg hook is the message.
        box.enable_hooks()
        box.write("probe-note.txt", "a benign change\n")
        box.stage_all()
    elif key == "git push, hooks enabled":
        # A healthy repository pushes a head that carries its trailer, and the
        # trailer is written by the commit-msg hook from a ledger entry a real
        # gate run produced. So the control has to produce one the same way,
        # and it is not forgeable: verify_ledger.py trailer emits nothing
        # without a record for that exact tree.
        box.enable_hooks()
        box.write("probe-note.txt", "a benign change\n")
        box.stage_all()
        box.run(["python3", "scripts/verify_ledger.py", "record",
                 "--gates", "fmt", "--corpus", "pass", "--profile", "sprint"])
        box.git("commit", "-m", "F-000, a probe")
    elif key.startswith("python3 scripts/verify_ledger.py record") \
            and "--comparison-report" in key:
        write_green_comparison_report()
    elif key.startswith("python3 scripts/verify_ledger.py assert"):
        argv = ["python3", "scripts/verify_ledger.py", "record",
                "--gates", "fmt", "--corpus", "pass", "--profile", "sprint"]
        if "--require-comparison" in key:
            write_green_comparison_report()
            argv.extend(["--comparison-report", ".claude/probe-comparison.json"])
        box.run(argv)
    elif key.startswith("python3 scripts/verify_ledger.py check-commit HEAD"):
        # The record is taken AFTER staging, because it is keyed on the tree
        # the index holds. Recording first and staging afterwards produces the
        # very tree mismatch this control has to be free of.
        box.enable_hooks()
        box.write("probe-note.txt", "a benign change\n")
        box.stage_all()
        argv = ["python3", "scripts/verify_ledger.py", "record",
                "--gates", "fmt", "--corpus", "pass", "--profile", "sprint"]
        if "--require-comparison" in key:
            write_green_comparison_report()
            argv.extend(["--comparison-report", ".claude/probe-comparison.json"])
        box.run(argv)
        box.git("commit", "-m", "F-000, a probe")
    elif key.endswith("--commit-msg probe-message.txt"):
        box.write("probe-message.txt", "F-000, a probe\n\nOne clause here.\n")
    elif key == "verify_ledger trailer":
        box.run(["python3", "scripts/verify_ledger.py", "record",
                 "--gates", "fmt", "--corpus", "pass", "--profile", "sprint"])
    elif key == "sprint_workflow validate-handoff":
        from guards.catalogue import sprint_state
        fid = sprint_state(box)
        box.write(".claude/probe-fid", fid)
        box.write(f".claude/handoffs/{fid}-ready.md",
                  f"# {fid} ready\n\n"
                  f"**Branch**: work/{fid.lower()}-agent\n"
                  f"**Base**: sprint/s03\n**Head**: 0123456789ab\n"
                  f"**Files touched**: scripts/probe.py\n"
                  f"**Review**: pass 1\n**Verify tree**: 0123456789ab\n")
    elif key == "bin/ocelli.sh compare":
        box.write("tools/oracle/out/run.json", "{}\n")
    elif key == "python3 scripts/pin_and_size_check.py --with-size":
        box.write("crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm", b"\x00" * 1000)
        for name in ("LICENSE-MIT", "LICENSE-APACHE"):
            box.write(f"crates/ocelli-wasm/pkg/{name}",
                      (box.path / name).read_bytes())
    elif key == "python3 scripts/corpus_check.py":
        # A healthy corpus is present and matches. The real one is not in git
        # and is absent in CI, so the control builds the smallest one that is.
        import hashlib
        body = b"probe case zero\n"
        header = box.read("corpus/manifest.tsv").splitlines()[0]
        box.write("corpus/manifest.tsv", header + "\n" + "\t".join([
            "probe/case0.dcm", "CT", "1.2.840.10008.1.2.1", "synthetic, probe",
            "Ocelli guard harness", "MIT", "https://example.invalid/licence",
            hashlib.sha256(body).hexdigest(), ""]) + "\n")
        box.write("corpus/data/probe/case0.dcm", body)
    elif key.startswith("split_hld"):
        # There is no healthy state for this one in a repository with no
        # private source, so its control is the refusal it always gives with
        # no source configured at all, which is a DIFFERENT refusal.
        pass


def run_probe(box: sb.Sandbox, guard_id: str, probe: Probe,
              problems: list[str], notes: list[str]) -> str:
    box.reset()
    try:
        if probe.mutate is not None:
            probe.mutate(box)
    except Exception as error:  # noqa: BLE001, a builder failure is a result
        problems.append(
            f"probe {probe.id} ({guard_id}) could not build its rejected "
            f"state: {type(error).__name__}: {error}. A probe that mutated "
            f"nothing would have left the guard green for the wrong reason.")
        return "error"

    done = probe.invoke.run(box)
    output = combined(done)
    wanted_refusal = probe.polarity == "refuse"
    refused = done.returncode != 0

    if wanted_refusal and not refused:
        message = (
            f"probe {probe.id} ({guard_id}) exited 0. The guard it aims at "
            f"did not fire, so it is a guard nobody has watched fail.")
        if probe.defect:
            notes.append(f"{probe.defect}  {message} DECLARED: "
                         f"{DEFECTS.get(probe.defect, '')}")
            return "known-defect"
        problems.append(message)
        return "fail"

    if not wanted_refusal and refused:
        message = (
            f"probe {probe.id} ({guard_id}) expected the guard to ACCEPT and "
            f"it exited {done.returncode}. A guard that refuses a legitimate "
            f"state is as useless as one that refuses nothing.\n"
            f"      {output.strip()[:400]}")
        if probe.defect:
            notes.append(f"{probe.defect}  {message.splitlines()[0]} "
                         f"DECLARED: {DEFECTS.get(probe.defect, '')}")
            return "known-defect"
        problems.append(message)
        return "fail"

    if probe.expect == "":
        if output.strip():
            problems.append(
                f"probe {probe.id} ({guard_id}) declares a SILENT refusal and "
                f"the run printed {output.strip()[:120]!r}. The absence of "
                f"output is the mechanism here.")
            return "fail"
    elif probe.expect not in output:
        problems.append(
            f"probe {probe.id} ({guard_id}) got the exit status it wanted, "
            f"but not at the declared refusal: nothing in the output contains "
            f"{probe.expect!r}. A run that went the right way for another "
            f"reason proves nothing about this guard.\n"
            f"      {output.strip()[:400]}")
        return "fail"

    if probe.defect:
        problems.append(
            f"probe {probe.id} ({guard_id}) is declared as known defect "
            f"{probe.defect} and it PASSED. The hole was fixed and the "
            f"declaration is now a lie. Delete the `defect` field from the "
            f"catalogue entry and remove {probe.defect} from DEFECTS.")
        return "fail"
    return "pass"


# ---------------------------------------------------------------------------
# The harness's own self test
# ---------------------------------------------------------------------------

SELF_TEST_PROPERTIES = (
    "the choke point refuses a cwd inside the real repository",
    "the choke point refuses a directory this harness did not create",
    "the banned `rm --cached` pair is refused by name",
    "the environment scrub removes an inherited GIT_DIR",
    "a probe builder that mutates nothing is a named failure",
    "the sandbox is a faithful copy of the repository",
    "the tripwire sees a planted change to each thing it captures",
    "an unknown `--only` id refuses rather than selecting nothing",
    "a run in which zero probes executed refuses",
    "every declared defect is claimed by a probe, and the other way",
)


def self_test() -> int:
    """The refusals in this harness only ever run on a mismatch.

    No gate run produces one, which is the same reason
    `tools/oracle/check_sidecars.py` carries a self test.

    **The printed count is derived from what actually ran.** It was a
    hardcoded "(10 properties)", so deleting properties 9 and 10 still printed
    ten and exited 0, and `probe-runner.self-test` is an accept probe that
    cannot see a number. `SELF_TEST_PROPERTIES` above names each one and a
    property whose block is gone fails here rather than shrinking the self
    test quietly.
    """
    problems: list[str] = []
    reached: set[int] = set()

    def check(label: str, condition: bool, detail: str = "") -> None:
        if not condition:
            problems.append(f"{label}: {detail}")

    # 1. The choke point refuses a cwd inside the real repository.
    reached.add(1)
    inside = sb.Sandbox(path=ROOT / "scripts")
    try:
        inside.git("status")
        problems.append("the git choke point ran inside the real repository")
    except sb.SandboxError as error:
        check("choke point message", "inside or above" in str(error)
              or "not a directory this harness created" in str(error),
              str(error))

    # 2. A directory this harness did not create is refused.
    reached.add(2)
    with tempfile.TemporaryDirectory() as other:
        elsewhere = sb.Sandbox(path=Path(other))
        try:
            elsewhere.git("status")
            problems.append("the choke point ran in a directory it did not "
                            "create")
        except sb.SandboxError as error:
            check("foreign directory message",
                  "this harness created" in str(error), str(error))

    # 3. The banned pair is refused by name.
    reached.add(3)
    with sb.sandbox() as box:
        try:
            box.git("rm", "--cached", "README.md")
            problems.append("`git rm --cached` was not refused")
        except sb.SandboxError as error:
            check("banned pair message", "false green" in str(error),
                  str(error))

        # 4. The environment scrub removes an inherited GIT_DIR.
        reached.add(4)
        os.environ["GIT_DIR"] = str(ROOT / ".git")
        try:
            env = sb.scrubbed_env()
            check("GIT_DIR scrubbed", "GIT_DIR" not in env,
                  "an inherited GIT_DIR survived into the child environment, "
                  "so a probe's git call could have written to the "
                  "developer's repository")
            check("global config neutralised",
                  env.get("GIT_CONFIG_GLOBAL") == os.devnull, "not /dev/null")
        finally:
            del os.environ["GIT_DIR"]

        # 5. A probe builder that mutates nothing is a named failure.
        reached.add(5)
        try:
            box.substitute("README.md", "a string that is not in the file",
                           "x")
            problems.append("substitute() accepted a no-op edit")
        except sb.SandboxError as error:
            check("no-op edit message", "mutated nothing" in str(error),
                  str(error))

        # 6. The sandbox is a faithful copy, by name and not by a threshold.
        # This was `copied > 100` over `box.path.rglob("*")`, and that walk
        # counts the sandbox's own `.git`, which is several hundred entries on
        # its own. Measured in the S03 review's fourth pass: a sandbox with
        # EVERY tracked file deleted still walked 948 entries and passed. The
        # property is named for faithfulness, so it compares against the
        # tracked set that `build` copies from.
        reached.add(6)
        tracked = sb.repo_tracked_paths()
        missing = [name for name in tracked if not (box.path / name).is_file()]
        check("sandbox is a faithful copy", bool(tracked) and not missing,
              f"{len(tracked)} path(s) are tracked and {len(missing)} of them "
              f"are not regular files in the sandbox, the first few being "
              f"{missing[:5]}")

    # 7. The tripwire detects a planted change to each thing it captures.
    reached.add(7)
    before = sb.tripwire_capture()
    for label in dict(sb.TRIPWIRE_READS):
        planted = dict(before)
        planted[label] = "something else"
        changed = sb.tripwire_compare(before, planted)
        check(f"tripwire sees {label}", len(changed) == 1,
              f"planting a change in {label} produced {len(changed)} reports")
    check("tripwire is quiet when nothing moved",
          sb.tripwire_compare(before, before) == [], "it fired on itself")

    # 8. An unknown --only id refuses rather than selecting nothing.
    reached.add(8)
    try:
        selected("floor", ["no-such-probe"])
        problems.append("--only accepted an id no entry declares")
    except SystemExit as error:
        check("unknown --only message", "which no catalogue entry declares"
              in str(error), str(error))

    # 9. A run in which zero probes executed refuses.
    reached.add(9)
    check("zero probes refuses", _zero_is_not_a_pass(0) is not None,
          "a run over an empty selection reported a pass")

    # 10. Every declared defect is claimed by a probe, and the other way.
    reached.add(10)
    declared = {p.defect for g in GUARDS for p in g.probes if p.defect}
    check("defects are claimed", declared == set(DEFECTS),
          f"catalogue probes name {sorted(declared)}, DEFECTS names "
          f"{sorted(DEFECTS)}")

    absent = [f"{n}. {SELF_TEST_PROPERTIES[n - 1]}"
              for n in range(1, len(SELF_TEST_PROPERTIES) + 1)
              if n not in reached]
    if absent:
        problems.append(
            "the self test declares " + str(len(SELF_TEST_PROPERTIES)) +
            " properties and these never ran: " + "; ".join(absent) +
            ". A property whose block was deleted leaves the printed count "
            "standing for work nothing did.")

    if problems:
        print("FAIL: the guard harness's own self test")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: the guard harness refuses what it says it refuses "
          f"({len(reached)} properties)")
    return 0


def _zero_is_not_a_pass(count: int) -> str | None:
    if count == 0:
        return ("zero probes ran, which is not a pass. A selection that "
                "matched nothing would otherwise report every probe caught "
                "having caught none.")
    return None


# ---------------------------------------------------------------------------


def main() -> int:
    parser = argparse.ArgumentParser()
    # `--profile` has no argparse default since the S03 review's tenth pass,
    # and the reason is `--list`. RUNNING defaults to the floor, because that
    # is the set CI's `guards` gate is about. LISTING is an inventory, and an
    # inventory that silently omits a third of the probes is the defect: bare
    # `--list` printed the floor set alone, every row marked `floor`, while
    # `.github/workflows/ci.yml`, `bin/ocelli.sh`, `docs/lld/guards.md` and
    # `scripts/guards/catalogue.py` all sent a reader here to check something
    # about the DEEP probes, which none of those rows was. The counts are not
    # written here, and the tenth pass wrote two of them in the commit that
    # deleted a third stale count from two other files under the banner that a
    # number beside a list goes stale the next time the list grows. It did:
    # they read 112 and 50 and the sets were 114 and 54 within a sprint. Run
    # the command. That is the same
    # shape as the fifth pass's `needs` finding one field over: a pointer to a
    # command that does not answer is worse than the sentence it replaced. So
    # `--list` alone is every probe, and `--list --profile floor` is still the
    # floor set for anyone who wants it.
    parser.add_argument("--profile", default=None,
                        choices=["floor", "deep"])
    parser.add_argument("--only", action="append", default=[])
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--record-budget", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        return self_test()

    profile = args.profile or ("deep" if args.list else "floor")
    probes = selected(profile, args.only)

    if args.list:
        for guard_id, probe in probes:
            defect = f"  KNOWN DEFECT {probe.defect}" if probe.defect else ""
            # `needs` is printed since the S03 review's fifth pass. Two files
            # sent a reader here for it, `.github/workflows/ci.yml`'s
            # guards-deep comment and `docs/lld/guards.md`, both saying to read
            # the command rather than the sentence, and the command did not
            # answer. A pointer to a command that does not carry the field is
            # worse than the sentence it replaced, because the sentence at
            # least said something.
            print(f"{probe.profile:5} L{probe.level} {probe.polarity:6} "
                  f"needs={probe.needs:9} {probe.id:38} {guard_id}{defect}")
            # `note` is where the R2 rationale for a probe's INPUT is written,
            # and it reached no output at all until the S03 review's second
            # pass counted it set on half the catalogue and read by nothing.
            if probe.note:
                print(f"{DIM}      {probe.note}{OFF}")
        refusals = [p for _, p in probes if p.polarity == "refuse"]
        guards = {gid for gid, p in probes if p.polarity == "refuse"}
        print(f"{len(probes)} probe(s): {len(refusals)} that must drive a "
              f"guard red, over {len(guards)} guard(s), and "
              f"{len(probes) - len(refusals)} that must be accepted")
        return 0

    empty = _zero_is_not_a_pass(len(probes))
    if empty:
        print(f"FAIL: {empty}")
        return 1

    problems: list[str] = []
    notes: list[str] = []
    counts = {"pass": 0, "fail": 0, "known-defect": 0, "error": 0}
    # Counted separately because they are different quantities and the summary
    # line reported one of them under the other's name until the S03 review's
    # second pass. `counts["pass"]` is a PROBE count and includes the accept
    # probes, which were never red, so it was never the number of guards
    # observed red.
    drove_red: set[str] = set()
    accepted = 0

    before = sb.tripwire_capture()
    started = time.monotonic()
    with sb.sandbox() as box:
        controls = run_controls(box, probes, problems)
        for guard_id, probe in probes:
            control = probe.control or probe.invoke
            key = f"{control.key}|{probe.control_status}|{probe.control_expect}"
            if not controls.get(key, True):
                continue
            outcome = run_probe(box, guard_id, probe, problems, notes)
            counts[outcome] += 1
            if outcome == "pass":
                if probe.polarity == "refuse":
                    drove_red.add(guard_id)
                else:
                    accepted += 1
            # `red` for a refusal probe that passed, `green` for an accept one.
            # It printed `red` for both until the S03 review's ninth pass, so
            # the sixteen accept probes announced the guard going red when what
            # they proved was the guard exiting 0. In a harness whose stated
            # premise is that a wrong result read as success voids a proof, the
            # per-probe instrument read wrong.
            passed = (f"{GREEN}red{OFF}" if probe.polarity == "refuse"
                      else f"{GREEN}green{OFF}")
            mark = {"pass": passed, "fail": f"{RED}HARNESS{OFF}",
                    "known-defect": f"{RED}defect{OFF}",
                    "error": f"{RED}error{OFF}"}[outcome]
            print(f"  {mark:18} {probe.id}")
    elapsed = time.monotonic() - started

    after = sb.tripwire_capture()
    moved = sb.tripwire_compare(before, after)
    if moved:
        problems.append(
            "the harness changed the developer's repository while it ran. "
            "Nothing here may write inside it, and that is a guarantee made "
            "by construction rather than by cleanup, so this is a defect in "
            "the harness and not in a guard:\n      " +
            "\n      ".join(moved))

    # `profile` and not `args.profile`, which is `None` when the flag is
    # absent. A run defaults to the floor, so this is the profile that ran.
    problems += _budget_problems(profile, elapsed, args.record_budget)

    print()
    for note in notes:
        print(f"{DIM}KNOWN DEFECT{OFF}  {note}")
    if problems:
        print(f"{RED}FAIL{OFF}: the guard harness")
        for problem in problems:
            print(f"  {problem}")
        return 1
    # `controls` is keyed exactly as `run_controls` dedupes, on the control's
    # own invoke and its declared status and fragment. Counting distinct
    # `probe.invoke.key` instead reported the wrong number in both profiles,
    # because eleven probes declare a control that is not their invoke.
    print(f"{GREEN}OK{OFF}: {counts['pass'] - accepted} refusal probe(s) drove "
          f"{len(drove_red)} guard(s) red for their declared reason, "
          f"{accepted} accept probe(s) green, {counts['known-defect']} known "
          f"defect(s) still open, {len(controls)} control(s) green, "
          f"{elapsed:.1f}s")
    return 0


def _budget_problems(profile: str, elapsed: float, record: bool) -> list[str]:
    """A recorded measurement, not a guess.

    `scripts/pin_and_size_check.py` gives the argument for the size budget and
    it holds here: a budget invented before the first measurement would be
    either meaningless or immediately wrong.
    """
    budget = load_budget()
    timings = budget.setdefault("wall_clock_seconds", {})
    if record:
        timings[profile] = round(elapsed, 1)
        # The note is `scripts/guards/census.py`'s BUDGET_NOTE, declared
        # once. It was written out in full here and consulted nowhere else,
        # so a key added to the budget left this sentence describing the
        # previous set. Written UNCONDITIONALLY, because `setdefault` is the
        # exact mechanism the sentence above names as the reason the note
        # could never be refreshed, and leaving it here read as the bug it
        # documents even while `guard_census.py --record` happened to keep the
        # value current.
        budget["note"] = BUDGET_NOTE
        BUDGET.write_text(json.dumps(budget, indent=2, sort_keys=True) + "\n")
        print(f"  {profile} wall clock recorded at {elapsed:.1f}s")
        return []
    if profile not in timings:
        # Never written outside --record-budget. A gate run that quietly
        # rewrote a tracked file would leave the developer's tree dirty and
        # would make the number a record of the last machine to run rather
        # than a baseline anybody agreed to.
        print(f"  {profile} wall clock {elapsed:.1f}s, no recorded baseline. "
              f"Record one with --record-budget.")
        return []
    recorded = timings[profile]
    # Generous, deliberately. This catches a change that made the harness ten
    # times slower and not a shared CI runner having a bad afternoon, because
    # a timing gate tuned tight is a timing gate that gets disabled.
    ceiling = recorded * 5 + 30
    if elapsed > ceiling:
        return [f"the {profile} profile took {elapsed:.1f}s against a "
                f"recorded {recorded}s and a ceiling of {ceiling:.1f}s. "
                f"Re-baseline deliberately with --record-budget and say why."]
    return []


if __name__ == "__main__":
    sys.exit(main())
