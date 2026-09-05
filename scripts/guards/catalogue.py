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


def _floor_gate_with_own_step(box: Sandbox) -> str:
    """A floor gate whose CI step is a `bin/ocelli.sh gate <name>` line.

    Read from `.github/workflows/ci.yml`, which is the artefact the guard is
    about. The gates to skip come from `NOT_IN_FLOOR`, which is the
    repository's own declaration of what the floor excludes and is itself in
    the declared-constant ratchet. It was an undocumented literal here, and a
    literal that matches neither `NOT_IN_FLOOR` nor anything in the workflow
    is a probe input nobody can check.
    """
    workflow = box.read(".github/workflows/ci.yml")
    declared = re.search(r"^NOT_IN_FLOOR = \{(.*?)\}",
                         box.read("scripts/ci_floor_check.py"), re.M | re.S)
    if declared is None:
        raise AssertionError(
            "scripts/ci_floor_check.py declares no NOT_IN_FLOOR, so this "
            "probe cannot tell a floor gate from one CI runs elsewhere.")
    excluded = set(re.findall(r'"([a-z-]+)"', declared.group(1)))
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


def _sprint_plan_disagreement(box: Sandbox) -> None:
    box.delete("docs/sprints/SPRINT_PLAN.md")


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


def _lint_policy_weakened(box: Sandbox) -> None:
    box.substitute("Cargo.toml", 'cast_possible_truncation = "deny"',
                   'cast_possible_truncation = "allow"')


def _lint_policy_uninherited(box: Sandbox) -> None:
    box.substitute("crates/ocelli-pixel/Cargo.toml",
                   "[lints]\nworkspace = true", "")


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
        refuses="A range where the specification requires an exact `=` pin, a "
                "pinned crate that has left the workspace table, and a wasm "
                "module over its recorded ceiling.",
        claims=("*",),
        probes=(
            Probe("pins.range", _relax_wgpu_pin,
                  script("python3", "scripts/pin_and_size_check.py"),
                  "is a RANGE, not an exact pin"),
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
                "actually run.",
        claims=("*",),
        probes=(
            Probe("ci-floor.missing-step",
                  lambda box: _delete_ci_step(box, leave_comment=False),
                  script("python3", "scripts/ci_floor_check.py"),
                  "and nothing in"),
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
        covered_by=("crates/ocelli-compute/tests/ui/ (trybuild, `test` gate)",),
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
                "sprint a story is in, and an absent plan.",
        claims=("*",),
        probes=(
            Probe("sprint-plan.absent", _sprint_plan_disagreement,
                  script("python3", "scripts/gen_sprint_plan.py", "--check"),
                  "does not exist"),
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
              "the locked graph without a network fetch. Owner F-X010.",
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
              "not watched them red. Owner F-X010.",
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
              "section-mapping branches. Owner F-X010.",
    ),

    # -- the new guard this story ships ------------------------------------
    Guard(
        id="lint-policy",
        file="scripts/lint_policy_check.py",
        gate="guards",
        spec="HLD 27.1, the denied lint table",
        refuses="A lint in HLD 27.1's table weakened below the level the "
                "specification sets, and a crate that stops inheriting the "
                "workspace lint table.",
        claims=("*",),
        probes=(
            Probe("lint-policy.weakened", _lint_policy_weakened,
                  script("python3", "scripts/lint_policy_check.py"),
                  "is 'allow' and HLD 27.1 requires"),
            Probe("lint-policy.uninherited", _lint_policy_uninherited,
                  script("python3", "scripts/lint_policy_check.py"),
                  "does not inherit the workspace lint table"),
        ),
    ),

    # -- this story's own machinery, watched by the same runner ------------
    Guard(
        id="census",
        file="scripts/guards/census.py",
        gate="guards guards-deep",
        spec="`.claude/plans/F-X009-design.md` section 7, completeness proved "
             "mechanically",
        refuses="A refusal site no catalogue entry claims, an entry no site "
                "backs, a floor entry needing a GPU, a browser or the corpus, "
                "a declared constant changed without its recorded value, and "
                "an uncovered count that has grown.",
        claims=("*",),
        probes=(
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
        owner="F-X010",
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
        limit="That suite needs a browser and is deliberately outside the "
              "`bench` gate, so these refusals are watched only when a "
              "developer runs the harness. Owner F-X010.",
    ),
    Guard(
        id="bench.page",
        file="tools/bench/page/app.mjs",
        gate="bench",
        spec="HLD section 26",
        refuses="A page serving an incomplete copy of the wasm artefact.",
        claims=("*",),
        covered_by=("tools/bench/tests/cold_start_test.mjs",),
        limit="Same browser dependency as bench.cold-start. Owner F-X010.",
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
              "F-X010.",
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
               "refusals did not matter. Owner F-X010.",
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
    Constant("lint-policy", "Cargo.toml", "workspace.lints",
             r"^\[workspace\.lints\.clippy\]\n(.*?)\n\n",
             why="HLD 27.1's denied lint table, verbatim."),
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
