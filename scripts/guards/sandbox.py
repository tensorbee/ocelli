#!/usr/bin/env python3
"""A disposable repository to drive a guard red in, and the tripwire over it.

**Nothing here may write inside the real repository.** A guard test that leaves
a staged DICOM behind or a corrupted index is worse than the defect it
prevents, so that guarantee is made by construction rather than by cleanup.

## The sandbox is a fresh repository, not a worktree

`docs/runbooks/guard-verification.md` uses `git worktree add --detach`, which
is right for a human doing this once and wrong for a standing harness, for two
reasons the runbook itself records. A worktree is a full checkout, so "every
path-scanning guard otherwise sees a second copy of the whole repository", and
`git worktree add` writes into the developer's `.git`.

So the build is:

1. `tempfile.mkdtemp(prefix="ocelli-guard-")`
2. copy the WORKING TREE content of every path in `git -C REPO_ROOT ls-files`,
   preserving the mode bit
3. in the copy, `git init`, `git add -A`, `git commit`
4. every git call runs with a scrubbed environment

**Working-tree content and not `HEAD`, deliberately.** A developer editing a
guard has to see their edit probed before they commit. Building from `HEAD`
would probe the previous version and report success, which is `AGENTS.md`'s
"stage before you gate" lesson arriving from the other side.

Using `ls-files` rather than a directory walk also excludes `corpus/data`,
`node_modules`, `target`, `tools/oracle/out` and `.claude/verify-ledger.json`
by construction, so no patient data, no rendered reference frame and no local
evidence file is ever copied anywhere. It also avoids `git write-tree`, which
would write objects into the developer's object database.

## The banned pair

The runbook records the trap in terms: `git rm --cached <path>` followed by
`git checkout -- <path>` leaves the working tree broken and the path untracked,
so "the very check you are probing then skips it and reports clean. That is a
false green produced by the cleanup rather than by the guard." `reset()` uses
`reset --hard` and `clean -fdx`, and `git()` refuses the pair outright so the
ban is watched rather than remembered.
"""

from __future__ import annotations

import os
import shutil
import signal
import subprocess
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent

SANDBOX_PREFIX = "ocelli-guard-"
STALE_AFTER_SECONDS = 3600

# The identity a probe commit is attributed to. Fixed, so a commit works on a
# machine with no git identity configured and never carries the developer's
# name.
HARNESS_NAME = "Ocelli guard harness"
HARNESS_EMAIL = "guards@ocelli.invalid"


class SandboxError(RuntimeError):
    """The harness refused to do something unsafe."""


def scrubbed_env(extra: dict[str, str] | None = None) -> dict[str, str]:
    """A child environment that cannot reach the developer's git state.

    Every inherited `GIT_*` variable is removed, so an exported
    `GIT_INDEX_FILE` or `GIT_DIR` cannot redirect a write into the developer's
    index. The global and system config are pointed at `/dev/null`, so the
    developer's config, aliases and hooks path cannot reach the sandbox.
    """
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    env["GIT_CONFIG_GLOBAL"] = os.devnull
    env["GIT_CONFIG_SYSTEM"] = os.devnull
    env["GIT_TERMINAL_PROMPT"] = "0"
    env["GIT_AUTHOR_NAME"] = HARNESS_NAME
    env["GIT_AUTHOR_EMAIL"] = HARNESS_EMAIL
    env["GIT_COMMITTER_NAME"] = HARNESS_NAME
    env["GIT_COMMITTER_EMAIL"] = HARNESS_EMAIL
    # `.githooks/pre-commit` and the guards it calls shell out to python3.
    env.setdefault("PYTHONDONTWRITEBYTECODE", "1")
    if extra:
        env.update(extra)
    return env


# ---------------------------------------------------------------------------
# `repo_read` is the ONLY function that runs git against the real repository,
# and `cwd=REPO_ROOT` appears once in this file, in it. Three functions name
# REPO_ROOT at all and the other two do not write: `Sandbox.git` names it to
# REFUSE a cwd inside or above it, and `build` names it to read a tracked file
# out of it. A reviewer reads those three and then knows the harness cannot
# write to the developer's repository.
#
# An earlier version of this banner said two functions and said nothing else
# below referred to REPO_ROOT, which was false, and a safety argument that
# miscounts its own choke points is not one. `grep -n REPO_ROOT` on this file
# is the check.
# ---------------------------------------------------------------------------

