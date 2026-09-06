#!/usr/bin/env python3
"""Prove the catalogue is complete, and that no guard has been quietly widened.

The checks are lettered below and none of them is sufficient alone. The
count is deliberately not written here: it read "six" while there were six
and the fifth pass added a seventh, a2, which is the sort of sentence this
repository has already paid for twice.

**a. Refusal-site discovery, strict in both directions.** Every site
`scripts/guards/discover.py` finds must be claimed by exactly one catalogue
entry, so the catalogue cannot fall behind. Every entry must claim at least one
site, so a deleted refusal cannot leave a stale entry that reads as coverage.
That is the discipline `docs/lld/oracle.md` already applies to
`unsupported.json`.

**a2. The per-entry site count, for the gap check a leaves open.** The census's
stated purpose is that a guard added next month arrives with its test, and
until the S03 review's fifth pass that rule did not apply to a guard added to a
file the catalogue ALREADY claims. Most entries claim their file with `"*"`, so
a new `problems.append` in an already-catalogued file lands in the probed
bucket and no number moves. The pass measured it from the other side: it added
224 lines to this module, four of them new refusal branches, and deleting each
in turn left the census, the floor probe profile and the unit suite all green.

So the site count of every `"*"` entry is recorded in
`ci/guard-probe-budget.json` and compared for EQUALITY. A refusal added to a
claimed file, or deleted from one, then fails until the number is re-recorded
in the same change, which is the same "put it in the diff" mechanism check c
uses for the declared constants. It does not claim the new refusal is probed.
It claims a reviewer sees that the file grew one, which is exactly what nothing
said before.

Equality and not a ratchet, deliberately. A refusal DELETED from a claimed file
is the loss check a exists to notice and it is equally invisible, because the
entry goes on claiming the other sites in its file. The catalogue was the
alternative home for these numbers, one per entry, and it was rejected: the
budget already carries every other recorded value, `--record` already writes
them in one place, and one number per catch-all entry spread through the
catalogue would be that many things to hand-edit rather than one command to
re-run. `python3 scripts/guard_census.py --record` prints how many there are,
and a count written here would go stale the first time an entry was added.

**What that number can and cannot see** is `scripts/guards/discover.py`'s shape
table, and the S03 review's sixth pass found the claim ahead of the mechanism.
A refusal built as a list, `problems += [...]`, `problems.extend(...)` or
`return ["..."]`, was invisible: eight in scanned guard files, including the
wasm size ceiling of HLD Appendix A gate A4. Deleting that ceiling left the
headline at 544 and exit 0, and only a probe caught it. Those shapes are
scanned now. **A refusal whose message is built into a local variable first is
FOUND**, and the sentence here said it was not until the S03 review's seventh
pass: `problems.append(` carries no requirement that its argument be a literal.
What was true is that such a refusal took its IDENTITY from the variable's
name, so `scripts/guard_probe.py`'s two inverted-success refusals were ONE site
and deleting the first of them moved no number here at all. `discover.py`
gives those a fallback identity now and declares the remaining limits where the
scan is rather than here.

**b. Gate and hook coverage, in both directions.** Every name in
`bin/ocelli.sh`'s `GATES` array has an entry or an explicit `delegated`
declaration with a reason, every entry's `gate` names a gate that array
declares, the gate COUNT is a ratchet that may only grow, and every executable
under `.githooks/` has entries. The three gate rules are one rule seen three
ways, because the S03 review's fourth pass deleted the `prose` row and measured
a green census, a green `ci_floor_check.py` over one gate fewer, and probes
still passing because they invoke `scripts/prose_check.py` rather than the
gate. The array is parsed with the same
regex `scripts/ci_floor_check.py` uses, and it is a second copy of that regex
rather than a shared one. `bin/ocelli.sh` carries a third. Nothing joins the
three, which is a duplication this module does not get to describe away.

**c. The declared-constant ratchet.** The class of weakening no probe can
reach. A probe proves a guard still refuses what it refuses, and cannot notice
that the guard's configuration has been widened, because after the widening the
guard is correct about its new, weaker rule. The COUNT of declared constants is
itself a ratchet, because deleting a `Constant` together with its recorded row
left every other check here green and disarmed the mechanism in a commit that
read as a cleanup.

**d. The profile rule.** An entry whose probe needs a GPU, a browser or the
corpus may not be in the floor. `.claude/WORKFLOW.md`'s floor definition turned
into a mechanism rather than a convention.

**e. The uncovered ratchet.** The count may only decrease. A new uncovered
refusal fails the floor, and once the sweep is recorded complete any non-zero
count fails `--profile deep`, which is what `run()` tests and what
`bin/ocelli.sh`'s `guards-deep` gate passes. `--profile` takes `floor` and
`deep` and has never taken `sprint`.

**f. `covered_by` names a test that reaches the file.** An entry with no probe
here claims a standing test elsewhere, and until the S03 review's second pass
nothing checked that the test opens the file it is claimed to cover. One did
not: `bench.runner` named a suite that never mentions `tools/bench/run.mjs`,
and its nine refusals were counted as watched. So each named path must resolve,
and at least one of them must reach the guarded file, by naming it, by
importing it, in either direction, or by declaring identifiers the guarded file
implements by name. That last shape is the oracle's: `faults.mjs` declares the
fault ids and the page implements each one.

**A directory claim is refused. `covered_by` names a file.** The first version
of check f accepted a claim naming a directory for the directory's own
existence, with no reachability check at all, and the review's third pass
measured what that costs: one line, `covered_by=("tools/bench/tests/",)`, put
the same nine refusals back into the covered bucket and the census printed
`0 watched by nothing` and exited 0. Pass 3 answered that by resolving a
directory to its files and asking whether one of them reached the guarded file,
and the fourth pass measured the hatch reopened one indirection out.
`scripts/guards/catalogue.py` names every guarded file by construction, as
`file="tools/bench/run.mjs"` and so on, so ANY directory whose walk reaches the
catalogue reached every entry: `covered_by=("scripts/",)`,
`("scripts/guards/",)` and `("docs/",)` each gave zero check-f problems, and
one line moved `bench.runner`'s nine unwatched refusals into the covered
bucket. No entry claims a directory today, so the route is removed rather than
narrowed again. An entry that cannot name a file drops the claim and is counted
as uncovered, which is what `bench.runner` does and what the printed number is
for.

**What the coverage number is and is not.** It is an ENTRY-level claim summed
over refusal sites. `covered_by` says a standing test reaches the file, and
check f verifies that. Neither says the named test drives THIS refusal red, and
`report_lines` says so where the number is printed rather than leaving a reader
to assume otherwise. Only a probe in this harness has been watched fail.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
from dataclasses import dataclass
from pathlib import Path

from .catalogue import CONSTANTS, DEFECTS, GUARDS, Constant, Guard
from .discover import SCAN_SUFFIXES, Site, discover, sites_collapsed

ROOT = Path(__file__).resolve().parent.parent.parent
BUDGET = ROOT / "ci" / "guard-probe-budget.json"

# What each recorded key is, written into the budget file itself so a reader
# who opens it does not have to come here. Declared ONCE, because it was
# written in `scripts/guard_probe.py` and consulted nowhere else, so adding a
# key left the note describing the previous set: `--record-budget` reaches it
# with `setdefault` and the note already existed, so it could never be
# refreshed. `scripts/guard_census.py --record` writes it unconditionally now.
BUDGET_NOTE = (
    "Recorded measurements, not guesses. `wall_clock_seconds` is what the "
    "harness took on the machine that recorded it. `constants` is the "
    "declared-constant ratchet, `constants_count` and `gates_declared` are the "
    "counts that catch one of those being removed rather than changed, "
    "`entry_sites` is the per-entry refusal count for the catalogue entries "
    "claiming their file with `\"*\"`, so a refusal added to an already "
    "claimed file moves a number when it takes one of the shapes "
    "scripts/guards/discover.py scans, `oracle_faults` is the fault-count "
    "ratchet "
    "and `uncovered` is the uncovered-refusal ratchet, all written by "
    "scripts/guard_census.py --record.")

# The two values `Guard.kind` may take. `guard` is a refusal some gate runs and
# `not-a-guard` is a file whose refusals no gate runs. Anything else is a typo,
# and a typo silently moves an entry's refusals out of the uncovered ratchet.
KINDS = frozenset({"guard", "not-a-guard"})

# Gates whose refusal is not ours to probe. Each needs a reason, because an
# omitted row and a deliberate "not ours" read identically later.
DELEGATED = {
    "fmt": "rustfmt decides. Its refusal is upstream's and probing it would "
           "assert a formatting rule this repository does not own.",
    "types": "tsc decides, for the same reason.",
    "clippy": "the lint levels are ours and scripts/lint_policy_check.py "
              "watches them. The individual lint's detection is upstream's.",
    "test": "cargo test runs the crates' own suites, which are their stories' "
            "tests. The trybuild compile-fail cases under "
            "crates/ocelli-compute/tests/ui/ are standing guard tests of "
            "exactly this kind and are recorded against the `device` entry.",
    "wasm": "the build is wasm-pack's. The size ceiling is pin_and_size_check "
            "and is probed.",
    "native": "the build is cargo's. Step 4 is target_feature_check and is "
              "probed.",
    "lint": "eslint decides. The rule's own definition is a declared constant "
            "in the ratchet below.",
}


@dataclass
class Match:
    guard: Guard
    sites: list[Site]


def gates_declared() -> list[str]:
    """The GATES array, read with `scripts/ci_floor_check.py`'s own idiom."""
    runner = (ROOT / "bin" / "ocelli.sh").read_text()
    return re.findall(r'^\s*"([a-z-]+)\|(?:no|YES)\|', runner, re.M)


