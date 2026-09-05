# Runbook, seeing every guard go red

## Why this exists

**Every guard is seen red before it is claimed.** Remove or revert the thing a
guard protects, one at a time, and watch it fail. A guard that has never been
observed failing is a guard nobody has tested. That was written as a standing
expectation of S01 in `docs/sprints/CURRENT_SPRINT.md`, against a count of
guards that is long out of date, and `15f7450` removed the paragraph when the
sprint moved on. It is restated here, without the count, because this document
is where it lives now. `bin/ocelli.sh gate --list` is the current inventory.

The failure mode is specific and it is not hypothetical. A guard that exits
zero on a clean tree looks identical to a guard whose detection is broken, and
the two stay indistinguishable until the day the guard was supposed to catch
something. This procedure separates them.

**A green run of this repository's gates is not evidence that the gates work.**
It is evidence that nothing was wrong, or that nothing was checked, and only
this procedure tells those apart.

## When to run it

- After adding a guard, before claiming it protects anything.
- After changing a guard's detection logic, including a refactor that "cannot
  change behaviour".
- After a Python or git upgrade that could change how a script reads the index.
- When onboarding a machine, together with `docs/DEVELOPER_SETUP.md`.

## Before you start, enable the hooks

Two of the guards below are hooks, and a hook does nothing until the clone opts
in. This is per clone and it is not tracked, so a fresh clone has the hooks
present and inert.

```bash
git config core.hooksPath .githooks
git config core.hooksPath            # must print .githooks
```

`core.hooksPath` lives in the shared repository config, so setting it once
covers every worktree of that clone, including the worker worktrees a parallel
sprint wave creates.

## Do it in a throwaway worktree

Every probe below deliberately breaks something. Do not do that in a worktree
that holds work.

```bash
git worktree add --detach /tmp/guardtest sprint/sNN
cd /tmp/guardtest
```

Reset between probes rather than trusting an undo:

```bash
git reset --hard <base> && git clean -fd
```

**One trap worth knowing.** `git rm --cached <path>` followed by `git checkout
-- <path>` does not restore the file, because `checkout --` reads the index and
`rm --cached` just removed the path from it. The working tree keeps the broken
content and the path stops being tracked, so the very check you are probing
then skips it and reports clean. That is a false green produced by the cleanup
rather than by the guard. Reset hard instead.

When you are finished:

```bash
git worktree remove /tmp/guardtest --force && git worktree prune
```

**Remove it.** A worktree is a full checkout, so every path-scanning guard
otherwise sees a second copy of the whole repository.

## The probes

Each row is one guard, the smallest change that should make it fail, and what
it printed when it did. Run the control afterwards and confirm the guard goes
green again, because a guard that fails on everything is as useless as one that
fails on nothing.

| # | Guard | Probe | Observed |
|---|-------|-------|----------|
| 1 | `staged_content_check.py` | Stage a file named `anon001`, no extension, with `DICM` at byte 128 | `anon001: this is DICOM. No patient data enters this repository, ever.` exit 1 |
| 2 | `staged_content_check.py` | Stage `fake.dcm` whose content is not DICOM at all | Refused on the suffix, exit 1. Needs `git add -f`, because `.gitignore` already covers `*.dcm` |
| 3 | `prose_check.py --staged` | An em-dash and a prose semicolon in a file under `.claude/plans/` | Both reported with line numbers, exit 1 |
| 4 | `prose_check.py --commit-msg` | The same two characters in a commit message body | Both reported against `commit message:3`, exit 1 |
| 5 | `deviation_check.py` | A plan citing `D-42`, which is not a row in `docs/hld/DEVIATIONS.md` | `cites D-42, which is not a row`, exit 1 |
| 6 | `unsafe_allowlist_check.py` | An `unsafe` block appended to `crates/ocelli-geom/src/lib.rs` | Named the file and line, printed the two-file allow-list, exit 1 |
| 7 | `pin_and_size_check.py` | `wgpu = "=30.0.1"` relaxed to `wgpu = "30.0.1"` | `is a RANGE, not an exact pin`, exit 1 |
| 8 | `backlog_check.py` | A backlog row set to `done` with no `SPRINT_TRACKER.md` row | `F-001 is done with no SPRINT_TRACKER.md row`, exit 1 |
| 9 | `source_provenance_check.py --staged` | A read-blocked project added as a dependency in `package.json` | Refused twice, once as a dependency and once as an unqualified mention, exit 1 |
| 10 | `corpus_check.py` | A manifest row whose file is present with a different digest | `expected 000000000000..., got 9dea77c83117...`, exit 1 |
| 11 | `corpus_check.py` | A manifest row whose file is absent | `corpus cases are absent`, exit 1 |
| 12 | `corpus_check.py --manifest-only` | A manifest row with an empty `licence` | `A case whose licence is unrecorded cannot be redistributed or cited`, exit 1 |
| 13 | `.githooks/commit-msg` | A hand-written `Ocelli-Verify:` trailer in the message | `Ocelli-Verify is written by this hook from the verify ledger`, exit 1 |
| 14 | `.githooks/commit-msg` | A `Co-Authored-By:` line naming an agent | `no agent co-author trailer`, exit 1 |
| 15 | `verify_ledger.py assert` | Any tree with no recorded gate run | `no verification recorded for the staged tree`, exit 1 |
| 16 | `verify_ledger.py trailer` | The same tree | Exit 1, and no trailer emitted, which is what makes the trailer unforgeable |
| 17 | `ci/check-bindgen-isolation.sh` | `wasm-bindgen = "0.2"` added to `crates/ocelli-geom/Cargo.toml` | `FAIL: ocelli-geom reaches wasm-bindgen`, exit 1 |
| 18 | Retired bootstrap converter | Omit its private redaction map | Failed closed before writing tracked Markdown. The converter and its external inputs left the verification path after S01 |
| 19a | `staged_content_check.py` | Stage a DICOM with `git add -N` rather than `git add` | **GREEN**, `OK: no patient data or build artefacts staged`. See the note below, this is a hole in the evidence and not in the hook |
| 19 | `no_std_check.py` | Add `glam = "0.30"`, default features on, to a crate that declares `no_std` | `ocelli-geom declares no_std and reaches a std feature`, exit 1. Green again with `default-features = false, features = ["libm"]` |
| 20 | `staged_content_check.py` | `git add -f tools/oracle/out/<row>.png`, a rendered reference frame of a real corpus row | `oracle output. A reference frame of a real corpus row is a rendered picture of patient data`, exit 1. The DICOM refusal does not catch a PNG and neither does the size limit |
| 21 | `check_sidecars.py --self-test` | Delete the `startswith("real/")` branch from `_show` | Eight failures, each naming the real row's value that leaked, exit 1. The redaction only ever runs on a mismatch, which no gate run produces, so it needs its own probe |
| 22 | `tools/oracle/run.mjs` | `--out` at a non-empty directory holding no `run.json` | `not an oracle output directory`, exit 1, and the directory survives intact. The harness empties its output before it renders and will not empty one it did not write |
| 23 | `tools/oracle/tests/faults.mjs` | Nothing. It IS the probe: one injected fault per refusal in the render page, covering all four oracle boundaries and the page's outer catch | Each exits 1 at its named boundary with its named reason, and the runner ends `OK: N injected fault(s), every one caught`. The catalogue in `tools/oracle/src/faults.mjs` is the count, so no number is written here. Run by `bin/ocelli.sh gate oracle` |
| 24 | `tools/oracle/run.mjs` | Run the driver through a symlinked repository path containing a space, with `--inject truncate` | Exit 1, the fault red. Reverting `isEntryPoint` to `import.meta.url === \`file://${process.argv[1]}\`` makes the same command exit **0 having printed nothing**, which is this harness's own defect class: a run that did nothing and reported success |
| 25 | `tools/oracle/run.mjs` | Remove `--use-angle=swiftshader` from `CHROMIUM_ARGS` | `which cornerstone3D does not recognise as a software rasteriser`, exit 1. The other three environment refusals fire under `--disable-webgl2`, a device scale factor of two, and a forced `useCPURendering` |

