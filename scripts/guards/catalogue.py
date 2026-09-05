#!/usr/bin/env python3
"""Every refusal this repository can produce, and what watches each one.

**Production data, not test data.** Several entries carry the bytes, the
manifest fragment or the argv a probe writes, so they are inputs to a mechanism
rather than assertions about one. That is the reason `docs/lld/oracle.md` gives
for keeping `tools/oracle/src/faults.mjs` under `src/` and not under `tests/`,
and it applies here unchanged.

Python rather than JSON, so an entry can carry a callable that builds a
rejected state. Under `scripts/` rather than under `docs/` or `.claude/`,
because a catalogue written as a document would itself be scanned by
`scripts/prose_check.py` and by `scripts/deviation_check.py`, and several
entries have to produce the exact text those two refuse.

## The two fields to read first

`spec` and `refuses`. `spec` is the normative citation and `refuses` is one
sentence written FROM it. **The probe is derived from `refuses`, never from the
guard's source**, which is HLD 27.2 R2 applied to a guard: a probe written from
the code asserts the current regex and passes forever once the regex has been
weakened to match. `expect` is allowed to be the implementation's own words,
and that is not a contradiction. Its only job is to prove the run went red for
the declared reason rather than by accident, which is the argument
`tools/oracle/src/faults.mjs` makes for its own fragments.

## The review question

For each probe: did its INPUT come from the citation or from the guard's
source. That is where the R2 risk lives, and it is what a microscope pass
should interrogate.

## What a known defect is

A probe that fails today because the guard it aims at has a hole this sprint
found. A `defect` field names the hole and `DEFECTS` at the foot of this file
carries them in full, so the count lives in one place and no sentence here can
go stale against it. `python3 scripts/guard_probe.py --list` marks each. A
known-defect probe that FAILS is reported and does not fail the gate. A
known-defect probe that PASSES fails the gate, because the hole was fixed and
the declaration is now a lie. That is the ratchet pointing in the direction
that matters.
"""

from __future__ import annotations

import hashlib
import json
import re
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .sandbox import Sandbox

Runner = Callable[[Sandbox], "subprocess.CompletedProcess[str]"]


@dataclass(frozen=True)
class Invoke:
    """How a guard is run, and the key its control run is cached under."""

    key: str
    run: Runner


def script(*argv: str) -> Invoke:
    """A level-3 run: the guard's real argv, with the sandbox as cwd."""
    listed = list(argv)
    return Invoke(key=" ".join(argv), run=lambda box: box.run(listed))


def python_snippet(label: str, body: str) -> Invoke:
    """A level-1 run: call the detection function with an in-memory input.

    The snippet exits 1 and prints when the function reports a problem, so the
    harness reads the same signal from a level-1 probe as from a level-3 one.
    """
    argv = ["python3", "-c", body]
    return Invoke(key=f"level1:{label}", run=lambda box: box.run(argv))


@dataclass(frozen=True)
class Probe:
    """One rejected state, and the refusal it must produce."""

    id: str
    mutate: Callable[[Sandbox], None] | None
    invoke: Invoke
    expect: str
    needs: str = "none"
    profile: str = "floor"
    level: int = 3
    # "refuse": the guard must exit non-zero. "accept": the guard must exit
    # zero. An accept probe is a false-positive probe, and the runbook's own
    # sentence is why it exists: a guard that fails on everything is as useless
    # as one that fails on nothing.
    polarity: str = "refuse"
    defect: str = ""
    note: str = ""
    # The control run, and it is mandatory. By default it is this same invoke
    # against the UNMUTATED sandbox, which must exit 0.
    #
    # Some invokes have no healthy state inside a sandbox, because the thing
    # they need is per-clone and `git ls-files` therefore never copies it: a
    # Python environment with pydicom, a private `.docx`, an npm install. For
    # those the control is still run and still mandatory, and it declares the
    # DIFFERENT refusal a healthy repository gives instead. That proves the
    # guard discriminates, which is the whole job of a control, rather than
    # skipping it and proving nothing.
    control: Invoke | None = None
    control_status: int = 0
    control_expect: str = ""


@dataclass(frozen=True)
class Guard:
    """One guard's refusals, its citation, and what watches them."""

    id: str
    file: str
    gate: str
    spec: str
    refuses: str
    claims: tuple[str, ...]
    probes: tuple[Probe, ...] = ()
    covered_by: tuple[str, ...] = ()
    kind: str = "guard"
    reason: str = ""
    owner: str = ""
    silent: str = ""
    limit: str = ""

    @property
    def covered(self) -> bool:
        return bool(self.probes) or bool(self.covered_by)


# ---------------------------------------------------------------------------
# Fixtures. Synthesised, never taken from the corpus.
# ---------------------------------------------------------------------------

# DICOM Part 10: 128 preamble bytes then "DICM". 132 bytes exercises the magic
# refusal exactly. A real corpus row would put patient data in a temporary
# directory for no gain, and the corpus is absent in CI anyway.
DICOM_FIXTURE = b"\x00" * 128 + b"DICM"


def _blocked_project(box: Sandbox) -> str:
    """The first read-blocked project, read from the POLICY not from the guard.

    `docs/SOURCE-POLICY.md` is the citation. Its table marks the read-blocked
    rows `**NO**` in the Read column, and taking the name from there rather
    than from the checker's own constant is what keeps this probe an assertion
    about the policy instead of an assertion about the regex.
    """
    text = box.read("docs/SOURCE-POLICY.md")
    for line in text.splitlines():
        cells = [c.strip() for c in line.split("|")]
        if len(cells) < 5:
            continue
        if cells[3] != "**NO**":
            continue
        name = cells[1].strip("*").split()[0]
        if name:
            return name
    raise AssertionError(
        "docs/SOURCE-POLICY.md declares no read-blocked project, so this "
        "probe has nothing to build and would report its guard silent.")


def _undeclared_deviation(box: Sandbox) -> str:
    """A `D-NN` token the register does not carry, read from the register."""
    text = box.read("docs/hld/DEVIATIONS.md")
    declared = set(re.findall(r"^\|\s*D-(\d{2})\s*\|", text, re.M))
    for number in range(50, 100):
        if f"{number:02d}" not in declared:
            return f"D-{number:02d}"
    raise AssertionError("every two-digit deviation token is declared")


def _first_no_std_crate(box: Sandbox) -> str:
    attribute = "#![cfg_attr(not(test), no_std)]"
    for crate in sorted((box.path / "crates").iterdir()):
        lib = crate / "src" / "lib.rs"
        if lib.is_file() and attribute in lib.read_text(encoding="utf-8"):
            return crate.name
    raise AssertionError("no crate declares no_std, so there is none to lose")


def _not_in_floor(box: Sandbox) -> set[str]:
    """The gates `NOT_IN_FLOOR` excludes, read from the repository.

    One reader rather than three. This parse was written out in
    `_floor_gate_with_own_step` and again in `_a_non_floor_gate_ci_runs`, and
    a third copy was about to be added for the fifth pass's probes.
    `NOT_IN_FLOOR` is the repository's own declaration of what the floor
    excludes and is itself in the declared-constant ratchet, so a literal here
    would be a probe input nobody can check.
    """
    declared = re.search(r"^NOT_IN_FLOOR = \{(.*?)\}",
                         box.read("scripts/ci_floor_check.py"), re.M | re.S)
    if declared is None:
        raise AssertionError(
            "scripts/ci_floor_check.py declares no NOT_IN_FLOOR, so this "
            "probe cannot tell a floor gate from one CI runs elsewhere.")
    return set(re.findall(r'"([a-z-]+)"', declared.group(1)))


def _floor_gate_with_own_step(box: Sandbox) -> str:
    """A floor gate whose CI step is a `bin/ocelli.sh gate <name>` line.

    Read from `.github/workflows/ci.yml`, which is the artefact the guard is
    about.
    """
    workflow = box.read(".github/workflows/ci.yml")
    excluded = _not_in_floor(box)
    for match in re.finditer(r"bin/ocelli\.sh gate ([a-z-]+)", workflow):
        if match.group(1) not in excluded:
            return match.group(1)
    raise AssertionError("no CI step invokes a floor gate by name")


# ---------------------------------------------------------------------------
# Probe builders
# ---------------------------------------------------------------------------

def _stage(box: Sandbox, rel: str, data: str | bytes) -> None:
    box.write(rel, data)
    box.stage_all()


def _dicom_at(rel: str) -> Callable[[Sandbox], None]:
    return lambda box: _stage(box, rel, DICOM_FIXTURE)


def _text_at(rel: str, body: str) -> Callable[[Sandbox], None]:
    return lambda box: _stage(box, rel, body)


def _unsafe_in_a_third_file(box: Sandbox) -> None:
    # HLD 27.2 R5 names two files. This is a third, and nothing about the
    # choice depends on the checker's comment-stripping regex.
    box.append("crates/ocelli-geom/src/lib.rs",
               "\npub fn probe_third_file() {\n"
               "    unsafe { core::hint::unreachable_unchecked() }\n"
               "}\n")
    box.stage_all()


def _blocked_mention(box: Sandbox) -> None:
    name = _blocked_project(box)
    box.write(".claude/plans/F-000-probe.md",
              f"# Probe\n\nThis plan takes its approach from {name}, whose "
              f"source it read.\n")
    box.stage_all()


def _blocked_dependency(box: Sandbox) -> None:
    name = _blocked_project(box)
    manifest = json.loads(box.read("package.json"))
    manifest.setdefault("dependencies", {})[name.lower()] = "^1.0.0"
    box.write("package.json", json.dumps(manifest, indent=2) + "\n")
    box.stage_all()


def _undeclared_deviation_citation(box: Sandbox) -> None:
    token = _undeclared_deviation(box)
    box.write(".claude/plans/F-000-probe.md",
              f"# Probe\n\nThis plan relies on approved deviation {token}.\n")
    box.stage_all()


def _relax_wgpu_pin(box: Sandbox) -> None:
    box.substitute("Cargo.toml", 'wgpu = "=30.0.1"', 'wgpu = "30.0.1"')


def _partial_wgpu_pin(box: Sandbox) -> None:
    # `=` and a major only. Cargo reads it as a band: it resolves against
    # 30.0.1 and `cargo update -p wgpu --precise 30.0.0` under it exits 0,
    # both measured, where `=30.0.1` refuses that version.
    box.substitute("Cargo.toml", 'wgpu = "=30.0.1"', 'wgpu = "=30"')


def _reorder_wgpu_table(box: Sandbox) -> None:
    # The pin is unchanged and still exact. Only the shape of the entry
    # changes, to the table form Cargo accepts everywhere else in this file.
    box.substitute("Cargo.toml", 'wgpu = "=30.0.1"',
                   'wgpu = { features = ["webgl"], version = "=30.0.1" }')


def _drop_wgpu_entry(box: Sandbox) -> None:
    box.substitute("Cargo.toml", 'wgpu = "=30.0.1"', "")


def _oversize_wasm(box: Sandbox) -> None:
    box.write("ci/wasm-size-budget.json",
              json.dumps({"bytes": 1000, "tolerance": 0.05}, indent=2) + "\n")
    box.write("crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm", b"\x00" * 4096)


def _corpus_two_rows(box: Sandbox, digests: tuple[str, str],
                     licences: tuple[str, str] = ("MIT", "MIT")) -> None:
    header = box.read("corpus/manifest.tsv").splitlines()[0]
    rows = [header]
    for index, (sha, licence) in enumerate(zip(digests, licences)):
        rows.append("\t".join([
            f"probe/case{index}.dcm", "CT", "1.2.840.10008.1.2.1",
            "synthetic, probe", "Ocelli guard harness", licence,
            "https://example.invalid/licence", sha, ""]))
    box.write("corpus/manifest.tsv", "\n".join(rows) + "\n")


def _corpus_digest_mismatch(box: Sandbox) -> None:
    body = b"probe case zero\n"
    good = hashlib.sha256(body).hexdigest()
    _corpus_two_rows(box, (good, "0" * 64))
    box.write("corpus/data/probe/case0.dcm", body)
    box.write("corpus/data/probe/case1.dcm", b"probe case one\n")


def _corpus_absent(box: Sandbox) -> None:
    body = b"probe case zero\n"
    good = hashlib.sha256(body).hexdigest()
    _corpus_two_rows(box, (good, hashlib.sha256(b"absent\n").hexdigest()))
    box.write("corpus/data/probe/case0.dcm", body)


def _corpus_unrecorded_licence(box: Sandbox) -> None:
    _corpus_two_rows(box, ("a" * 64, "b" * 64), licences=("MIT", ""))


def _delete_ci_step(box: Sandbox, leave_comment: bool) -> None:
    gate = _floor_gate_with_own_step(box)
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    # The comment names the gate exactly as a step would, which is the point:
    # a person reading the file sees the gate named and nothing runs it.
    replacement = (f"      # the `bin/ocelli.sh gate {gate}` step was here"
                   if leave_comment else "")
    box.substitute(".github/workflows/ci.yml", line, replacement)


def _put_gate_step_behind(box: Sandbox, gate: str, condition: str) -> None:
    """Put one named gate's step behind an `if:`, leaving the step in place."""
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    body = line.lstrip().removeprefix("- ")
    box.substitute(".github/workflows/ci.yml", line,
                   f"{indent}- if: {condition}\n{indent}  {body}")


def _gate_step_behind(box: Sandbox, condition: str) -> None:
    """Put a floor gate's step behind an `if:`, leaving the step in place.

    The step is still a real `run:` line naming the gate, so every earlier
    version of this check passed. Only the events it executes on change, and
    for a condition that excludes the pull request that is the same outcome as
    deleting it.
    """
    _put_gate_step_behind(box, _floor_gate_with_own_step(box), condition)


def _gate_step_split_across_events(box: Sandbox) -> None:
    """Replace a floor gate's one step with two, one per automatic event.

    Between them they cover every event the floor claims, which is the
    question `--floor` actually asks. The first version of the condition
    reader asked whether ONE step covered them all, refused this, and named
    no missing event while doing it, so the message could not be acted on.
    """
    gate = _floor_gate_with_own_step(box)
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    body = line.lstrip().removeprefix("- ")
    box.substitute(
        ".github/workflows/ci.yml", line,
        f"{indent}- if: github.event_name == 'push'\n{indent}  {body}\n"
        f"{indent}- if: github.event_name == 'pull_request'\n{indent}  {body}")


def _drop_no_std_from_one_crate(box: Sandbox) -> None:
    name = _first_no_std_crate(box)
    box.substitute(f"crates/{name}/src/lib.rs",
                   "#![cfg_attr(not(test), no_std)]", "")


def _drop_no_std_everywhere(box: Sandbox) -> None:
    attribute = "#![cfg_attr(not(test), no_std)]"
    hit = False
    for crate in sorted((box.path / "crates").iterdir()):
        lib = crate / "src" / "lib.rs"
        if lib.is_file() and attribute in lib.read_text(encoding="utf-8"):
            box.substitute(f"crates/{crate.name}/src/lib.rs", attribute, "")
            hit = True
    if not hit:
        raise AssertionError("no crate declared no_std to remove")


def _glam_reaches_std(box: Sandbox) -> None:
    box.substitute(
        "Cargo.toml",
        'glam = { version = "0.30", default-features = false, '
        'features = ["libm"] }',
        'glam = { version = "0.30" }')


def _bindgen_direct_dependency(box: Sandbox) -> None:
    box.append("crates/ocelli-geom/Cargo.toml",
               "\nwasm-bindgen = { workspace = true }\n")


def _bindgen_target_gated(box: Sandbox) -> None:
    box.append("crates/ocelli-geom/Cargo.toml",
               "\n[target.'cfg(target_arch = \"wasm32\")'.dependencies]\n"
               "wasm-bindgen = { workspace = true }\n")


def _bindgen_in_source(box: Sandbox) -> None:
    """A real D2 violation, not a string shaped to satisfy a grep.

    D2 is that `wasm-bindgen` appears in exactly one crate, and what that bans
    is a second crate USING it. The earlier probe appended a string literal
    reading `wasm_bindgen`, which satisfies the guard's word-boundary grep and
    is not a violation of anything.
    """
    box.append("crates/ocelli-geom/src/lib.rs",
               "\nuse wasm_bindgen::prelude::*;\n\n"
               "#[wasm_bindgen]\npub fn probe_exported() {}\n")


def _device_creator(box: Sandbox) -> None:
    """A crate other than the renderer bringing a device into existence.

    Written from HLD section 31's sentence, "ocelli-compute never creates a
    `wgpu::Device`, it borrows the one ocelli-render owns", and from wgpu's own
    API for doing so. `Adapter::request_device` is how a `wgpu::Device` comes
    into existence, and the crate it is planted in is the crate section 31
    names. Neither choice was read off `ci/check-device-ownership.sh`'s
    `CREATORS` list, which is what the earlier `let _ = request_adapter;`
    amounted to.
    """
    box.append("crates/ocelli-compute/src/lib.rs",
               "\n/// Probe: HLD 31 forbids exactly this.\n"
               "pub async fn probe_own_device(adapter: &wgpu::Adapter) {\n"
               "    let _ = adapter.request_device("
               "&wgpu::DeviceDescriptor::default()).await;\n"
               "}\n")


def _device_context_gone(box: Sandbox) -> None:
    box.substitute("crates/ocelli-render/src/gpu.rs",
                   "pub struct GpuContext", "pub struct RenamedContext")


def _device_owned_accessor(box: Sandbox) -> None:
    """An accessor that hands an owned device out of `GpuContext`.

    The one probe here whose input cannot come from anywhere but the guard's
    own vocabulary: the rule bans a SHAPE of accessor and the guard is a list
    of four names, so a probe must write one of the four or nothing fires.
    HLD 31 names none of them. `OWNED_ACCESSORS` is therefore in the
    declared-constant ratchet, which is the compensating mechanism: narrowing
    the list to the one name this probe writes fails the census in the same
    change, where a probe alone would stay green with three shapes unguarded.
    """
    box.append("crates/ocelli-render/src/gpu.rs",
               "\n// probe\npub fn into_device() {}\n")


def _device_derives_clone(box: Sandbox) -> None:
    box.substitute("crates/ocelli-render/src/gpu.rs",
                   "pub struct GpuContext",
                   "#[derive(Clone)]\npub struct GpuContext")


def _prose_em_dash(box: Sandbox) -> None:
    box.write(".claude/plans/F-000-probe.md",
              "# Probe\n\nThe rule \u2014 and it is a rule \u2014 is plain "
              "prose.\n")
    box.stage_all()


def _prose_semicolon(box: Sandbox) -> None:
    box.write(".claude/plans/F-000-probe.md",
              "# Probe\n\nThe first clause is here, the second follows; that "
              "is the shape refused.\n")
    box.stage_all()


def _prose_commit_message(box: Sandbox) -> None:
    box.write("probe-message.txt",
              "F-000, a probe\n\nOne clause; and a second.\n")


def _sprint_plan_absent(box: Sandbox) -> None:
    box.delete("docs/sprints/SPRINT_PLAN.md")


def _sprint_plan_wrong_estimate(box: Sandbox) -> None:
    """Change one sprint-table row's estimate and leave the backlog alone.

    The refusal is derived from `gen_sprint_plan.py`'s own docstring rule, that
    a planned F-ID appears with the sprint and the estimate the allocation
    carries. It is NOT derived from a drift found in the tree, and this
    docstring said it was in the strongest form available until the S03
    review's fifth pass checked the claim. **No committed state ever held
    F-X014 at two different estimates**: `git show fe18a91:` gives `1w` in
    `SPRINT_PLAN.md`, `BACKLOG.md` and `allocation.json`, and
    `git show 4139a54:` gives `2w` in all three. Pass 3 widened the story and
    the plan row lagged inside that same edit, which is a drift that lasted one
    edit and that no check has ever caught. The probe is worth having anyway,
    for the reason the guard census gives everywhere else: a comparison nobody
    has watched go red is indistinguishable from one that is broken.
    """
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    row = re.search(r"^\|\s*F-X?\d{3}[a-z]?\s*\|[^|]*\|[^|]*\|[^|]*\|"
                    r"\s*(\d+)w\s*\|\s*$", text, re.M)
    if row is None:
        raise AssertionError("SPRINT_PLAN.md carries no estimated story row")
    weeks = int(row.group(1))
    box.substitute("docs/sprints/SPRINT_PLAN.md", row.group(0),
                   row.group(0).replace(f"| {weeks}w |", f"| {weeks + 1}w |"))


def _sprint_plan_wrong_sprint(box: Sandbox) -> None:
    """Move the first sprint's first story row into the next sprint's table.

    The guard has claimed this refusal since it was written and no probe
    watched it, so the branch that actually matters was proved only by the
    absent-file one above.
    """
    head = re.compile(r"^####\s+Sprint\s+S\d+\s*$")
    story = re.compile(r"^\|\s*F-X?\d{3}[a-z]?\s*\|")
    lines = box.read("docs/sprints/SPRINT_PLAN.md").splitlines(keepends=True)

    heads = [i for i, line in enumerate(lines) if head.match(line.rstrip("\n"))]
    if len(heads) < 2:
        raise AssertionError("SPRINT_PLAN.md carries fewer than two sprints")
    rows = [i for i in range(heads[0], heads[1]) if story.match(lines[i])]
    if not rows:
        raise AssertionError("the first sprint table carries no story row")

    moved = lines.pop(rows[0])
    after = next(i for i in range(rows[0], len(lines))
                 if head.match(lines[i].rstrip("\n")))
    lines.insert(after + 1, moved)
    box.write("docs/sprints/SPRINT_PLAN.md", "".join(lines))