def profile_problems(rows: list[tuple[str, str, str]]) -> list[str]:
    """`.claude/WORKFLOW.md`'s floor definition, as a mechanism.

    Each row is (probe id, needs, profile). Taken as data rather than read off
    the catalogue so the rule itself has a level-1 probe.

    The two floor branches are `if`/`elif` and that is load-bearing. Written as
    two independent `if`s the second subsumed the first, so the GPU sentence
    could never be the only one printed, and deleting the GPU branch outright
    changed nothing a probe or a test could see. Both of this module's floor
    tests and `census.floor-needing-a-gpu` matched on the words the two
    messages share.
    """
    problems = []
    for probe_id, needs, profile in rows:
        if profile == "floor" and needs in {"gpu", "browser", "corpus"}:
            problems.append(
                f"{probe_id} needs {needs} and may not be in the floor. "
                f"`--floor` is the gate set that runs with no GPU, no browser "
                f"and no corpus, and a floor entry needing one of those makes "
                f"that claim false.")
        elif profile == "floor" and needs != "none":
            problems.append(
                f"{probe_id} needs {needs} and may not be in the floor. An "
                f"entry is floor only when its run needs no cargo, no npm, no "
                f"wasm-pack, no browser, no corpus and no GPU.")
        if profile not in {"floor", "deep"}:
            problems.append(f"{probe_id} declares profile {profile!r}")
    return problems