### Probe 19a is a green that means nothing, and the distinction matters

A file staged with `git add -N` appears in `git status` as `A` and in
`git diff --cached --name-only` as nothing at all. Every `--staged` guard here
reads the second one. So running them over an intent-to-add tree returns
`OK` while a DICOM sits in it.

**That is not a way to commit a DICOM.** Git refuses to commit an
intent-to-add-only path, and `git commit -a` updates the index before the hook
runs, so the guard sees the content either way. Both were tested.

What it is, is a way to obtain a green that answered a question about an empty
set. An agent that stages with `add -N`, runs `bin/ocelli.sh gate content` and
reads `OK` has learned nothing and believes it has learned something. That is
the same shape as probe 18 and probe 19, arriving from a third direction, and
it is why `AGENTS.md` now says `git add -N` does not count as staging.

### Probe 18 was a bootstrap lesson, not a current gate

Probe 18 did not exist when this runbook was started. It was added because
running the other probes surfaced the thing it now checks.

During bootstrap, the HLD converter stripped commercial product names before
writing Markdown. Its loader originally returned an empty rule list when the
private map was absent, so redaction quietly became the identity function. The
fail-closed probe prevented unsanitized text from reaching the repository.
After S01 the tracked Markdown became authoritative and the converter left the
gate, so no external source or redaction map is part of verification now.

**That is the shape to watch for in every guard here: not one that fails
wrongly, but one that has nothing to check and says so by succeeding.** Probes
15 and 16 are the same shape from the other side, which is why they are the
pair called out below.

### Probe 19 caught its own guard failing open, on the first run

Probe 19 was written for deviation D-09, which disables glam's default `std`
feature so the core crates stay `no_std`. The obvious check, compiling for
`wasm32-unknown-unknown`, does not catch a D-09 revert: that target ships a
`std` implementation and a `no_std` crate may depend on a `std` crate without
error, so it exits 0 either way. That was measured, by reverting the workspace
entry and watching every gate stay green.

The replacement reads the dependency graph instead. **Its first version
reported clean on a tree that was not**, because `cargo tree` prefixes every
line below the root with box-drawing characters and the pattern was anchored at
the start of the line, so it matched nothing at all.

It said `OK: 11 no_std crate(s) reach no std feature` while a crate reached
`std`. Nothing about that output looks wrong. It was found in the minute after
the script was written, by this procedure, and it would otherwise have been
found by whoever eventually shipped a bloated wasm module and went looking for
why.

**Two guards in this table now exist because running the table found them.**
That is the argument for the procedure, and it is stronger than the argument
the runbook opened with.

### The control that matters most

Probe 16 is the one to understand rather than tick. The `Ocelli-Verify` trailer
is the only evidence CI has that the corpus ran, because CI runs no GPU and no
corpus under deviation D-04. Probe 13 proves a trailer cannot be written by
hand, and probe 16 proves the hook emits nothing when the ledger has no record
for that exact tree. Together those two are the whole mechanism. If either
stops holding, D-04's compensating control is gone and nothing will say so.

## The standing probes, which run without anyone typing this

Everything above this line is a procedure a person performs. F-X009 made it a
gate. `scripts/guards/catalogue.py` declares each refusal with the citation
that says what it is FOR, `scripts/guard_probe.py` drives each one red inside a
disposable repository under the system temporary directory, and
`scripts/guard_census.py` proves the declaration is complete by discovering
refusal sites and refusing any site no entry claims.

**The table below is generated from that catalogue and the prose around it is
not.** A hand-written table drifts the moment a guard is added, which is the
defect this whole document is about. Re-render it with
`python3 scripts/guard_census.py --render-runbook`, and the `guards` gate
carries the `--check`.

A row marked with a `G-` number is a probe that fails today, against a hole
this sprint found in a guard that was already here. The rows carry the numbers
and the list beneath the table carries each hole in full, so no sentence here
counts them and none can go stale against them. A probe that fails for a
declared reason is reported rather than hidden. A probe marked with a defect
that starts PASSING fails the gate, because the hole was fixed and the
declaration became a lie, which is how two of the four were retired in S03.

The rows the earlier table records and this one does not are not gaps in
coverage. Rows 21 to 25 are the oracle's, and they are adopted rather than
copied: `tools/oracle/src/faults.mjs` already declares them and
`tools/oracle/tests/faults.mjs` already replays them on every `oracle` gate.
The census reads that file and verifies the adoption instead of re-declaring
it, for the reason `CLAUDE.md` gives about tier C and the LUT chain.

<!-- BEGIN GENERATED PROBE TABLE, scripts/guard_census.py -->