def _sprint_plan_wrong_milestone_summary(box: Sandbox) -> None:
    """Change one milestone summary line's engineer-week total.

    `render` writes this line and `--check` did not read it until the S03
    review's fourth pass, which found M1 claiming 61 engineer-weeks against
    an allocation saying 62. A generated line nothing compares is a
    hand-maintained line that looks generated.
    """
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    line = re.search(r"^_S\d+ to S\d+, \d+ stories, (\d+) engineer-weeks\._$",
                     text, re.M)
    if line is None:
        raise AssertionError("SPRINT_PLAN.md carries no milestone summary line")
    weeks = int(line.group(1))
    box.substitute("docs/sprints/SPRINT_PLAN.md", line.group(0),
                   line.group(0).replace(f"{weeks} engineer-weeks",
                                         f"{weeks + 1} engineer-weeks"))


def _sprint_plan_stale_goal_line(box: Sandbox) -> None:
    """Drop a story title out of a sprint's generated goal paragraph.

    The same defect the fourth pass found in S04's goal line, which still
    named a story title the table row beside it had already replaced.
    """
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    goal = re.search(r"^\*\*Goal\*\*: (.+)$", text, re.M)
    if goal is None:
        raise AssertionError("SPRINT_PLAN.md carries no goal line")
    box.substitute("docs/sprints/SPRINT_PLAN.md", goal.group(0),
                   "**Goal**: A title no story in the allocation carries.")


def _sprint_plan_row_in_two_sprints(box: Sandbox) -> None:
    """Copy a late sprint's story row into the first sprint's table.

    `gen_sprint_plan.py`'s own docstring says every planned F-ID appears in
    "exactly one sprint table", and the parser let the last occurrence win, so
    the copy agreed with the allocation and nothing saw the duplicate.
    Measured in the S03 review's fifth pass: copying S72's F-149 row into S01's
    table left `--check` at exit 0. A story planned into two sprints is a
    planning error that reads as a plan.
    """
    head = re.compile(r"^####\s+Sprint\s+S\d+\s*$")
    story = re.compile(r"^\|\s*F-X?\d{3}[a-z]?\s*\|")
    lines = box.read("docs/sprints/SPRINT_PLAN.md").splitlines(keepends=True)
    heads = [i for i, line in enumerate(lines) if head.match(line.rstrip("\n"))]
    if len(heads) < 2:
        raise AssertionError("SPRINT_PLAN.md carries fewer than two sprints")
    rows = [i for i in range(heads[-1], len(lines)) if story.match(lines[i])]
    if not rows:
        raise AssertionError("the last sprint table carries no story row")
    first_rows = [i for i in range(heads[0], heads[1]) if story.match(lines[i])]
    if not first_rows:
        raise AssertionError("the first sprint table carries no story row")
    lines.insert(first_rows[0], lines[rows[0]])
    box.write("docs/sprints/SPRINT_PLAN.md", "".join(lines))


def _sprint_plan_milestone_summary_deleted(box: Sandbox) -> None:
    """Delete one generated milestone summary line.

    The comparison was positional, so an absent line shifted every line after
    it and the refusal named the wrong milestone, which is a message a
    maintainer cannot act on. The lines are matched on the sprint span they
    name now, and an absent span is refused by name.
    """
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    line = re.search(r"^_S\d+ to S\d+, \d+ stories, \d+ engineer-weeks\._$",
                     text, re.M)
    if line is None:
        raise AssertionError("SPRINT_PLAN.md carries no milestone summary line")
    box.substitute("docs/sprints/SPRINT_PLAN.md", line.group(0), "")


def _sprint_plan_extra_milestone_summary(box: Sandbox) -> None:
    """Add a summary line for a span the allocation has no milestone for."""
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    line = re.search(r"^_S\d+ to S\d+, \d+ stories, \d+ engineer-weeks\._$",
                     text, re.M)
    if line is None:
        raise AssertionError("SPRINT_PLAN.md carries no milestone summary line")
    box.substitute("docs/sprints/SPRINT_PLAN.md", line.group(0),
                   line.group(0) + "\n\n_S98 to S99, 1 stories, "
                                   "1 engineer-weeks._")


def _sprint_plan_two_goal_lines(box: Sandbox) -> None:
    """Give one sprint a second `**Goal**` line above the generated one.

    The parser kept the FIRST per sprint, so a stale paragraph above a
    corrected one won and the corrected one was never read. `render` writes
    exactly one.
    """
    text = box.read("docs/sprints/SPRINT_PLAN.md")
    goal = re.search(r"^\*\*Goal\*\*: (.+)$", text, re.M)
    if goal is None:
        raise AssertionError("SPRINT_PLAN.md carries no **Goal** line")
    box.substitute("docs/sprints/SPRINT_PLAN.md", goal.group(0),
                   "**Goal**: something a previous edit left behind.\n\n"
                   + goal.group(0))


def _stale_codex_adapter(box: Sandbox) -> None:
    box.append(".claude/commands/verify.md",
               "\nA line the adapter has not seen.\n")


def _renumber_an_error_code(box: Sandbox) -> None:
    registry = json.loads(box.read("ci/error-codes.json"))
    codes = registry["codes"]
    codes[0]["number"] = codes[0]["number"] + 1000
    box.write("ci/error-codes.json", json.dumps(registry, indent=2) + "\n")


def _backlog_done_without_record(box: Sandbox) -> None:
    text = box.read("docs/sprints/BACKLOG.md")
    match = re.search(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|.*\|\s*pending\s*\|.*$",
                      text, re.M)
    if match is None:
        raise AssertionError("BACKLOG.md carries no pending row to mark done")
    box.substitute("docs/sprints/BACKLOG.md", match.group(0),
                   match.group(0).replace("| pending |", "| done |"))


def _backlog_bad_status(box: Sandbox) -> None:
    text = box.read("docs/sprints/BACKLOG.md")
    match = re.search(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|.*\|\s*pending\s*\|.*$",
                      text, re.M)
    if match is None:
        raise AssertionError("BACKLOG.md carries no pending row to corrupt")
    box.substitute("docs/sprints/BACKLOG.md", match.group(0),
                   match.group(0).replace("| pending |", "| almost |"))


def _hook_out_of_phase(box: Sandbox) -> None:
    data = json.loads(box.read("docs/sprints/allocation.json"))
    for story in data["stories"]:
        if story["eid"] == "E1.8":
            story["phase"] = "P2"
            break
    else:
        raise AssertionError("E1.8 is not in allocation.json")
    box.write("docs/sprints/allocation.json", json.dumps(data, indent=2) + "\n")


def _ledger_record(box: Sandbox, corpus: str) -> None:
    box.run(["python3", "scripts/verify_ledger.py", "record",
             "--gates", "fmt,clippy", "--corpus", corpus,
             "--profile", "sprint"])


def _ledger_red_corpus(box: Sandbox) -> None:
    _ledger_record(box, "fail")


def _ledger_absent_corpus(box: Sandbox) -> None:
    _ledger_record(box, "absent")


def commit_with(message: str) -> Invoke:
    """A real `git commit`, so the hooks run when the sandbox has enabled them.

    The staged content is the probe's business and lands through `mutate`, so
    the control can run this same commit over a benign staged change and prove
    the hook lets a good commit through.
    """
    return Invoke(
        key=f"git commit: {message.splitlines()[-1][:40]}",
        run=lambda box: box.git("commit", "-m", message, check=False))


CLEAN_COMMIT = commit_with("F-000, a probe")


def _stage_a_dicom_for_commit(box: Sandbox) -> None:
    box.enable_hooks()
    box.write("probe/anon001", DICOM_FIXTURE)
    box.stage_all()


def _stage_a_benign_change(box: Sandbox) -> None:
    box.enable_hooks()
    box.write("probe-note.txt", "a benign change\n")
    box.stage_all()


def _enable_hooks(box: Sandbox) -> None:
    box.enable_hooks()


def _push(sandbox: Sandbox) -> "subprocess.CompletedProcess[str]":
    # Both calls go through the choke point, including the bare remote's own
    # init. A direct subprocess call here would be the one git invocation in
    # the harness that nothing checks, which is exactly the shape the choke
    # point exists to make impossible.
    remote = sandbox.path / ".probe-remote.git"
    sandbox.git("init", "--bare", "-q", str(remote))
    return sandbox.git("push", "--quiet", str(remote), "HEAD:refs/heads/probe",
                       check=False)


def _trailer_without_a_record(
        sandbox: Sandbox) -> "subprocess.CompletedProcess[str]":
    return sandbox.run(["python3", "scripts/verify_ledger.py", "trailer"])


def _commit_amended_tree(box: Sandbox) -> None:
    """Record a ledger entry, then change the tree under it.

    The trailer then names a tree that is not the commit's, which is the
    branch `check-commit` exists for and the load-bearing half of D-04's
    mechanism 2.
    """
    box.enable_hooks()
    box.write("probe-note.txt", "one\n")
    box.stage_all()
    _ledger_record(box, "pass")
    box.git("commit", "-m", "F-000, a probe")
    # Hooks OFF for the amend, and that is the point rather than a shortcut.
    # The commit-msg hook refuses to let a message carrying an Ocelli-Verify
    # trailer through, so carrying one over to a different tree is something
    # only a person editing history by hand can do, and this builds exactly
    # that state.
    box.git("config", "--unset", "core.hooksPath")
    box.write("probe-note.txt", "two\n")
    box.stage_all()
    box.git("commit", "--amend", "--no-edit", "-q")


def sprint_state(box: Sandbox) -> str:
    """Write the per-clone sprint run state the workflow tool needs.

    It is gitignored evidence, so `git ls-files` never copies it and the
    sandbox has none. The sprint comes from `docs/sprints/CURRENT_SPRINT.md`
    and the story from `docs/sprints/allocation.json`, both of which are
    tracked and are the tool's own authorities.
    """
    sprint = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)",
                       box.read("docs/sprints/CURRENT_SPRINT.md"), re.M)
    if sprint is None:
        raise AssertionError("CURRENT_SPRINT.md names no sprint")
    name = sprint.group(1)
    allocation = json.loads(box.read("docs/sprints/allocation.json"))
    fids = sorted(s["fid"] for s in allocation["stories"]
                  if s.get("sprint") == name)
    if not fids:
        raise AssertionError(f"allocation.json puts no story in {name}")
    box.write(f".claude/scratch/{name}-run.json", json.dumps({
        "sprint": name,
        "phase": "integration",
        "features": {fid: {"state": "prepared"} for fid in fids},
    }, indent=1) + "\n")
    return fids[0]


def _handoff(box: Sandbox, branch: str) -> None:
    fid = sprint_state(box)
    box.write(f".claude/handoffs/{fid}-ready.md",
              f"# {fid} ready\n\n"
              f"**Branch**: {branch.replace('FID', fid.lower())}\n"
              f"**Base**: sprint/s03\n"
              f"**Head**: 0123456789ab\n"
              f"**Review**: pass 1, zero defects\n"
              f"**Verify tree**: 0123456789ab\n")
    box.write(".claude/probe-fid", fid)


def _handoff_wrong_branch(box: Sandbox) -> None:
    _handoff(box, "work/not-this-story-agent")


def _handoff_backticked_branch(box: Sandbox) -> None:
    _handoff(box, "`work/FID-agent`")


def _validate_handoff(sandbox: Sandbox) -> "subprocess.CompletedProcess[str]":
    fid = (sandbox.path / ".claude" / "probe-fid").read_text().strip()
    return sandbox.run(["python3", "scripts/sprint_workflow.py",
                        "validate-handoff", fid])


def _empty_source_directory(box: Sandbox) -> None:
    (box.path / "probe-source").mkdir(parents=True, exist_ok=True)


def _unresolvable_graph(box: Sandbox) -> None:
    """A workspace whose feature graph cannot resolve.

    An unknown FEATURE rather than an unknown crate, so cargo refuses from the
    lock file it already has and never reaches the network. A guard that
    cannot resolve the graph and exits 0 is the shape runbook probe 18
    describes: nothing to check, said by succeeding.
    """
    box.substitute("crates/ocelli-core/Cargo.toml",
                   "glam = { workspace = true }",
                   'glam = { workspace = true, features = ["probe-no-such"] }')


def _budget_edit(box: Sandbox, key: str, value: object) -> None:
    budget = json.loads(box.read("ci/guard-probe-budget.json"))
    budget[key] = value
    box.write("ci/guard-probe-budget.json",
              json.dumps(budget, indent=2, sort_keys=True) + "\n")


def _budget_drop(box: Sandbox, key: str) -> None:
    budget = json.loads(box.read("ci/guard-probe-budget.json"))
    if key not in budget:
        raise AssertionError(f"{key} is not recorded, so dropping it is a "
                             f"no-op and the probe would prove nothing")
    del budget[key]
    box.write("ci/guard-probe-budget.json",
              json.dumps(budget, indent=2, sort_keys=True) + "\n")


def _budget_add_constant(box: Sandbox) -> None:
    budget = json.loads(box.read("ci/guard-probe-budget.json"))
    budget["constants"]["scripts/probe.py:NOT_DECLARED"] = {
        "digest": "0" * 16, "tunable": False, "guard": "probe"}
    box.write("ci/guard-probe-budget.json",
              json.dumps(budget, indent=2, sort_keys=True) + "\n")


def _widen_a_declared_constant(box: Sandbox) -> None:
    """Widen the DICOM suffix allow-list, read from the file rather than typed.

    The earlier version carried
    `DICOM_SUFFIXES = {".dcm", ".dicom", ".ima"}` as a literal, which is a
    second copy of another guard's source line inside this catalogue. The
    ratchet's rule is that a strictness-deciding value cannot change without
    its recorded value changing with it, so the probe only has to produce SOME
    change to that value, and reading the current one is how it stays true
    when the list grows.
    """
    current = re.search(r"^DICOM_SUFFIXES = (\{.*?\})$",
                        box.read("scripts/staged_content_check.py"), re.M)
    if current is None:
        raise AssertionError(
            "scripts/staged_content_check.py declares no DICOM_SUFFIXES, so "
            "the ratchet has nothing to widen and this probe would report its "
            "guard silent.")
    box.substitute("scripts/staged_content_check.py", current.group(1),
                   '{".dcm"}')


def _retire_a_declared_constant(box: Sandbox) -> None:
    """Delete one `Constant` AND its recorded row. The two-line cleanup.

    The ratchet on the ratchet, and it was watched by nothing. Both loops go
    quiet afterwards, the one over `CONSTANTS` and the one over the recorded
    rows, so the single mechanism that notices a guard being WIDENED rather
    than broken could be disarmed in a commit that reads as tidying up and the
    widening land in the next commit, also green. The count is what catches it.

    The entry removed is the LAST one in the tuple, found by structure rather
    than named, so this probe does not go stale when the catalogue's constants
    are reordered.
    """
    text = box.read("scripts/guards/catalogue.py")
    entries = list(re.finditer(
        r"^    Constant\(\"[\w.-]+\", \"([^\"]+)\", \"(\w+)\",.*?\n"
        r"(?=^    Constant\(|^\)$)", text, re.M | re.S))
    if not entries:
        raise AssertionError(
            "the catalogue declares no Constant this probe can retire, so the "
            "ratchet on the ratchet cannot be exercised.")
    last = entries[-1]
    box.substitute("scripts/guards/catalogue.py", last.group(0), "")
    budget = json.loads(box.read("ci/guard-probe-budget.json"))
    key = f"{last.group(1)}:{last.group(2)}"
    if key not in budget.get("constants", {}):
        raise AssertionError(
            f"{key} is declared in CONSTANTS and has no recorded row, so this "
            f"probe would leave an orphan-row refusal rather than the "
            f"narrowing it is about.")
    del budget["constants"][key]
    box.write("ci/guard-probe-budget.json",
              json.dumps(budget, indent=2, sort_keys=True) + "\n")


def _an_unclaimed_executable_hook(box: Sandbox) -> None:
    """Add an executable file under `.githooks/` with no catalogue entry.

    A hook is a refusal a clone that opts in actually runs, so one nobody
    declared is one nobody probed. The branch that says so was added by the
    S03 review's fourth pass and deleting it left the census, the floor probe
    profile and the unit suite all green.

    The body carries no refusal shape, so the state this builds is exactly an
    undeclared HOOK and not an undeclared refusal site, which check a already
    covers.
    """
    box.write(".githooks/probe-hook", "#!/bin/sh\nexit 0\n").chmod(0o755)


def _the_oracle_runner_is_gone(box: Sandbox) -> None:
    """Delete `tools/oracle/run.mjs`, the runner that replays the faults.

    The adoption check used to read `if runner.is_file() and ...`, which failed
    OPEN: deleting the runner skipped the branch entirely, so removing it was
    quieter than breaking it while the same loss of `faults.mjs` three lines
    above was refused. The fourth pass closed that and nothing watched the
    close.
    """
    box.delete("tools/oracle/run.mjs")


def _an_unrecognised_guard_kind(box: Sandbox) -> None:
    """Mistype one entry's `kind`, which decides which bucket it lands in.

    An unrecognised kind is not `"guard"`, so the entry's refusals leave the
    uncovered ratchet and are reported as declared out of scope. A typo is
    enough, the validation that says so was added by the fourth pass, and
    deleting that validation left every count here unchanged.

    The line is found by SHAPE, an eight-space-indented `kind=` field, rather
    than by the value it holds. Searching for the value found this function's
    own argument first and mutated the probe instead of a catalogue entry, so
    the census exited 0 and the harness reported the guard silent.
    """
    text = box.read("scripts/guards/catalogue.py")
    field = re.search(r'^ {8}kind="[a-z-]+",$', text, re.M)
    if field is None:
        raise AssertionError(
            "no catalogue entry declares a `kind` field on its own line, so "
            "this probe cannot mistype one.")
    box.substitute("scripts/guards/catalogue.py", "\n" + field.group(0) + "\n",
                   '\n        kind="probe-typo",\n')


def _drop_the_entry_site_counts(box: Sandbox) -> None:
    """Remove the recorded per-entry site counts from the budget."""
    _budget_drop(box, "entry_sites")


def _a_refusal_in_an_already_claimed_file(box: Sandbox) -> None:
    """Add a refusal to a file a catch-all catalogue entry already claims.

    This is the gap the fifth pass named. Check a makes a guard added next
    month arrive with its test, and that rule did not apply to a guard added to
    a file the catalogue already claims: most entries claim their file with
    `"*"`, so a new `problems.append` lands in the probed bucket and no number
    moves. The pass measured it from the other side, on this harness's own
    census module, where four new refusal branches could each be deleted with
    everything staying green.

    The file is chosen from the catalogue's own catch-all entries rather than
    named, and the refusal is appended in a function nothing calls, so the
    guard it is added to still behaves exactly as before.
    """
    text = box.read("scripts/guards/catalogue.py")
    for match in re.finditer(r'file="(scripts/[\w/]+\.py)",\s*\n\s*gate=',
                             text):
        target = match.group(1)
        if not (box.path / target).is_file():
            continue
        if 'claims=("*",)' not in text[match.end():match.end() + 600]:
            continue
        box.append(target,
                   "\n\ndef _probe_added_refusal(problems):\n"
                   "    problems.append("
                   "'a refusal added to a file the catalogue already "
                   "claims')\n")
        return
    raise AssertionError(
        "no catalogue entry claims a Python guard under scripts/ with a "
        "catch-all, so the gap this probe is about cannot be built.")


def _lint_policy_weakened(box: Sandbox) -> None:
    box.substitute("Cargo.toml", 'cast_possible_truncation = "deny"',
                   'cast_possible_truncation = "allow"')


def _lint_policy_uninherited(box: Sandbox) -> None:
    box.substitute("crates/ocelli-pixel/Cargo.toml",
                   "[lints]\nworkspace = true", "")


def _a_crate_root(box: Sandbox) -> str:
    """The first crate root under `crates/`, read rather than named.

    A crate ROOT specifically, because `#![allow(...)]` there governs the whole
    crate and that is the widest form of the bypass.
    """
    for crate in sorted((box.path / "crates").iterdir()):
        lib = crate / "src" / "lib.rs"
        if lib.is_file():
            return f"crates/{crate.name}/src/lib.rs"
    raise AssertionError("no crate carries a src/lib.rs, so there is no crate "
                         "root to plant an inner attribute in")


def _a_crate_module_file(box: Sandbox) -> str:
    """A `.rs` file in a crate that is NOT the crate root.

    The second half of the bypass. `#![allow(...)]` at the top of a module file
    applies to that module, measured with cargo under 1.97.1, and the guard
    read `src/lib.rs` and nothing else.
    """
    for crate in sorted((box.path / "crates").iterdir()):
        for path in sorted((crate / "src").glob("*.rs")):
            if path.name != "lib.rs":
                return f"crates/{crate.name}/src/{path.name}"
    raise AssertionError("every crate is a single lib.rs, so there is no "
                         "module file for this probe to plant one in")


def _prepend(box: Sandbox, rel: str, line: str) -> None:
    box.write(rel, line + "\n" + box.read(rel))


def _group_allow_at_a_crate_root(box: Sandbox) -> None:
    """`#![allow(clippy::pedantic)]`, which names none of HLD 27.1's lints.

    The input is HLD 27.1's table read together with clippy's own group
    membership, and the membership was MEASURED rather than read off the
    guard's list. On a minimal crate carrying `cast_possible_truncation =
    "deny"` and one `x as i32`, under the pinned 1.97.1 toolchain, cargo clippy
    exits 101 without this attribute and 0 with it. So the `clippy` gate goes
    green with the arithmetic denies switched off, which is the defect class
    CLAUDE.md names as the one that reaches patients.
    """
    _prepend(box, _a_crate_root(box), "#![allow(clippy::pedantic)]")


def _expect_attribute_at_a_crate_root(box: Sandbox) -> None:
    """`#![expect(...)]`, the RFC 2383 form, which silences a lint like allow.

    This was a live bypass in the first version of the group-allow fix, which
    matched `allow` and nothing else. Measured on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`, under the pinned
    1.97.1 toolchain: no attribute exits 101,
    `#![expect(clippy::cast_possible_truncation)]` exits 0 and
    `#![expect(clippy::pedantic)]` exits 0. Both the named route and the group
    route were open, and `expect` reaches `pedantic` where `allow` does not
    reach `warnings`, so it is not even the same shape of hole.
    """
    _prepend(box, _a_crate_root(box),
             "#![expect(clippy::cast_possible_truncation)]")


