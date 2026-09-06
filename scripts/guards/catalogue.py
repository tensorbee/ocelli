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
import os
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
    _write_packaged_licences(box)


def _write_packaged_licences(box: Sandbox, *, omit: str = "") -> None:
    for name in ("LICENSE-MIT", "LICENSE-APACHE"):
        if name != omit:
            box.write(f"crates/ocelli-wasm/pkg/{name}",
                      (box.path / name).read_bytes())


def _missing_packaged_apache_licence(box: Sandbox) -> None:
    box.write("crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm", b"\x00" * 1000)
    _write_packaged_licences(box, omit="LICENSE-APACHE")


def _symlinked_packaged_apache_licence(box: Sandbox) -> None:
    box.write("crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm", b"\x00" * 1000)
    _write_packaged_licences(box, omit="LICENSE-APACHE")
    packaged = box.path / "crates/ocelli-wasm/pkg/LICENSE-APACHE"
    packaged.symlink_to("../../../LICENSE-APACHE")


def _stale_operational_parity_target(box: Sandbox) -> None:
    box.substitute(".claude/commands/parity.md", "5.8.2", "5.8.9")


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


def _quirk_record(box: Sandbox) -> dict:
    return json.loads(box.read("corpus/quirks.json"))


def _write_quirk_record(box: Sandbox, record: dict) -> None:
    box.write("corpus/quirks.json", json.dumps(record, indent=2) + "\n")


def _quirk_without_authority(box: Sandbox) -> None:
    record = _quirk_record(box)
    del record["quirks"][0]["expectation"]["authority"]
    _write_quirk_record(box, record)


def _quirk_with_ocelli_as_authority(box: Sandbox) -> None:
    record = _quirk_record(box)
    record["quirks"][0]["expectation"]["authority"] = {
        "kind": "ocelli-output"
    }
    _write_quirk_record(box, record)


def _quirk_with_absent_manifest_path(box: Sandbox) -> None:
    record = _quirk_record(box)
    record["quirks"][0]["generator"]["paths"] = ["synthetic/absent.dcm"]
    _write_quirk_record(box, record)


def _quirk_without_mutation_evidence(box: Sandbox) -> None:
    record = _quirk_record(box)
    record["quirks"][0]["mutations"] = []
    _write_quirk_record(box, record)


def _quirk_without_mutation(kind: str) -> Callable[[Sandbox], None]:
    def mutate(box: Sandbox) -> None:
        record = _quirk_record(box)
        record["quirks"][0]["mutations"] = [
            mutation for mutation in record["quirks"][0]["mutations"]
            if mutation.get("kind") != kind
        ]
        _write_quirk_record(box, record)
    return mutate


def _quirk_with_tracked_generated_dicom(box: Sandbox) -> None:
    path = "corpus/data/synthetic/ct_sigmoid_width_half.dcm"
    box.write(path, DICOM_FIXTURE)
    box.git("add", "-f", path)


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
    """Put one named gate's step behind an `if:`, leaving the step in place.

    Two step SHAPES, and writing only the first was a defect this harness
    could not see until `scripts/ci_floor_check.py` started parsing the
    workflow in the S03 review's twelfth pass. A step whose `run:` is its
    first key opens with `- `, and one that carries a `name:` as well does
    not, so turning that second shape's `run:` line into a new `- ` list item
    produced YAML that GitHub Actions rejects outright. The line-by-line
    reader accepted it and the probe passed, which means three probes here
    were asserting this guard's behaviour on a file that is not a workflow.
    A probe input has to be a state the real system can be IN.
    """
    workflow = box.read(".github/workflows/ci.yml")
    line = next(l for l in workflow.splitlines()
                if f"bin/ocelli.sh gate {gate}" in l)
    indent = " " * (len(line) - len(line.lstrip()))
    if line.lstrip().startswith("- "):
        body = line.lstrip().removeprefix("- ")
        replacement = f"{indent}- if: {condition}\n{indent}  {body}"
    else:
        # The `run:` is not the step's first key, so the condition is a
        # SIBLING key written above it and not a new list item.
        replacement = f"{indent}if: {condition}\n{line}"
    box.substitute(".github/workflows/ci.yml", line, replacement)


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

    The step is chosen by `_gate_step_pieces` below, which requires a step
    whose whole text is one `- run:` line. This builder writes TWO steps in
    place of one, so a step carrying a `name:` key as well cannot be rewritten
    by replacing a single line, and doing it anyway produced a workflow GitHub
    Actions rejects. That went unnoticed for as long as this guard read the
    file line by line.
    """
    _, line, indent, body = _gate_step_pieces(box)
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


def _add_no_std_to_one_other_crate(box: Sandbox) -> None:
    attribute = "#![cfg_attr(not(test), no_std)]"
    lib = box.path / "crates" / "ocelli-compute" / "src" / "lib.rs"
    box.write(
        "crates/ocelli-compute/src/lib.rs",
        f"{attribute}\n{lib.read_text(encoding='utf-8')}",
    )


def _shrink_expected_no_std_set(box: Sandbox) -> None:
    box.substitute(
        "scripts/no_std_check.py",
        '    "ocelli-cache",\n',
        "",
    )


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


def _sprint_plan_hand_curated(box: Sandbox) -> None:
    """Give the bare writer content it must refuse rather than replace."""
    box.substitute(
        "docs/sprints/SPRINT_PLAN.md",
        "# Sprint Plan\n",
        "# Sprint Plan\n\nA hand-curated paragraph the generator cannot recover.\n",
    )


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


def _changed_skill_example_digit(box: Sandbox) -> None:
    box.substitute(
        ".claude/skills/dicom-tooling/SKILL.md",
        "    40   127.820   127.500",
        "    40   127.821   127.500",
    )


def _reverse_skill_sigmoid_exponent(box: Sandbox) -> None:
    box.substitute(
        ".claude/skills/dicom-tooling/SKILL.md",
        "math.exp(-4 * (x - c) / w)",
        "math.exp(4 * (x - c) / w)",
    )


def _reverse_skill_sigmoid_width_precondition(box: Sandbox) -> None:
    box.substitute(
        ".claude/skills/dicom-tooling/SKILL.md",
        '"""PS3.3 C.11.2.1.3.1. Requires w > 0."""\n    assert w > 0',
        '"""PS3.3 C.11.2.1.3.1. Requires w > 0."""\n    assert w < 0',
    )


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


def green_comparison_document(box: Sandbox) -> dict:
    """The serializer-owned green fixture with two real, distinct inputs."""
    contract = json.loads(box.read("tools/oracle/report-contract.json"))
    report = contract["greenReport"]
    reference = ".claude/probe-reference"
    candidate = ".claude/probe-candidate"
    box.write(f"{reference}/.keep", "reference\n")
    box.write(f"{candidate}/.keep", "candidate\n")
    report["reference"] = reference
    report["candidate"] = candidate
    return report


def _comparison_report(box: Sandbox, *, verdict: str = "pass",
                       claimed: int = 1, green: bool = True) -> None:
    report = green_comparison_document(box)
    report["claimedVerdictViews"] = claimed
    report["gateVerdict"] = verdict
    report["green"] = green
    if verdict == "pass":
        report["pass"] = claimed
    else:
        report["pass"] = 0
        report["fail"] = max(claimed, 1)
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _malformed_comparison_report(box: Sandbox) -> None:
    box.write(".claude/probe-comparison.json", "{not json\n")


def _identity_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["operation"] = "identity"
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _red_comparison_report(box: Sandbox) -> None:
    _comparison_report(box, verdict="comparison-failure", green=False)


def _zero_judgement_comparison_report(box: Sandbox) -> None:
    _comparison_report(box, claimed=0)


def _failed_count_in_green_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["pass"] = 0
    report["fail"] = 1
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _coverage_problem_in_green_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["coverageProblems"] = ["a controlled coverage problem"]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _problem_in_green_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["problems"] = ["a controlled input problem"]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _absorbed_divergence_in_green_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["absorbedDivergences"] = ["a controlled absorbed divergence"]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _missing_coverage_problems_in_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    del report["coverageProblems"]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _top_level_absent_in_green_comparison_report(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["absent"] = 1
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _invalid_coverage_count(box: Sandbox, field: str, variant: str) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    coverage = report["coverage"]
    if variant == "missing":
        del coverage[field]
    else:
        values = {
            "null": None,
            "bool": False,
            "negative": -1,
            "string": "0",
        }
        coverage[field] = values[variant]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _contradictory_unmeasured_counts(box: Sandbox) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    report["unmeasured"] = 1
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _invalid_top_level_unmeasured(box: Sandbox, variant: str) -> None:
    _comparison_report(box)
    report = json.loads(box.read(".claude/probe-comparison.json"))
    if variant == "missing":
        del report["unmeasured"]
    else:
        values = {
            "null": None,
            "bool": False,
            "negative": -1,
            "string": "0",
        }
        report["unmeasured"] = values[variant]
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _coverage_count_probes() -> tuple[Probe, ...]:
    probes = []
    for field in ("unmeasured", "unsupportedSourceRows",
                  "declaredVolumeRefusals"):
        for variant in ("missing", "null", "bool", "negative", "string"):
            probes.append(Probe(
                f"ledger.comparison-coverage-{field}-{variant}",
                lambda box, field=field, variant=variant:
                    _invalid_coverage_count(box, field, variant),
                script("python3", "scripts/verify_ledger.py", "record",
                       "--comparison-report",
                       ".claude/probe-comparison.json"),
                (f"comparison report coverage has invalid keys, missing=['{field}']"
                 if variant == "missing"
                 else f"comparison report has invalid coverage count for {field}"),
            ))
    return tuple(probes)


def _report_shape_mutation(box: Sandbox, variant: str) -> None:
    report = green_comparison_document(box)
    record = report["records"][0]
    if variant == "records-missing":
        del report["records"]
    elif variant == "records-null":
        report["records"] = None
    elif variant == "records-empty":
        report["records"] = []
    elif variant == "duplicate-record":
        report["records"].append(record)
    elif variant == "record-fail":
        record["outcome"] = "fail"
    elif variant == "duplicate-record-id":
        report["records"].append(dict(record))
    elif variant == "views-missing":
        del report["views"]
    elif variant == "views-zero":
        report["views"] = 0
    elif variant == "counts-exceed-views":
        report["pass"] = 1000
        report["claimedVerdictViews"] = 1000
    elif variant == "reference-equals-candidate":
        report["candidate"] = report["reference"]
    elif variant == "reference-missing":
        del report["reference"]
    elif variant == "candidate-missing":
        del report["candidate"]
    elif variant == "hashes-missing":
        del report["renderHashes"]
    elif variant == "aggregate-hash-invalid":
        report["renderHashes"]["reference"] = "not-a-hash"
    elif variant == "aggregate-hash-inconsistent":
        report["renderHashes"]["reference"] = "f" * 64
    elif variant == "qualifier-histogram":
        report["qualifiers"] = {"weak": 1}
    elif variant == "top-level-unknown":
        report["invented"] = 0
    elif variant == "coverage-unknown":
        report["coverage"]["invented"] = 0
    elif variant == "record-id-missing":
        del record["id"]
    elif variant == "record-id-null":
        record["id"] = None
    elif variant == "record-id-empty":
        record["id"] = ""
    elif variant == "record-unknown":
        record["invented"] = 0
    elif variant == "record-hash-invalid":
        record["renderHashes"]["candidate"] = "not-a-hash"
    elif variant == "record-hashes-unknown":
        record["renderHashes"]["invented"] = "0" * 64
    elif variant == "statistics-null":
        record["statistics"] = None
    elif variant == "statistics-unknown":
        record["statistics"]["invented"] = 0
    elif variant == "channel-unknown":
        record["statistics"]["full"][0]["invented"] = 0
    elif variant == "parameter-divergence-missing":
        record["parameterDivergences"] = [{
            "field": "/voi/windowWidth",
            "reference": 1,
            "candidate": 2,
            "attributedTo": "ours",
        }]
    elif variant == "geometry-divergence-unknown":
        record["geometryDivergences"] = [{
            "field": "camera.position[0]",
            "reference": 0.0,
            "candidate": 1.0,
            "difference": 1.0,
            "bound": 0.1,
            "invented": 0,
        }]
    else:
        raise AssertionError(f"unknown report-shape mutation {variant}")
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _duplicate_report_key(box: Sandbox, fragment: str) -> None:
    report = json.dumps(green_comparison_document(box))
    box.write(
        ".claude/probe-comparison.json",
        report.replace(fragment, f"{fragment}, {fragment}", 1) + "\n",
    )


def _nonfinite_report_number(box: Sandbox) -> None:
    report = json.dumps(green_comparison_document(box))
    box.write(
        ".claude/probe-comparison.json",
        report.replace('"signedMeanDiff": 0.0',
                       '"signedMeanDiff": NaN', 1) + "\n",
    )


def _report_semantic_mutation(box: Sandbox, variant: str) -> None:
    report = green_comparison_document(box)
    record = report["records"][0]
    statistics = record["statistics"]
    regions = ("full", "image", "informative")
    if variant == "resolved-input-alias":
        report["candidate"] = "./.claude/probe-reference"
    elif variant == "mono-not-monochrome":
        record["monochromeFrame"] = False
    elif variant == "class-two-pass":
        record["toleranceClass"] = "colour-or-us"
        statistics["channels"] = 3
        for region in regions:
            statistics[region] = [dict(statistics[region][0]) for _ in range(3)]
    elif variant == "weak-with-decimated-rung":
        record["outcome"] = "unmeasured"
        record["qualifiers"] = ["weak"]
        record["rung"] = "decimated"
        record["notes"] = ["controlled impossible attribution"]
    elif variant == "mono-with-class-two-rung":
        record["outcome"] = "unmeasured"
        record["qualifiers"] = ["unstated-threshold"]
        record["rung"] = "class-two"
        record["notes"] = ["controlled impossible class"]
    elif variant == "signed-histogram-not-array":
        statistics["full"][0]["signedHistogram"] = None
    elif variant == "signed-histogram-empty":
        statistics["full"][0]["signedHistogram"] = []
    elif variant == "signed-histogram-entry-shape":
        statistics["full"][0]["signedHistogram"] = [[0]]
    elif variant == "signed-histogram-difference":
        statistics["full"][0]["signedHistogram"] = [[256, 1]]
    elif variant == "signed-histogram-count":
        statistics["full"][0]["signedHistogram"] = [[0, True]]
    elif variant == "signed-histogram-zero-count":
        statistics["full"][0]["signedHistogram"] = [[0, 0]]
    elif variant == "signed-histogram-total":
        statistics["full"][0]["signedHistogram"] = [[0, 2]]
    elif variant == "signed-histogram-summary":
        statistics["full"][0]["signedHistogram"] = [[1, 1]]
    elif variant == "signed-histogram-sum-range":
        _set_signed_distribution(statistics["full"][0], [(255, (1 << 32) - 1)])
    elif variant == "maximum-contradicts-counts":
        statistics["full"][0]["maxAbsDiff"] = 255
    elif variant == "percentile-contradicts-counts":
        statistics["full"][0]["percentile999AbsDiff"] = 255
    elif variant == "signed-mean-exceeds-maximum":
        statistics["full"][0]["signedMeanDiff"] = 256.0
    elif variant == "predicate-contradicts-statistics":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(3, 1)])
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["signedMeanDiff"] = 3.0
    elif variant == "bias-contradicts-statistics":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(1, 1)])
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["signedMeanDiff"] = 1.0
    elif variant == "count-exceeds-producer-limit":
        too_many = 1 << 32
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(0, too_many)])
        statistics["imagePixels"] = too_many
        statistics["informativePixels"] = too_many
    elif variant == "signed-mean-not-pixel-derived":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(1, 1)])
            channel["signedMeanDiff"] = 0.5
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["signedMeanDiff"] = 0.5
    elif variant == "signed-mean-contradicts-buckets":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(1, 1)])
            channel["signedMeanDiff"] = 0.0
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
    elif variant == "fractional-pixel-derived-mean":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(3, 1)])
            channel["signedMeanDiff"] = 2.9999995
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["predicatePasses"] = False
        statistics["signedMeanDiff"] = 2.9999995
    elif variant == "percentile-sum-disagree":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(3, 1998), (255, 2)])
            channel["signedMeanDiff"] = 255.0
        statistics["imagePixels"] = 2000
        statistics["informativePixels"] = 2000
        statistics["rowsTouched"] = 40
        statistics["columnsTouched"] = 50
        statistics["predicatePasses"] = False
        statistics["biasPasses"] = False
        statistics["signedMeanDiff"] = 255.0
    elif variant == "negative-zero":
        for region in regions:
            statistics[region][0]["signedMeanDiff"] = -0.0
        statistics["signedMeanDiff"] = -0.0
    elif variant == "single-tail-percentile":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(255, 1)])
            channel["percentile999AbsDiff"] = 3
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["predicatePasses"] = False
        statistics["biasPasses"] = False
        statistics["signedMeanDiff"] = 255.0
    elif variant == "single-tail-unattainable-sum":
        for region in regions:
            _set_tail_difference(statistics[region][0], 3)
            statistics[region][0]["signedMeanDiff"] = 0.0
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["predicatePasses"] = False
    elif variant == "rank-at-end-percentile":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(3, 1), (255, 1)])
            channel["percentile999AbsDiff"] = 3
        statistics["imagePixels"] = 2
        statistics["informativePixels"] = 2
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 2
        statistics["predicatePasses"] = False
        statistics["biasPasses"] = False
        statistics["signedMeanDiff"] = 129.0
    elif variant == "mono-pass-no-informative":
        statistics["informative"] = []
        statistics["informativePixels"] = 0
        statistics["informativeFraction"] = 0.0
    elif variant == "full-histogram-composition":
        _add_zero_background(statistics)
        _set_signed_distribution(statistics["full"][0], [(0, 1), (1, 1)])
    elif variant == "full-mean-composition":
        _add_opposed_background(statistics)
        _set_signed_distribution(statistics["full"][0], [(1, 2)])
    elif variant == "full-maximum-composition":
        image = statistics["image"][0]
        _set_tail_difference(image, 3)
        statistics["informative"][0] = dict(image)
        background = dict(image)
        _set_tail_difference(background, 255)
        statistics["background"] = [background]
        full = statistics["full"][0]
        _set_signed_distribution(full, [(129, 2)])
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 2
        statistics["predicatePasses"] = False
        statistics["biasPasses"] = False
        statistics["signedMeanDiff"] = 3.0
    elif variant == "full-without-background":
        for region, signed_mean in (("full", 1.0), ("image", -1.0),
                                    ("informative", -1.0)):
            _set_one_difference(statistics[region][0], signed_mean)
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["signedMeanDiff"] = -1.0
    elif variant == "informative-histogram-exceeds-image":
        _set_one_difference(statistics["informative"][0], 1.0)
    elif variant == "informative-maximum-exceeds-image":
        for region, maximum in (("full", 3), ("image", 3),
                                ("informative", 4)):
            _set_tail_difference(statistics[region][0], maximum)
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["predicatePasses"] = False
        statistics["biasPasses"] = False
        statistics["signedMeanDiff"] = 4.0
    elif variant == "touched-presence":
        for region in regions:
            _set_one_difference(statistics[region][0], 1.0)
        statistics["signedMeanDiff"] = 1.0
    elif variant == "touched-count":
        for region in regions:
            channel = statistics[region][0]
            _set_signed_distribution(channel, [(1, 2)])
        statistics["imagePixels"] = 2
        statistics["informativePixels"] = 2
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["signedMeanDiff"] = 1.0
    elif variant == "top-signed-mean-source":
        for region in regions:
            _set_one_difference(statistics[region][0], 1.0)
        statistics["rowsTouched"] = 1
        statistics["columnsTouched"] = 1
        statistics["biasPasses"] = False
    elif variant == "weak-above-floor":
        record["outcome"] = "unmeasured"
        record["qualifiers"] = ["weak"]
        record["rung"] = "weak"
        record["notes"] = ["controlled impossible weak attribution"]
    elif variant == "input-cannot-resolve":
        report["candidate"] = ".claude/probe-missing"
    elif variant == "input-is-file":
        report["candidate"] = ".claude/probe-reference/.keep"
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _set_signed_distribution(
        channel: dict, entries: list[tuple[int, int]]) -> None:
    entries = sorted(entries)
    pixels = sum(count for _, count in entries)
    absolute = [0] * 256
    signed_sum = 0
    for difference, count in entries:
        absolute[abs(difference)] += count
        signed_sum += difference * count
    cumulative = 0
    percentile = 255
    for difference, count in enumerate(absolute):
        cumulative += count
        if cumulative / pixels >= 0.999:
            percentile = difference
            break
    channel.update({
        "pixels": pixels,
        "signedHistogram": [[difference, count] for difference, count in entries],
        "maxAbsDiff": next(
            difference for difference in range(255, -1, -1)
            if absolute[difference] > 0
        ),
        "countAtZero": absolute[0],
        "countAtOne": absolute[1],
        "countAtTwo": absolute[2],
        "countOverTwo": sum(absolute[3:]),
        "fractionWithinOneLsb": (absolute[0] + absolute[1]) / pixels,
        "differingFraction": (pixels - absolute[0]) / pixels,
        "signedMeanDiff": signed_sum / pixels,
        "percentile999AbsDiff": percentile,
    })


def _set_one_difference(channel: dict, signed_mean: float) -> None:
    _set_signed_distribution(channel, [(int(signed_mean), 1)])


def _set_tail_difference(channel: dict, maximum: int) -> None:
    _set_signed_distribution(channel, [(maximum, 1)])


def _add_zero_background(statistics: dict) -> None:
    full = statistics["full"][0]
    _set_signed_distribution(full, [(0, 2)])
    statistics["background"] = [dict(statistics["image"][0])]


def _add_opposed_background(statistics: dict) -> None:
    _set_signed_distribution(statistics["full"][0], [(-1, 1), (1, 1)])
    for region in ("image", "informative"):
        _set_one_difference(statistics[region][0], 1.0)
    background = dict(statistics["image"][0])
    _set_one_difference(background, -1.0)
    statistics["background"] = [background]
    statistics["rowsTouched"] = 1
    statistics["columnsTouched"] = 1
    statistics["signedMeanDiff"] = 1.0


def _ledger_from_parent_directory(box: Sandbox) -> subprocess.CompletedProcess:
    body = (
        "from pathlib import Path; import os, runpy, sys; "
        "root = Path.cwd(); os.chdir(root.parent); "
        "sys.argv = [str(root / 'scripts/verify_ledger.py'), 'record', "
        "'--comparison-report', str(root / '.claude/probe-comparison.json')]; "
        "runpy.run_path(str(root / 'scripts/verify_ledger.py'), "
        "run_name='__main__')"
    )
    return box.run(["python3", "-c", body])


def _report_contract_mutation(box: Sandbox, variant: str) -> None:
    report = green_comparison_document(box)
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")
    path = "tools/oracle/report-contract.json"
    raw = box.read(path)
    if variant == "unknown-root":
        contract = json.loads(raw)
        contract["unusedAuthority"] = 0
        raw = json.dumps(contract)
    elif variant == "unknown-schemas":
        contract = json.loads(raw)
        contract["schemas"]["unused"] = []
        raw = json.dumps(contract)
    elif variant == "unknown-vocabularies":
        contract = json.loads(raw)
        contract["vocabularies"]["unused"] = []
        raw = json.dumps(contract)
    elif variant == "unknown-semantics":
        contract = json.loads(raw)
        contract["semantics"]["unused"] = 0
        raw = json.dumps(contract)
    elif variant == "unknown-hash-algorithms":
        contract = json.loads(raw)
        contract["hashAlgorithms"]["unused"] = "sha256"
        raw = json.dumps(contract)
    elif variant == "duplicate-root":
        raw = raw.replace('"version": 1', '"version": 1, "version": 1', 1)
    elif variant == "duplicate-nested":
        raw = raw.replace('"report": [', '"report": [], "report": [', 1)
    elif variant == "wrong-version":
        contract = json.loads(raw)
        contract["version"] = 2
        raw = json.dumps(contract)
    elif variant == "empty-hash-algorithm":
        contract = json.loads(raw)
        contract["hashAlgorithms"]["view"] = ""
        raw = json.dumps(contract)
    elif variant == "green-report-not-object":
        contract = json.loads(raw)
        contract["greenReport"] = []
        raw = json.dumps(contract)
    elif variant == "invalid-schema-array":
        contract = json.loads(raw)
        contract["schemas"]["report"].append("story")
        raw = json.dumps(contract)
    elif variant == "invalid-vocabulary-array":
        contract = json.loads(raw)
        contract["vocabularies"]["kinds"].append("stack")
        raw = json.dumps(contract)
    elif variant == "invalid-channel-count":
        contract = json.loads(raw)
        contract["semantics"]["channelCountByClass"]["mono16"] = 0
        raw = json.dumps(contract)
    elif variant == "invalid-green-qualifier":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredQualifiers"].append("invented")
        raw = json.dumps(contract)
    elif variant == "invalid-semantic-number":
        contract = json.loads(raw)
        contract["semantics"]["informativeFractionFloor"] = -1
        raw = json.dumps(contract)
    elif variant == "states-not-array":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"] = None
        raw = json.dumps(contract)
    elif variant == "state-unknown-key":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"][0]["invented"] = 0
        raw = json.dumps(contract)
    elif variant == "state-duplicate-qualifier":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"][0]["qualifiers"].append("weak")
        raw = json.dumps(contract)
    elif variant == "state-invalid-class":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"][0]["toleranceClass"] = "invented"
        raw = json.dumps(contract)
    elif variant == "state-invalid-qualifier":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"][0]["qualifiers"] = ["bias"]
        raw = json.dumps(contract)
    elif variant == "state-invalid-rung":
        contract = json.loads(raw)
        contract["semantics"]["greenUnmeasuredStates"][0]["rung"] = "invented"
        raw = json.dumps(contract)
    elif variant == "duplicate-state":
        contract = json.loads(raw)
        states = contract["semantics"]["greenUnmeasuredStates"]
        states.append(dict(states[0]))
        raw = json.dumps(contract)
    box.write(path, raw + ("" if raw.endswith("\n") else "\n"))


def _comparison_run_hash(report: dict, side: str, algorithm: str) -> str:
    records = sorted(
        report["records"],
        key=lambda record: (record["kind"], record["id"],
                            record["renderHashes"][side]),
    )
    digest = hashlib.sha256()
    digest.update(algorithm.encode() + b"\0")
    digest.update(len(records).to_bytes(8, "little"))
    for record in records:
        for field in (record["kind"], record["id"],
                      record["renderHashes"][side]):
            encoded = field.encode()
            digest.update(len(encoded).to_bytes(8, "little"))
            digest.update(encoded)
    return digest.hexdigest()


def _green_unmeasured_state_report(box: Sandbox, index: int) -> None:
    report = green_comparison_document(box)
    contract = json.loads(box.read("tools/oracle/report-contract.json"))
    state = contract["semantics"]["greenUnmeasuredStates"][index]
    record = json.loads(json.dumps(report["records"][0]))
    record["id"] = f"probe-unmeasured-{index}"
    record["toleranceClass"] = state["toleranceClass"]
    record["outcome"] = "unmeasured"
    record["qualifiers"] = state["qualifiers"]
    record["rung"] = state["rung"]
    record["notes"] = ["controlled green unmeasured state"]
    if state["toleranceClass"] == "colour-or-us":
        record["monochromeFrame"] = False
        record["statistics"]["channels"] = 3
        for region in ("full", "image", "informative"):
            channel = record["statistics"][region][0]
            record["statistics"][region] = [dict(channel) for _ in range(3)]
    if "weak" in state["qualifiers"]:
        record["statistics"]["informative"] = []
        record["statistics"]["informativePixels"] = 0
        record["statistics"]["informativeFraction"] = 0.0
    report["records"].append(record)
    report["records"].sort(key=lambda item: item["id"])
    report["views"] = 2
    report["unmeasured"] = 1
    report["coverage"]["unmeasured"] = 1
    report["qualifiers"] = {qualifier: 1 for qualifier in state["qualifiers"]}
    for side in ("reference", "candidate"):
        report["renderHashes"][side] = _comparison_run_hash(
            report, side, contract["hashAlgorithms"]["run"]
        )
    box.write(".claude/probe-comparison.json", json.dumps(report) + "\n")


def _report_semantic_probes() -> tuple[Probe, ...]:
    cases = (
        ("resolved-input-alias", "directories are equal"),
        ("mono-not-monochrome", "is not monochrome"),
        ("class-two-pass", "pass record 'probe-view' is inconsistent"),
        ("weak-with-decimated-rung", "has the wrong rung"),
        ("mono-with-class-two-rung", "contradicts its class"),
        ("signed-histogram-not-array", "has no record 'probe-view' statistics.full[0].signedHistogram array"),
        ("signed-histogram-empty", "signedHistogram is empty"),
        ("signed-histogram-entry-shape", "invalid record 'probe-view' statistics.full[0].signedHistogram[0]"),
        ("signed-histogram-difference", "invalid record 'probe-view' statistics.full[0].signedHistogram[0] difference"),
        ("signed-histogram-count", "invalid record 'probe-view' statistics.full[0].signedHistogram[0] count"),
        ("signed-histogram-zero-count", "signedHistogram[0] count is zero"),
        ("signed-histogram-total", "signed histogram does not total pixels"),
        ("signed-histogram-summary", "channel counts contradict signed histogram"),
        ("signed-histogram-sum-range", "signed sum exceeds producer range"),
        ("maximum-contradicts-counts", "maximum contradicts signed histogram"),
        ("percentile-contradicts-counts", "percentile contradicts signed histogram"),
        ("signed-mean-exceeds-maximum", "signed mean contradicts signed histogram"),
        ("predicate-contradicts-statistics", "predicate contradicts statistics"),
        ("bias-contradicts-statistics", "bias verdict contradicts statistics"),
        ("count-exceeds-producer-limit", "invalid record 'probe-view' statistics.full[0].pixels"),
        ("signed-mean-not-pixel-derived", "signed mean contradicts signed histogram"),
        ("signed-mean-contradicts-buckets", "signed mean contradicts signed histogram"),
        ("fractional-pixel-derived-mean", "signed mean contradicts signed histogram"),
        ("percentile-sum-disagree", "signed mean contradicts signed histogram"),
        ("negative-zero", "signedMeanDiff is negative zero"),
        ("single-tail-percentile", "percentile contradicts signed histogram"),
        ("single-tail-unattainable-sum", "signed mean contradicts signed histogram"),
        ("rank-at-end-percentile", "percentile contradicts signed histogram"),
        ("mono-pass-no-informative", "pass record 'probe-view' is inconsistent"),
        ("full-histogram-composition", "full signed histogram is not image plus background"),
        ("full-mean-composition", "full signed histogram is not image plus background"),
        ("full-maximum-composition", "full signed histogram is not image plus background"),
        ("full-without-background", "full region is not the whole image"),
        ("informative-histogram-exceeds-image", "informative signed histogram exceeds image"),
        ("informative-maximum-exceeds-image", "informative signed histogram exceeds image"),
        ("touched-presence", "touched counts contradict differences"),
        ("touched-count", "touched counts contradict differing pixels"),
        ("top-signed-mean-source", "signed mean contradicts its source region"),
        ("weak-above-floor", "is not low-information"),
        ("input-cannot-resolve", "candidate directory cannot be resolved"),
        ("input-is-file", "candidate directory is not a directory"),
    )
    invoke = script("python3", "scripts/verify_ledger.py", "record",
                    "--comparison-report", ".claude/probe-comparison.json")
    refusal_probes = tuple(
        Probe(
            f"ledger.comparison-semantic-{variant}",
            lambda box, variant=variant: _report_semantic_mutation(box, variant),
            invoke,
            expected,
        )
        for variant, expected in cases
    )
    caller_directory = Probe(
        "ledger.comparison-relative-path-from-parent",
        lambda box: box.write(
            ".claude/probe-comparison.json",
            json.dumps(green_comparison_document(box)) + "\n",
        ),
        Invoke(
            "python3 scripts/verify_ledger.py record --comparison-report "
            ".claude/probe-comparison.json from parent",
            _ledger_from_parent_directory,
        ),
        "recorded tree",
        polarity="accept",
    )
    return refusal_probes + (caller_directory,)


def _report_contract_probes() -> tuple[Probe, ...]:
    cases = (
        ("unknown-root", "root has invalid keys"),
        ("unknown-schemas", "schemas has invalid keys"),
        ("unknown-vocabularies", "vocabularies has invalid keys"),
        ("unknown-semantics", "semantics has invalid keys"),
        ("unknown-hash-algorithms", "hashAlgorithms has invalid keys"),
        ("duplicate-root", "duplicate JSON key 'version'"),
        ("duplicate-nested", "duplicate JSON key 'report'"),
        ("wrong-version", "version is not the integer 1"),
        ("empty-hash-algorithm", "hashAlgorithms values are not non-empty strings"),
        ("green-report-not-object", "greenReport is not an object"),
        ("invalid-schema-array", "schemas.report is not a unique non-empty string array"),
        ("invalid-vocabulary-array", "vocabularies.kinds is not a unique non-empty string array"),
        ("invalid-channel-count", "channelCountByClass values are not positive integers"),
        ("invalid-green-qualifier", "greenUnmeasuredQualifiers contains an invalid value"),
        ("invalid-semantic-number", "semantics.informativeFractionFloor is not a finite non-negative number"),
        ("states-not-array", "semantics.greenUnmeasuredStates is not an array"),
        ("state-unknown-key", "greenUnmeasuredStates[0] has invalid keys"),
        ("state-duplicate-qualifier", "greenUnmeasuredStates[0].qualifiers is not a unique"),
        ("state-invalid-class", "green unmeasured state has an invalid class"),
        ("state-invalid-qualifier", "green unmeasured state has invalid qualifiers"),
        ("state-invalid-rung", "green unmeasured state has an invalid rung"),
        ("duplicate-state", "green unmeasured states contain a duplicate"),
    )
    invoke = script("python3", "scripts/verify_ledger.py", "record",
                    "--comparison-report", ".claude/probe-comparison.json")
    return tuple(
        Probe(
            f"ledger.report-contract-{variant}",
            lambda box, variant=variant: _report_contract_mutation(box, variant),
            invoke,
            expected,
        )
        for variant, expected in cases
    )


def _green_unmeasured_state_probes() -> tuple[Probe, ...]:
    contract_path = Path(__file__).resolve().parents[2] / "tools/oracle/report-contract.json"
    contract = json.loads(contract_path.read_text())
    state_count = len(contract["semantics"]["greenUnmeasuredStates"])
    invoke = script("python3", "scripts/verify_ledger.py", "record",
                    "--comparison-report", ".claude/probe-comparison.json")
    return tuple(
        Probe(
            f"ledger.comparison-green-unmeasured-state-{index}",
            lambda box, index=index: _green_unmeasured_state_report(box, index),
            invoke,
            "recorded tree",
            polarity="accept",
        )
        for index in range(state_count)
    )


def _report_shape_probes() -> tuple[Probe, ...]:
    cases = (
        ("records-missing", "root has invalid keys"),
        ("records-null", "has no records array"),
        ("records-empty", "records array is empty"),
        ("duplicate-record", "duplicate record identifiers"),
        ("record-fail", "green record 'probe-view' has outcome fail"),
        ("duplicate-record-id", "duplicate record identifiers"),
        ("views-missing", "root has invalid keys"),
        ("views-zero", "views count is not the record count"),
        ("counts-exceed-views", "pass count contradicts records"),
        ("reference-equals-candidate", "directories are equal"),
        ("reference-missing", "root has invalid keys"),
        ("candidate-missing", "root has invalid keys"),
        ("hashes-missing", "root has invalid keys"),
        ("aggregate-hash-invalid", "invalid aggregate reference render hash"),
        ("aggregate-hash-inconsistent", "aggregate reference render hash contradicts records"),
        ("qualifier-histogram", "qualifier histogram contradicts records"),
        ("top-level-unknown", "root has invalid keys"),
        ("coverage-unknown", "coverage has invalid keys"),
        ("record-id-missing", "records[0] has invalid keys"),
        ("record-id-null", "invalid records[0].id"),
        ("record-id-empty", "invalid records[0].id"),
        ("record-unknown", "records[0] has invalid keys"),
        ("record-hash-invalid", "invalid records[0] candidate render hash"),
        ("record-hashes-unknown", "records[0].renderHashes has invalid keys"),
        ("statistics-null", "statistics is not an object"),
        ("statistics-unknown", "statistics has invalid keys"),
        ("channel-unknown", "statistics.full[0] has invalid keys"),
        ("parameter-divergence-missing", "parameterDivergences[0] has invalid keys"),
        ("geometry-divergence-unknown", "geometryDivergences[0] has invalid keys"),
    )
    invoke = script("python3", "scripts/verify_ledger.py", "record",
                    "--comparison-report", ".claude/probe-comparison.json")
    return tuple(
        Probe(
            f"ledger.comparison-shape-{variant}",
            lambda box, variant=variant: _report_shape_mutation(box, variant),
            invoke,
            expected,
        )
        for variant, expected in cases
    )


def _top_level_unmeasured_probes() -> tuple[Probe, ...]:
    probes = []
    for variant in ("missing", "null", "bool", "negative", "string"):
        probes.append(Probe(
            f"ledger.comparison-unmeasured-{variant}",
            lambda box, variant=variant:
                _invalid_top_level_unmeasured(box, variant),
            script("python3", "scripts/verify_ledger.py", "record",
                   "--comparison-report", ".claude/probe-comparison.json"),
            ("comparison report root has invalid keys, missing=['unmeasured']"
             if variant == "missing"
             else "comparison report has invalid unmeasured count"),
        ))
    return tuple(probes)


def _ledger_without_comparison(box: Sandbox) -> None:
    _ledger_record(box, "pass")


def _commit_without_comparison(box: Sandbox) -> None:
    box.enable_hooks()
    box.write("probe-note.txt", "one\n")
    box.stage_all()
    _ledger_record(box, "pass")
    box.git("commit", "-m", "F-000, a probe")


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


def _write_close_state(
        box: Sandbox,
        *,
        sprint_reviews: list[dict[str, object]] | None = None,
        verifications: list[dict[str, object]] | None = None,
        legacy: bool = False,
) -> None:
    """Write completed sprint state for close-preflight probes."""
    sprint = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)",
                       box.read("docs/sprints/CURRENT_SPRINT.md"), re.M)
    if sprint is None:
        raise AssertionError("CURRENT_SPRINT.md names no sprint")
    name = sprint.group(1)
    allocation = json.loads(box.read("docs/sprints/allocation.json"))
    fids = sorted(s["fid"] for s in allocation["stories"]
                  if s.get("sprint") == name)
    tree = box.git("write-tree").stdout.strip()
    data: dict[str, object] = {
        "sprint": name,
        "phase": "review",
        "features": {
            fid: {
                "state": "completed",
                "reviews": [{"pass": 1, "defects": 0, "smells": 0,
                             "nitpicks": 0}],
            }
            for fid in fids
        },
        "verifications": verifications if verifications is not None else [{
            "profile": "sprint",
            "result": "pass",
            "gates": "all",
            "corpus": "pass",
            "tree": tree,
        }],
    }
    if not legacy:
        data["sprint_reviews"] = (
            sprint_reviews if sprint_reviews is not None else [{
                "pass": 1,
                "defects": 0,
                "smells": 0,
                "nitpicks": 0,
                "tree": tree,
            }]
        )
    box.write(f".claude/scratch/{name}-run.json",
              json.dumps(data, indent=1) + "\n")


def _close_preflight(box: Sandbox) -> "subprocess.CompletedProcess[str]":
    sprint = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)",
                       box.read("docs/sprints/CURRENT_SPRINT.md"), re.M)
    if sprint is None:
        raise AssertionError("CURRENT_SPRINT.md names no sprint")
    state = box.path / ".claude" / "scratch" / f"{sprint.group(1)}-run.json"
    if not state.exists():
        _write_close_state(box)
    return box.run(["python3", "scripts/sprint_workflow.py",
                    "close-preflight", sprint.group(1)])


def _close_legacy_state(box: Sandbox) -> None:
    _write_close_state(box, legacy=True, verifications=[{
        "profile": "sprint", "result": "pass", "gates": "all",
        "corpus": "pass",
    }])


def _close_dirty_sprint_review(box: Sandbox) -> None:
    tree = box.git("write-tree").stdout.strip()
    _write_close_state(box, sprint_reviews=[{
        "pass": 2, "defects": 1, "smells": 0, "nitpicks": 0,
        "tree": tree,
    }])


def _close_stale_sprint_review(box: Sandbox) -> None:
    _write_close_state(box, sprint_reviews=[{
        "pass": 2, "defects": 0, "smells": 0, "nitpicks": 0,
        "tree": "0" * 40,
    }])


def _close_stale_verification(box: Sandbox) -> None:
    _write_close_state(box, verifications=[{
        "profile": "sprint", "result": "pass", "gates": "all",
        "corpus": "pass", "tree": "0" * 40,
    }])


def _close_failed_latest_verification(box: Sandbox) -> None:
    tree = box.git("write-tree").stdout.strip()
    _write_close_state(box, verifications=[
        {"profile": "sprint", "result": "pass", "gates": "all",
         "corpus": "pass", "tree": tree},
        {"profile": "sprint", "result": "fail", "gates": "all",
         "corpus": "pass", "tree": tree},
    ])


def _close_tree_changed_after_evidence(box: Sandbox) -> None:
    _write_close_state(box)
    box.write("probe-close-change.txt", "changes the staged tree\n")
    box.stage_all()


def _close_carried(box: Sandbox, *, recorded: bool) -> None:
    """Put one story in carried state, with or without a tracked reason."""
    sprint = re.search(r"^#\s+Current sprint,\s*(S[\d.]+)",
                       box.read("docs/sprints/CURRENT_SPRINT.md"), re.M)
    if sprint is None:
        raise AssertionError("CURRENT_SPRINT.md names no sprint")
    name = sprint.group(1)
    allocation = json.loads(box.read("docs/sprints/allocation.json"))
    allocated_fids = sorted(
        story["fid"] for story in allocation["stories"]
        if story.get("sprint") == name
    )
    if not allocated_fids:
        raise AssertionError(f"allocation.json puts no story in {name}")
    current_sprint = box.read("docs/sprints/CURRENT_SPRINT.md")
    section = re.search(
        rf"^## Carried forward from {re.escape(name)}\s*$\n"
        r"(.*?)(?=^## |\Z)",
        current_sprint,
        re.M | re.S,
    )
    recorded_fids = set() if section is None else set(re.findall(
        r"^- \*\*(F-X?\d{3}[a-z]?)\*\*\s+\S.*$",
        section.group(1),
        re.M,
    ))

    candidates = sorted(
        fid for fid in allocated_fids
        if (fid in recorded_fids) == recorded
    )
    if recorded and not candidates:
        fid = allocated_fids[0]
        heading = f"## Carried forward from {name}\n"
        reason = f"\n- **{fid}** probe-only recorded reason\n"
        if heading in current_sprint:
            current_sprint = current_sprint.replace(
                heading,
                heading + reason,
                1,
            )
        else:
            current_sprint = current_sprint.rstrip() + (
                f"\n\n{heading}{reason}"
            )
        box.write("docs/sprints/CURRENT_SPRINT.md", current_sprint)
        box.stage_all()
        box.git("commit", "-q", "-m", "probe recorded carry")
        candidates = [fid]
    if not candidates:
        kind = "recorded" if recorded else "unrecorded"
        raise AssertionError(f"sprint has no {kind} carry-forward candidate")

    _write_close_state(box)
    state_path = box.path / ".claude" / "scratch" / f"{name}-run.json"
    data = json.loads(state_path.read_text())
    data["features"][candidates[0]]["state"] = "carried"
    box.write(
        str(state_path.relative_to(box.path)),
        json.dumps(data, indent=1) + "\n",
    )


def _close_unrecorded_carry(box: Sandbox) -> None:
    _close_carried(box, recorded=False)


def _close_recorded_carry(box: Sandbox) -> None:
    _close_carried(box, recorded=True)


def _handoff(box: Sandbox, branch: str) -> None:
    fid = sprint_state(box)
    box.write(f".claude/handoffs/{fid}-ready.md",
              f"# {fid} ready\n\n"
              f"**Branch**: {branch.replace('FID', fid.lower())}\n"
              f"**Base**: sprint/s03\n"
              f"**Head**: 0123456789ab\n"
              f"**Files touched**: scripts/probe.py\n"
              f"**Review**: pass 1, zero defects\n"
              f"**Verify tree**: 0123456789ab\n")
    box.write(".claude/probe-fid", fid)


def _handoff_wrong_branch(box: Sandbox) -> None:
    _handoff(box, "work/not-this-story-agent")


def _handoff_backticked_branch(box: Sandbox) -> None:
    _handoff(box, "`work/FID-agent`")


def _handoff_without_files_touched(box: Sandbox) -> None:
    _handoff(box, "work/FID-agent")
    fid = (box.path / ".claude" / "probe-fid").read_text().strip()
    box.substitute(
        f".claude/handoffs/{fid}-ready.md",
        "**Files touched**: scripts/probe.py\n",
        "",
    )


def _handoff_field_value(box: Sandbox, field: str, old: str, new: str) -> None:
    _handoff(box, "work/FID-agent")
    fid = (box.path / ".claude" / "probe-fid").read_text().strip()
    box.substitute(
        f".claude/handoffs/{fid}-ready.md",
        f"**{field}**: {old}\n",
        f"**{field}**: {new.replace('FID', fid.lower())}\n",
    )


def _handoff_multiple_code_spans(box: Sandbox) -> None:
    _handoff(box, "`work/FID-agent` `forged`")


def _handoff_unmatched_code_span(box: Sandbox) -> None:
    _handoff_field_value(box, "Files touched", "scripts/probe.py",
                         "`scripts/probe.py")


def _handoff_embedded_code_span(box: Sandbox) -> None:
    _handoff_field_value(box, "Head", "0123456789ab", "0123`forged`456")


def _handoff_empty_code_span(box: Sandbox) -> None:
    _handoff_field_value(box, "Review", "pass 1, zero defects", "``")


def _handoff_forged_suffix(box: Sandbox) -> None:
    _handoff_field_value(box, "Base", "sprint/s03", "`sprint/s03`forged")


def _handoff_duplicate_head(box: Sandbox) -> None:
    _handoff(box, "work/FID-agent")
    fid = (box.path / ".claude" / "probe-fid").read_text().strip()
    path = f".claude/handoffs/{fid}-ready.md"
    box.write(path, box.read(path) + "**Head**: forged\n")


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


def _a_manifest_that_is_not_utf8(box: Sandbox) -> None:
    """A `Cargo.toml` that is not UTF-8 text at all.

    `main` read it with an unguarded `CARGO.read_text(encoding="utf-8")` until
    the S03 review's twelfth pass, so this arrived as a
    `UnicodeDecodeError` traceback with no `FAIL:` header. Fail-closed, and
    still the wrong way to tell a maintainer what to do, which is the fifth
    pass's finding in `scripts/ci_floor_check.py` one file over.

    A lone 0x80 byte, which is a continuation byte with no lead byte and is
    therefore not valid UTF-8 in any position. cargo reads a manifest as UTF-8
    too, so this is a file cargo refuses, not one it reads differently.
    """
    box.write("Cargo.toml",
              b"[workspace]\nmembers = [\"crates/*\"]\n# \x80\n")


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


def _quoted_group_row(box: Sandbox) -> None:
    """A group row whose KEY is quoted, planted at the end of the table.

    The route past the row regex AND its declared backstop at once, and the one
    the eleventh review pass named the class from. TOML's quoted key is the
    same key as the bare one, so `"pedantic"` is `pedantic` to cargo.

    MEASURED under the pinned 1.97.1 toolchain on a minimal workspace carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: baseline cargo
    clippy exit 101, and with `"pedantic" = { level = "allow", priority = 1 }`
    appended as the LAST line of `[workspace.lints.clippy]`, exit 0. In this
    repository the same row left `scripts/lint_policy_check.py` at exit 0
    printing "no group row weaker than deny" and `scripts/guard_census.py` at
    exit 0, because `LINT_ROW`'s name class started `[A-Za-z_]` and the
    declared constant's capture ended at the last line beginning with a bare
    key, and `"` is in neither class.

    The POSITION is the whole point and is why this probe plants at the end
    rather than beside a named row: the same row in the MIDDLE of the table
    moved the recorded digest from `adf2cb2237be28da` to `034c52d0054007ea`
    and the census held. Only at the end were both mechanisms blind.
    """
    last = _last_lint_table_row(box)
    box.substitute(
        "Cargo.toml", last,
        last + '\n"pedantic" = { level = "allow", priority = 1 }')


def _quoted_required_row(box: Sandbox) -> None:
    """A REQUIRED row whose key is quoted, which weakens nothing.

    The accept half of the same spelling, and the direction a repair could get
    wrong by refusing the quoting itself rather than reading through it.
    MEASURED under the pinned 1.97.1 toolchain: `"cast_possible_truncation" =
    "deny"` leaves cargo clippy at 101, so the lint is denied and the guard has
    to say so.
    """
    row = re.search(r'^cast_possible_truncation = "[a-z]+"$',
                    box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no `cast_possible_truncation` row in the "
            "quoted-level form, so this probe cannot quote its key.")
    name, _, level = row.group(0).partition(" = ")
    box.substitute("Cargo.toml", row.group(0), f'"{name}" = {level}')


def _dotted_required_row(box: Sandbox) -> None:
    """A REQUIRED row written as a dotted key, which weakens nothing.

    The other legitimate spelling, and the one the guard's declared limit used
    to name as a residual on the refusing side only. MEASURED under the pinned
    1.97.1 toolchain: `cast_possible_truncation.level = "deny"` leaves cargo
    clippy at 101. A parser reading TOML rather than matching it sees the same
    row here as in the quoted, bare and inline forms, and this probe is what
    says the guard did not simply learn one more alternation.
    """
    row = re.search(r'^cast_possible_truncation = ("[a-z]+")$',
                    box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no `cast_possible_truncation` row in the "
            "quoted-level form, so this probe cannot rewrite it as a dotted "
            "key.")
    box.substitute("Cargo.toml", row.group(0),
                   f"cast_possible_truncation.level = {row.group(1)}")


def _two_line_group_row(box: Sandbox) -> None:
    """A group row spread over two lines, which was a DECLARED residual.

    TOML 1.0 puts an inline table on one line, cargo's parser accepts it over
    two, and `LINT_ROW` read one line at a time. MEASURED under the pinned
    1.97.1 toolchain: `pedantic = { level = "allow",` and `priority = 1 }` on
    the next line takes cargo clippy from 101 to 0, and the guard exited 0.
    The declared constant caught it, which is why the guard's limit called it a
    division of labour rather than a hole.

    It is not a residual now, and the reason is worth more than the fix: the
    guard reads the document with `tomllib`, `tomllib` rejects a multi-line
    inline table, and a document cargo accepts and this parser rejects is
    refused rather than read as an empty table. That is the fail-closed
    direction, and the regexes it replaced had it the other way.
    """
    last = _last_lint_table_row(box)
    box.substitute(
        "Cargo.toml", last,
        last + '\npedantic = { level = "allow",\n  priority = 1 }')


def _a_member_manifest_that_is_not_toml(box: Sandbox) -> None:
    """A workspace member whose Cargo.toml cannot be parsed.

    The fail-closed half of reading the member manifests with `tomllib`. The
    regex pair this replaced answered "does not inherit" for a document nobody
    could parse, which is the right direction under a refusal that names the
    wrong thing, and one rule weaker in the same shape would have been silence.

    The member is chosen for the property rather than named, so the probe does
    not go stale when a crate is added or renamed.
    """
    for member in sorted((box.path / "crates").iterdir()):
        candidate = member / "Cargo.toml"
        if not candidate.is_file():
            continue
        rel = candidate.relative_to(box.path).as_posix()
        box.append(rel, "\nthis line = = is not toml\n")
        return
    raise AssertionError(
        "no crate under `crates/` carries a Cargo.toml, so there is no member "
        "manifest for this probe to make unparseable.")


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


def _comment_in_the_lint_path(box: Sandbox) -> None:
    """`#![allow(clippy::/*c*/pedantic)]`, which Rust reads as the group allow.

    The same input as `lint-policy.whitespace-in-the-lint-path` one lexer rule
    further on. Rust removes a comment before it sees a token, so the two are
    the same attribute, and the guard normalised whitespace and nothing else.
    MEASURED under the pinned 1.97.1 toolchain on a crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: baseline exit 101,
    this attribute exit 0,
    `#![allow(clippy::/*c*/cast_possible_truncation)]` exit 0, and the
    newline-and-`//` form exit 0. Planted in a sandbox clone of this repository
    the whole `guards` gate was green.
    """
    _prepend(box, _a_crate_root(box), "#![allow(clippy::/*c*/pedantic)]")


def _unreadable_lint_argument(box: Sandbox) -> None:
    """A block comment holding the `)` the attribute regex stops at.

    The second half of the same route, and it is not closed by stripping
    comments: `INNER_ALLOW` captures up to the first `)` and this puts one
    inside a comment, so what reaches the name split is the fragment
    `clippy::/*` and no set contains it. MEASURED under the pinned 1.97.1
    toolchain: `#![allow(clippy::/*)*/pedantic)]` takes cargo clippy from 101
    to 0 on a crate denying `cast_possible_truncation`. The guard refuses the
    unreadable argument list by name rather than reading a fragment as an
    unrecognised lint.
    """
    _prepend(box, _a_crate_root(box), "#![allow(clippy::/*)*/pedantic)]")


def _rustflags_allow_a_denied_lint(box: Sandbox) -> None:
    """`.cargo/config.toml` lowering a denied lint for the whole workspace.

    cargo reads this file and nothing in `scripts/`, `bin/`, `ci/`,
    `.githooks/` or `.github/` mentioned `rustflags` before the seventh pass.
    MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: no config exits
    101, `[build] rustflags = ["-Aclippy::pedantic"]` exits 0,
    `["-Aclippy::cast_possible_truncation"]` exits 0, the
    `[target.'cfg(all())']` form exits 0 and the bare-string form exits 0.
    There is no `.cargo/` in this repository today, so the state is the file's
    arrival rather than an edit to one.

    The lint written is the group, for the same reason
    `lint-policy.group-allow` writes it: it names none of HLD 27.1's five and
    switches four of them off.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["-Aclippy::pedantic"]\n')