| # | Guard | Probe | Drives red | Profile | Watched |
|---|-------|-------|------------|---------|---------|
| 1 | `scripts/staged_content_check.py` | `content.dicom-magic`, level 3 | must refuse, output carries `this is DICOM` | floor | `bin/ocelli.sh gate guards` |
| 2 | `scripts/staged_content_check.py` | `content.dicom-suffix`, level 3 | must refuse, output carries `this is DICOM` | floor | `bin/ocelli.sh gate guards` |
| 3 | `scripts/staged_content_check.py` | `content.no-allowlist`, level 3 | must refuse, output carries `There is no allowlist` | floor | `bin/ocelli.sh gate guards` |
| 4 | `scripts/staged_content_check.py` | `content.oracle-output`, level 3 | must refuse, output carries `oracle output` | floor | `bin/ocelli.sh gate guards` |
| 5 | `scripts/staged_content_check.py` | `content.compare-output`, level 3 | must refuse, output carries `comparator output` | floor | `bin/ocelli.sh gate guards` |
| 6 | `scripts/staged_content_check.py` | `content.spike-output`, level 3 | must refuse, output carries `spike output` | floor | `bin/ocelli.sh gate guards` |
| 7 | `scripts/staged_content_check.py` | `content.build-artefact`, level 3 | must refuse, output carries `build artefact` | floor | `bin/ocelli.sh gate guards` |
| 8 | `scripts/staged_content_check.py` | `content.size-limit`, level 3 | must refuse, output carries `byte limit` | floor | `bin/ocelli.sh gate guards` |
| 9 | `scripts/unsafe_allowlist_check.py` | `unsafe.third-file`, level 3 | must refuse, output carries ``unsafe` outside the allow-list (HLD section 27.2 R5)` | floor | `bin/ocelli.sh gate guards` |
| 10 | `scripts/prose_check.py` | `prose.em-dash`, level 3 | must refuse, output carries `em-dash` | floor | `bin/ocelli.sh gate guards` |
| 11 | `scripts/prose_check.py` | `prose.semicolon`, level 3 | must refuse, output carries `semicolon in prose` | floor | `bin/ocelli.sh gate guards` |
| 12 | `scripts/prose_check.py` | `prose.commit-message`, level 3 | must refuse, output carries `semicolon in prose` | floor | `bin/ocelli.sh gate guards` |
| 13 | `scripts/deviation_check.py` | `deviations.undeclared-citation`, level 3 | must refuse, output carries `which is not a row` | floor | `bin/ocelli.sh gate guards` |
| 14 | `scripts/deviation_check.py` | `deviations.stale-d01`, level 3 | must refuse, output carries `records rust-version 1.97.1` | floor | `bin/ocelli.sh gate guards` |
| 15 | `scripts/deviation_check.py` | `deviations.stale-resolver`, level 3 | must refuse, output carries `records resolver 3` | floor | `bin/ocelli.sh gate guards` |
| 16 | `scripts/source_provenance_check.py` | `provenance.dependency`, level 3 | must refuse, output carries `depends on` | floor | `bin/ocelli.sh gate guards` |
| 17 | `scripts/source_provenance_check.py` | `provenance.unqualified-mention`, level 3 | must refuse, output carries `with no statement that it is out of bounds` | floor | `bin/ocelli.sh gate guards` |
| 18 | `scripts/pin_and_size_check.py` | `pins.range`, level 3 | must refuse, output carries `is a RANGE, not an exact pin` | floor | `bin/ocelli.sh gate guards` |
| 19 | `scripts/pin_and_size_check.py` | `pins.partial-version`, level 3 | must refuse, output carries `PARTIAL version` | floor | `bin/ocelli.sh gate guards` |
| 20 | `scripts/pin_and_size_check.py` | `pins.absent`, level 3 | must refuse, output carries `is not declared in [workspace.dependencies]` | floor | `bin/ocelli.sh gate guards` |
| 21 | `scripts/pin_and_size_check.py` | `pins.size-ceiling`, level 3 | must refuse, output carries `byte ceiling` | floor | `bin/ocelli.sh gate guards` |
| 22 | `scripts/pin_and_size_check.py` | `pins.table-form`, level 3 | must ACCEPT, output carries `pinned exactly` | floor | `bin/ocelli.sh gate guards` |
| 23 | `scripts/no_std_check.py` | `nostd.reaches-std`, level 3 | must refuse, output carries `reaches a std feature` | deep | `bin/ocelli.sh gate guards-deep` |
| 24 | `scripts/no_std_check.py` | `nostd.none-declared`, level 3 | must refuse, output carries `no crate under crates/ declares no_std` | deep | `bin/ocelli.sh gate guards-deep` |
| 25 | `scripts/no_std_check.py` | `nostd.loses-a-crate`, level 3 **G-02, fails today** | must refuse, output carries `stopped declaring no_std` | deep | `bin/ocelli.sh gate guards-deep` |
| 26 | `scripts/ci_floor_check.py` | `ci-floor.event-gated`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 27 | `scripts/ci_floor_check.py` | `ci-floor.event-gated-accept`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 28 | `scripts/ci_floor_check.py` | `ci-floor.complementary-steps`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 29 | `scripts/ci_floor_check.py` | `ci-floor.missing-step`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 30 | `scripts/ci_floor_check.py` | `ci-floor.partial-arm`, level 3 | must refuse, output carries `runs only part of it on` | floor | `bin/ocelli.sh gate guards` |
| 31 | `scripts/ci_floor_check.py` | `ci-floor.whole-arm-through-the-runner`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 32 | `scripts/ci_floor_check.py` | `ci-floor.non-floor-gate-not-run`, level 3 | must refuse, output carries `is excluded from the floor and needs no GPU` | floor | `bin/ocelli.sh gate guards` |
| 33 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-lists-disagree`, level 3 | must refuse, output carries `disagree about which gates the floor excludes` | floor | `bin/ocelli.sh gate guards` |
| 34 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-list-reordered`, level 3 | must ACCEPT, output carries `exclusion list and NOT_IN_FLOOR agree on` | floor | `bin/ocelli.sh gate guards` |
| 35 | `scripts/ci_floor_check.py` | `ci-floor.gate-with-no-arm-command`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 36 | `scripts/ci_floor_check.py` | `ci-floor.narrowed-arm-command`, level 3 | must refuse, output carries `runs only part of it on` | floor | `bin/ocelli.sh gate guards` |
| 37 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-list-unreadable`, level 3 | must refuse, output carries `exclusion list where this parser looks for it` | floor | `bin/ocelli.sh gate guards` |
| 38 | `scripts/ci_floor_check.py` | `ci-floor.run-gate-region-unreadable`, level 3 | must refuse, output carries `carries no `run_gate() {`` | floor | `bin/ocelli.sh gate guards` |
| 39 | `scripts/ci_floor_check.py` | `ci-floor.no-arms-at-all`, level 3 | must refuse, output carries `declares no case arm this parser can read` | floor | `bin/ocelli.sh gate guards` |
| 40 | `scripts/ci_floor_check.py` | `ci-floor.comment-only`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 41 | `scripts/verify_ledger.py` | `ledger.no-record`, level 3 | must refuse, output carries `no verification recorded for the staged tree` | floor | `bin/ocelli.sh gate guards` |
| 42 | `scripts/verify_ledger.py` | `ledger.red-corpus`, level 3 | must refuse, output carries `the corpus is RED for tree` | floor | `bin/ocelli.sh gate guards` |
| 43 | `scripts/verify_ledger.py` | `ledger.require-corpus`, level 3 | must refuse, output carries `and this gate requires 'pass'` | floor | `bin/ocelli.sh gate guards` |
| 44 | `scripts/verify_ledger.py` | `ledger.bad-state`, level 3 | must refuse, output carries `--corpus must be one of` | floor | `bin/ocelli.sh gate guards` |
| 45 | `scripts/verify_ledger.py` | `ledger.trailer-silent`, level 3 | must refuse, output carries `` | floor | `bin/ocelli.sh gate guards` |
| 46 | `scripts/verify_ledger.py` | `ledger.no-trailer`, level 3 | must refuse, output carries `carries no Ocelli-Verify trailer` | floor | `bin/ocelli.sh gate guards` |
| 47 | `scripts/verify_ledger.py` | `ledger.tree-mismatch`, level 3 | must refuse, output carries `but the commit's tree is` | floor | `bin/ocelli.sh gate guards` |
| 48 | `.githooks/pre-commit` | `hooks.pre-commit.dicom`, level 3 | must refuse, output carries `Commit refused` | floor | `bin/ocelli.sh gate guards` |
| 49 | `.githooks/commit-msg` | `hooks.commit-msg.forged-trailer`, level 3 | must refuse, output carries `written by this hook from the verify ledger` | floor | `bin/ocelli.sh gate guards` |
| 50 | `.githooks/commit-msg` | `hooks.commit-msg.agent-coauthor`, level 3 | must refuse, output carries `no agent co-author trailer` | floor | `bin/ocelli.sh gate guards` |
| 51 | `.githooks/commit-msg` | `hooks.commit-msg.prose`, level 3 | must refuse, output carries `semicolon in prose` | floor | `bin/ocelli.sh gate guards` |
| 52 | `.githooks/pre-push` | `hooks.pre-push.unverified`, level 3 | must refuse, output carries `Push refused` | floor | `bin/ocelli.sh gate guards` |
| 53 | `ci/check-bindgen-isolation.sh` | `bindgen.reaches`, level 3 | must refuse, output carries `reaches wasm-bindgen` | deep | `bin/ocelli.sh gate guards-deep` |
| 54 | `ci/check-bindgen-isolation.sh` | `bindgen.declares`, level 3 | must refuse, output carries `declares wasm-bindgen as a direct dependency` | deep | `bin/ocelli.sh gate guards-deep` |
| 55 | `ci/check-bindgen-isolation.sh` | `bindgen.in-source`, level 3 | must refuse, output carries `names wasm_bindgen in source` | deep | `bin/ocelli.sh gate guards-deep` |
| 56 | `ci/check-device-ownership.sh` | `device.creator`, level 3 | must refuse, output carries `creates a GPU device or surface` | floor | `bin/ocelli.sh gate guards` |
| 57 | `ci/check-device-ownership.sh` | `device.contract-gone`, level 3 | must refuse, output carries `no longer defines GpuContext` | floor | `bin/ocelli.sh gate guards` |
| 58 | `ci/check-device-ownership.sh` | `device.owned-accessor`, level 3 | must refuse, output carries `hands out an owned device` | floor | `bin/ocelli.sh gate guards` |
| 59 | `ci/check-device-ownership.sh` | `device.derives-clone`, level 3 | must refuse, output carries `derives Clone` | floor | `bin/ocelli.sh gate guards` |
| 60 | `scripts/backlog_check.py` | `backlog.done-without-record`, level 3 | must refuse, output carries `is done with no SPRINT_TRACKER.md row` | floor | `bin/ocelli.sh gate guards` |
| 61 | `scripts/backlog_check.py` | `backlog.bad-status`, level 3 | must refuse, output carries `expected one of` | floor | `bin/ocelli.sh gate guards` |
| 62 | `scripts/backlog_check.py` | `backlog.hook-out-of-phase`, level 3 | must refuse, output carries `not P1` | floor | `bin/ocelli.sh gate guards` |
| 63 | `scripts/gen_sprint_plan.py` | `sprint-plan.two-sprint-tables`, level 3 | must refuse, output carries `SPRINT_PLAN.md sprint tables` | floor | `bin/ocelli.sh gate guards` |
| 64 | `scripts/gen_sprint_plan.py` | `sprint-plan.milestone-summary-absent`, level 3 | must refuse, output carries `carries no summary line for it` | floor | `bin/ocelli.sh gate guards` |
| 65 | `scripts/gen_sprint_plan.py` | `sprint-plan.milestone-summary-spurious`, level 3 | must refuse, output carries `has no milestone spanning those sprints` | floor | `bin/ocelli.sh gate guards` |
| 66 | `scripts/gen_sprint_plan.py` | `sprint-plan.two-goal-lines`, level 3 | must refuse, output carries `**Goal** lines` | floor | `bin/ocelli.sh gate guards` |
| 67 | `scripts/gen_sprint_plan.py` | `sprint-plan.absent`, level 3 | must refuse, output carries `does not exist` | floor | `bin/ocelli.sh gate guards` |
| 68 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-sprint`, level 3 | must refuse, output carries `is in sprint` | floor | `bin/ocelli.sh gate guards` |
| 69 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-estimate`, level 3 | must refuse, output carries `is estimated` | floor | `bin/ocelli.sh gate guards` |
| 70 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-milestone-summary`, level 3 | must refuse, output carries `milestone summary line` | floor | `bin/ocelli.sh gate guards` |
| 71 | `scripts/gen_sprint_plan.py` | `sprint-plan.stale-goal-line`, level 3 | must refuse, output carries `**Goal** line` | floor | `bin/ocelli.sh gate guards` |
| 72 | `scripts/sync_agent_skills.py` | `skills.stale-adapter`, level 3 | must refuse, output carries `is stale, its source changed` | floor | `bin/ocelli.sh gate guards` |
| 73 | `scripts/sprint_workflow.py` | `handoff.wrong-branch`, level 3 | must refuse, output carries `does not start with` | floor | `bin/ocelli.sh gate guards` |
| 74 | `scripts/sprint_workflow.py` | `handoff.backticked-branch`, level 3 **G-04, fails today** | must ACCEPT, output carries `validates` | floor | `bin/ocelli.sh gate guards` |
| 75 | `scripts/sprint_workflow.py` | `sprint-lifecycle.not-in-sprint`, level 3 | must refuse, output carries `is not in sprint` | floor | `bin/ocelli.sh gate guards` |
| 76 | `scripts/error_code_check.py` | `errors.renumbered`, level 3 | must refuse, output carries `so this is a renumbering` | floor | `bin/ocelli.sh gate guards` |
| 77 | `scripts/bench_check.py` | none in this harness | A benchmark registry whose subject stories do not resolve, a subject whose story has not landed carrying a runner or a recorded number, and a playwright pin that has drifted between the two harnesses. | - | scripts/tests/test_bench_check.py (25 cases, run by the `bench` gate, which is in the floor) |
| 78 | `scripts/corpus_check.py` | `corpus.digest-mismatch`, level 3 | must refuse, output carries `does not match its manifest digest` | floor | `bin/ocelli.sh gate guards` |
| 79 | `scripts/corpus_check.py` | `corpus.absent`, level 3 | must refuse, output carries `corpus cases are absent` | floor | `bin/ocelli.sh gate guards` |
| 80 | `scripts/corpus_check.py` | `corpus.unrecorded-licence`, level 3 | must refuse, output carries `cannot be redistributed or cited` | floor | `bin/ocelli.sh gate guards` |
| 81 | `scripts/corpus_tests.py` | `corpus-tests.skip-is-not-a-pass`, level 3 | must refuse, output carries `FAIL: a prerequisite` | floor | `bin/ocelli.sh gate guards` |
| 82 | `scripts/corpus_synth.py` | none in this harness | A manifest whose header is not the recorded column set. | - | scripts/tests/test_corpus_synth.py (run by the `corpus-tests` gate) |
| 83 | `scripts/target_feature_check.py` | `target-features.cannot-run`, level 3 | must refuse, output carries `could not run` | deep | `bin/ocelli.sh gate guards-deep` |
| 84 | `scripts/package_check.py` | `packages.exports-not-in-tarball`, level 1 | must refuse, output carries `advertises` | floor | `bin/ocelli.sh gate guards` |
| 85 | `scripts/package_check.py` | `packages.version-skew`, level 1 | must refuse, output carries `the Rust workspace is` | floor | `bin/ocelli.sh gate guards` |
| 86 | `bin/ocelli.sh` | `runner.absent-prerequisite`, level 3 | must refuse, output carries `reference stack is not installed` | floor | `bin/ocelli.sh gate guards` |
| 87 | `scripts/source_dir.py` | `source-dir.unconfigured`, level 3 | must refuse, output carries `are not configured` | floor | `bin/ocelli.sh gate guards` |
| 88 | `scripts/split_hld.py` | `split-hld.cannot-run`, level 3 | must refuse, output carries `A check that cannot run is NOT a check that` | floor | `bin/ocelli.sh gate guards` |
| 89 | `scripts/lint_policy_check.py` | `lint-policy.weakened`, level 3 | must refuse, output carries `is 'allow' and HLD 27.1 requires` | floor | `bin/ocelli.sh gate guards` |
| 90 | `scripts/lint_policy_check.py` | `lint-policy.uninherited`, level 3 | must refuse, output carries `does not inherit the workspace lint table` | floor | `bin/ocelli.sh gate guards` |
| 91 | `scripts/lint_policy_check.py` | `lint-policy.group-allow`, level 3 | must refuse, output carries `allows the lint group` | floor | `bin/ocelli.sh gate guards` |
| 92 | `scripts/lint_policy_check.py` | `lint-policy.expect-attribute`, level 3 | must refuse, output carries `re-allows` | floor | `bin/ocelli.sh gate guards` |
| 93 | `scripts/lint_policy_check.py` | `lint-policy.nothing-scanned`, level 3 | must refuse, output carries `not one `.rs` file was read` | floor | `bin/ocelli.sh gate guards` |
| 94 | `scripts/lint_policy_check.py` | `lint-policy.allow-outside-the-crate-root`, level 3 | must refuse, output carries `re-allows `cast_possible_truncation`` | floor | `bin/ocelli.sh gate guards` |
| 95 | `scripts/lint_policy_check.py` | `lint-policy.item-allow-is-permitted`, level 3 | must ACCEPT, output carries `carry no inner allow or expect of a denied lint` | floor | `bin/ocelli.sh gate guards` |
| 96 | `scripts/lint_policy_check.py` | `lint-policy.outer-allow-on-a-module`, level 3 | must refuse, output carries `an outer attribute on a `mod` item covers the whole module tree` | floor | `bin/ocelli.sh gate guards` |
| 97 | `scripts/lint_policy_check.py` | `lint-policy.whitespace-in-the-lint-path`, level 3 | must refuse, output carries `allows the lint group` | floor | `bin/ocelli.sh gate guards` |
| 98 | `scripts/lint_policy_check.py` | `lint-policy.member-outside-crates-uninherited`, level 3 | must refuse, output carries `does not inherit the workspace lint table` | floor | `bin/ocelli.sh gate guards` |
| 99 | `scripts/lint_policy_check.py` | `lint-policy.member-outside-crates-group-allow`, level 3 | must refuse, output carries `allows the lint group` | floor | `bin/ocelli.sh gate guards` |
| 100 | `scripts/lint_policy_check.py` | `lint-policy.member-unresolvable`, level 3 | must refuse, output carries `resolves to no directory carrying a Cargo.toml` | floor | `bin/ocelli.sh gate guards` |
| 101 | `scripts/lint_policy_check.py` | `lint-policy.no-members-declared`, level 3 | must refuse, output carries `declares no `members` this parser can read` | floor | `bin/ocelli.sh gate guards` |
| 102 | `scripts/lint_policy_check.py` | `lint-policy.group-row-in-the-workspace-table`, level 3 | must refuse, output carries `carries the lint GROUP` | floor | `bin/ocelli.sh gate guards` |
| 103 | `scripts/guards/census.py` | `census.constants-count-shrunk`, level 3 | must refuse, output carries `Narrowing the declared-constant ratchet` | floor | `bin/ocelli.sh gate guards` |
| 104 | `scripts/guards/census.py` | `census.unclaimed-executable-hook`, level 3 | must refuse, output carries `is executable in a clone that opts in` | floor | `bin/ocelli.sh gate guards` |
| 105 | `scripts/guards/census.py` | `census.oracle-runner-gone`, level 3 | must refuse, output carries `so the `oracle` gate has no runner` | floor | `bin/ocelli.sh gate guards` |
| 106 | `scripts/guards/census.py` | `census.unrecognised-kind`, level 3 | must refuse, output carries `which is not one of` | floor | `bin/ocelli.sh gate guards` |
| 107 | `scripts/guards/census.py` | `census.no-entry-site-count`, level 3 | must refuse, output carries `records no per-entry site count` | floor | `bin/ocelli.sh gate guards` |
| 108 | `scripts/guards/census.py` | `census.refusal-in-a-claimed-file`, level 3 | must refuse, output carries `refusal site(s) to` | floor | `bin/ocelli.sh gate guards` |
| 109 | `scripts/guards/census.py` | `census.unclaimed-site`, level 3 | must refuse, output carries `no catalogue entry claims` | floor | `bin/ocelli.sh gate guards` |
| 110 | `scripts/guards/census.py` | `census.changed-constant`, level 3 | must refuse, output carries `changed without its recorded value` | floor | `bin/ocelli.sh gate guards` |
| 111 | `scripts/guards/census.py` | `census.no-std-set-shrunk`, level 3 | must refuse, output carries `changed without its recorded value` | floor | `bin/ocelli.sh gate guards` |
| 112 | `scripts/guards/census.py` | `census.uncovered-grew`, level 3 | must refuse, output carries `The ratchet may only decrease` | floor | `bin/ocelli.sh gate guards` |
| 113 | `scripts/guards/census.py` | `census.no-ceiling`, level 3 | must refuse, output carries `records no uncovered ceiling` | floor | `bin/ocelli.sh gate guards` |
| 114 | `scripts/guards/census.py` | `census.orphan-recorded-constant`, level 3 | must refuse, output carries `is not declared in the catalogue's CONSTANTS` | floor | `bin/ocelli.sh gate guards` |
| 115 | `scripts/guards/census.py` | `census.gate-without-an-entry`, level 3 | must refuse, output carries `has no catalogue entry and no `delegated` reason` | floor | `bin/ocelli.sh gate guards` |
| 116 | `scripts/guards/census.py` | `census.floor-needing-a-gpu`, level 1 | must refuse, output carries `no GPU, no browser and no corpus` | floor | `bin/ocelli.sh gate guards` |
| 117 | `scripts/guard_census.py` | `census-runbook.markers-gone`, level 3 | must refuse, output carries `carries no generated-table markers` | floor | `bin/ocelli.sh gate guards` |
| 118 | `scripts/guard_probe.py` | `probe-runner.self-test`, level 3 | must ACCEPT, output carries `OK` | floor | `bin/ocelli.sh gate guards` |
| 119 | `scripts/guards/sandbox.py` | none in this harness | A git call outside the sandbox, a git call in a directory this harness did not create, and the `rm --cached` and `checkout --` pair the runbook records as a false-green trap. | - | scripts/guard_probe.py --self-test; scripts/tests/test_guard_catalogue.py |
| 120 | `scripts/guards/discover.py` | none in this harness | Nothing on its own. It is the scanner the census refuses from. | - | scripts/tests/test_guard_catalogue.py |
| 121 | `scripts/guards/catalogue.py` | none in this harness | A probe builder that mutated nothing, and a fixture drawn from a citation that has gone away. | - | scripts/tests/test_guard_catalogue.py |
| 122 | `tools/oracle/run.mjs` | none in this harness | A run that reached no row, a decode that produced nothing, a frame that never presented, a read-back still showing the sentinel, a volume that did not load, and the environment refusals that keep the rasteriser honest. | - | tools/oracle/src/faults.mjs (23 injected faults, replayed by tools/oracle/tests/faults.mjs on every `oracle` gate); tools/oracle/tests/args_test.mjs; tools/oracle/tests/paths_test.mjs; tools/oracle/tests/pins_test.mjs |
| 123 | `tools/oracle/page/app.mjs` | none in this harness | A render that did not present, a read-back that is not comparable, and a frame the page cannot attribute. | - | tools/oracle/src/faults.mjs |
| 124 | `tools/oracle/page/volume.mjs` | none in this harness | A volume that did not load, a geometry that is not a permutation of the declared one, and a reformat that never presented. | - | tools/oracle/src/faults.mjs |
| 125 | `tools/oracle/src/volume.mjs` | none in this harness | A volume subject whose members disagree, a spacing or orientation that does not resolve, and a declared truth the reference does not reproduce. | - | tools/oracle/tests/volume_test.mjs (run inside `bin/ocelli.sh oracle`'s unit pass) |
| 126 | `tools/oracle/src/manifest.mjs` | none in this harness | A manifest the reference half cannot read, and a row it cannot resolve to a case. | - | tools/oracle/tests/manifest_test.mjs |
| 127 | `tools/oracle/src/geometry.mjs` | none in this harness | A geometry the reference cannot express, and one that does not round-trip. | - | tools/oracle/tests/geometry_test.mjs |
| 128 | `tools/oracle/src/voi.mjs` | none in this harness | A VOI declaration the reference cannot apply, and a window the row does not carry. | - | tools/oracle/tests/params_test.mjs |
| 129 | `tools/oracle/src/params.mjs` | none in this harness | A render parameter the page does not implement, and a parameter set that does not resolve for a row. | - | tools/oracle/tests/params_test.mjs |
| 130 | `tools/oracle/src/unsupported.mjs` | none in this harness | A row declared unsupported that renders, and a row that fails without a declaration. | - | tools/oracle/tests/unsupported_test.mjs |
| 131 | `tools/oracle/src/output.mjs` | none in this harness | An output directory that is not the harness's own. | - | tools/oracle/tests/output_test.mjs |
| 132 | `tools/oracle/src/pins.mjs` | none in this harness | A reference stack installed at a version nobody pinned. | - | tools/oracle/tests/pins_test.mjs |
| 133 | `tools/oracle/src/sidecar.mjs` | none in this harness | A sidecar the comparator cannot read. | - | tools/oracle/tests/sidecar_test.mjs |
| 134 | `tools/oracle/check_sidecars.py` | none in this harness | A cross-read mismatch reported without redacting a real row's values, and a sidecar pydicom and the reference disagree about. | - | tools/oracle/check_sidecars.py --self-test, run by tools/oracle/run.mjs under both interpreters |
| 135 | `tools/oracle/src/faults.mjs` | none in this harness | A fault name nothing declares, and a filter that selected nothing reading as success. | - | tools/oracle/tests/faults.mjs |
| 136 | `tools/bench/src/registry.mjs` | none in this harness | A registry entry with no subject story, a duplicate subject, and a runner for a subject whose story has not landed. | - | tools/bench/tests/registry_test.mjs (run by the `bench` gate) |
| 137 | `tools/bench/src/record.mjs` | none in this harness | A record written against a host class it was not measured on, and a malformed baseline. | - | tools/bench/tests/record_test.mjs; tools/bench/tests/hostclass_test.mjs |
| 138 | `tools/bench/src/state.mjs` | none in this harness | A run state the harness cannot resume from. | - | tools/bench/tests/state_test.mjs |
| 139 | `tools/bench/run.mjs` | none in this harness | An argument the harness does not accept, a subject that does not exist, a comparison on a machine that does not own the baseline, and a browser that is not installed. | - | nothing |
| 140 | `tools/bench/src/runners/wasm_cold_start.mjs` | none in this harness | A cold-start measurement taken against a stub, an incomplete artefact copy, and a page that never reported. | - | tools/bench/tests/cold_start_test.mjs |
| 141 | `tools/bench/page/app.mjs` | none in this harness | A page serving an incomplete copy of the wasm artefact. | - | tools/bench/tests/cold_start_test.mjs |
| 142 | `scripts/panic_probe.mjs` | none in this harness | A run that measured the probe's stub rather than the module. | - | bin/ocelli.sh gate panic, which builds a second module carrying the panic-probe feature and runs this file on every floor gate |

Known defects this table names, in full:

- **G-02.** scripts/no_std_check.py loses a crate rather than failing. Its crate set is built from the crates that match the attribute, so a crate deleting it is not reported, it stops being checked. The only backstop fires when NO crate declares it. The recorded NO_STD_CRATES constant is what catches it today.
- **G-04.** scripts/sprint_workflow.py validate-handoff has an undocumented contract. It needs a literal `**Head**` and parses the branch as a bare token, so backticks break it, and this repository writes every path in backticks. It cost one handoff rewritten at integration in S03.

What a probe above does NOT reach, declared rather than left to be discovered:

- **`scripts/source_provenance_check.py`.** The URL clause has no probe, deliberately. `docs/SOURCE-POLICY.md` names the projects and does not name their addresses, so a probe would have to take its input from the guard's own URL list, which is the R2 failure this catalogue exists to avoid: it would assert the current regex and pass forever once the regex was weakened. The list itself is a declared constant in the ratchet, so a change to it is caught there and lands in front of a reviewer.
- **`scripts/ci_floor_check.py`.** The non-floor rule exempts a gate `bin/ocelli.sh` marks YES in its GPU column, which is `oracle` and deviation D-04's reason for it, and that column is not in the declared-constant ratchet. Marking `guards-deep|YES|` would therefore exempt it without this check noticing. It is left as a limit rather than recorded, because the column is a semantic claim in the runner's own gate table where a false entry reads as false to a person, and because `NOT_IN_FLOOR` and the runner's exclusion list are both watched, so the OTHER routes out of the floor are closed. The second limit is that this file cannot evaluate `github.ref`, so what it proves about `guards-deep` is that CI runs it on `workflow_dispatch`. The workflow's own comment claims a push to `main` as well and that half is unproven, which the OK line says in as many words rather than leaving a reader to infer the stronger claim. The third limit is the arm-command extractor's own vocabulary. It recognises `python3 `, `npm run `, `cargo ` and `ci/`, so `node`, `wasm-pack` and `"$0"` are invisible and the three `node --test` suites in `bench`, the wasm-pack build in `panic` and the `"$0"` self-calls in `wasm` and `native` are not demanded of CI by a check whose message says every command in the arm is. Nothing is lost today, because each of those four gates is invoked by NAME in ci.yml and a step naming a gate runs its arm entire by definition. Widening the vocabulary would cost the other claim: `panic` would stop being a gate with no extractable command, and probe `ci-floor.gate-with-no-arm-command` is what watches that one.
- **`scripts/verify_ledger.py`.** `assert` with a real record cannot be controlled green in the sandbox without recording one first, so the control for this invoke records a passing entry and then asserts.
- **`scripts/verify_ledger.py`.** The `records a RED corpus` and `records corpus=` branches of check-commit need a commit carrying a trailer this harness would have to forge, and the commit-msg hook refuses exactly that. They are reached instead by ledger.assert's equivalents.
- **`ci/check-device-ownership.sh`.** The trybuild compile-fail cases under crates/ocelli-compute/tests/ui/ are the strong half of section 31 and run in the `test` gate, and they are recorded here rather than as `covered_by`. They assert the same property through the type system and they do not open ci/check-device-ownership.sh, so counting them as coverage of THIS guard's refusals would be the claim check f exists to refuse. The four probes above are what watch those refusals.
- **`scripts/sprint_workflow.py`.** The other twenty-two refusals in this file belong to the sprint lifecycle commands, and the entry below owns them. The field list is a second limit and a sharper one. Both probes here are about the BRANCH rule, and reaching it means writing a handoff that passes the field check first, so their input carries the five `**Field**` markers from the tool's own tuple. `.claude/commands/complete-feature.md` names six items including the files touched, which the tool does not require, so the citation and the code do not agree and no probe can see that. `HANDOFF_FIELDS` is in the declared-constant ratchet instead, which is what puts a change to the contract in front of a reviewer. G-04 is the same undocumented contract seen from the branch side.
- **`scripts/sprint_workflow.py`.** One probe over the shape shared by every lifecycle refusal. The remaining branches need a sprint mid-flight, which the sandbox cannot build without writing sprint state, and docs/sprints/ is outside this story's write set.
- **`scripts/bench_check.py`.** No level-3 probe. The suite above is the negative-case set for this guard and it runs on every floor gate, so a level-3 probe would need a second registry fixture that the suite already carries.
- **`scripts/target_feature_check.py`.** The per-target divergence branch itself needs a dependency whose features differ by target, which cannot be built from the locked graph without a network fetch. Owner F-X014.
- **`scripts/package_check.py`.** The consumer install, the node import, the two tsc resolutions and the publish dry run are level 3 and need an npm install the sandbox does not carry. They run for real in the `packages` gate on every push, green, and this harness has not watched them red. Owner F-X014.
- **`scripts/split_hld.py`.** The redaction map's fail-closed branch, which is runbook probe 18 itself, sits behind a pandoc conversion of the private `.docx`. Neither is in this repository, so a sandbox cannot reach it, and the same is true of the drift and section-mapping branches. Owner F-X014.
- **`scripts/lint_policy_check.py`.** `REFUSED_GROUPS` is a list of nine names that exists only in the guard, and a probe can only ever write one of them, so narrowing the list to `clippy::pedantic` would leave `lint-policy.group-allow` green with eight groups unguarded. That is the shape `device.owned-accessor` has, and the answer is the same: the set is in the declared-constant ratchet, so narrowing it fails the census in the same change. Two of the nine are measured to reach 27.1's table under clippy 1.97.1 and the other seven are refused as blanket allows, which the message says rather than overclaiming.
- **`tools/oracle/run.mjs`.** Adopted and not re-declared, and not re-run either, because the run needs a browser and the `oracle` gate already does it. A second declaration of the same faults is the same defect as a second copy of the LUT chain, except that it only runs where nobody looks. What the census verifies instead is that the catalogue still carries faults, that each still names the message fragment proving its own boundary, that tools/oracle/run.mjs still reaches the runner, and that the count has not shrunk below the recorded 23.
- **`tools/bench/src/runners/wasm_cold_start.mjs`.** Four of that suite's five tests are in the `bench` gate since the S03 review's fourth pass, which moved playwright to an `await import` inside `run()`. The fifth launches a browser and is opted into with OCELLI_BENCH_BROWSER=1, so the refusals THIS entry names, a measurement against a stub, an incomplete artefact copy and a page that never reported, are still watched only when a developer runs the harness. Owner F-X014.
- **`tools/bench/page/app.mjs`.** Same browser dependency as the fifth test bench.cold-start describes. The four tests that joined the `bench` gate do not serve the page, so this entry's refusal is watched only when a developer runs the harness. Owner F-X014.
- **`scripts/panic_probe.mjs`.** The stub refusal itself has not been watched red. It fires only when the module fails to export what the probe imports, which needs a broken wasm-pack build to construct. Owner F-X014.

Declared out of scope. Each of these carries refusals that the census counts and that no gate runs, so calling them guards would inflate the coverage number:

- **`scripts/populate_corpus.py`.** A corpus acquisition tool a developer runs by hand. No gate and no CI step invokes it, and its first act is to refuse when the locked Python environment is absent, which the sandbox always is because `.venv` is not tracked. The property its refusals protect, that a case matches the digest its manifest row records, is verified afterwards and independently by scripts/corpus_check.py, which IS in a gate and whose digest and presence refusals are both probed above.
- **`scripts/import_backlog_xlsx.py`.** A bootstrap importer over a private spreadsheet that is not in this repository and is not fetched by anything. No gate and no CI step invokes it. Its OUTPUT is tracked, and that output is watched by scripts/backlog_check.py and scripts/gen_sprint_plan.py, both of which are in the floor and both of which carry probes above.
- **`tools/spikes/a1-htj2k/run.mjs`.** A throwaway spike harness for Appendix A gate A1, invoked by no gate and by no CI step. `content.spike-output` guards its output DIRECTORY and is not a backstop for its refusals, unlike the probed backstops populate-corpus and bootstrap-importer name. Nothing watches these go red.
- **`tools/spikes/common/compare.mjs`.** Shared helper for the same throwaway spike harnesses, invoked by no gate and by no CI step. The measurement is throwaway and the ANSWER is not: this file produced every digest in both Appendix A answer files, and `docs/spikes/GATES.md`'s A1 verdict and the decision to file F-X013 rest on them. Its own suite, `tools/spikes/common/tests/compare_test.mjs`, is run by nothing, which is pass 1's smell S19 and is unfixed. Out of scope here means no gate runs the file, not that its refusals did not matter. Owner F-X014.
- **`tools/spikes/common/extract.py`.** The spike harnesses' corpus extractor, invoked by no gate. Its refusals protect a throwaway measurement rather than the repository. `content.spike-output` guards the output DIRECTORY, which is not a backstop for these refusals, so nothing watches them go red.
- **`tools/spikes/a2-jpeg-ls/anchors.py`.** A throwaway spike harness for Appendix A gate A2, invoked by no gate. Same argument as spikes.a1, including that its output directory being guarded is not a backstop for its refusals.

<!-- END GENERATED PROBE TABLE -->

## What a guard-verification run is not

It is not a substitute for the gates. `bin/ocelli.sh gate --floor` says whether
this tree is clean. This procedure says whether that answer means anything.

It also does not prove a guard catches everything in its class. Probe 1 shows
the DICOM check catches a file with the magic bytes and no extension, which is
the case the check exists for. It does not show the check catches a DICOM
embedded inside an archive, and nothing in this repository claims it does.

## Record of runs

| Date | Sprint | Guards probed | Result |
|------|--------|---------------|--------|
| 2026-09-04 | S01 | Rows 1 to 17 | Every probe went red with a specific message, every control returned green |
| 2026-09-04 | S01 | Row 18 | Added during the same run, after the exercise surfaced the fail-open redaction loader. Red with a specific message, and `docs/hld/` provably untouched |
| 2026-09-04 | S01 | Row 19 | Added for deviation D-09. Its first version reported clean on a broken tree, was corrected, then went red with a specific message and green on the correct shape. The S01 sprint review wired it into the `nostd` gate and CI |