def _no_crate_sources_at_all(box: Sandbox) -> None:
    """Delete every `.rs` file in every workspace MEMBER.

    A scan that read nothing is not a scan that found nothing. The check
    printed `0 .rs file(s) carry no inner allow` and exited 0 over exactly
    this state, which is AGENTS.md's named failure of answering a question
    about an empty set in the language of success.

    Every member and not `crates/` alone, which is the fifth pass's finding
    arriving in this probe. With the walk widened to the manifest's members,
    emptying `crates/` left thirteen `.rs` files in `tools/oracle`, so the
    probe stopped building the state it is about and reported the guard
    silent, which is exactly what the harness's inverted success is for.
    """
    removed = 0
    for member in _member_directories(box):
        for path in sorted(member.rglob("*.rs")):
            if path.relative_to(member).parts[:1] == ("target",):
                continue
            path.unlink()
            removed += 1
    if removed == 0:
        raise AssertionError("no workspace member carries a .rs file already, "
                             "so this probe cannot create the state it is "
                             "about")


def _named_allow_outside_the_crate_root(box: Sandbox) -> None:
    """A denied lint re-allowed in a module file rather than in `lib.rs`.

    The lint name comes from HLD 27.1's table. The FILE is the point: an inner
    attribute in a module file governs that module, so reading only
    `src/lib.rs` answered a question about one file and said so by succeeding.
    """
    _prepend(box, _a_crate_module_file(box),
             "#![allow(clippy::cast_possible_truncation)]")


def _item_allow_with_a_reason(box: Sandbox) -> None:
    """The forms HLD 27.1's note PERMITS, at the expression that needs it.

    An outer `#[allow(...)]` on one item, which is the deliberate visible
    choice the note asks for. One character separates it from the refused
    inner form, so a guard that refused this would be refusing the remedy it
    recommends in its own message.

    FOUR item kinds since the S03 review's fifth pass, and that is the point.
    The probe planted its allow on a `fn` alone, so the guard's new refusal of
    an outer allow on a `mod` had nothing asserting it does not also refuse
    the other item kinds. A `fn`, a `struct`, an `impl` and a statement are
    each local in the way 27.1's note means and a `mod` is not.
    """
    box.append(
        _a_crate_module_file(box),
        "\n// HLD 27.1's note, the deliberate visible choice at the "
        "expression\n// that needs it. None of these is a `mod`.\n"
        "#[allow(clippy::cast_possible_truncation, reason = \"probe\")]\n"
        "pub fn probe_local_allow(x: i64) -> i32 {\n    x as i32\n}\n"
        "\n#[allow(clippy::cast_possible_truncation, reason = \"probe\")]\n"
        "pub struct ProbeLocalAllow(pub i32);\n"
        "\n#[allow(clippy::cast_possible_truncation, reason = \"probe\")]\n"
        "impl ProbeLocalAllow {\n"
        "    pub fn make(x: i64) -> i32 {\n        x as i32\n    }\n}\n"
        "\npub fn probe_local_statement(x: i64) -> i32 {\n"
        "    #[allow(clippy::cast_possible_truncation, reason = \"probe\")]\n"
        "    let narrowed = x as i32;\n    narrowed\n}\n")


def _workspace_members(box: Sandbox) -> list[str]:
    """The globs `Cargo.toml`'s `[workspace] members` declares, in order.

    Read from the manifest, which is the artefact the guard was wrong about.
    A literal `crates/*` here would be the very assumption that cost the
    workspace its fourteenth member.
    """
    block = re.search(r"^\[workspace\]$(.*?)(?=^\[|\Z)", box.read("Cargo.toml"),
                      re.M | re.S)
    listing = re.search(r"^\s*members\s*=\s*\[(.*?)\]", block.group(1),
                        re.M | re.S) if block else None
    if listing is None:
        raise AssertionError("Cargo.toml declares no [workspace] members, so "
                             "these probes have no member to work on.")
    return re.findall(r'"([^"]+)"', listing.group(1))


def _member_directories(box: Sandbox) -> list[Path]:
    """Every directory the manifest's `members` globs resolve to."""
    found: list[Path] = []
    for pattern in _workspace_members(box):
        hits = (sorted(box.path.glob(pattern)) if "*" in pattern
                else [box.path / pattern])
        found.extend(p for p in hits if (p / "Cargo.toml").is_file())
    if not found:
        raise AssertionError("Cargo.toml's members resolve to no directory "
                             "carrying a manifest")
    return found


def _a_crate_root_with_a_module(box: Sandbox) -> tuple[str, str]:
    """A crate root declaring a `mod` item, and that item's line.

    The FIRST crate root is not enough. Most crates here are a single file, so
    `_a_crate_root` returned one with no module in it and the probe could not
    build the state it is about, which the harness reported as a builder
    failure rather than as a pass.
    """
    for crate in sorted((box.path / "crates").iterdir()):
        lib = crate / "src" / "lib.rs"
        if not lib.is_file():
            continue
        module = re.search(r"^(?:pub )?mod [a-z_]+;$",
                           lib.read_text(encoding="utf-8"), re.M)
        if module is not None:
            return f"crates/{crate.name}/src/lib.rs", module.group(0)
    raise AssertionError(
        "no crate root declares a `mod` item, so there is no module for a "
        "probe to put an outer allow on.")


def _a_member_outside_crates(box: Sandbox) -> str:
    """A workspace member the old hard-coded `crates/` walk never reached.

    Derived from the manifest rather than named. `tools/oracle` is the one
    today, a compiled member with thirteen `.rs` files that was checked for
    neither `[lints] workspace = true` nor an inner allow.
    """
    for pattern in _workspace_members(box):
        if pattern.startswith("crates/") or "*" in pattern:
            continue
        if (box.path / pattern / "Cargo.toml").is_file():
            return pattern
    raise AssertionError(
        "every workspace member is under crates/, so there is no member "
        "outside the old walk for this probe to plant anything in. If that is "
        "now true of the repository, this probe has nothing to say and the "
        "hole it is about cannot be built.")


def _member_outside_crates_uninherited(box: Sandbox) -> None:
    """Drop `[lints] workspace = true` from a member outside `crates/`.

    Measured together with the group allow below: with both applied, the check
    exited 0 printing "13 crate(s) inherit the table, 33 .rs file(s)". The
    manifest says fourteen members.
    """
    member = _a_member_outside_crates(box)
    box.substitute(f"{member}/Cargo.toml", "[lints]\nworkspace = true", "")


def _member_outside_crates_group_allow(box: Sandbox) -> None:
    """`#![allow(clippy::pedantic)]` in a member outside `crates/`.

    The same attribute `lint-policy.group-allow` plants in a crate root, in
    the half of the workspace the walk never visited. Measured under the
    pinned 1.97.1 toolchain that the attribute silences four of HLD 27.1's
    five lints.
    """
    member = _a_member_outside_crates(box)
    sources = sorted(p for p in (box.path / member).rglob("*.rs")
                     if p.relative_to(box.path / member).parts[:1]
                     != ("target",))
    if not sources:
        raise AssertionError(
            f"{member} carries no .rs file, so this probe cannot plant an "
            f"attribute in the part of the workspace the walk missed.")
    _prepend(box, sources[0].relative_to(box.path).as_posix(),
             "#![allow(clippy::pedantic)]")


def _unresolvable_workspace_member(box: Sandbox) -> None:
    """Name a member that is not there.

    cargo refuses this workspace. The check walked the members it could
    resolve and reported the smaller number as a pass, which is
    `AGENTS.md`'s named failure of answering a question about a smaller set
    in the language of success.
    """
    patterns = _workspace_members(box)
    box.substitute("Cargo.toml", json.dumps(patterns),
                   json.dumps([*patterns, "tools/probe-absent-member"]))


def _no_workspace_members_at_all(box: Sandbox) -> None:
    """Delete the `members` key. Every check below then reads an empty set."""
    patterns = _workspace_members(box)
    box.substitute("Cargo.toml", f"members = {json.dumps(patterns)}", "")


def _whitespace_in_the_lint_path(box: Sandbox) -> None:
    """`#![allow(clippy :: pedantic)]`, which Rust reads as `clippy::pedantic`.

    MEASURED under the pinned 1.97.1 toolchain, on a crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32` in a module file:
    no attribute exits 101, this attribute exits 0, and `clippy:: pedantic`
    exits 0 too. The captured name carried its spaces into the set lookup and
    matched nothing, so the whole `REFUSED_GROUPS` list was one space away
    from being unreachable.
    """
    _prepend(box, _a_crate_root(box), "#![allow(clippy :: pedantic)]")


def _outer_allow_on_a_module(box: Sandbox) -> None:
    """`#[allow(...)]` on a `mod` item, which governs the whole module tree.

    Derived from HLD 27.1's note, which asks for a deliberate visible choice
    at the expression that needs it. A module item is not an expression, and
    the scope was MEASURED: with one `x as i32` in `src/inner.rs` and
    `src/lib.rs` reading
    `#[allow(clippy::cast_possible_truncation)] pub mod inner;`, cargo clippy
    goes from 101 to 0 under the pinned 1.97.1 toolchain. That is the same
    scope the guard already refused for an inner attribute, and the guard's
    stated reason for permitting the outer form was that it is local.
    """
    root, module = _a_crate_root_with_a_module(box)
    box.substitute(root, module,
                   "#[allow(clippy::cast_possible_truncation)]\n" + module)


def _group_row_in_the_workspace_table(box: Sandbox) -> None:
    """A group row in `[workspace.lints.clippy]`, in TOML's inline form.

    MEASURED under the pinned 1.97.1 toolchain: on a crate carrying
    `cast_possible_truncation = "deny"`, adding
    `pedantic = { level = "allow", priority = 1 }` beside it takes cargo
    clippy from 101 to 0, because the higher priority is applied last and the
    group wins. The row regex matched a quoted level only, so this was
    invisible and the check went on printing that all five lints were at or
    above HLD 27.1's level.
    """
    # HLD 27.1's first row, quoted here as `_lint_policy_weakened` quotes it.
    # The lint comes from the specification's table and not from the guard's
    # transcription of it.
    row = re.search(r'^cast_possible_truncation = "[a-z]+"$',
                    box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no `cast_possible_truncation` row in the "
            "quoted form, so this probe cannot add a group row beside one.")
    box.substitute("Cargo.toml", row.group(0),
                   row.group(0) +
                   '\npedantic = { level = "allow", priority = 1 }')


def _last_lint_table_row(box: Sandbox) -> str:
    """The last `name = value` line of `[workspace.lints.clippy]`.

    Read from the manifest rather than named, so the two probes below plant
    their row at the END of the table wherever the table ends today. A blank
    line does not close a TOML table, which is the half of the sixth pass's
    bypass the declared-constant capture missed.
    """
    region = re.search(r"^\[workspace\.lints\.clippy\]\n((?:(?!^\[)[\s\S])*)",
                       box.read("Cargo.toml"), re.M)
    if region is None:
        raise AssertionError(
            "Cargo.toml carries no [workspace.lints.clippy] table, so there "
            "is no row for this probe to plant one after.")
    rows = re.findall(r"^[\w-]+\s*=[^\n]*$", region.group(1), re.M)
    if not rows:
        raise AssertionError(
            "[workspace.lints.clippy] carries no `name = value` row, so this "
            "probe cannot find the end of the table.")
    return rows[-1]


def _group_row_after_a_blank_line_with_a_trailing_comment(
        box: Sandbox) -> None:
    """The fifth route past the lint policy, and it took the whole gate.

    `LINT_ROW` anchored on `\\s*$` and a TOML trailing comment is not
    whitespace, so the row was invisible to the guard. Put it after a blank
    line and it was invisible to the declared-constant capture too, which
    stopped at the first `\\n\\n`. MEASURED under the pinned 1.97.1 toolchain
    on a minimal crate carrying `cast_possible_truncation = "deny"` and one
    `x as i32`: cargo clippy exits 101, and with
    `pedantic = { level = "allow", priority = 1 } # keeps noise down` appended
    it exits 0. In this repository the same row after a blank line left
    `lint_policy_check.py` at exit 0 printing "no group row weaker than deny",
    `guard_census.py` at exit 0 and `bin/ocelli.sh gate guards` ALL GREEN,
    with four of HLD 27.1's five lints off.
    """
    last = _last_lint_table_row(box)
    box.substitute(
        "Cargo.toml", last,
        last + '\n\npedantic = { level = "allow", priority = 1 }'
               ' # keeps noise down')


def _required_row_with_a_trailing_comment(box: Sandbox) -> None:
    """A trailing comment on a REQUIRED row, which weakens nothing.

    The accept direction of the same regex, and the direction that says which
    fix was made. Stripping the comment before matching is not the same as
    loosening the anchor to `.*$`, and only this probe can tell the two apart:
    under the old anchor the row read as ABSENT and the guard refused, naming
    a lint that is present at exactly the level 27.1 asks for. A repair that
    made the guard tolerate the comment by ignoring the row body would pass
    the group probe above and fail here.
    """
    row = re.search(r'^cast_possible_truncation = "[a-z]+"$',
                    box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no `cast_possible_truncation` row in the "
            "quoted form, so this probe cannot comment one.")
    box.substitute("Cargo.toml", row.group(0),
                   row.group(0) + "  # HLD 27.1, and this comment is not a "
                                  "weakening")


def _exclude_a_named_workspace_member(box: Sandbox) -> None:
    """`exclude` naming an explicitly listed member, with a group allow in it.

    cargo does not apply `exclude` to a member `members` names outright.
    MEASURED on this workspace under the pinned 1.97.1 toolchain: with
    `members = ["crates/*", "tools/oracle"]` and `exclude = ["tools/oracle"]`,
    `cargo metadata --no-deps` reports 14 packages with the oracle among them
    and clippy compiles it. The guard dropped it and printed "13 workspace
    member(s) ... 33 .rs file(s)", which is the pair of numbers its own header
    records as the fifth pass's defect, reached through a different key. So
    the group allow planted here was in a member the walk no longer visited.
    """
    member = _a_member_outside_crates(box)
    patterns = _workspace_members(box)
    box.substitute("Cargo.toml", f"members = {json.dumps(patterns)}",
                   f"members = {json.dumps(patterns)}\n"
                   f"exclude = {json.dumps([member])}")
    _member_outside_crates_group_allow(box)


def _ci_arm_commands(box: Sandbox, gate: str) -> list[str]:
    """A gate's arm commands, read through the guard's own runner parser.

    Reading `bin/ocelli.sh` is reading the ARTEFACT this guard is about, which
    is the same move `_floor_gate_with_own_step` makes on `ci.yml`. The
    alternative, writing the `backlog` arm's two commands here, would be a
    second copy of the runner's own line inside this catalogue.

    Imported lazily rather than at module scope, because every caller of this
    catalogue already puts `scripts/` on `sys.path` and a top-level import here
    would make the catalogue depend on one particular guard.
    """
    import ci_floor_check
    return ci_floor_check.gate_commands(box.read("bin/ocelli.sh")).get(gate, [])


def _a_floor_gate_run_command_by_command(box: Sandbox) -> tuple[str, list[str]]:
    """A floor gate CI runs as its separate commands rather than by name.

    Returns the gate and the `ci.yml` lines running each of its arm commands.
    Only a gate with two or more such lines can show the defect, because the
    hole was that running ONE of several counted as running the gate.
    """
    workflow = box.read(".github/workflows/ci.yml")
    named = set(re.findall(r"bin/ocelli\.sh gate ([a-z-]+)", workflow))
    for gate in sorted(set(re.findall(r'^\s*"([a-z-]+)\|no\|',
                                      box.read("bin/ocelli.sh"), re.M))):
        if gate in named:
            continue
        arm = [c.strip() for c in _ci_arm_commands(box, gate)]
        lines = [line for line in workflow.splitlines()
                 if any(c and c in line for c in arm)]
        if len(arm) >= 2 and len(lines) >= 2:
            return gate, lines
    raise AssertionError(
        "no floor gate has two or more arm commands that ci.yml runs as "
        "separate steps, so there is no partial invocation to build and this "
        "probe would report its guard silent.")


def _delete_one_command_of_a_gate(box: Sandbox) -> None:
    """Delete ONE step of a multi-command gate and leave the rest.

    The reviewer's measurement: with `gen_sprint_plan.py --check` deleted the
    check exited 0, with `backlog_check.py` deleted instead it exited 0, and
    only deleting both made it exit 1. So the estimate comparison could be
    removed from every pull request by deleting one line.
    """
    _, lines = _a_floor_gate_run_command_by_command(box)
    box.substitute(".github/workflows/ci.yml", lines[-1], "")


def _one_step_for_the_whole_arm(box: Sandbox) -> None:
    """Replace a gate's several steps with one `bin/ocelli.sh gate <name>`.

    The accept direction, and it is why the rule is not simply "every command
    must appear". A step naming the gate runs its whole arm by definition, and
    a check that demanded the commands as well would refuse the arrangement
    `ci.yml` already uses for `errors`, `bench`, `packages` and `guards`.
    """
    gate, lines = _a_floor_gate_run_command_by_command(box)
    indent = " " * (len(lines[0]) - len(lines[0].lstrip()))
    box.substitute(".github/workflows/ci.yml", lines[0],
                   f"{indent}- run: bin/ocelli.sh gate {gate}")
    for line in lines[1:]:
        box.substitute(".github/workflows/ci.yml", line, "")


def _a_non_floor_gate_ci_runs(box: Sandbox) -> str:
    """A gate outside the floor that needs no GPU and that ci.yml runs by name.

    Both facts are read from the repository. The exclusion comes from
    `NOT_IN_FLOOR`, which is in the declared-constant ratchet, and the GPU
    column comes from `bin/ocelli.sh`'s own GATES table, which is what says
    `oracle` is the one excluded gate CI may not run.
    """
    excluded = _not_in_floor(box)
    runner = box.read("bin/ocelli.sh")
    gpu = set(re.findall(r'^\s*"([a-z-]+)\|YES\|', runner, re.M))
    workflow = box.read(".github/workflows/ci.yml")
    for match in re.finditer(r"bin/ocelli\.sh gate ([a-z-]+)", workflow):
        name = match.group(1)
        if name in excluded and name not in gpu:
            return name
    raise AssertionError(
        "no CI step invokes a gate that is outside the floor and needs no "
        "GPU, so there is no such step for this probe to delete.")


def _delete_the_non_floor_ci_step(box: Sandbox) -> None:
    """Delete the CI step running a non-floor gate, job and comments intact.

    The reviewer deleted the whole `guards-deep` job and this check, the census
    and the floor probes all exited 0. This is the smaller version of that: the
    job, its name and the comment explaining its trigger all stay, and only the
    `run:` line goes, so a reader of `ci.yml` still sees the gate named and
    nothing runs it.
    """
    gate = _a_non_floor_gate_ci_runs(box)
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    box.substitute(".github/workflows/ci.yml", line, "")


def _runner_exclusion_list(box: Sandbox) -> tuple[str, list[str]]:
    """The runner's `--floor` exclusion line and the names on it, in order.

    In ORDER. The set is what matters to the guard and the order is what
    matters to a text substitution, and sorting the names before searching for
    them is how the first version of the two probes below failed to find a
    line that was right in front of it.
    """
    line = next(l for l in box.read("bin/ocelli.sh").splitlines()
                if 'case "$name" in' in l and "continue" in l)
    names = re.findall(r"[a-z-]+", line.split(" in ", 1)[1].split(")", 1)[0])
    if not names:
        raise AssertionError(
            "bin/ocelli.sh's --floor arm excludes no gate by name, so there "
            "is no list for these probes to disagree with.")
    return line, names


def _exclude_a_floor_gate_in_the_runner_only(box: Sandbox) -> None:
    """Add a floor gate to `bin/ocelli.sh`'s exclusion list and nowhere else.

    The reviewer added `prose` to the shell list alone: `ci_floor_check.py`
    exited 0, `gate --floor` silently stopped running `prose`, and the net the
    runner's comment claimed caught nothing. The gate here is read from the
    runner's own GATES table rather than named, so the probe stays true when
    the table changes.
    """
    _, names = _runner_exclusion_list(box)
    gate = next(g for g in re.findall(r'^\s*"([a-z-]+)\|no\|',
                                      box.read("bin/ocelli.sh"), re.M)
                if g not in names)
    box.substitute("bin/ocelli.sh", f"in {'|'.join(names)})",
                   f"in {'|'.join([*names, gate])})")


def _a_floor_gate_with_no_arm_command(box: Sandbox) -> str:
    """A floor gate CI names whose arm yields no extractable command.

    `ci_floor_check.py`'s own claim is that such a gate can only be satisfied
    by a step naming it, because `all([])` is true and a vacuous pass would be
    the widest hole in the file. The claim was FALSE for `panic`: an arm ended
    at the next case LABEL rather than at its `;;`, so it swallowed the comment
    block introducing the following arm and the command regex read commands out
    of prose.
    """
    excluded = _not_in_floor(box)
    workflow = box.read(".github/workflows/ci.yml")
    for gate in re.findall(r'^\s*"([a-z-]+)\|no\|', box.read("bin/ocelli.sh"),
                           re.M):
        if gate in excluded or _ci_arm_commands(box, gate):
            continue
        if f"bin/ocelli.sh gate {gate}" in workflow:
            return gate
    raise AssertionError(
        "every floor gate CI names by gate has an extractable arm command, so "
        "there is no gate whose only route to CI is the name, and this probe "
        "cannot build the state it is about.")