def constant_value(constant: Constant, root: Path) -> str | None:
    """The recorded text of one strictness-deciding value."""
    if "*" in constant.file:
        # A set spread over several files, such as the crates declaring
        # no_std. The value is the sorted list of files that match.
        hits = sorted(
            p.relative_to(root).as_posix()
            for p in root.glob(constant.file)
            if p.is_file()
            and re.search(constant.pattern, p.read_text(encoding="utf-8")))
        return json.dumps(hits) if hits else None
    path = root / constant.file
    if not path.is_file():
        return None
    match = re.search(constant.pattern, path.read_text(encoding="utf-8"),
                      re.M | re.S)
    if match is None:
        return None
    return re.sub(r"\s+", " ", match.group(1)).strip()


def constant_digest(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()[:16]


def load_budget() -> dict:
    if not BUDGET.exists():
        return {}
    return json.loads(BUDGET.read_text())


def match_sites(sites: list[Site]) -> tuple[list[Match], list[str]]:
    """Claim every site with exactly one entry, and refuse both directions."""
    problems: list[str] = []
    by_file: dict[str, list[Site]] = {}
    for site in sites:
        by_file.setdefault(site.file, []).append(site)

    claimed: dict[str, list[str]] = {}
    matches: list[Match] = []

    # Explicit claims first, and two explicit entries matching the same site is
    # still a double claim. `"*"` is a catch-all and takes only the sites in
    # its file that no explicit claim already owns, so two entries can split a
    # file without either of them enumerating the other's refusals, and every
    # site still ends up with exactly one owner.
    for guard in GUARDS:
        if "*" in guard.claims:
            continue
        mine = [site for site in by_file.get(guard.file, [])
                if any(re.search(pattern, site.message)
                       for pattern in guard.claims)]
        for site in mine:
            claimed.setdefault(site.key, []).append(guard.id)
        matches.append(Match(guard=guard, sites=mine))
    for guard in GUARDS:
        if "*" not in guard.claims:
            continue
        mine = [site for site in by_file.get(guard.file, [])
                if site.key not in claimed]
        for site in mine:
            claimed.setdefault(site.key, []).append(guard.id)
        matches.append(Match(guard=guard, sites=mine))
    order = [g.id for g in GUARDS]
    matches.sort(key=lambda m: order.index(m.guard.id))

    for match in matches:
        if match.sites or match.guard.silent:
            continue
        problems.append(
            f"catalogue entry `{match.guard.id}` claims no refusal site in "
            f"{match.guard.file}. Either the refusal was deleted and the "
            f"entry is stale, or its message was reworded and the claim no "
            f"longer matches. A stale entry reads as coverage.")

    for site in sites:
        owners = claimed.get(site.key, [])
        if not owners:
            problems.append(
                f"{site.file}:{site.line} is a refusal no catalogue entry "
                f"claims: {site.message[:110]!r}. Add an entry to "
                f"scripts/guards/catalogue.py naming what the refusal is FOR "
                f"and how to drive it red.")
        elif len(owners) > 1:
            problems.append(
                f"{site.file}:{site.line} is claimed by {len(owners)} "
                f"entries ({', '.join(owners)}). Exactly one entry owns a "
                f"refusal, or the census counts its coverage twice.")

    return matches, problems


# A path inside a `covered_by` sentence. The rest of the sentence is prose for
# a reader and this is the part a machine can check. The trailing-slash
# alternative is here so a DIRECTORY claim is seen and refused by name rather
# than falling through as no path at all.
COVERED_PATH = re.compile(
    r"([\w][\w./-]*\.(?:py|mjs|js|sh|ts|json)|[\w][\w./-]*/)")

# A record declared as `"kebab-name": {`, which is the shape
# `tools/oracle/src/faults.mjs` uses for the faults the render pages implement
# by name. Two files linked this way are linked more tightly than by a
# filename mention, and nothing else in the catalogue is shaped like it.
DECLARED_ID = re.compile(r'^ {2}"([a-z0-9]+(?:-[a-z0-9]+)+)":\s*\{', re.M)


def _reaches(covering: str, target: Path) -> str | None:
    """Does `covering`'s text reach the file at `target`."""
    if target.name in covering:
        return "names it"
    if f"{target.parent.name}/{target.stem}" in covering:
        return "names it"
    if re.search(rf"\bimport\b[^\n]*\b{re.escape(target.stem)}\b", covering):
        return "imports it"
    return None


def _how(named: Path, base: Path, guard_file: str,
         target_text: str) -> str | None:
    """How one candidate file reaches the guarded file, or `None`.

    The three routes, in the order they were added: the covering file names or
    imports the guarded one, the guarded one names the covering file, or the
    covering file declares identifiers the guarded file implements by name.
    """
    text = named.read_text(encoding="utf-8", errors="replace")
    how = (_reaches(text, Path(guard_file))
           or _reaches(target_text, named.relative_to(base)))
    if how is None and any(name in target_text
                           for name in DECLARED_ID.findall(text)):
        how = "declares ids it implements"
    return how


def covered_by_problems(root: Path | None = None) -> list[str]:
    """Check f. A named standing test has to open the file it is named for.

    Cheap and blunt on purpose. It cannot show that the test drives a
    particular refusal red, and nothing claims it does. What it does show is
    that the test and the file are connected at all, which is the thing that
    was asserted about `tools/bench/run.mjs` and was not true.
    """
    base = root or ROOT
    problems: list[str] = []
    for guard in GUARDS:
        if not guard.covered_by:
            continue
        target = base / guard.file
        target_text = (target.read_text(encoding="utf-8", errors="replace")
                       if target.is_file() else "")
        reached: list[str] = []
        for claim in guard.covered_by:
            match = COVERED_PATH.search(claim)
            if match is None:
                continue
            named = base / match.group(1)
            if named.is_dir():
                # Refused outright, and the reason is that a directory claim
                # cannot be checked here without the catalogue satisfying it.
                # `scripts/guards/catalogue.py` writes every guarded path as
                # `file="tools/bench/run.mjs"`, so a walk of any directory that
                # reaches the catalogue reaches every guarded file, and the
                # pass 3 rule that a directory must HOLD a file reaching the
                # target was true of `scripts/`, `scripts/guards/` and `docs/`
                # for every entry in the catalogue. Excluding the catalogue
                # from the walk would only move the hatch to the next file that
                # happens to name a path. Nothing claims a directory today, so
                # the route is gone.
                problems.append(
                    f"catalogue entry `{guard.id}` is covered by the directory "
                    f"{match.group(1)}. `covered_by` names a FILE, because a "
                    f"directory claim is satisfied by the catalogue itself: "
                    f"scripts/guards/catalogue.py names every guarded file by "
                    f"construction, so any directory whose walk reaches it "
                    f"reaches every entry, and one line put nine unwatched "
                    f"refusals into the covered bucket. Name the test file, or "
                    f"drop the claim and take the uncovered count.")
                continue
            if not named.is_file():
                problems.append(
                    f"catalogue entry `{guard.id}` is covered by "
                    f"{match.group(1)}, which is not in this repository. A "
                    f"claim of coverage naming a file that is gone reads as "
                    f"coverage forever.")
                continue
            how = _how(named, base, guard.file, target_text)
            if how is not None:
                reached.append(f"{match.group(1)} {how}")
        if not reached:
            problems.append(
                f"catalogue entry `{guard.id}` claims {guard.file} is covered "
                f"by a standing test, and no test it names opens that file. "
                f"Either name a test that does open the file, or drop the "
                f"claim and take the uncovered count: a false claim of "
                f"coverage is worse than the gap it hides, because the gap can "
                f"be fixed and the claim will be counted as coverage forever.")
    return problems


def catch_all_sites(matches: list[Match]) -> dict[str, int]:
    """How many refusal sites each `claims=("*",)` entry owns.

    The catch-all is what makes a new refusal in an already-claimed file
    invisible, so this is the number that has to be recorded. An entry with
    explicit claims is not here: adding a refusal it does not match makes the
    site unclaimed, which check a already refuses by name.
    """
    return {match.guard.id: len(match.sites) for match in matches
            if "*" in match.guard.claims}


def catch_all_problems(matches: list[Match],
                       recorded: dict[str, int] | None) -> list[str]:
    """Check a2. Every catch-all entry's site count, compared for equality."""
    found = catch_all_sites(matches)
    if recorded is None:
        return [
            f"{BUDGET.relative_to(ROOT)} records no per-entry site count, so "
            f"a refusal added to a file the catalogue already claims with "
            f"`\"*\"` cannot be noticed. That is the census's own purpose not "
            f"applying to its own files: four refusals added to "
            f"scripts/guards/census.py in the S03 review's fourth pass were "
            f"watched by nothing and every count here stayed still. Record it "
            f"with `python3 scripts/guard_census.py --record`."]
    problems: list[str] = []
    for guard_id in sorted(set(found) | set(recorded)):
        was = recorded.get(guard_id)
        now = found.get(guard_id)
        if was == now:
            continue
        if was is None:
            problems.append(
                f"catalogue entry `{guard_id}` claims its file with `\"*\"` "
                f"and has no recorded site count. Record it in the change that "
                f"adds the entry, so the number it starts from is in the same "
                f"diff as the entry.")
        elif now is None:
            problems.append(
                f"`{guard_id}` has a recorded site count and is no longer a "
                f"catch-all entry in the catalogue. A recorded number nothing "
                f"reads is not a ratchet.")
        else:
            direction = "grew" if now > was else "shrank"
            problems.append(
                f"catalogue entry `{guard_id}` {direction} from {was} refusal "
                f"site(s) to {now}. It claims its file with `\"*\"`, so a "
                f"refusal added to that file lands in the probed bucket and "
                f"nothing else here moves: the census's rule that a guard "
                f"arrives with its test does not otherwise reach a file the "
                f"catalogue already claims. Add a probe for the new refusal, "
                f"or declare why it needs none, and re-record in this diff so "
                f"a reviewer sees the count move.")
    return problems


def wall_clock_problems(recorded: dict[str, float] | None) -> list[str]:
    """The recorded profile timings have to be a possible pair.

    Every other key in `ci/guard-probe-budget.json` is checked here and
    `wall_clock_seconds` was checked by nothing, which is how the S03 review's
    eighth pass found `deep: 17.2` sitting beside `floor: 18.1`.
    `scripts/guard_probe.py`'s `selected()` returns EVERY probe for the deep
    profile and only the `profile == "floor"` ones for the floor, so deep runs
    a superset of floor's work and cannot be the faster of the two. That pair
    is not two measurements of one tree: the seventh pass re-recorded floor
    while moving seventeen probes into deep and left deep at its pre-move
    value, so the `5x + 30` ceiling for the profile that now carries every
    `lint-policy` probe was set from a run that did not contain them. The gate
    was not red, because a stale ceiling that is too GENEROUS never is.

    The superset relation is derived from the catalogue rather than asserted,
    so this stays true if the profile rule is ever rewritten: with the floor's
    probe ids not a subset of the deep set, the comparison says nothing and is
    not made.
    """
    if not recorded:
        return []
    floor = {p.id for g in GUARDS for p in g.probes if p.profile == "floor"}
    deep = {p.id for g in GUARDS for p in g.probes}
    if not floor or not floor <= deep:
        return []
    if "floor" not in recorded or "deep" not in recorded:
        return []
    if recorded["deep"] >= recorded["floor"]:
        return []
    return [
        f"ci/guard-probe-budget.json records the deep profile at "
        f"{recorded['deep']}s and the floor profile at {recorded['floor']}s. "
        f"The deep profile runs all {len(deep)} probe(s) and the floor runs "
        f"{len(floor)} of them, a strict superset, so deep cannot be the "
        f"faster of the two and the pair is not two measurements of one tree. "
        f"One of them was recorded before a change that moved work between the "
        f"profiles, which leaves that profile's `5x + 30` ceiling standing for "
        f"a run it never contained. Re-record both with "
        f"`python3 scripts/guard_probe.py --profile floor --record-budget` and "
        f"the same for `deep`, on one machine, in this diff."]


def oracle_adoption(recorded: int | None) -> tuple[int, list[str]]:
    """Verify the adoption of the oracle's fault catalogue, do not copy it.

    A second declaration of the same faults is the same defect as a second copy
    of the LUT chain, except that it only runs where nobody looks. So the
    census checks three things about the real catalogue and re-declares none of
    them: that it carries faults, that each names the specific message fragment
    that proves the run went red at its own boundary, and that the runner the
    `oracle` gate invokes still replays them. The count is a ratchet, because
    F-X007 grew it from twelve to twenty-three and a silent shrink is exactly
    the kind of loss this story exists to notice.
    """
    problems: list[str] = []
    path = ROOT / "tools" / "oracle" / "src" / "faults.mjs"
    runner = ROOT / "tools" / "oracle" / "run.mjs"
    if not path.is_file():
        return 0, ["tools/oracle/src/faults.mjs is gone, and every oracle "
                   "entry in the catalogue claims coverage from it."]
    text = path.read_text(encoding="utf-8")
    names = re.findall(r'^  "?([a-zA-Z0-9_-]+)"?:\s*\{', text, re.M)
    expectations = re.findall(r"expect:\s*[\'\"](.+?)[\'\"],", text)
    if not names:
        problems.append(
            "tools/oracle/src/faults.mjs declares no fault, so the oracle "
            "entries claim coverage from a catalogue that is empty. That is "
            "the shape the oracle's own runner refuses when a filter selects "
            "nothing.")
    if len(expectations) != len(names):
        problems.append(
            f"tools/oracle/src/faults.mjs declares {len(names)} fault(s) and "
            f"{len(expectations)} message fragment(s). A fault without one "
            f"proves only that the run failed, and a run that failed for "
            f"another reason proves nothing about the guard it aimed at.")
    # The absent-runner case is a problem and not a skip. This was
    # `if runner.is_file() and "tests/faults.mjs" not in ...`, which failed
    # OPEN: deleting tools/oracle/run.mjs skipped the branch entirely, so
    # removing the runner was quieter than breaking it, while the same loss of
    # faults.mjs three lines above was refused.
    if not runner.is_file():
        problems.append(
            "tools/oracle/run.mjs is gone, so the `oracle` gate has no runner "
            "and nothing replays the fault catalogue. Every oracle entry in "
            "the catalogue claims coverage from a replay that cannot happen.")
    elif "tests/faults.mjs" not in runner.read_text(encoding="utf-8"):
        problems.append(
            "tools/oracle/run.mjs no longer reaches tests/faults.mjs, so the "
            "`oracle` gate has stopped replaying the fault catalogue and the "
            "adoption in section 8 of F-X009's plan is no longer true.")
    if recorded is not None and len(names) < recorded:
        problems.append(
            f"tools/oracle/src/faults.mjs declares {len(names)} fault(s) and "
            f"{recorded} were recorded. A fault catalogue may grow. It "
            f"shrinking means a refusal stopped being watched, which is what "
            f"this census exists to notice.")
    return len(names), problems


def run(profile: str = "floor") -> tuple[int, list[str]]:
    """The census. Returns (uncovered site count, problems)."""
    problems: list[str] = []
    budget = load_budget()
    sites = discover()
    matches, claim_problems = match_sites(sites)
    problems += claim_problems

    # `kind` decides which bucket an entry's refusals land in, and nothing
    # validated it. A typo such as `kind="not_a_guard"` is not `"guard"`, so
    # the entry left the uncovered ratchet and was reported as declared out of
    # scope, which is the quietest way to move refusals out of the count.
    for guard in GUARDS:
        if guard.kind not in KINDS:
            problems.append(
                f"catalogue entry `{guard.id}` declares kind "
                f"{guard.kind!r}, which is not one of "
                f"{', '.join(sorted(KINDS))}. An unrecognised kind is not "
                f"`guard`, so the entry's refusals leave the uncovered ratchet "
                f"and are reported as declared out of scope. A typo is enough.")

    # b. Every gate has an entry or a declared reason, and every entry names a
    # gate that exists. The second direction was missing, and deleting a row
    # from `bin/ocelli.sh`'s GATES array shrank `--floor`, `--sprint` and
    # `--all` with nothing to say so: the entry kept its probes, the probes
    # kept passing because they invoke the underlying script directly, and the
    # census counted 487 refusals all claimed.
    declared_gates = gates_declared()
    entry_gates = {name for g in GUARDS for name in g.gate.split()
                   if name != "-"}
    for gate in declared_gates:
        if gate in entry_gates or gate in DELEGATED:
            continue
        problems.append(
            f"the `{gate}` gate has no catalogue entry and no `delegated` "
            f"reason. A gate nobody declared is a gate nobody probed.")
    for name in sorted(DELEGATED):
        if name in declared_gates:
            continue
        problems.append(
            f"`{name}` is declared delegated and is not a gate. Remove the "
            f"declaration rather than leaving a reason for nothing.")
    for guard in GUARDS:
        for name in guard.gate.split():
            if name == "-" or name in declared_gates:
                continue
            problems.append(
                f"catalogue entry `{guard.id}` names the `{name}` gate and "
                f"bin/ocelli.sh's GATES array does not declare it. The entry "
                f"still carries its probes and they still pass, because a "
                f"probe invokes the guard's own script rather than the gate, "
                f"so a deleted row shrinks `--floor`, `--sprint` and `--all` "
                f"and leaves every count here unchanged. Restore the gate, or "
                f"move the entry to `-` and say in `reason` what runs it now.")

    # The same deletion seen from the other side, because an entry can be moved
    # to `-` in the same commit that deletes the row and the loop above then
    # has nothing to say. The count may only grow.
    recorded_gates = budget.get("gates_declared")
    if recorded_gates is None:
        problems.append(
            f"{BUDGET.relative_to(ROOT)} records no gate count, so a row "
            f"deleted from bin/ocelli.sh's GATES array cannot be noticed. "
            f"Record it with `python3 scripts/guard_census.py --record`.")
    elif len(declared_gates) < recorded_gates:
        problems.append(
            f"bin/ocelli.sh's GATES array declares {len(declared_gates)} "
            f"gate(s) and {recorded_gates} were recorded. Deleting a row "
            f"silently shrinks `--floor`, `--sprint` and `--all`. If the gate "
            f"was retired deliberately, re-record in this diff so a reviewer "
            f"sees which one went.")

    hooks = sorted(p.name for p in (ROOT / ".githooks").iterdir()
                   if p.is_file() and os.access(p, os.X_OK))
    hook_files = {g.file for g in GUARDS}
    for name in hooks:
        if f".githooks/{name}" not in hook_files:
            problems.append(
                f".githooks/{name} is executable in a clone that opts in and "
                f"has no catalogue entry.")

    # c. The declared-constant ratchet.
    recorded = budget.get("constants", {})
    for constant in CONSTANTS:
        key = f"{constant.file}:{constant.name}"
        value = constant_value(constant, ROOT)
        if value is None:
            problems.append(
                f"{key} is recorded in the constant ratchet and cannot be "
                f"read from the file. Either it was renamed, in which case "
                f"say so in the catalogue, or the guard's strictness now "
                f"lives somewhere the ratchet cannot see.")
            continue
        digest = constant_digest(value)
        if key not in recorded:
            problems.append(
                f"{key} is declared in the ratchet and has no recorded value "
                f"in {BUDGET.relative_to(ROOT)}. Record it in the same change "
                f"that adds it.")
            continue
        if recorded[key]["digest"] != digest:
            why = constant.why or "This value decides how strict the guard is."
            problems.append(
                f"{key} changed without its recorded value being updated in "
                f"the same change. {why} Recorded "
                f"{recorded[key]['digest']}, found {digest}. If the change is "
                f"deliberate, update ci/guard-probe-budget.json in this diff "
                f"so a reviewer sees the widening.")
    for key in sorted(recorded):
        if key not in {f"{c.file}:{c.name}" for c in CONSTANTS}:
            problems.append(
                f"{key} has a recorded value and is not declared in the "
                f"catalogue's CONSTANTS. A recorded value nothing reads is "
                f"not a ratchet.")

    # The ratchet on the ratchet. Deleting a `Constant` AND its recorded row is
    # a two-line edit that reads as a cleanup, and every check above stays
    # green afterwards: the loop over CONSTANTS no longer visits it and the
    # loop over `recorded` no longer sees it. The one mechanism that catches a
    # guard being WIDENED could therefore be disarmed in one green commit and
    # the widening land in the next, also green. The count may only grow.
    recorded_constants = budget.get("constants_count")
    if recorded_constants is None:
        problems.append(
            f"{BUDGET.relative_to(ROOT)} records no constant count, so a "
            f"declared constant removed together with its recorded row cannot "
            f"be noticed. Record it with "
            f"`python3 scripts/guard_census.py --record`.")
    elif len(CONSTANTS) < recorded_constants:
        problems.append(
            f"the catalogue declares {len(CONSTANTS)} constant(s) and "
            f"{recorded_constants} were recorded. Narrowing the "
            f"declared-constant ratchet disarms the only check that notices a "
            f"guard being widened rather than broken. If a constant was "
            f"retired deliberately, re-record in this diff and say which one "
            f"and why.")

    # c2. The recorded wall clocks have to be POSSIBLE.
    problems += wall_clock_problems(budget.get("wall_clock_seconds"))

    # a2. The per-entry site count for the catch-all entries, which is what
    # makes a refusal added to an already-claimed file visible in a diff.
    problems += catch_all_problems(matches, budget.get("entry_sites"))

    # d. The profile rule.
    rows = [(p.id, p.needs, p.profile) for g in GUARDS for p in g.probes]
    problems += profile_problems(rows)

    # f. A named standing test has to open the file it is named for.
    problems += covered_by_problems()

    # The oracle adoption is verified rather than asserted.
    _, oracle_problems = oracle_adoption(budget.get("oracle_faults"))
    problems += oracle_problems

    # e. The uncovered ratchet. `report_lines` is what names each uncovered
    # entry, its reason and its owner. This only counts, because `run()`
    # returns a count and problems and nothing else.
    uncovered_sites = sum(len(m.sites) for m in matches
                          if m.guard.kind == "guard" and not m.guard.covered)

    ratchet = budget.get("uncovered", {})
    ceiling = ratchet.get("sites")
    if ceiling is None:
        problems.append(
            f"{BUDGET.relative_to(ROOT)} records no uncovered ceiling, so the "
            f"ratchet cannot hold.")
    elif uncovered_sites > ceiling:
        problems.append(
            f"{uncovered_sites} refusal(s) are watched by nothing, and the "
            f"recorded ceiling is {ceiling}. The ratchet may only decrease. "
            f"A refusal added without a probe is a guard nobody has watched "
            f"fail.")
    elif profile == "deep" and ratchet.get("sweep_complete") \
            and uncovered_sites > 0:
        problems.append(
            f"{uncovered_sites} refusal(s) are watched by nothing and the "
            f"sweep is recorded complete, so any non-zero count fails this "
            f"profile.")

    return uncovered_sites, problems


def report_lines(profile: str = "floor") -> list[str]:
    """What the census prints on a green run, in the shape a skip takes.

    **Every number here is an entry-level count summed over refusal sites.**
    It says which BUCKET a refusal's guard entry is in, and not that this
    refusal has been driven red. The wording says so, because the earlier
    wording, "N watched by M probes, N by a named standing test, 0 watched by
    nothing", read as a per-refusal claim and was not one: 76 probes cannot
    drive 227 refusals red, and the S03 review measured 37 of 77 sites in four
    entries never executed by the test their entry named.
    """
    sites = discover()
    matches, _ = match_sites(sites)
    lines = []
    probes = sum(len(g.probes) for g in GUARDS)
    probed_sites = sum(len(m.sites) for m in matches if m.guard.probes)
    adopted = sum(len(m.sites) for m in matches
                  if not m.guard.probes and m.guard.covered_by)
    out_of_scope = sum(len(m.sites) for m in matches
                       if m.guard.kind != "guard")
    uncovered = sum(len(m.sites) for m in matches
                    if m.guard.kind == "guard" and not m.guard.covered)
    lines.append(
        f"OK: {len(sites)} refusal(s) in {len({s.file for s in sites})} "
        f"file(s), all claimed")
    lines.append(
        f"  {probed_sites} belong to an entry carrying one of this harness's "
        f"{probes} probe(s), {adopted} to an entry naming a standing test "
        f"that opens the file, {out_of_scope} declared out of scope, "
        f"{uncovered} watched by nothing")
    lines.append(
        f"  These are ENTRY-level buckets summed over sites, not a count of "
        f"refusals driven red. {sites_collapsed()} further refusal(s) share "
        f"another's words and are not counted at all. Refusals under "
        f"`crates/` and in any file that is not "
        f"{', '.join(sorted(SCAN_SUFFIXES))} are outside this scan by "
        f"decision 7 of F-X009's plan, so this is not every refusal in the "
        f"repository.")
    for match in matches:
        if match.guard.kind == "guard" and not match.guard.covered:
            lines.append(f"  UNCOVERED  {match.guard.id} "
                         f"({len(match.sites)} refusal(s)) "
                         f"[owner {match.guard.owner or 'unassigned'}] "
                         f"{match.guard.reason or 'no reason recorded'}")
        if match.guard.limit:
            lines.append(f"  LIMIT      {match.guard.id}: {match.guard.limit}")
    for defect, text in sorted(DEFECTS.items()):
        lines.append(f"  DEFECT     {defect}: {text.split('.')[0]}.")
    return lines