def _a_cargo_config_that_lowers_nothing(box: Sandbox) -> None:
    """A `.cargo/config.toml` carrying no allow of a denied lint.

    The direction that says which fix was made. The guard refuses the FLAG and
    not the file, because a cargo config is the ordinary home for an alias, a
    linker choice, a target runner and `[net]` settings, none of which touches
    a lint level. A guard that refused the file's existence would be refusing a
    legitimate state, which is the runbook's own sentence, and would push a
    real need into a workaround nothing watches. `-D warnings` here is a
    RAISING flag and must be permitted.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["-Dwarnings"]\n\n'
              '[alias]\nprobe-check = "check --workspace"\n')


def _a_path_dependency_member(box: Sandbox) -> None:
    """A crate cargo makes a member through a path dependency.

    `[workspace] members` is not the member set: cargo additionally makes every
    path dependency of a member a member. MEASURED in the S03 review's seventh
    pass on this repository: adding `vendor/probe` and
    `probe-vendored = { path = "../../vendor/probe" }` to
    `crates/ocelli-core/Cargo.toml` made `cargo metadata --no-deps` report 15
    packages while the guard printed "14 workspace member(s)" at exit 0 and the
    census exited 0 beside it. The new crate carries no `[lints] workspace =
    true`, so it compiles under a smaller set of rules, and that is the fifth
    pass's `tools/oracle` defect reached through a different key.

    The crate deliberately carries no `[lints]` section, because the refusal
    this drives is the inheritance one and the point is that the crate was
    never asked.
    """
    root = _a_crate_root(box)
    member = Path(root).parent.parent.as_posix()
    box.write("vendor/probe/Cargo.toml",
              '[package]\nname = "probe-vendored"\nversion = "0.0.0"\n'
              'edition = "2021"\n')
    box.write("vendor/probe/src/lib.rs", "pub fn probe() {}\n")
    manifest = box.read(f"{member}/Cargo.toml")
    line = 'probe-vendored = { path = "../../vendor/probe" }'
    if "[dependencies]" in manifest:
        box.substitute(f"{member}/Cargo.toml", "[dependencies]",
                       f"[dependencies]\n{line}")
    else:
        box.write(f"{member}/Cargo.toml",
                  f"{manifest}\n[dependencies]\n{line}\n")


def _a_module_source_outside_the_member(box: Sandbox) -> tuple[str, str]:
    """Where a `#[path]` module outside the member goes, and its `#[path]`.

    Relative to the crate root file rather than at a fixed depth, so the probe
    does not go stale if the layout moves.
    """
    root = _a_crate_root(box)
    outside = "probe-shared/shared.rs"
    relative = os.path.relpath(outside, Path(root).parent.as_posix())
    return outside, relative


def _a_module_outside_the_member(box: Sandbox) -> None:
    """A module whose source is outside the member, carrying a group allow.

    `#[path]` puts a module's source anywhere, and a walk of
    `member.rglob("*.rs")` never opens it. MEASURED under the pinned 1.97.1
    toolchain: a crate whose `src/lib.rs` reads
    `#[path = "../../shared_outside/shared.rs"] pub mod shared;` and whose
    `shared.rs` holds one `x as i32` exits 101, and with
    `#![allow(clippy::pedantic)]` at the top of that file it exits 0 while the
    guard never reads the file.
    """
    outside, relative = _a_module_source_outside_the_member(box)
    box.write(outside, "#![allow(clippy::pedantic)]\n"
                       "pub fn probe(x: i64) -> i32 { x as i32 }\n")
    box.append(_a_crate_root(box),
               f'\n#[path = "{relative}"]\npub mod probe_shared;\n')


def _a_clean_module_outside_the_member(box: Sandbox) -> None:
    """The same layout with nothing switched off, which must be accepted.

    A module source outside the member directory is legal Rust and says
    nothing about lint levels on its own, so following `#[path]` must not turn
    the layout itself into a refusal.
    """
    outside, relative = _a_module_source_outside_the_member(box)
    box.write(outside, "pub fn probe(x: i64) -> i64 { x + 1 }\n")
    box.append(_a_crate_root(box),
               f'\n#[path = "{relative}"]\npub mod probe_shared;\n')


def _a_cfg_attr_module_outside_the_member(box: Sandbox) -> None:
    """The same module, declared through `#[cfg_attr(all(), path = "...")]`.

    Not a fifth key. It is key three written another way, and the guard read
    one spelling: `MODULE_PATH` required `path` to be the first thing inside
    the attribute, so a `cfg_attr` wrapper made the whole module invisible
    while `INNER_ALLOW` twenty lines above deliberately reads an `allow`
    reached through exactly that wrapper and says so.

    MEASURED under the pinned 1.97.1 toolchain on a minimal workspace carrying
    `cast_possible_truncation = "deny"`, with the module source holding
    `#![allow(clippy::pedantic)]` and one `x as i32`: the plain form is refused
    at guard exit 1, this form gave guard exit 0, and both take `cargo clippy
    --workspace --all-targets -- -D warnings` from 101 to 0. Planted in a full
    clone of this repository the same pair holds, measured at 101 without the
    attribute and 0 with it, and the guard printed its usual "46 .rs file(s)"
    line at exit 0 having read neither file.
    """
    outside, relative = _a_module_source_outside_the_member(box)
    box.write(outside, "#![allow(clippy::pedantic)]\n"
                       "pub fn probe(x: i64) -> i32 { x as i32 }\n")
    box.append(_a_crate_root(box),
               f'\n#[cfg_attr(all(), path = "{relative}")]\n'
               f'pub mod probe_shared;\n')


def _a_clean_cfg_attr_module_outside_the_member(box: Sandbox) -> None:
    """The same `cfg_attr` layout with nothing switched off, which must pass.

    The direction widening `MODULE_PATH` could get wrong. `cfg_attr` is legal
    Rust and says nothing about lint levels, and the guard now resolves the
    file it names and REFUSES a `#[path]` it cannot resolve, so an over-tight
    read of the wrapper would refuse a crate that moves a module behind a
    feature. The `.rs` file count in the OK line is what proves the file was
    read rather than skipped: it moves from 46 to 47, measured.
    """
    outside, relative = _a_module_source_outside_the_member(box)
    box.write(outside, "pub fn probe(x: i64) -> i64 { x + 1 }\n")
    box.append(_a_crate_root(box),
               f'\n#[cfg_attr(all(), path = "{relative}")]\n'
               f'pub mod probe_shared;\n')


def _a_raw_string_module_outside_the_member(box: Sandbox) -> None:
    """The same module, declared through `#[path = r"..."]`.

    The second spelling of key three, and the file already knew this one too:
    `INCLUDE_PATH` below `MODULE_PATH` carries `(?:r#*)?` for exactly
    this, because a raw string literal is the same file name written another
    way. MEASURED under the pinned 1.97.1 toolchain, on the minimal workspace
    and again in a full clone of this repository: guard exit 0 with cargo
    clippy taken from 101 to 0, the guard printing "46 .rs file(s)" having
    read neither file.
    """
    outside, relative = _a_module_source_outside_the_member(box)
    box.write(outside, "#![allow(clippy::pedantic)]\n"
                       "pub fn probe(x: i64) -> i32 { x as i32 }\n")
    box.append(_a_crate_root(box),
               f'\n#[path = r"{relative}"]\npub mod probe_shared;\n')


def _deny_a_group_in_rustflags(box: Sandbox) -> None:
    """`-Dclippy::pedantic` in a cargo config, which RAISES and must pass.

    The acceptance direction of the tenth pass's message split, and the reason
    the refusal was not narrowed instead. `clippy::pedantic` defaults to allow,
    so a project may legitimately turn it on from a cargo config, and the flag
    that does that is `-D`. MEASURED under the pinned 1.97.1 toolchain on a
    minimal crate carrying `cast_possible_truncation = "deny"` and one
    `x as i32`: `["-Dclippy::pedantic"]` exits 101 with the `clippy` gate's own
    `-D warnings` and 101 without it, so it weakens nothing in either
    invocation.

    `-W` on the same group is a different measurement and stays refused:
    without `-D warnings` it takes the same crate from 101 to 0 and prints
    `warning: casting i64 to i32 may truncate the value` where the error was,
    because a group flag outranks the manifest for every member lint. This
    probe is what stops the obvious over-tight repair, refusing any rustflag
    naming a `REFUSED_GROUPS` member, and the obvious over-loose one, dropping
    groups from `ALLOWING_FLAGS`'s reach, from both reading as correct.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["-Dclippy::pedantic"]\n')


def _a_crate_root_the_manifest_moves(box: Sandbox) -> tuple[str, str, str]:
    """Where a `[lib] path` outside the member goes, and the manifest to edit.

    Returns the member directory, its manifest and the path to write into
    `[lib] path`, all derived from the repository rather than named, so this
    does not go stale if the layout moves. The destination is relative to the
    manifest, which is what cargo resolves a `[lib] path` against.
    """
    root = _a_crate_root(box)
    member = Path(root).parent.parent.as_posix()
    outside = "probe-root/lib.rs"
    relative = os.path.relpath(outside, member)
    return member, f"{member}/Cargo.toml", relative


def _crate_root_outside_the_member(box: Sandbox) -> None:
    """A `[lib] path` pointing outside the member, carrying a group allow.

    The eighth route past this guard and the third key to the fifth pass's
    `tools/oracle` defect. `member_sources` walked `member.rglob("*.rs")` plus
    every `#[path]` it could follow, and a manifest's `[lib] path` puts the
    crate ROOT anywhere, so the walk read the file that is no longer compiled
    and never opened the one that is.

    MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"`: with
    `[lib] path = "../../outside/lib.rs"` and that file holding
    `#![allow(clippy::pedantic)]` and one `x as i32`,
    `cargo clippy --workspace --all-targets -- -D warnings` exits 0 against a
    baseline of 101, and `cargo metadata --no-deps` reports the moved file as
    the package's only `src_path`. In a full copy of this repository the guard
    exited 0 printing "46 .rs file(s), `#[path]` modules followed".

    The attribute is the group allow rather than a named lint, for the reason
    `lint-policy.group-allow` gives: it names none of HLD 27.1's five and
    switches four of them off.
    """
    member, manifest, relative = _a_crate_root_the_manifest_moves(box)
    box.write("probe-root/lib.rs",
              "#![allow(clippy::pedantic)]\n"
              "pub fn probe(x: i64) -> i32 { x as i32 }\n")
    box.substitute(manifest, "[lints]",
                   f'[lib]\npath = "{relative}"\n\n[lints]')


def _clean_crate_root_outside_the_member(box: Sandbox) -> None:
    """The same layout with nothing switched off, which must be accepted.

    A `[lib] path` outside the member is legal cargo and says nothing about
    lint levels, so seeding the walk from `targets[].src_path` must not turn
    the layout itself into a refusal. The member's own `src/lib.rs` is left
    where it is, unread by cargo and still read by the rglob, which is the
    state that would make a guard refusing the SHAPE rather than the attribute
    look correct.
    """
    member, manifest, relative = _a_crate_root_the_manifest_moves(box)
    box.write("probe-root/lib.rs", "pub fn probe(x: i64) -> i64 { x + 1 }\n")
    box.substitute(manifest, "[lints]",
                   f'[lib]\npath = "{relative}"\n\n[lints]')


def _crate_root_the_guard_cannot_open(box: Sandbox) -> None:
    """A `[lib] path` naming a file that is not there.

    cargo metadata answers happily, MEASURED under the pinned 1.97.1
    toolchain: it reports the missing path as the package's `src_path` without
    checking it. So a root the guard cannot open is a state the guard can
    actually reach, and it is the same argument as an unresolvable `#[path]`:
    a file clippy compiles that this pass did not read, which is exactly the
    state the group-allow measurement above was taken in.
    """
    _, manifest, relative = _a_crate_root_the_manifest_moves(box)
    box.substitute(manifest, "[lints]",
                   f'[lib]\npath = "{relative}"\n\n[lints]')


def _cap_lints_allow(box: Sandbox) -> None:
    """`--cap-lints allow` in `.cargo/config.toml`, which names no lint at all.

    The guard refused `-A`, `--allow`, `-W` and `--warn`, which are the flags
    that NAME a lint. `--cap-lints` caps every lint in the crate graph, HLD
    27.1's five included, and sat in neither the refused set nor the declared
    limit. MEASURED under the pinned 1.97.1 toolchain on a minimal crate
    carrying `cast_possible_truncation = "deny"` and one `x as i32`, with the
    `clippy` gate's own `-D warnings` passed: no config exits 101,
    `rustflags = ["--cap-lints", "allow"]` exits 0, `["--cap-lints=allow"]`
    exits 0 and the bare string `"--cap-lints allow"` exits 0. Planted in a
    full copy of this repository the guard exited 0 printing "1 cargo
    config(s) lower no denied lint through rustflags", which asserts the false
    thing positively rather than staying silent about it.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["--cap-lints", "allow"]\n')


def _cap_lints_warn(box: Sandbox) -> None:
    """`--cap-lints=warn`, the other weakening level and the other spelling.

    There are four levels, so two probes cover every value that weakens the
    table and the pair needs no place in the declared-constant ratchet.
    MEASURED under the pinned 1.97.1 toolchain: `warn` takes cargo clippy from
    101 to 0 as `allow` does, because the cap is applied AFTER the `-D
    warnings` the `clippy` gate passes rather than before it, which is the
    reading that makes `warn` look harmless. The `=` spelling is here rather
    than beside `allow` so both forms of the flag are exercised.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["--cap-lints=warn"]\n')


def _cap_lints_deny(box: Sandbox) -> None:
    """`--cap-lints deny`, which is the direction that says which fix was made.

    The guard refuses a `--cap-lints` LEVEL that weakens and not the flag, on
    the same argument that made it refuse the flag and not the file. MEASURED
    under the pinned 1.97.1 toolchain: `deny` leaves cargo clippy at 101 and
    so does `forbid`, so refusing either would be refusing a legitimate state.
    """
    box.write(".cargo/config.toml",
              '[build]\nrustflags = ["--cap-lints", "deny"]\n')


def _force_warn_a_denied_lint(box: Sandbox) -> None:
    """`--force-warn` on a denied lint, the flag that was expected to raise.

    It was put down beside `-D` and `-F` as a raising flag and it is not one.
    MEASURED under the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`, with `-D warnings`
    passed exactly as the `clippy` gate passes it:
    `["--force-warn", "clippy::cast_possible_truncation"]` exits 0, the `=`
    form exits 0 and `["--force-warn", "clippy::pedantic"]` exits 0, while
    `["-Dwarnings"]` and `["-Fclippy::cast_possible_truncation"]` both stay at
    101. It FORCES the level to warn and outranks `-D warnings`, so it
    silences a denied lint exactly as `-A` does.

    The lint is read from HLD 27.1's table in `Cargo.toml` rather than named,
    for the reason `_lint_policy_weakened` gives: the input comes from the
    specification's copy and not from the guard's transcription of it.
    """
    row = re.search(r'^([a-z_]+) = "deny"$', box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no denied clippy lint in the quoted form, so "
            "this probe has no lint to force to warn.")
    box.write(".cargo/config.toml",
              f'[build]\nrustflags = ["--force-warn", '
              f'"clippy::{row.group(1)}"]\n')


def _rustflags_that_are_empty(box: Sandbox) -> None:
    """`rustflags = []`, which lowers nothing and was refused as unreadable.

    `_flag_values` returned `tokens or None` and `None` is this guard's "a
    value this parser cannot read", so an empty list, which is what a config
    is left holding when the last flag is removed, was reported as a rustflags
    value reaching rustc unread. An empty ANSWER is not no answer, and a guard
    that refuses a legitimate state is the runbook's own sentence.
    """
    box.write(".cargo/config.toml", "[build]\nrustflags = []\n")


def _dotted_lints_inheritance(box: Sandbox) -> None:
    """`lints.workspace = true` above `[package]`, which cargo does inherit.

    TOML's dotted spelling of the same table, and the guard's regex wanted a
    `[lints]` header with `workspace = true` under it. MEASURED under the
    pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`: with this line
    above `[package]`, cargo clippy exits 101, so the table IS inherited,
    while the guard found nothing and refused a legitimate manifest at exit 1.
    Written above the first table header deliberately: under `[package]` the
    same line is `package.lints`, which cargo reports as an unused manifest
    key and does NOT inherit, and refusing that spelling is right.
    """
    member = Path(_a_crate_root(box)).parent.parent.as_posix()
    manifest = f"{member}/Cargo.toml"
    box.substitute(manifest, "[lints]\nworkspace = true\n", "")
    box.write(manifest, "lints.workspace = true\n\n" + box.read(manifest))


def _an_included_source(box: Sandbox) -> tuple[str, str, str]:
    """Where an `include!`d file goes, its argument, and the crate root.

    Derived from the repository rather than named, exactly as
    `_a_module_source_outside_the_member` derives its own. rustc resolves an
    `include!` argument against the directory of the file the macro is written
    in, which is not the rule `#[path]` uses, so the relative path is computed
    from the crate root's directory and from nothing else.
    """
    root = _a_crate_root(box)
    outside = "probe-included/hidden.rs"
    relative = os.path.relpath(outside, Path(root).parent.as_posix())
    return outside, relative, root