def _an_automatic_event(box: Sandbox) -> str:
    """One event the workflow declares that a change triggers.

    Read through the guard's own reader, because the floor's claim is about the
    events `ci.yml` declares and this catalogue may not hold a second copy of
    that list. Two or more are needed: a condition naming the only automatic
    event blocks nothing, so the state this feeds could not be built.
    """
    import ci_floor_check
    events = sorted(ci_floor_check.workflow_events(
        box.read(".github/workflows/ci.yml")) - ci_floor_check.MANUAL_EVENTS)
    if len(events) < 2:
        raise AssertionError(
            f"the workflow declares {len(events)} automatic event(s), so a "
            f"condition naming one of them excludes no other and this probe "
            f"cannot build the state it is about.")
    return events[0]


def _a_no_arm_command_gate_behind_a_condition(box: Sandbox) -> None:
    """Keep a no-arm-command gate's naming step and gate it to one event.

    This is what watches `covers`'s `bool(arm)` clause, and until the S03
    review's sixth pass nothing did. `ci-floor.gate-with-no-arm-command` DELETES
    the naming step, so `steps_running` comes back empty and the refusal arrives
    from the final "nothing in ci.yml runs it" branch without `covers` ever
    being consulted: dropping `bool(arm) and` left that probe green.

    Here the step stays, so `steps_running` is not empty and the per-event
    question is the one that decides. On the event the condition excludes the
    gate is not named, its arm yields no extractable command, and `bool(arm)`
    is the only thing between that and `all([])` being true. MEASURED: with the
    clause the check refuses at "is behind a condition that does not run it on",
    and with `bool(arm) and` dropped that sentence is gone.

    `ci-floor.event-gated` happens to reach the same clause today, because
    `_floor_gate_with_own_step` returns the first gate `ci.yml` names and that
    gate's arm is empty. That is an accident of ordering rather than a
    selection, and a probe whose discrimination depends on one is a probe that
    stops discriminating the day a step moves. This one selects for the
    property by name.
    """
    _put_gate_step_behind(box, _a_floor_gate_with_no_arm_command(box),
                          f"github.event_name == '{_an_automatic_event(box)}'")


def _a_gate_with_an_unextractable_arm_command(box: Sandbox) -> str:
    """A floor gate CI names whose arm runs something the extractor cannot see.

    And that also has commands the extractor CAN see, because the bypass this
    feeds is expanding a gate-name step into those, which is an edit that reads
    as making CI more explicit. `bench` is the one today: two `python3`
    commands the extractor reads and one `node --test` line carrying five
    suites that it does not.
    """
    import ci_floor_check
    runner = box.read("bin/ocelli.sh")
    unseen = ci_floor_check.unseen_commands(runner)
    visible = ci_floor_check.gate_commands(runner)
    excluded = _not_in_floor(box)
    workflow = box.read(".github/workflows/ci.yml")
    for gate in re.findall(r'^\s*"([a-z-]+)\|no\|', runner, re.M):
        if gate in excluded or gate not in unseen or not visible.get(gate):
            continue
        if f"bin/ocelli.sh gate {gate}" in workflow:
            return gate
    raise AssertionError(
        "no floor gate CI names by gate has both an extractable arm command "
        "and one the extractor cannot see, so the state this probe is about "
        "cannot be built.")


def _expand_a_gate_step_into_its_visible_commands(box: Sandbox) -> None:
    """Replace a gate-name step with the arm commands the extractor can read.

    The measured bypass, in full. `COMMAND_PREFIXES` recognises `python3 `,
    `npm run `, `cargo ` and `ci/`, so the `node --test` line in the `bench`
    arm is invisible. Replacing `- run: bin/ocelli.sh gate bench` with its two
    `python3` commands left `ci_floor_check.py` at exit 0 printing "every
    command in each gate's arm", and five node test files left CI in a two-line
    edit that reads as expanding the step. The docstring's declaration that
    nothing was lost, because these gates "are each invoked by NAME in ci.yml",
    was enforced by nothing.
    """
    gate = _a_gate_with_an_unextractable_arm_command(box)
    commands = _ci_arm_commands(box, gate)
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(".github/workflows/ci.yml", line,
                   "\n".join(f"{indent}- run: {command}"
                             for command in commands))


def _a_command_a_runner_comment_names(box: Sandbox) -> str:
    """A command string that appears inside a `run_gate` COMMENT.

    This is the class of string the old arm parser leaked into the arm before
    it. `npm run test` reached `arms['panic']` from the `bench` comment
    block's sentence about `npm run test:browser`, and that command appears
    nowhere in the `panic` arm. Adding a step that runs it is a legitimate
    change to `ci.yml` and it must not satisfy an unrelated gate.
    """
    runner = box.read("bin/ocelli.sh")
    body = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    for line in body.splitlines():
        if "#" not in line:
            continue
        found = re.findall(r"(?:python3 |npm run |cargo |ci/)[\w./ -]+",
                           line.split("#", 1)[1])
        if found:
            return re.sub(r"\s+", " ", found[0]).strip()
    raise AssertionError(
        "no comment inside run_gate names a command, so the string the old "
        "parser leaked cannot be reconstructed and this probe would build a "
        "state that is not the one it is about.")


def _swap_a_named_gate_step_for_an_unrelated_command(box: Sandbox) -> None:
    """Delete the step naming a no-command gate and add a legitimate one.

    The measured bypass, in full. Deleting the `bin/ocelli.sh gate panic` step
    alone already refused. Adding a real `- run: npm run test` step beside it
    returned the check to exit 0 with "all 25 floor gate(s) are invoked by CI",
    because the arm parser had read that very string out of the comment block
    beneath the arm. `panic` is HLD section 23's wasm panic-hook proof, the one
    property no native test can observe.
    """
    gate = _a_floor_gate_with_no_arm_command(box)
    command = _a_command_a_runner_comment_names(box)
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(".github/workflows/ci.yml", line,
                   f"{indent}- run: {command}")


def _a_gate_whose_arm_names_a_test_suite(box: Sandbox) -> tuple[str, str]:
    """A floor gate CI names by gate, and one arm command carrying `-p`.

    `-p <suite>` is what a `unittest discover` command narrows itself with, and
    it sat after a `\\` continuation, which the extraction class stopped at. So
    the recorded arm command was the discovery root alone and `runs_command`
    matched any unittest step at all.
    """
    excluded = _not_in_floor(box)
    workflow = box.read(".github/workflows/ci.yml")
    for gate in re.findall(r'^\s*"([a-z-]+)\|no\|', box.read("bin/ocelli.sh"),
                           re.M):
        if gate in excluded:
            continue
        if f"bin/ocelli.sh gate {gate}" not in workflow:
            continue
        for command in _ci_arm_commands(box, gate):
            if " -p " in command:
                return gate, command
    raise AssertionError(
        "no floor gate CI names by gate has an arm command carrying `-p`, so "
        "there is no narrowed discovery for this probe to widen.")


def _replace_a_gate_step_with_a_narrowed_arm(box: Sandbox) -> None:
    """Run a gate's arm command by command, with a suite that finds nothing.

    Every command of the arm is present and one of them discovers zero tests.
    Under the parser that dropped everything after a `\\` continuation the
    recorded command was `python3 -B -m unittest discover -s scripts/tests`,
    `runs_command` searches rather than matches, and this exited 0.
    """
    gate, narrowed = _a_gate_whose_arm_names_a_test_suite(box)
    commands = _ci_arm_commands(box, gate)
    suite = narrowed.split(" -p ", 1)[1].strip()
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    steps = "\n".join(
        f"{indent}- run: "
        f"{command.replace(suite, 'test_nothing_at_all.py')}"
        for command in commands)
    box.substitute(".github/workflows/ci.yml", line, steps)


