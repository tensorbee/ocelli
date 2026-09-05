#!/usr/bin/env python3
"""The guard census. Is the catalogue complete, and has any guard been widened.

`scripts/guard_probe.py` proves each declared refusal still refuses. This
proves the DECLARATION is complete, which is the half a probe cannot reach:
a probe run over a catalogue that has fallen behind is a green answer to a
question nobody asked.

Five checks, described in `scripts/guards/census.py`. The two that matter most
are the bidirectional site matching, which makes a guard added next month
arrive with its test or turn CI red, and the declared-constant ratchet, which
is the only thing that notices a guard being widened rather than broken.

Usage:
  python3 scripts/guard_census.py                 # the floor census
  python3 scripts/guard_census.py --profile deep  # also fails on any uncovered
  python3 scripts/guard_census.py --record        # rewrite the recorded values
  python3 scripts/guard_census.py --check-runbook # the generated probe table
  python3 scripts/guard_census.py --render-runbook
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

# See `scripts/guard_probe.py` for why this is here and why it is before the
# import: importing `guards.*` writes `__pycache__` inside the repository
# otherwise, and this file runs on every floor gate.
sys.dont_write_bytecode = True

sys.path.insert(0, str(Path(__file__).resolve().parent))

from guards import census  # noqa: E402
from guards.catalogue import CONSTANTS, DEFECTS, GUARDS  # noqa: E402
from guards.census import (BUDGET, ROOT, constant_digest,  # noqa: E402
                           constant_value, load_budget, match_sites)
from guards.discover import discover  # noqa: E402

RUNBOOK = ROOT / "docs" / "runbooks" / "guard-verification.md"
BEGIN = "<!-- BEGIN GENERATED PROBE TABLE, scripts/guard_census.py -->"
END = "<!-- END GENERATED PROBE TABLE -->"


def render_table() -> str:
    """The probe table, projected from the catalogue.

    The prose around it in the runbook is the author's and is hand-written.
    Only the table is generated, because a table of probes drifts the moment a
    guard is added, which is the defect this story exists to fix.
    """
    lines = [
        BEGIN,
        "",
        "| # | Guard | Probe | Drives red | Profile | Watched |",
        "|---|-------|-------|------------|---------|---------|",
    ]
    number = 0
    for guard in GUARDS:
        if guard.kind != "guard":
            continue
        if not guard.probes:
            watched = "; ".join(guard.covered_by) or "nothing"
            number += 1
            lines.append(
                f"| {number} | `{guard.file}` | none in this harness | "
                f"{guard.refuses} | - | {watched} |")
            continue
        for probe in guard.probes:
            number += 1
            polarity = ("must refuse" if probe.polarity == "refuse"
                        else "must ACCEPT")
            defect = (f" **{probe.defect}, fails today**" if probe.defect
                      else "")
            lines.append(
                f"| {number} | `{guard.file}` | `{probe.id}`, level "
                f"{probe.level}{defect} | {polarity}, output carries "
                f"`{probe.expect}` | {probe.profile} | "
                f"`bin/ocelli.sh gate "
                f"{'guards' if probe.profile == 'floor' else 'guards-deep'}` |")
    lines.append("")
    lines.append("Known defects this table names, in full:")
    lines.append("")
    for key in sorted(DEFECTS):
        lines.append(f"- **{key}.** {DEFECTS[key]}")

    lines.append("")
    lines.append("What a probe above does NOT reach, declared rather than "
                 "left to be discovered:")
    lines.append("")
    for guard in GUARDS:
        if guard.limit:
            lines.append(f"- **`{guard.file}`.** {guard.limit}")

    lines.append("")
    lines.append("Declared out of scope. Each of these carries refusals that "
                 "the census counts and that no gate runs, so calling them "
                 "guards would inflate the coverage number:")
    lines.append("")
    for guard in GUARDS:
        if guard.kind != "guard":
            lines.append(f"- **`{guard.file}`.** {guard.reason}")
    lines.append("")
    lines.append(END)
    return "\n".join(lines)


def runbook_body(rendered: str) -> str:
    text = RUNBOOK.read_text(encoding="utf-8")
    if BEGIN not in text or END not in text:
        raise SystemExit(
            f"FAIL: {RUNBOOK.relative_to(ROOT)} carries no generated-table "
            f"markers. The table is a projection of the catalogue and the "
            f"prose around it is hand-written, so the markers have to stay.")
    head = text.split(BEGIN)[0]
    tail = text.split(END, 1)[1]
    return head + rendered + tail


def record_constants() -> int:
    budget = load_budget()
    constants = {}
    for constant in CONSTANTS:
        value = constant_value(constant, ROOT)
        if value is None:
            print(f"  cannot read {constant.file}:{constant.name}")
            continue
        constants[f"{constant.file}:{constant.name}"] = {
            "digest": constant_digest(value),
            "tunable": constant.tunable,
            "guard": constant.guard,
        }
    budget["constants"] = constants
    budget["oracle_faults"] = census.oracle_adoption(None)[0]
    sites = discover()
    matches, _ = match_sites(sites)
    uncovered = sum(len(m.sites) for m in matches
                    if m.guard.kind == "guard" and not m.guard.covered)
    ratchet = budget.setdefault("uncovered", {})
    previous = ratchet.get("sites")
    if previous is not None and uncovered > previous:
        print(f"  the uncovered ceiling RISES from {previous} to {uncovered}. "
              f"That is the ratchet going the wrong way, and it belongs in "
              f"this diff with a reason beside it.")
    if ratchet.get("sweep_complete") and uncovered > 0:
        # Derived and not remembered. `setdefault` left this true forever once
        # it had been true once, so a run that found uncovered refusals wrote
        # a ceiling that contradicted the flag beside it.
        print(f"  the sweep was recorded complete and is not: {uncovered} "
              f"refusal(s) are watched by nothing.")
    ratchet["sites"] = uncovered
    ratchet["sweep_complete"] = uncovered == 0
    BUDGET.parent.mkdir(parents=True, exist_ok=True)
    BUDGET.write_text(json.dumps(budget, indent=2, sort_keys=True) + "\n")
    print(f"recorded {len(constants)} constant(s), uncovered={uncovered}, "
          f"sweep_complete={ratchet['sweep_complete']}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", default="floor",
                        choices=["floor", "deep"])
    parser.add_argument("--record", action="store_true")
    parser.add_argument("--check-runbook", action="store_true")
    parser.add_argument("--render-runbook", action="store_true")
    args = parser.parse_args()

    if args.record:
        return record_constants()

    if args.render_runbook:
        RUNBOOK.write_text(runbook_body(render_table()), encoding="utf-8")
        print(f"rendered the probe table into "
              f"{RUNBOOK.relative_to(ROOT)}")
        return 0

    uncovered, problems = census.run(args.profile)

    if args.check_runbook or not problems:
        expected = runbook_body(render_table())
        if RUNBOOK.read_text(encoding="utf-8") != expected:
            problems.append(
                f"{RUNBOOK.relative_to(ROOT)}'s probe table has drifted from "
                f"the catalogue. It is generated between the markers and the "
                f"prose around it is not. Run "
                f"`python3 scripts/guard_census.py --render-runbook`.")

    if problems:
        print("FAIL: the guard census")
        for problem in problems:
            print(f"  {problem}")
        return 1

    for line in census.report_lines(args.profile):
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main())