def repo_read(*args: str) -> str:
    """A READ-ONLY git call against the real repository.

    `ls-files` for the copy, and the tripwire's status reads. Nothing here
    takes a write verb, and `git()` below refuses to run against REPO_ROOT at
    all, so a write can only arrive by editing this function.

    `config` is the one verb that is a read or a write depending on its next
    argument, and `git config core.hooksPath X` in REPO_ROOT would rewrite the
    developer's `.git/config`. The tripwire needs `config --get`, so the verb
    cannot simply be banned: the READ forms are listed and everything else is
    refused. The whole safety argument of this file is that a reviewer reads
    these two functions and then knows the harness cannot write, and a verb
    that is one argument away from a write breaks that.
    """
    forbidden = {"add", "commit", "rm", "checkout", "reset", "clean", "init",
                 "apply", "merge", "rebase", "push", "worktree", "gc",
                 "write-tree", "update-index", "stash", "restore", "switch",
                 "mv", "tag", "branch", "fetch", "pull", "am", "cherry-pick"}
    config_reads = {"--get", "--get-all", "--get-regexp", "--list", "-l",
                    "--get-urlmatch"}
    if args and args[0] in forbidden:
        raise SandboxError(
            f"repo_read refuses `git {args[0]}`. This function is the only "
            f"path to the developer's repository and it is read-only.")
    if args and args[0] == "config" and (
            len(args) < 2 or args[1] not in config_reads):
        raise SandboxError(
            f"repo_read refuses `git config {' '.join(args[1:2])}`. `config` "
            f"is a read only in its {', '.join(sorted(config_reads))} forms. "
            f"Every other form writes, and `core.hooksPath` written here is "
            f"the developer's clone.")
    return subprocess.run(["git", *args], cwd=REPO_ROOT, capture_output=True,
                          text=True, check=True,
                          env=scrubbed_env()).stdout


def repo_tracked_paths() -> list[str]:
    out = repo_read("ls-files", "-z")
    return [n for n in out.split("\0") if n]


# ---------------------------------------------------------------------------