def _reformat_the_runner_exclusion_list(box: Sandbox) -> None:
    """Rewrite the `--floor` exclusion as one case arm per name.

    A cosmetic reformat that changes nothing about which gates the floor runs,
    and that the exclusion parser cannot read. It is refused, which is right,
    and until the fifth pass it arrived as a bare `RuntimeError` traceback with
    no `FAIL:` header, so the refusal could not be acted on by the person who
    made the edit.
    """
    line, names = _runner_exclusion_list(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(
        "bin/ocelli.sh", line.strip(),
        'case "$name" in\n'
        + "".join(f"{indent}  {name}) continue ;;\n" for name in names)
        + f"{indent}esac")


def _hide_the_run_gate_region(box: Sandbox) -> None:
    """Rename `run_gate`, so the arm parser cannot find the region at all."""
    box.substitute("bin/ocelli.sh", "run_gate() {", "run_one_gate() {")


def _empty_the_run_gate_region(box: Sandbox) -> None:
    """Leave the region markers and put no case arm between them.

    The other half of the same refusal. A region this parser can find and read
    no arm out of would give every gate an empty arm, `covers` would fall back
    to the gate-name route for all of them, and the per-command rule would hold
    over nothing.
    """
    box.substitute("bin/ocelli.sh", "run_gate() {",
                   "run_gate() {\nskip() {\n")


def _nested_case_in_a_gate_arm(box: Sandbox) -> None:
    """Add a nested `case` after a gate arm's command, inside the same arm.

    `ARM` ends an arm at its `;;` and a nested `case` ends its own branches the
    same way, so the arm parser stops at the FIRST inner `;;` and keeps only
    what came before it. Here that is a real command CI runs, so every command
    the parser can see is accounted for and the rest of the arm is dropped
    without a word. The shape is legal shell and does nothing, which is the
    point: the loss is in the parser and not in the runner.

    The gate is chosen for the property rather than named: a single-line arm
    whose extractable command `ci.yml` runs, so that WITHOUT the refusal this
    exits 0 rather than refusing for an unrelated reason.
    """
    runner = box.read("bin/ocelli.sh")
    workflow = box.read(".github/workflows/ci.yml")
    body = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    for line in body.splitlines():
        match = re.match(r"^([ \t]*)([a-z-]+)\)(.*);;\s*$", line)
        if match is None or "case" in match.group(3):
            continue
        commands = _ci_arm_commands(box, match.group(2))
        if not commands or not all(c in workflow for c in commands):
            continue
        box.substitute(
            "bin/ocelli.sh", line,
            f"{match.group(1)}{match.group(2)}){match.group(3).rstrip()}\n"
            f"{match.group(1)}             case \"$OSTYPE\" in *) : ;; esac ;;")
        return
    raise AssertionError(
        "no gate has a single-line arm whose every extractable command "
        "appears in ci.yml, so this probe cannot build an arm whose tail is "
        "dropped silently and would refuse for an unrelated reason.")


def _reorder_the_runner_exclusion_list(box: Sandbox) -> None:
    """Rewrite the runner's exclusion list in a different order, same names.

    The accept direction. The two lists are a SET written twice, so a check
    that compared them as text would refuse a reordering that changes nothing,
    and this repository has already paid once for a positional read of a Cargo
    entry (probe `pins.table-form`).
    """
    _, names = _runner_exclusion_list(box)
    if len(names) < 2:
        raise AssertionError("the runner excludes fewer than two gates, so "
                             "there is no order to change")
    box.substitute("bin/ocelli.sh", f"in {'|'.join(names)})",
                   f"in {'|'.join(reversed(names))})")


# ---------------------------------------------------------------------------
# The catalogue
# ---------------------------------------------------------------------------

GUARDS: tuple[Guard, ...] = (

    # -- patient data and derived pictures of it ---------------------------
    Guard(
        id="content",
        file="scripts/staged_content_check.py",
        gate="content",
        spec="CLAUDE.md hard rule, and corpus/README.md",
        refuses="Any staged or tracked DICOM, by magic bytes as well as by "
                "suffix and with no allowlist, any picture or buffer derived "
                "from a corpus row, any build artefact, and any oversized "
                "file.",
        claims=("*",),
        probes=(
            Probe("content.dicom-magic", _dicom_at("probe/anon001"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "this is DICOM",
                  note="Magic bytes, no suffix. `anon001` is a very normal "
                       "way to receive one."),
            Probe("content.dicom-suffix",
                  _text_at("probe/fake.dcm", "not DICOM at all\n"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "this is DICOM",
                  note="The suffix clause. The content is not DICOM and the "
                       "refusal is still correct."),
            Probe("content.no-allowlist",
                  _dicom_at("docs/examples/reference.dcm"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "There is no allowlist",
                  note="A path somebody would plausibly want exempted. The "
                       "third clause of the hard rule."),
            Probe("content.oracle-output",
                  _text_at("tools/oracle/out/probe.png", "PNG probe\n"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "oracle output"),
            Probe("content.compare-output",
                  _text_at("tools/oracle/compare-out/probe.diff.raw", "raw\n"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "comparator output"),
            Probe("content.spike-output",
                  _text_at("tools/spikes/out/probe.j2c", "codestream\n"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "spike output"),
            Probe("content.build-artefact",
                  _text_at("packages/core/dist/probe.js", "built\n"),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "build artefact"),
            Probe("content.size-limit",
                  lambda box: _stage(box, "probe/large.bin",
                                     b"\x01" * (3 * 1024 * 1024)),
                  script("python3", "scripts/staged_content_check.py",
                         "--tracked"),
                  "byte limit"),
        ),
    ),

    # -- HLD 27.2 R5 --------------------------------------------------------
    Guard(
        id="unsafe",
        file="scripts/unsafe_allowlist_check.py",
        gate="unsafe",
        spec="HLD 27.2 R5, `docs/hld/24-agent-code-standards.md`",
        refuses="An `unsafe` keyword in any file other than the two the rule "
                "names, so that auditing every unsafe line means reading two "
                "files.",
        claims=("*",),
        probes=(
            Probe("unsafe.third-file", _unsafe_in_a_third_file,
                  script("python3", "scripts/unsafe_allowlist_check.py"),
                  "`unsafe` outside the allow-list (HLD section 27.2 R5)",
                  note="The probe appends to a crate that is neither of the "
                       "two R5 names. It deliberately does NOT test "
                       "`is_unsafe` or `unsafe` in a doc comment, which are "
                       "facts about the current regex rather than about R5. "
                       "The fragment is the refusal HEADER and not the bare "
                       "words `outside the allow-list`, which the guard's OK "
                       "line also contains: with an accept probe that would "
                       "have been a false green, and here it was kept from "
                       "mattering only by the exit-status check running "
                       "first."),
        ),
    ),

    # -- voice --------------------------------------------------------------
    Guard(
        id="prose",
        file="scripts/prose_check.py",
        gate="prose",
        spec="CLAUDE.md, no em-dash and no prose semicolon in tracked prose",
        refuses="An em-dash or a prose semicolon in the operator-facing prose "
                "this project writes, and in a commit message.",
        claims=("*",),
        probes=(
            Probe("prose.em-dash", _prose_em_dash,
                  script("python3", "scripts/prose_check.py"), "em-dash"),
            Probe("prose.semicolon", _prose_semicolon,
                  script("python3", "scripts/prose_check.py"),
                  "semicolon in prose"),
            Probe("prose.commit-message", _prose_commit_message,
                  script("python3", "scripts/prose_check.py", "--commit-msg",
                         "probe-message.txt"),
                  "semicolon in prose",
                  note="The commit-message path is a different reader over "
                       "the same rules, and .githooks/commit-msg is the only "
                       "thing that calls it."),
        ),
    ),

    # -- the deviation register --------------------------------------------
    Guard(
        id="deviations",
        file="scripts/deviation_check.py",
        gate="deviations",
        spec="HLD Part II opening, and `docs/hld/DEVIATIONS.md`",
        refuses="A plan or review citing a deviation the register does not "
                "carry, and a register row whose claim about Cargo.toml has "
                "stopped being true.",
        claims=("*",),
        probes=(
            Probe("deviations.undeclared-citation",
                  _undeclared_deviation_citation,
                  script("python3", "scripts/deviation_check.py"),
                  "which is not a row",
                  note="The token is read from the register at probe time, "
                       "which is the only place it is safe to write one. A "
                       "design plan carrying an undeclared token is refused "
                       "by this very guard."),
            Probe("deviations.stale-d01",
                  lambda box: box.substitute("Cargo.toml",
                                             'rust-version = "1.97.1"',
                                             'rust-version = "1.90.0"'),
                  script("python3", "scripts/deviation_check.py"),
                  "records rust-version 1.97.1"),
            Probe("deviations.stale-resolver",
                  lambda box: box.substitute("Cargo.toml", 'resolver = "3"',
                                             'resolver = "2"'),
                  script("python3", "scripts/deviation_check.py"),
                  "records resolver 3"),
        ),
    ),

    # -- source policy ------------------------------------------------------
    Guard(
        id="provenance",
        file="scripts/source_provenance_check.py",
        gate="provenance",
        spec="`docs/SOURCE-POLICY.md`, HLD Appendix C.2.1 and 27.2 R7",
        refuses="A project the policy marks unreadable appearing as a "
                "dependency, as a URL, or named anywhere without a statement "
                "that it must not be used.",
        claims=("*",),
        probes=(
            Probe("provenance.dependency", _blocked_dependency,
                  script("python3", "scripts/source_provenance_check.py"),
                  "depends on",
                  note="The name comes from the policy table's Read column, "
                       "not from the checker's constant."),
            Probe("provenance.unqualified-mention", _blocked_mention,
                  script("python3", "scripts/source_provenance_check.py"),
                  "with no statement that it is out of bounds"),
        ),
        limit="The URL clause has no probe, deliberately. "
              "`docs/SOURCE-POLICY.md` names the projects and does not name "
              "their addresses, so a probe would have to take its input from "
              "the guard's own URL list, which is the R2 failure this "
              "catalogue exists to avoid: it would assert the current regex "
              "and pass forever once the regex was weakened. The list itself "
              "is a declared constant in the ratchet, so a change to it is "
              "caught there and lands in front of a reviewer.",
    ),

    # -- pins and the size ceiling -----------------------------------------
    Guard(
        id="pins",
        file="scripts/pin_and_size_check.py",
        gate="pins",
        spec="HLD 15.2 and 27.2 R4, and story E1.2 for the ceiling",
        refuses="A range where the specification requires an exact `=` pin, "
                "an `=` in front of a partial version or a second comparator "
                "after it, a pinned crate that has left the workspace table, "
                "and a wasm module over its recorded ceiling.",
        claims=("*",),
        probes=(
            Probe("pins.range", _relax_wgpu_pin,
                  script("python3", "scripts/pin_and_size_check.py"),
                  "is a RANGE, not an exact pin"),
            Probe("pins.partial-version", _partial_wgpu_pin,
                  script("python3", "scripts/pin_and_size_check.py"),
                  "PARTIAL version",
                  note="`wgpu = \"=30\"` starts with `=` and is a range. "
                       "Measured with cargo rather than read: under it "
                       "`cargo update -p wgpu --precise 30.0.0` exits 0 and "
                       "the resolver leaves 30.0.1 in the lock, while "
                       "`=30.0.1` refuses 30.0.0 outright. This was the "
                       "residue of G-03 that the S03 review's third pass "
                       "measured, the guard having tested only that the spec "
                       "starts with `=`."),
            Probe("pins.absent", _drop_wgpu_entry,
                  script("python3", "scripts/pin_and_size_check.py"),
                  "is not declared in [workspace.dependencies]"),
            Probe("pins.size-ceiling", _oversize_wasm,
                  script("python3", "scripts/pin_and_size_check.py",
                         "--with-size"),
                  "byte ceiling",
                  note="Story E1.2's ceiling arithmetic, watched in the floor "
                       "with no wasm-pack. The probe writes a module of a "
                       "known length and a small recorded baseline."),
            Probe("pins.table-form", _reorder_wgpu_table,
                  script("python3", "scripts/pin_and_size_check.py"),
                  "pinned exactly",
                  polarity="accept",
                  note="The pin is UNCHANGED and still exact, only written in "
                       "the table form Cargo accepts. R4 is about the version "
                       "being exact and says nothing about the entry's shape, "
                       "so the guard must still find it. This was G-03 and "
                       "failed until S03: the version was read positionally "
                       "as the first quoted string in the entry, so a table "
                       "whose first value happened to start with `=` passed "
                       "with a caret range unread."),
        ),
    ),

    # -- the no_std posture -------------------------------------------------
    Guard(
        id="nostd",
        file="scripts/no_std_check.py",
        gate="nostd",
        spec="deviation D-09, and each crate's own "
             "`#![cfg_attr(not(test), no_std)]`",
        refuses="A crate declaring `no_std` whose resolved dependency graph "
                "reaches a `std` feature, and a crate quietly leaving the "
                "declared set.",
        claims=("*",),
        probes=(
            Probe("nostd.reaches-std", _glam_reaches_std,
                  script("python3", "scripts/no_std_check.py"),
                  "reaches a std feature", needs="cargo", profile="deep",
                  note="D-09's worked case, reverted. The obvious check, a "
                       "wasm32 build, exits 0 either way, which was measured "
                       "in S01."),
            Probe("nostd.none-declared", _drop_no_std_everywhere,
                  script("python3", "scripts/no_std_check.py"),
                  "no crate under crates/ declares no_std",
                  needs="cargo", profile="deep",
                  note="The backstop. It is the ONLY thing that fires when "
                       "the attribute goes away, and it needs every crate to "
                       "drop it at once."),
            Probe("nostd.loses-a-crate", _drop_no_std_from_one_crate,
                  script("python3", "scripts/no_std_check.py"),
                  "stopped declaring no_std",
                  needs="cargo", profile="deep",
                  defect="G-02",
                  note="D-09 is a claim about a SET of crates. A crate that "
                       "deletes the attribute leaves the set, and the guard "
                       "reports a smaller number rather than a problem. The "
                       "declared-constant ratchet in scripts/guard_census.py "
                       "is what catches this today."),
        ),
    ),

    # -- the floor's own claim ---------------------------------------------
    Guard(
        id="ci-floor",
        file="scripts/ci_floor_check.py",
        gate="ci",
        spec="`.claude/WORKFLOW.md`, `--floor` is \"what CI runs\"",
        refuses="A gate in the floor that `.github/workflows/ci.yml` does not "
                "actually run, or runs only on events the floor does not "
                "cover.",
        claims=("*",),
        probes=(
            Probe("ci-floor.event-gated",
                  lambda box: _gate_step_behind(
                      box, "github.event_name == 'workflow_dispatch'"),
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="The step is untouched and still names the gate. Only "
                       "the event changes, and a gate that runs on a manual "
                       "dispatch and not on a pull request has the same "
                       "effect as no step at all. The S03 review's third pass "
                       "measured this route to G-01's outcome after the "
                       "prefix and comment routes were shut."),
            Probe("ci-floor.event-gated-accept",
                  lambda box: _gate_step_behind(
                      box, "github.event_name != 'workflow_dispatch'"),
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The other direction, and it is the point. This "
                       "condition excludes only the manual dispatch, so the "
                       "gate still runs on every push and every pull request "
                       "the floor covers and the check must still pass. A "
                       "check that refused every `if:` would be as useless as "
                       "one that read none of them."),
            Probe("ci-floor.complementary-steps",
                  _gate_step_split_across_events,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="Two steps, one per automatic event, covering the "
                       "floor between them. Coverage is asked per event and "
                       "not per step, because asking one step to satisfy "
                       "every event refuses this arrangement and then reports "
                       "no missing event, which is a refusal a maintainer "
                       "cannot act on."),
            Probe("ci-floor.missing-step",
                  lambda box: _delete_ci_step(box, leave_comment=False),
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in"),
            Probe("ci-floor.partial-arm",
                  _delete_one_command_of_a_gate,
                  script("python3", "scripts/ci_floor_check.py"),
                  "runs only part of it on",
                  note="A gate is every command in its arm. The `backlog` "
                       "gate is `backlog_check.py && gen_sprint_plan.py "
                       "--check`, and the S03 review's fourth pass measured "
                       "that deleting either step alone left this check at 0 "
                       "and only deleting both made it 1. So the estimate "
                       "comparison added in the same pass could be removed "
                       "from every pull request by deleting one line. The "
                       "probe deletes ONE step and leaves the other."),
            Probe("ci-floor.whole-arm-through-the-runner",
                  _one_step_for_the_whole_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The other direction, and it is why the rule is not "
                       "\"every command must appear as its own step\". A step "
                       "running `bin/ocelli.sh gate <name>` runs the whole arm "
                       "by definition, which is how ci.yml already invokes "
                       "`errors`, `bench`, `packages` and `guards`. A check "
                       "that demanded the commands as well would refuse the "
                       "arrangement the workflow uses today."),
            Probe("ci-floor.non-floor-gate-not-run",
                  _delete_the_non_floor_ci_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is excluded from the floor and needs no GPU",
                  note="`NOT_IN_FLOOR` took `guards-deep` out of the floor and "
                       "nothing else asserted CI ran it, so the S03 review's "
                       "fourth pass deleted the whole job and this check, the "
                       "census and the floor probes all exited 0. "
                       "`guards-deep` is what runs the cargo probes and "
                       "`census --profile deep`, which is the sweep-complete "
                       "rule. The job, its name and its trigger comment are "
                       "left in place here and only the `run:` line goes, so "
                       "the probe is the comment-only lesson again from the "
                       "other side of the floor."),
            Probe("ci-floor.exclusion-lists-disagree",
                  _exclude_a_floor_gate_in_the_runner_only,
                  script("python3", "scripts/ci_floor_check.py"),
                  "disagree about which gates the floor excludes",
                  note="bin/ocelli.sh's comment said a name in one list and "
                       "not the other would make the `ci` gate demand a step "
                       "for a gate the floor never runs. The reviewer added "
                       "`prose` to the shell list alone and measured the "
                       "opposite: this check exited 0 and `gate --floor` "
                       "silently stopped running `prose`. That direction "
                       "removes work rather than adding a demand, so it had "
                       "no detection at all. The gate is read from the "
                       "runner's own GATES table rather than named here."),
            Probe("ci-floor.exclusion-list-reordered",
                  _reorder_the_runner_exclusion_list,
                  script("python3", "scripts/ci_floor_check.py"),
                  "exclusion list and NOT_IN_FLOOR agree on",
                  polarity="accept",
                  note="The two lists are a SET written twice. A comparison "
                       "made on the text rather than on the set would refuse a "
                       "reordering that changes nothing, and this repository "
                       "has already paid once for a positional read of a "
                       "Cargo entry in probe `pins.table-form`."),
            Probe("ci-floor.no-arm-command-gate-behind-a-condition",
                  _a_no_arm_command_gate_behind_a_condition,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="What watches `covers`'s `bool(arm)` clause, and "
                       "nothing did until the sixth pass. This entry claimed "
                       "`ci-floor.gate-with-no-arm-command` was the watch and "
                       "it is not: that probe DELETES the naming step, so "
                       "`steps_running` is empty and the refusal comes from "
                       "the final \"nothing in ci.yml runs it\" branch without "
                       "`covers` being consulted at all. Dropping "
                       "`bool(arm) and` left it green. Here the step stays and "
                       "is gated to one event, so the per-event question is "
                       "the one that decides. MEASURED: with the clause the "
                       "check refuses at this fragment, and with it dropped "
                       "the fragment is gone. `ci-floor.event-gated` reaches "
                       "the same clause today only because the first gate "
                       "ci.yml names happens to have an empty arm, which is "
                       "an accident of ordering rather than a selection."),
            Probe("ci-floor.gate-with-an-unextractable-arm-command",
                  _expand_a_gate_step_into_its_visible_commands,
                  script("python3", "scripts/ci_floor_check.py"),
                  "so it cannot demand those commands step by step",
                  note="The declared prefix limit was a live hole and the OK "
                       "line asserted the opposite. `COMMAND_PREFIXES` cannot "
                       "see `node`, so the `bench` arm's three `node --test` "
                       "suites were not demanded of CI, and the docstring's "
                       "claim that nothing was lost because these gates are "
                       "\"each invoked by NAME in ci.yml\" was enforced by "
                       "nothing. MEASURED: replacing "
                       "`- run: bin/ocelli.sh gate bench` with its two "
                       "extractable commands left the check at exit 0 printing "
                       "\"every command in each gate's arm\", and five node "
                       "test files left CI in a two-line edit that reads as "
                       "expanding the step. Same shape as the `arms['panic']` "
                       "bug the fifth pass fixed, one level out."),
            Probe("ci-floor.gate-with-no-arm-command",
                  _swap_a_named_gate_step_for_an_unrelated_command,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="The worst thing the fifth pass found. An arm ended at "
                       "the next case LABEL rather than at its `;;`, so it "
                       "swallowed the comment block introducing the following "
                       "arm and the command regex read commands out of prose: "
                       "`arms['panic']` came out as `['npm run test']`, from "
                       "the `bench` comment block's sentence about "
                       "`npm run test:browser`. Deleting the "
                       "`bin/ocelli.sh gate panic` step refused, and adding a "
                       "legitimate `- run: npm run test` step beside the "
                       "deletion returned the check to exit 0 with `all 25 "
                       "floor gate(s) are invoked by CI`. `panic` is HLD "
                       "section 23's wasm panic-hook proof, the one property "
                       "no native test can observe. What this probe watches is "
                       "that arm-ending rule and the final \"nothing runs it\" "
                       "branch, and NOT `covers`'s `bool(arm)` clause: the "
                       "builder deletes the naming step, so `steps_running` "
                       "comes back empty and `covers` is never consulted. "
                       "`ci-floor.no-arm-command-gate-behind-a-condition` "
                       "above is what watches that clause. This entry claimed "
                       "otherwise until the sixth pass measured it."),
            Probe("ci-floor.narrowed-arm-command",
                  _replace_a_gate_step_with_a_narrowed_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "runs only part of it on",
                  note="Every command of the arm appears as a step and one of "
                       "them discovers zero tests. The extraction class "
                       "stopped at a `\\` continuation, so `-p <suite>` fell "
                       "off the end of `errors`, `bench` and `guards`, and "
                       "`runs_command` searches rather than matches, so any "
                       "unittest step satisfied any of them. Measured at exit "
                       "0 with `-p test_nothing_at_all.py`."),
            Probe("ci-floor.exclusion-list-unreadable",
                  _reformat_the_runner_exclusion_list,
                  script("python3", "scripts/ci_floor_check.py"),
                  "exclusion list where this parser looks for it",
                  note="A cosmetic reformat into one case arm per name. The "
                       "refusal is right, the floor's exclusion decision "
                       "cannot be read from that shape, and until the fifth "
                       "pass it arrived as a bare RuntimeError traceback with "
                       "no `FAIL:` header. Fail-closed is not the same as "
                       "actionable, and the fragment this probe expects is "
                       "printed under the header now."),
            Probe("ci-floor.run-gate-region-unreadable",
                  _hide_the_run_gate_region,
                  script("python3", "scripts/ci_floor_check.py"),
                  "carries no `run_gate() {`",
                  note="The arm parser's own empty-set case. It used to be a "
                       "bare `str.index` ValueError. A parser that cannot "
                       "find the region gives every gate an empty arm, and an "
                       "empty arm is exactly what `covers` treats as "
                       "gate-name-only."),
            Probe("ci-floor.nested-case-in-an-arm",
                  _nested_case_in_a_gate_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="`ARM` ends an arm at its `;;` and a nested case ends "
                       "its own branches the same way, so the parser kept only "
                       "what came before the first inner `;;`. The S03 review's "
                       "sixth pass found this degrading safe by accident: the "
                       "one shape tried left the arm EMPTY and `covers` "
                       "refuses an empty arm. With a real command before the "
                       "nested case the parser keeps that command, every "
                       "extractable command is then accounted for, and the "
                       "rest of the arm is dropped at exit 0. This probe "
                       "builds that shape. The parser refuses it now rather "
                       "than truncating, because balancing `case`/`esac` here "
                       "would be a second shell parser in a file that has one "
                       "already."),
            Probe("ci-floor.no-arms-at-all",
                  _empty_the_run_gate_region,
                  script("python3", "scripts/ci_floor_check.py"),
                  "declares no case arm this parser can read",
                  note="The region is present and holds no arm. Every gate "
                       "would then fall back to the gate-name route and the "
                       "per-command rule would hold over nothing, which is "
                       "the same vacuous pass `covers` refuses one level "
                       "down."),
            Probe("ci-floor.comment-only",
                  lambda box: _delete_ci_step(box, leave_comment=True),
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="A YAML comment naming the gate, with the real step "
                       "deleted. The claim the floor makes is that CI RUNS "
                       "the gate, and a comment runs nothing. This was G-01 "
                       "and failed until S03: the check was a plain substring "
                       "test over the whole workflow file, which a comment "
                       "satisfied, and which `gate guards` also satisfied "
                       "from inside `gate guards-deep`."),
        ),
        limit="The non-floor rule exempts a gate `bin/ocelli.sh` marks YES in "
              "its GPU column, which is `oracle` and deviation D-04's reason "
              "for it, and that column is not in the declared-constant "
              "ratchet. Marking `guards-deep|YES|` would therefore exempt it "
              "without this check noticing. It is left as a limit rather than "
              "recorded, because the column is a semantic claim in the "
              "runner's own gate table where a false entry reads as false to "
              "a person, and because `NOT_IN_FLOOR` and the runner's "
              "exclusion list are both watched, so the OTHER routes out of "
              "the floor are closed. The second limit is that this file "
              "cannot evaluate `github.ref`, so what it proves about "
              "`guards-deep` is that CI runs it on `workflow_dispatch`. The "
              "workflow's own comment claims a push to `main` as well and "
              "that half is unproven, which the OK line says in as many "
              "words rather than leaving a reader to infer the stronger "
              "claim. The third limit is the arm-command extractor's own "
              "vocabulary. It recognises `python3 `, `npm run `, `cargo ` and "
              "`ci/`, so `node`, `wasm-pack` and `\"$0\"` are invisible and "
              "the check cannot demand those commands of CI step by step. "
              "That was declared as a limit and taken on trust until the S03 "
              "review's sixth pass measured it open: replacing "
              "`- run: bin/ocelli.sh gate bench` with its two extractable "
              "commands left the check at exit 0 and five node test files out "
              "of CI. The vocabulary is unchanged and the CONSEQUENCE is now "
              "a rule: `unseen_commands` reports what the extractor cannot "
              "see and a gate holding one of those is refused unless a step "
              "invokes it by name, per event. So the remaining limit is only "
              "that the refusal names the gate rather than the command, and "
              "the `all([])` claim still holds, because `panic`, `native` and "
              "`oracle` still yield no extractable command and "
              "`ci-floor.no-arm-command-gate-behind-a-condition` is what "
              "watches `bool(arm)`. The fourth limit is what "
              "`unseen_commands` itself cannot judge: a statement whose first "
              "word is in `SHELL_NOISE` is treated as not being the work, so "
              "an arm that did its work inside an `if` or a `for` would be "
              "read as having none. Nothing does today, and `ARM` refuses a "
              "nested `case` outright rather than truncating the arm at its "
              "inner `;;`.",
    ),

    # -- D-04's chain, the part CI reads ------------------------------------
    Guard(
        id="ledger.assert",
        file="scripts/verify_ledger.py",
        gate="-",
        spec="deviation D-04 mechanism 1, and HLD 27.2 R6",
        refuses="A staged tree with no recorded gate run, a tree whose "
                "recorded corpus is red, and a corpus state outside the "
                "declared set.",
        claims=(r"no verification recorded", r"the corpus is RED",
                r"corpus is ' ' for tree", r"--corpus must be one of"),
        probes=(
            Probe("ledger.no-record", None,
                  script("python3", "scripts/verify_ledger.py", "assert"),
                  "no verification recorded for the staged tree",
                  polarity="refuse",
                  note="No mutation is needed and that is the point: the "
                       "sandbox carries no ledger, because the ledger is "
                       "gitignored per-clone evidence and `git ls-files` "
                       "therefore never copies it."),
            Probe("ledger.red-corpus", _ledger_red_corpus,
                  script("python3", "scripts/verify_ledger.py", "assert"),
                  "the corpus is RED for tree"),
            Probe("ledger.require-corpus", _ledger_absent_corpus,
                  script("python3", "scripts/verify_ledger.py", "assert",
                         "--require-corpus"),
                  "and this gate requires 'pass'"),
            Probe("ledger.bad-state", None,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--corpus", "probably"),
                  "--corpus must be one of",
                  control=script("python3", "scripts/verify_ledger.py",
                                 "record", "--corpus", "absent")),
        ),
        limit="`assert` with a real record cannot be controlled green in the "
              "sandbox without recording one first, so the control for this "
              "invoke records a passing entry and then asserts.",
    ),
    Guard(
        id="ledger.trailer",
        file="scripts/verify_ledger.py",
        gate="-",
        spec="deviation D-04, the note that the trailer is unforgeable",
        refuses="Emitting a provenance trailer for a tree the ledger has no "
                "record for. It exits non-zero and prints NOTHING, which is "
                "what makes the trailer evidence rather than assertion.",
        claims=(),
        silent="This refusal is a bare `return 1` with no message, and the "
               "absence of output IS the mechanism. Runbook probe 16.",
        probes=(
            Probe("ledger.trailer-silent", None,
                  Invoke("verify_ledger trailer", _trailer_without_a_record),
                  "", note="The expectation is empty output and a non-zero "
                           "exit. The runner treats an empty `expect` as "
                           "\"the refusal has no words\" and asserts the "
                           "output is empty instead."),
        ),
    ),
    Guard(
        id="ledger.check-commit",
        file="scripts/verify_ledger.py",
        gate="-",
        spec="deviation D-04 mechanism 2, the CI side",
        refuses="A head with no provenance trailer, a trailer naming a tree "
                "that is not the commit's, and a trailer recording a red or "
                "unrun corpus.",
        claims=(r"carries no trailer", r"trailer names tree",
                r"records a RED corpus", r"records corpus="),
        probes=(
            Probe("ledger.no-trailer", None,
                  script("python3", "scripts/verify_ledger.py",
                         "check-commit", "HEAD"),
                  "carries no Ocelli-Verify trailer"),
            Probe("ledger.tree-mismatch", _commit_amended_tree,
                  script("python3", "scripts/verify_ledger.py",
                         "check-commit", "HEAD"),
                  "but the commit's tree is",
                  note="The load-bearing half of D-04's mechanism 2, and it "
                       "had never been observed red by anyone. The ledger "
                       "entry is written against one tree, the commit is then "
                       "amended so its tree differs, and the trailer becomes "
                       "evidence about content that is not in the commit."),
        ),
        limit="The `records a RED corpus` and `records corpus=` branches of "
              "check-commit need a commit carrying a trailer this harness "
              "would have to forge, and the commit-msg hook refuses exactly "
              "that. They are reached instead by ledger.assert's equivalents.",
    ),

    # -- the hooks, executable for the first time ---------------------------
    Guard(
        id="hooks.pre-commit",
        file=".githooks/pre-commit",
        gate="-",
        spec="CLAUDE.md hard rule, and deviation D-04 mechanism 1",
        refuses="A commit carrying patient data, a source-policy violation or "
                "a voice-rule violation in the staged set.",
        claims=("*",),
        probes=(
            Probe("hooks.pre-commit.dicom", _stage_a_dicom_for_commit,
                  CLEAN_COMMIT,
                  "Commit refused",
                  note="`.githooks/*` are inert until a clone sets "
                       "core.hooksPath, which is per clone and untracked, so "
                       "no gate can fire them in the developer's repository. "
                       "In the sandbox they run for real."),
        ),
    ),
    Guard(
        id="hooks.commit-msg",
        file=".githooks/commit-msg",
        gate="-",
        spec="HLD 27.2 R6, and deviation D-04",
        refuses="A hand-written provenance trailer, an agent co-author "
                "trailer, and a voice-rule violation in the message.",
        claims=("*",),
        probes=(
            Probe("hooks.commit-msg.forged-trailer", _stage_a_benign_change,
                  commit_with("F-000, a probe\n\n"
                              "Ocelli-Verify: profile=sprint gates=fmt "
                              "corpus=pass tree=000000000000\n"),
                  "written by this hook from the verify ledger",
                  control=CLEAN_COMMIT,
                  note="Runbook probe 13. With probe 16 this is the whole of "
                       "D-04's compensating control. The staged change is the "
                       "same benign one the control commits, so the ONLY "
                       "difference between the two runs is the message."),
            Probe("hooks.commit-msg.agent-coauthor", _stage_a_benign_change,
                  commit_with("F-000, a probe\n\n"
                              "Co-Authored-By: Claude "
                              "<noreply@example.invalid>\n"),
                  "no agent co-author trailer",
                  control=CLEAN_COMMIT),
            Probe("hooks.commit-msg.prose", _stage_a_benign_change,
                  commit_with("F-000, a probe\n\n"
                              "One clause; and a second.\n"),
                  "semicolon in prose",
                  control=CLEAN_COMMIT,
                  note="The voice rules reach a commit message only through "
                       "this hook, so this is the only place that path is "
                       "watched end to end."),
        ),
    ),
    Guard(
        id="hooks.pre-push",
        file=".githooks/pre-push",
        gate="-",
        spec="deviation D-04 mechanism 1, \"push is refused without it\"",
        refuses="A push whose head carries no verification evidence.",
        claims=("*",),
        probes=(
            Probe("hooks.pre-push.unverified", _enable_hooks,
                  Invoke("git push, hooks enabled", _push),
                  "Push refused",
                  note="The head the sandbox is built at carries no trailer, "
                       "because the ledger is per-clone evidence that "
                       "`git ls-files` never copies. The control commits one "
                       "the way a healthy repository does, through the "
                       "commit-msg hook and a real ledger record, and the "
                       "same push then succeeds."),
        ),
    ),

    # -- decision D2 and HLD 31 --------------------------------------------
    Guard(
        id="bindgen",
        file="ci/check-bindgen-isolation.sh",
        gate="bindgen",
        spec="HLD 15.3, decision D2, and deviation D-12 for the wasm32 half",
        refuses="Any crate but the boundary crate reaching wasm-bindgen on "
                "the host, declaring it in its own manifest under any target "
                "gate, or naming it in source.",
        claims=("*",),
        probes=(
            Probe("bindgen.reaches", _bindgen_direct_dependency,
                  script("ci/check-bindgen-isolation.sh"),
                  "reaches wasm-bindgen", needs="cargo", profile="deep"),
            Probe("bindgen.declares", _bindgen_target_gated,
                  script("ci/check-bindgen-isolation.sh"),
                  "declares wasm-bindgen as a direct dependency",
                  needs="cargo", profile="deep",
                  note="Target-gated, so `cargo tree` on the host does not "
                       "see it and only D-12's direct-declaration pass can. "
                       "That is the exact case F-002 added that pass for."),
            Probe("bindgen.in-source", _bindgen_in_source,
                  script("ci/check-bindgen-isolation.sh"),
                  "names wasm_bindgen in source",
                  needs="cargo", profile="deep"),
        ),
    ),
    Guard(
        id="device",
        file="ci/check-device-ownership.sh",
        gate="device",
        spec="HLD section 31 and the section 38 hook, story E1.8",
        refuses="A crate other than the renderer bringing a device into "
                "existence, the contract type disappearing, an accessor "
                "handing out an owned device, and a second owner by Clone.",
        claims=("*",),
        probes=(
            Probe("device.creator", _device_creator,
                  script("ci/check-device-ownership.sh"),
                  "creates a GPU device or surface",
                  note="Input from section 31's own sentence and from wgpu's "
                       "API for creating a device, planted in the crate "
                       "section 31 names. Not from the guard's CREATORS "
                       "list, which is separately in the ratchet."),
            Probe("device.contract-gone", _device_context_gone,
                  script("ci/check-device-ownership.sh"),
                  "no longer defines GpuContext",
                  note="Section 31 is not satisfied by nobody holding a "
                       "device, it is satisfied by ONE crate holding it."),
            Probe("device.owned-accessor", _device_owned_accessor,
                  script("ci/check-device-ownership.sh"),
                  "hands out an owned device"),
            Probe("device.derives-clone", _device_derives_clone,
                  script("ci/check-device-ownership.sh"),
                  "derives Clone"),
        ),
        limit="The trybuild compile-fail cases under "
              "crates/ocelli-compute/tests/ui/ are the strong half of section "
              "31 and run in the `test` gate, and they are recorded here "
              "rather than as `covered_by`. They assert the same property "
              "through the type system and they do not open "
              "ci/check-device-ownership.sh, so counting them as coverage of "
              "THIS guard's refusals would be the claim check f exists to "
              "refuse. The four probes above are what watch those refusals.",
    ),

    # -- the ledgers --------------------------------------------------------
    Guard(
        id="backlog",
        file="scripts/backlog_check.py",
        gate="backlog",
        spec="`.claude/WORKFLOW.md`, the ledgers agree, and HLD section 38's "
             "five Phase 1 hooks",
        refuses="A story marked done with no completion record, a duplicate "
                "or malformed status, a story in the allocation and not the "
                "backlog, a dropped import defect, and a section 38 hook "
                "leaving Phase 1.",
        claims=("*",),
        probes=(
            Probe("backlog.done-without-record", _backlog_done_without_record,
                  script("python3", "scripts/backlog_check.py"),
                  "is done with no SPRINT_TRACKER.md row"),
            Probe("backlog.bad-status", _backlog_bad_status,
                  script("python3", "scripts/backlog_check.py"),
                  "expected one of"),
            Probe("backlog.hook-out-of-phase", _hook_out_of_phase,
                  script("python3", "scripts/backlog_check.py"),
                  "not P1"),
        ),
    ),
    Guard(
        id="sprint-plan",
        file="scripts/gen_sprint_plan.py",
        gate="backlog",
        spec="`.claude/WORKFLOW.md`, the sprint plan is derived and not "
             "hand-maintained",
        refuses="A sprint plan that disagrees with the backlog about which "
                "sprint a story is in or how large it is, a story planned "
                "into two sprint tables at once, a generated milestone "
                "summary or goal line that has drifted from the allocation it "
                "is written from or that is absent, duplicated or spurious, "
                "and an absent plan.",
        claims=("*",),
        probes=(
            Probe("sprint-plan.two-sprint-tables",
                  _sprint_plan_row_in_two_sprints,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "SPRINT_PLAN.md sprint tables",
                  note="The rule comes from the guard's own docstring, that "
                       "every planned F-ID appears in \"exactly one sprint "
                       "table\", which was stated and not checked: the parser "
                       "let the last occurrence win. Measured in the fifth "
                       "pass at exit 0 with S72's F-149 row copied into "
                       "S01's table, where it agreed with the allocation "
                       "because the later row overwrote it."),
            Probe("sprint-plan.milestone-summary-absent",
                  _sprint_plan_milestone_summary_deleted,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "carries no summary line for it",
                  note="The lines were compared by POSITION, so one absent "
                       "line shifted every line after it and the refusal "
                       "named the wrong milestone. They are matched on the "
                       "sprint span they name now, which is the identity a "
                       "reader has anyway."),
            Probe("sprint-plan.milestone-summary-spurious",
                  _sprint_plan_extra_milestone_summary,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "has no milestone spanning those sprints",
                  note="The other direction of the same span comparison. A "
                       "summary line is generated, so one the allocation "
                       "cannot account for is hand-written prose wearing a "
                       "generated line's shape."),
            Probe("sprint-plan.two-goal-lines", _sprint_plan_two_goal_lines,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "**Goal** lines",
                  note="The parser kept the FIRST goal line per sprint, so a "
                       "stale paragraph above a corrected one won and the "
                       "corrected one was never read. `render` writes exactly "
                       "one per sprint."),
            Probe("sprint-plan.absent", _sprint_plan_absent,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "does not exist"),
            Probe("sprint-plan.wrong-sprint", _sprint_plan_wrong_sprint,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "is in sprint"),
            Probe("sprint-plan.wrong-estimate", _sprint_plan_wrong_estimate,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "is estimated"),
            Probe("sprint-plan.wrong-milestone-summary",
                  _sprint_plan_wrong_milestone_summary,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "milestone summary line"),
            Probe("sprint-plan.stale-goal-line", _sprint_plan_stale_goal_line,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "**Goal** line"),
        ),
    ),
    Guard(
        id="skills",
        file="scripts/sync_agent_skills.py",
        gate="skills",
        spec="`.claude/WORKFLOW.md`, one workflow and two hosts",
        refuses="A Codex adapter that has fallen behind its canonical command "
                "or skill file, or that has no canonical source at all.",
        claims=("*",),
        probes=(
            Probe("skills.stale-adapter", _stale_codex_adapter,
                  script("python3", "scripts/sync_agent_skills.py", "--check"),
                  "is stale, its source changed"),
        ),
    ),
    Guard(
        id="handoff",
        file="scripts/sprint_workflow.py",
        gate="-",
        spec="`.claude/WORKFLOW.md` and `.claude/commands/complete-feature.md`",
        refuses="A handoff missing a required field, or naming a branch that "
                "is not this story's.",
        claims=(r"handoff has no", r"branch does not start with",
                r"FAIL: handoff for", r"does not exist"),
        probes=(
            Probe("handoff.wrong-branch", _handoff_wrong_branch,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "does not start with"),
            Probe("handoff.backticked-branch", _handoff_backticked_branch,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "validates",
                  polarity="accept",
                  defect="G-04",
                  note="This repository writes every path in backticks and "
                       "the branch is parsed as a bare token, so a correct "
                       "handoff written in the house style is refused. The "
                       "contract is documented nowhere. It cost one handoff "
                       "rewritten at integration this sprint."),
        ),
        limit="The other twenty-two refusals in this file belong to the "
              "sprint lifecycle commands, and the entry below owns them. The "
              "field list is a second limit and a sharper one. Both probes "
              "here are about the BRANCH rule, and reaching it means writing "
              "a handoff that passes the field check first, so their input "
              "carries the five `**Field**` markers from the tool's own "
              "tuple. `.claude/commands/complete-feature.md` names six items "
              "including the files touched, which the tool does not require, "
              "so the citation and the code do not agree and no probe can see "
              "that. `HANDOFF_FIELDS` is in the declared-constant ratchet "
              "instead, which is what puts a change to the contract in front "
              "of a reviewer. G-04 is the same undocumented contract seen "
              "from the branch side.",
    ),
    Guard(
        id="sprint-lifecycle",
        file="scripts/sprint_workflow.py",
        gate="-",
        spec="`.claude/WORKFLOW.md`, the sprint lifecycle",
        refuses="An init, start, complete, close or release-notes step taken "
                "out of order, against a story that is not in the sprint, or "
                "against notes that are empty, placeholder or stale.",
        claims=("*",),
        probes=(
            Probe("sprint-lifecycle.not-in-sprint",
                  lambda box: (sprint_state(box),
                               box.write(".claude/probe-fid", "F-999")),
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "is not in sprint"),
        ),
        limit="One probe over the shape shared by every lifecycle refusal. "
              "The remaining branches need a sprint mid-flight, which the "
              "sandbox cannot build without writing sprint state, and "
              "docs/sprints/ is outside this story's write set.",
    ),

    # -- the error registry, the benchmarks and the corpus -----------------
    Guard(
        id="errors",
        file="scripts/error_code_check.py",
        gate="errors",
        spec="HLD section 23, error codes are stable and versioned",
        refuses="A renumbering across Rust, TypeScript and the registry, a "
                "reused number or name, a code either side cannot name, and a "
                "code outside its crate's declared range.",
        claims=("*",),
        probes=(
            Probe("errors.renumbered", _renumber_an_error_code,
                  script("python3", "scripts/error_code_check.py"),
                  "so this is a renumbering",
                  note="The fragment names the RENUMBERING branch and not the "
                       "`FAIL:` header. This script produces four independent "
                       "problem classes under two headers, so a probe that "
                       "expected `FAIL` rode on whichever one still worked: "
                       "with the Rust-registry comparison deleted it stayed "
                       "green on the range check alone."),
        ),
        covered_by=("scripts/tests/test_error_code_check.py "
                    "(15 cases, run by the `errors` gate)",),
    ),
    Guard(
        id="bench",
        file="scripts/bench_check.py",
        gate="bench",
        spec="HLD section 26's last rule, and story E1.6",
        refuses="A benchmark registry whose subject stories do not resolve, a "
                "subject whose story has not landed carrying a runner or a "
                "recorded number, and a playwright pin that has drifted "
                "between the two harnesses.",
        claims=("*",),
        covered_by=("scripts/tests/test_bench_check.py "
                    "(25 cases, run by the `bench` gate, which is in the "
                    "floor)",),
        limit="No level-3 probe. The suite above is the negative-case set for "
              "this guard and it runs on every floor gate, so a level-3 "
              "probe would need a second registry fixture that the suite "
              "already carries.",
    ),
    Guard(
        id="corpus",
        file="scripts/corpus_check.py",
        gate="corpus",
        spec="`corpus/README.md` and HLD section 25.1's two tolerance classes",
        refuses="A manifest row whose file is absent or carries a different "
                "digest, a row with an unrecorded licence, a malformed "
                "manifest, and a corpus that stops covering a transfer syntax "
                "the codec registry claims.",
        claims=("*",),
        probes=(
            Probe("corpus.digest-mismatch", _corpus_digest_mismatch,
                  script("python3", "scripts/corpus_check.py"),
                  "does not match its manifest digest",
                  note="Runbook probe 10, made a FLOOR probe. Nothing in this "
                       "path parses DICOM, so the fixture is two files of "
                       "ASCII and a two-row manifest, and it needs no corpus."),
            Probe("corpus.absent", _corpus_absent,
                  script("python3", "scripts/corpus_check.py"),
                  "corpus cases are absent",
                  note="Runbook probe 11, likewise."),
            Probe("corpus.unrecorded-licence", _corpus_unrecorded_licence,
                  script("python3", "scripts/corpus_check.py",
                         "--manifest-only"),
                  "cannot be redistributed or cited",
                  note="Runbook probe 12."),
        ),
        covered_by=("scripts/tests/test_corpus_check.py "
                    "(run by the `corpus-tests` gate)",),
    ),
    Guard(
        id="corpus-tests",
        file="scripts/corpus_tests.py",
        gate="corpus-tests",
        spec="`.claude/WORKFLOW.md`, a skipped gate is not a pass",
        refuses="A run under an interpreter that cannot import the DICOM "
                "tooling, rather than reporting a skip as a pass.",
        claims=("*",),
        probes=(
            Probe("corpus-tests.skip-is-not-a-pass", None,
                  Invoke("corpus_tests --require-prerequisites",
                         lambda box: box.run(
                             ["python3", "scripts/corpus_tests.py",
                              "--require-prerequisites"],
                             env={"OCELLI_PYTHON": "/nonexistent/python"})),
                  "FAIL: a prerequisite",
                  control=Invoke("corpus_tests",
                                 lambda box: box.run(
                                     ["python3", "scripts/corpus_tests.py"],
                                     env={"OCELLI_PYTHON":
                                          "/nonexistent/python"})),
                  control_status=3,
                  control_expect="SKIPPED: a prerequisite",
                  note="A skip is not a pass, and the two must be told apart "
                       "by the exit code rather than by reading the output. "
                       "The control is the SAME suite without the flag, which "
                       "a healthy repository answers with a named skip and "
                       "exit 3, and the probe is the flag that turns that "
                       "skip into a refusal. No pydicom is copied into the "
                       "sandbox, so both halves are the sandbox's own state."),
        ),
    ),
    Guard(
        id="populate-corpus",
        file="scripts/populate_corpus.py",
        gate="-",
        spec="none. It is invoked by no gate.",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="A corpus acquisition tool a developer runs by hand. No gate "
               "and no CI step invokes it, and its first act is to refuse "
               "when the locked Python environment is absent, which the "
               "sandbox always is because `.venv` is not tracked. The "
               "property its refusals protect, that a case matches the "
               "digest its manifest row records, is verified afterwards and "
               "independently by scripts/corpus_check.py, which IS in a gate "
               "and whose digest and presence refusals are both probed above.",
    ),
    Guard(
        id="corpus-synth",
        file="scripts/corpus_synth.py",
        gate="corpus-tests",
        spec="`corpus/README.md`, the generator's recorded tool versions",
        refuses="A manifest whose header is not the recorded column set.",
        claims=("*",),
        covered_by=("scripts/tests/test_corpus_synth.py "
                    "(run by the `corpus-tests` gate)",),
    ),

    # -- the cross-target proof and the packages ---------------------------
    Guard(
        id="target-features",
        file="scripts/target_feature_check.py",
        gate="native",
        spec="story E1.7 and HLD section 4's per-target crate table",
        refuses="A workspace dependency resolving a different feature set per "
                "target, and a run that could not resolve the graph at all.",
        claims=("*",),
        probes=(
            Probe("target-features.cannot-run", _unresolvable_graph,
                  script("python3", "scripts/target_feature_check.py"),
                  "could not run",
                  needs="cargo", profile="deep",
                  note="Fail-closed. A feature check that cannot resolve the "
                       "graph and exits 0 is the shape runbook probe 18 "
                       "describes: nothing to check, said by succeeding."),
        ),
        limit="The per-target divergence branch itself needs a dependency "
              "whose features differ by target, which cannot be built from "
              "the locked graph without a network fetch. Owner F-X014.",
    ),
    Guard(
        id="packages",
        file="scripts/package_check.py",
        gate="packages",
        spec="story E1.3 and `docs/RELEASE.md`",
        refuses="A published package whose version skews from the workspace, "
                "an advertised exports path absent from the tarball, a "
                "tarball carrying sources or build state, and a consumer that "
                "cannot import or type-check what was published.",
        claims=("*",),
        probes=(
            Probe(
                "packages.exports-not-in-tarball", None,
                python_snippet(
                    "package_check.check_tarball",
                    "import sys; sys.path.insert(0, 'scripts');\n"
                    "import package_check as p;\n"
                    "m = {'name': '@ocelli/core', 'version': '0.1.0',\n"
                    "     'exports': {'.': {'import': './dist/index.js'}},\n"
                    "     'files': ['dist']};\n"
                    "names = ['package.json', 'README.md', 'LICENSE-MIT',\n"
                    "         'LICENSE-APACHE'];\n"
                    "bad = p.check_tarball('core', m, names);\n"
                    "print('\\n'.join(bad));\n"
                    "sys.exit(1 if bad else 0)\n"),
                "advertises",
                level=1,
                control=python_snippet(
                    "package_check.check_tarball, healthy",
                    "import sys; sys.path.insert(0, 'scripts');\n"
                    "import package_check as p;\n"
                    "m = {'name': '@ocelli/core', 'version': '0.1.0',\n"
                    "     'exports': {'.': {'import': './dist/index.js'}},\n"
                    "     'files': ['dist']};\n"
                    "names = ['package.json', 'README.md', 'LICENSE-MIT',\n"
                    "         'LICENSE-APACHE', 'dist/index.js'];\n"
                    "bad = p.check_tarball('core', m, names);\n"
                    "print('\\n'.join(bad));\n"
                    "sys.exit(1 if bad else 0)\n"),
                note="Level 1. `check_tarball` is a pure function over a "
                     "manifest and a list of names, so the refusal the file "
                     "exists for is watched in the floor with no npm. The "
                     "input is an exports map naming a path the name list "
                     "does not carry, which is story E1.3's sentence and not "
                     "the function's code."),
            Probe(
                "packages.version-skew", None,
                python_snippet(
                    "package_check.check_manifest",
                    "import sys; sys.path.insert(0, 'scripts');\n"
                    "import package_check as p;\n"
                    "m = {'name': '@ocelli/core', 'version': '0.0.9'};\n"
                    "bad = p.check_manifest('core', m, '0.1.0');\n"
                    "print('\\n'.join(bad));\n"
                    "sys.exit(1 if bad else 0)\n"),
                "the Rust workspace is",
                level=1,
                control=python_snippet(
                    "package_check.check_manifest, healthy",
                    "import sys; sys.path.insert(0, 'scripts');\n"
                    "import package_check as p;\n"
                    "m = {'name': '@ocelli/core', 'version': '0.1.0'};\n"
                    "bad = p.check_manifest('core', m, '0.1.0');\n"
                    "print('\\n'.join(bad));\n"
                    "sys.exit(1 if bad else 0)\n")),
        ),
        limit="The consumer install, the node import, the two tsc "
              "resolutions and the publish dry run are level 3 and need an "
              "npm install the sandbox does not carry. They run for real in "
              "the `packages` gate on every push, green, and this harness has "
              "not watched them red. Owner F-X014.",
    ),

    # -- the runner and the source resolver --------------------------------
    Guard(
        id="runner",
        file="bin/ocelli.sh",
        gate="-",
        spec="`.claude/WORKFLOW.md`, a skipped gate is not a pass",
        refuses="A command whose prerequisite is absent, rather than running "
                "it and reporting whatever comes out.",
        claims=("*",),
        probes=(
            Probe("runner.absent-prerequisite", None,
                  script("bin/ocelli.sh", "oracle"),
                  "reference stack is not installed",
                  control=script("bin/ocelli.sh", "gate", "--list"),
                  note="The sandbox never carries tools/oracle/node_modules, "
                       "because `git ls-files` does not, so the refusal is "
                       "the sandbox's natural state. The control is "
                       "`gate --list`, which proves the runner itself works "
                       "in the sandbox and does not print this message."),
        ),
    ),
    Guard(
        id="source-dir",
        file="scripts/source_dir.py",
        gate="-",
        spec="runbook probe 18, the bootstrap converter's fail-closed loader",
        refuses="A bootstrap converter run with no configured private source, "
                "rather than treating redaction as the identity function.",
        claims=("*",),
        probes=(
            Probe("source-dir.unconfigured", None,
                  script("python3", "scripts/source_dir.py"),
                  "are not configured",
                  control=Invoke(
                      "source_dir with a configured directory",
                      lambda box: box.run(
                          ["python3", "scripts/source_dir.py"],
                          env={"OCELLI_SOURCE_DIR": str(box.path)})),
                  note="Runbook probe 18 made standing. Its loader once "
                       "returned an empty rule list when the private map was "
                       "absent, so redaction quietly became the identity "
                       "function."),
        ),
    ),
    Guard(
        id="split-hld",
        file="scripts/split_hld.py",
        gate="-",
        spec="runbook probe 18, and `docs/hld/` is the authored text",
        refuses="Writing tracked Markdown from a private source with no "
                "redaction map beside it, and a split that lost or invented "
                "content.",
        claims=("*",),
        probes=(
            Probe("split-hld.cannot-run", _empty_source_directory,
                  Invoke("split_hld --check, empty source directory",
                         lambda box: box.run(
                             ["python3", "scripts/split_hld.py", "--check"],
                             env={"OCELLI_SOURCE_DIR":
                                  str(box.path / "probe-source")})),
                  "A check that cannot run is NOT a check that",
                  control=script("python3", "scripts/split_hld.py", "--check"),
                  control_status=1,
                  control_expect="are not configured",
                  note="Runbook probe 18's lesson generalised: a check with "
                       "nothing to check must not say so by succeeding. The "
                       "control is the same command with no source directory "
                       "configured at all, which refuses for a DIFFERENT "
                       "declared reason and with a different exit code, so "
                       "the probe proves the converter tells the two apart "
                       "rather than refusing everything."),
        ),
        limit="The redaction map's fail-closed branch, which is runbook probe "
              "18 itself, sits behind a pandoc conversion of the private "
              "`.docx`. Neither is in this repository, so a sandbox cannot "
              "reach it, and the same is true of the drift and "
              "section-mapping branches. Owner F-X014.",
    ),

    # -- the new guard this story ships ------------------------------------
    Guard(
        id="lint-policy",
        file="scripts/lint_policy_check.py",
        gate="guards",
        spec="HLD 27.1, the denied lint table",
        refuses="A lint in HLD 27.1's table weakened below the level the "
                "specification sets, a crate that stops inheriting the "
                "workspace lint table, and any `.rs` file in a crate putting "
                "one of those lints back to sleep with an inner allow, by "
                "name or through a group that contains it.",
        claims=("*",),
        probes=(
            Probe("lint-policy.weakened", _lint_policy_weakened,
                  script("python3", "scripts/lint_policy_check.py"),
                  "is 'allow' and HLD 27.1 requires"),
            Probe("lint-policy.uninherited", _lint_policy_uninherited,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table"),
            Probe("lint-policy.group-allow", _group_allow_at_a_crate_root,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The most serious thing the S03 review found. "
                       "`#![allow(clippy::pedantic)]` names none of HLD "
                       "27.1's five lints and disables four of them, and "
                       "`clippy::restriction` disables indexing_slicing. "
                       "MEASURED with cargo rather than read: on a minimal "
                       "crate carrying `cast_possible_truncation = \"deny\"` "
                       "and one `x as i32`, under the pinned 1.97.1 "
                       "toolchain, cargo clippy exits 101 without the "
                       "attribute and 0 with it. The guard matched by lint "
                       "NAME, so both the `clippy` gate and the `guards` gate "
                       "went green with the arithmetic denies switched off, "
                       "which is the defect class CLAUDE.md names as the one "
                       "that reaches patients."),
            Probe("lint-policy.expect-attribute",
                  _expect_attribute_at_a_crate_root,
                  script("python3", "scripts/lint_policy_check.py"),
                  "re-allows"),
            Probe("lint-policy.nothing-scanned", _no_crate_sources_at_all,
                  script("python3", "scripts/lint_policy_check.py"),
                  "not one `.rs` file was read"),
            Probe("lint-policy.allow-outside-the-crate-root",
                  _named_allow_outside_the_crate_root,
                  script("python3", "scripts/lint_policy_check.py"),
                  "re-allows `cast_possible_truncation`",
                  note="The second half of the same defect. The guard read "
                       "`<crate>/src/lib.rs` and nothing else, so an allow in "
                       "`main.rs` or in any module file was invisible. "
                       "Measured that it applies: with `src/lib.rs` carrying "
                       "nothing but `pub mod inner;` and the attribute in "
                       "`src/inner.rs`, cargo clippy goes from 101 to 0. An "
                       "inner attribute in a module file governs that module."),
            Probe("lint-policy.item-allow-is-permitted",
                  _item_allow_with_a_reason,
                  script("python3", "scripts/lint_policy_check.py"),
                  "carry no inner allow or expect of a denied lint",
                  polarity="accept",
                  note="The direction that is not obvious. HLD 27.1's note "
                       "asks for a deliberate, visible choice, and the outer "
                       "`#[allow(...)]` on one item with a reason IS that "
                       "choice. One character separates it from the refused "
                       "form, so a guard that refused this would be refusing "
                       "the remedy its own message recommends. FOUR item "
                       "kinds since the fifth pass, a fn, a struct, an impl "
                       "and a statement, because the probe planted on a `fn` "
                       "alone and the guard now refuses the same attribute on "
                       "a `mod`."),
            Probe("lint-policy.outer-allow-on-a-module",
                  _outer_allow_on_a_module,
                  script("python3", "scripts/lint_policy_check.py"),
                  "an outer attribute on a `mod` item covers the whole "
                  "module tree",
                  note="The refusal above's other half, and the guard used to "
                       "give opposite verdicts on the same scope. It refused "
                       "an inner attribute at a crate root BECAUSE it covers "
                       "the crate, and permitted an outer one on a module, "
                       "which covers a module. MEASURED under the pinned "
                       "1.97.1 toolchain: `src/inner.rs` with one `x as i32` "
                       "and `src/lib.rs` reading "
                       "`#[allow(clippy::cast_possible_truncation)] pub mod "
                       "inner;` takes cargo clippy from 101 to 0. The accept "
                       "probe planted its outer allow on a `fn`, so the `mod` "
                       "case was never exercised in either direction."),
            Probe("lint-policy.whitespace-in-the-lint-path",
                  _whitespace_in_the_lint_path,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="Rust tokenises `clippy :: pedantic` exactly as "
                       "`clippy::pedantic`, and the captured name carried its "
                       "spaces into the set lookup and matched nothing. So "
                       "the whole nine-name REFUSED_GROUPS list was one space "
                       "away from unreachable. MEASURED under 1.97.1: no "
                       "attribute exits 101, `#![allow(clippy :: pedantic)]` "
                       "exits 0, `#![allow(clippy:: pedantic)]` exits 0 and "
                       "`#![expect(clippy :: cast_possible_truncation)]` "
                       "exits 0."),
            Probe("lint-policy.member-outside-crates-uninherited",
                  _member_outside_crates_uninherited,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table",
                  note="The workspace has fourteen members and the check read "
                       "thirteen: `crates/` was hard-coded and Cargo.toml "
                       "says `members = [\"crates/*\", \"tools/oracle\"]`. "
                       "The member is read from the manifest here rather than "
                       "named, because a literal would be the same assumption "
                       "that cost the workspace its fourteenth member."),
            Probe("lint-policy.member-outside-crates-group-allow",
                  _member_outside_crates_group_allow,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The other half of the same hole. With both this and "
                       "the uninherited probe applied, the check exited 0 "
                       "printing `13 crate(s) inherit the table, 33 .rs "
                       "file(s)`, and the census exited 0 beside it. "
                       "`tools/oracle` is a compiled member with thirteen "
                       "`.rs` files."),
            Probe("lint-policy.member-unresolvable",
                  _unresolvable_workspace_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "resolves to no directory carrying a Cargo.toml",
                  note="cargo refuses this workspace. A check that walks the "
                       "members it can resolve and reports the smaller number "
                       "as a pass is AGENTS.md's named failure of answering a "
                       "question about a smaller set in the language of "
                       "success."),
            Probe("lint-policy.no-members-declared",
                  _no_workspace_members_at_all,
                  script("python3", "scripts/lint_policy_check.py"),
                  "declares no `members` this parser can read",
                  note="The empty-set case for the member walk, which is the "
                       "same shape as `lint-policy.nothing-scanned` one level "
                       "up. With no member the inheritance pass, the "
                       "attribute pass and the file count all answer a "
                       "question about nothing."),
            Probe("lint-policy.group-row-in-the-workspace-table",
                  _group_row_in_the_workspace_table,
                  script("python3", "scripts/lint_policy_check.py"),
                  "carries the lint GROUP",
                  note="The table switching itself off in one line. The row "
                       "regex matched a quoted level only, so "
                       "`pedantic = { level = \"allow\", priority = 1 }` was "
                       "invisible and the check went on printing that all "
                       "five lints were at or above 27.1's level. MEASURED "
                       "under 1.97.1: that row beside "
                       "`cast_possible_truncation = \"deny\"` takes cargo "
                       "clippy from 101 to 0, because the higher priority is "
                       "applied last."),
            Probe("lint-policy.group-row-with-a-trailing-comment",
                  _group_row_after_a_blank_line_with_a_trailing_comment,
                  script("python3", "scripts/lint_policy_check.py"),
                  "carries the lint GROUP",
                  note="The fifth consecutive route past this guard, and the "
                       "first that took the whole `guards` gate with it. "
                       "`LINT_ROW` anchored on `\\s*$` and a TOML trailing "
                       "comment is not whitespace. On a REQUIRED row that "
                       "failed safe, reporting the row missing. On the group "
                       "row the fifth pass added parsing for it failed OPEN. "
                       "Put after a blank line the declared-constant capture "
                       "missed it too, because a non-greedy `\\n\\n` stopped "
                       "inside the table. MEASURED with cargo on a minimal "
                       "crate under 1.97.1: baseline exit 101, and with "
                       "`pedantic = { level = \"allow\", priority = 1 } "
                       "# keeps noise down` appended, exit 0. In this "
                       "repository the same row left this check at 0, the "
                       "census at 0 and `gate guards` ALL GREEN."),
            Probe("lint-policy.commented-required-row-is-permitted",
                  _required_row_with_a_trailing_comment,
                  script("python3", "scripts/lint_policy_check.py"),
                  "clippy lint(s) at or above HLD 27.1's level",
                  polarity="accept",
                  note="The accept half of the same regex, and it is what "
                       "distinguishes the fix that was made from the one "
                       "that looks like it. Stripping the comment before "
                       "matching keeps the row body under the same anchor. "
                       "Loosening the anchor to `.*$` would pass the group "
                       "probe above and would also read "
                       "`cast_possible_truncation = \"deny\" is what we want` "
                       "as a row. A comment on a required row weakens "
                       "nothing and the guard has to say so."),
            Probe("lint-policy.excluded-named-member",
                  _exclude_a_named_workspace_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="`exclude` was applied to explicitly listed members "
                       "and cargo does not do that. MEASURED: with "
                       "`members = [\"crates/*\", \"tools/oracle\"]` and "
                       "`exclude = [\"tools/oracle\"]`, `cargo metadata "
                       "--no-deps` reports 14 packages with the oracle among "
                       "them and clippy compiles it, while this guard printed "
                       "\"13 workspace member(s) ... 33 .rs file(s)\" and "
                       "exited 0. That is the same pair of numbers the "
                       "header records as the fifth pass's defect, reached "
                       "through a different key, and `gate guards` went red "
                       "only because probe "
                       "`lint-policy.member-outside-crates-group-allow` "
                       "happens to pick `tools/oracle`."),
        ),
        limit="`REFUSED_GROUPS` is a list of nine names that exists only in "
              "the guard, and a probe can only ever write one of them, so "
              "narrowing the list to `clippy::pedantic` would leave "
              "`lint-policy.group-allow` green with eight groups unguarded. "
              "That is the shape `device.owned-accessor` has, and the answer "
              "is the same: the set is in the declared-constant ratchet, so "
              "narrowing it fails the census in the same change. Two of the "
              "nine are measured to reach 27.1's table under clippy 1.97.1 "
              "and the other seven are refused as blanket allows, which the "
              "message says rather than overclaiming. The second limit is a "
              "table row spread over two lines. `LINT_ROW` reads one line, so "
              "`pedantic = { level = \"allow\",` followed by `priority = 1 }` "
              "is invisible to it, MEASURED at exit 0. The declared constant "
              "`Cargo.toml:workspace.lints` is the backstop and was measured "
              "too: the same pair moves its digest from adf2cb2237be28da to "
              "e3d02e83d8b52dab and the census refuses. That division of "
              "labour is why the sixth pass had to fix both the row regex and "
              "the constant's capture, and it is the reason a probe here is "
              "not the whole answer.",
    ),

    # -- this story's own machinery, watched by the same runner ------------
    Guard(
        id="census",
        file="scripts/guards/census.py",
        gate="guards guards-deep",
        spec="`.claude/plans/F-X009-design.md` section 7, completeness proved "
             "mechanically",
        refuses="A refusal site no catalogue entry claims, an entry no site "
                "backs, an entry whose declared kind is not one of the two, a "
                "floor entry needing a GPU, a browser or the corpus, a "
                "declared constant changed or retired without its recorded "
                "value, an executable hook with no entry, an oracle fault "
                "catalogue nothing replays, a catch-all entry whose refusal "
                "count has moved, and an uncovered count that has grown.",
        claims=("*",),
        probes=(
            Probe("census.constants-count-shrunk",
                  _retire_a_declared_constant,
                  script("python3", "scripts/guard_census.py"),
                  "Narrowing the declared-constant ratchet",
                  note="The ratchet on the ratchet, and it was watched by "
                       "nothing until the fifth pass. Deleting a `Constant` "
                       "together with its recorded row is a two-line edit "
                       "that reads as a cleanup and leaves every other check "
                       "here green, because the loop over CONSTANTS no longer "
                       "visits it and the loop over the recorded rows no "
                       "longer sees it. So the one mechanism that notices a "
                       "guard being WIDENED could be disarmed in one green "
                       "commit and the widening land in the next, also "
                       "green."),
            Probe("census.unclaimed-executable-hook",
                  _an_unclaimed_executable_hook,
                  script("python3", "scripts/guard_census.py"),
                  "is executable in a clone that opts in",
                  note="A hook is a refusal a clone that opts in actually "
                       "runs, so one nobody declared is one nobody probed. "
                       "The branch was added by the fourth pass and deleting "
                       "it left the census, the floor probe profile and the "
                       "unit suite all green. The planted hook carries no "
                       "refusal shape, so what this builds is an undeclared "
                       "HOOK and not an undeclared refusal site."),
            Probe("census.oracle-runner-gone", _the_oracle_runner_is_gone,
                  script("python3", "scripts/guard_census.py"),
                  "so the `oracle` gate has no runner",
                  note="This branch was added precisely because the previous "
                       "version failed OPEN: `if runner.is_file() and ...` "
                       "meant deleting the runner skipped the check "
                       "entirely, so removing it was quieter than breaking "
                       "it, while the same loss of faults.mjs three lines "
                       "above was refused. Nothing watched the close."),
            Probe("census.unrecognised-kind", _an_unrecognised_guard_kind,
                  script("python3", "scripts/guard_census.py"),
                  "which is not one of",
                  note="`kind` decides which bucket an entry's refusals land "
                       "in, and a typo is not `\"guard\"`, so the entry "
                       "leaves the uncovered ratchet and is reported as "
                       "declared out of scope. That is the quietest way to "
                       "move refusals out of the count, the validation was "
                       "added by the fourth pass, and deleting the validation "
                       "changed no number anywhere."),
            Probe("census.no-entry-site-count", _drop_the_entry_site_counts,
                  script("python3", "scripts/guard_census.py"),
                  "records no per-entry site count"),
            Probe("census.refusal-in-a-claimed-file",
                  _a_refusal_in_an_already_claimed_file,
                  script("python3", "scripts/guard_census.py"),
                  "refusal site(s) to",
                  note="The mechanism gap the fifth pass named, and the "
                       "reason the four probes above it exist at all. Check a "
                       "makes a guard added next month arrive with its test, "
                       "and that rule did not reach a guard added to a file "
                       "the catalogue ALREADY claims: most entries use "
                       "`claims=(\"*\",)`, so a new `problems.append` lands "
                       "in the probed bucket and no number moves. Measured on "
                       "this harness's own census module, where four new "
                       "refusal branches could each be deleted with the "
                       "census, the floor probes and the unit suite all "
                       "green. The recorded per-entry count is what moves "
                       "now, so the addition is at least visible in the "
                       "diff."),
            Probe("census.unclaimed-site",
                  lambda box: _stage(
                      box, "scripts/probe_new_guard.py",
                      "def check(problems):\n"
                      "    problems.append('a refusal nobody declared')\n"),
                  script("python3", "scripts/guard_census.py"),
                  "no catalogue entry claims",
                  note="This is the mechanism that makes a guard added next "
                       "month arrive with its test. The author sees red in "
                       "CI on the push that adds the refusal."),
            Probe("census.changed-constant", _widen_a_declared_constant,
                  script("python3", "scripts/guard_census.py"),
                  "changed without its recorded value",
                  note="The class of weakening no probe can reach. After the "
                       "allow-list is widened the guard is CORRECT about its "
                       "new, weaker rule, so only a recorded value notices."),
            Probe("census.no-std-set-shrunk", _drop_no_std_from_one_crate,
                  script("python3", "scripts/guard_census.py"),
                  "changed without its recorded value",
                  note="The fix shape for defect G-02. `no_std_check.py` "
                       "loses the crate silently and the recorded set does "
                       "not."),
            Probe("census.uncovered-grew",
                  lambda box: _budget_edit(box, "uncovered",
                                           {"sites": -1,
                                            "sweep_complete": True}),
                  script("python3", "scripts/guard_census.py"),
                  "The ratchet may only decrease",
                  note="The uncovered ratchet, exercised by lowering the "
                       "recorded ceiling rather than by adding an uncovered "
                       "refusal, which would need a second catalogue."),
            Probe("census.no-ceiling",
                  lambda box: _budget_drop(box, "uncovered"),
                  script("python3", "scripts/guard_census.py"),
                  "records no uncovered ceiling"),
            Probe("census.orphan-recorded-constant",
                  lambda box: _budget_add_constant(box),
                  script("python3", "scripts/guard_census.py"),
                  "is not declared in the catalogue's CONSTANTS",
                  note="A recorded value nothing reads is not a ratchet, it "
                       "is a number that looks like one."),
            Probe("census.gate-without-an-entry",
                  lambda box: box.substitute(
                      "bin/ocelli.sh",
                      '  "ci|no|',
                      '  "probe-gate|no|a gate nobody declared"\n  "ci|no|'),
                  script("python3", "scripts/guard_census.py"),
                  "has no catalogue entry and no `delegated` reason"),
            Probe("census.floor-needing-a-gpu", None,
                  python_snippet(
                      "census.profile_agrees",
                      "import sys; sys.path.insert(0, 'scripts');\n"
                      "from guards import census;\n"
                      "bad = census.profile_problems(\n"
                      "    [('probe', 'gpu', 'floor')]);\n"
                      "print('\\n'.join(bad));\n"
                      "sys.exit(1 if bad else 0)\n"),
                  "no GPU, no browser and no corpus",
                  level=1,
                  note="The fragment is the GPU branch's own sentence. `may "
                       "not be in the floor` is in both floor branches, so it "
                       "went on matching with the GPU rule deleted.",
                  control=python_snippet(
                      "census.profile_agrees, healthy",
                      "import sys; sys.path.insert(0, 'scripts');\n"
                      "from guards import census;\n"
                      "bad = census.profile_problems(\n"
                      "    [('probe', 'gpu', 'deep'),\n"
                      "     ('probe', 'none', 'floor')]);\n"
                      "print('\\n'.join(bad));\n"
                      "sys.exit(1 if bad else 0)\n")),
        ),
    ),
    Guard(
        id="census-runbook",
        file="scripts/guard_census.py",
        gate="guards",
        spec="`.claude/plans/F-X009-design.md` decision 3, the probe table is "
             "generated between markers",
        refuses="A runbook whose generated-table markers are gone, so the "
                "table and the catalogue could drift with nothing saying so.",
        claims=("*",),
        probes=(
            Probe("census-runbook.markers-gone",
                  lambda box: box.substitute(
                      "docs/runbooks/guard-verification.md",
                      "<!-- BEGIN GENERATED PROBE TABLE, "
                      "scripts/guard_census.py -->", ""),
                  script("python3", "scripts/guard_census.py",
                         "--check-runbook"),
                  "carries no generated-table markers"),
        ),
    ),
    Guard(
        id="probe-runner",
        file="scripts/guard_probe.py",
        gate="guards guards-deep",
        spec="`.claude/plans/F-X009-design.md` section 4, the recursion",
        refuses="A probe whose guard exited 0, a guard that failed its "
                "unmutated control, an unknown `--only` id, a run in which "
                "zero probes executed, and a harness that left residue in the "
                "developer's repository.",
        claims=("*",),
        probes=(
            Probe("probe-runner.self-test", None,
                  script("python3", "scripts/guard_probe.py", "--self-test"),
                  "OK",
                  polarity="accept",
                  note="The harness's own refusals only ever run on a "
                       "mismatch, which no gate run produces. Same reason "
                       "tools/oracle/check_sidecars.py carries one."),
        ),
    ),
    Guard(
        id="sandbox",
        file="scripts/guards/sandbox.py",
        gate="guards",
        spec="`.claude/plans/F-X009-design.md` section 3, nothing is written "
             "inside the real repository",
        refuses="A git call outside the sandbox, a git call in a directory "
                "this harness did not create, and the `rm --cached` and "
                "`checkout --` pair the runbook records as a false-green "
                "trap.",
        claims=("*",),
        covered_by=("scripts/guard_probe.py --self-test",
                    "scripts/tests/test_guard_catalogue.py"),
    ),
    Guard(
        id="discover",
        file="scripts/guards/discover.py",
        gate="guards",
        spec="`.claude/plans/F-X009-design.md` section 7, completeness proved "
             "mechanically",
        refuses="Nothing on its own. It is the scanner the census refuses "
                "from.",
        claims=("*",),
        silent="This module prints nothing and raises nothing, so it has no "
               "refusal site to claim. It carried five until the S03 review's "
               "second pass: the shape table in its own docstring, counted as "
               "code, which inflated the census headline with prose and made "
               "deleting a documentation row turn the gate red.",
        covered_by=("scripts/tests/test_guard_catalogue.py",),
    ),
    Guard(
        id="catalogue",
        file="scripts/guards/catalogue.py",
        gate="guards",
        spec="`.claude/plans/F-X009-design.md` section 2",
        refuses="A probe builder that mutated nothing, and a fixture drawn "
                "from a citation that has gone away.",
        claims=("*",),
        covered_by=("scripts/tests/test_guard_catalogue.py",),
    ),

    # -- the oracle, adopted rather than copied ----------------------------
    Guard(
        id="oracle.faults",
        file="tools/oracle/run.mjs",
        gate="oracle",
        spec="HLD section 11, and `docs/lld/oracle.md`",
        refuses="A run that reached no row, a decode that produced nothing, a "
                "frame that never presented, a read-back still showing the "
                "sentinel, a volume that did not load, and the environment "
                "refusals that keep the rasteriser honest.",
        claims=("*",),
        covered_by=("tools/oracle/src/faults.mjs (23 injected faults, replayed "
                    "by tools/oracle/tests/faults.mjs on every `oracle` gate)",
                    "tools/oracle/tests/args_test.mjs",
                    "tools/oracle/tests/paths_test.mjs",
                    "tools/oracle/tests/pins_test.mjs"),
        limit="Adopted and not re-declared, and not re-run either, because "
              "the run needs a browser and the `oracle` gate already does it. "
              "A second declaration of the same faults is the same defect as "
              "a second copy of the LUT chain, except that it only runs where "
              "nobody looks. What the census verifies instead is that the "
              "catalogue still carries faults, that each still names the "
              "message fragment proving its own boundary, that "
              "tools/oracle/run.mjs still reaches the runner, and that the "
              "count has not shrunk below the recorded 23.",
    ),
    Guard(
        id="oracle.page",
        file="tools/oracle/page/app.mjs",
        gate="oracle",
        spec="`docs/lld/oracle.md`, every refusal in page/app.mjs is reached "
             "by a fault",
        refuses="A render that did not present, a read-back that is not "
                "comparable, and a frame the page cannot attribute.",
        claims=("*",),
        covered_by=("tools/oracle/src/faults.mjs",),
    ),
    Guard(
        id="oracle.page-volume",
        file="tools/oracle/page/volume.mjs",
        gate="oracle",
        spec="`docs/lld/oracle.md`, the volume boundaries",
        refuses="A volume that did not load, a geometry that is not a "
                "permutation of the declared one, and a reformat that never "
                "presented.",
        claims=("*",),
        covered_by=("tools/oracle/src/faults.mjs",),
    ),
    Guard(
        id="oracle.volume",
        file="tools/oracle/src/volume.mjs",
        gate="oracle",
        spec="`docs/lld/oracle.md`, the volume half of the reference",
        refuses="A volume subject whose members disagree, a spacing or "
                "orientation that does not resolve, and a declared truth the "
                "reference does not reproduce.",
        claims=("*",),
        covered_by=("tools/oracle/tests/volume_test.mjs "
                    "(run inside `bin/ocelli.sh oracle`'s unit pass)",),
    ),
    Guard(
        id="oracle.manifest",
        file="tools/oracle/src/manifest.mjs",
        gate="oracle",
        spec="`corpus/manifest.tsv` is the corpus contract",
        refuses="A manifest the reference half cannot read, and a row it "
                "cannot resolve to a case.",
        claims=("*",),
        covered_by=("tools/oracle/tests/manifest_test.mjs",),
    ),
    Guard(
        id="oracle.geometry",
        file="tools/oracle/src/geometry.mjs",
        gate="oracle",
        spec="HLD section 19, image plane geometry",
        refuses="A geometry the reference cannot express, and one that does "
                "not round-trip.",
        claims=("*",),
        covered_by=("tools/oracle/tests/geometry_test.mjs",),
    ),
    Guard(
        id="oracle.voi",
        file="tools/oracle/src/voi.mjs",
        gate="oracle",
        spec="HLD section 18, the LUT chain's VOI stage",
        refuses="A VOI declaration the reference cannot apply, and a window "
                "the row does not carry.",
        claims=("*",),
        covered_by=("tools/oracle/tests/params_test.mjs",),
    ),
    Guard(
        id="oracle.params",
        file="tools/oracle/src/params.mjs",
        gate="oracle",
        spec="`tools/oracle/render-params.json` is the declared parameter set",
        refuses="A render parameter the page does not implement, and a "
                "parameter set that does not resolve for a row.",
        claims=("*",),
        covered_by=("tools/oracle/tests/params_test.mjs",),
    ),
    Guard(
        id="oracle.unsupported",
        file="tools/oracle/src/unsupported.mjs",
        gate="oracle",
        spec="`tools/oracle/unsupported.json`, strict in both directions",
        refuses="A row declared unsupported that renders, and a row that "
                "fails without a declaration.",
        claims=("*",),
        covered_by=("tools/oracle/tests/unsupported_test.mjs",),
    ),
    Guard(
        id="oracle.output",
        file="tools/oracle/src/output.mjs",
        gate="oracle",
        spec="runbook probe 22, the harness will not empty a directory it did "
             "not write",
        refuses="An output directory that is not the harness's own.",
        claims=("*",),
        covered_by=("tools/oracle/tests/output_test.mjs",),
    ),
    Guard(
        id="oracle.pins",
        file="tools/oracle/src/pins.mjs",
        gate="oracle",
        spec="HLD section 11, the reference is pinned at 5.8.2",
        refuses="A reference stack installed at a version nobody pinned.",
        claims=("*",),
        covered_by=("tools/oracle/tests/pins_test.mjs",),
    ),
    Guard(
        id="oracle.sidecar",
        file="tools/oracle/src/sidecar.mjs",
        gate="oracle",
        spec="`docs/lld/oracle.md`, the sidecar contract",
        refuses="A sidecar the comparator cannot read.",
        claims=("*",),
        covered_by=("tools/oracle/tests/sidecar_test.mjs",),
    ),
    Guard(
        id="oracle.sidecar-redaction",
        file="tools/oracle/check_sidecars.py",
        gate="oracle",
        spec="runbook probe 21, the redaction only ever runs on a mismatch",
        refuses="A cross-read mismatch reported without redacting a real "
                "row's values, and a sidecar pydicom and the reference "
                "disagree about.",
        claims=("*",),
        covered_by=("tools/oracle/check_sidecars.py --self-test, run by "
                    "tools/oracle/run.mjs under both interpreters",),
    ),
    Guard(
        id="oracle.faults-declaration",
        file="tools/oracle/src/faults.mjs",
        gate="oracle",
        spec="`docs/lld/oracle.md`, a filter that selects nothing is refused",
        refuses="A fault name nothing declares, and a filter that selected "
                "nothing reading as success.",
        claims=("*",),
        covered_by=("tools/oracle/tests/faults.mjs",),
    ),

    # -- the benchmark harness ----------------------------------------------
    Guard(
        id="bench.registry",
        file="tools/bench/src/registry.mjs",
        gate="bench",
        spec="HLD section 26, and story E1.6",
        refuses="A registry entry with no subject story, a duplicate subject, "
                "and a runner for a subject whose story has not landed.",
        claims=("*",),
        covered_by=("tools/bench/tests/registry_test.mjs "
                    "(run by the `bench` gate)",),
    ),
    Guard(
        id="bench.record",
        file="tools/bench/src/record.mjs",
        gate="bench",
        spec="HLD section 26, a duration is recorded and not asserted",
        refuses="A record written against a host class it was not measured "
                "on, and a malformed baseline.",
        claims=("*",),
        covered_by=("tools/bench/tests/record_test.mjs",
                    "tools/bench/tests/hostclass_test.mjs"),
    ),
    Guard(
        id="bench.state",
        file="tools/bench/src/state.mjs",
        gate="bench",
        spec="HLD section 26",
        refuses="A run state the harness cannot resume from.",
        claims=("*",),
        covered_by=("tools/bench/tests/state_test.mjs",),
    ),
    Guard(
        id="bench.runner",
        file="tools/bench/run.mjs",
        gate="bench",
        spec="HLD section 26, and `docs/lld/benchmarks.md`",
        refuses="An argument the harness does not accept, a subject that does "
                "not exist, a comparison on a machine that does not own the "
                "baseline, and a browser that is not installed.",
        claims=("*",),
        owner="F-X014",
        reason="Nothing watches these nine refusals. This entry claimed "
               "`scripts/tests/test_bench_check.py (the 7 argument refusals "
               "and the 4 run-time refusals)` until the S03 review's second "
               "pass measured it: that suite never opens `run.mjs`, "
               "`bench_check.py` carries no mirror of its argument "
               "validation, and the four node suites the `bench` gate runs do "
               "not reference it either. Seven plus four is also eleven and "
               "there are nine. A claim of coverage that is false is worse "
               "than the gap it hides, so the gap is recorded instead.",
    ),
    Guard(
        id="bench.cold-start",
        file="tools/bench/src/runners/wasm_cold_start.mjs",
        gate="bench",
        spec="HLD section 26, Appendix A gate A4's cold-start half",
        refuses="A cold-start measurement taken against a stub, an incomplete "
                "artefact copy, and a page that never reported.",
        claims=("*",),
        covered_by=("tools/bench/tests/cold_start_test.mjs",),
        limit="Four of that suite's five tests are in the `bench` gate since "
              "the S03 review's fourth pass, which moved playwright to an "
              "`await import` inside `run()`. The fifth launches a browser "
              "and is opted into with OCELLI_BENCH_BROWSER=1, so the "
              "refusals THIS entry names, a measurement against a stub, an "
              "incomplete artefact copy and a page that never reported, are "
              "still watched only when a developer runs the harness. Owner "
              "F-X014.",
    ),
    Guard(
        id="bench.page",
        file="tools/bench/page/app.mjs",
        gate="bench",
        spec="HLD section 26",
        refuses="A page serving an incomplete copy of the wasm artefact.",
        claims=("*",),
        covered_by=("tools/bench/tests/cold_start_test.mjs",),
        limit="Same browser dependency as the fifth test bench.cold-start "
              "describes. The four tests that joined the `bench` gate do not "
              "serve the page, so this entry's refusal is watched only when "
              "a developer runs the harness. Owner F-X014.",
    ),
    Guard(
        id="panic-probe",
        file="scripts/panic_probe.mjs",
        gate="panic",
        spec="HLD section 23, the panic record survives the trap",
        refuses="A run that measured the probe's stub rather than the module.",
        claims=("*",),
        covered_by=("bin/ocelli.sh gate panic, which builds a second module "
                    "carrying the panic-probe feature and runs this file on "
                    "every floor gate",),
        limit="The stub refusal itself has not been watched red. It fires "
              "only when the module fails to export what the probe imports, "
              "which needs a broken wasm-pack build to construct. Owner "
              "F-X014.",
    ),

    # -- declared out of scope, with the reason -----------------------------
    Guard(
        id="bootstrap-importer",
        file="scripts/import_backlog_xlsx.py",
        gate="-",
        spec="none. It is invoked by no gate.",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="A bootstrap importer over a private spreadsheet that is not "
               "in this repository and is not fetched by anything. No gate "
               "and no CI step invokes it. Its OUTPUT is tracked, and that "
               "output is watched by scripts/backlog_check.py and "
               "scripts/gen_sprint_plan.py, both of which are in the floor "
               "and both of which carry probes above.",
    ),
    Guard(
        id="spikes.a1",
        file="tools/spikes/a1-htj2k/run.mjs",
        gate="-",
        spec="HLD Appendix A gate A1, `docs/spikes/GATES.md`",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="A throwaway spike harness for Appendix A gate A1, invoked by "
               "no gate and by no CI step. `content.spike-output` guards its "
               "output DIRECTORY and is not a backstop for its refusals, "
               "unlike the probed backstops populate-corpus and "
               "bootstrap-importer name. Nothing watches these go red.",
    ),
    Guard(
        id="spikes.compare",
        file="tools/spikes/common/compare.mjs",
        gate="-",
        spec="HLD Appendix A gates A1 and A2, `docs/spikes/GATES.md`",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="Shared helper for the same throwaway spike harnesses, invoked "
               "by no gate and by no CI step. The measurement is throwaway "
               "and the ANSWER is not: this file produced every digest in "
               "both Appendix A answer files, and `docs/spikes/GATES.md`'s A1 "
               "verdict and the decision to file F-X013 rest on them. Its own "
               "suite, `tools/spikes/common/tests/compare_test.mjs`, is run "
               "by nothing, which is pass 1's smell S19 and is unfixed. Out "
               "of scope here means no gate runs the file, not that its "
               "refusals did not matter. Owner F-X014.",
    ),
    Guard(
        id="spikes.extract",
        file="tools/spikes/common/extract.py",
        gate="-",
        spec="HLD Appendix A gates A1 and A2, `docs/spikes/GATES.md`",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="The spike harnesses' corpus extractor, invoked by no gate. "
               "Its refusals protect a throwaway measurement rather than the "
               "repository. `content.spike-output` guards the output "
               "DIRECTORY, which is not a backstop for these refusals, so "
               "nothing watches them go red.",
    ),
    Guard(
        id="spikes.a2",
        file="tools/spikes/a2-jpeg-ls/anchors.py",
        gate="-",
        spec="HLD Appendix A gate A2, `docs/spikes/GATES.md`",
        refuses="Nothing this repository verifies.",
        claims=("*",),
        kind="not-a-guard",
        reason="A throwaway spike harness for Appendix A gate A2, invoked by "
               "no gate. Same argument as spikes.a1, including that its "
               "output directory being guarded is not a backstop for its "
               "refusals.",
    ),
)


# ---------------------------------------------------------------------------
# The declared-constant ratchet
# ---------------------------------------------------------------------------

@dataclass(frozen=True)
class Constant:
    """One value that decides how strict a guard is."""

    guard: str
    file: str
    name: str
    pattern: str
    tunable: bool = False
    why: str = ""


# A probe proves a guard still refuses what it refuses. It cannot notice that
# the guard's own configuration has been WIDENED, because after the widening
# the guard is correct about its new, weaker rule. So the strictness-deciding
# values are recorded, and a change to one fails the census until the recorded
# value is updated in the same change. That puts the widening in the diff and
# in front of a reviewer rather than in a refactor nobody reads.
#
# `unsafe_allowlist_check.py` already says adding a path to ALLOWED "is a
# design-plan decision with a recorded rationale, not an edit to this script
# made to get a build green". Until now nothing enforced that sentence.
#
# `tunable` marks a value somebody may legitimately tune. It is still recorded,
# because the point is that the change appears in a diff, and it still fails
# the census when it changes without its recorded value being updated.
CONSTANTS: tuple[Constant, ...] = (
    Constant("unsafe", "scripts/unsafe_allowlist_check.py", "ALLOWED",
             r"^ALLOWED = \{(.*?)^\}",
             why="HLD 27.2 R5 names exactly two files."),
    Constant("content", "scripts/staged_content_check.py", "DICOM_SUFFIXES",
             r"^DICOM_SUFFIXES = (\{.*?\})"),
    Constant("content", "scripts/staged_content_check.py", "ARTEFACT_PARTS",
             r"^ARTEFACT_PARTS = (\{.*?\})"),
    Constant("content", "scripts/staged_content_check.py", "MAX_BYTES",
             r"^MAX_BYTES = (.*?)$", tunable=True,
             why="A size limit somebody may legitimately raise for a "
                 "particular file, with a reason in the design plan."),
    Constant("content", "scripts/staged_content_check.py",
             "ORACLE_OUTPUT_PREFIXES", r"^ORACLE_OUTPUT_PREFIXES = (.*?)$"),
    Constant("content", "scripts/staged_content_check.py",
             "SPIKE_OUTPUT_PREFIXES", r"^SPIKE_OUTPUT_PREFIXES = (.*?)$"),
    Constant("content", "scripts/staged_content_check.py",
             "COMPARE_OUTPUT_PREFIXES", r"^COMPARE_OUTPUT_PREFIXES = (.*?)$"),
    Constant("content", "scripts/staged_content_check.py",
             "SIZE_EXEMPT_SUFFIXES",
             r"path\.suffix not in (\{[^}]*\})", tunable=True,
             why="The suffixes exempt from the size limit. The script keeps "
                 "them inline rather than as a named constant, so the "
                 "recorded value is the literal set at the comparison."),
    Constant("prose", "scripts/prose_check.py", "INCLUDE_PREFIXES",
             r"^INCLUDE_PREFIXES = (\(.*?\))\n", ),
    Constant("prose", "scripts/prose_check.py", "INCLUDE_EXACT",
             r"^INCLUDE_EXACT = (\{.*?\})\n"),
    Constant("provenance", "scripts/source_provenance_check.py", "BLOCKED",
             r"^BLOCKED = \{(.*?)^\}",
             why="HLD Appendix C.2.1's Read column set to NO."),
    Constant("provenance", "scripts/source_provenance_check.py",
             "BLOCKED_URLS", r"^BLOCKED_URLS = \[(.*?)^\]"),
    Constant("provenance", "scripts/source_provenance_check.py",
             "POLICY_FILES", r"^POLICY_FILES = \{(.*?)^\}",
             why="Files permitted to name a read-blocked project. Widening "
                 "this list to make a build green is the exact shape this "
                 "repository distrusts."),
    Constant("pins", "scripts/pin_and_size_check.py", "EXACT_PINNED",
             r"^EXACT_PINNED = \{(.*?)^\}"),
    Constant("pins", "scripts/pin_and_size_check.py", "TOLERANCE",
             r"^TOLERANCE = (.*?)$", tunable=True,
             why="Growth tolerated before the size gate fails. A story that "
                 "grows the module 5% should say why, and raising this is "
                 "one of the things it might say."),
    Constant("ci-floor", "scripts/ci_floor_check.py", "NOT_IN_FLOOR",
             r"^NOT_IN_FLOOR = (\{.*?\})$",
             why="Deviation D-04. A gate leaving the floor is a decision."),
    Constant("ledger.assert", "scripts/verify_ledger.py", "CORPUS_STATES",
             r"^CORPUS_STATES = (\{.*?\})$"),
    Constant("backlog", "scripts/backlog_check.py", "HOOK_EIDS",
             r"^HOOK_EIDS = (\[.*?\])$",
             why="HLD section 38's five Phase 1 hooks."),
    Constant("backlog", "scripts/backlog_check.py", "DECLARED_DEFECTS",
             r"^DECLARED_DEFECTS = (\{.*?\})$"),
    Constant("backlog", "scripts/backlog_check.py", "VALID_STATUS",
             r"^VALID_STATUS = (\{.*?\})$"),
    Constant("device", "ci/check-device-ownership.sh", "CREATORS",
             r"^CREATORS='(.*?)'$"),
    # The accessor shapes exist only in this grep. HLD 31 names none of them,
    # so `device.owned-accessor` has to write one of the four and narrowing
    # the four to that one left the probe green with three shapes unguarded.
    # This is the mechanism that notices, and it is why CREATORS' exposure was
    # always smaller than the accessor list's.
    Constant("device", "ci/check-device-ownership.sh", "OWNED_ACCESSORS",
             r"grep -qE 'pub fn \(([a-z_|]+)\)'",
             why="The accessor shapes that defeat section 31 without any "
                 "crate calling a creator. Four names, and a probe can only "
                 "ever write one of them."),
    # `validate-handoff`'s required fields, which are a contract nothing else
    # documents. `.claude/commands/complete-feature.md` names six items and
    # this tuple requires five, so the handoff probes have to write the tuple
    # to reach the branch check they are actually about. Recording it is what
    # makes a change to the contract land in a diff.
    Constant("handoff", "scripts/sprint_workflow.py", "HANDOFF_FIELDS",
             r'for field in (\("Branch".*?\)):',
             why="G-04's undocumented contract. A field added or removed here "
                 "changes what every worker must write and is documented "
                 "nowhere else."),
    # THE WHOLE TABLE, and not to the first blank line. A blank line does not
    # end a TOML table, so the non-greedy `\n\n` stopped the capture inside the
    # table it was recording: the S03 review's sixth pass put
    # `pedantic = { level = "allow", priority = 1 } # keeps noise down` after a
    # blank line and inside `[workspace.lints.clippy]`, and this digest did not
    # move. `lint_policy_check.py`'s row regex missed it too, for the trailing
    # comment, so `bin/ocelli.sh gate guards` was ALL GREEN with four of HLD
    # 27.1's five lints switched off. Both halves are closed, and this is the
    # half that still catches a row the row parser cannot read at all.
    #
    # The capture is bounded by the next `[` header and then backtracks to the
    # LAST `name = value` line inside it, so a comment block that introduces
    # the following section and happens to sit before its header is not part of
    # the recorded value. Every row of the table is, wherever the blank lines
    # fall.
    Constant("lint-policy", "Cargo.toml", "workspace.lints",
             r"^\[workspace\.lints\.clippy\]\n((?:(?!^\[)[\s\S])*[\w-]\s*=[^\n]*)",
             why="HLD 27.1's denied lint table, verbatim, and every other row "
                 "of the table it sits in. A row added anywhere in that table "
                 "moves this digest, blank lines and trailing comments "
                 "included."),
    # The group names that exist only in this guard. HLD 27.1 names five
    # lints and no groups, so `lint-policy.group-allow` has to write one of
    # the nine and narrowing the nine to that one would leave the probe green
    # with eight groups unguarded. Same shape as OWNED_ACCESSORS above, same
    # answer. The recorded value is the NAMES and not the explanations beside
    # them, so rewording a message does not move the digest.
    Constant("lint-policy", "scripts/lint_policy_check.py", "REFUSED_GROUPS",
             r"^REFUSED_GROUPS = \{(.*?)^\}",
             why="A group allow disables every lint in the group without "
                 "naming one of them. Measured: `#![allow(clippy::pedantic)]` "
                 "takes cargo clippy from 101 to 0 on a crate that denies "
                 "cast_possible_truncation. Dropping a name here is the "
                 "widening no probe can see."),
    # TWO selectors since the S03 sprint review, and the rename is the point.
    # The single `NO_CACHED_WASM_VIEW` matched only
    # `new DataView(wasm.memory.buffer)` and missed the destructured shape
    # `const { memory } = wasm; new DataView(memory.buffer)`, which is what
    # `packages/core/src/panic.ts` actually writes. So the rule advertised in
    # `gate --list` as "the cached-wasm-view ban (HLD 17.2)" was one
    # destructuring away from silent. Both are recorded, because either one
    # weakening is a weakening of the ban.
    Constant("lint-policy", "eslint.config.js", "NO_CACHED_WASM_VIEW_MEMBER",
             r"^const NO_CACHED_WASM_VIEW_MEMBER = \{\n  selector:\n(.*?)\n  message"),
    Constant("lint-policy", "eslint.config.js", "NO_CACHED_WASM_VIEW_DESTRUCTURED",
             r"^const NO_CACHED_WASM_VIEW_DESTRUCTURED = \{\n  selector:\n(.*?)\n  message"),
    # A THIRD selector since the S03 sprint review's second pass. The two
    # above are keyed on a view built directly over `wasm.memory.buffer` or
    # over a destructured `memory`, and both miss an alias: `const mem =
    # wasm.memory` and `const { buffer } = wasm.memory` escaped them. A view
    # over the alias is the same hazard, so the third selector is recorded on
    # the same footing as the other two.
    Constant("lint-policy", "eslint.config.js", "NO_CACHED_WASM_MEMORY_ALIAS",
             r"^const NO_CACHED_WASM_MEMORY_ALIAS = \{\n  selector:\n"
             r"(.*?)\n  message"),
    Constant("lint-policy", "eslint.config.js", "LINEAR_MEMORY_ALLOWANCE",
             r'files: (\["packages/core/src/bulk\.ts".*?\])',
             why="HLD 17.2 says two functions. F-005 widened this from one "
                 "file to two, and a third is not granted."),
    # The fix shape for defect G-02: `no_std_check.py` builds its crate set
    # from the crates that match the attribute, so a crate deleting it is not
    # reported, it stops being checked. The set is recorded here instead.
    Constant("nostd", "crates/*/src/lib.rs", "NO_STD_CRATES",
             r"#!\[cfg_attr\(not\(test\), no_std\)\]",
             why="Deviation D-09 is a claim about a SET of crates, and the "
                 "guard cannot notice the set shrinking. This can."),
)


DEFECTS = {
    "G-02": "scripts/no_std_check.py loses a crate rather than failing. Its "
            "crate set is built from the crates that match the attribute, so "
            "a crate deleting it is not reported, it stops being checked. The "
            "only backstop fires when NO crate declares it. The recorded "
            "NO_STD_CRATES constant is what catches it today.",
    "G-04": "scripts/sprint_workflow.py validate-handoff has an undocumented "
            "contract. It needs a literal `**Head**` and parses the branch as "
            "a bare token, so backticks break it, and this repository writes "
            "every path in backticks. It cost one handoff rewritten at "
            "integration in S03.",
}