def _an_include_macro_hiding_a_group_allow(box: Sandbox) -> None:
    """`include!` of a file that switches HLD 27.1's table off for a module.

    The FOURTH key to the set of files this guard reads, after the member glob,
    `targets[].src_path` and `#[path]`. `include!` pastes another file's tokens
    in at that point, so the file is compiled and is named by none of the other
    three. MEASURED under the pinned 1.97.1 toolchain on a minimal crate
    carrying `cast_possible_truncation = "deny"`: with `crates/a/src/lib.rs`
    reading `include!("../../../outside/hidden.rs")`, `hidden.rs` carrying
    `#[allow(clippy::pedantic)]` on a `#[path = "inner.rs"] pub mod` and the
    module holding one `x as i32`, `cargo clippy --workspace --all-targets --
    -D warnings` exits 0 against a baseline of 101. In a full copy of this
    repository the same pair holds, measured at 101 without the attribute and
    0 with it, and the guard exited 0 printing its usual "46 .rs file(s)" line
    having read neither file.

    The attribute is the OUTER form on a `mod` rather than an inner one, and
    that is rustc's constraint rather than a choice: an inner `#![allow(...)]`
    at the top of an `include!`d file is rejected, measured. The two have the
    same scope, which is the fifth pass's finding.
    """
    outside, relative, root = _an_included_source(box)
    box.write(outside, "#[allow(clippy::pedantic)]\n"
                       "#[path = \"inner.rs\"]\npub mod probe_inner;\n")
    box.write("probe-included/inner.rs",
              "pub fn probe(x: i64) -> i32 { x as i32 }\n")
    box.append(root, f'\ninclude!("{relative}");\n')


def _a_clean_include_macro(box: Sandbox) -> None:
    """The same layout with nothing switched off, which must be accepted.

    `include!` is legal Rust and says nothing about lint levels, so following
    it must not turn the construct itself into a refusal. Without this the
    obvious repair, refusing any `include!` at all, would pass the probe above
    and would refuse a crate that generates code, which is the runbook's
    sentence about a guard that fails on everything.
    """
    outside, relative, root = _an_included_source(box)
    box.write(outside, "pub fn probe(x: i64) -> i64 { x + 1 }\n")
    box.append(root, f'\ninclude!("{relative}");\n')


def _an_include_macro_naming_no_file(box: Sandbox) -> None:
    """`include!` of a path that resolves to nothing.

    The same argument as an unresolvable `#[path]`: it names a file that is
    compiled and this pass did not read, and an unread file is the state every
    measurement in the guard's header was taken in. Refused rather than skipped.
    """
    _, relative, root = _an_included_source(box)
    box.append(root, f'\ninclude!("{relative}");\n')


def _an_include_macro_the_guard_cannot_resolve(box: Sandbox) -> None:
    """`include!(concat!(env!("OUT_DIR"), "/generated.rs"))`.

    The argument a build script writes, and the shape that says which repair
    was made. Only the build knows where `OUT_DIR` is, so the guard cannot
    resolve it and refuses rather than passing over an `include!` whose file
    name it could not read. Generated code carrying a group allow is the same
    hole through a path nobody typed.
    """
    root = _a_crate_root(box)
    box.append(root,
               '\ninclude!(concat!(env!("OUT_DIR"), "/probe_generated.rs"));\n')


def _a_member_source_that_is_not_utf8(box: Sandbox) -> None:
    """A `.rs` file under a member that cannot be decoded.

    `member_sources` caught `(OSError, UnicodeDecodeError)` and CONTINUED with
    the path still in its result, and `main` then read the same file again with
    no guard at all, so this state was a raw traceback at exit 1 rather than a
    refusal under the FAIL header. That is the presentation
    `scripts/ci_floor_check.py` stopped giving in the fifth pass, and the guard
    was fail-open in one place and fail-closed by accident in the other. The
    file is read once now and a file that cannot be read is a refusal.
    """
    member = Path(_a_crate_root(box)).parent.as_posix()
    box.write(f"{member}/probe_not_utf8.rs", b"// \xff\xfe not utf-8\n")


def _dotted_rustflags_key(box: Sandbox) -> None:
    """`build.rustflags` written as a dotted key, which the `^` anchor missed.

    `RUSTFLAG_KEY` was `^[^\\S\\n]*(?:rustflags|RUSTFLAGS)\\s*=\\s*`, so the key
    had to start its line and TOML's dotted spelling never did. MEASURED under
    the pinned 1.97.1 toolchain on a minimal crate carrying
    `cast_possible_truncation = "deny"` and one `x as i32`, with the `clippy`
    gate's own `-D warnings` passed: `build.rustflags =
    ["-Aclippy::cast_possible_truncation"]` takes cargo clippy from 101 to 0
    and `build.rustflags = ["-Aclippy::pedantic"]` does the same, while
    `_flag_values` returned `[]` for both. Planted in a full copy of this
    repository the guard printed "1 cargo config(s) lower no denied lint
    through rustflags" at exit 0, which is the eighth pass's `--cap-lints`
    outcome exactly: a positive assertion of the false thing.
    """
    box.write(".cargo/config.toml",
              'build.rustflags = ["-Aclippy::pedantic"]\n')


def _quoted_rustflags_key(box: Sandbox) -> None:
    """`"rustflags"` written as a quoted key under `[build]`.

    TOML's third spelling of the same key, and the second the anchor could not
    see: the key does not start the line's first non-space character as a bare
    word. MEASURED under the pinned 1.97.1 toolchain, `[build]` with
    `"rustflags" = ["-Aclippy::pedantic"]` takes cargo clippy from 101 to 0.
    A parsed document has one key here where the regex had three spellings,
    which is why the repair was `tomllib` rather than a fourth alternative.
    """
    box.write(".cargo/config.toml",
              '[build]\n"rustflags" = ["-Aclippy::pedantic"]\n')


def _a_cargo_config_that_is_not_toml(box: Sandbox) -> None:
    """A cargo config that does not parse.

    cargo refuses a config it cannot parse, so this is not a working state, and
    a parse failure read as a file declaring no rustflags would answer a
    question about an empty set in the language of success. The regex this
    replaced had no notion of a document at all, so a malformed config was
    silently a config with no `rustflags` in it.

    The document is written DOTTED and with the array left open, which is the
    combination that tells the two repairs apart. Written bracketed the old
    regex matched the key, failed to close the array and refused already, so a
    bracketed malformed config discriminates nothing. Dotted, it matched
    nothing at all and the guard exited 0 over a file holding
    `-Aclippy::pedantic`.
    """
    box.write(".cargo/config.toml",
              'build.rustflags = ["-Aclippy::pedantic"\n')


def _a_dotted_cargo_config_that_lowers_nothing(box: Sandbox) -> None:
    """Dotted keys throughout, and not one of them lowers a lint.

    The direction the `tomllib` repair could get wrong. A dotted key is
    ordinary TOML and a cargo config is the ordinary home for a job count, an
    alias and a linker choice, so reading the document must not turn the
    SPELLING into a refusal. `-Dwarnings` here raises a level and must be
    permitted, exactly as it is in the bracketed form.
    """
    box.write(".cargo/config.toml",
              'build.jobs = 4\n'
              'build.rustflags = ["-Dwarnings"]\n'
              'alias.probe-check = "check --workspace"\n')


def _a_required_row_with_a_tail(box: Sandbox) -> None:
    """A required row whose line carries text that is not a comment.

    What `lint-policy.commented-required-row-is-permitted` claims to
    discriminate and does not. MEASURED in the seventh pass: deleting `_row_body`
    entirely and loosening `LINT_ROW`'s tail to `.*$` leaves both that accept
    probe and `lint-policy.group-row-with-a-trailing-comment` green, so the
    quote-aware twenty-line function the sixth pass added is deletable with the
    harness silent. This is the input that tells the two repairs apart: with the
    tail anchored the row is not a row and the guard says the lint is absent,
    and with the tail loosened to `.*$` the same line reads as a `deny` row and
    the guard says nothing at all.

    `and we mean it` and not a `#` comment, deliberately. A comment is legal
    TOML and is removed by the parser, so it would test nothing here.
    """
    row = re.search(r'^cast_possible_truncation = "[a-z]+"$',
                    box.read("Cargo.toml"), re.M)
    if row is None:
        raise AssertionError(
            "Cargo.toml carries no `cast_possible_truncation` row in the "
            "quoted form, so this probe has no row to put a tail on.")
    box.substitute("Cargo.toml", row.group(0),
                   f"{row.group(0)} and we mean it")


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


def _leave_one_visible_arm_command(box: Sandbox, gate: str,
                                   keep: str | None = None) -> None:
    """Reduce an arm to one visible command without removing other work.

    F-X010 requires a named invocation as soon as an arm has several visible
    commands. Older probes that expand a named step must keep one visible
    command or that stronger rule masks the parser boundary they are about.
    Shell builtins remain real arm statements but are outside the extractor's
    deliberately small command vocabulary.
    """
    commands = _ci_arm_commands(box, gate)
    runner = box.read("bin/ocelli.sh")
    survivor = (keep if keep is not None else
                next((command for command in commands
                      if command not in runner), commands[0]))
    for command in commands:
        if command != survivor and command in box.read("bin/ocelli.sh"):
            box.substitute("bin/ocelli.sh", command, "true")


def _named_visible_multi_command_floor_gate(
        box: Sandbox) -> tuple[str, str, list[str]]:
    """A named floor gate with several visible and no invisible commands.

    The absence of an invisible command isolates F-X010's rule from the older
    extractor-vocabulary rule. The returned workflow line is the real named
    step, so every mutation starts from a control the guard accepts.
    """
    import ci_floor_check
    runner = box.read("bin/ocelli.sh")
    workflow = box.read(".github/workflows/ci.yml")
    excluded = _not_in_floor(box)
    invisible = ci_floor_check.unseen_commands(runner)
    for line in workflow.splitlines():
        match = re.search(r"bin/ocelli\.sh gate ([a-z-]+)", line)
        if match is None or not line.lstrip().startswith("- run:"):
            continue
        gate = match.group(1)
        commands = _ci_arm_commands(box, gate)
        if (gate not in excluded and len(commands) >= 2
                and gate not in invisible):
            return gate, line, commands
    raise AssertionError(
        "no floor gate with several visible and no invisible arm commands is "
        "invoked by name, so the F-X010 probes cannot isolate their rule.")


def _split_a_multi_command_gate_across_steps(box: Sandbox) -> None:
    """Replace one named gate step with its exact commands in arm order."""
    gate, _, _ = _named_visible_multi_command_floor_gate(box)
    _expand_the_step_for(box, gate)


def _reorder_a_multi_command_gate_across_steps(box: Sandbox) -> None:
    """Replace one named gate step with its exact commands in reverse order."""
    _, line, commands = _named_visible_multi_command_floor_gate(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(
        ".github/workflows/ci.yml", line,
        "\n".join(f"{indent}- run: {command}"
                  for command in reversed(commands)))


def _split_a_multi_command_gate_across_jobs(box: Sandbox) -> None:
    """Put exact arm commands in separate jobs with no ordering edge."""
    _, line, commands = _named_visible_multi_command_floor_gate(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(".github/workflows/ci.yml", line,
                   f"{indent}- run: {commands[0]}")
    workflow = box.read(".github/workflows/ci.yml").rstrip()
    second_job = (
        "\n\n  f_x010_split:\n"
        "    runs-on: ubuntu-latest\n"
        "    steps:\n" +
        "".join(f"      - run: {command}\n" for command in commands[1:]))
    box.write(".github/workflows/ci.yml", workflow + second_job)


def _name_a_multi_command_gate_step(box: Sandbox) -> None:
    """Add a display name while keeping the gate in its existing CI job."""
    gate, line, _ = _named_visible_multi_command_floor_gate(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(
        ".github/workflows/ci.yml", line,
        f"{indent}- name: Run the {gate} gate through its arm\n"
        f"{indent}  run: bin/ocelli.sh gate {gate}")


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
    F-X010 gives every visible multi-command arm its own stronger reason to
    require a name. One visible command is replaced with the shell builtin
    `true` first, leaving exactly one visible command plus the invisible work,
    so this probe still discriminates the extractor-vocabulary rule rather
    than passing for F-X010's new reason.
    """
    gate = _a_gate_with_an_unextractable_arm_command(box)
    runner = box.read("bin/ocelli.sh")
    commands = _ci_arm_commands(box, gate)
    removable = next(command for command in commands if command in runner)
    box.substitute("bin/ocelli.sh", removable, "true")
    _expand_the_step_for(box, gate)


def _expand_the_step_for(box: Sandbox, gate: str) -> None:
    """Replace ONE named gate's CI step with its extractable arm commands.

    Taken as an argument rather than resolved here, because a caller that has
    already mutated the runner must expand the step for the gate it mutated.
    Resolving twice picked a DIFFERENT gate the second time and the probe then
    refused for that gate instead, which reads as a pass and discriminates
    nothing. Measured while building `ci-floor.work-inside-an-if`.
    """
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
    _leave_one_visible_arm_command(box, gate, narrowed)
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


def _work_inside_an_if_in_a_gate_arm(box: Sandbox) -> None:
    """Wrap an arm's unextractable command in an `if`, and expand its step.

    The measured bypass, in full. `unseen_commands` splits statements on
    `[\\n;{}()]|&&|\\|\\||\\|`, so `if node --test ...; then true; fi` is ONE
    statement whose head is `if`, and `if` was a head the scan treated as not
    being the work. Dropping the head dropped the command with it. MEASURED in
    the S03 review's seventh pass: with the `bench` arm's `node --test` line
    wrapped that way and the `bin/ocelli.sh gate bench` step replaced by its two
    `python3` commands, `unseen_commands` reported `bench: None` and the check
    exited 0, which is five node suites out of CI. That is byte for byte the
    outcome the sixth pass measured and turned into a rule, reached through the
    statement scanner instead of through the extractor.

    Both halves are needed. The wrap alone changes no verdict, because the gate
    is still named in `ci.yml`, and expanding the step alone is what the sixth
    pass's probe already covers.
    """
    import ci_floor_check
    gate = _a_gate_with_an_unextractable_arm_command(box)
    _leave_one_visible_arm_command(box, gate)
    runner = box.read("bin/ocelli.sh")
    region = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    arm = re.search(rf"^[ \t]*{re.escape(gate)}\).*?;;", region, re.M | re.S)
    if arm is None:
        raise AssertionError(
            f"the `{gate}` arm is not readable as a case label through to its "
            f"`;;`, so this probe cannot wrap its work without rewriting the "
            f"arm and would be building a different state.")
    head = ci_floor_check.unseen_commands(runner)[gate][0].split(" ", 1)[0]
    at = re.search(rf"(?<![\w./-]){re.escape(head)}[ \t]", arm.group(0))
    if at is None:
        raise AssertionError(
            f"the `{gate}` arm's unextractable command does not begin with a "
            f"word this probe can find in the arm text, so the wrap would land "
            f"somewhere other than in front of the command.")
    text = arm.group(0)
    box.substitute(
        "bin/ocelli.sh", text,
        f"{text[:at.start()]}if {text[at.start():-2]}; then true; fi ;;")
    # THIS gate's step, not whichever gate a second resolution would pick. The
    # wrap has just made this arm's unextractable command invisible to a broken
    # `unseen_commands`, so re-resolving would find a different gate and the
    # probe would refuse for that one, which reads as a pass and discriminates
    # nothing. Measured while building this probe.
    _expand_the_step_for(box, gate)


def _a_single_line_arm_ci_runs(box: Sandbox) -> tuple[str, str, str, str,
                                                      list[str]]:
    """A one-line gate arm whose every extractable command `ci.yml` runs.

    Returns the whole line, its indent, the gate name, the arm body up to the
    `;;` and the arm's extractable commands. Chosen for the property rather
    than named, exactly as `_nested_case_in_a_gate_arm` chooses one: WITHOUT
    the refusal under probe the check has to exit 0, or the probe would be
    watching an unrelated failure and would read as a pass.
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
        return (line, match.group(1), match.group(2),
                match.group(3).rstrip(), commands)
    raise AssertionError(
        "this repository has no one-line gate arm whose extractable commands "
        "all appear in ci.yml, so the eighth pass's arm-parser probes have "
        "nothing to extend and would each refuse for a reason that is not the "
        "one they are about.")


def _nested_case_after_a_bang(box: Sandbox) -> None:
    """A nested `case` introduced by `!`, which is not a word character.

    The seventh pass added the keyword alternative
    `\\b(?:then|do|else|elif|!)[ \\t]` to `NESTED_CASE`, and `\\b` needs a WORD
    character immediately before the token it precedes. `!` is not one, so
    `&& ! case ...` matched nothing while `ARM` went on truncating the arm at
    the inner `;;`. MEASURED in the S03 review's eighth pass on a synthetic
    `prose` arm reading `python3 scripts/prose_check.py && ! case "$OSTYPE" in
    *) : ;; esac && python3 scripts/prose_check.py --extra`: `arms['prose']`
    held one command, `unseen['prose']` was `None`, no refusal fired and the
    real trailing command was dropped at exit 0. That is the same fail-open
    the seventh pass closed for `then` and `do`, one alternation branch along.

    The command appended after the inner case is a real one from the same arm
    with an argument added, so what the truncation drops is work CI does not
    run.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}! case \"$OSTYPE\" in *) : ;; esac &&\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _arm_terminator_inside_a_quote(box: Sandbox) -> None:
    """A `;;` inside a QUOTED STRING, which is not the end of an arm.

    The eighth pass closed the comment route by stripping comments before the
    arm was delimited, and a string cannot be stripped. MEASURED in the S03
    review's ninth pass on the one-line `fmt` arm rewritten as `fmt) cargo fmt
    --all --check && echo "a ;; b" && python3 scripts/prose_check.py --extra
    ;;`, which `sh -n` accepts: `scripts/ci_floor_check.py` exited 0 with
    `arms['fmt']` holding one command, `unseen['fmt']` `None`, no refusal and
    the real trailing command dropped. The arm's end is found by a scan that
    steps over quoted spans now, and the statement split and the comment strip
    use that same scan, because three regexes over shell that must agree about
    quoting are three chances to disagree.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}echo \"a ;; b\" &&\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _a_quoted_string_in_an_arm(box: Sandbox) -> None:
    """The same quoted string with nothing dropped, which must be accepted.

    The direction the quote scanner could get wrong, and the first attempt at
    this fix DID get it wrong. `STATEMENT_BREAK` split inside the string as
    well, so `echo "a ;; b"` became `echo "a` and ` b"` and the check refused,
    naming `b"` as a command CI does not run. A quoted string in a gate arm is
    ordinary shell and weakens nothing, so the split respects quotes too.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    box.substitute("bin/ocelli.sh", line,
                   f"{indent}{gate}){tail} && echo \"a ;; b\" ;;")


def _nested_case_after_while(box: Sandbox) -> None:
    """A nested `case` introduced by `while`, which the alternation did not hold.

    `NESTED_CASE` listed `then|do|else|elif` plus `!` while `SHELL_INTRODUCERS`
    twenty lines below held ten names including `while`, `until` and `if`. The
    same file knew `while` introduces a command in one function and not in the
    other, and the smaller list was the fail-open. MEASURED in the S03 review's
    ninth pass on the one-line `fmt` arm, with `sh -n` accepting the file:
    `arms['fmt']` held one command, `unseen['fmt']` was `None`, no refusal
    fired and the real trailing command was dropped at exit 0. The alternation
    is DERIVED from `SHELL_INTRODUCERS` now, so the two cannot drift again.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}while case \"$OSTYPE\" in *) false ;; esac; do : ; done &&\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _nested_case_after_if(box: Sandbox) -> None:
    """A nested `case` introduced by `if`, the other keyword the lists differed on.

    The seventh pass's probe used `if true; then case ...`, so the inner case
    followed `then` and `then` was in the alternation. `if case ... esac; then`
    puts it after `if`, which was not, and MEASURED in the S03 review's ninth
    pass on the one-line `fmt` arm it left the check at exit 0 with one
    extracted command, `unseen` `None` and the real trailing command dropped,
    with `sh -n` accepting the file. The message is worded apart from
    `_nested_case_after_while`'s deliberately: two refusals in one file whose
    words normalise alike are ONE site to the census, and these two probes
    would then read as covering one refusal between them.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}if case \"$OSTYPE\" in *) true ;; esac; then : ; fi &&\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _nested_case_in_a_backtick(box: Sandbox) -> None:
    """A nested `case` inside a BACKTICK command substitution.

    The fourth fully fail-open terminator shape, and the one the ninth pass's
    three fixes all missed. `NESTED_CASE` matched `case` after `^`, after one
    of `[\\n;{}()&|]` or after a `SHELL_INTRODUCERS` word, and a backtick is
    none of the three, while `_quote_spans` knew `"` and `'` and not `` ` ``.
    `$(case ...)` was caught only because `(` happens to sit in that character
    class. MEASURED in the S03 review's tenth pass on the one-line `fmt` arm
    rewritten as `fmt) cargo fmt --all --check && test -n `case x in *) echo y
    ;; esac` && python3 scripts/prose_check.py --extra ;;`, which `bash -n`
    accepts: `scripts/ci_floor_check.py` exited 0, `arm_bodies['fmt']` came out
    as `cargo fmt --all --check && test -n `case x in *) echo y`, `unseen`
    was `None` and bash really runs the dropped command.

    **It is worse than the comment shape the eighth pass closed**, and that is
    why it needed both halves of the fix rather than one. The residue the
    truncation leaves is `test -n `case x in *) echo y`, whose heads are `test`
    and `echo`, and both are in `SHELL_NOISE`, so the unseen-command rule that
    catches the other shapes fires on nothing at all here.

    The command appended after the substitution is a real one from the same
    arm with an argument added, so what the truncation drops is work CI does
    not run.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}test -n `case x in *) echo y ;; esac` &&\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _a_backtick_in_an_arm(box: Sandbox) -> None:
    """A backtick command substitution with nothing dropped, which must pass.

    The direction the backtick fix could get wrong. A command substitution in a
    gate arm is ordinary shell and weakens nothing, so treating `` ` `` as a
    span must not turn the construct itself into a refusal, and the obvious
    over-tight repair, refusing any backtick in an arm, would pass the probe
    above and refuse a legitimate runner. The substitution's head is `printf`,
    which is in `SHELL_NOISE`, so the statement scan has nothing to demand of
    CI and the only thing under test is the delimiter.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    box.substitute("bin/ocelli.sh", line,
                   f"{indent}{gate}){tail} && test -n `printf x` ;;")


def _comment_after_a_substitution_in_an_arm(box: Sandbox) -> None:
    """A `#` straight after the `)` that closes a `$( ... )`, which is text.

    **A REGRESSION the S03 review's tenth pass introduced while closing a
    fail-open, and the eleventh pass measured it.** That pass made `)` a word
    start for the comment strip, which is right, POSIX and bash begin a
    comment at a `#` that begins a word and a word begins after an unquoted
    operator. The scanner could not tell an operator `)` from the `)` closing a
    command substitution, because it knew backticks and did not know `$(`.

    MEASURED. `bash -c 'echo A$(printf x)#no && echo RAN_SECOND'` prints both
    lines, so bash reads `#no` as part of the word rather than as a comment.
    With this shape planted in the one-line arm below, `bash -n` green,
    `scripts/ci_floor_check.py` exited 0 with the `--probe-extra` command
    dropped, and the SAME input at `e2b11d8`, the commit before `)` joined
    `WORD_BREAK`, exited 1 naming that command. So this route was not a
    survival, it was opened by the previous repair.

    The dropped command is on the FIRST line and the arm's `;;` on the second,
    deliberately: a comment that swallows the `;;` as well leaves the arm with
    no terminator, which the parser refuses for a different reason and which
    would read as a pass while discriminating nothing.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}) echo $(printf x)#no && {commands[0]} --probe-extra "
        f"&&\n{pad}{tail.strip()} ;;")


def _a_substitution_in_an_arm(box: Sandbox) -> None:
    """A `$( ... )` with nothing dropped, which must be accepted.

    The direction the fix could get wrong, and it is the same direction the
    backtick fix could get wrong one delimiter along. A command substitution in
    a gate arm is ordinary shell and weakens nothing, and the over-tight
    repair, taking `)` back out of `WORD_BREAK` or refusing any `$(`,
    would pass the probe above and either refuse a legitimate runner or reopen
    the tenth pass's `;#` route, which a `case` pattern's `)` needs closed. The
    substitution's head is `printf`, which is in `SHELL_NOISE`, so the
    statement scan has nothing to demand of CI and the only thing under test is
    the delimiter.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    box.substitute("bin/ocelli.sh", line,
                   f"{indent}{gate}) test -n $(printf x) &&{tail} ;;")


def _an_arm_with_a_heredoc(box: Sandbox, redirection: str,
                           body: str = "true ;;") -> None:
    """A here-document in the arm, and a real command CI does not run.

    One builder for the delimiter spellings, because they differ in the
    redirection and in nothing else, and a copy per spelling is how the arm
    parser acquired four almost identical probes in earlier passes.

    The body and its terminator sit at column 0. `<<-` is not used, so a
    leading tab would be part of the line and the terminator would not match,
    which is a property of the runner's own indentation rather than of this
    probe.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}: {redirection}\n"
        f"{body}\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _a_backslash_quoted_heredoc_delimiter(box: Sandbox) -> None:
    """`<<\\EOF`, the third quoting mechanism beside `'` and `"`.

    `HEREDOC` modelled bash's delimiter as
    `<<(-?)(?!<)[ \\t]*(['\\"]?)([A-Za-z_]\\w*)\\2`, which knows two of the
    three quoting mechanisms. When the redirection is not recognised the body
    is scanned as CODE, so a `;;` in it ends the arm and everything after it
    leaves in silence.

    MEASURED in the S03 review's twelfth pass, planted in the one-line arm
    below with `bash -n` accepting the runner:
    `scripts/ci_floor_check.py` exited 0 with the trailing command dropped, and
    the same shape run under bash prints both markers, so the dropped command
    really runs. The delimiter is read as a shell WORD by the scanner itself
    now, which is the one word production this file has.
    """
    _an_arm_with_a_heredoc(box, "<<\\EOF", "true ;;\nEOF")


def _a_quoted_heredoc_delimiter_with_a_hyphen(box: Sandbox) -> None:
    """`<<'EOF-1'`, where the character class stops before the closing quote.

    The second of the twelfth pass's two measured fail-opens, and it is the one
    that shows the class was the wrong SHAPE rather than the wrong class:
    `\\w*` matched `EOF`, the back-reference to the opening `'` then failed to
    match `-`, and the whole redirection went unrecognised. Same measurement,
    exit 0 with the command dropped and bash really running it.

    Its near miss, the unquoted `<<EOF-1`, is not a second probe here: it is a
    legitimate here-document that terminates, and
    `scripts/tests/test_guard_readers.py` asserts it reads correctly rather
    than refusing for a reason that names the wrong line.
    """
    _an_arm_with_a_heredoc(box, "<<'EOF-1'", "true ;;\nEOF-1")


def _a_heredoc_body_that_never_closes(box: Sandbox) -> None:
    """A here-document whose body never meets its delimiter.

    Fail-closed before and after, and the REFUSAL is what changed.
    `_heredoc_end`'s docstring said this state "reports the whole remainder so
    the caller's unclosed-span refusal is what fires", and `shell_pieces`
    cleared `pending` before its own `if pending:` test, so `unclosed` came
    back empty and the arm was refused for reaching the end of the region with
    no `;;` terminator, about an arm whose `;;` is right there. bash refuses
    this runner too, warning that the here-document is delimited by end of
    file, so the direction was never in doubt. What the guard SAID was.
    """
    _an_arm_with_a_heredoc(box, "<<EOF", "true ;;")


def _a_heredoc_in_an_arm(box: Sandbox) -> None:
    """A here-document with nothing dropped, which must be accepted.

    The direction the delimiter fix could get wrong, and the previous scanner
    DID get it wrong. MEASURED before the fix with exactly this shape: the
    check exited 1 reporting that the arm "runs 'a note', 'EOF-1'", so two
    lines of English were demanded of CI as commands. A here-document body is
    DATA, it is its own piece kind in the scanner now, and `_split_statements`
    drops it. The redirection's head is `:`, which is in `SHELL_NOISE`, so the
    statement scan has nothing to demand of CI and the only thing under test is
    the body.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}: <<'EOF-1'\n"
        f"a note the arm carries about why it runs\n"
        f"EOF-1\n"
        f"{pad};;")


def _a_continuation_at_the_end_of_an_arm_comment(box: Sandbox) -> None:
    """A backslash ending a COMMENT line, which continues nothing in bash.

    `arm_bodies` ran `CONTINUATION.sub(" ", region)` BEFORE the tokenizer,
    which is the one thing the tokenizer's own header says no pass may do: a
    regex pre-pass over shell cannot tell a comment from anything else, and a
    comment ends at its newline whatever precedes that newline.

    MEASURED. `echo A # c \\` then `echo B` prints both lines under bash, so
    the comment continues nothing. With this shape planted in the one-line arm
    below, the pre-pass joined the next line INTO the comment, the planted
    command vanished entirely, `arms` came back holding the NEXT gate's
    command, and the check refused while naming two gates neither of which was
    the one edited. Fail-closed by luck and pointing at the wrong file.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}# the reason the flag below is here \\\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _a_continuation_inside_an_arm_command(box: Sandbox) -> None:
    """One arm command split across a continuation, which must be accepted.

    The direction the fix could get wrong, and it is why the continuation is a
    PIECE the scanner emits rather than a deletion. `gate_commands` extracts
    with a class that stops at a backslash, so a command left unjoined loses
    everything after it and the arm command CI runs verbatim then reads as
    absent. Four arms in the runner are already written this way.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    head, _, rest = commands[0].partition(" ")
    if not rest or commands[0] not in line:
        raise AssertionError(
            f"the `{gate}` arm's command `{commands[0]}` is one word, or it "
            f"does not appear verbatim in its own line, so there is nowhere "
            f"in it to put a continuation and this probe would mutate "
            f"nothing while reading as a pass.")
    box.substitute("bin/ocelli.sh", line,
                   line.replace(commands[0],
                                f"{head} \\\n{indent}  {rest}", 1))


def _a_gate_named_outside_the_name_class(box: Sandbox) -> None:
    """A gate name carrying a DOT, which is outside `GATE_NAME`.

    **The refusal this plants was watched by nothing until the S03 review's
    twelfth pass.** `ci-floor.gate-name-with-a-digit` plants `prose2`, which
    is INSIDE `[A-Za-z0-9_-]+`, and expects the floor-coverage refusal, and the
    unit test carrying the refusal's name asserted `gate_row_problems` was
    EMPTY. MEASURED with the `if not GATE_NAME.match(name)` branch disabled:
    the census, all 54 `ci-floor` probes and both unit suites stayed at their
    unmutated status.

    A dot rather than any other character, because the refusal's own sentence
    is that a gate name reaches `re.escape`-free patterns in
    `scripts/ci_floor_check.py`, in `scripts/guards/census.py` and in this
    file's probe builders, and a dot in a name is a regex wildcard in every one
    of them.
    """
    _a_gates_entry_the_reader_cannot_use(
        box, '"pro.se|no|a dotted name nothing declares"')


def _a_gate_name_with_a_digit_is_permitted(box: Sandbox) -> None:
    """A digit in a gate name, with a real arm CI runs, which must be accepted.

    The direction the widened class could get wrong, and it is the sentence
    `_a_gate_named_outside_the_python_class` wrote down and did not probe:
    "an arm identical to the gate's left `scripts/ci_floor_check.py`
    legitimately at exit 0, because every command in the arm really was run by
    CI." The over-tight repair, narrowing `GATE_NAME` back towards `[a-z-]+`,
    passes every refusal probe beside this one and refuses a runner bash reads
    without complaint.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    _a_gates_entry_the_reader_cannot_use(
        box, f'"{gate}2|no|a second {gate} pass CI already runs"')
    box.substitute("bin/ocelli.sh", line,
                   f"{line}\n{indent}{gate}2){tail} ;;")


def _a_gates_entry_the_reader_cannot_use(box: Sandbox, entry: str) -> None:
    """Put `entry` into `bin/ocelli.sh`'s GATES array, above the first row.

    The array is found by its own opening line rather than by naming a gate, so
    these probes do not go stale when the first gate changes.
    """
    runner = box.read("bin/ocelli.sh")
    match = re.search(r"^GATES=\(\n([ \t]*)", runner, re.M)
    if match is None:
        raise AssertionError(
            "bin/ocelli.sh carries no `GATES=(` array opening a line, so "
            "there is no array for this probe to add an entry to.")
    box.substitute("bin/ocelli.sh", match.group(0),
                   f"{match.group(0)}{entry}\n{match.group(1)}")


def _a_gate_named_outside_the_python_class(box: Sandbox) -> None:
    """A gate whose name carries a DIGIT, which bash reads and Python did not.

    `bin/ocelli.sh` reads an entry with `IFS='|' read -r name gpu desc`, which
    imposes no character class on the name. `scripts/ci_floor_check.py` and
    `scripts/guards/census.py` both matched `[a-z-]+`, in two copies of one
    regex.

    MEASURED at HEAD with a `prose2` gate carrying a real arm: bash reported 29
    gates and Python 28, `scripts/ci_floor_check.py` exited 0, the census
    exited 0, `gates_declared` did not move because the Python side never
    counted the entry, and `gate --floor` selected the gate while no CI step
    ran it and no catalogue entry claimed it. An entry the reader cannot parse
    became an OMISSION rather than a refusal, which is the one outcome a guard
    may not have.

    The arm is a real command CI does not run, so the gate is genuinely
    unrun rather than accidentally covered by another gate's step. Measured
    while building this probe: an arm identical to the `prose` gate's left
    `scripts/ci_floor_check.py` legitimately at exit 0, because every command
    in the arm really was run by CI.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    _a_gates_entry_the_reader_cannot_use(
        box, f'"{gate}2|no|a second {gate} pass nothing declares"')
    box.substitute("bin/ocelli.sh", line,
                   f"{line}\n{indent}{gate}2) {commands[0]} --probe-extra ;;")


def _a_comment_inside_the_gates_array(box: Sandbox) -> None:
    """A comment and a blank line inside the GATES array, which change nothing.

    The direction the entry reader could get wrong. bash ignores both, so a
    reader that demanded one quoted entry per line would refuse a legitimate
    runner, which is the runbook's own sentence about a guard that fails on
    everything. The scanner drops a comment and treats a blank line as
    whitespace between words, by the same rule it uses inside a gate arm.
    """
    _a_gates_entry_the_reader_cannot_use(
        box, "# The gates, grouped. This comment is not an entry.\n")


def _an_unbalanced_quote_in_an_arm(box: Sandbox) -> None:
    """A quote that is opened in the LAST arm and never closed.

    The fail-closed half of the quote scan. `_arm_end` refuses rather than
    guessing which `;;` was meant, and the position matters: a quote opened in
    an EARLIER arm is closed by the next quote anywhere in the region and read
    as one very long arm, which the per-command rule then refuses for a
    different reason. MEASURED both ways in the S03 review's ninth pass, and
    the guard's declared limit records it. The last arm is chosen by property
    rather than named, so the probe does not go stale when an arm is added.
    """
    runner = box.read("bin/ocelli.sh")
    region = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    arms = re.findall(r"^[ \t]*([a-z-]+)\)(?:.*?);;", region, re.M | re.S)
    if not arms:
        raise AssertionError(
            "bin/ocelli.sh's `run_gate` has no case arm this probe can find, "
            "so there is no last arm to leave a quote open in and the "
            "unbalanced-quote branch cannot be reached.")
    last = re.search(rf"^[ \t]*{re.escape(arms[-1])}\).*?;;", region,
                     re.M | re.S)
    box.substitute("bin/ocelli.sh", last.group(0),
                   last.group(0)[:-2] + '&& echo "unclosed ;;')


def _arm_comment_holding_a_terminator(box: Sandbox) -> None:
    """A `;;` inside a shell comment, which is not the end of an arm.

    `SHELL_COMMENT` was applied to `match.group(2)` AFTER `ARM` had matched,
    and `ARM` stops at the first `;;`, so a comment carrying one truncated the
    arm before the comment was stripped. MEASURED in the S03 review's eighth
    pass on a synthetic `prose` arm whose comment line read `# the voice
    rules, then the second pass ;; see the LLD` with a real second `python3
    scripts/prose_check.py --extra` after it: `arms['prose']` held one
    command, `unseen['prose']` was `None`, no refusal fired and the trailing
    command was dropped at exit 0. Comments are stripped from the whole
    `run_gate` region before `ARM.finditer` now, which is the only order in
    which a comment cannot terminate an arm.
    """
    line, indent, gate, tail, commands = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate}){tail} &&\n"
        f"{pad}# the gate's second half ;; and the reason it is second\n"
        f"{pad}{commands[0]} --probe-extra ;;")


def _work_behind_the_command_builtin(box: Sandbox) -> None:
    """Hide an arm's unextractable command behind `command`, and expand its step.

    `command foo args` RUNS foo, which is the exact property that moved `eval`
    into `SHELL_INTRODUCERS`, and `command` sat in `SHELL_NOISE` beside it.
    This file's own declared limit named the pair as the residue and nothing
    read that sentence. MEASURED in the S03 review's eighth pass: rewriting the
    `bench` arm's `node --test` line as `command node --test ...` and
    replacing the `bin/ocelli.sh gate bench` step with its two `python3`
    commands gave `unseen['bench'] == None` and exit 0, five node suites out of
    CI. That is byte for byte the outcome the sixth and seventh passes each
    measured and turned into a rule, reached through a third head.

    Both halves are needed, for the reason `_work_inside_an_if_in_a_gate_arm`
    gives: the rewrite alone changes no verdict while the gate is still named
    in `ci.yml`, and expanding the step alone is the sixth pass's probe.
    """
    import ci_floor_check
    gate = _a_gate_with_an_unextractable_arm_command(box)
    _leave_one_visible_arm_command(box, gate)
    runner = box.read("bin/ocelli.sh")
    region = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    arm = re.search(rf"^[ \t]*{re.escape(gate)}\).*?;;", region, re.M | re.S)
    if arm is None:
        raise AssertionError(
            f"the `{gate}` arm cannot be read from its case label through to "
            f"its `;;`, so this probe cannot put `command` in front of the "
            f"work without rewriting the arm, which would build a state other "
            f"than the one measured.")
    head = ci_floor_check.unseen_commands(runner)[gate][0].split(" ", 1)[0]
    at = re.search(rf"(?<![\w./-]){re.escape(head)}[ \t]", arm.group(0))
    if at is None:
        raise AssertionError(
            f"the `{gate}` arm's unextractable command does not start with a "
            f"word this probe can locate in the arm text, so `command` would "
            f"be inserted somewhere other than in front of that command.")
    text = arm.group(0)
    box.substitute("bin/ocelli.sh", text,
                   f"{text[:at.start()]}command {text[at.start():]}")
    # This gate's step, for the reason `_expand_the_step_for` records: the
    # rewrite has just made this arm's unextractable command invisible to a
    # broken `unseen_commands`, so re-resolving would pick a different gate.
    _expand_the_step_for(box, gate)


def _a_lookup_with_the_command_builtin(box: Sandbox) -> None:
    """`command -v x`, which looks a command up and runs nothing.

    The direction that says which fix was made. `bin/ocelli.sh`'s `panic` arm
    already writes `command -v wasm-pack >/dev/null || { ... }` to decide
    whether the real work can run, so a repair that treated every `command` as
    work would report an argument list as an unseen command and demand a
    gate-name step for an arm that runs its own commands as CI steps today.
    `-v` and `-V` are the lookup options and `-p` only changes the PATH they
    search, so only the first two make it a lookup.

    `git` rather than `python3` deliberately: the extraction class is anchored
    on `python3 `, `npm run `, `cargo ` and `ci/`, and naming one of those
    inside the guard line would add an arm command CI does not run and refuse
    for a reason that is not this one.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    pad = indent + " " * (len(gate) + 1)
    box.substitute(
        "bin/ocelli.sh", line,
        f"{indent}{gate})command -v git >/dev/null || "
        f"{{ skip \"git is absent\"; return 3; }}\n"
        f"{pad}{tail.lstrip()} ;;")