@dataclass
class Sandbox:
    """A disposable git repository. Every git call goes through `git`."""

    path: Path
    # The commit the sandbox was built at. `reset()` returns to THIS and not
    # to HEAD, because several probes make real commits through the hooks and
    # `reset --hard` alone would leave them in place. State leaking from one
    # probe into the next is the same false green as a bad cleanup, arriving
    # from the other direction.
    base: str = ""

    # -- the choke point ---------------------------------------------------

    def git(self, *args: str, check: bool = True) -> subprocess.CompletedProcess:
        """The ONE way this harness runs git for a mutation.

        Four refusals, and the fourth is a catalogue entry of its own.
        """
        resolved = self.path.resolve()
        if resolved == REPO_ROOT or REPO_ROOT in resolved.parents \
                or resolved in REPO_ROOT.parents:
            raise SandboxError(
                f"refusing to run git in {resolved}, which is inside or above "
                f"the real repository at {REPO_ROOT}. A probe never touches "
                f"the developer's tree.")
        if not resolved.name.startswith(SANDBOX_PREFIX):
            raise SandboxError(
                f"refusing to run git in {resolved}, which is not a directory "
                f"this harness created. A sandbox is named {SANDBOX_PREFIX}* "
                f"under the system temporary directory and nothing else is.")
        if args and args[0] != "init" and not (resolved / ".git").exists():
            raise SandboxError(
                f"refusing to run git in {resolved}, which carries no .git. "
                f"The handle is not pointing at a built sandbox.")
        if args and args[0] == "rm" and "--cached" in args:
            raise SandboxError(
                "`git rm --cached` is banned in this harness. Followed by "
                "`git checkout --` it leaves the path untracked with the "
                "broken content in place, so the very guard being probed then "
                "skips it and reports clean. That is a false green produced "
                "by the cleanup rather than by the guard. Use reset().")
        return subprocess.run(["git", *args], cwd=resolved,
                              capture_output=True, text=True, check=check,
                              env=scrubbed_env())

    # -- building the rejected state ---------------------------------------

    def write(self, rel: str, data: str | bytes) -> Path:
        target = self.path / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        if isinstance(data, bytes):
            target.write_bytes(data)
        else:
            target.write_text(data, encoding="utf-8")
        return target

    def read(self, rel: str) -> str:
        return (self.path / rel).read_text(encoding="utf-8")

    def append(self, rel: str, text: str) -> None:
        target = self.path / rel
        target.write_text(target.read_text(encoding="utf-8") + text,
                          encoding="utf-8")

    def substitute(self, rel: str, old: str, new: str) -> None:
        """Replace the first occurrence of `old`, refusing a no-op.

        A probe builder that silently stopped mutating anything would leave the
        guard green and the run would report a guard that "did not fire". This
        turns that into a named failure at the point of the edit instead.
        """
        target = self.path / rel
        body = target.read_text(encoding="utf-8")
        if old not in body:
            raise SandboxError(
                f"probe edit of {rel} found nothing matching {old!r}. The "
                f"probe would have mutated nothing and the guard would have "
                f"been green for the wrong reason.")
        target.write_text(body.replace(old, new, 1), encoding="utf-8")

    def delete(self, rel: str) -> None:
        target = self.path / rel
        if not target.exists():
            raise SandboxError(f"probe delete of {rel} found nothing there.")
        target.unlink()

    def stage_all(self) -> None:
        """Stage everything, forced, because several probes write ignored paths.

        `git add -N` deliberately not used anywhere in this harness. The
        runbook's probe 19a records that an intent-to-add path is invisible to
        `git diff --cached --name-only`, so every `--staged` guard would return
        OK over an empty set.

        The verify ledger is excluded by pathspec. It is per-clone evidence,
        `.gitignore` covers it, and forcing it into the index would change the
        staged tree out from under the very record it holds.
        """
        self.git("add", "-A", "-f", "--",
                 ".", ":(exclude).claude/verify-ledger.json")

    def reset(self) -> None:
        self.git("reset", "--hard", "-q", self.base or "HEAD")
        self.git("clean", "-fdxq")

    def enable_hooks(self) -> None:
        self.git("config", "core.hooksPath", ".githooks")

    # -- running the guard --------------------------------------------------

    def run(self, argv: list[str], env: dict[str, str] | None = None,
            timeout: int = 300) -> subprocess.CompletedProcess:
        """Run a guard's real argv with the sandbox as cwd.

        The exit code is read from THIS command and never from the end of a
        pipe, which is the lesson `AGENTS.md` records.
        """
        return subprocess.run(argv, cwd=self.path, capture_output=True,
                              text=True, check=False, env=scrubbed_env(env),
                              timeout=timeout)

    def commit(self, message: str) -> subprocess.CompletedProcess:
        """A real commit, so the hooks run if `enable_hooks` was called."""
        return self.git("commit", "-q", "-m", message, check=False)


def sweep_stale(now: float | None = None) -> int:
    """Remove `ocelli-guard-*` directories older than an hour.

    Layer 3 of the cleanup. A `kill -9` leaves at most one orphan until the
    next run rather than one per run forever.
    """
    now = now or time.time()
    removed = 0
    base = Path(tempfile.gettempdir())
    for entry in base.glob(f"{SANDBOX_PREFIX}*"):
        try:
            if not entry.is_dir():
                continue
            if now - entry.stat().st_mtime < STALE_AFTER_SECONDS:
                continue
            shutil.rmtree(entry, ignore_errors=True)
            removed += 1
        except OSError:
            continue
    return removed


def build() -> Sandbox:
    """Copy the working-tree content of `git ls-files` into a fresh repo."""
    names = repo_tracked_paths()
    if not names:
        raise SandboxError(
            "git ls-files returned nothing, so the sandbox would be an empty "
            "repository and every probe would report its guard silent. A run "
            "over an empty set is the failure this harness exists to refuse.")
    target = Path(tempfile.mkdtemp(prefix=SANDBOX_PREFIX))
    for name in names:
        source = REPO_ROOT / name
        if not source.is_file():
            # Refused rather than skipped. A tracked path that is a symlink, a
            # gitlink or a broken link was dropped silently, so the sandbox
            # differed from the repository and a guard about that path could
            # not fire. There are none today, every tracked entry being
            # 100644 or 100755, and a silent divergence between the copy and
            # the original is the one thing the control run assumes away.
            raise SandboxError(
                f"{name} is tracked and is not a regular file in the working "
                f"tree. The sandbox is a copy of `git ls-files`, so a path it "
                f"cannot copy makes the copy differ from the repository and "
                f"every probe over that path prove nothing. Symlink, gitlink "
                f"or deleted-but-tracked, each needs a decision rather than a "
                f"skip.")
        destination = target / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, destination)
    box = Sandbox(path=target)
    box.git("init", "-q", "-b", "main")
    box.git("config", "user.name", HARNESS_NAME)
    box.git("config", "user.email", HARNESS_EMAIL)
    box.git("config", "commit.gpgsign", "false")
    box.stage_all()
    box.git("commit", "-q", "-m", "base")
    box.base = box.git("rev-parse", "HEAD").stdout.strip()
    return box


