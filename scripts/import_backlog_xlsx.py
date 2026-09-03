#!/usr/bin/env python3
"""One-shot importer, docs/Rust-WASM-Imaging-Backlog.xlsx -> docs/sprints/BACKLOG.md.

Run once at repo bootstrap. After the import, `docs/sprints/BACKLOG.md` is the
source of truth for story status and this script is kept for provenance only,
so a reader can see exactly how the 190 spreadsheet rows became F-IDs.

F-IDs are assigned F-001..F-190 in spreadsheet row order. The spreadsheet ID
(E1.1, E30.4) is preserved in the `Epic ref` column so the HLD's parity
checklist, which cites E-IDs in its `Covered by` column, still resolves.

Milestones group epics. Sprints pack stories inside a milestone under two
caps, at most MAX_STORIES rows and at most MAX_WEEKS estimated engineer-weeks,
and never place a story in a sprint earlier than the sprint holding a story it
depends on.

Usage:
  python3 scripts/import_backlog_xlsx.py            # write BACKLOG.md + allocation
  python3 scripts/import_backlog_xlsx.py --check    # re-derive and diff, exit 1 on drift
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

try:
    import openpyxl
except ImportError:  # pragma: no cover
    sys.exit("openpyxl is required: pip install openpyxl")

ROOT = Path(__file__).resolve().parent.parent

# The spreadsheet lives OUTSIDE the repository. It carries the full 190-story
# plan with engineer-week estimates, which publishes the scale and shape of the
# investment. The DERIVED backlog stays tracked, so the workflow is unaffected.
sys.path.insert(0, str(Path(__file__).resolve().parent))
from source_dir import resolve as _resolve_source  # noqa: E402

SOURCE_DIR, _SOURCE_ORIGIN = _resolve_source()
XLSX = SOURCE_DIR / "Rust-WASM-Imaging-Backlog.xlsx"
EXIT_SKIPPED = 3
BACKLOG = ROOT / "docs" / "sprints" / "BACKLOG.md"
ALLOCATION = ROOT / "docs" / "sprints" / "allocation.json"
ADDITIONS = ROOT / "docs" / "sprints" / "additions.json"

MAX_STORIES = 6
MAX_WEEKS = 16

# Epic -> milestone. Phase 1 milestones M1..M13 follow the HLD's own ordering,
# the oracle before any port code (D7) and the first ten files of section 28.
# Phase 1.5 milestones M14..M18 follow the Part III section order.
MILESTONES: list[tuple[str, str, list[str], str]] = [
    ("M1", "Foundations and the differential oracle", ["E1", "E2"],
     "The workspace builds to wasm and to native, and the oracle renders the "
     "corpus through cornerstone3D before any port code exists."),
    ("M2", "DICOM ingest and the pixel pipeline", ["E3", "E4"],
     "A frame parses, decodes and passes the hand-computed LUT fixtures of "
     "HLD section 18.3."),
    ("M3", "Cache and the render core", ["E5", "E6"],
     "One budgeted cache, one wgpu device, the LUT chain running as a shader "
     "stage on both capability tiers."),
    ("M4", "Public API, the boundary, and the stack viewport", ["E16", "E7"],
     "The three-channel boundary is real and a stack viewport diffs clean "
     "against cornerstone3D. This is the Phase 1 credibility gate."),
    ("M5", "Volume, MPR and 3D rendering", ["E8", "E9"],
     "Volumes assemble progressively, reslice obliquely and ray-cast on both "
     "tiers."),
    ("M6", "Segmentation rendering", ["E10"],
     "All three representations render: labelmap, contour, surface."),
    ("M7", "Tool framework and geometry", ["E11"],
     "Interaction state in TypeScript, hit-testing and measurement "
     "mathematics in Rust."),
    ("M8", "Annotation tools", ["E12"],
     "The 26 annotation classes and the ROI statistics engine."),
    ("M9", "Segmentation tools", ["E13"],
     "The 12 segmentation classes."),
    ("M10", "Manipulation and utility tools", ["E14"],
     "The 25 manipulation and utility classes."),
    ("M11", "Annotation state and DICOM interop", ["E15"],
     "SR is the native annotation type, and SEG, RTSTRUCT and TID 1500 round "
     "trip."),
    ("M12", "Migration and rollout", ["E17"],
     "Both libraries coexist, viewport by viewport, with shadow mode before "
     "each cutover."),
    ("M13", "Performance, hardening and release", ["E18", "E19"],
     "Binary size and cold start inside budget, the browser matrix "
     "certified, semver and provenance policy in force."),
    ("M14", "Out-of-core volume streaming", ["E30"],
     "Bounded GPU residency on unbounded data. The first of the three "
     "checkable claims in HLD C.7."),
    ("M15", "The WebGPU compute subsystem", ["E31"],
     "Kernels sharing the renderer's device, every tier-A kernel carrying a "
     "declared fallback."),
    ("M16", "Prompted segmentation and standards-native annotations",
     ["E32", "E34"],
     "SAM2-class prompting against GPU-resident tensors, and GSPS write with "
     "coded concepts."),
    ("M17", "Multi-monitor, attestation and live sessions",
     ["E33", "E36", "E37"],
     "Real multi-monitor, a published divergence bound, and a CRDT over "
     "state that is already a serialisable struct."),
    ("M18", "Whole-slide imaging and the unified scene graph", ["E35"],
     "Radiology and pathology in one scene graph, sharing one annotation "
     "coordinate model."),
]

# HLD section 38, the five Part III hooks that must land inside Phase 1.
# Keyed by spreadsheet ID so a re-numbered F-ID cannot detach a hook silently.
HOOK_EIDS = ["E1.8", "E2.7", "E5.6", "E8.8", "E15.1"]

# Recorded backlog defects. Kept as data so the guard can assert they are still
# declared rather than quietly repaired. See docs/sprints/BACKLOG.md.
KNOWN_DEFECTS = {
    "E35.3": "declared dependency E4.9 does not exist, E4 ends at E4.8",
    "E36.3": "declared dependency E23.1 is a Phase 2 story, so this Phase 1.5 "
             "story cannot start inside Phase 1.5 as written",
}

# The spreadsheet's Notes column cites prior art, and some of that prior art is
# read-blocked by HLD Appendix C.2.1. F-094 (E15.1, SR as the native annotation
# model) is the sharpest case: its note names dwv as a converging precedent,
# and it is exactly the story where an engineer or an agent would go looking.
#
# So the note carries the block. Annotating on import is better than an
# allowlist entry, because the warning lands where the temptation is rather
# than in a policy file nobody opens.
BLOCKED_SOURCES = {
    "dwv": "dwv is GPL-3.0 and must NOT be read by anyone on this project, "
           "human or agent. Take the idea from DICOM PS3.3 and PS3.16, which "
           "is where dwv took it from. See docs/hld/C-competitive-position.md.",
    "horos": "Horos must NOT be read by anyone on this project. LGPL-3 with a "
             "linked AGPL-3 component. See docs/hld/C-competitive-position.md.",
}


# Deviation D-02, docs/hld/DEVIATIONS.md. Four story titles in the spreadsheet
# name crates with a `tb-` prefix, a pre-naming artefact from before the
# project was called Ocelli. HLD sections 4 and 15.1 both give `ocelli-` and
# are the later authority.
#
# Renaming on import rather than in the spreadsheet means the backlog, the
# sprint plan and the crate directory all agree on one spelling, which is what
# a reader searching for a crate name needs. The spreadsheet is left alone as
# the received artefact.
CRATE_RENAMES = {
    "tb-core": "ocelli-core",
    "tb-dicom": "ocelli-dicom",
    "tb-codec": "ocelli-codec",
    "tb-pixel": "ocelli-pixel",
    "tb-volume": "ocelli-volume",
    "tb-cache": "ocelli-cache",
    "tb-compute": "ocelli-compute",
    "tb-render": "ocelli-render",
    "tb-viewport": "ocelli-viewport",
    "tb-geom": "ocelli-geom",
    "tb-seg": "ocelli-seg",
    "tb-wasm": "ocelli-wasm",
    "tb-native": "ocelli-native",
}


# Commercial product names in the spreadsheet's Notes column. The rules live
# with the private source documents for the reason given in split_hld.py: a
# map of what was redacted still contains it.
def _load_redactions() -> list[tuple[str, str]]:
    path = SOURCE_DIR / "redactions.json"
    if not path.exists():
        return []
    return [(r[0], r[1]) for r in json.loads(path.read_text())["rules"]]


def redact_commercial(text: str) -> str:
    for old, new in _load_redactions():
        text = text.replace(old, new)
    return text


def normalise_crate_names(text: str) -> str:
    """Apply deviation D-02: `tb-*` crate names become `ocelli-*`."""
    for old, new in CRATE_RENAMES.items():
        text = re.sub(rf"\b{re.escape(old)}\b", new, text)
    return text


def annotate_blocked_sources(note: str) -> str:
    """Append the read-block to any note citing a blocked project."""
    additions = [
        text for name, text in BLOCKED_SOURCES.items()
        if re.search(rf"\b{name}\b", note, re.I)
    ]
    if not additions:
        return note
    return f"{note} [SOURCE POLICY: {' '.join(additions)}]"


PHASE_LABEL = {
    "P1 - Cornerstone parity": "P1",
    "P1.5 - Differentiators": "P1.5",
    "P2 - Server (roadmap)": "P2",
    "P3 - Workstation (roadmap)": "P3",
}


def load_rows() -> list[dict]:
    wb = openpyxl.load_workbook(XLSX, data_only=True)
    ws = wb["Backlog"]
    rows = list(ws.iter_rows(values_only=True))
    header = list(rows[0])
    out = []
    for index, raw in enumerate(r for r in rows[1:] if r[0]):
        row = dict(zip(header, raw))
        deps = (row["Depends on"] or "").strip()
        out.append({
            "fid": f"F-{index + 1:03d}",
            "eid": row["ID"].strip(),
            "phase": PHASE_LABEL[row["Phase"]],
            "epic": row["Epic"].strip(),
            "epic_key": row["Epic"].split()[0],
            "story": normalise_crate_names(row["Story"].strip()),
            "layer": (row["Layer"] or "").strip(),
            "weeks": int(row["Est (eng-weeks)"]),
            "depends_eids": [d.strip() for d in deps.split(",") if d.strip()],
            "notes": redact_commercial(annotate_blocked_sources(
                normalise_crate_names((row["Notes"] or "").strip()))),
        })
    return out


def assign_milestones(rows: list[dict]) -> None:
    epic_to_milestone = {}
    for key, _title, epics, _goal in MILESTONES:
        for epic in epics:
            epic_to_milestone[epic] = key
    for row in rows:
        row["milestone"] = epic_to_milestone.get(row["epic_key"], "")


def pack_sprints(rows: list[dict]) -> None:
    """Assign S01.. to every P1 and P1.5 story, milestone by milestone."""
    by_eid = {r["eid"]: r for r in rows}
    sprint_of: dict[str, int] = {}
    sprint_number = 0
    planned = [r for r in rows if r["phase"] in ("P1", "P1.5")]

    for key, _title, _epics, _goal in MILESTONES:
        pool = [r for r in planned if r["milestone"] == key]
        placed: set[str] = set()
        while pool:
            sprint_number += 1
            bucket: list[dict] = []
            weeks = 0
            progressed = True
            while progressed and len(bucket) < MAX_STORIES:
                progressed = False
                for row in list(pool):
                    if len(bucket) >= MAX_STORIES:
                        break
                    if weeks and weeks + row["weeks"] > MAX_WEEKS:
                        continue
                    # A dependency inside the planned set must already sit in an
                    # EARLIER sprint. A dependency outside it (a Phase 2 story,
                    # or the non-existent E4.9) is recorded, not enforced.
                    blocked = False
                    for dep in row["depends_eids"]:
                        target = by_eid.get(dep)
                        if target is None or target["phase"] not in ("P1", "P1.5"):
                            continue
                        if dep not in sprint_of or sprint_of[dep] >= sprint_number:
                            blocked = True
                            break
                    if blocked:
                        continue
                    bucket.append(row)
                    weeks += row["weeks"]
                    pool.remove(row)
                    progressed = True
            if not bucket:
                # Everything left in this milestone depends on something in this
                # same sprint. Force the smallest one so the packer terminates.
                row = min(pool, key=lambda r: r["weeks"])
                bucket.append(row)
                pool.remove(row)
            for row in bucket:
                row["sprint"] = f"S{sprint_number:02d}"
                sprint_of[row["eid"]] = sprint_number
                placed.add(row["eid"])

    for row in rows:
        row.setdefault("sprint", "")


def load_additions() -> list[dict]:
    """Stories added after the import. See docs/sprints/additions.json.

    These are NOT run through the sprint packer. The packer exists to turn a
    dependency graph into an order, and an addition is placed deliberately by
    a person who already knows where it belongs. What IS enforced is the
    packer's invariant: an addition may not sit at or before the sprint
    holding something it depends on.
    """
    if not ADDITIONS.exists():
        return []
    payload = json.loads(ADDITIONS.read_text())
    out = []
    for item in payload["stories"]:
        out.append({
            "fid": item["fid"],
            "eid": item["ref"],
            "phase": item["phase"],
            "epic": item["epic"],
            "epic_key": item["epic"].split()[0],
            "story": item["story"],
            "layer": item["layer"],
            "weeks": item["weeks"],
            "depends_eids": [],
            "depends_fids_declared": item["depends_fids"],
            "notes": item["notes"],
            "milestone": item["milestone"],
            "sprint": item["sprint"],
            "added": True,
        })
    return out


def check_addition_order(rows: list[dict]) -> list[str]:
    """An addition must land strictly after everything it depends on."""
    number = {r["fid"]: int(r["sprint"][1:]) for r in rows if r["sprint"]}
    problems = []
    for row in rows:
        if not row.get("added"):
            continue
        mine = number.get(row["fid"])
        for dep in row["depends_fids_declared"]:
            theirs = number.get(dep)
            if theirs is None:
                problems.append(
                    f"{row['fid']} depends on {dep}, which carries no sprint")
            elif mine is not None and theirs >= mine:
                problems.append(
                    f"{row['fid']} is in {row['sprint']} and depends on {dep} "
                    f"in S{theirs:02d}. An addition must land strictly after "
                    f"what it depends on.")
    return problems


def resolve_dependencies(rows: list[dict]) -> None:
    by_eid = {r["eid"]: r["fid"] for r in rows}
    known = {r["fid"] for r in rows}
    for row in rows:
        if row.get("added"):
            row["depends_fids"] = [
                d if d in known else f"{d} (UNRESOLVED)"
                for d in row["depends_fids_declared"]
            ]
            continue
        resolved = []
        for dep in row["depends_eids"]:
            resolved.append(by_eid.get(dep, f"{dep} (UNRESOLVED)"))
        row["depends_fids"] = resolved


def render_backlog(rows: list[dict]) -> str:
    lines: list[str] = []
    w = lines.append

    w("# Backlog")
    w("")
    w("Live status table for every F-ID. This file is the **execution-time")
    w("tracker** and the source of truth for story status. The story rationale,")
    w("architecture and acceptance context live in `docs/hld/`.")
    w("")
    w("Statuses: `pending`, `in-progress`, `done`, `archived`, `superseded`.")
    w("")
    w("Updated by `/complete-feature` (single-row updates) and `/sync-status`")
    w("(consistency audit). The summary block is regenerated by")
    w("`scripts/gen_backlog_summary.py` and asserted by `/verify`.")
    w("")
    w("F-IDs were assigned F-001..F-190 in spreadsheet row order by")
    w("`scripts/import_backlog_xlsx.py`. The `Epic ref` column carries the")
    w("original spreadsheet ID so the HLD parity checklist, whose `Covered by`")
    w("column cites E-IDs, still resolves.")
    w("")

    w("<!-- AUTOGEN:backlog-summary START -->")
    w("## Summary")
    w("")
    w("| Milestone | Phase | F-IDs | Done | In Progress | Pending | Est (eng-weeks) |")
    w("|-----------|-------|-------|------|-------------|---------|-----------------|")
    total_ids = total_weeks = 0
    for key, title, _epics, _goal in MILESTONES:
        group = [r for r in rows if r["milestone"] == key]
        if not group:
            continue
        weeks = sum(r["weeks"] for r in group)
        total_ids += len(group)
        total_weeks += weeks
        w(f"| {key}, {title} | {group[0]['phase']} | {len(group)} | 0 | 0 | "
          f"{len(group)} | {weeks} |")
    roadmap = [r for r in rows if r["phase"] in ("P2", "P3")]
    roadmap_weeks = sum(r["weeks"] for r in roadmap)
    w(f"| Roadmap, Phase 2 and Phase 3 (unscheduled) | P2/P3 | {len(roadmap)} "
      f"| 0 | 0 | {len(roadmap)} | {roadmap_weeks} |")
    w(f"| **Total** | | **{total_ids + len(roadmap)}** | **0** | **0** | "
      f"**{total_ids + len(roadmap)}** | **{total_weeks + roadmap_weeks}** |")
    w("")
    w("_Counting rule. Each F-ID is counted exactly once, in the milestone it")
    w("occupies, so the F-IDs column sums to the Total. Phase 2 and Phase 3 rows")
    w("carry no milestone and no sprint, they are tracked but unscheduled._")
    w("")
    by_eid = {r["eid"]: r for r in rows}
    hooks = ", ".join(
        f"{by_eid[e]['fid']} ({e})" for e in HOOK_EIDS
    )
    w(f"_Phase 1 is {sum(r['weeks'] for r in rows if r['phase'] == 'P1')} "
      f"engineer-weeks across "
      f"{len([r for r in rows if r['phase'] == 'P1'])} stories, and Phase 1.5 "
      f"is {sum(r['weeks'] for r in rows if r['phase'] == 'P1.5')} across "
      f"{len([r for r in rows if r['phase'] == 'P1.5'])}. Those are the HLD's")
    w("own figures, section 38 and the Part III preamble, reached here by")
    w("summing the imported rows. HLD section 38 states Phase 1 grows from 382")
    w("to 397 engineer-weeks to carry the five Part III hooks, which are")
    w(f"{hooks}. They are inside the count above._")
    w("<!-- AUTOGEN:backlog-summary END -->")
    w("")

    w("## Recorded defects in the imported backlog")
    w("")
    w("Carried as declared rather than silently repaired, because a dependency")
    w("quietly repointed is a planning error that reappears at implementation.")
    w("`scripts/backlog_check.py` asserts both rows still name them.")
    w("")
    w("| F-ID | Epic ref | Defect |")
    w("|------|----------|--------|")
    for eid, text in KNOWN_DEFECTS.items():
        fid = next(r["fid"] for r in rows if r["eid"] == eid)
        w(f"| {fid} | {eid} | {text} |")
    w("")

    for key, title, _epics, goal in MILESTONES:
        group = [r for r in rows if r["milestone"] == key]
        if not group:
            continue
        w(f"### {key}, {title}")
        w("")
        w(goal)
        w("")
        w("| F-ID | Epic ref | Sprint | Story | Layer | Est | Depends on | Status |")
        w("|------|----------|--------|-------|-------|-----|------------|--------|")
        for row in group:
            deps = ", ".join(row["depends_fids"]) or "-"
            w(f"| {row['fid']} | {row['eid']} | {row['sprint']} | {row['story']} "
              f"| {row['layer']} | {row['weeks']}w | {deps} | pending |")
        w("")

    w("### Roadmap, Phase 2 and Phase 3")
    w("")
    w("Tracked, unscheduled. No sprint is assigned and no milestone claims")
    w("them. They exist here so a Phase 1 design that would make one of them")
    w("expensive is visible at design time, which is the whole point of HLD")
    w("section 13.")
    w("")
    w("| F-ID | Epic ref | Phase | Epic | Story | Layer | Est | Status |")
    w("|------|----------|-------|------|-------|-------|-----|--------|")
    for row in rows:
        if row["phase"] not in ("P2", "P3"):
            continue
        w(f"| {row['fid']} | {row['eid']} | {row['phase']} | {row['epic']} "
          f"| {row['story']} | {row['layer']} | {row['weeks']}w | pending |")
    w("")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    if not XLSX.exists():
        print(f"SKIPPED: the backlog spreadsheet is not present at {XLSX}.")
        print(f"Resolved from: {_SOURCE_ORIGIN}. It lives outside the")
        print("repository because it carries the full")
        print("effort plan. Record its location with")
        print("`python3 scripts/source_dir.py --set PATH` to verify the")
        print("backlog. A check that cannot run is NOT a check that passed,")
        print("which is why this exits 3 and not 0.")
        return EXIT_SKIPPED

    rows = load_rows()
    assign_milestones(rows)
    pack_sprints(rows)
    rows.extend(load_additions())
    resolve_dependencies(rows)

    order_problems = check_addition_order(rows)
    if order_problems:
        print("FAIL: a project-added story is misplaced")
        for problem in order_problems:
            print(f"  {problem}")
        return 1

    text = render_backlog(rows)

    allocation = {
        "milestones": [
            {"key": k, "title": t, "epics": e, "goal": g}
            for k, t, e, g in MILESTONES
        ],
        "stories": rows,
    }
    payload = json.dumps(allocation, indent=1) + "\n"

    if args.check:
        drift = []
        if not BACKLOG.exists() or BACKLOG.read_text() != text:
            drift.append(str(BACKLOG.relative_to(ROOT)))
        if not ALLOCATION.exists() or ALLOCATION.read_text() != payload:
            drift.append(str(ALLOCATION.relative_to(ROOT)))
        if drift:
            print("FAIL: re-derived import differs from tracked file(s): "
                  + ", ".join(drift))
            return 1
        print("OK: imported backlog matches the spreadsheet")
        return 0

    BACKLOG.write_text(text)
    ALLOCATION.write_text(payload)
    sprints = sorted({r["sprint"] for r in rows if r["sprint"]})
    print(f"wrote {BACKLOG.relative_to(ROOT)} "
          f"({len(rows)} stories, {len(sprints)} sprints)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