def _an_impossible_wall_clock_pair(box: Sandbox) -> None:
    """Record the deep profile as faster than the floor, which cannot happen.

    `scripts/guard_probe.py`'s `selected()` returns EVERY probe for the deep
    profile and only the `profile == "floor"` ones for the floor, so deep runs
    a strict superset of the floor's work. FOUND in the S03 review's eighth
    pass as a recorded pair, `deep: 17.2` beside `floor: 18.1`: the seventh
    pass re-recorded floor while moving seventeen probes into deep and left
    deep at its pre-move value, so the `5x + 30` ceiling for the profile that
    now carries every `lint-policy` probe was set from a run that did not
    contain them. Nothing compared the two, because a ceiling that is too
    generous never turns a gate red. Measured over four runs on this machine,
    with the eighth pass's fourteen new probes in, at floor 19.6s to 20.6s and
    deep 27.4s to 28.9s.

    The value written is derived from the recorded floor rather than fixed, so
    the probe keeps meaning the same thing after a re-baseline.
    """
    budget = json.loads(box.read("ci/guard-probe-budget.json"))
    timings = budget.get("wall_clock_seconds", {})
    if "floor" not in timings or "deep" not in timings:
        raise AssertionError(
            "ci/guard-probe-budget.json records no floor and deep wall clock "
            "pair, so there is no pair for this probe to make impossible and "
            "the census would be answering a question about nothing.")
    timings["deep"] = round(timings["floor"] / 2, 1)
    box.write("ci/guard-probe-budget.json",
              json.dumps(budget, indent=2, sort_keys=True) + "\n")


def _nested_case_after_a_keyword(box: Sandbox) -> None:
    """A nested `case` introduced by `then`, which the refusal did not see.

    `NESTED_CASE` required `case` to follow the start of the arm body or one of
    `[\\n;{}()&|]`, so a keyword and a space defeated it while `ARM` went on
    truncating the body at the inner `;;`. MEASURED on a synthetic arm: the body
    was kept only as far as the inner case, `arms` held one command, `unseen`
    was `None`, no refusal fired, and a real second command after the inner
    case was dropped at exit 0.

    The arm is chosen for the property rather than named, exactly as
    `_nested_case_in_a_gate_arm` chooses one: a single-line arm whose
    extractable command `ci.yml` runs, so that WITHOUT the refusal this exits 0
    rather than refusing for an unrelated reason. The command appended after
    the inner case is a real one from the same arm, so what the truncation
    drops is work CI is supposed to run.
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
        pad = match.group(1) + " " * (len(match.group(2)) + 1)
        box.substitute(
            "bin/ocelli.sh", line,
            f"{match.group(1)}{match.group(2)}){match.group(3).rstrip()} &&\n"
            f"{pad}if true; then case \"$OSTYPE\" in *) : ;; esac; fi &&\n"
            f"{pad}{commands[0]} --probe-extra ;;")
        return
    raise AssertionError(
        "no gate has a single-line arm this probe can extend with a `then "
        "case`, because none has an extractable command that ci.yml runs, so "
        "the keyword-position shape cannot be built and the run would refuse "
        "for an unrelated reason. The message is worded apart from "
        "`_nested_case_in_a_gate_arm`'s deliberately: two refusals in one file "
        "whose words normalise alike are ONE site to the census, so a probe on "
        "either would read as covering both.")


def _widen_a_ci_step_past_its_arm_command(box: Sandbox) -> None:
    """Add an argument to a CI step that the gate's arm command does not carry.

    `runs_command` tested for a SUBSTRING, so a step running the arm's command
    plus arguments satisfied it. MEASURED in the seventh pass: changing a CI
    step to `python3 scripts/prose_check.py --only-this-one-file README.md`
    left the check at exit 0 with the gate reported as invoked, while CI
    checked one file. `ci-floor.narrowed-arm-command` probes the opposite
    direction, an argument the arm already carried being narrowed, and cannot
    see an argument being added.

    The step is chosen for the property rather than named: a floor gate whose
    arm is ONE extractable command that `ci.yml` runs verbatim, so the widened
    step is the only route to the gate and nothing else covers it.
    """
    excluded = _not_in_floor(box)
    workflow = box.read(".github/workflows/ci.yml")
    for gate in re.findall(r'^\s*"([a-z-]+)\|no\|', box.read("bin/ocelli.sh"),
                           re.M):
        if gate in excluded or f"bin/ocelli.sh gate {gate}" in workflow:
            continue
        commands = _ci_arm_commands(box, gate)
        if len(commands) != 1:
            continue
        line = next((l for l in workflow.splitlines()
                     if l.strip() == f"- run: {commands[0]}"), None)
        if line is None:
            continue
        box.substitute(".github/workflows/ci.yml", line,
                       f"{line} --probe-only-this-one-file README.md")
        return
    raise AssertionError(
        "no floor gate is run by a CI step whose command is exactly its one "
        "arm command, so there is no step this probe can widen without "
        "changing what else covers the gate.")


# ---------------------------------------------------------------------------
# The workflow is YAML, and it was read line by line until the twelfth pass
# ---------------------------------------------------------------------------
#
# Nine probes for the routes the S03 review's twelfth pass measured and five
# for the legitimate workflows the reader refused. Each builder writes a file
# GitHub Actions reads exactly as it reads the original, or, for the two
# refusals the parse introduces, one it cannot read at all.
#
# The mutations are all in `.github/workflows/ci.yml` and none of them touches
# `scripts/ci_floor_check.py`, which is what makes them assertions about the
# workflow's grammar rather than about the guard's source.

WORKFLOW_PATH = ".github/workflows/ci.yml"

# A condition that is FALSE on both automatic events. The bypasses use it
# because a step behind it does not run on a pull request, which is the same
# outcome as deleting the step, and the check has to say so.
MANUAL_ONLY = "github.event_name == 'workflow_dispatch'"


def _a_floor_gate_whose_step_is_one_line(box: Sandbox) -> tuple[str, str]:
    """A floor gate whose whole CI step is one `- run: bin/ocelli.sh gate X`.

    ONE LINE, and that is the property rather than a preference.
    `_floor_gate_with_own_step` returns the first floor gate `ci.yml` names,
    which today is `native`, whose step carries a `name:` key as well. Every
    builder below rewrites the step as a whole, so a step whose first key is
    somewhere else on the page cannot be rewritten by replacing one line
    without producing YAML that neither GitHub nor this check can read. The
    gate is still chosen by property and read from the repository.
    """
    excluded = _not_in_floor(box)
    for line in box.read(WORKFLOW_PATH).splitlines():
        match = re.fullmatch(r"\s*- run: bin/ocelli\.sh gate ([a-z-]+)", line)
        if match and match.group(1) not in excluded:
            return match.group(1), line
    raise AssertionError(
        "no floor gate's CI step is a single `- run: bin/ocelli.sh gate <name>` "
        "line, so there is no step these builders can rewrite whole and they "
        "would each plant a workflow that is invalid for a second reason.")


def _gate_step_pieces(box: Sandbox) -> tuple[str, str, str, str]:
    """That gate, its step line, the line's indentation and its body."""
    gate, line = _a_floor_gate_whose_step_is_one_line(box)
    return gate, line, " " * (len(line) - len(line.lstrip())), \
        line.lstrip().removeprefix("- ")


def _a_floor_gate_run_as_its_one_arm_command(box: Sandbox) -> tuple[str, str]:
    """A floor gate whose only CI step is its one arm command, and that line.

    The same selection `_widen_a_ci_step_past_its_arm_command` makes and for
    the same reason: the step is the only route to the gate, so a probe that
    changes how the step is SPELLED changes nothing else about the gate's
    coverage.
    """
    excluded = _not_in_floor(box)
    workflow = box.read(WORKFLOW_PATH)
    for gate in re.findall(r'^\s*"([a-z-]+)\|no\|', box.read("bin/ocelli.sh"),
                           re.M):
        if gate in excluded or f"bin/ocelli.sh gate {gate}" in workflow:
            continue
        commands = _ci_arm_commands(box, gate)
        if len(commands) != 1:
            continue
        line = next((l for l in workflow.splitlines()
                     if l.strip() == f"- run: {commands[0]}"), None)
        if line is not None:
            return commands[0], line
    raise AssertionError(
        "no floor gate is run by a CI step whose command is exactly its one "
        "arm command, so this probe has no step whose spelling it can change "
        "without changing what else covers the gate.")


def _step_keys_in_the_other_order(box: Sandbox) -> None:
    """A gate step with `run:` written ABOVE its `if:`, nothing else changed.

    The twelfth pass's first route and the one no spelling rule reaches.
    `run_commands` attached whatever `if:` it had seen so far, so this file
    exited 0 and the identical two keys in the other order exited 1. A YAML
    mapping has no key order, so those are one workflow and the check gave two
    answers, on `guards` in an edit that reads as tidying.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- {body}\n{indent}  if: {MANUAL_ONLY}")


def _a_quoted_if_key(box: Sandbox) -> None:
    """The same bypass spelled `"if":`, with the key first.

    The second spelling of route 1. The condition sits ABOVE the `run:` here,
    so the line-by-line reader would have attached it had it recognised the
    key, and it did not: its pattern was `^\\s*(?:-\\s+)?if:`. `"if"` and `if`
    are one key after a parse. The workflow's own `on:` head pattern already
    spelled `(?:on|"on"|'on'|true)`, so this file knew keys may be quoted in
    one function and not in the other, which is the drift shape
    `NESTED_CASE` and `SHELL_INTRODUCERS` were joined to remove.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f'{indent}- "if": {MANUAL_ONLY}\n{indent}  {body}')


def _a_flow_mapping_step_with_run_first(box: Sandbox) -> None:
    """Route 1 again, as a multi-line flow mapping with `run` before `if`.

    A third spelling of one mapping. The block form and the flow form are the
    same node to a parser and were two different answers to a line reader.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(
        WORKFLOW_PATH, line,
        f"{indent}- {{\n{indent}    {body},\n"
        f"{indent}    if: {MANUAL_ONLY}\n{indent}  }}")


def _a_folded_run_scalar_narrowing_a_command(box: Sandbox) -> None:
    """Narrow a gate's one CI command, spelled as a FOLDED block scalar.

    Route 2, and it is the seventh pass's hole reached by changing one
    character. `run: >` folds its body into one line, and the reader split a
    block scalar into one `Command` per line, so `python3
    scripts/prose_check.py` and its narrowing argument became two commands and
    the argv equality in `runs_command` matched the first. MEASURED at exit 0,
    where `ci-floor.widened-ci-step` writes the same narrowing on one plain
    line and is refused at exit 1.
    """
    command, line = _a_floor_gate_run_as_its_one_arm_command(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: >\n"
                   f"{indent}    {command}\n"
                   f"{indent}    --probe-only-this-one-file README.md")


def _a_quoted_event_key_in_on(box: Sandbox) -> None:
    """Quote one key in `on:` and gate a floor step to the other event.

    Route 3. Quoting a key removed the event from the set the floor is checked
    against, so the check compared the workflow with a smaller floor and
    printed the narrowed event list as its OK line. The gate then legitimately
    fails to run on the event nobody was asking about.

    Both halves are read from the repository: the event comes from the
    workflow's own `on:` block through the guard's own reader, and the gate
    from the first floor gate `ci.yml` names.
    """
    import ci_floor_check
    workflow = box.read(WORKFLOW_PATH)
    events = sorted(ci_floor_check.workflow_events(workflow)
                    - ci_floor_check.MANUAL_EVENTS)
    if len(events) < 2:
        raise AssertionError(
            "the workflow declares fewer than two automatic events, so "
            "hiding one leaves nothing for the surviving step to miss.")
    hidden, kept = events[0], events[1]
    box.substitute(WORKFLOW_PATH, f"  {hidden}:", f'  "{hidden}":')
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- if: github.event_name == '{kept}'\n"
                   f"{indent}  {body}")


def _a_gate_name_inside_a_run_string(box: Sandbox) -> None:
    """Replace a gate's step with an `echo` that MENTIONS the gate.

    Route 4a. The gate name was matched anywhere in a `run:` line and a string
    is anywhere, so `echo "if the size budget moves, run bin/ocelli.sh gate
    panic locally"` satisfied the gate at exit 0 with the real step gone. That
    is the comment-only lesson one language along: a comment runs nothing and
    neither does a sentence inside a string. `shell_pieces` already placed that
    string in a span and this file used it on `bin/ocelli.sh` and never on a
    `run:` body.
    """
    gate, line, indent, _ = _gate_step_pieces(box)
    box.substitute(
        WORKFLOW_PATH, line,
        f'{indent}- run: echo "if this fails, run bin/ocelli.sh gate {gate} '
        f'locally"')


def _a_run_key_under_with(box: Sandbox) -> None:
    """Replace a gate's step with an action whose INPUT is called `run`.

    Route 4b. A `run:` was a command at any depth, so an action input under
    `with:` was read as a step's shell. It is not one: GitHub runs the action,
    and `run` is a string it hands the action. The step is a real step and a
    reader of `ci.yml` still sees the gate named, which is what makes this the
    same shape as the comment-only probe.
    """
    gate, line, indent, _ = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- uses: actions/github-script@v7\n"
                   f"{indent}  with:\n"
                   f"{indent}    run: bin/ocelli.sh gate {gate}")


def _an_unparseable_workflow(box: Sandbox) -> None:
    """Open a flow sequence in a `run:` and never close it.

    The fail-closed half of parsing the workflow. GitHub Actions reads this
    file with a YAML parser, so a file no parser can read is a workflow that
    does not run at all, and reporting coverage over whatever a line scanner
    salvaged from it would be the widest possible false green.
    """
    gate, line, indent, _ = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: [bin/ocelli.sh gate {gate}")


def _a_run_that_is_not_a_scalar(box: Sandbox) -> None:
    """Write a step's `run:` as a sequence rather than as a command.

    The other half of the parse's own refusal. The tree is readable and the
    step's shape is not one GitHub accepts, so the reader refuses it by name
    rather than reading a smaller workflow. A shape skipped here is a step
    whose gate then reads as uninvoked, or worse as covered by whatever was
    read in its place.
    """
    gate, line, indent, _ = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run:\n"
                   f"{indent}    - bin/ocelli.sh gate {gate}")


def _a_quoted_run_scalar(box: Sandbox) -> None:
    """Quote a step's `run:` value. The same command, refused at exit 1.

    The first of the five legitimate workflows the line reader refused. Its
    pattern took everything after `run:` as the command, quotes included, so
    the argv comparison saw `"python3` and `scripts/prose_check.py"` and
    reported the gate uninvoked while CI ran it.
    """
    command, line = _a_floor_gate_run_as_its_one_arm_command(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(WORKFLOW_PATH, line, f'{indent}- run: "{command}"')


def _a_block_scalar_with_an_indentation_indicator(box: Sandbox) -> None:
    """`run: |2`, which names the body's indentation instead of inferring it.

    The second. The reader compared the text after `run:` against a set of six
    block headers and an explicit indentation indicator is in none of them, so
    the body was not read as a body and the command vanished. The indicator is
    ordinary YAML and the value it produces is identical.
    """
    command, line = _a_floor_gate_run_as_its_one_arm_command(box)
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |2\n{indent}    {command}")


def _a_background_operator_in_a_gate_arm(box: Sandbox) -> None:
    """Join two arm commands with `&` instead of `&&`, on one line.

    **The fourteenth route, and it is the sixth, seventh and eighth passes with
    a different operator.** bash's `&` terminates a list exactly as `;` does,
    and `STATEMENT_BREAK` carried `&&` and no `&`, so `A & B` was ONE statement
    whose head is `A`. `unseen_commands` drops a statement whose head is a
    `COMMAND_PREFIXES` prefix, so the command after the `&` left the arm.

    MEASURED in a real clone: the `&&` before `node --test` in the `bench` arm
    rewritten as `&` on one line, with `- run: bin/ocelli.sh gate bench`
    replaced by the arm's two extractable `python3` commands, gave `bash -n` 0
    and the check exit 0 printing "every command in each gate's arm", `bench`
    gone from the named-only list and six node suites out of CI. bash disagrees
    on the same body: `case g in g) echo M1 & echo M2 ;; esac` prints both.

    The step is expanded for the SAME gate the runner was mutated for, which is
    `_expand_the_step_for`'s own reason for taking the gate as an argument, and
    the line search is bounded to that gate's OWN arm for the same reason. It
    was not, in the first version of this builder, and the unbounded search
    found the `panic` arm's `node scripts/panic_probe.mjs` first while the step
    was expanded for `bench`. The probe then drove the guard red for a state
    that is not the one it is about, which reads as a pass and discriminates
    nothing: MEASURED green against the UNFIXED guard.
    """
    import ci_floor_check
    gate = _a_gate_with_an_unextractable_arm_command(box)
    _leave_one_visible_arm_command(box, gate)
    runner = box.read("bin/ocelli.sh")
    head = ci_floor_check.unseen_commands(runner)[gate][0].split(" ", 1)[0]
    label = re.search(rf"^[ \t]*{re.escape(gate)}\)", runner, re.M)
    following = re.search(r"^[ \t]*[A-Za-z0-9_-]+\)", runner[label.end():],
                          re.M)
    stop = label.end() + (following.start() if following else len(runner))
    lines = runner[label.start():stop].splitlines(keepends=True)
    for index, line in enumerate(lines):
        if index == 0 or not line.lstrip().startswith(f"{head} "):
            continue
        before = lines[index - 1]
        if not before.rstrip().endswith("&&"):
            continue
        box.substitute(
            "bin/ocelli.sh", before + line,
            f"{before.rstrip()[:-2].rstrip()} & {line.lstrip()}")
        _expand_the_step_for(box, gate)
        return
    raise AssertionError(
        f"the `{gate}` arm does not chain its unextractable command onto the "
        f"line above it with `&&`, so the operator this probe is about has "
        f"nowhere to go and the run would refuse for an unrelated reason.")


def _a_redirection_in_a_gate_arm(box: Sandbox) -> None:
    """`>&2` in a gate arm, which separates nothing and must be accepted.

    The direction the `&` fix could get wrong, and the naive spelling
    `r"[\\n;{}()]|&&|\\|\\||\\||&"` DID get it wrong. `&` is the second
    character of `>&` and `<&` and the first of `&>`, none of which is a
    control operator. MEASURED with that spelling over the real runner:
    `unseen['panic']` grew the entry `'2'`, a file descriptor reported as a
    command CI does not run, out of the `panic` arm's own `echo "wasm-pack is
    not installed. ..." >&2`. It cost no exit code there only because `panic`
    already holds unseen commands and CI names the gate.
    """
    line, indent, gate, tail, _ = _a_single_line_arm_ci_runs(box)
    box.substitute("bin/ocelli.sh", line,
                   f"{indent}{gate}){tail} && echo \"a note\" >&2 ;;")


def _a_gate_name_in_a_heredoc_body(box: Sandbox) -> None:
    """Replace a gate's step with a here-document whose BODY names the gate.

    Route 4a of the twelfth pass in the spelling that pass did not close.
    `run_commands` read the body one LINE at a time while the workflow itself
    was parsed, so the cross-line state that makes the body DATA was discarded
    between the line declaring the redirection and the line it governs.
    MEASURED in a real clone with the `bin/ocelli.sh gate panic` step rewritten
    this way: exit 0, and bash confirms the runner is never called, on HLD
    section 23's wasm panic-hook proof.
    """
    gate, line, indent, _ = _gate_step_pieces(box)
    box.substitute(
        WORKFLOW_PATH, line,
        f"{indent}- run: |\n"
        f"{indent}    cat <<'PROBE_EOF'\n"
        f"{indent}    bin/ocelli.sh gate {gate}\n"
        f"{indent}    PROBE_EOF")


def _a_continued_command_in_a_ci_step(box: Sandbox) -> None:
    """The same step's one command continued onto a second line.

    The FALSE REFUSAL half of the same line of code, and it is why this fix
    arrives with an accept probe. Read line by line, `python3
    scripts/staged_content_check.py \\` and `--tracked` are two commands and
    neither is the arm's, so the gate read as uninvoked. MEASURED at exit 1,
    while bash runs it as one command.
    """
    command, line = _a_floor_gate_run_as_its_one_arm_command(box)
    head, _, tail = command.partition(" ")
    indent = " " * (len(line) - len(line.lstrip()))
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    {head} \\\n"
                   f"{indent}      {tail}")


def _continue_on_error_on_a_gate_step(box: Sandbox) -> None:
    """`continue-on-error: true` on the step that runs a floor gate.

    `continue-on-error` was in the parsed tree and nothing read it. The string
    occurred nowhere in `scripts/`, in `docs/lld/guards.md` or in the runbook.
    `--floor` claims to be what CI runs, and a step whose failure cannot fail
    the run does not run the gate in the sense that claim means, in one line
    that reads as tolerating flakiness.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- continue-on-error: true\n"
                   f"{indent}  {body}")


def _continue_on_error_on_the_job(box: Sandbox) -> None:
    """The same key one level up, on the JOB holding that step.

    A job key rather than a step key, so a reader looking at the step sees
    nothing at all. It takes every step in the job with it.
    """
    _plant_under_the_job_key(box, "    continue-on-error: true\n")


# The job key above the gate step, and the block a probe plants under it. TWO
# builders need it, `continue-on-error` and `defaults.run.shell`, and its one
# refusal is written once here rather than once in each: a message duplicated
# per caller is a second refusal site watched by whatever watches the first,
# which is the shape the census counts and this catalogue argues against
# everywhere else.
def _plant_under_the_job_key(box: Sandbox, block: str) -> None:
    """Insert `block` directly under the job key holding the gate step."""
    gate, line, _, _ = _gate_step_pieces(box)
    lines = box.read(WORKFLOW_PATH).splitlines()
    index = lines.index(line)
    for position in range(index, -1, -1):
        if re.fullmatch(r"( {2})([A-Za-z0-9_-]+):", lines[position]):
            box.substitute(WORKFLOW_PATH, lines[position] + "\n",
                           f"{lines[position]}\n{block}")
            return
    raise AssertionError(
        f"the step running `{gate}` sits under no job key this probe can find, "
        f"so the job-level form of the key cannot be planted.")


def _a_gate_step_whose_failure_is_swallowed(box: Sandbox) -> None:
    """`|| true` appended to the run, which is the third route to the same end.

    The step is there, it names the gate, bash runs the gate, and the step's
    exit status is `true`'s. MEASURED under `bash -e`: `false || true` followed
    by another line exits 0. This one was arguably inside the declared limit,
    that the reader "splits on the boolean operators without evaluating them",
    and the other two were covered by nothing.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line, f"{indent}- {body} || true")


def _a_gate_after_a_swallowed_and_condition(box: Sandbox) -> None:
    """Put the gate on the conditional right side of a non-final AND-list.

    MEASURED under `bash -e`: `false && GATE` followed by a successful command
    exits 0 without running the gate. The final command is load-bearing. With
    the AND-list last, the failed left side makes the step fail and the gate
    need not run for the step to remain a valid CI control.
    """
    _, line, indent, body = _gate_step_pieces(box)
    command = body.removeprefix("run: ").strip()
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    false && {command}\n"
                   f'{indent}    echo "later success"')


# The `run:` body of the step `_gate_step_pieces` picks, without the YAML key.
# Five builders below rewrite that ONE command into a shape whose failure the
# shell discards or plainly does not, and each needs the command rather than
# the key.
#
# No refusal of its own, deliberately. `_a_floor_gate_whose_step_is_one_line`
# has already matched `- run: bin/ocelli.sh gate <name>` whole, so the command
# is non-empty by construction, and a check here would be a refusal site that
# no state can reach.
def _gate_step_command(box: Sandbox) -> tuple[str, str, str]:
    """The gate step's line, its indentation, and the command it runs."""
    _, line, indent, body = _gate_step_pieces(box)
    return line, indent, body.removeprefix("run: ").strip()


def _a_custom_shell_template_on_a_gate_step(box: Sandbox) -> None:
    """`shell: bash {0}` on the step that runs a floor gate.

    The fourteenth pass's first route, and the reason `shell:` is refused by
    NAME rather than read. `python` and `pwsh` fail closed on their own: neither
    is bash, so the runner cannot sit at a bash statement head in either. A
    CUSTOM TEMPLATE is the dangerous value, because it is still bash and the
    `-e` is gone. MEASURED, exit status read from the shell itself: a file
    holding `false` then `echo AFTER` run as `bash -e <file>` exits 1 and run as
    `bash <file>` exits 0. So every statement in the body is swallowed, and the
    check went on reporting the gates as run at exit 0 with this key planted on
    the real `bin/ocelli.sh gate guards` step.

    The step, its command and its `if:` are untouched, which is what makes this
    the `continue-on-error` family again in a key nobody reads as tolerating
    anything.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- {body}\n{indent}  shell: bash {{0}}")


def _a_measured_shell_on_a_gate_step(box: Sandbox) -> None:
    """`shell: bash` on the same step, which must NOT be refused.

    The other direction, and it is the one a refusal-by-name can get wrong.
    `shell: bash` is `bash --noprofile --norc -eo pipefail {0}`, which is
    STRICTER than the default `bash -e {0}` rather than looser: errexit is
    present and pipefail is added. MEASURED: `false` then `echo AFTER` under
    that argv exits 1. A check that refused every `shell:` it saw would refuse
    a workflow written more carefully than the one it accepts.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- {body}\n{indent}  shell: bash")


def _a_custom_shell_template_for_the_whole_workflow(box: Sandbox) -> None:
    """The same template at WORKFLOW level, touching no step at all.

    `defaults.run.shell` is settable on the workflow and on the job, and the
    one that matters is the one no step mentions: it takes every `run:` in the
    file with it, so a reader looking at the gate step sees nothing. That is
    `_continue_on_error_on_the_job`'s argument one level further out, and it is
    a different node of the parsed tree, so a repair could close the step key
    and leave this open.

    The block is planted above `jobs:` rather than beside a step, because
    that is the whole point of the shape. A workflow carrying no top-level
    `jobs:` key on a line of its own is `Sandbox.substitute`'s own refusal of a
    no-op rather than a second one written here, which is what that refusal is
    for: a builder that silently stopped mutating anything would leave the
    guard green and the run would report a guard that "did not fire".
    """
    box.substitute(WORKFLOW_PATH, "\njobs:\n",
                   "\ndefaults:\n  run:\n    shell: bash {0}\n\njobs:\n")


def _a_custom_shell_template_for_the_job(box: Sandbox) -> None:
    """The same template on the JOB holding the gate step.

    The third and last place `shell:` is settable, and it is a lookup of its
    own rather than a spelling of either other one: the job's value overrides
    the workflow's and the step's overrides both, so the three are read at
    three sites and a repair could close any two. This is the level
    `_continue_on_error_on_the_job` plants its key at, and it is planted
    through the same helper.
    """
    _plant_under_the_job_key(
        box, "    defaults:\n      run:\n        shell: bash {0}\n")


def _a_set_plus_e_before_a_gate_invocation(box: Sandbox) -> None:
    """`set +e` on the line above the gate, in the same `run:` body.

    The step is there, it names the gate, bash runs the gate, and the gate's
    red is discarded. MEASURED: `set +e` then `false` then `echo AFTER` under
    `bash -ec` exits 0, where `false` then `echo AFTER` exits 1.

    This is the `|| true` route written as a shell OPTION rather than as an
    operator, so the separator reading in `_tolerated_statements` cannot see it
    at all: the statement's own separators are newlines and nothing about the
    statement is unusual. It takes the rest of the body with it.

    The body therefore has to CONTINUE past the gate, which is what the `echo
    AFTER` in the measurement above is doing and what this plant omitted when
    it was first written. With the gate last, `set +e` changes nothing.
    """
    line, indent, command = _gate_step_command(box)
    # The trailing command is LOAD-BEARING and is the reason this probe went
    # HARNESS when it was first written without one. `set +e` stops errexit,
    # and errexit is not what makes the LAST command's status the script's
    # status, so with the gate last the failure still reaches the step and the
    # guard is right to count it. MEASURED, which is the pair the docstring
    # above cites: `set +e` then `false` exits 1, and `set +e` then `false`
    # then `echo` exits 0. A plant that does not match the measurement beside
    # it tests a shape that is not the defect.
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    set +e\n"
                   f"{indent}    {command}\n"
                   f'{indent}    echo "gate step done"')


def _errexit_restored_before_a_gate_invocation(box: Sandbox) -> None:
    """`set +e` and then `set -e` before the gate, which must NOT be refused.

    The direction a head test would get wrong. A body that turns errexit off
    and back on again runs the gate under errexit, so the gate's failure fails
    the step and CI runs it in the sense `--floor` means. MEASURED: `set +e`
    then `set -e` then `false` then `echo AFTER` under `bash -ec` exits 1,
    which is the control for the refuse probe above.

    `_errexit_switch` is what has to read this, and it is a reader rather than
    a substring test for the row beside it: `set +o pipefail` turns errexit off
    in neither direction and `+e` inside `+eu` turns it off.
    """
    line, indent, command = _gate_step_command(box)
    # The trailing command is LOAD-BEARING, for the reason its sibling above
    # carries the same line. With the gate last, `_errexit_exempt` short
    # circuits on `index != last` and never consults `errexit_off` at all, so
    # the `set -e` restore this probe exists to watch is not reached and the
    # probe cannot flip on its own defence. MEASURED in the S03 review's
    # fifteenth pass: destroying the restore reading left this probe green.
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    set +e\n"
                   f"{indent}    set -e\n"
                   f"{indent}    {command}\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_invocation_in_an_if_condition(box: Sandbox) -> None:
    """The gate as the CONDITION of an `if`, whose failure the shell tests.

    MEASURED: `if false; then true; fi` then `echo AFTER` under `bash -ec`
    exits 0. errexit does not fire on a command whose status is being tested,
    so the gate runs, its red is read as a branch, and the step is green. The
    runner is still at a statement head and `invoked_gates` still sees it,
    which is exactly why the shape reads as an invocation to anyone counting
    invocations.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    if {command}; then true; fi\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_invocation_in_an_if_body(box: Sandbox) -> None:
    """The same `if`, with the gate in the `then` BODY, which is REFUSED.

    This was an ACCEPT probe until the S03 review's fifteenth pass, on the
    argument that the body runs under errexit, which it does: `if true; then
    false; fi` then `echo AFTER` under `bash -ec` exits 1. That argument
    answers the wrong question. The body runs under errexit WHEN IT RUNS, and
    whether it runs depends on a condition this scanner cannot evaluate, so
    the same construct with `if false` never calls the gate at all and this
    file counted it invoked. The fifteenth pass measured five more of that
    shape, including a `case` arm, a function body and a statement after
    `exit`, where the runner is never executed.

    So the question is no longer whether the failure reaches the step. It is
    whether the shell is GUARANTEED to run the command, and a compound body is
    not. The cost is a false refusal on a workflow that would have worked, and
    it is fail-CLOSED and names the gate. No step in `.github/workflows/ci.yml`
    puts a gate in a compound body, measured at 0 such statements.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    if true; then {command}; fi\n"
                   f'{indent}    echo "gate step done"')


def _a_negated_gate_invocation(box: Sandbox) -> None:
    """`! bin/ocelli.sh gate <name>`, whose status the shell inverts.

    The third errexit-exempt context and the smallest edit of the three: one
    character. MEASURED: `! false` then `echo AFTER` under `bash -ec` exits 0.
    The gate's red becomes the step's green, and its green becomes the step's
    red, so the step is not merely tolerant of the gate failing, it is wired
    backwards. Quoted in the YAML because `!` opens a tag there.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line, f"{indent}- run: '! {command}'")


def _a_gate_invocation_in_a_function_body(box: Sandbox) -> None:
    """The gate defined in a function that nothing calls. A TOTAL bypass.

    MEASURED under `bash -e`: a body of `f() {{ echo RAN; }}` then `echo done`
    prints only `done`. The runner is never executed, and this file reported
    every floor gate invoked at exit 0 until the S03 review's fifteenth pass.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    guards_step() {{ {command}; }}\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_invocation_in_a_case_arm(box: Sandbox) -> None:
    """The gate in a `case` arm whose pattern the runner never matches.

    MEASURED: `case "x" in Windows) echo RAN ;; esac` then `echo done` prints
    only `done`. The arm reads as an invocation to anything counting names.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f'{indent}    case "$RUNNER_OS" in Windows) {command} ;; '
                   f"esac\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_invocation_after_exit(box: Sandbox) -> None:
    """The gate on a line the shell never reaches, after a top-level `exit`.

    MEASURED: `exit 0` then `echo RAN` under `bash -e` prints nothing and
    exits 0. The step is green, the gate is listed, and it did not run.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    exit 0\n"
                   f"{indent}    {command}")


