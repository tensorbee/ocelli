#!/usr/bin/env python3
"""Resumable sprint run state for /run-sprint.

State lives in `.claude/scratch/SNN-run.json`, which is gitignored. It is
working memory for one autonomous run, not a delivery record. The delivery
record is `docs/sprints/`, and where the two disagree, `docs/sprints/` wins.

The point of the file is that `/run-sprint` can be interrupted, resumed, or
picked up by a different agent without redoing work or losing track of which
worktrees and branches are live.

Usage:
  python3 scripts/sprint_workflow.py init --sprint S01 [--resume]
  python3 scripts/sprint_workflow.py status
  python3 scripts/sprint_workflow.py set-phase implementation
  python3 scripts/sprint_workflow.py mark-feature F-001 --state completed
  python3 scripts/sprint_workflow.py record-review F-001 --pass 2 --defects 0 --smells 0
  python3 scripts/sprint_workflow.py record-verification --profile sprint --result pass
  python3 scripts/sprint_workflow.py validate-handoff F-001
  python3 scripts/sprint_workflow.py close-preflight S01
  python3 scripts/sprint_workflow.py release-notes v0.1.0 --check
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRATCH = ROOT / ".claude" / "scratch"
HANDOFFS = ROOT / ".claude" / "handoffs"
SPRINTS = ROOT / "docs" / "sprints"
ALLOCATION = SPRINTS / "allocation.json"
CHANGELOG = ROOT / "CHANGELOG.md"

PHASES = ["design", "implementation", "integration", "verification",
          "review", "ready_to_close", "blocked"]
STATES = ["pending", "claimed", "in-progress", "reviewed", "prepared",
          "integrated", "completed", "blocked"]

FID = re.compile(r"^F-X?\d{3}[a-z]?$")
TAG = re.compile(r"^v\d+\.\d+\.\d+$")


def state_path(sprint: str) -> Path:
    return SCRATCH / f"{sprint}-run.json"


def active_sprint() -> str:
    """The sprint named by CURRENT_SPRINT.md, which is the authority."""
    text = (SPRINTS / "CURRENT_SPRINT.md").read_text()
    match = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)", text, re.M)
    if match is None:
        sys.exit("CURRENT_SPRINT.md does not name a sprint in its title")
    return match.group(1)


def load(sprint: str | None = None) -> dict:
    sprint = sprint or active_sprint()
    path = state_path(sprint)
    if not path.exists():
        sys.exit(f"no run state for {sprint}. Run: "
                 f"python3 scripts/sprint_workflow.py init --sprint {sprint}")
    return json.loads(path.read_text())


def save(data: dict) -> None:
    SCRATCH.mkdir(parents=True, exist_ok=True)
    state_path(data["sprint"]).write_text(json.dumps(data, indent=1) + "\n")


def planned_features(sprint: str) -> list[dict]:
    data = json.loads(ALLOCATION.read_text())
    return [s for s in data["stories"] if s["sprint"] == sprint]


def cmd_init(args: argparse.Namespace) -> int:
    sprint = args.sprint or active_sprint()
    path = state_path(sprint)

    if path.exists() and args.resume:
        data = json.loads(path.read_text())
        print(f"resumed {sprint} at phase {data['phase']}")
        return cmd_status(argparse.Namespace(sprint=sprint))

    if path.exists() and not args.resume:
        sys.exit(f"{path.relative_to(ROOT)} exists. Pass --resume to continue, "
                 f"or delete it deliberately to restart.")

    stories = planned_features(sprint)
    if not stories:
        sys.exit(f"{sprint} has no stories in allocation.json")

    data = {
        "sprint": sprint,
        "phase": "design",
        "max_review_passes": args.max_review_passes,
        "max_workers": args.max_workers,
        "features": {
            s["fid"]: {
                "eid": s["eid"],
                "title": s["story"],
                "weeks": s["weeks"],
                "depends": s["depends_eids"],
                "state": "pending",
                "owner": None,
                "branch": None,
                "worktree": None,
                "base": None,
                "head": None,
                "reviews": [],
            }
            for s in stories
        },
        "verifications": [],
    }
    save(data)
    print(f"initialised {sprint} with {len(stories)} stories")
    return 0


def cmd_status(args: argparse.Namespace) -> int:
    data = load(getattr(args, "sprint", None))
    print(f"sprint {data['sprint']}, phase {data['phase']}")
    print(f"{'F-ID':<9}{'state':<13}{'owner':<10}{'reviews':<9}title")
    counts: dict[str, int] = {}
    for fid, f in sorted(data["features"].items()):
        counts[f["state"]] = counts.get(f["state"], 0) + 1
        last = f["reviews"][-1] if f["reviews"] else None
        review = (f"p{last['pass']} {last['defects']}d/{last['smells']}s"
                  if last else "-")
        print(f"{fid:<9}{f['state']:<13}{str(f['owner'] or '-'):<10}"
              f"{review:<9}{f['title'][:48]}")
    print()
    print("  ".join(f"{k}={v}" for k, v in sorted(counts.items())))

    remaining = [fid for fid, f in data["features"].items()
                 if f["state"] != "completed"]
    if remaining and data["phase"] != "blocked":
        print(f"\n{len(remaining)} stories are not complete. The run is not "
              f"finished: {', '.join(sorted(remaining))}")
    return 0


def cmd_set_phase(args: argparse.Namespace) -> int:
    if args.phase not in PHASES:
        sys.exit(f"phase must be one of {PHASES}")
    data = load()
    data["phase"] = args.phase
    save(data)
    print(f"phase = {args.phase}")
    return 0


def cmd_mark_feature(args: argparse.Namespace) -> int:
    data = load()
    fid = args.fid
    if fid not in data["features"]:
        sys.exit(f"{fid} is not in sprint {data['sprint']}")
    feature = data["features"][fid]

    if args.state:
        if args.state not in STATES:
            sys.exit(f"state must be one of {STATES}")
        feature["state"] = args.state
    for field in ("owner", "branch", "worktree", "base", "head"):
        value = getattr(args, field)
        if value:
            feature[field] = value

    save(data)
    print(f"{fid}: state={feature['state']} owner={feature['owner']}")
    return 0


def cmd_record_review(args: argparse.Namespace) -> int:
    data = load()
    feature = data["features"].get(args.fid)
    if feature is None:
        sys.exit(f"{args.fid} is not in sprint {data['sprint']}")

    feature["reviews"].append({
        "pass": args.pass_number,
        "defects": args.defects,
        "smells": args.smells,
        "nitpicks": args.nitpicks,
    })
    clean = args.defects == 0 and args.smells == 0
    if clean:
        feature["state"] = "reviewed"
    save(data)

    print(f"{args.fid} pass {args.pass_number}: {args.defects} defects, "
          f"{args.smells} smells, {args.nitpicks} nitpicks")
    if not clean:
        print("  not clean. The loop continues. Zero defects AND zero smells.")

    # Surface a non-converging loop without stopping it.
    recent = feature["reviews"][-3:]
    if len(recent) == 3 and all(r["defects"] + r["smells"] > 0 for r in recent):
        print(f"  NOTE: {args.fid} has had findings on three consecutive "
              f"passes. Worth telling the operator, without stopping the loop.")
    return 0


def cmd_record_verification(args: argparse.Namespace) -> int:
    data = load()
    data["verifications"].append({
        "profile": args.profile,
        "result": args.result,
        "gates": args.gates,
        "corpus": args.corpus,
    })
    save(data)
    print(f"recorded {args.profile} verification: {args.result}")
    return 0


def cmd_validate_handoff(args: argparse.Namespace) -> int:
    data = load()
    fid = args.fid
    feature = data["features"].get(fid)
    if feature is None:
        sys.exit(f"{fid} is not in sprint {data['sprint']}")

    path = HANDOFFS / f"{fid}-ready.md"
    problems = []
    if not path.exists():
        problems.append(f"{path.relative_to(ROOT)} does not exist")
    else:
        text = path.read_text()
        for field in ("Branch", "Base", "Head", "Review", "Verify tree"):
            if f"**{field}**" not in text:
                problems.append(f"handoff has no **{field}** field")

        branch = re.search(r"\*\*Branch\*\*:\s*(\S+)", text)
        if branch:
            expected = f"work/{fid.lower()}-"
            if not branch.group(1).startswith(expected):
                problems.append(
                    f"branch {branch.group(1)} does not start with "
                    f"{expected}. The F-ID is hyphenated exactly as written, "
                    f"so {fid} is {expected}<agent>.")

    if problems:
        print(f"FAIL: handoff for {fid}")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: handoff for {fid} validates")
    print("  NOTE: this checks SHAPE, not ancestry. If the story was "
          "remediated after the handoff was written, the handoff is STALE "
          "and must be regenerated at the reviewed head. /integrate-feature "
          "precondition 2 is what catches that.")
    return 0


def cmd_close_preflight(args: argparse.Namespace) -> int:
    sprint = args.sprint or active_sprint()
    data = load(sprint)
    problems = []

    incomplete = sorted(fid for fid, f in data["features"].items()
                        if f["state"] != "completed")
    if incomplete:
        problems.append(f"not completed: {', '.join(incomplete)}")

    for fid, feature in sorted(data["features"].items()):
        if not feature["reviews"]:
            problems.append(f"{fid} has no recorded review pass")
            continue
        last = feature["reviews"][-1]
        if last["defects"] or last["smells"]:
            problems.append(
                f"{fid} last review pass {last['pass']} reports "
                f"{last['defects']} defects and {last['smells']} smells")

    sprint_verified = [v for v in data["verifications"]
                       if v["profile"] == "sprint" and v["result"] == "pass"]
    if not sprint_verified:
        problems.append("no passing sprint-profile verification recorded")

    leftover = sorted(p.name for p in HANDOFFS.glob("*-ready.md")) \
        if HANDOFFS.is_dir() else []
    if leftover:
        problems.append(
            f"handoffs remain: {', '.join(leftover)}. A handoff that was not "
            f"consumed is proof the integration step did not run.")

    try:
        worktrees = subprocess.run(
            ["git", "worktree", "list", "--porcelain"], cwd=ROOT,
            capture_output=True, text=True, check=True).stdout
        extra = [l.split(" ", 1)[1] for l in worktrees.splitlines()
                 if l.startswith("worktree ") and
                 Path(l.split(" ", 1)[1]).resolve() != ROOT]
        if extra:
            problems.append(f"worktrees remain: {', '.join(extra)}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass

    if problems:
        print(f"FAIL: {sprint} is not ready to close")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: {sprint} is ready to close")
    return 0


def cmd_release_notes(args: argparse.Namespace) -> int:
    tag = args.tag
    if not TAG.match(tag):
        sys.exit(f"tag must be vX.Y.Z, got {tag!r}")

    text = CHANGELOG.read_text()
    heading = f"## {tag}"
    if heading not in text:
        print(f"FAIL: CHANGELOG.md has no section headed exactly '{heading}'")
        return 1

    start = text.index(heading) + len(heading)
    rest = text[start:]
    end = rest.find("\n## ")
    body = (rest if end == -1 else rest[:end]).strip("\n")

    if args.render:
        print(body)
        return 0

    problems = []
    if not body.strip():
        problems.append("the section is empty")
    for placeholder in ("TODO", "TBD", "FIXME", "XXX"):
        if placeholder in body:
            problems.append(f"contains placeholder text {placeholder!r}")
    if not re.search(r"^### ", body, re.M):
        problems.append("has no ### content section")

    unreleased = re.search(r"^## Unreleased\n(.*?)(?=\n## |\Z)", text,
                           re.M | re.S)
    if unreleased and unreleased.group(1).strip():
        above = text.index("## Unreleased") < text.index(heading)
        meaningful = re.search(r"^[-*] ", unreleased.group(1), re.M)
        if above and meaningful:
            problems.append(
                "## Unreleased above this section is not empty, so changes "
                "landed after the notes were written and the notes describe "
                "something other than what is being released")

    if problems:
        print(f"FAIL: release notes for {tag}")
        for problem in problems:
            print(f"  {problem}")
        return 1
    print(f"OK: release notes for {tag} validate ({len(body.splitlines())} "
          f"lines)")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("init")
    p.add_argument("--sprint")
    p.add_argument("--resume", action="store_true")
    p.add_argument("--max-review-passes", type=int, default=3)
    p.add_argument("--max-workers", type=int, default=0)
    p.set_defaults(func=cmd_init)

    p = sub.add_parser("status")
    p.add_argument("--sprint")
    p.set_defaults(func=cmd_status)

    p = sub.add_parser("set-phase")
    p.add_argument("phase")
    p.set_defaults(func=cmd_set_phase)

    p = sub.add_parser("mark-feature")
    p.add_argument("fid")
    p.add_argument("--state")
    p.add_argument("--owner")
    p.add_argument("--branch")
    p.add_argument("--worktree")
    p.add_argument("--base")
    p.add_argument("--head")
    p.set_defaults(func=cmd_mark_feature)

    p = sub.add_parser("record-review")
    p.add_argument("fid")
    p.add_argument("--pass", dest="pass_number", type=int, required=True)
    p.add_argument("--defects", type=int, required=True)
    p.add_argument("--smells", type=int, required=True)
    p.add_argument("--nitpicks", type=int, default=0)
    p.set_defaults(func=cmd_record_review)

    p = sub.add_parser("record-verification")
    p.add_argument("--profile", required=True)
    p.add_argument("--result", required=True, choices=["pass", "fail"])
    p.add_argument("--gates", default="")
    p.add_argument("--corpus", default="absent")
    p.set_defaults(func=cmd_record_verification)

    p = sub.add_parser("validate-handoff")
    p.add_argument("fid")
    p.set_defaults(func=cmd_validate_handoff)

    p = sub.add_parser("close-preflight")
    p.add_argument("sprint", nargs="?")
    p.set_defaults(func=cmd_close_preflight)

    p = sub.add_parser("release-notes")
    p.add_argument("tag")
    p.add_argument("--check", action="store_true")
    p.add_argument("--render", action="store_true")
    p.set_defaults(func=cmd_release_notes)

    args = parser.parse_args()
    if getattr(args, "fid", None) and not FID.match(args.fid):
        sys.exit(f"F-ID must look like F-001 or F-001a, got {args.fid!r}")
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
