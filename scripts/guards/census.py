#!/usr/bin/env python3
"""Prove the catalogue is complete, and that no guard has been quietly widened.

Five checks, because none of them is sufficient alone.

**a. Refusal-site discovery, strict in both directions.** Every site
`scripts/guards/discover.py` finds must be claimed by exactly one catalogue
entry, so the catalogue cannot fall behind. Every entry must claim at least one
site, so a deleted refusal cannot leave a stale entry that reads as coverage.
That is the discipline `docs/lld/oracle.md` already applies to
`unsupported.json`.

**b. Gate and hook coverage.** Every name in `bin/ocelli.sh`'s `GATES` array
has an entry or an explicit `delegated` declaration with a reason, and every
executable under `.githooks/` has entries. The array is parsed with the idiom
`scripts/ci_floor_check.py` already uses, so the repository has one parser for
it and not two.

**c. The declared-constant ratchet.** The class of weakening no probe can
reach. A probe proves a guard still refuses what it refuses, and cannot notice
that the guard's configuration has been widened, because after the widening the
guard is correct about its new, weaker rule.

**d. The profile rule.** An entry whose probe needs a GPU, a browser or the
corpus may not be in the floor. `.claude/WORKFLOW.md`'s floor definition turned
into a mechanism rather than a convention.

**e. The uncovered ratchet.** The count may only decrease. A new uncovered
refusal fails the floor, and once the sweep is recorded complete any non-zero
count fails `--sprint`.
"""

from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path

from .catalogue import CONSTANTS, DEFECTS, GUARDS, Constant, Guard
from .discover import Site, discover

ROOT = Path(__file__).resolve().parent.parent.parent
BUDGET = ROOT / "ci" / "guard-probe-budget.json"

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
    """
    problems = []
    for probe_id, needs, profile in rows:
        if profile == "floor" and needs in {"gpu", "browser", "corpus"}:
            problems.append(
                f"{probe_id} needs {needs} and may not be in the floor. "
                f"`--floor` is the gate set that runs with no GPU, no browser "
                f"and no corpus, and a floor entry needing one of those makes "
                f"that claim false.")
        if profile == "floor" and needs not in {"none"}:
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
    if runner.is_file() and "tests/faults.mjs" not in runner.read_text(
            encoding="utf-8"):
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
    sites = discover()
    matches, claim_problems = match_sites(sites)
    problems += claim_problems

    # b. Every gate has an entry or a declared reason.
    entry_gates = {name for g in GUARDS for name in g.gate.split()
                   if name != "-"}
    for gate in gates_declared():
        if gate in entry_gates or gate in DELEGATED:
            continue
        problems.append(
            f"the `{gate}` gate has no catalogue entry and no `delegated` "
            f"reason. A gate nobody declared is a gate nobody probed.")
    for name in sorted(DELEGATED):
        if name in gates_declared():
            continue
        problems.append(
            f"`{name}` is declared delegated and is not a gate. Remove the "
            f"declaration rather than leaving a reason for nothing.")
    hooks = sorted(p.name for p in (ROOT / ".githooks").iterdir()
                   if p.is_file())
    hook_files = {g.file for g in GUARDS}
    for name in hooks:
        if f".githooks/{name}" not in hook_files:
            problems.append(
                f".githooks/{name} is executable in a clone that opts in and "
                f"has no catalogue entry.")

    # c. The declared-constant ratchet.
    budget = load_budget()
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

    # d. The profile rule.
    rows = [(p.id, p.needs, p.profile) for g in GUARDS for p in g.probes]
    problems += profile_problems(rows)

    # The oracle adoption is verified rather than asserted.
    _, oracle_problems = oracle_adoption(budget.get("oracle_faults"))
    problems += oracle_problems

    # e. The uncovered ratchet.
    uncovered_sites = 0
    uncovered: list[str] = []
    for match in matches:
        if match.guard.kind != "guard":
            continue
        if match.guard.covered:
            continue
        uncovered_sites += len(match.sites)
        uncovered.append(
            f"{match.guard.id} ({match.guard.file}, {len(match.sites)} "
            f"refusal(s)): {match.guard.reason or 'no reason recorded'} "
            f"[owner {match.guard.owner or 'unassigned'}]")

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
    """What the census prints on a green run, in the shape a skip takes."""
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
        f"  {probed_sites} watched by {probes} probe(s) in this harness, "
        f"{adopted} by a named standing test, {out_of_scope} declared out of "
        f"scope, {uncovered} watched by nothing")
    for match in matches:
        if match.guard.kind == "guard" and not match.guard.covered:
            lines.append(f"  UNCOVERED  {match.guard.id} "
                         f"({len(match.sites)} refusal(s)) "
                         f"{match.guard.reason}")
        if match.guard.limit:
            lines.append(f"  LIMIT      {match.guard.id}: {match.guard.limit}")
    for defect, text in sorted(DEFECTS.items()):
        lines.append(f"  DEFECT     {defect}: {text.split('.')[0]}.")
    return lines