def _shopt_unsets_errexit_before_a_gate(box: Sandbox) -> None:
    """`shopt -uo errexit`, which is bash's other spelling of `set +o errexit`.

    `shopt -o` writes the same option set `set -o` writes, so this turns
    errexit off and `_errexit_switch` read only the `set` family until the S03
    review's fifteenth pass. MEASURED: `shopt -uo errexit` then `false` then
    `echo AFTER` under `bash -e` exits 0.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    shopt -uo errexit\n"
                   f"{indent}    {command}\n"
                   f'{indent}    echo "gate step done"')


def _a_scoped_set_e_that_does_not_restore(box: Sandbox) -> None:
    """`set +e`, then a `set -e` inside a SUBSHELL, which restores nothing.

    The fail-OPEN half of the subshell limit, which was declared as though
    both halves were fail-closed. A `set -e` inside `( )` is scoped to the
    subshell, so errexit is still off in the parent. MEASURED: `set +e` then
    `( set -e )` then `false` then `echo AFTER` under `bash -e` exits 0.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    set +e\n"
                   f"{indent}    ( set -e )\n"
                   f"{indent}    {command}\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_in_a_case_arm_inside_a_loop(box: Sandbox) -> None:
    """A `case` arm inside a `for` body, which a HEAD test cannot see.

    The statement is `do case "$X" in z`, whose first word is `do`, so a
    reader that tests only the head never sees the `case`, and the `)` that
    follows then reads as a subshell CLOSE rather than an arm opener. The
    depth returns to zero and the gate is counted. MEASURED in the S03
    review's sixteenth pass at exit 0, with bash never running the runner.

    This is the shape that says `_compound_tokens` has to scan a whole
    statement rather than its first word.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f'{indent}    for i in 1; do case "$RUNNER_OS" in z) '
                   f"{command} ;; esac; done\n"
                   f'{indent}    echo "gate step done"')


def _a_gate_after_an_elif_chain(box: Sandbox) -> None:
    """A real top-level invocation AFTER an `elif` chain, which must PASS.

    The false-refusal direction of the same defect, and the reason `elif` is a
    net close rather than another opener. An `if`/`elif`/`fi` carries TWO
    `then` heads and ONE `fi`, so a reader that increments on each `then` and
    decrements once never returns to depth zero, and every statement after the
    `fi` is refused. The gate here is genuinely at the top level and genuinely
    fails the step, so refusing it would refuse a workflow doing the right
    thing.
    """
    line, indent, command = _gate_step_command(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    if true; then echo a; elif false; then echo "
                   f"b; fi\n"
                   f"{indent}    {command}")


def _the_unsafe_gate_named_only_in_a_comment(box: Sandbox) -> None:
    """The `unsafe` arm running something else, with the real command in a
    trailing comment, and the CI step deleted.

    `scripts/lint_policy_check.py` read the arm with
    `^\\s*unsafe\\)[^\\n]*?python3 scripts/unsafe_allowlist_check\\.py`, and
    `[^\\n]*?` reaches a `#` as happily as it reaches a command. MEASURED in a
    real clone: `bash -n` 0, the guard exit 0 PRINTING "unsafe_code denied by
    scripts/unsafe_allowlist_check.py in the `unsafe` gate, which is the
    declared substitution", `scripts/ci_floor_check.py` exit 0, the census exit
    0 and both unit suites exit 0, while the script ran nowhere and HLD 27.1's
    `unsafe_code` deny and HLD 27.2 R5 were enforced by nothing.

    The repair already existed two files over: `_ci_arm_commands` above calls
    `ci_floor_check.gate_commands`, which is comment-stripped and span-aware by
    construction, and the guard is its third caller now.

    The arm is given a real command from ANOTHER gate rather than `true`, so
    the runner stays a file whose arms all do work and the only thing this
    probe changes is which work the `unsafe` arm does.
    """
    runner = box.read("bin/ocelli.sh")
    line = next(l for l in runner.splitlines()
                if re.match(r"^\s*unsafe\)", l))
    replacement = next(
        c for c in _ci_arm_commands(box, "prose") if c.startswith("python3 "))
    box.substitute(
        "bin/ocelli.sh", line,
        f"{line[:line.index('unsafe)')]}unsafe)      {replacement} ;;"
        f"  # python3 scripts/unsafe_allowlist_check.py")
    workflow = box.read(WORKFLOW_PATH)
    wanted = "- run: python3 scripts/unsafe_allowlist_check.py"
    step = next(l for l in workflow.splitlines() if l.strip() == wanted)
    box.substitute(WORKFLOW_PATH, step + "\n", "")


def _an_unclosed_quote_in_a_ci_step(box: Sandbox) -> None:
    """A `run:` body with a quote that is opened and never closed.

    bash will not run the body either, so the direction is not in doubt. What
    is in doubt is the MESSAGE: the swallowed text can only hide commands from
    this reader, so without a refusal here the check reports whichever gate
    went missing and sends its reader to `bin/ocelli.sh` for a defect in
    `ci.yml`. This is `_arm_end`'s argument, one file along.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- run: |\n"
                   f"{indent}    echo 'never closed\n"
                   f"{indent}    {body.removeprefix('run: ')}")


def _a_runner_that_is_not_utf8(box: Sandbox) -> None:
    """`bin/ocelli.sh` as bytes no UTF-8 decoder accepts."""
    box.write("bin/ocelli.sh", b"\xff\xfe unsafe) python3 x.py ;;\n")


def _a_runner_with_no_run_gate_region(box: Sandbox) -> None:
    """`bin/ocelli.sh` with the `run_gate` region renamed out from under the
    arm parser, which is a runner somebody restructured rather than a broken
    file. The gate-arm reader refuses, and the question this guard asks about
    that arm is then UNKNOWN, which is not the same as answered no."""
    box.substitute("bin/ocelli.sh", "run_gate() {", "run_gate_renamed() {")