class sandbox:  # noqa: N801, a context manager reads better lower case here
    """`with sandbox() as box:` and it is gone afterwards, four ways.

    1. the `finally` below
    2. SIGINT and SIGTERM handlers that remove it and re-raise
    3. `sweep_stale` on the next run, for a `kill -9`
    4. no git call writes inside REPO_ROOT, so there is no residue there

    Point 4 said "nothing is ever written inside REPO_ROOT" until the S03
    review's second pass, and that was false. `scrubbed_env` sets
    `PYTHONDONTWRITEBYTECODE` for CHILDREN, and the PARENT is the process that
    imports `guards.*`, so a run left `scripts/guards/__pycache__` behind.
    `.gitignore` covers it, so the tripwire's `ls-files --others
    --exclude-standard` could not see it either, and a stale `.pyc` is capable
    of making the harness build a rejected state that does not match its
    source, which is the one failure the inversion argument assumes away.
    `scripts/guard_probe.py` and `scripts/guard_census.py` now set
    `sys.dont_write_bytecode` before that import.
    """

    def __init__(self) -> None:
        self.box: Sandbox | None = None
        self._previous: dict[int, object] = {}

    def _handler(self, signum: int, frame) -> None:  # noqa: ANN001
        self._cleanup()
        previous = self._previous.get(signum)
        signal.signal(signum, previous if callable(previous)
                      else signal.SIG_DFL)
        os.kill(os.getpid(), signum)

    def _cleanup(self) -> None:
        if self.box is not None:
            shutil.rmtree(self.box.path, ignore_errors=True)
            self.box = None

    def __enter__(self) -> Sandbox:
        sweep_stale()
        for signum in (signal.SIGINT, signal.SIGTERM):
            try:
                self._previous[signum] = signal.signal(signum, self._handler)
            except ValueError:
                pass  # not the main thread, the finally still runs
        self.box = build()
        return self.box

    def __exit__(self, *exc: object) -> bool:
        try:
            self._cleanup()
        finally:
            for signum, previous in self._previous.items():
                try:
                    signal.signal(signum, previous)  # type: ignore[arg-type]
                except (ValueError, TypeError):
                    pass
        return False


# ---------------------------------------------------------------------------
# The tripwire
# ---------------------------------------------------------------------------

TRIPWIRE_READS: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("HEAD", ("rev-parse", "HEAD")),
    ("unstaged", ("diff", "--name-status")),
    ("staged", ("diff", "--cached", "--name-status")),
    ("untracked", ("ls-files", "--others", "--exclude-standard")),
    ("hooksPath", ("config", "--get", "core.hooksPath")),
)


def tripwire_capture() -> dict[str, str]:
    """What the real repository looks like, content-level and not stat-level.

    Stat-level would fire on `git status` legitimately refreshing the index's
    stat cache, and a tripwire tuned away within a week is not a tripwire.
    """
    state: dict[str, str] = {}
    for label, args in TRIPWIRE_READS:
        try:
            state[label] = repo_read(*args).strip()
        except subprocess.CalledProcessError as error:
            # `config --get` exits 1 when the key is unset, which is a real
            # and normal state, so it is recorded rather than raised.
            state[label] = f"<unset:{error.returncode}>"
    return state


def tripwire_compare(before: dict[str, str],
                     after: dict[str, str]) -> list[str]:
    changed = []
    for label, _ in TRIPWIRE_READS:
        if before.get(label) != after.get(label):
            changed.append(
                f"{label} changed while the harness ran.\n"
                f"      before: {before.get(label)!r}\n"
                f"      after:  {after.get(label)!r}")
    return changed
