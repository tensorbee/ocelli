#!/usr/bin/env python3
"""Run the corpus tooling test suites, and refuse to call a skip a pass.

`bin/ocelli.sh gate test` is `cargo test --workspace`, which is Rust only.
Without this runner the tests under `scripts/tests/` execute when somebody
remembers the command, and are counted as coverage forever whether they ran or
not. That is the defect class a green suite cannot report on.

No count appears in this file on purpose. The runner prints `ran=` per suite on
every invocation, so the number is available from the thing that produces it,
and a number written down here would be one more place to go stale.

## Why this is not a two-line shell arm

Two things make it worth a script.

**A skipped test must fail this gate.** Run the suites under an interpreter
with no pydicom and `test_corpus_synth` reports as a single skip, the process
exits 0, and a shell arm reading `$?` reports the gate green while the
hand-computed PS3.3 fixture, the determinism proof and every conformance
assertion did not run. So this reads the skip COUNT out of the result object,
not the exit status.

**The interpreter is a property of the machine, not of the project.** The
generator needs pydicom, numpy and the codec plugins, and the checkout cannot
know where that lives. Resolved the way `scripts/source_dir.py` resolves the
private source documents, and for the same reason: an exported variable has to
be remembered every session, and forgetting it should not quietly change what
ran.

## The one skip that is allowed, where it is allowed, and where it is not

A prerequisite that is genuinely absent (no interpreter with pydicom, no
DCMTK, no OpenJPH) makes the generator suite exit 3, which `bin/ocelli.sh`
counts and names as SKIPPED and never as a pass. That is a different thing
from a test inside a suite we did decide to run reporting itself skipped,
which is a failure here.

**`--require-prerequisites` turns that skip into a failure**, and CI passes it.
The reason is that `gates_cmd` in `bin/ocelli.sh` returns 0 when a gate skips,
which is right for `docs` and `wasm` whose skips are permanent and expected,
and wrong for a CI job whose earlier steps exist precisely to install these
prerequisites. Without the flag, an OpenJPH build that installed outside PATH
would give that job a green tick having run only the stdlib suite. A developer
running `bin/ocelli.sh gate --floor` on a machine with no DCMTK still gets the
skip, which is the behaviour they want.

`scripts/tests/test_corpus_check.py` needs nothing but the standard library,
so it always runs and can always fail. Only the generator suite has
prerequisites to be absent.

Usage:
  python3 scripts/corpus_tests.py                 # run both suites
  python3 scripts/corpus_tests.py --which         # show the resolved interpreter
  python3 scripts/corpus_tests.py --set PATH      # record it for this clone
"""

from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TESTS = ROOT / "scripts" / "tests"
CONFIG = ROOT / ".ocelli-python-path"
ENV_VAR = "OCELLI_PYTHON"

# A sibling of the checkout, so the default is machine-neutral. `ocelli-tools`
# sits beside the repository because a virtualenv full of DICOM codecs is not
# a thing to commit. Both the canonical checkout and a linked worktree are
# covered, since a worktree lives one level deeper.
FALLBACKS = (
    ROOT.parent / "ocelli-tools" / "venv" / "bin" / "python",
    ROOT.parent.parent / "ocelli-tools" / "venv" / "bin" / "python",
)

# What the generator suite needs beyond the standard library.
REQUIRED_MODULES = ("pydicom", "numpy")
REQUIRED_TOOLS = {
    "dcmcjpeg": "DCMTK, for the four JPEG syntaxes pydicom cannot encode",
    "ojph_compress": "OpenJPH, for the three HTJ2K syntaxes",
}

STDLIB_SUITE = "test_corpus_check.py"
TOOLED_SUITE = "test_corpus_synth.py"

SUMMARY = "OCELLI-SUITE"
SKIP_LINE = "OCELLI-SKIP"

SKIPPED_EXIT = 3