def _a_flow_mapping_step(box: Sandbox) -> None:
    """One step written as a flow mapping on one line.

    The third, and it is the accept direction of `_a_flow_mapping_step_with_
    run_first`. The condition here is TRUE on every automatic event, so the
    step covers the floor and the check must say so: a repair that refused
    the flow form outright would pass the refuse probe and refuse this.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- {{if: github.event_name != "
                   f"'workflow_dispatch', {body}}}")


def _a_comment_after_an_if(box: Sandbox) -> None:
    """A trailing YAML comment on an `if:` that permits every automatic event.

    The fourth, and the worst of the five, because the refusal it produced
    QUOTED the condition back at its author: the comment was part of the
    condition text, `_permits` could not read the result, and the message said
    the gate does not run on the events that condition plainly runs on. A
    reader who trusted the message would have gone looking at the condition,
    which was correct.
    """
    _, line, indent, body = _gate_step_pieces(box)
    box.substitute(WORKFLOW_PATH, line,
                   f"{indent}- if: github.event_name != 'workflow_dispatch'"
                   f"  # never on a manual dispatch\n{indent}  {body}")


def _on_as_a_block_sequence(box: Sandbox) -> None:
    """`on:` written as a block sequence of event names.

    The fifth. GitHub documents this as `on: [push, pull_request]` and a block
    sequence is the same node laid out over lines. The reader handled the
    mapping and the inline list, its docstring named those two, and this third
    shape read as declaring no automatic event at all, which refused the whole
    floor at once with a message about the workflow being manual.

    The events are taken from the workflow's own `on:` block through the
    guard's reader, so this rewrites what is there rather than asserting a
    list.
    """
    import ci_floor_check
    workflow = box.read(WORKFLOW_PATH)
    events = sorted(ci_floor_check.workflow_events(workflow))
    head = re.search(r"^on:\n(?:[ \t]+\S.*\n)+", workflow, re.M)
    if head is None or not events:
        raise AssertionError(
            "the workflow declares no `on:` block this probe can rewrite as a "
            "sequence, so it would mutate nothing.")
    box.substitute(WORKFLOW_PATH, head.group(0),
                   "on:\n" + "".join(f"  - {event}\n" for event in events))


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
# The harness's own inverted success, which nothing watched until the seventh
# pass. `scripts/guard_probe.py`'s docstring calls a probe whose guard exits 0
# "a FAILURE OF THE HARNESS, not a pass", and that sentence was carried by two
# `problems.append(message)` statements no probe reached. Measured: deleting
# the first of them and returning `"pass"` instead left `entry_sites` at 19,
# the census at exit 0 over "566 refusal(s), all claimed", `--self-test` at 0
# with 10 properties, `--profile floor` at 0 with 106 probes red and the unit
# suite at 49. The whole `guards` gate was ALL GREEN with the single mechanism
# that gives every probe result its meaning removed.
# ---------------------------------------------------------------------------


# The signature every guard script in `scripts/` writes. The state below
# replaces the first statement of `main` rather than the top of the file, and
# that is deliberate: several probe builders IMPORT the guard they aim at to
# choose their input, and a `sys.exit(0)` at module scope would take those
# builders down with it and turn a HARNESS report into an `error` report, which
# is red for the wrong reason.
GUARD_MAIN = "def main() -> int:"


def _a_probe_over_a_single_script(box: Sandbox) -> tuple[str, str]:
    """A floor probe whose whole guard is one `python3 scripts/<name>.py`.

    Selected for the property rather than named, because the property is what
    the state below needs: a guard that can be made to stop refusing by one
    edit to one file, and a probe whose declared reason for running it is that
    it refuses. A named probe would go stale the day it moved, and this
    catalogue has already paid for a probe whose discrimination rested on an
    accident of ordering.

    `control is None` matters. The state this feeds neuters the guard, so the
    control has to be the same invoke against the unmutated sandbox: it must be
    the run that PASSES on a healthy copy, or the harness's refusal would be
    read as the control's rather than as the probe's.
    """
    for guard in GUARDS:
        if not guard.file.startswith("scripts/") \
                or not guard.file.endswith(".py"):
            continue
        for probe in guard.probes:
            if probe.polarity != "refuse" or probe.defect:
                continue
            if probe.mutate is None or probe.control is not None:
                continue
            if probe.profile != "floor" or probe.needs != "none":
                continue
            if probe.invoke.key != f"python3 {guard.file}":
                continue
            if GUARD_MAIN not in box.read(guard.file):
                continue
            return guard.file, probe.id
    raise AssertionError(
        "no floor probe in this catalogue drives a guard that is one "
        "`python3 scripts/<name>.py` with a `main()` this builder can neuter, "
        "so the state this probe is about cannot be built and it would report "
        "the harness healthy without having tested it.")


def _a_guard_that_refuses_nothing(box: Sandbox) -> None:
    """Make the guard one catalogue probe aims at exit 0 on every input.

    This is the inverted-success rule's own rejected state, and the harness has
    to REPORT it rather than counting the probe as green. Nothing built it
    until the seventh pass, so `probe-runner` carried one probe, an accept
    probe on `--self-test`, for nineteen refusal sites.

    The guard still runs, still imports, and still prints its own OK line's
    absence, which is what makes this the shape a real regression takes: a
    check rewritten until it no longer detects anything is a check that exits
    0, not one that crashes.
    """
    guard_file, _ = _a_probe_over_a_single_script(box)
    box.substitute(
        guard_file, GUARD_MAIN,
        f"{GUARD_MAIN}\n    return 0  # planted: a guard that refuses nothing")


def _the_harness_must_refuse_a_guard_that_refuses_nothing(
        box: Sandbox) -> "subprocess.CompletedProcess[str]":
    """`guard_probe.py --only <the neutered guard's probe>`, status INVERTED.

    The inversion is the whole reason this probe discriminates, and it took a
    measurement to establish. The obvious form is a `refuse` probe: neuter a
    guard, run the harness, require a non-zero status. MEASURED in the S03
    review's seventh pass with the defect planted, the append at `run_probe`'s
    refuse branch deleted and `"pass"` returned instead: the inner harness
    exits 0, the OUTER harness reads that through the same deleted branch, and
    the probe prints `red` and exits 0. A probe cannot report a refusal through
    the refusal it is watching.

    `run_probe` has two such branches, one per polarity, and they are two
    refusal sites rather than one since `scripts/guards/discover.py` stopped
    identifying a refusal by the local variable its message was built in. So
    this probe is declared `accept` over the inverted status, and its own
    failure therefore travels through the OTHER branch, the one for a guard
    that refused a legitimate state. With the defect planted the outer harness
    reports HARNESS at "expected the guard to ACCEPT and it exited 1".

    The id is resolved from the catalogue at run time rather than written into
    the invoke's key, for the same reason the builder resolves the file that
    way.
    """
    _, probe_id = _a_probe_over_a_single_script(box)
    done = box.run(["python3", "scripts/guard_probe.py", "--only", probe_id])
    return subprocess.CompletedProcess(
        done.args, 0 if done.returncode != 0 else 1, done.stdout, done.stderr)


HARNESS_OVER_ONE_PROBE = Invoke(
    key="guard_probe --only <a probe whose guard is one script>, inverted",
    run=_the_harness_must_refuse_a_guard_that_refuses_nothing)


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
        gate="pins wasm",
        spec="HLD 15.2 and 27.2 R4, story E1.2 for the ceiling, and "
             "deviation D-11 for the operational parity target",
        refuses="A range where the specification requires an exact `=` pin, "
                "an `=` in front of a partial version or a second comparator "
                "after it, a pinned crate that has left the workspace table, "
                "a wasm module over its recorded ceiling, an operational "
                "parity consumer naming a target other than 5.8.2, and a "
                "generated wasm package missing either regular, "
                "byte-identical dual-licence grant.",
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
            Probe("pins.package-licence-absent",
                  _missing_packaged_apache_licence,
                  script("python3", "scripts/pin_and_size_check.py",
                         "--with-size"),
                  "LICENSE-APACHE is absent",
                  note="F-X008. The workspace's `MIT OR Apache-2.0` choice "
                       "requires both grants in the package. The probe keeps "
                       "the wasm under budget and packages the MIT grant, so "
                       "only the absent Apache grant can satisfy it."),
            Probe("pins.package-licence-symlink",
                  _symlinked_packaged_apache_licence,
                  script("python3", "scripts/pin_and_size_check.py",
                         "--with-size"),
                  "LICENSE-APACHE is a symlink, not a regular file",
                  note="F-X008. The symlink resolves to the correct "
                       "repository grant, so only accepting a link in place "
                       "of package-owned licence bytes can satisfy it."),
            Probe("pins.stale-operational-parity",
                  _stale_operational_parity_target,
                  script("node", "--test", "tools/oracle/tests/pins_test.mjs"),
                  "does not name the oracle pin",
                  note="F-X008 and D-11. The executable authority remains "
                       "the exact 5.8.2 dependency pin. The rejected command "
                       "claims 5.8.9 while the generator remains correct, so "
                       "the test must read each operational consumer rather "
                       "than merely finding 5.8.2 somewhere in the tree."),
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
                  note="D-09 is a claim about a SET of crates. A crate that "
                       "deletes the attribute must be named by the direct "
                       "comparison with EXPECTED_NO_STD_CRATES before any "
                       "dependency graph is resolved."),
            Probe("nostd.gains-a-crate", _add_no_std_to_one_other_crate,
                  script("python3", "scripts/no_std_check.py"),
                  "unexpectedly declares no_std",
                  needs="cargo", profile="deep",
                  note="The reverse set comparison. Entry-point and wgpu "
                       "crates are deliberately outside the no_std set. A "
                       "new attribute on one must be a reviewed posture "
                       "change rather than silently joining by construction."),
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
            Probe("ci-floor.multi-command-split-steps",
                  _split_a_multi_command_gate_across_steps,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="F-X010. Every exact argv remains in the same job and "
                       "in arm order, but separate YAML steps do not preserve "
                       "the arm's `&&` failure semantics. Before this story "
                       "the guard accepted this expansion."),
            Probe("ci-floor.multi-command-reordered",
                  _reorder_a_multi_command_gate_across_steps,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="F-X010. Every exact argv remains present and only "
                       "their order changes. Per-command set coverage accepted "
                       "this before the named-invocation rule."),
            Probe("ci-floor.multi-command-split-jobs",
                  _split_a_multi_command_gate_across_jobs,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="F-X010. Every exact argv remains in the workflow but "
                       "lands in a different job with no ordering edge. A "
                       "workflow-wide command set cannot prove one gate arm."),
            Probe("ci-floor.multi-command-named-in-area-job",
                  _name_a_multi_command_gate_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="F-X010's accepted direction. The useful area job and "
                       "a descriptive step name remain, while the run command "
                       "delegates ordering and exit semantics to the gate."),
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
                  "and nothing in",
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
            Probe("ci-floor.work-inside-an-if",
                  _work_inside_an_if_in_a_gate_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "so it cannot demand those commands step by step",
                  note="`unseen_commands` failed OPEN on an `if` arm. It "
                       "splits statements on `[\\n;{}()]|&&|\\|\\||\\|`, so "
                       "`then <cmd>` and `do <cmd>` are one statement whose "
                       "head is a keyword, and the keyword was in the list of "
                       "heads treated as not being the work. MEASURED: "
                       "wrapping the `bench` arm's `node --test` line as "
                       "`if node --test ...; then true; fi` and replacing the "
                       "`gate bench` step with its two `python3` commands gave "
                       "`unseen bench: None` and exit 0, five node suites out "
                       "of CI. That is byte for byte the outcome pass 6 "
                       "measured and made a rule, reached through the "
                       "statement scanner instead of through the extractor. "
                       "The declared limit said such an arm \"would be read as "
                       "having none\", which is true only when the WHOLE arm "
                       "is inside the `if`."),
            Probe("ci-floor.nested-case-after-a-keyword",
                  _nested_case_after_a_keyword,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="`NESTED_CASE` required `case` to follow `^` or one of "
                       "`[\\n;{}()&|]`, so a keyword and a space defeated it "
                       "while `ARM` went on truncating the arm at the inner "
                       "`;;`. MEASURED on a synthetic arm: the body was kept "
                       "only as far as the inner case, `arms` held one "
                       "command, `unseen` was `None`, no refusal fired and a "
                       "real command after the inner case was dropped "
                       "silently at exit 0. `case` is matched at any statement "
                       "position now, and the keyword list is spelled out "
                       "rather than replaced by a bare `\\bcase\\b`, because "
                       "a word boundary sits inside `--lower-case` too."),
            Probe("ci-floor.widened-ci-step",
                  _widen_a_ci_step_past_its_arm_command,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="`runs_command` prefix-matched, so a CI step running "
                       "the arm's command PLUS arguments satisfied it. "
                       "MEASURED: changing a step to `python3 "
                       "scripts/prose_check.py --only-this-one-file "
                       "README.md` left the check at exit 0 with the gate "
                       "reported as invoked while CI checked one file. "
                       "`ci-floor.narrowed-arm-command` probes the opposite "
                       "direction, an argument the arm already carried being "
                       "narrowed, and could not see one being added. The "
                       "comparison is on the argument VECTOR now, with the "
                       "interpreter normalised and one declared addition, "
                       "`--require-prerequisites` on corpus_tests.py, which "
                       "is strictly stronger than the arm."),
            Probe("ci-floor.nested-case-after-a-bang",
                  _nested_case_after_a_bang,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="The seventh pass's own keyword alternative, one "
                       "branch along. `\\b` needs a word character "
                       "immediately before the token it precedes and `!` is "
                       "not one, so `&& ! case ...` matched nothing while "
                       "`ARM` truncated the arm at the inner `;;`. MEASURED "
                       "on a synthetic one-line arm: one command in `arms`, "
                       "`unseen` None, no refusal, and the real trailing "
                       "command dropped at exit 0. The `\\b` is gone and the "
                       "keywords carry their own boundary, which `!` does not "
                       "need because it cannot be the tail of a word."),
            Probe("ci-floor.arm-comment-holding-a-terminator",
                  _arm_comment_holding_a_terminator,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="`SHELL_COMMENT` ran AFTER `ARM` had matched, and `ARM` "
                       "stops at the first `;;`, so a `;;` inside a shell "
                       "comment ended the arm before the comment was "
                       "stripped. MEASURED on a synthetic one-line arm whose "
                       "comment carried `;;` and whose real second command "
                       "followed it: one command in `arms`, `unseen` None, no "
                       "refusal and the second command dropped at exit 0. "
                       "Comments and joined continuations are removed from "
                       "the whole `run_gate` region before `ARM.finditer` "
                       "now, which is the only order in which a comment "
                       "cannot terminate an arm."),
            Probe("ci-floor.work-behind-the-command-builtin",
                  _work_behind_the_command_builtin,
                  script("python3", "scripts/ci_floor_check.py"),
                  "so it cannot demand those commands step by step",
                  note="The third head to the same outcome, and this file "
                       "DECLARED it: the seventh pass's limit named `command` "
                       "beside `eval` as the residue of the introducer split, "
                       "and nothing read that sentence. `command foo args` "
                       "runs foo, which is the property that moved `eval` out "
                       "of the noise list, while `command` stayed in it. "
                       "MEASURED: rewriting the `bench` arm's `node --test` "
                       "line as `command node --test ...` and replacing the "
                       "`gate bench` step with its two `python3` commands "
                       "gave `unseen['bench'] == None` and exit 0, five node "
                       "suites out of CI, which is byte for byte the sixth "
                       "and seventh passes' outcome."),
            Probe("ci-floor.lookup-with-the-command-builtin",
                  _a_lookup_with_the_command_builtin,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The other direction, and the reason `command` needed "
                       "a reader rather than a move to the other set. "
                       "`command -v x` looks a command up and runs nothing, "
                       "which is what the `panic` arm writes to decide "
                       "whether wasm-pack is there at all. A repair that "
                       "treated every `command` as work would report an "
                       "argument list as an unseen command and demand a "
                       "gate-name step for an arm that runs its own commands "
                       "as steps today, which is the runbook's sentence about "
                       "a guard that fails on everything."),
            Probe("ci-floor.arm-terminator-inside-a-quote",
                  _arm_terminator_inside_a_quote,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="The eighth pass's comment route one lexer rule along, "
                       "and stripping cannot reach this one. `ARM` stopped at "
                       "the first `;;` and a `;;` inside a quoted string is "
                       "not one. MEASURED in the ninth pass on the one-line "
                       "`fmt` arm, with `sh -n` accepting the file: one "
                       "command in `arms`, `unseen` `None`, no refusal and the "
                       "real trailing command dropped at exit 0. The arm's end "
                       "is found by a scan over quoted spans now."),
            Probe("ci-floor.quoted-string-in-an-arm-is-permitted",
                  _a_quoted_string_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the fix could get wrong, and the first "
                       "attempt DID get it wrong. Making only the arm scan "
                       "quote-aware left `STATEMENT_BREAK` splitting inside "
                       "the string, so `echo \"a ;; b\"` became `echo \"a` and "
                       "` b\"` and the check refused, naming `b\"` as a "
                       "command CI does not run. A quoted string in a gate arm "
                       "weakens nothing and the guard has to say so."),
            Probe("ci-floor.nested-case-after-while",
                  _nested_case_after_while,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="`NESTED_CASE` alternated `then|do|else|elif` plus `!` "
                       "while `SHELL_INTRODUCERS` twenty lines below held ten "
                       "names, `while`, `until` and `if` among them. The same "
                       "file knew `while` introduces a command in one function "
                       "and not in the other. MEASURED in the ninth pass on "
                       "the one-line `fmt` arm: one command in `arms`, "
                       "`unseen` `None`, no refusal and the real trailing "
                       "command dropped at exit 0, with `sh -n` green. The "
                       "alternation is DERIVED from the set now."),
            Probe("ci-floor.nested-case-after-if",
                  _nested_case_after_if,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="The other keyword the two lists differed on. The "
                       "seventh pass's probe wrote `if true; then case ...`, "
                       "so the inner case followed `then`, which WAS in the "
                       "alternation. `if case ... esac; then` puts it after "
                       "`if`, which was not, and it is measured at exit 0 with "
                       "the trailing command dropped."),
            Probe("ci-floor.nested-case-in-a-backtick",
                  _nested_case_in_a_backtick,
                  script("python3", "scripts/ci_floor_check.py"),
                  "holds a nested `case`",
                  note="The FOURTH terminator shape and the only one that was "
                       "fully fail-open after the ninth pass. A backtick is "
                       "not `^`, not one of `[\\n;{}()&|]` and not a "
                       "`SHELL_INTRODUCERS` word, and `_quote_spans` knew `\"` "
                       "and `'` and not `` ` ``, so `$(case ...)` was caught "
                       "only because `(` sits in that class and "
                       "`` `case ...` `` was caught by nothing. MEASURED in "
                       "the tenth pass on the one-line `fmt` arm, with "
                       "`bash -n` green: exit 0, one command in the arm, "
                       "`unseen` `None`, and bash really runs the dropped "
                       "command. Worse than the comment shape, because the "
                       "residue's heads are `test` and `echo`, both in "
                       "`SHELL_NOISE`, so the unseen-command rule fires on "
                       "nothing either. The backtick is a span in "
                       "`shell_pieces` and a member of `NESTED_CASE`'s class "
                       "now, so the arm cannot end inside a substitution and "
                       "a `case` cannot hide in one."),
            Probe("ci-floor.a-backtick-in-an-arm",
                  _a_backtick_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the backtick fix could get wrong. A "
                       "command substitution in a gate arm is ordinary shell "
                       "and weakens nothing, and the over-tight repair, "
                       "refusing any backtick in an arm, would pass the probe "
                       "above and refuse a legitimate runner. The "
                       "substitution's head is in `SHELL_NOISE`, so the "
                       "delimiter is the only thing under test."),
            Probe("ci-floor.unbalanced-quote-in-an-arm",
                  _an_unbalanced_quote_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "span is opened and never closed",
                  note="The fail-CLOSED half of the span scan. A parser that "
                       "cannot delimit an arm must refuse rather than report "
                       "whatever it stopped at. The probe plants the quote in "
                       "the LAST arm, which is the only position from which a "
                       "later quote in the region cannot close it, and that "
                       "dependence is declared in the entry's limit rather "
                       "than left to be discovered."),
            Probe("ci-floor.comment-after-a-substitution",
                  _comment_after_a_substitution_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="A REGRESSION the tenth pass introduced, not a "
                       "survival. That pass made `)` a word start, which is "
                       "right, and the scanner could not tell an operator `)` "
                       "from the one closing a `$( ... )`, because it knew "
                       "backticks and not `$(`. MEASURED: `bash -c 'echo "
                       "A$(printf x)#no && echo RAN_SECOND'` prints both "
                       "lines, this check exited 0 with the trailing command "
                       "dropped, and the same input at `e2b11d8`, before `)` "
                       "joined the set, exited 1. There is one scanner with a "
                       "span stack now and a `)` it opened is never a word "
                       "boundary."),
            Probe("ci-floor.a-substitution-in-an-arm",
                  _a_substitution_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the fix could get wrong. `$( ... )` in "
                       "a gate arm is ordinary shell, and the two over-tight "
                       "repairs, taking `)` out of `WORD_BREAK` or "
                       "refusing `$(` outright, would each pass the probe "
                       "above while either reopening the tenth pass's `;#` "
                       "route or refusing a legitimate runner."),
            Probe("ci-floor.gate-name-with-a-digit",
                  _a_gate_named_outside_the_python_class,
                  script("python3", "scripts/ci_floor_check.py"),
                  "gate is in the CI floor",
                  note="The third foreign grammar. `bin/ocelli.sh` reads an "
                       "entry with `IFS='|' read -r name gpu desc` and "
                       "imposes no character class, and this file and "
                       "scripts/guards/census.py both matched `[a-z-]+` in "
                       "two copies of one regex. MEASURED at HEAD: bash 29 "
                       "gates against Python 28, this check exit 0, the "
                       "census exit 0, `gates_declared` unmoved, and "
                       "`gate --floor` selecting a gate no CI step ran. An "
                       "entry the reader cannot use was an omission, which is "
                       "the one outcome a guard may not have. What this "
                       "watches, said exactly since the twelfth pass: "
                       "`prose2` is INSIDE the widened class, so the refusal "
                       "here is the floor-coverage one and what is proved is "
                       "that the entry is COUNTED. The name-class refusal is "
                       "`ci-floor.gate-name-outside-the-class` and it was "
                       "watched by nothing at all."),
            Probe("ci-floor.gate-name-outside-the-class",
                  _a_gate_named_outside_the_name_class,
                  script("python3", "scripts/ci_floor_check.py"),
                  "whose name is outside",
                  note="The refusal `gate_row_problems` has for a name it "
                       "cannot use, and nothing reached it. MEASURED with the "
                       "`if not GATE_NAME.match(name)` branch disabled: the "
                       "census, all 54 `ci-floor` probes and both unit suites "
                       "stayed at their unmutated status, and the unit test "
                       "carrying the refusal's name planted `prose2`, which "
                       "is inside the class, and asserted the problem list "
                       "was EMPTY. A dot is the shape that matters, because "
                       "the refusal's own sentence is that a gate name "
                       "reaches `re.escape`-free patterns in three files and "
                       "a dot is a wildcard in every one of them."),
            Probe("ci-floor.gate-name-with-a-digit-permitted",
                  _a_gate_name_with_a_digit_is_permitted,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the widened class could get wrong, and "
                       "it is the sentence the probe above wrote down and did "
                       "not watch: a gate named with a digit whose arm CI "
                       "really runs is a legitimate runner. The over-tight "
                       "repair, narrowing `GATE_NAME` back towards `[a-z-]+`, "
                       "passes every refusal probe beside this one."),
            Probe("ci-floor.heredoc-delimiter-backslash-quoted",
                  _a_backslash_quoted_heredoc_delimiter,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="The twelfth pass's first measured fail-open, and the "
                       "last hand-written production in the tokenizer. "
                       "`HEREDOC` spelled bash's delimiter as a regex with an "
                       "invented character class, `(['\\\"]?)([A-Za-z_]\\\\w*)"
                       "\\\\2`, and bash accepts any WORD quoted by any of "
                       "three mechanisms. Unrecognised, the body is scanned "
                       "as CODE and its `;;` ends the arm. MEASURED with "
                       "`bash -n` green: exit 0 with the trailing command "
                       "dropped, and the same shape run under bash prints "
                       "both markers."),
            Probe("ci-floor.heredoc-delimiter-quoted-with-a-hyphen",
                  _a_quoted_heredoc_delimiter_with_a_hyphen,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="The second, and the one that shows the class was the "
                       "wrong SHAPE rather than the wrong class: `\\\\w*` "
                       "stopped at the hyphen and the back-reference to the "
                       "opening quote then failed to match it. Same "
                       "measurement, exit 0 with the command dropped. The "
                       "message is worded apart from its sibling "
                       "deliberately: two refusals whose words normalise "
                       "alike are ONE site to the census, and the two probes "
                       "would read as covering one refusal between them."),
            Probe("ci-floor.heredoc-body-never-closed",
                  _a_heredoc_body_that_never_closes,
                  script("python3", "scripts/ci_floor_check.py"),
                  "never meets its delimiter",
                  note="Fail-closed before and after, and the REFUSAL is what "
                       "changed. `_heredoc_end`'s docstring said this state "
                       "fires the caller's unclosed-span refusal, and "
                       "`shell_pieces` cleared `pending` before its own `if "
                       "pending:` test, so the guard said the arm reaches the "
                       "end of the region with no `;;` terminator about an "
                       "arm whose `;;` is right there. bash refuses this "
                       "runner too, warning that the here-document is "
                       "delimited by end of file."),
            Probe("ci-floor.heredoc-in-an-arm",
                  _a_heredoc_in_an_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the delimiter fix could get wrong, and "
                       "the previous scanner DID get it wrong. MEASURED with "
                       "this exact shape before the fix: exit 1 reporting "
                       "that the arm runs 'a note', 'EOF-1', so two lines of "
                       "English were demanded of CI as commands. A body is "
                       "DATA, it is its own piece kind now, and "
                       "`_split_statements` drops it."),
            Probe("ci-floor.continuation-after-an-arm-comment",
                  _a_continuation_at_the_end_of_an_arm_comment,
                  script("python3", "scripts/ci_floor_check.py"),
                  "visible multi-command floor arm must be invoked by name",
                  note="A regex pre-pass over shell running BEFORE the "
                       "tokenizer, which is the one thing the tokenizer's own "
                       "header says no pass may do. MEASURED: a comment "
                       "line ending in a backslash, then `echo B`, prints "
                       "both under bash, so a backslash "
                       "ending a comment continues nothing, and "
                       "`CONTINUATION.sub` joined the next line INTO the "
                       "comment. The planted command vanished entirely, "
                       "`arms` came back holding the NEXT gate's command, and "
                       "the check refused while naming two gates neither of "
                       "which was the one edited."),
            Probe("ci-floor.continuation-inside-an-arm-command",
                  _a_continuation_inside_an_arm_command,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the fix could get wrong, and why the "
                       "continuation is a PIECE the scanner emits rather than "
                       "a deletion. `gate_commands` extracts with a class "
                       "that stops at a backslash, so a command left unjoined "
                       "loses everything after it and the arm command CI runs "
                       "verbatim reads as absent. Four arms in the runner are "
                       "written this way today."),
            Probe("ci-floor.gates-entry-missing-a-field",
                  lambda box: _a_gates_entry_the_reader_cannot_use(
                      box, '"probe-extra|no"'),
                  script("python3", "scripts/ci_floor_check.py"),
                  "which is not `name|needs_gpu|description`",
                  note="The runner reads three fields and an entry with two "
                       "leaves `desc` empty there and a row this file would "
                       "have to guess at here. Refused rather than guessed, "
                       "on the same argument as the name class above."),
            Probe("ci-floor.gates-gpu-column-unknown",
                  lambda box: _a_gates_entry_the_reader_cannot_use(
                      box, '"probe-extra|maybe|a third value for the column"'),
                  script("python3", "scripts/ci_floor_check.py"),
                  "in the GPU column, which is neither",
                  note="`gpu_gates` reads that column to decide which "
                       "excluded gate CI may not run at all under deviation "
                       "D-04, and an unrecognised value reads there as `no`, "
                       "which is the permissive answer. The runner's own "
                       "comment above the array declares the two values."),
            Probe("ci-floor.gates-array-unreadable",
                  lambda box: box.substitute(
                      "bin/ocelli.sh", "GATES=(\n", "GATE_LIST=(\n"),
                  script("python3", "scripts/ci_floor_check.py"),
                  "carries no `GATES=(`",
                  note="The fail-closed half of reading the array. A reader "
                       "that found nothing and reported agreement would say "
                       "the floor is covered about a list it could not read, "
                       "which is the shape this repository refuses "
                       "everywhere else."),
            Probe("ci-floor.comment-inside-the-gates-array",
                  _a_comment_inside_the_gates_array,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the entry reader could get wrong. bash "
                       "ignores a comment and a blank line inside an array, "
                       "so a reader demanding one quoted entry per line would "
                       "refuse a legitimate runner. The array is read as "
                       "shell WORDS by the same scanner the arms are read "
                       "with, which is what makes both true at once."),
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

            # -- the workflow is YAML, and it was read by hand -------------
            Probe("ci-floor.step-keys-in-the-other-order",
                  _step_keys_in_the_other_order,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="The twelfth pass's first route and the only one no "
                       "spelling rule could have closed. `run_commands` "
                       "attached whatever `if:` it had seen SO FAR, so "
                       "`- run: bin/ocelli.sh gate guards` followed by its "
                       "`if:` exited 0 while the identical two lines in the "
                       "other order exited 1. A YAML mapping has no key "
                       "order, so one workflow got two answers, and the gate "
                       "it got them on watches every other gate. "
                       "`ci-floor.event-gated` is the same condition written "
                       "the other way round and is what proves the difference "
                       "was the ORDER."),
            Probe("ci-floor.quoted-if-key",
                  _a_quoted_if_key,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="The same route spelled `\"if\":`, with the key above "
                       "the `run:` so that order is not what carries it. The "
                       "reader's pattern was `^\\s*(?:-\\s+)?if:` while its "
                       "own `on:` head pattern spelled "
                       "`(?:on|\"on\"|'on'|true)`, so this file knew keys may "
                       "be quoted in one function and not in the other. "
                       "Measured at exit 0."),
            Probe("ci-floor.flow-mapping-run-before-if",
                  _a_flow_mapping_step_with_run_first,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="Route 1's third spelling, a multi-line flow mapping "
                       "with `run` before `if`. The block form and the flow "
                       "form are one node to a parser and were two answers to "
                       "a line reader. Measured at exit 0."),
            Probe("ci-floor.folded-run-scalar",
                  _a_folded_run_scalar_narrowing_a_command,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="The seventh pass's hole reached by changing one "
                       "character. A `run: >` folds its body into one line and "
                       "the reader split a block scalar per LINE, so a "
                       "narrowed command became two commands and the argv "
                       "equality matched the first. MEASURED at exit 0, where "
                       "`ci-floor.widened-ci-step` writes the same narrowing "
                       "on one plain line and is refused at exit 1."),
            Probe("ci-floor.quoted-event-key",
                  _a_quoted_event_key_in_on,
                  script("python3", "scripts/ci_floor_check.py"),
                  "is behind a condition that does not run it on",
                  note="Route 3. `\"pull_request\":` in the `on:` block "
                       "narrowed the event set the floor is checked against, "
                       "so with the `guards` step gated to push the check "
                       "printed \"all 25 floor gate(s) are invoked by CI on "
                       "push\" at exit 0. The head pattern already allowed a "
                       "quoted key and the body scan did not, which is one "
                       "rule written twice."),
            Probe("ci-floor.gate-name-inside-a-run-string",
                  _a_gate_name_inside_a_run_string,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="Route 4a, and it is the comment-only lesson one "
                       "language along. A gate name was matched anywhere in a "
                       "`run:` line and a string is anywhere, so `echo \"... "
                       "run bin/ocelli.sh gate panic locally\"` satisfied the "
                       "gate at exit 0 with the real step deleted. `panic` is "
                       "HLD section 23's wasm panic-hook proof, the one "
                       "property no native test can observe. `shell_pieces` "
                       "already put that string in a span and this file used "
                       "it on `bin/ocelli.sh` and never on a `run:` body."),
            Probe("ci-floor.run-key-under-with",
                  _a_run_key_under_with,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="Route 4b. A `run:` was a command at any depth, so an "
                       "action INPUT called `run` under `with:` satisfied the "
                       "gate at exit 0. GitHub hands that string to the "
                       "action and runs no shell with it. A `run:` is reached "
                       "as `jobs.<id>.steps[n].run` now and nowhere else."),
            Probe("ci-floor.workflow-unparseable",
                  _an_unparseable_workflow,
                  script("python3", "scripts/ci_floor_check.py"),
                  "cannot be parsed as YAML",
                  note="The first of the two refusals the parse introduces, "
                       "and it is a fail-open in its own right rather than "
                       "only a fail-closed half. MEASURED against the line "
                       "reader: `- run: [bin/ocelli.sh gate guards`, which no "
                       "YAML parser accepts and which GitHub Actions "
                       "therefore cannot run at all, left the check at exit 0 "
                       "reporting the whole floor covered, because the text "
                       "after `run:` still held the gate name. GitHub reads "
                       "this file with a YAML parser, so a file no parser can "
                       "read is a workflow that does not run, and reporting "
                       "coverage over whatever a line scanner salvaged from "
                       "it is the widest false green available here. It arrives under the same `FAIL:` header as "
                       "every other refusal rather than as a traceback, which "
                       "is the presentation the fifth pass fixed for the "
                       "runner's parsers."),
            Probe("ci-floor.run-that-is-not-a-scalar",
                  _a_run_that_is_not_a_scalar,
                  script("python3", "scripts/ci_floor_check.py"),
                  "which this reader cannot use",
                  note="The second refusal the parse introduces. The tree is "
                       "readable and the step's shape is not one GitHub "
                       "accepts, so it is refused by name rather than read as "
                       "a smaller workflow. A shape skipped here is a step "
                       "whose gate then reads as uninvoked, or worse as "
                       "covered by whatever was read in its place, which is "
                       "the omission-rather-than-refusal shape the eleventh "
                       "pass removed from the GATES reader."),
            Probe("ci-floor.quoted-run-scalar",
                  _a_quoted_run_scalar,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The first of five legitimate workflows the line "
                       "reader REFUSED, and a guard that refuses a legitimate "
                       "state is the runbook's own sentence. Everything after "
                       "`run:` was the command, quotes included, so the argv "
                       "comparison saw `\"python3` and reported the gate "
                       "uninvoked while CI ran it. Measured at exit 1."),
            Probe("ci-floor.block-scalar-indentation-indicator",
                  _a_block_scalar_with_an_indentation_indicator,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The second. The reader compared the text after `run:` "
                       "against six literal block headers and `|2` is in none "
                       "of them, so the body was never read as a body and the "
                       "command vanished. An indentation indicator is "
                       "ordinary YAML and produces an identical value."),
            Probe("ci-floor.flow-mapping-step-is-permitted",
                  _a_flow_mapping_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The third, and the direction "
                       "`ci-floor.flow-mapping-run-before-if` could get "
                       "wrong. A repair that refused the flow form outright "
                       "would pass that probe and refuse this workflow, which "
                       "is the trade the dependency-free option would have "
                       "had to make on three of these five."),
            Probe("ci-floor.comment-after-an-if",
                  _a_comment_after_an_if,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The fourth and the worst of them, because the refusal "
                       "QUOTED the condition back at its author. The comment "
                       "was part of the condition text, `_permits` could not "
                       "read the result, and the message said the gate does "
                       "not run on the events that condition plainly runs on. "
                       "A reader who trusted the message would have gone and "
                       "looked at a correct condition."),
            Probe("ci-floor.on-as-a-block-sequence",
                  _on_as_a_block_sequence,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The fifth. GitHub documents this as `on: [push, "
                       "pull_request]` and a block sequence is the same node "
                       "laid out over lines. `workflow_events` handled the "
                       "mapping and the inline list, its docstring named "
                       "those two, and this third shape read as declaring no "
                       "automatic event, which refused the WHOLE floor at "
                       "once with a message about the workflow being manual."),
            Probe("ci-floor.background-operator-in-an-arm",
                  _a_background_operator_in_a_gate_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "so it cannot demand those commands step by step",
                  note="**The fourteenth route, and it is the sixth, seventh "
                       "and eighth passes with a different operator.** bash's "
                       "`&` terminates a list exactly as `;` does and "
                       "`STATEMENT_BREAK` carried `&&` and no `&`, so `A & B` "
                       "was ONE statement whose head is `A` and a head that is "
                       "a `COMMAND_PREFIXES` prefix took `B` out of "
                       "`unseen_commands` with it. MEASURED: the `&&` before "
                       "`node --test` in the `bench` arm rewritten as `&` on "
                       "one line, with the gate-name step replaced by the "
                       "arm's two extractable commands, gave `bash -n` 0 and "
                       "the check exit 0 printing \"every command in each "
                       "gate's arm\", `bench` gone from the named-only list "
                       "and six node suites out of CI. The oracle in "
                       "`scripts/tests/test_guard_readers.py` could not see "
                       "it: `scanner_keeps_in_the_arm` tests where the arm "
                       "ENDS and every marker was inside the extent either "
                       "way. `scanner_runs_in_the_arm` is what reaches it."),
            Probe("ci-floor.redirection-in-an-arm-is-permitted",
                  _a_redirection_in_a_gate_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction the `&` fix could get wrong, and the "
                       "naive spelling DID get it wrong. `&` is the second "
                       "character of `>&` and `<&` and the first of `&>`. "
                       "MEASURED with `r\"[\\n;{}()]|&&|\\|\\||\\||&\"` over "
                       "the real runner: `unseen['panic']` grew the entry "
                       "`'2'`, a file descriptor reported as a command CI does "
                       "not run, out of the `panic` arm's own `echo "
                       "\"wasm-pack "
                       "is not installed. ...\" >&2`. It cost no exit code "
                       "there only because `panic` already holds unseen "
                       "commands and CI names the gate, so an arm whose only "
                       "unextractable text was a redirection would have "
                       "refused a legitimate state."),
            Probe("ci-floor.gate-name-in-a-heredoc-body",
                  _a_gate_name_in_a_heredoc_body,
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in",
                  note="Route 4a of the twelfth pass in the spelling that pass "
                       "did not close. `run_commands` read a `run:` body one "
                       "LINE at a time while the workflow itself was parsed, "
                       "so the cross-line state that makes a here-document "
                       "body DATA was discarded. MEASURED with the "
                       "`bin/ocelli.sh gate panic` step rewritten as `cat "
                       "<<'EOF'` / the invocation / `EOF`: exit 0, and bash "
                       "confirms the runner is never called. `panic` is HLD "
                       "section 23's wasm panic-hook proof, the one property "
                       "no native test can observe. The continuation spelling, "
                       "`echo not \\\\` then the invocation, reaches the same "
                       "place and bash prints `not bin/ocelli.sh gate panic`."),
            Probe("ci-floor.continued-ci-command-is-permitted",
                  _a_continued_command_in_a_ci_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The FALSE REFUSAL half of the same line of code, which "
                       "is why the fix arrives with an accept probe. Read line "
                       "by line, `python3 "
                       "scripts/staged_content_check.py \\\\` "
                       "and `--tracked` are two commands and neither is the "
                       "arm's, so the `content` gate read as uninvoked. "
                       "MEASURED at exit 1, while bash runs it as one "
                       "command."),
            Probe("ci-floor.continue-on-error-on-a-gate-step",
                  _continue_on_error_on_a_gate_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "because it is not guaranteed to run it and report its failure",
                  note="`continue-on-error` was in the parsed tree and nothing "
                       "read it. The string occurred nowhere in `scripts/`, in "
                       "`docs/lld/guards.md` or in the runbook. MEASURED as "
                       "valid YAML leaving the check at exit 0 with the gate "
                       "reported covered. `--floor` claims to be what CI runs, "
                       "and a step whose failure cannot fail the run does not "
                       "run the gate in the sense that claim means, on the "
                       "gate that watches every other gate, in one line that "
                       "reads as tolerating flakiness."),
            Probe("ci-floor.continue-on-error-on-the-job",
                  _continue_on_error_on_the_job,
                  script("python3", "scripts/ci_floor_check.py"),
                  "because it is not guaranteed to run it and report its failure",
                  note="The same key one level up, where a reader looking at "
                       "the STEP sees nothing at all, and it takes every step "
                       "in the job with it. Both levels are read now, which is "
                       "why there are two probes rather than one: the key is "
                       "reached through a different node of the parsed tree in "
                       "each and a repair could close either alone."),
            Probe("ci-floor.gate-step-failure-swallowed",
                  _a_gate_step_whose_failure_is_swallowed,
                  script("python3", "scripts/ci_floor_check.py"),
                  "because it is not guaranteed to run it and report its failure",
                  note="`|| true` appended to the run, the third route to the "
                       "same end. The step is there, it names the gate, bash "
                       "runs the gate and the step's exit status is `true`'s. "
                       "This one was arguably inside the declared limit, that "
                       "the reader \"splits on the boolean operators without "
                       "evaluating them\", and the other two were covered by "
                       "nothing. `_tolerated_statements` is measured against "
                       "`bash -e` rather than read out of the errexit "
                       "paragraph, because the obvious reading of that "
                       "paragraph is wrong: `false && true` followed by "
                       "another line exits 0."),
            Probe("ci-floor.gate-after-swallowed-and-condition",
                  _a_gate_after_a_swallowed_and_condition,
                  script("python3", "scripts/ci_floor_check.py"),
                  "an earlier `&&` means it runs only when the left-hand side succeeds",
                  note="The gate is the right side of `false && gate`, with "
                       "a later successful statement. bash exits 0 without "
                       "running the gate. The scanner must discredit the "
                       "invocation without also rejecting a terminal `cd x "
                       "&& gate`, whose failed prefix makes the step red."),
            Probe("ci-floor.custom-shell-template-on-a-gate-step",
                  _a_custom_shell_template_on_a_gate_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "sets a `shell:` on step",
                  note="The fourteenth pass's first route, and it is the "
                       "`continue-on-error` family in a key nobody reads as "
                       "tolerating anything. The step is untouched, it names "
                       "the gate, and `bash {0}` is still bash with NO `-e`. "
                       "MEASURED, status read from the shell: a file holding "
                       "`false` then `echo AFTER` exits 1 under `bash -e "
                       "<file>` and 0 under `bash <file>`, so every statement "
                       "in the body is swallowed, and the check exited 0 with "
                       "the key planted on the real `bin/ocelli.sh gate "
                       "guards` step. `python` and `pwsh` fail closed without "
                       "this rule, neither being bash, which is why the "
                       "refusal is by NAME against a measured set rather than "
                       "a list of bad values."),
            Probe("ci-floor.measured-shell-on-a-gate-step",
                  _a_measured_shell_on_a_gate_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "floor gate(s) are invoked by CI on",
                  polarity="accept",
                  note="The direction a refusal-by-name can get wrong. "
                       "`shell: bash` is `bash --noprofile --norc -eo pipefail "
                       "{0}`, which is STRICTER than the default `bash -e "
                       "{0}`: errexit is present and pipefail is added. "
                       "MEASURED at exit 1 for `false` then `echo AFTER` under "
                       "that argv. A check that refused every `shell:` it saw "
                       "would refuse a workflow written more carefully than "
                       "the one it accepts, which is the runbook's own "
                       "sentence about a guard that fails on everything."),
            Probe("ci-floor.custom-shell-template-for-the-workflow",
                  _a_custom_shell_template_for_the_whole_workflow,
                  script("python3", "scripts/ci_floor_check.py"),
                  "sets `defaults.run.shell:` on the workflow",
                  note="The same template written ONCE at workflow level, "
                       "touching no step, and it takes every `run:` in the "
                       "file with it. That is "
                       "`ci-floor.continue-on-error-on-the-job`'s argument one "
                       "level further out: a reader looking at the gate step "
                       "sees nothing at all, and the key is reached through a "
                       "different node of the parsed tree, so a repair could "
                       "close the step key and leave this open. MEASURED at "
                       "exit 0 before the fix."),
            Probe("ci-floor.custom-shell-template-for-the-job",
                  _a_custom_shell_template_for_the_job,
                  script("python3", "scripts/ci_floor_check.py"),
                  "sets `defaults.run.shell:` on the job",
                  note="The third and last place `shell:` is settable, and it "
                       "is a lookup of its own rather than a spelling of "
                       "either other one: the job's value overrides the "
                       "workflow's and the step's overrides both, so the three "
                       "are read at three sites and a repair could close any "
                       "two. Deleting the line that falls back from the job to "
                       "the workflow leaves this probe red and takes "
                       "`ci-floor.custom-shell-template-for-the-workflow` to "
                       "HARNESS, and reading no job `defaults:` at all does "
                       "the reverse, which is what says the two are not one "
                       "probe written twice."),
            Probe("ci-floor.set-plus-e-before-a-gate-step",
                  _a_set_plus_e_before_a_gate_invocation,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `set`",
                  note="`|| true` written as a shell OPTION rather than as an "
                       "operator, so the separator reading in "
                       "`_tolerated_statements` cannot see it: the statement's "
                       "own separators are newlines and nothing about the "
                       "statement is unusual. MEASURED under `bash -ec`: `set "
                       "+e` then `false` then `echo AFTER` exits 0, where "
                       "`false` then `echo AFTER` exits 1. It takes the rest "
                       "of the body with it, which is what makes it worse "
                       "than the `|| true` the thirteenth pass measured."),
            Probe("ci-floor.errexit-restored-before-a-gate-step",
                  _errexit_restored_before_a_gate_invocation,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `set`",
                  note="The direction a `set +e` head test would get wrong. A "
                       "body that turns errexit off and back on runs the gate "
                       "under errexit, so the gate's red fails the step and CI "
                       "runs it in the sense `--floor` means. MEASURED under "
                       "`bash -ec`: `set +e` then `set -e` then `false` then "
                       "`echo AFTER` exits 1, against the 0 the probe above "
                       "measures. `_errexit_switch` is a reader rather than a "
                       "substring test for the row beside it: `set +o "
                       "pipefail` turns errexit off in neither direction and "
                       "`+e` inside `+eu` turns it off."),
            Probe("ci-floor.gate-inside-an-if-condition",
                  _a_gate_invocation_in_an_if_condition,
                  script("python3", "scripts/ci_floor_check.py"),
                  "the shell keyword `if`",
                  note="The runner is still at a statement head, so "
                       "`invoked_gates` still sees it and the shape reads as "
                       "an invocation to anyone counting invocations. errexit "
                       "does not fire on a command whose status is being "
                       "tested: MEASURED under `bash -ec`, `if false; then "
                       "true; fi` then `echo AFTER` exits 0. The gate runs, "
                       "its red is read as a branch, and the step is green."),
            Probe("ci-floor.gate-inside-an-if-body",
                  _a_gate_invocation_in_an_if_body,
                  script("python3", "scripts/ci_floor_check.py"),
                  "the shell keyword `if`",
                  note="An ACCEPT probe until the S03 review's fifteenth "
                       "pass, on the argument that a `then` body runs under "
                       "errexit. It does, WHEN IT RUNS, and that answers the "
                       "wrong question: the same construct with `if false` "
                       "never calls the gate and this file counted it "
                       "invoked. Five further shapes of that class were "
                       "measured at exit 0 in the same pass, including a "
                       "`case` arm, a function body and a statement after "
                       "`exit`, where the runner cannot execute at all. The "
                       "question is whether the shell is GUARANTEED to run "
                       "the command, and a compound body is not. The cost is "
                       "a false refusal that names the gate, and "
                       "`.github/workflows/ci.yml` carries 0 compound "
                       "statements in its `run:` bodies."),
            Probe("ci-floor.gate-in-a-function-body",
                  _a_gate_invocation_in_a_function_body,
                  script("python3", "scripts/ci_floor_check.py"),
                  "carries a `(`",
                  note="A TOTAL bypass rather than a discarded failure. The "
                       "runner is never executed at all, and the check "
                       "reported every floor gate invoked at exit 0. Found by "
                       "the S03 review's fifteenth pass, which measured the "
                       "grammar of the generated oracle rather than the "
                       "reader, and found six contexts the grammar had no "
                       "production for."),
            Probe("ci-floor.gate-in-an-unmatched-case-arm",
                  _a_gate_invocation_in_a_case_arm,
                  script("python3", "scripts/ci_floor_check.py"),
                  "the shell keyword `case`",
                  note="The `case` arm is the shape a real workflow would "
                       "plausibly carry, since gating a step on `$RUNNER_OS` "
                       "is ordinary. On every runner but the named one the "
                       "gate does not run, and nothing said so."),
            Probe("ci-floor.gate-after-a-top-level-exit",
                  _a_gate_invocation_after_exit,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `exit`",
                  note="Unreachable code, which is the one shape where no "
                       "condition and no status is involved at all: the "
                       "statement simply cannot execute. It was counted."),
            Probe("ci-floor.shopt-unsets-errexit",
                  _shopt_unsets_errexit_before_a_gate,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `shopt`",
                  note="`shopt -o` writes the `set -o` option set, so this is "
                       "`set +o errexit` in a spelling `_errexit_switch` did "
                       "not read. The `set` family was measured exhaustively "
                       "in the fourteenth pass and `shopt` was not considered "
                       "at all, which is the enumeration failure one level up "
                       "from the one that pass fixed."),
            Probe("ci-floor.scoped-set-e-does-not-restore",
                  _a_scoped_set_e_that_does_not_restore,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `set`",
                  note="The fail-OPEN half of the subshell limit, which "
                       "`_errexit_exempt` and this catalogue both declared as "
                       "though both halves were fail-closed. A `set -e` "
                       "inside `( )` is scoped to the subshell and restores "
                       "nothing in the parent, so reading it as a restore "
                       "counted the gate. The declared half does hold and is "
                       "measured: a `set +e` inside an uncalled function "
                       "still refuses."),
            Probe("ci-floor.gate-in-a-case-arm-inside-a-loop",
                  _a_gate_in_a_case_arm_inside_a_loop,
                  script("python3", "scripts/ci_floor_check.py"),
                  "the shell keyword `for`",
                  note="The shape that measured the FIFTEENTH pass's own "
                       "nesting rule as a head test. `do case ... in z` hides "
                       "the `case` behind the `do`, so the `)` read as a "
                       "subshell close and the depth returned to zero. Found "
                       "by fuzzing the rule rather than by reading it, which "
                       "is how the class this repository keeps finding gets "
                       "found."),
            Probe("ci-floor.gate-after-an-elif-chain",
                  _a_gate_after_an_elif_chain,
                  script("python3", "scripts/ci_floor_check.py"),
                  "the shell keyword `if`",
                  note="The false-refusal direction of the same defect and "
                       "the reason `elif` is a net CLOSE. Two `then` and one "
                       "`fi` left the depth above zero for the rest of the "
                       "body, so a genuine top-level invocation after the "
                       "`fi` was refused. A guard that refuses a legitimate "
                       "state is as useless as one that refuses nothing."),
            Probe("ci-floor.negated-gate-step",
                  _a_negated_gate_invocation,
                  script("python3", "scripts/ci_floor_check.py"),
                  "a `!` negation",
                  note="The third errexit-exempt context and the smallest edit "
                       "of the three: one character. MEASURED under `bash "
                       "-ec`: `! false` then `echo AFTER` exits 0. The gate's "
                       "red becomes the step's green and its green becomes the "
                       "step's red, so the step is not merely tolerant of the "
                       "gate failing, it is wired backwards, and a reader of "
                       "`ci.yml` still sees the gate invoked."),
            Probe("ci-floor.unclosed-quote-in-a-ci-step",
                  _an_unclosed_quote_in_a_ci_step,
                  script("python3", "scripts/ci_floor_check.py"),
                  "opened and never closed",
                  note="The refusal that arrived with the whole-body scan. An "
                       "unclosed span swallows everything after it into one "
                       "piece, which can only HIDE commands and therefore only "
                       "cause refusals, so the direction was never in doubt. "
                       "The MESSAGE was: without this the check names "
                       "whichever gate went missing and sends its reader to "
                       "`bin/ocelli.sh` for a defect in `ci.yml`. That is "
                       "`_arm_end`'s argument one file along, and bash will "
                       "not run the body either."),
            Probe("ci-floor.pyyaml-absent", None,
                  script("python3", "-S", "scripts/ci_floor_check.py"),
                  "PyYAML is not installed",
                  control=script("python3", "scripts/ci_floor_check.py"),
                  note="The dependency's own failure mode, and the reason it "
                       "is a refusal rather than a fallback. `-S` skips "
                       "`site`, so site-packages is off the path and the "
                       "import fails exactly as it fails on a machine that "
                       "never installed it. What must NOT happen is a quiet "
                       "return to the line-by-line reader, which is why there "
                       "is no `except ImportError` anywhere in that file: the "
                       "reader it replaced had four measured fail-open routes "
                       "and a fallback to it would be a floor gate that "
                       "silently checks less wherever a dependency is "
                       "missing. The control is the same file run WITHOUT "
                       "`-S`, which exits 0, so this probe discriminates "
                       "between the dependency being absent and the check "
                       "being broken. Its one assumption is that PyYAML lives "
                       "in site-packages rather than beside the standard "
                       "library, and if that is ever false here the probe "
                       "reports HARNESS rather than passing quietly."),
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
              "cannot evaluate `github.ref`, so a step behind a condition "
              "naming a branch counts on no event and the OK line prints the "
              "events it PROVED rather than the events that can happen. No "
              "step in `.github/workflows/ci.yml` sits behind such a "
              "condition today, so the limit currently drops nothing, and "
              "this sentence says so rather than naming a job. It named one "
              "until the S03 review's tenth pass, and the job had been "
              "deleted a pass earlier: it said what this file proves about "
              "`guards-deep` is that CI runs it on `workflow_dispatch`, and "
              "that the push-to-main half is unproven, while the check prints "
              "`pull_request, push, workflow_dispatch` for that gate because "
              "the step moved into the unconditional `guards` job. Both "
              "halves were false, in a declared limit, which this project "
              "treats as load-bearing. The third limit is the arm-command "
              "extractor's own "
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
              "watches `bool(arm)`. The fourth limit STATED THE WRONG "
              "CONSEQUENCE until the seventh pass. It said a statement whose "
              "first word is in `SHELL_NOISE` is not the work, so an arm that "
              "did its work inside an `if` or a `for` \"would be read as "
              "having none\", and that is true only when the WHOLE arm is "
              "inside the `if`. The measured shape is narrower and worse: "
              "`if node --test ...; then true; fi` is one statement whose head "
              "is a keyword, so dropping the head dropped the command and left "
              "every OTHER command in the arm visible, which is a gate that "
              "reads as fully covered with one command gone. The head is "
              "dropped and the remainder RE-SCANNED now, `SHELL_INTRODUCERS` "
              "and `SHELL_NOISE` are the two halves of that split, and "
              "`ci-floor.work-inside-an-if` watches it. What was left as a "
              "limit was the split itself, that a builtin's remainder is "
              "treated as arguments, so a command hidden after `command` or "
              "`eval` in a form this file does not model would still be "
              "invisible. `command` was HALF of that named pair and it was "
              "live: MEASURED in the eighth pass, rewriting the `bench` arm's "
              "`node --test` line as `command node --test ...` gave `unseen "
              "bench: None` and exit 0 with the gate-name step expanded, five "
              "node suites out of CI. It has a reader now, "
              "`command_builtin_runs`, which treats `-v` and `-V` as a lookup, "
              "steps over `-p` and `--`, re-scans what is left and fails "
              "CLOSED on an option it does not model. "
              "`ci-floor.work-behind-the-command-builtin` and "
              "`ci-floor.lookup-with-the-command-builtin` watch both "
              "directions. What remains of the split is any OTHER head whose "
              "remainder is really a command, and neither `eval` nor "
              "`command` is one of them any more. The fifth limit is the arm "
              "parser's own fail-opens, and the eighth pass called them two "
              "when two was not the count. FIVE are closed and probed now. "
              "`NESTED_CASE` carried a `\\b` in front of an alternation "
              "containing `!`, which needs a word character before it, so "
              "`&& ! case` matched nothing. `SHELL_COMMENT` ran AFTER `ARM` "
              "had matched, so a `;;` inside a shell comment ended the arm "
              "before the comment was stripped. The ninth pass measured three "
              "more, each dropping a real trailing command at exit 0 with "
              "`sh -n` accepting the file: a `;;` inside a QUOTED STRING, "
              "which no amount of stripping reaches, and `while case` and "
              "`if case`, which are statement positions `SHELL_INTRODUCERS` "
              "already knew about and `NESTED_CASE`'s hand-written "
              "`then|do|else|elif|!` did not. The alternation is DERIVED from "
              "`SHELL_INTRODUCERS` now, so the two cannot drift again. "
              "**The claim that the arm's end, the statement split and the "
              "comment strip all use one scan was half true until the S03 "
              "review's eleventh pass.** They shared the rule for OPENING a "
              "span and the delimiter set. The rule for CLOSING one was "
              "written out three times and agreed only because all three were "
              "edited in one commit, which is the condition that claim says "
              "had been removed. There is one tokenizer now, "
              "`ci_floor_check.shell_pieces`, with a span STACK, and its "
              "callers consume pieces rather than reimplementing a close "
              "loop. It covers the single quote, the double quote, the "
              "backtick, a nesting-counted `$( ... )` and `${ ... }`, a "
              "backslash escape, a here-document body and the comment itself, "
              "so a `#` that opens a comment and a quote that opens a span "
              "are decided by one rule rather than by an ordering between "
              "two passes. `ci_floor_check.gate_entries` reads the GATES "
              "array with the same scanner, which is what removed the second "
              "copy of the gate-row regex from `scripts/guards/census.py`. "
              "**The seventh limit is what the tokenizer still does not "
              "model, and it was written as an ENUMERATION of two constructs "
              "until the S03 review's twelfth pass, which measured the "
              "enumeration wrong.** It named `$'...'` and a substitution "
              "inside a double quote and said the residue was those two. It "
              "was not. A here-document body was claimed as covered and was "
              "covered for a DELIMITER SUBSET only, which was where the "
              "twelfth fail-open lived. Process substitution was not modelled "
              "and not mentioned, and a regex pre-pass over line "
              "continuations ran BEFORE the tokenizer and appeared nowhere. "
              "An enumeration of a grammar's constructs is not a limit, it is "
              "a claim that the author thought of all of them, and twelve "
              "passes say that shape does not hold. So the limit is stated as "
              "what the reader CANNOT SEE and why. **It was then stated as ONE "
              "thing, that the scanner does not model compound commands, and "
              "one thing was not the count either: the S03 review's thirteenth "
              "pass measured the fourteenth route in a production the sentence "
              "did not mention.** That claim is accurate about `shell_pieces` "
              "and `shell_pieces` is not the whole shell reader. It is ONE "
              "tokenizer with FOUR hand-written productions on top of it, "
              "`STATEMENT_BREAK`, `NESTED_CASE`, the "
              "`SHELL_INTRODUCERS`/`SHELL_NOISE` split and "
              "`ARM_LABEL`/`GATE_INVOCATION`, and the route lived in the "
              "first. \"One tokenizer\" was achieved and \"one reader\" was "
              "not, and the limit read as if it had been. Per production, "
              "then. THE TOKENIZER models spans and words and does not model "
              "bash's COMPOUND COMMANDS, so it cannot tell a `(` that opens a "
              "subshell from one that ends a `case` pattern, and it cannot "
              "tell `((` arithmetic from `( (` nested subshells the way bash "
              "does, which is by attempting the arithmetic parse and "
              "backtracking. `$(case y in *) ... esac)` therefore closes at "
              "the pattern's `)` and the arm ends at the inner `;;`, which "
              "`NESTED_CASE` refuses and "
              "`scripts/tests/test_guard_readers.py` asserts is the refusal "
              "carrying the weight rather than the scan. `(( a << b ))` reads "
              "the `<<` as a here-document whose body then runs to the end of "
              "the region, which refuses, and a here-document written inside "
              "a substitution is not queued at all, its body being inside the "
              "same span. `$'...'` is a span so its EXTENT is right and its C "
              "escapes are not decoded, which lands a name outside "
              "`GATE_NAME` and refuses, and a continuation INSIDE a double "
              "quote is left in place, which reaches `runs_command`'s text "
              "comparison and refuses. THE STATEMENT SCANNER, "
              "`STATEMENT_BREAK` and `_split_statements`, sees the control "
              "operators and nothing else about a list. MEASURED: `coproc case "
              "x in *) : ;; esac` is accepted by `bash -n`, is MISSED by "
              "`NESTED_CASE`, and fails closed only because the statement "
              "scanner reports `coproc case x in *` as a command CI does not "
              "run, so the refusal that carries the weight there is not the "
              "one the compound-command sentence names. "
              "`_tolerated_statements`, which decides whose failure `bash -e` "
              "discards, reads the same separators and inherits every one of "
              "these blind spots. `NESTED_CASE` derives its alternation from "
              "`SHELL_INTRODUCERS`, so `time case` matches and `coproc case` "
              "does not, measured both ways. The "
              "`SHELL_INTRODUCERS`/`SHELL_NOISE` split is a list rather than a "
              "grammar, and what remains of it is any head other than `eval` "
              "and `command`, each of which was measured on the wrong side and "
              "each of which has a reader now. `ARM_LABEL` and "
              "`GATE_INVOCATION` both anchor at a statement head, so both "
              "inherit the statement scanner's residue rather than adding one. "
              "Every consequence above is fail-closed and each is asserted "
              "where it is rather than assumed away. **Was bash asked "
              "instead, and it can "
              "be.** `declare -f run_gate` is bash's own reparse. It is not "
              "the reader, for three measured reasons: `set -n` does not "
              "define functions, so `declare -f` needs the audited file "
              "EXECUTED, and this catalogue plants adversarial shell into "
              "that very file inside a disposable clone. The output is a "
              "pretty-printed form with no stability contract, differing "
              "between the two bash versions on this machine, `cat <<'EOF'` "
              "and `esac` under 5.3.15 against `cat  <<'EOF'` and `esac;` "
              "under 3.2.57, and it normalises neither spelling the twelfth "
              "pass measured, `<<\\\\EOF` coming back as `<<'EOF'` and "
              "`<<'EOF-1'` unchanged. bash IS asked, in "
              "`scripts/tests/test_guard_readers.py`, where every span-table "
              "row and the delimiter production is run as a synthetic arm of "
              "`echo` markers and what bash PRINTS is compared with what the "
              "scanner attributes to the arm. That found the backtick's "
              "`opens` flag wrong: `x=`printf '%s' 'a`b'`` is an error to "
              "bash, so a quote does not hide a closing backtick, and the "
              "scanner had been accepting a file bash refuses. MEASURED over "
              "both regions the scanner is used on: after comments are "
              "stripped the `run_gate` region carries 0 `$'`, 0 `$(`, 0 "
              "`${`, 0 `<<` and 0 backticks against 48 before, and the GATES "
              "array carries 0 of all five. What is NOT probed is the "
              "unbalanced-quote refusal in `_arm_end`, and it is probed: "
              "`ci-floor.unbalanced-quote-in-an-arm` plants the unclosed "
              "quote in the LAST arm, which is the only position from which a "
              "later quote in the region cannot close it. That dependence on "
              "position is the sixth limit, declared here rather than hidden: "
              "an unclosed quote in an EARLIER arm is closed by the next "
              "quote in the region and read as a very long arm, which the "
              "per-command rule then refuses for a different reason, measured "
              "at exit 1 with a message naming the wrong thing. "
              "**The eighth limit is the workflow's own grammar, and it was "
              "not declared at all until the S03 review's twelfth pass, which "
              "measured four fail-open routes and five refusals of workflows "
              "GitHub Actions runs correctly.** `.github/workflows/ci.yml` is "
              "PARSED now, with `yaml.BaseLoader`, which is deviation D-17 "
              "and the only third-party import in the CI floor. What remains "
              "a limit is what a parse does not decide. The gate-name match "
              "is anchored at a STATEMENT HEAD, so a real invocation this "
              "file cannot see at a head is not counted: `sh -c "
              "'bin/ocelli.sh gate x'` and `FOO=1 bin/ocelli.sh gate x` are "
              "refusals rather than passes, which is fail-CLOSED and names "
              "the gate. `_split_statements` splits on the boolean operators "
              "without evaluating them, so `false && bin/ocelli.sh gate x` "
              "counts as an invocation and never runs, and that is a shape "
              "nobody has measured in this repository rather than one that "
              "has been shown safe. **That sentence used to cover `|| true` as "
              "well and it should not have, which the S03 review's "
              "thirteenth pass measured.** A step whose FAILURE is "
              "discarded does not run the gate in the sense `--floor` "
              "means. `continue-on-error` on the step, the same key on "
              "the job, and `|| true` on the run were three plants that "
              "each left this check at exit 0 on the gate that watches "
              "every other gate, and the first two were covered by "
              "nothing at all: the string occurred nowhere in "
              "`scripts/`, in `docs/lld/guards.md` or in this runbook. "
              "Both keys are read now and `_tolerated_statements` "
              "answers the third, MEASURED against `bash -e` rather "
              "than read out of the errexit paragraph, whose obvious "
              "reading is wrong: `false && true` followed by another "
              "line exits 0. **The SHELL the body runs under was the "
              "next limit and the S03 review's fourteenth pass "
              "measured it open.** It said no step in "
              "`.github/workflows/ci.yml` set one, which was true, and "
              "that a `shell:` which is not a shell would be read as "
              "bash, which named the wrong danger. `python` and `pwsh` "
              "fail closed on their own, neither being bash. The value "
              "that does not is a CUSTOM TEMPLATE: `shell: bash {0}` "
              "is still bash and has no `-e`, MEASURED with a file "
              "holding `false` then `echo AFTER` at exit 1 under `bash "
              "-e <file>` and 0 under `bash <file>`, so every statement "
              "in the body is swallowed and the check exited 0 with the "
              "key on the real `bin/ocelli.sh gate guards` step. The "
              "same template written once at workflow level under "
              "`defaults:` took every `run:` in the file with it, also "
              "at exit 0. All three levels are read now and the value "
              "is refused BY NAME unless it is in `MEASURED_SHELLS`, "
              "which is `bash` and `sh` and is closed against values "
              "GitHub adds later. `ci-floor.custom-shell-template-on-a-"
              "gate-step` and `ci-floor.custom-shell-template-for-the-"
              "workflow` watch the two levels a repair could close "
              "separately, and `ci-floor.measured-shell-on-a-gate-step` "
              "watches the direction a refusal by name gets wrong, "
              "`shell: bash` being `bash --noprofile --norc -eo "
              "pipefail {0}` and therefore STRICTER than the default. "
              "The same pass measured the other half of the errexit "
              "reading, which `_tolerated_statements` could not see "
              "because it reads the separators AROUND a statement and "
              "these are properties of the statement's position: a "
              "`set +e` earlier in the body, an `if`, `elif`, `while` "
              "or `until` CONDITION, and a `!` negation, each measured "
              "under `bash -ec` at 0 where the plain command exits 1, "
              "each leaving the check at 0 with `guards` reported "
              "covered. `_errexit_exempt` answers all three and "
              "`ci-floor.set-plus-e-before-a-gate-step`, "
              "`ci-floor.gate-inside-an-if-condition` and "
              "`ci-floor.negated-gate-step` watch them one defence "
              "each, against `ci-floor.errexit-restored-before-a-gate-"
              "step` and `ci-floor.gate-inside-an-if-body` for the "
              "shapes that really do run the gate. What remains a limit "
              "there is stated in `_errexit_exempt` itself and both "
              "halves are fail-CLOSED, so each costs a refusal naming "
              "the gate rather than a pass: a `set +e` inside a `( )` "
              "subshell or a function body is scoped to it and this "
              "scanner models neither, so the exemption runs to the end "
              "of the body or to the next `set -e`, and a `then`, "
              "`else` or `do` ends the exemption wherever it appears, "
              "so a body using one of those words as a plain argument "
              "ends it early. `set -o pipefail` inside a body is not "
              "read, and that direction is fail-CLOSED too: pipefail "
              "makes a failure this file already treats as discarded "
              "reach the step, so the file counts LESS coverage than "
              "exists rather than more. A "
              "`continue-on-error` whose value is a `${{ }}` "
              "expression is treated as tolerating, on `_permits`' own "
              "rule: a value this file cannot read as harmless is not "
              "read as harmless. `on` is read as the string key, so the "
              "YAML 1.1 boolean spelling `true:` reads as no event block at "
              "all, which refuses with the top-level keys named. And the "
              "shapes the twelfth pass deliberately did NOT claim, because it "
              "could not establish them against a real parse, are anchors, "
              "the merge key, multi-document files and U+2028. The first "
              "three would now be answered by PyYAML rather than by this "
              "file, and a multi-document workflow raises a `YAMLError`, "
              "which `ci-floor.workflow-unparseable` is the probe for. That "
              "is a consequence rather than a claim and it is written here "
              "as one.",
    ),

    # -- D-04's chain, the part CI reads ------------------------------------
    Guard(
        id="ledger.assert",
        file="scripts/verify_ledger.py",
        gate="-",
        spec="deviation D-04 mechanism 1, and HLD 27.2 R6",
        refuses="A staged tree with no recorded gate run, a tree whose "
                "recorded corpus is red, a corpus state outside the declared "
                "set, malformed or red comparison evidence, a green report "
                "with failed views, input problems, coverage problems, "
                "absorbed divergences or absent views, a report missing one "
                "of those emitted fields, an invalid coverage count, "
                "contradictory unmeasured counts, any object outside the "
                "serializer's closed schema, duplicate JSON keys or record "
                "identifiers, a summary or aggregate hash that disagrees "
                "with the records, canonically identical input directories, "
                "impossible green attribution states, statistics that "
                "contradict their histograms or tolerance verdicts, zero "
                "judged views, and a required comparison record that is "
                "absent.",
        claims=(r"no verification recorded", r"the corpus is RED",
                r"corpus is ' ' for tree", r"--corpus must be one of",
                r"comparison report", r"non-finite JSON number",
                r"has invalid keys",
                r"is not a unique non-empty string array",
                r"version is not the integer 1",
                r"hashAlgorithms values are not non-empty strings",
                r"greenReport is not an object",
                r"channelCountByClass values are not positive integers",
                r"greenUnmeasuredQualifiers contains an invalid value",
                r"is not a finite non-negative number",
                r"semantics.greenUnmeasuredStates is not an array",
                r"green unmeasured state",
                r"report contract is invalid",
                r"comparison evidence is required for tree"),
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
            Probe("ledger.comparison-malformed", _malformed_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report is not valid JSON"),
            Probe("ledger.comparison-identity", _identity_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "not produced by the explicit candidate gate"),
            Probe("ledger.comparison-red", _red_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report is not green"),
            Probe("ledger.comparison-zero", _zero_judgement_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report judged zero views"),
            Probe("ledger.comparison-failed-count",
                  _failed_count_in_green_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report is green but has failed views"),
            Probe("ledger.comparison-coverage-problem",
                  _coverage_problem_in_green_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                  ".claude/probe-comparison.json"),
                  "comparison report is green but has coverage problems"),
            Probe("ledger.comparison-input-problem",
                  _problem_in_green_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report is green but has problems"),
            Probe("ledger.comparison-absorbed-divergence",
                  _absorbed_divergence_in_green_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report is green but has absorbed divergences"),
            Probe("ledger.comparison-missing-coverage-problems",
                  _missing_coverage_problems_in_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report root has invalid keys, missing=['coverageProblems']"),
            Probe("ledger.comparison-top-level-absent",
                  _top_level_absent_in_green_comparison_report,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report has absent views"),
            *_coverage_count_probes(),
            *_top_level_unmeasured_probes(),
            Probe("ledger.comparison-unmeasured-contradiction",
                  _contradictory_unmeasured_counts,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "comparison report unmeasured count disagrees with coverage"),
            *_report_shape_probes(),
            *_report_semantic_probes(),
            *_report_contract_probes(),
            *_green_unmeasured_state_probes(),
            Probe("ledger.comparison-duplicate-top-level-key",
                  lambda box: _duplicate_report_key(box, '"fail": 0'),
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "duplicate JSON key 'fail'"),
            Probe("ledger.comparison-duplicate-nested-key",
                  lambda box: _duplicate_report_key(
                      box, '"id": "probe-view"'),
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "duplicate JSON key 'id'"),
            Probe("ledger.comparison-nonfinite-number",
                  _nonfinite_report_number,
                  script("python3", "scripts/verify_ledger.py", "record",
                         "--comparison-report",
                         ".claude/probe-comparison.json"),
                  "non-finite JSON number NaN"),
            Probe("ledger.require-comparison", _ledger_without_comparison,
                  script("python3", "scripts/verify_ledger.py", "assert",
                         "--require-comparison"),
                  "comparison evidence is required"),
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
                "that is not the commit's, a trailer recording a red or "
                "unrun corpus, and a required comparison field that is "
                "missing or malformed.",
        claims=(r"carries no trailer", r"trailer names tree",
                r"records a RED corpus", r"records corpus=",
                r"carries malformed comparison evidence",
                r"comparison evidence is required\."),
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
            Probe("ledger.commit-require-comparison", _commit_without_comparison,
                  script("python3", "scripts/verify_ledger.py",
                         "check-commit", "HEAD", "--require-comparison"),
                  "comparison evidence is required"),
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
        spec="`.claude/WORKFLOW.md`, the sprint roadmap is hand-curated after "
             "bootstrap",
        refuses="A sprint plan that disagrees with the backlog about which "
                "sprint a story is in or how large it is, a story planned "
                "into two sprint tables at once, a generated milestone "
                "summary or goal line that has drifted from the allocation it "
                "is written from or that is absent, duplicated or spurious, "
                "an absent plan, and a bare write that would replace an "
                "existing hand-curated plan.",
        claims=("*",),
        probes=(
            Probe("sprint-plan.existing-refuses-write",
                  _sprint_plan_hand_curated,
                  script("python3", "scripts/gen_sprint_plan.py"),
                  "refuses to overwrite",
                  control=script("python3", "scripts/gen_sprint_plan.py",
                                 "--force"),
                  note="F-X020. The rejected state differs from the control "
                       "only in authority: the bare command has none to "
                       "replace a file, while --force is explicit. The "
                       "marker paragraph cannot be reconstructed from "
                       "allocation.json, so a writer that runs destroys it."),
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
        id="skill-examples",
        file="scripts/skill_examples_check.py",
        gate="skills",
        spec="HLD 27.2 R2 and R3, HLD 27.3, and deviation D-13",
        refuses="A malformed or duplicate marked skill example, an empty "
                "example set, a nonzero or timed-out Python example, hidden "
                "stderr, unexpected assertion output, or stdout that differs "
                "from the declared result.",
        claims=("*",),
        probes=(
            Probe(
                "skill-examples.changed-expected-digit",
                _changed_skill_example_digit,
                script("python3", "scripts/skill_examples_check.py"),
                "stdout differs",
                note="HLD 27.3 requires changing one expected value and "
                     "watching the check fail. This changes the centre-row "
                     "digit beside the PS3.3-derived VOI example without "
                     "changing its calculation.",
            ),
            Probe(
                "skill-examples.reversed-sigmoid-exponent",
                _reverse_skill_sigmoid_exponent,
                script("python3", "scripts/skill_examples_check.py"),
                "stdout differs",
                note="PS3.3 C.11.2.1.3.1 fixes the exponent sign. The "
                     "selected input reduces the correct exponent to +1, so "
                     "reversing the formula changes the declared output.",
            ),
            Probe(
                "skill-examples.reversed-sigmoid-width-precondition",
                _reverse_skill_sigmoid_width_precondition,
                script("python3", "scripts/skill_examples_check.py"),
                "exited 1",
                note="PS3.3 C.11.2.1.3.1 requires positive width. Reversing "
                     "that predicate rejects the positive-width arithmetic "
                     "call before the zero-width refusal is reached.",
            ),
        ),
    ),
    Guard(
        id="handoff",
        file="scripts/sprint_workflow.py",
        gate="-",
        spec="`.claude/WORKFLOW.md` and `.claude/commands/complete-feature.md`",
        refuses="A handoff missing or duplicating a required field, carrying "
                "a malformed field value, or naming a branch that is not "
                "this story's.",
        claims=(r"handoff has no", r"branch does not start with",
                r"value must be", r"handoff has duplicate",
                r"handoff field is malformed",
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
                  note="This repository writes every path in backticks and "
                       "the validator accepts that house style by unwrapping "
                       "one matching code-span pair before applying the exact "
                       "story branch prefix."),
            Probe("handoff.missing-files-touched",
                  _handoff_without_files_touched,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "handoff has no **Files touched** field",
                  note="The completion command requires the file list as one "
                       "of six handoff fields. The validator must enforce the "
                       "same contract rather than a five-field subset."),
            Probe("handoff.multiple-code-spans", _handoff_multiple_code_spans,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "**Branch** value must be non-empty plain text or exactly "
                  "one Markdown code span"),
            Probe("handoff.unmatched-code-span", _handoff_unmatched_code_span,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "**Files touched** value must be non-empty plain text or "
                  "exactly one Markdown code span"),
            Probe("handoff.embedded-code-span", _handoff_embedded_code_span,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "**Head** value must be non-empty plain text or exactly one "
                  "Markdown code span"),
            Probe("handoff.empty-code-span", _handoff_empty_code_span,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "**Review** value must be non-empty plain text or exactly "
                  "one Markdown code span"),
            Probe("handoff.forged-suffix", _handoff_forged_suffix,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "**Base** value must be non-empty plain text or exactly one "
                  "Markdown code span"),
            Probe("handoff.duplicate-head", _handoff_duplicate_head,
                  Invoke("sprint_workflow validate-handoff",
                         _validate_handoff),
                  "handoff has duplicate **Head** fields"),
        ),
        limit="The other twenty-two refusals in this file belong to the "
              "sprint lifecycle commands, and the entry below owns them. "
              "These probes cover the branch grammar, malformed code spans, "
              "duplicate fields and one absent required field. They do not "
              "repeat the same shapes independently for all six fields. The "
              "six-field tuple is "
              "documented in `.claude/commands/complete-feature.md` and is "
              "also in the declared-constant ratchet, so adding, removing or "
              "renaming a field becomes a reviewed change.",
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
            Probe("sprint-lifecycle.close-legacy-state",
                  _close_legacy_state,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "no sprint-scope review recorded"),
            Probe("sprint-lifecycle.close-dirty-review",
                  _close_dirty_sprint_review,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "latest sprint review pass 2 reports 1 defects and 0 smells"),
            Probe("sprint-lifecycle.close-stale-review",
                  _close_stale_sprint_review,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "latest sprint review tree 000000000000 is stale"),
            Probe("sprint-lifecycle.close-stale-verification",
                  _close_stale_verification,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "latest sprint-profile verification tree 000000000000 is stale"),
            Probe("sprint-lifecycle.close-latest-verification-failed",
                  _close_failed_latest_verification,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "latest sprint-profile verification did not pass"),
            Probe("sprint-lifecycle.close-tree-changed",
                  _close_tree_changed_after_evidence,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "latest sprint review tree",
                  note="The evidence is recorded first, then a tracked file "
                       "is staged. This proves both records are identities of "
                       "one tree rather than durable booleans."),
            Probe("sprint-lifecycle.close-unrecorded-carry",
                  _close_unrecorded_carry,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "is carried but has no recorded carry-forward reason"),
            Probe("sprint-lifecycle.close-recorded-carry",
                  _close_recorded_carry,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "is ready to close", polarity="accept"),
            Probe("sprint-lifecycle.close-current-evidence",
                  None,
                  Invoke("sprint_workflow close-preflight",
                         _close_preflight),
                  "is ready to close", polarity="accept"),
        ),
        limit="The remaining lifecycle branches belong to init, feature "
              "state transitions and release notes. The close probes build "
              "ignored sprint state from allocation.json and use the "
              "sandbox's real staged tree, so legacy, missing, dirty, stale, "
              "failed, carried and current evidence are exercised without "
              "touching the developer's run state.",
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
        id="quirks",
        file="scripts/quirk_check.py",
        gate="quirks",
        spec="HLD sections 11 and 27.2, and deviation D-05",
        refuses="A field quirk without independent expectation provenance, "
                "a callable synthetic recipe, one matching generator-owned "
                "manifest row, active mutation evidence, or with a generated "
                "DICOM tracked in git, plus an unknown property at any "
                "declared schema object. The 26-case checker suite mutates "
                "every object vocabulary and watches the added refusal.",
        claims=("*",),
        probes=(
            Probe("quirks.missing-authority", _quirk_without_authority,
                  script("python3", "scripts/quirk_check.py"),
                  "missing expectation authority"),
            Probe("quirks.ocelli-derived-expectation",
                  _quirk_with_ocelli_as_authority,
                  script("python3", "scripts/quirk_check.py"),
                  "expectation authority kind 'ocelli-output' is not independent"),
            Probe("quirks.absent-manifest-row",
                  _quirk_with_absent_manifest_path,
                  script("python3", "scripts/quirk_check.py"),
                  "needs exactly one manifest row"),
            Probe("quirks.missing-mutation-evidence",
                  _quirk_without_mutation_evidence,
                  script("python3", "scripts/quirk_check.py"),
                  "mutation evidence must be a non-empty array"),
            Probe("quirks.missing-voi-function-mutation",
                  _quirk_without_mutation("generator-voi-function"),
                  script("python3", "scripts/quirk_check.py"),
                  "required mutation generator-voi-function is missing"),
            Probe("quirks.missing-window-width-mutation",
                  _quirk_without_mutation("generator-window-width"),
                  script("python3", "scripts/quirk_check.py"),
                  "required mutation generator-window-width is missing"),
            Probe("quirks.missing-attribution-mutation",
                  _quirk_without_mutation("disable-reference-attribution"),
                  script("python3", "scripts/quirk_check.py"),
                  "required mutation disable-reference-attribution is missing"),
            Probe("quirks.tracked-generated-dicom",
                  _quirk_with_tracked_generated_dicom,
                  script("python3", "scripts/quirk_check.py"),
                  "generated path is a tracked DICOM"),
        ),
        covered_by=("scripts/tests/test_quirk_check.py (26 cases, run by the "
                    "`quirks` gate)",),
    ),
    Guard(
        id="quirk-mutation-boundaries",
        file="scripts/quirk_mutation_boundaries.py",
        gate="quirk-mutations",
        spec="F-014's approved design and HLD section 27.2 R2",
        refuses="A generated SIGMOID quirk case whose function or width no "
                "longer matches the independently declared boundary.",
        claims=("*",),
        covered_by=("scripts/tests/test_quirk_mutations.py opens and checks "
                    "the named boundary, and `bin/ocelli.sh gate "
                    "quirk-mutations` executes it green then under both "
                    "checker-owned generator mutations",),
    ),
    Guard(
        id="quirk-mutations",
        file="scripts/quirk_mutations.py",
        gate="quirk-mutations",
        spec="F-014's approved design and HLD section 27.2 R6",
        refuses="Mutation evidence whose healthy control is red, whose fixed "
                "edit stays green, whose failure has the wrong signature, or "
                "whose registry row, kind, signature or replacement differs "
                "from the executable contract, including a fixture symbol "
                "with no live mutation target or a regression row that differs "
                "from the executable attribution mutation.",
        claims=("*",),
        covered_by=("scripts/tests/test_quirk_mutations.py (9 cases, run by "
                    "the `quirks` gate), plus the three live mutations run by "
                    "the `quirk-mutations` gate",),
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
        limit="The per-target divergence branch needs a dependency whose "
              "features differ by target. The locked graph contains no such "
              "fixture, and the disposable sandbox has no network authority "
              "to fetch a new dependency graph, so that branch has no probe.",
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
        limit="The consumer install, node import, two tsc resolutions and "
              "publish dry run need an npm install. `node_modules` is ignored "
              "and therefore absent from the `git ls-files` sandbox, so these "
              "level-3 refusals have no catalogue probe. The `packages` gate "
              "runs them against the real install on every push.",
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
        limit="The redaction-map fail-closed branch sits behind pandoc "
              "conversion of the private source `.docx`. The document and "
              "conversion input are absent from the tracked repository, so a "
              "`git ls-files` sandbox cannot reach that branch or the related "
              "drift and section-mapping refusals.",
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
                  "is 'allow' and HLD 27.1 requires",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.uninherited", _lint_policy_uninherited,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table",
                  needs="cargo", profile="deep"),
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
                       "that reaches patients.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.expect-attribute",
                  _expect_attribute_at_a_crate_root,
                  script("python3", "scripts/lint_policy_check.py"),
                  "re-allows",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.manifest-not-utf8",
                  _a_manifest_that_is_not_utf8,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cannot be read as UTF-8 text",
                  note="The read was unguarded, so a `Cargo.toml` that is not "
                       "UTF-8 arrived as a `UnicodeDecodeError` traceback "
                       "with no FAIL header. Fail-closed, and still the wrong "
                       "way to tell a maintainer what to do, which is the "
                       "S03 review's fifth-pass finding in the CI floor check "
                       "one file over. The byte planted is a lone 0x80, a "
                       "continuation byte with no lead byte, which cargo "
                       "refuses as well."),
            Probe("lint-policy.nothing-scanned", _no_crate_sources_at_all,
                  script("python3", "scripts/lint_policy_check.py"),
                  "not one `.rs` file was read",
                  needs="cargo", profile="deep"),
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
                       "inner attribute in a module file governs that module.",
                  needs="cargo", profile="deep"),
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
                       "a `mod`.",
                  needs="cargo", profile="deep"),
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
                       "case was never exercised in either direction.",
                  needs="cargo", profile="deep"),
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
                       "exits 0.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.member-outside-crates-uninherited",
                  _member_outside_crates_uninherited,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table",
                  note="The workspace has fourteen members and the check read "
                       "thirteen: `crates/` was hard-coded and Cargo.toml "
                       "says `members = [\"crates/*\", \"tools/oracle\"]`. "
                       "The member is read from the manifest here rather than "
                       "named, because a literal would be the same assumption "
                       "that cost the workspace its fourteenth member.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.member-outside-crates-group-allow",
                  _member_outside_crates_group_allow,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The other half of the same hole. With both this and "
                       "the uninherited probe applied, the check exited 0 "
                       "printing `13 crate(s) inherit the table, 33 .rs "
                       "file(s)`, and the census exited 0 beside it. "
                       "`tools/oracle` is a compiled member with thirteen "
                       "`.rs` files.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.member-unresolvable",
                  _unresolvable_workspace_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "resolves to no directory carrying a Cargo.toml",
                  note="cargo refuses this workspace. A check that walks the "
                       "members it can resolve and reports the smaller number "
                       "as a pass is AGENTS.md's named failure of answering a "
                       "question about a smaller set in the language of "
                       "success.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.no-members-declared",
                  _no_workspace_members_at_all,
                  script("python3", "scripts/lint_policy_check.py"),
                  "declares no `members` this parser can read",
                  note="The empty-set case for the member walk, which is the "
                       "same shape as `lint-policy.nothing-scanned` one level "
                       "up. With no member the inheritance pass, the "
                       "attribute pass and the file count all answer a "
                       "question about nothing.",
                  needs="cargo", profile="deep"),
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
                       "applied last.",
                  needs="cargo", profile="deep"),
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
                       "census at 0 and `gate guards` ALL GREEN.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.quoted-group-row",
                  _quoted_group_row,
                  script("python3", "scripts/lint_policy_check.py"),
                  "carries the lint GROUP",
                  note="The fifth spelling of one row in five passes, and the "
                       "first to defeat the row regex and its declared "
                       "backstop together. MEASURED under 1.97.1 on a minimal "
                       "workspace: baseline 101, and `\"pedantic\" = { level "
                       "= \"allow\", priority = 1 }` as the LAST line of the "
                       "table gives cargo 0, the guard 0 printing \"no group "
                       "row weaker than deny\" and the census 0. In the "
                       "MIDDLE of the table the ratchet held, digest "
                       "`034c52d0054007ea` against `adf2cb2237be28da`, so the "
                       "position is what this probe is about. Both halves "
                       "read the table with `tomllib` now.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.two-line-group-row",
                  _two_line_group_row,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cannot be parsed as TOML",
                  note="A DECLARED residual until the eleventh pass. TOML 1.0 "
                       "puts an inline table on one line and cargo accepts it "
                       "over two, MEASURED at cargo clippy 101 to 0 with the "
                       "guard at exit 0. `tomllib` rejects it, and a document "
                       "cargo accepts and this parser rejects is refused "
                       "rather than read as an empty table, which is the "
                       "fail-closed direction the regexes had backwards.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unparseable-member-manifest",
                  _a_member_manifest_that_is_not_toml,
                  script("python3", "scripts/lint_policy_check.py"),
                  "is a workspace member cargo reports and its Cargo.toml "
                  "cannot be parsed as TOML",
                  note="The fail-closed half of reading the MEMBER manifests "
                       "with `tomllib`. The regex pair this replaced answered "
                       "\"does not inherit\" for a document nobody could "
                       "parse, which is a refusal naming the wrong thing, and "
                       "one rule weaker in the same shape is silence. The run "
                       "is red for a second reason too, because cargo cannot "
                       "read the workspace either, so the expected fragment "
                       "is the member sentence rather than the exit code.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.quoted-required-row-is-permitted",
                  _quoted_required_row,
                  script("python3", "scripts/lint_policy_check.py"),
                  "clippy lint(s) at or above HLD 27.1's level",
                  polarity="accept",
                  note="The accept half of the quoted key. MEASURED under "
                       "1.97.1: `\"cast_possible_truncation\" = \"deny\"` "
                       "leaves cargo clippy at 101, so the lint is denied and "
                       "the guard has to say so. A repair that refused the "
                       "quoting rather than reading through it would pass the "
                       "group probe above and refuse a legitimate manifest.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.dotted-required-row-is-permitted",
                  _dotted_required_row,
                  script("python3", "scripts/lint_policy_check.py"),
                  "clippy lint(s) at or above HLD 27.1's level",
                  polarity="accept",
                  note="The other legitimate spelling, and what says the "
                       "guard did not simply learn one more alternation. "
                       "MEASURED under 1.97.1: "
                       "`cast_possible_truncation.level = \"deny\"` leaves "
                       "cargo clippy at 101. A parser reading TOML sees the "
                       "same row in the bare, quoted, dotted and inline "
                       "forms, and each of those was a separate finding in a "
                       "separate pass while the reader was a regex.",
                  needs="cargo", profile="deep"),
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
                       "nothing and the guard has to say so.",
                  needs="cargo", profile="deep"),
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
                       "happens to pick `tools/oracle`.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.comment-in-the-lint-path",
                  _comment_in_the_lint_path,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The sixth route past this guard and the second "
                       "lexer rule it did not know. `_names_in` normalised "
                       "whitespace and nothing else, and Rust strips comments "
                       "before it sees a token, so `clippy::/*x*/pedantic` is "
                       "the same token stream as `clippy::pedantic`. MEASURED "
                       "with cargo on a minimal crate under the pinned "
                       "1.97.1: baseline exit 101, "
                       "`#![allow(clippy::/*c*/pedantic)]` exit 0, "
                       "`#![allow(clippy::/*c*/cast_possible_truncation)]` "
                       "exit 0, and a newline-and-`//`-comment form exit 0. "
                       "Planted in a sandbox clone of this repository the "
                       "whole `guards` gate was green.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unreadable-lint-argument",
                  _unreadable_lint_argument,
                  script("python3", "scripts/lint_policy_check.py"),
                  "could not read to its end",
                  note="The half of the same route that stripping comments "
                       "does not close. `INNER_ALLOW` captures up to the "
                       "first `)` and a block comment can hold one, so what "
                       "reaches the name split is the fragment `clippy::/*`. "
                       "MEASURED under 1.97.1: "
                       "`#![allow(clippy::/*)*/pedantic)]` exits 0 on a crate "
                       "denying cast_possible_truncation. The guard fails "
                       "CLOSED on a list it did not read to its end rather "
                       "than reporting a fragment as an unrecognised lint.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.rustflags-allow",
                  _rustflags_allow_a_denied_lint,
                  script("python3", "scripts/lint_policy_check.py"),
                  "in `rustflags`",
                  note="`.cargo/config.toml` is read by cargo and was read by "
                       "nothing here: no file under scripts/, bin/, ci/, "
                       ".githooks/ or .github/ mentioned `rustflags`. "
                       "MEASURED on the minimal crate: "
                       "`[build] rustflags = [\"-Aclippy::pedantic\"]` exit "
                       "0, `[\"-Aclippy::cast_possible_truncation\"]` exit 0, "
                       "the `[target.'cfg(all())']` form exit 0 and the "
                       "bare-string form exit 0. Added to an ocelli sandbox, "
                       "where there is no `.cargo/` today, every check stayed "
                       "green.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cargo-config-is-permitted",
                  _a_cargo_config_that_lowers_nothing,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo config(s) lower no denied lint through rustflags",
                  polarity="accept",
                  note="The direction that says which decision was made. The "
                       "guard refuses the FLAG and not the file. A cargo "
                       "config is the ordinary home for an alias, a linker "
                       "choice, a target runner and `[net]` settings, and a "
                       "guard refusing its existence would be refusing a "
                       "legitimate state and pushing a real need into a "
                       "workaround nothing watches. `-D warnings` here RAISES "
                       "a level and must be permitted.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.path-dependency-member",
                  _a_path_dependency_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table",
                  note="`[workspace] members` is not the member set. cargo "
                       "makes every path dependency of a member a member too, "
                       "and `workspace_members` read only the globs. "
                       "MEASURED: adding `vendor/probe` and "
                       "`probe-vendored = { path = \"../../vendor/probe\" }` "
                       "to a crate manifest made `cargo metadata --no-deps` "
                       "report 15 packages while the guard printed \"14 "
                       "workspace member(s)\" at exit 0 and the census exited "
                       "0. That is pass 5's `tools/oracle` defect through a "
                       "different key, which is why the set comes from cargo "
                       "now rather than from a harder read of the globs.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.module-outside-the-member",
                  _a_module_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="`member_sources` globbed `member.rglob(\"*.rs\")`, so "
                       "a `#[path]` module whose source is outside the member "
                       "directory was never opened. MEASURED under 1.97.1: a "
                       "crate whose lib.rs reads `#[path = "
                       "\"../../shared_outside/shared.rs\"] pub mod shared;` "
                       "exits 101, and with `#![allow(clippy::pedantic)]` at "
                       "the top of that file it exits 0 while the guard never "
                       "reads it.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.clean-module-outside-the-member",
                  _a_clean_module_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "1 `#[path]` module(s) followed",
                  polarity="accept",
                  note="The other direction. A module source outside the "
                       "member directory is legal Rust and says nothing about "
                       "lint levels, so following `#[path]` must not turn the "
                       "layout itself into a refusal. The expect fragment "
                       "carries the COUNT since the S03 review's tenth pass. "
                       "The OK line named the four keys in a fixed string, so "
                       "this probe passed on a sentence that would have "
                       "printed identically had the file been skipped, and "
                       "the count is derived from the sources actually read.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cfg-attr-module-path",
                  _a_cfg_attr_module_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="Key three with a spelling the code did not read, and "
                       "the file knew it elsewhere: `INNER_ALLOW` "
                       "deliberately matches an `allow` reached through "
                       "`cfg_attr` and says so, while `MODULE_PATH` required "
                       "`path` to be the first thing inside the attribute. "
                       "MEASURED under the pinned 1.97.1 toolchain on a "
                       "minimal workspace: the plain form refused at guard "
                       "exit 1, this form at guard exit 0, and both take "
                       "cargo clippy from 101 to 0. Planted in a full clone "
                       "of this repository the guard printed its usual "
                       "\"46 .rs file(s)\" line at exit 0 having read neither "
                       "file.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.raw-string-module-path",
                  _a_raw_string_module_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The second spelling, and the file knew this one too: "
                       "`INCLUDE_PATH` below `MODULE_PATH` already "
                       "carries `(?:r#*)?`, because a raw string literal is "
                       "the same file name written another way. MEASURED the "
                       "same way, on the minimal workspace and again in a "
                       "full clone: guard exit 0 with cargo clippy taken from "
                       "101 to 0.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.clean-cfg-attr-module-path",
                  _a_clean_cfg_attr_module_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "1 `#[path]` module(s) followed",
                  polarity="accept",
                  note="The direction widening `MODULE_PATH` could get wrong. "
                       "`cfg_attr` is legal Rust and says nothing about lint "
                       "levels, and the guard REFUSES a `#[path]` it cannot "
                       "resolve, so an over-tight read of the wrapper would "
                       "refuse a crate that moves a module behind a feature. "
                       "The count in the expect is what proves the file was "
                       "read: 46 .rs files becomes 47, measured.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.required-row-with-a-tail",
                  _a_required_row_with_a_tail,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cannot be parsed as TOML",
                  note="A required row with rubbish after its value, which is "
                       "not TOML at all. This probe watched `LINT_ROW`'s tail "
                       "anchor until the eleventh pass: MEASURED in the "
                       "seventh, deleting `_row_body` and loosening the tail "
                       "to `.*$` left both accept twins green while "
                       "`cast_possible_truncation = \"deny\" and we mean it` "
                       "read as a `deny` row and the guard said nothing. "
                       "There is no row regex to loosen now, so what it "
                       "watches is the direction that replaced it: a document "
                       "`tomllib` cannot parse is REFUSED, where the regexes "
                       "returned an empty table and the check went on to "
                       "report five absent lints. The refusal is stronger "
                       "than the one this probe used to expect, which is why "
                       "the expected fragment moved.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.crate-root-outside-the-member",
                  _crate_root_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The eighth route past this guard and the THIRD key to "
                       "pass 5's `tools/oracle` defect. Pass 7 followed "
                       "`#[path]` from a crate root it assumed was under "
                       "`src/`, and a manifest's `[lib] path`, `[[bin]] path` "
                       "or `[[test]] path` puts that root anywhere. MEASURED "
                       "under the pinned 1.97.1 toolchain on a minimal crate "
                       "carrying `cast_possible_truncation = \"deny\"`: with "
                       "`[lib] path = \"../../outside/lib.rs\"` and that file "
                       "holding `#![allow(clippy::pedantic)]` and one "
                       "`x as i32`, cargo clippy exits 0 against a baseline of "
                       "101, and in a full copy of this repository the guard "
                       "exited 0 printing \"46 .rs file(s), `#[path]` modules "
                       "followed\", having scanned the now-unused "
                       "`crates/ocelli-core/src/lib.rs`. `cargo metadata` "
                       "already returns `targets[].src_path` for every target "
                       "of every member, so the walk is seeded from cargo's "
                       "own answer rather than from a fourth guess.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.clean-crate-root-outside-the-member",
                  _clean_crate_root_outside_the_member,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo target root(s) seeded",
                  polarity="accept",
                  note="The other direction. A `[lib] path` outside the "
                       "member is legal cargo and says nothing about lint "
                       "levels, so seeding the walk from `targets[].src_path` "
                       "must not turn the LAYOUT into a refusal. The member's "
                       "own src/lib.rs is left in place and unread by cargo, "
                       "which is the state that would make a guard refusing "
                       "the shape rather than the attribute look correct. "
                       "The accept status is the assertion: this legal layout "
                       "must not be refused. The paired "
                       "`crate-root-outside-the-member` probe puts a group "
                       "allow in the same moved root and proves it is scanned. "
                       "The output fragment confirms the production path, but "
                       "does not couple this property to the workspace-wide "
                       "target count, which changes when unrelated targets "
                       "land.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unreadable-crate-root",
                  _crate_root_the_guard_cannot_open,
                  script("python3", "scripts/lint_policy_check.py"),
                  "as a compilation root of the workspace member",
                  note="cargo metadata answers happily on a `[lib] path` that "
                       "names no file, MEASURED under the pinned 1.97.1 "
                       "toolchain, so a seeded root the guard cannot open is "
                       "a state the guard actually reaches. Refused rather "
                       "than skipped, on the same argument as an unresolvable "
                       "`#[path]`: it names a file clippy compiles and this "
                       "pass did not read, which is exactly the state the "
                       "group-allow measurement above was taken in.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cap-lints-allow",
                  _cap_lints_allow,
                  script("python3", "scripts/lint_policy_check.py"),
                  "caps EVERY lint",
                  note="The eighth pass's other route into the same guard. "
                       "`rustflag_problems` refused `-A`, `--allow`, `-W` and "
                       "`--warn`, the flags that NAME a lint, and "
                       "`--cap-lints` names none and caps all of them. It was "
                       "in neither the refused set nor the declared "
                       "out-of-scope list, which named only "
                       "`$CARGO_HOME/config.toml`, `RUSTFLAGS` and "
                       "`--config`, so that declaration was not exhaustive. "
                       "MEASURED under the pinned 1.97.1 toolchain with the "
                       "`clippy` gate's own -D warnings passed: the two-token "
                       "form, the `=` form and the bare-string form all take "
                       "cargo from 101 to 0, and planted in a full copy of "
                       "this repository the guard exited 0 printing \"1 cargo "
                       "config(s) lower no denied lint through rustflags\", "
                       "which asserts the false thing positively.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cap-lints-warn",
                  _cap_lints_warn,
                  script("python3", "scripts/lint_policy_check.py"),
                  "caps EVERY lint",
                  note="The second of the two weakening levels, and there are "
                       "only four levels, so this pair covers the whole space "
                       "and `CAP_LINTS_KEEPING` needs no place in the "
                       "declared-constant ratchet. MEASURED under 1.97.1: "
                       "`warn` takes cargo clippy from 101 to 0 as `allow` "
                       "does, because the cap is applied AFTER the "
                       "-D warnings the `clippy` gate passes rather than "
                       "before it, which is the reading that makes `warn` "
                       "look harmless.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cap-lints-deny-is-permitted",
                  _cap_lints_deny,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo config(s) lower no denied lint through rustflags",
                  polarity="accept",
                  note="Which decision was made, again. The guard refuses a "
                       "`--cap-lints` LEVEL that weakens and not the flag, on "
                       "the same argument that made it refuse the flag and "
                       "not the file. MEASURED under 1.97.1: `deny` leaves "
                       "cargo clippy at 101 and so does `forbid`, so refusing "
                       "either would be refusing a legitimate state.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.force-warn-a-denied-lint",
                  _force_warn_a_denied_lint,
                  script("python3", "scripts/lint_policy_check.py"),
                  "in `rustflags`",
                  note="The flag that was expected to RAISE and was measured "
                       "to weaken. `--force-warn` was put down beside `-D` "
                       "and `-F` in the eighth pass's own instruction and "
                       "cargo says otherwise: MEASURED under the pinned "
                       "1.97.1 toolchain with -D warnings passed exactly as "
                       "the `clippy` gate passes it, "
                       "`[\"--force-warn\", \"clippy::cast_possible_truncation\"]` "
                       "exits 0, the `=` form exits 0 and the group form "
                       "exits 0, while `[\"-Dwarnings\"]` and "
                       "`[\"-Fclippy::cast_possible_truncation\"]` both stay "
                       "at 101. It forces the level to warn and OUTRANKS "
                       "-D warnings. The lint is read from HLD 27.1's copy in "
                       "Cargo.toml rather than named here.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.deny-a-group-is-permitted",
                  _deny_a_group_in_rustflags,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo config(s) lower no denied lint through rustflags",
                  polarity="accept",
                  note="The acceptance direction of the tenth pass's message "
                       "split, and the reason the refusal was not narrowed "
                       "instead. `clippy::pedantic` defaults to allow, so a "
                       "project may legitimately turn it on from a cargo "
                       "config, and `-D` is the flag that does it: MEASURED "
                       "under the pinned 1.97.1 toolchain, "
                       "`[\"-Dclippy::pedantic\"]` exits 101 with the "
                       "`clippy` gate's own -D warnings and 101 without it. "
                       "`-W` on the same group is a DIFFERENT measurement and "
                       "stays refused: without -D warnings it takes the same "
                       "crate from 101 to 0 and demotes the "
                       "cast_possible_truncation error to a warning, because "
                       "a group flag outranks the manifest for every member "
                       "lint. This probe is what stops the over-tight repair, "
                       "refusing any rustflag naming a group, and the "
                       "over-loose one, dropping groups from "
                       "`ALLOWING_FLAGS`'s reach, from both reading as "
                       "correct.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.empty-rustflags-is-permitted",
                  _rustflags_that_are_empty,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo config(s) lower no denied lint through rustflags",
                  polarity="accept",
                  note="`_flag_values` returned `tokens or None` and this "
                       "guard's `None` means \"a value this parser cannot "
                       "read\", so `rustflags = []`, which is what a config "
                       "holds when the last flag is removed, was refused as "
                       "unread arguments reaching rustc. An empty ANSWER is "
                       "not no answer. An unclosed `[` and a value holding "
                       "something that is not a string literal are still "
                       "`None`, because in those two there really are "
                       "arguments this parser did not read.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.dotted-lints-inheritance-is-permitted",
                  _dotted_lints_inheritance,
                  script("python3", "scripts/lint_policy_check.py"),
                  "workspace member(s) from",
                  polarity="accept",
                  note="A guard refusing a legitimate state. TOML's dotted "
                       "spelling `lints.workspace = true` above `[package]` "
                       "is the same table as `[lints]` with `workspace = "
                       "true` under it, and the guard's regex read one of "
                       "them. MEASURED under the pinned 1.97.1 toolchain on a "
                       "minimal crate carrying `cast_possible_truncation = "
                       "\"deny\"` and one `x as i32`: with that line above "
                       "`[package]` cargo clippy exits 101, so the table IS "
                       "inherited, while the guard refused at exit 1. Written "
                       "ABOVE the first table header deliberately: under "
                       "`[package]` the same line is `package.lints`, which "
                       "cargo reports as an unused manifest key and does not "
                       "inherit, measured at exit 0, and refusing that "
                       "spelling is right.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.include-macro",
                  _an_include_macro_hiding_a_group_allow,
                  script("python3", "scripts/lint_policy_check.py"),
                  "allows the lint group",
                  note="The FOURTH key to the set of files this guard reads, "
                       "after the member glob, `targets[].src_path` and "
                       "`#[path]`, and the ninth consecutive route past the "
                       "guard. `include!` pastes a file's tokens in, so that "
                       "file is compiled and is named by none of the other "
                       "three. MEASURED under the pinned 1.97.1 toolchain: an "
                       "`include!`d file carrying `#[allow(clippy::pedantic)]` "
                       "on a `#[path]` module takes cargo clippy from 101 to 0 "
                       "in a minimal crate AND in a full copy of this "
                       "repository, where the guard exited 0 printing its "
                       "usual \"46 .rs file(s)\" line having read neither "
                       "file. The attribute is the outer form on a `mod` "
                       "because rustc REJECTS an inner `#![allow(...)]` at the "
                       "top of an included file, measured, and the two have "
                       "the same scope, which is the fifth pass's finding.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.clean-include-macro",
                  _a_clean_include_macro,
                  script("python3", "scripts/lint_policy_check.py"),
                  "1 `include!`(s) followed",
                  polarity="accept",
                  note="The other direction. `include!` is legal Rust and says "
                       "nothing about lint levels, so following it must not "
                       "turn the construct into a refusal. Without this the "
                       "obvious repair, refusing every `include!`, would pass "
                       "the probe above and refuse a crate that generates "
                       "code, which is the runbook's sentence about a guard "
                       "that fails on everything.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unresolvable-include-macro",
                  _an_include_macro_naming_no_file,
                  script("python3", "scripts/lint_policy_check.py"),
                  "did not read the source it pastes in",
                  note="The same argument as an unresolvable `#[path]`: a file "
                       "that is compiled and was not read, which is the state "
                       "every measurement in the guard's header was taken in. "
                       "rustc resolves the argument against the directory of "
                       "the file the macro is written in, which is NOT the "
                       "rule `#[path]` uses, so the resolution is written out "
                       "rather than shared.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.computed-include-macro",
                  _an_include_macro_the_guard_cannot_resolve,
                  script("python3", "scripts/lint_policy_check.py"),
                  "could not read a file name out of it",
                  note="`include!(concat!(env!(\"OUT_DIR\"), \"/x.rs\"))` is "
                       "what a build script writes, and only the build knows "
                       "where that is. The guard refuses rather than passing "
                       "over an `include!` whose file name it could not read, "
                       "because generated code carrying a group allow is the "
                       "same hole through a path nobody typed.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unreadable-member-source",
                  _a_member_source_that_is_not_utf8,
                  script("python3", "scripts/lint_policy_check.py"),
                  "is reached by this check's walk of the workspace member",
                  note="`member_sources` caught `(OSError, "
                       "UnicodeDecodeError)` and CONTINUED with the path still "
                       "in its result, and `main` then read the same file "
                       "again with no guard at all, so this state was a raw "
                       "traceback at exit 1 rather than a refusal under the "
                       "FAIL header. That is the presentation "
                       "`scripts/ci_floor_check.py` stopped giving in the "
                       "fifth pass and the eighth pass restated four lines "
                       "earlier in this same guard. One read, one answer. "
                       "This probe discriminates on the MESSAGE and not on the "
                       "exit status, and that is worth saying: the unfixed "
                       "guard also exits 1 here, by tracebacking, so only the "
                       "`expect` fragment tells the refusal from the crash.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.dotted-rustflags-key",
                  _dotted_rustflags_key,
                  script("python3", "scripts/lint_policy_check.py"),
                  "in `rustflags`",
                  note="`RUSTFLAG_KEY`'s `^` anchor required the key to start "
                       "its line, and TOML's dotted spelling never does. "
                       "MEASURED under the pinned 1.97.1 toolchain: "
                       "`build.rustflags = "
                       "[\"-Aclippy::cast_possible_truncation\"]` takes cargo "
                       "clippy from 101 to 0 and the group form does too, "
                       "while `_flag_values` returned `[]` for both. Planted "
                       "in a full copy of this repository the guard printed "
                       "\"1 cargo config(s) lower no denied lint through "
                       "rustflags\" at exit 0, which is the eighth pass's "
                       "`--cap-lints` outcome exactly. The config is parsed "
                       "with `tomllib` now, which retires the whole "
                       "regex-against-TOML class from that function.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.quoted-rustflags-key",
                  _quoted_rustflags_key,
                  script("python3", "scripts/lint_policy_check.py"),
                  "in `rustflags`",
                  note="TOML's third spelling of the same key and the second "
                       "the anchor could not see. MEASURED under the pinned "
                       "1.97.1 toolchain: `[build]` with `\"rustflags\" = "
                       "[\"-Aclippy::pedantic\"]` takes cargo clippy from 101 "
                       "to 0. A parsed document has ONE key here where the "
                       "regex had three spellings, which is why the repair was "
                       "`tomllib` rather than a fourth alternative.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.cargo-config-unparseable",
                  _a_cargo_config_that_is_not_toml,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not parse as TOML",
                  note="cargo refuses a config it cannot parse, so this is not "
                       "a working state, and a parse failure read as a file "
                       "declaring no rustflags answers a question about an "
                       "empty set in the language of success. The regex this "
                       "replaced had no notion of a document at all, so a "
                       "malformed config was silently a config with no "
                       "`rustflags` in it. It discriminates on the MESSAGE "
                       "rather than on the exit status, and the reason is "
                       "worth recording: a malformed config at the repository "
                       "root also makes `cargo metadata` fail, which this "
                       "guard already refuses on, so the unfixed guard exits 1 "
                       "here for a different reason and only the `expect` "
                       "fragment tells the two apart. A malformed config in a "
                       "SUBDIRECTORY is not read by a root `cargo metadata` at "
                       "all, and there the new refusal is the only one.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.dotted-cargo-config-is-permitted",
                  _a_dotted_cargo_config_that_lowers_nothing,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cargo config(s) lower no denied lint through rustflags",
                  polarity="accept",
                  note="The direction the `tomllib` repair could get wrong. A "
                       "dotted key is ordinary TOML and a cargo config is the "
                       "ordinary home for a job count, an alias and a linker "
                       "choice, so reading the document must not turn the "
                       "SPELLING into a refusal. `-Dwarnings` raises a level "
                       "and must be permitted here exactly as it is in the "
                       "bracketed form.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.runner-not-utf8",
                  _a_runner_that_is_not_utf8,
                  script("python3", "scripts/lint_policy_check.py"),
                  "Whether the `unsafe` gate still runs",
                  note="The guard reads `bin/ocelli.sh` to establish the "
                       "declared substitution for HLD 27.1's `unsafe_code` "
                       "deny, and a runner it cannot decode leaves that "
                       "question UNKNOWN. An unknown is not an enforcement, so "
                       "it refuses under the FAIL header rather than arriving "
                       "as a `UnicodeDecodeError` traceback, which is the "
                       "presentation the fifth pass took away from "
                       "`scripts/ci_floor_check.py` and the eighth pass from "
                       "this file's `Cargo.toml` read.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.runner-arms-unreadable",
                  _a_runner_with_no_run_gate_region,
                  script("python3", "scripts/lint_policy_check.py"),
                  "cannot be read for its gate arms",
                  note="The other half of reading the arm through "
                       "`ci_floor_check.gate_commands`: that reader REFUSES a "
                       "runner it cannot delimit, and this guard must not turn "
                       "the refusal into the answer no. A restructured runner "
                       "is a person's decision and the message says so.",
                  needs="cargo", profile="deep"),
            Probe("lint-policy.unsafe-gate-named-only-in-a-comment",
                  _the_unsafe_gate_named_only_in_a_comment,
                  script("python3", "scripts/lint_policy_check.py"),
                  "neither mechanism is present",
                  note="**One line of regex producing a positive assertion of "
                       "a false thing.** The declared substitution for HLD "
                       "27.1's `unsafe_code` deny is that the `unsafe` gate "
                       "runs `scripts/unsafe_allowlist_check.py`, and this "
                       "guard read the arm with "
                       "`^\\\\s*unsafe\\\\)[^\\\\n]*?python3 "
                       "scripts/unsafe_allowlist_check\\\\.py`, where "
                       "`[^\\\\n]*?` reaches a `#` as happily as a command. "
                       "MEASURED with the arm rewritten to run another gate's "
                       "command and the real one moved into a trailing "
                       "comment, and the CI step deleted: `bash -n` 0, this "
                       "guard exit 0 PRINTING \"unsafe_code denied by "
                       "scripts/unsafe_allowlist_check.py in the `unsafe` "
                       "gate, which is the declared substitution\", "
                       "`scripts/ci_floor_check.py` exit 0, the census exit 0 "
                       "and both unit suites exit 0. The script then ran "
                       "nowhere and HLD 27.2 R5 was enforced by nothing. The "
                       "repair already existed two files over: "
                       "`_ci_arm_commands` above calls "
                       "`ci_floor_check.gate_commands`, which is "
                       "comment-stripped and span-aware by construction, and "
                       "this guard is its third caller now. That is the "
                       "propagation failure again, the answer present in the "
                       "repository and not reaching the caller.",
                  needs="cargo", profile="deep"),
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
              "message says rather than overclaiming. **The second limit "
              "was a TOML row this guard's regex could not read, and it is "
              "not a limit any more.** It was declared twice and both "
              "declarations claimed a division of labour with the declared "
              "constant `Cargo.toml:workspace.lints`: a two-line inline table "
              "for the sixth pass, a dotted key for the ninth, each measured "
              "past `LINT_ROW` at cargo clippy 101 to 0 and each said to be "
              "caught on the digest instead. **That sentence was false of the "
              "third spelling and the eleventh pass measured it.** A QUOTED "
              "key defeated both mechanisms at once: `\"pedantic\" = { level "
              "= \"allow\", priority = 1 }` as the LAST line of the table "
              "gave cargo 0, the guard 0 printing \"no group row weaker than "
              "deny\" and the census 0, because the constant's capture ended "
              "at the last line beginning with a bare key. In the middle of "
              "the table the ratchet held, `034c52d0054007ea` against "
              "`adf2cb2237be28da`, so the boundary was exact. Both halves "
              "read the tables with `tomllib` now and the constant records "
              "the guard's own parse, so every spelling is one code path and "
              "the two mechanisms cannot agree with each other while "
              "disagreeing with the grammar. What is refused rather than "
              "read is a document `tomllib` cannot parse, which is the "
              "two-line row, and `lint-policy.two-line-group-row` watches it. "
              "The "
              "third limit arrived with the seventh pass's fix and is the "
              "PROFILE: this guard reads its member set from `cargo metadata "
              "--no-deps`, so every probe here declares `needs=\"cargo\"` and "
              "sits in the deep profile rather than in the floor, which is "
              "the same rule `nostd` already lives under for `cargo tree`. "
              "The guard itself still runs on every pull request in the "
              "`guards` gate, and since the ninth pass so does the deep "
              "harness, because `gate guards-deep` is a STEP in the "
              "unconditional `guards` job rather than the push-to-main job it "
              "used to be. This sentence said the harness watching the guard "
              "had moved to push-to-main, and it was still saying it a pass "
              "after that job was deleted, which the S03 review's tenth pass "
              "found. What the profile costs is a second run of the floor "
              "set, and what it buys is a member set cargo computes rather "
              "than one this file guesses at, after two passes in which the "
              "guess was wrong through a different key each time. The eighth "
              "pass found a THIRD key to that same defect and it is now "
              "closed rather than declared: a manifest's `[lib] path`, "
              "`[[bin]] path` or `[[test]] path` puts the crate root outside "
              "the member, measured at exit 0 with a group allow in it, so "
              "the walk is seeded from `cargo metadata`'s own "
              "`targets[].src_path` and the rglob is kept beside it for the "
              "modules no target names. **The NINTH pass found the fourth key "
              "and stopped calling the result a set the guard knows.** "
              "`include!` pastes a file's tokens in and is named by none of "
              "the other three, measured at cargo clippy 101 to 0 with the "
              "guard at exit 0, and it is followed and refused now like "
              "`#[path]` is. What that leaves is the honest statement of the "
              "class rather than another closed route: the source list is a "
              "RECONSTRUCTION from four keys, every pass since the fifth has "
              "found a new one, and `member_sources`'s docstring, this guard's "
              "OK line and `docs/lld/guards.md` all say so rather than "
              "claiming \"every `.rs` file a workspace member compiles\", "
              "which is what the first two claimed through four passes in "
              "which it was false. rustc's own dep-info at "
              "`target/<profile>/deps/*.d` IS the set, and it was considered "
              "as the authority and rejected with four measurements: it exists "
              "only after a build, a stale one narrows in silence, freshness "
              "by mtime would refuse after every keystroke, and making it "
              "fresh means this guard runs `cargo check --workspace "
              "--all-targets` at 10.8s in a clone with no `target/` and again "
              "inside every one of these probes, whose sandbox is a fresh copy "
              "of `git ls-files`. The fourth "
              "limit is what a rustflags scan cannot reach: cargo also reads "
              "`$CARGO_HOME/config.toml`, the `RUSTFLAGS` environment "
              "variable and `--config` on the command line, none of which is "
              "in this repository. That list was stated as if it were "
              "exhaustive and was not: `--cap-lints` is in the repository's "
              "own `.cargo/config.toml` space, names no lint so nothing in "
              "the flag scan saw it, and is MEASURED to take cargo clippy "
              "from 101 to 0 at `allow` and at `warn`. It has its own branch "
              "and two probes now, which is the whole space of weakening "
              "levels rather than a sample of it, so `CAP_LINTS_KEEPING` "
              "needs no place in the ratchet where `ALLOWING_FLAGS` does. "
              "`ALLOWING_FLAGS` joined the ratchet in the same pass, for the "
              "reason `REFUSED_GROUPS` is in it: narrowing it to `{\"-A\": 1}` "
              "was measured to leave both its probes, the census and the "
              "guard at exit 0 with `--allow`, `-W` and `--warn` unguarded. "
              "The fifth limit is ONE refusal here with no probe, declared "
              "rather than left to be found: a workspace member whose "
              "Cargo.toml cannot be read. The eighth pass replaced a bare "
              "FileNotFoundError traceback with a refusal under the FAIL "
              "header, which is the presentation `scripts/ci_floor_check.py` "
              "stopped giving in the fifth pass, and the branch cannot be "
              "driven from a sandbox: cargo cannot report a member whose "
              "manifest it could not read either, so the state is reachable "
              "only through the glob fallback and only by a filesystem "
              "permission that `Sandbox.reset` cannot restore. A probe that "
              "left a sandbox unrecoverable would cost more than the branch "
              "is worth.",
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
            Probe("census.impossible-wall-clock-pair",
                  _an_impossible_wall_clock_pair,
                  script("python3", "scripts/guard_census.py"),
                  "cannot be the faster of the two",
                  note="The one recorded key in ci/guard-probe-budget.json "
                       "that nothing read. `selected()` returns EVERY probe "
                       "for the deep profile and the floor's are a subset, so "
                       "deep runs a strict superset of the floor's work and "
                       "cannot be faster. FOUND in the S03 review's eighth "
                       "pass as `deep: 17.2` beside `floor: 18.1`: the "
                       "seventh pass re-recorded floor while moving seventeen "
                       "probes into deep and left deep at its pre-move value, "
                       "so the `5x + 30` ceiling for the profile that now "
                       "carries every `lint-policy` probe was set from a run "
                       "that did not contain them. The gate was never red, "
                       "because a ceiling that is too generous never is. "
                       "Measured over four runs on the recording machine at "
                       "floor 19.6s to 20.6s and deep 27.4s to 28.9s. The "
                       "superset relation is derived from the "
                       "catalogue rather than asserted, so the check says "
                       "nothing if the profile rule is ever rewritten."),
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
            Probe("census.no-std-set-shrunk", _shrink_expected_no_std_set,
                  script("python3", "scripts/guard_census.py"),
                  "changed without its recorded value",
                  note="The explicit expected no_std set is a reviewed "
                       "posture value. Narrowing that value must move its "
                       "recorded digest even though the direct guard agrees "
                       "with the new set."),
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
            Probe("census.gate-name-with-a-digit",
                  _a_gate_named_outside_the_python_class,
                  script("python3", "scripts/guard_census.py"),
                  "has no catalogue entry and no `delegated` reason",
                  note="The same route as `ci-floor.gate-name-with-a-digit` "
                       "seen from the census, and both are needed because the "
                       "two checks run in different gates and a gate the "
                       "reader dropped was invisible to both. This module "
                       "carried its own copy of the row regex and its "
                       "docstring said `bin/ocelli.sh` carried a third, which "
                       "was the imprecision that hid the defect: the runner "
                       "carries no regex at all. It calls "
                       "`ci_floor_check.declared_gates` now."),
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
            Probe("probe-runner.a-guard-that-refuses-nothing",
                  _a_guard_that_refuses_nothing,
                  HARNESS_OVER_ONE_PROBE,
                  "did not fire, so it is a guard nobody has watched fail",
                  polarity="accept",
                  control_status=1,
                  control_expect="refusal probe(s) drove",
                  note="THE inverted-success rule, and it was watched by "
                       "nothing while it was the sentence that gives every "
                       "other result in this harness its meaning. The input "
                       "is this file's own docstring, not the runner's "
                       "source: \"A probe whose guard exits 0 is a FAILURE OF "
                       "THE HARNESS, not a pass.\" So the state is a guard "
                       "that exits 0 on every input, built by returning from "
                       "`main` before the first check rather than by breaking "
                       "the file, because a check rewritten until it detects "
                       "nothing exits 0 and does not crash. MEASURED in the "
                       "S03 review's seventh pass: with `problems.append` at "
                       "the refuse branch deleted and `\"pass\"` returned "
                       "instead, the census exited 0 at 566 refusals, "
                       "`--self-test` exited 0 with 10 properties, "
                       "`--profile floor` exited 0 with 106 probes red, the "
                       "unit suite passed 49, and `entry_sites` for this "
                       "entry stayed at 19. The whole `guards` gate was ALL "
                       "GREEN. Declared `accept` over an INVERTED status, and "
                       "that is not a flourish: the obvious `refuse` form was "
                       "measured green under the same defect, because a probe "
                       "cannot report a refusal through the refusal it is "
                       "watching. The inversion routes this probe's own "
                       "failure through the other polarity's branch. The "
                       "control is the DIFFERENT status a healthy repository "
                       "gives, 1 rather than 0, for the same reason "
                       "split_hld's is."),
        ),
        limit="This entry's other refusals are watched by "
              "`probe-runner.self-test`, and one class of mutation to this "
              "catalogue reaches the harness as `error` rather than as "
              "`HARNESS`. A probe builder that resolves its own target "
              "through the guard it aims at, which "
              "`_expand_a_gate_step_into_its_visible_commands` does through "
              "`ci_floor_check.unseen_commands`, raises when that function is "
              "broken, and `run_probe` reports a builder failure as `error`. "
              "Both are red and both fail the gate, so nothing is lost, and a "
              "reader who expects `HARNESS` and sees `error` is reading the "
              "builder's refusal rather than the harness's. The builder above "
              "avoids the same trap by editing `main`'s first statement "
              "rather than the top of the guard's file, so a builder that "
              "imports the guard still runs.",
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
        limit="The self-test drives both new F-013 truth-projection refusals: "
              "a malformed source binding raises, and unequal source scopes "
              "remain unequal before the main checker reports them.",
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
        refuses="An argument the harness does not accept, and a runner for a "
                "subject whose story is not done.",
        claims=("*",),
        covered_by=("tools/bench/tests/run_test.mjs (run by the `bench` gate)",),
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
        limit="The test that reaches a measurement against a stub, an "
              "incomplete artefact copy and a page that never reports launches "
              "Chromium. It is opted into with OCELLI_BENCH_BROWSER=1 and "
              "cannot run in the floor or a disposable sandbox that has no "
              "browser install. The developer browser suite watches it.",
    ),
    Guard(
        id="bench.page",
        file="tools/bench/page/app.mjs",
        gate="bench",
        spec="HLD section 26",
        refuses="A page serving an incomplete copy of the wasm artefact, and "
                "a mark count that does not match the phase list.",
        claims=("*",),
        covered_by=("tools/bench/tests/cold_start_test.mjs",),
        limit="The incomplete-artefact refusal executes in the benchmark page "
              "and requires the Chromium path, which the floor and disposable "
              "sandbox do not carry. The mark-count refusal does not share "
              "that limit and is watched in the floor by the phase-table unit "
              "test.",
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
        limit="The stub refusal fires only when the wasm module fails to "
              "export what the probe imports. Constructing that state needs a "
              "broken wasm-pack build and its generated artefact, neither of "
              "which exists in the tracked disposable sandbox.",
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
    """One value that decides how strict a guard is.

    `pattern` is a regex over the file's text and is how almost every entry
    below is read, because almost every entry below records a Python literal
    out of a Python file, which is a grammar a regex can pin down between two
    anchors it owns.

    `read` is the alternative, and it exists because one entry is not that.
    A value in a FOREIGN grammar has to be read by that grammar's parser or the
    ratchet is a second regex disagreeing with the guard's first one, which is
    the eleventh review pass's finding: the recorded slice of
    `[workspace.lints.clippy]` and `scripts/lint_policy_check.py`'s row regex
    were two hand-rolled TOML readers that were kept in step with each other
    and neither of which was in step with TOML. A quoted key was outside both.
    When `read` is set it takes the file's text and returns the recorded value,
    or `None` when the value cannot be read at all, which the census refuses.
    Exactly one of `pattern` and `read` is set.
    """

    guard: str
    file: str
    name: str
    pattern: str = ""
    tunable: bool = False
    why: str = ""
    read: Callable[[str], str | None] | None = None


def _workspace_lints_rows(text: str) -> str | None:
    """`Cargo.toml`'s two lints tables, through the GUARD'S OWN parser.

    Imported lazily for the reason `_ci_arm_commands` gives: every caller of
    this catalogue puts `scripts/` on `sys.path`, and a top-level import would
    make the catalogue depend on one particular guard.

    Calling the guard's parser rather than writing a second one is the whole
    repair. The ratchet's job is to notice the table being WIDENED, and it can
    only do that over the rows cargo reads. Reading them a second way was how a
    quoted key ended up outside the recorded value while the guard was blind to
    it as well, so cargo exit 0, guard exit 0 and census exit 0 all held at once
    with four of HLD 27.1's five lints switched off.

    This does not make the ratchet redundant. `lint_policy_check.py` refuses
    the rows it knows are wrong, HLD 27.1's five at too low a level and the nine
    named groups. The ratchet refuses ANY change to either table, including a
    row for a lint no rule here has heard of, and including a `priority` moved
    under a level left alone.
    """
    import lint_policy_check
    return lint_policy_check.workspace_lints_rows(text)


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
    Constant("pins", "scripts/pin_and_size_check.py", "PACKAGE_LICENCES",
             r"^PACKAGE_LICENCES = (\(.*?\))$",
             why="Both grants required by `MIT OR Apache-2.0`. Narrowing the "
                 "tuple would make a package with only one grant pass."),
    Constant("pins", "scripts/pin_and_size_check.py", "TOLERANCE",
             r"^TOLERANCE = (.*?)$", tunable=True,
             why="Growth tolerated before the size gate fails. A story that "
                 "grows the module 5% should say why, and raising this is "
                 "one of the things it might say."),
    Constant("ci-floor", "scripts/ci_floor_check.py", "NOT_IN_FLOOR",
             r"^NOT_IN_FLOOR = (\{.*?\})$",
             why="Deviation D-04. A gate leaving the floor is a decision."),
    # The ONE declared exception to `runs_command`'s argument-vector equality,
    # and pass 7 closed the substring match, declared exactly one exception
    # and left nothing watching it. MEASURED in the S03 review's eighth pass:
    # adding a second entry for `prose_check.py` with
    # `--only-this-one-file README.md`, plus the matching CI step, left
    # `ci_floor_check.py`, the census, the unit suite and the floor probe
    # profile all at exit 0, which is byte for byte the outcome pass 7
    # measured and fixed. Same shape as `REFUSED_GROUPS` and
    # `OWNED_ACCESSORS`: a probe can only ever exercise the one entry that is
    # already declared, so it cannot see a second one arrive.
    #
    # The whole body is captured, the reason strings included. Each entry's
    # reason is the justification for a step running MORE than its gate's arm,
    # and rewording one has to force a re-read of what the exception is for,
    # which is the same trade `scripts/guards/discover.py` records for a
    # refusal's identity.
    Constant("ci-floor", "scripts/ci_floor_check.py", "PERMITTED_ADDITIONS",
             r"^PERMITTED_ADDITIONS[^=\n]*= \{(.*?)^\}",
             why="Every entry here is a CI step permitted to run more than "
                 "the gate arm it stands for. One is declared today, "
                 "`--require-prerequisites` on corpus_tests.py, and it is "
                 "strictly stronger than the arm. A second entry is how an "
                 "arbitrary argument becomes legal, and it lands in a diff "
                 "nobody reads unless this moves."),
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
    # `validate-handoff`'s required fields. The completion command documents
    # the same six-field template, and recording the tuple makes a change to
    # that contract land in a diff even if a branch probe still passes.
    Constant("handoff", "scripts/sprint_workflow.py", "HANDOFF_FIELDS",
             r'^HANDOFF_FIELDS = (\(.*?^\))',
             why="The documented six-field integration handoff contract. A "
                 "field added, removed or renamed here changes what every "
                 "parallel worker must write."),
    # THE WHOLE TABLE, PARSED, and this entry stopped being a regex in the
    # S03 review's ELEVENTH pass. What it recorded before was a text slice of
    # `Cargo.toml` captured by a regex that ran from the
    # `[workspace.lints.clippy]` header to the last line beginning with a bare
    # key. Six passes tuned that capture. Their measurements are kept here
    # because they are the argument for not doing it a seventh time:
    #
    #   pass 6   the capture stopped at the first blank line, so a row after
    #            one was outside it and the digest did not move
    #   pass 9   the backtrack was unanchored, matched `f=` inside an awk
    #            snippet in a COMMENT, and ran twelve lines past the table:
    #            digest `319e61ab2cda36f4` for a prose edit with the table
    #            byte identical, which trains the next author to re-record on
    #            sight
    #   pass 10  the backtrack was anchored to the start of a line, `.` was
    #            put in the key class for the dotted row, and both halves of
    #            the mechanism were made to agree with each other
    #   pass 11  `"pedantic" = { level = "allow", priority = 1 }` appended as
    #            the LAST line of the table. `"` is not in `[\w.-]`, so the
    #            capture ended at the line above it and the row was outside
    #            the recorded value. MEASURED on a minimal workspace under the
    #            pinned 1.97.1 toolchain: cargo clippy 101 to 0, guard exit 0
    #            printing "no group row weaker than deny", census exit 0.
    #
    # Pass 10 made the two hand-rolled TOML readers agree. It did not make
    # either agree with TOML, and pass 11 walked through the gap between them.
    # So this records `lint_policy_check.workspace_lints_rows`, which is the
    # guard's own `tomllib` parse of both tables rendered as sorted
    # `table.name = <json>` rows. Every spelling of a row is the same value
    # here because it is the same value to cargo, the whole ROW is recorded
    # rather than its level so a moved `priority` moves the digest, and the
    # sort means a reordered table records the same digest, which it should:
    # order carries no meaning in TOML and `priority` is where precedence
    # lives.
    #
    # A `Cargo.toml` that does not parse records NOTHING, and the census
    # refuses a constant it cannot read. That is the third mechanism failing
    # closed on the same input as the guard, rather than recording a digest
    # over an empty table.
    Constant("lint-policy", "Cargo.toml", "workspace.lints",
             read=_workspace_lints_rows,
             why="HLD 27.1's denied lint table and every other row of both "
                 "`[workspace.lints.*]` tables, parsed with `tomllib` by the "
                 "guard's own reader. A row added, removed, renamed, "
                 "re-levelled or re-prioritised in either table moves this "
                 "digest, in any TOML spelling, because the spelling is gone "
                 "by the time the value is rendered. Six passes tuned the "
                 "regex this replaced and the seventh input walked past it."),
    # The group names that exist only in this guard. HLD 27.1 names five
    # lints and no groups, so `lint-policy.group-allow` has to write one of
    # the nine and narrowing the nine to that one would leave the probe green
    # with eight groups unguarded. Same shape as OWNED_ACCESSORS above, same
    # answer. The recorded value is the NAMES and not the explanations beside
    # them, so rewording a message does not move the digest.
    # The same shape one field over, and it was not recorded until the S03
    # review's eighth pass. `lint-policy.rustflags-allow` writes
    # `-Aclippy::pedantic`, so it can only ever exercise one of the names, and
    # MEASURED: narrowing the set to `{"-A": 1}` leaves that probe, its accept
    # twin, the census and the guard itself all at exit 0 with `--allow`, `-W`
    # and `--warn` unguarded. `REFUSED_GROUPS` sits four lines below and is
    # recorded for exactly this reason.
    Constant("lint-policy", "scripts/lint_policy_check.py", "ALLOWING_FLAGS",
             r"^ALLOWING_FLAGS = (\{.*?\})\n\n",
             why="The rustflags that lower a lint's level. Five names, and a "
                 "probe can only ever write one of them. `--force-warn` is "
                 "among them and was expected to raise: measured under the "
                 "pinned 1.97.1 toolchain it takes cargo clippy from 101 to 0 "
                 "on a crate denying cast_possible_truncation, because it "
                 "outranks the -D warnings the `clippy` gate passes."),
    Constant("lint-policy", "scripts/lint_policy_check.py", "REFUSED_GROUPS",
             r"^REFUSED_GROUPS = \{(.*?)^\}",
             why="A group allow disables every lint in the group without "
                 "naming one of them. Measured: `#![allow(clippy::pedantic)]` "
                 "takes cargo clippy from 101 to 0 on a crate that denies "
                 "cast_possible_truncation. Dropping a name here is the "
                 "widening no probe can see."),
    # F-X017 replaced two spelling-based view selectors with a type-aware
    # rule. Record both the constructor set and the whole detection path so a
    # narrowed type test, lost computed-property route, or removed constructor
    # lands as a strictness change. The executable node suite proves the
    # positive and negative semantics in the `lint` gate.
    Constant("lint-policy", "eslint.config.js", "TYPED_ARRAY_CONSTRUCTORS",
             r"^const TYPED_ARRAY_CONSTRUCTORS = new Set\(\[(.*?)^\]\);"),
    Constant("lint-policy", "eslint.config.js", "WASM_VIEW_DETECTION",
             r"^function memberName\(node\) \{(.*?)^const ocelliPlugin = \{\n"
             r"  rules: \{ \"no-wasm-memory-view\": noWasmMemoryViewRule \},\n"
             r"\};"),
    Constant("lint-policy", "eslint.config.js", "WASM_VIEW_SELF_CHECK",
             r"^function assertTheBanIsIntact\(config\) \{(.*?)^\}\n\n"
             r"^const config = tseslint\.config\("),
    # The syntax half remains for a buffer alias, whose ArrayBuffer type no
    # longer carries its wasm-memory provenance.
    Constant("lint-policy", "eslint.config.js", "NO_CACHED_WASM_MEMORY_ALIAS",
             r"^const NO_CACHED_WASM_MEMORY_ALIAS = \{\n  selector:\n"
             r"(.*?)\n  message"),
    Constant("lint-policy", "eslint.config.js", "LINEAR_MEMORY_ALLOWANCE",
             r'files: (\["packages/core/src/bulk\.ts".*?\])',
             why="HLD 17.2 says two functions. F-005 widened this from one "
                 "file to two, and a third is not granted."),
    Constant("lint-policy", "bin/ocelli.sh", "WASM_VIEW_TEST_REGISTRATION",
             r"^    lint\)(.*?) ;;$",
             why="The type-aware semantic probes need ignored node_modules "
                 "and therefore run in the lint gate rather than a disposable "
                 "catalogue sandbox. Removing their node command must move a "
                 "recorded strictness value."),
    Constant("nostd", "scripts/no_std_check.py", "EXPECTED_NO_STD_CRATES",
             r"^EXPECTED_NO_STD_CRATES = (frozenset\(\{.*?^\}\))",
             why="Deviation D-09's explicit crate set. Direct comparison "
                 "catches a source attribute entering or leaving it, and this "
                 "digest makes a deliberate posture change visible."),
)


DEFECTS = {}