# ---------------------------------------------------------------------------
# Resolving an interpreter that can actually import what the suite needs
# ---------------------------------------------------------------------------

def has_modules(interpreter: Path) -> bool:
    """Ask the candidate itself. A path that looks right and cannot import
    pydicom is not an answer, and checking by name would accept it."""
    program = "import " + ", ".join(REQUIRED_MODULES)
    try:
        return subprocess.run([str(interpreter), "-c", program],
                              capture_output=True, timeout=60).returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


def candidates() -> list[tuple[Path, str, bool]]:
    """(path, where it came from, was it asked for by name).

    An explicitly configured interpreter is authoritative. Falling through
    from a typo in $OCELLI_PYTHON to a working default would run a DIFFERENT
    interpreter from the one asked for and report success, which is the same
    class of quiet as the one this whole runner exists to close.
    """
    found: list[tuple[Path, str, bool]] = []
    override = os.environ.get(ENV_VAR)
    if override:
        found.append((Path(override).expanduser(), ENV_VAR, True))
    if CONFIG.exists():
        recorded = CONFIG.read_text(encoding="utf-8").strip()
        if recorded:
            found.append((Path(recorded).expanduser(), CONFIG.name, True))
    # The interpreter running this script, which is the whole answer in CI
    # where the packages are installed into the runner's own Python.
    found.append((Path(sys.executable), "the running interpreter", False))
    found += [(path, "default", False) for path in FALLBACKS]
    return found


def resolve() -> tuple[Path | None, str]:
    """Return (interpreter, where it came from), or (None, why not)."""
    tried = []
    for path, origin, explicit in candidates():
        if not path.exists():
            reason = f"{path} ({origin}): not present"
        elif not has_modules(path):
            reason = (f"{path} ({origin}): cannot import "
                      f"{' and '.join(REQUIRED_MODULES)}")
        else:
            return path, origin
        tried.append(reason)
        if explicit:
            tried.append(f"{origin} was set explicitly, so no fallback was "
                         f"tried. Fix it or unset it.")
            break
    return None, "\n".join(f"    {line}" for line in tried)


def missing_tools() -> list[str]:
    return [f"{name} ({why})" for name, why in REQUIRED_TOOLS.items()
            if shutil.which(name) is None]


# ---------------------------------------------------------------------------
# Running one suite, and reporting counts rather than an exit status
# ---------------------------------------------------------------------------

def run_suite_here(pattern: str) -> int:
    """Executed by the RESOLVED interpreter, via --run-suite. Prints counts."""
    loader = unittest.TestLoader()
    suite = loader.discover(start_dir=str(TESTS), pattern=pattern,
                            top_level_dir=str(TESTS))
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    for test, reason in result.skipped:
        print(f"{SKIP_LINE} {test} :: {reason}")
    print(f"{SUMMARY} {pattern} ran={result.testsRun} "
          f"failures={len(result.failures)} errors={len(result.errors)} "
          f"skipped={len(result.skipped)}")
    return 0


def counts_from(output: str, pattern: str) -> dict[str, int] | None:
    for line in output.splitlines():
        if line.startswith(f"{SUMMARY} {pattern} "):
            fields = line.split()[2:]
            return {key: int(value) for key, value in
                    (field.split("=", 1) for field in fields)}
    return None


def run_suite(interpreter: Path, pattern: str) -> tuple[bool, str]:
    """Run one suite under `interpreter`. Returns (ok, one-line verdict)."""
    result = subprocess.run(
        [str(interpreter), str(Path(__file__).resolve()),
         "--run-suite", pattern],
        capture_output=True, text=True)
    sys.stdout.write(result.stdout)
    sys.stderr.write(result.stderr)

    counts = counts_from(result.stdout, pattern)
    if counts is None:
        return False, (f"{pattern}: the suite did not report a summary, so "
                       f"nothing here knows what ran. Exit status was "
                       f"{result.returncode}.")
    if counts["ran"] == 0:
        return False, f"{pattern}: zero tests ran, which is not a pass."
    if counts["skipped"]:
        return False, (
            f"{pattern}: {counts['skipped']} test(s) SKIPPED. A skipped test "
            f"is not a passed test, and the lines above name each one. If a "
            f"prerequisite is missing, install it rather than accepting the "
            f"skip.")
    if counts["failures"] or counts["errors"]:
        return False, (f"{pattern}: {counts['failures']} failure(s), "
                       f"{counts['errors']} error(s).")
    return True, f"{pattern}: {counts['ran']} passed."


# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--set", metavar="PATH",
                        help="record the interpreter for this clone")
    parser.add_argument("--which", action="store_true",
                        help="print the resolved interpreter and its origin")
    parser.add_argument("--require-prerequisites", action="store_true",
                        help="treat an absent prerequisite as a failure rather "
                             "than a skip. For a caller that installed them.")
    parser.add_argument("--run-suite", metavar="PATTERN",
                        help=argparse.SUPPRESS)
    args = parser.parse_args()

    if args.run_suite:
        return run_suite_here(args.run_suite)

    if args.set:
        target = Path(args.set).expanduser()
        if not target.exists():
            print(f"FAIL: {target} does not exist")
            return 1
        if not has_modules(target):
            print(f"FAIL: {target} cannot import "
                  f"{' and '.join(REQUIRED_MODULES)}")
            print("Point --set at the interpreter of the environment that has")
            print("the DICOM tooling, see corpus/README.md.")
            return 1
        CONFIG.write_text(str(target) + "\n", encoding="utf-8")
        print(f"recorded {target} in {CONFIG.name}")
        return 0

    interpreter, origin = resolve()
    if args.which:
        if interpreter is None:
            print(f"no interpreter with {' and '.join(REQUIRED_MODULES)}. "
                  f"Tried:\n{origin}")
            return 1
        print(f"{interpreter}   (from {origin})")
        return 0

    verdicts, failed, skipped = [], False, False

    # The stdlib suite has no prerequisites, so it runs whatever else is true.
    ok, verdict = run_suite(interpreter or Path(sys.executable), STDLIB_SUITE)
    verdicts.append(verdict)
    failed |= not ok

    absent = missing_tools()
    if interpreter is None:
        skipped = True
        verdicts.append(
            f"{TOOLED_SUITE}: SKIPPED, no interpreter can import "
            f"{' and '.join(REQUIRED_MODULES)}. Tried:\n{origin}\n"
            f"  Record one with: python3 scripts/corpus_tests.py --set PATH, "
            f"or export {ENV_VAR}.")
    elif absent:
        skipped = True
        verdicts.append(
            f"{TOOLED_SUITE}: SKIPPED, not on PATH: {', '.join(absent)}. "
            f"See corpus/README.md for what the generator needs.")
    else:
        ok, verdict = run_suite(interpreter, TOOLED_SUITE)
        verdicts.append(verdict)
        failed |= not ok

    print()
    if interpreter is not None:
        print(f"interpreter: {interpreter}   (from {origin})")
    for verdict in verdicts:
        print(f"  {verdict}")

    if failed:
        print("\nFAIL: corpus tooling tests")
        return 1
    if skipped and args.require_prerequisites:
        print("\nFAIL: a prerequisite is absent and is named above, and "
              "--require-prerequisites says that is a failure here. The "
              "caller installed these, so an absent one means an install "
              "silently half-succeeded rather than that this machine never "
              "had them.")
        return 1
    if skipped:
        print("\nSKIPPED: a prerequisite is absent and is named above. This "
              "exits 3, which the gate runner counts and names as a skip. It "
              "is never reported as a pass.")
        return SKIPPED_EXIT
    print("\nOK: corpus tooling tests")
    return 0


if __name__ == "__main__":
    sys.exit(main())
