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
| 22 | `scripts/pin_and_size_check.py` | `pins.package-licence-absent`, level 3 | must refuse, output carries `LICENSE-APACHE is absent` | floor | `bin/ocelli.sh gate guards` |
| 23 | `scripts/pin_and_size_check.py` | `pins.package-licence-symlink`, level 3 | must refuse, output carries `LICENSE-APACHE is a symlink, not a regular file` | floor | `bin/ocelli.sh gate guards` |
| 24 | `scripts/pin_and_size_check.py` | `pins.stale-operational-parity`, level 3 | must refuse, output carries `does not name the oracle pin` | floor | `bin/ocelli.sh gate guards` |
| 25 | `scripts/pin_and_size_check.py` | `pins.table-form`, level 3 | must ACCEPT, output carries `pinned exactly` | floor | `bin/ocelli.sh gate guards` |
| 26 | `scripts/no_std_check.py` | `nostd.reaches-std`, level 3 | must refuse, output carries `reaches a std feature` | deep | `bin/ocelli.sh gate guards-deep` |
| 27 | `scripts/no_std_check.py` | `nostd.none-declared`, level 3 | must refuse, output carries `no crate under crates/ declares no_std` | deep | `bin/ocelli.sh gate guards-deep` |
| 28 | `scripts/no_std_check.py` | `nostd.loses-a-crate`, level 3 | must refuse, output carries `stopped declaring no_std` | deep | `bin/ocelli.sh gate guards-deep` |
| 29 | `scripts/no_std_check.py` | `nostd.gains-a-crate`, level 3 | must refuse, output carries `unexpectedly declares no_std` | deep | `bin/ocelli.sh gate guards-deep` |
| 30 | `scripts/ci_floor_check.py` | `ci-floor.event-gated`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 31 | `scripts/ci_floor_check.py` | `ci-floor.event-gated-accept`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 32 | `scripts/ci_floor_check.py` | `ci-floor.complementary-steps`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 33 | `scripts/ci_floor_check.py` | `ci-floor.missing-step`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 34 | `scripts/ci_floor_check.py` | `ci-floor.multi-command-split-steps`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 35 | `scripts/ci_floor_check.py` | `ci-floor.multi-command-reordered`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 36 | `scripts/ci_floor_check.py` | `ci-floor.multi-command-split-jobs`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 37 | `scripts/ci_floor_check.py` | `ci-floor.multi-command-named-in-area-job`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 38 | `scripts/ci_floor_check.py` | `ci-floor.non-floor-gate-not-run`, level 3 | must refuse, output carries `is excluded from the floor and needs no GPU` | floor | `bin/ocelli.sh gate guards` |
| 39 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-lists-disagree`, level 3 | must refuse, output carries `disagree about which gates the floor excludes` | floor | `bin/ocelli.sh gate guards` |
| 40 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-list-reordered`, level 3 | must ACCEPT, output carries `exclusion list and NOT_IN_FLOOR agree on` | floor | `bin/ocelli.sh gate guards` |
| 41 | `scripts/ci_floor_check.py` | `ci-floor.no-arm-command-gate-behind-a-condition`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 42 | `scripts/ci_floor_check.py` | `ci-floor.gate-with-an-unextractable-arm-command`, level 3 | must refuse, output carries `so it cannot demand those commands step by step` | floor | `bin/ocelli.sh gate guards` |
| 43 | `scripts/ci_floor_check.py` | `ci-floor.gate-with-no-arm-command`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 44 | `scripts/ci_floor_check.py` | `ci-floor.narrowed-arm-command`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 45 | `scripts/ci_floor_check.py` | `ci-floor.exclusion-list-unreadable`, level 3 | must refuse, output carries `exclusion list where this parser looks for it` | floor | `bin/ocelli.sh gate guards` |
| 46 | `scripts/ci_floor_check.py` | `ci-floor.run-gate-region-unreadable`, level 3 | must refuse, output carries `carries no `run_gate() {`` | floor | `bin/ocelli.sh gate guards` |
| 47 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-in-an-arm`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 48 | `scripts/ci_floor_check.py` | `ci-floor.no-arms-at-all`, level 3 | must refuse, output carries `declares no case arm this parser can read` | floor | `bin/ocelli.sh gate guards` |
| 49 | `scripts/ci_floor_check.py` | `ci-floor.work-inside-an-if`, level 3 | must refuse, output carries `so it cannot demand those commands step by step` | floor | `bin/ocelli.sh gate guards` |
| 50 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-after-a-keyword`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 51 | `scripts/ci_floor_check.py` | `ci-floor.widened-ci-step`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 52 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-after-a-bang`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 53 | `scripts/ci_floor_check.py` | `ci-floor.arm-comment-holding-a-terminator`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 54 | `scripts/ci_floor_check.py` | `ci-floor.work-behind-the-command-builtin`, level 3 | must refuse, output carries `so it cannot demand those commands step by step` | floor | `bin/ocelli.sh gate guards` |
| 55 | `scripts/ci_floor_check.py` | `ci-floor.lookup-with-the-command-builtin`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 56 | `scripts/ci_floor_check.py` | `ci-floor.arm-terminator-inside-a-quote`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 57 | `scripts/ci_floor_check.py` | `ci-floor.quoted-string-in-an-arm-is-permitted`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 58 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-after-while`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 59 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-after-if`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 60 | `scripts/ci_floor_check.py` | `ci-floor.nested-case-in-a-backtick`, level 3 | must refuse, output carries `holds a nested `case`` | floor | `bin/ocelli.sh gate guards` |
| 61 | `scripts/ci_floor_check.py` | `ci-floor.a-backtick-in-an-arm`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 62 | `scripts/ci_floor_check.py` | `ci-floor.unbalanced-quote-in-an-arm`, level 3 | must refuse, output carries `span is opened and never closed` | floor | `bin/ocelli.sh gate guards` |
| 63 | `scripts/ci_floor_check.py` | `ci-floor.comment-after-a-substitution`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 64 | `scripts/ci_floor_check.py` | `ci-floor.a-substitution-in-an-arm`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 65 | `scripts/ci_floor_check.py` | `ci-floor.gate-name-with-a-digit`, level 3 | must refuse, output carries `gate is in the CI floor` | floor | `bin/ocelli.sh gate guards` |
| 66 | `scripts/ci_floor_check.py` | `ci-floor.gate-name-outside-the-class`, level 3 | must refuse, output carries `whose name is outside` | floor | `bin/ocelli.sh gate guards` |
| 67 | `scripts/ci_floor_check.py` | `ci-floor.gate-name-with-a-digit-permitted`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 68 | `scripts/ci_floor_check.py` | `ci-floor.heredoc-delimiter-backslash-quoted`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 69 | `scripts/ci_floor_check.py` | `ci-floor.heredoc-delimiter-quoted-with-a-hyphen`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 70 | `scripts/ci_floor_check.py` | `ci-floor.heredoc-body-never-closed`, level 3 | must refuse, output carries `never meets its delimiter` | floor | `bin/ocelli.sh gate guards` |
| 71 | `scripts/ci_floor_check.py` | `ci-floor.heredoc-in-an-arm`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 72 | `scripts/ci_floor_check.py` | `ci-floor.continuation-after-an-arm-comment`, level 3 | must refuse, output carries `visible multi-command floor arm must be invoked by name` | floor | `bin/ocelli.sh gate guards` |
| 73 | `scripts/ci_floor_check.py` | `ci-floor.continuation-inside-an-arm-command`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 74 | `scripts/ci_floor_check.py` | `ci-floor.gates-entry-missing-a-field`, level 3 | must refuse, output carries `which is not `name|needs_gpu|description`` | floor | `bin/ocelli.sh gate guards` |
| 75 | `scripts/ci_floor_check.py` | `ci-floor.gates-gpu-column-unknown`, level 3 | must refuse, output carries `in the GPU column, which is neither` | floor | `bin/ocelli.sh gate guards` |
| 76 | `scripts/ci_floor_check.py` | `ci-floor.gates-array-unreadable`, level 3 | must refuse, output carries `carries no `GATES=(`` | floor | `bin/ocelli.sh gate guards` |
| 77 | `scripts/ci_floor_check.py` | `ci-floor.comment-inside-the-gates-array`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 78 | `scripts/ci_floor_check.py` | `ci-floor.comment-only`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 79 | `scripts/ci_floor_check.py` | `ci-floor.step-keys-in-the-other-order`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 80 | `scripts/ci_floor_check.py` | `ci-floor.quoted-if-key`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 81 | `scripts/ci_floor_check.py` | `ci-floor.flow-mapping-run-before-if`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 82 | `scripts/ci_floor_check.py` | `ci-floor.folded-run-scalar`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 83 | `scripts/ci_floor_check.py` | `ci-floor.quoted-event-key`, level 3 | must refuse, output carries `is behind a condition that does not run it on` | floor | `bin/ocelli.sh gate guards` |
| 84 | `scripts/ci_floor_check.py` | `ci-floor.gate-name-inside-a-run-string`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 85 | `scripts/ci_floor_check.py` | `ci-floor.run-key-under-with`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 86 | `scripts/ci_floor_check.py` | `ci-floor.workflow-unparseable`, level 3 | must refuse, output carries `cannot be parsed as YAML` | floor | `bin/ocelli.sh gate guards` |
| 87 | `scripts/ci_floor_check.py` | `ci-floor.run-that-is-not-a-scalar`, level 3 | must refuse, output carries `which this reader cannot use` | floor | `bin/ocelli.sh gate guards` |
| 88 | `scripts/ci_floor_check.py` | `ci-floor.quoted-run-scalar`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 89 | `scripts/ci_floor_check.py` | `ci-floor.block-scalar-indentation-indicator`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 90 | `scripts/ci_floor_check.py` | `ci-floor.flow-mapping-step-is-permitted`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 91 | `scripts/ci_floor_check.py` | `ci-floor.comment-after-an-if`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 92 | `scripts/ci_floor_check.py` | `ci-floor.on-as-a-block-sequence`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 93 | `scripts/ci_floor_check.py` | `ci-floor.background-operator-in-an-arm`, level 3 | must refuse, output carries `so it cannot demand those commands step by step` | floor | `bin/ocelli.sh gate guards` |
| 94 | `scripts/ci_floor_check.py` | `ci-floor.redirection-in-an-arm-is-permitted`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 95 | `scripts/ci_floor_check.py` | `ci-floor.gate-name-in-a-heredoc-body`, level 3 | must refuse, output carries `and nothing in` | floor | `bin/ocelli.sh gate guards` |
| 96 | `scripts/ci_floor_check.py` | `ci-floor.continued-ci-command-is-permitted`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 97 | `scripts/ci_floor_check.py` | `ci-floor.continue-on-error-on-a-gate-step`, level 3 | must refuse, output carries `because it is not guaranteed to run it and report its failure` | floor | `bin/ocelli.sh gate guards` |
| 98 | `scripts/ci_floor_check.py` | `ci-floor.continue-on-error-on-the-job`, level 3 | must refuse, output carries `because it is not guaranteed to run it and report its failure` | floor | `bin/ocelli.sh gate guards` |
| 99 | `scripts/ci_floor_check.py` | `ci-floor.gate-step-failure-swallowed`, level 3 | must refuse, output carries `because it is not guaranteed to run it and report its failure` | floor | `bin/ocelli.sh gate guards` |
| 100 | `scripts/ci_floor_check.py` | `ci-floor.gate-after-swallowed-and-condition`, level 3 | must refuse, output carries `an earlier `&&` means it runs only when the left-hand side succeeds` | floor | `bin/ocelli.sh gate guards` |
| 101 | `scripts/ci_floor_check.py` | `ci-floor.custom-shell-template-on-a-gate-step`, level 3 | must refuse, output carries `sets a `shell:` on step` | floor | `bin/ocelli.sh gate guards` |
| 102 | `scripts/ci_floor_check.py` | `ci-floor.measured-shell-on-a-gate-step`, level 3 | must ACCEPT, output carries `floor gate(s) are invoked by CI on` | floor | `bin/ocelli.sh gate guards` |
| 103 | `scripts/ci_floor_check.py` | `ci-floor.custom-shell-template-for-the-workflow`, level 3 | must refuse, output carries `sets `defaults.run.shell:` on the workflow` | floor | `bin/ocelli.sh gate guards` |
| 104 | `scripts/ci_floor_check.py` | `ci-floor.custom-shell-template-for-the-job`, level 3 | must refuse, output carries `sets `defaults.run.shell:` on the job` | floor | `bin/ocelli.sh gate guards` |
| 105 | `scripts/ci_floor_check.py` | `ci-floor.set-plus-e-before-a-gate-step`, level 3 | must refuse, output carries `a `set`` | floor | `bin/ocelli.sh gate guards` |
| 106 | `scripts/ci_floor_check.py` | `ci-floor.errexit-restored-before-a-gate-step`, level 3 | must refuse, output carries `a `set`` | floor | `bin/ocelli.sh gate guards` |
| 107 | `scripts/ci_floor_check.py` | `ci-floor.gate-inside-an-if-condition`, level 3 | must refuse, output carries `the shell keyword `if`` | floor | `bin/ocelli.sh gate guards` |
| 108 | `scripts/ci_floor_check.py` | `ci-floor.gate-inside-an-if-body`, level 3 | must refuse, output carries `the shell keyword `if`` | floor | `bin/ocelli.sh gate guards` |
| 109 | `scripts/ci_floor_check.py` | `ci-floor.gate-in-a-function-body`, level 3 | must refuse, output carries `carries a `(`` | floor | `bin/ocelli.sh gate guards` |
| 110 | `scripts/ci_floor_check.py` | `ci-floor.gate-in-an-unmatched-case-arm`, level 3 | must refuse, output carries `the shell keyword `case`` | floor | `bin/ocelli.sh gate guards` |
| 111 | `scripts/ci_floor_check.py` | `ci-floor.gate-after-a-top-level-exit`, level 3 | must refuse, output carries `a `exit`` | floor | `bin/ocelli.sh gate guards` |
| 112 | `scripts/ci_floor_check.py` | `ci-floor.shopt-unsets-errexit`, level 3 | must refuse, output carries `a `shopt`` | floor | `bin/ocelli.sh gate guards` |
| 113 | `scripts/ci_floor_check.py` | `ci-floor.scoped-set-e-does-not-restore`, level 3 | must refuse, output carries `a `set`` | floor | `bin/ocelli.sh gate guards` |
| 114 | `scripts/ci_floor_check.py` | `ci-floor.gate-in-a-case-arm-inside-a-loop`, level 3 | must refuse, output carries `the shell keyword `for`` | floor | `bin/ocelli.sh gate guards` |
| 115 | `scripts/ci_floor_check.py` | `ci-floor.gate-after-an-elif-chain`, level 3 | must refuse, output carries `the shell keyword `if`` | floor | `bin/ocelli.sh gate guards` |
| 116 | `scripts/ci_floor_check.py` | `ci-floor.negated-gate-step`, level 3 | must refuse, output carries `a `!` negation` | floor | `bin/ocelli.sh gate guards` |
| 117 | `scripts/ci_floor_check.py` | `ci-floor.unclosed-quote-in-a-ci-step`, level 3 | must refuse, output carries `opened and never closed` | floor | `bin/ocelli.sh gate guards` |
| 118 | `scripts/ci_floor_check.py` | `ci-floor.pyyaml-absent`, level 3 | must refuse, output carries `PyYAML is not installed` | floor | `bin/ocelli.sh gate guards` |
| 119 | `scripts/verify_ledger.py` | `ledger.no-record`, level 3 | must refuse, output carries `no verification recorded for the staged tree` | floor | `bin/ocelli.sh gate guards` |
| 120 | `scripts/verify_ledger.py` | `ledger.red-corpus`, level 3 | must refuse, output carries `the corpus is RED for tree` | floor | `bin/ocelli.sh gate guards` |
| 121 | `scripts/verify_ledger.py` | `ledger.require-corpus`, level 3 | must refuse, output carries `and this gate requires 'pass'` | floor | `bin/ocelli.sh gate guards` |
| 122 | `scripts/verify_ledger.py` | `ledger.bad-state`, level 3 | must refuse, output carries `--corpus must be one of` | floor | `bin/ocelli.sh gate guards` |
| 123 | `scripts/verify_ledger.py` | `ledger.trailer-silent`, level 3 | must refuse, output is SILENT | floor | `bin/ocelli.sh gate guards` |
| 124 | `scripts/verify_ledger.py` | `ledger.no-trailer`, level 3 | must refuse, output carries `carries no Ocelli-Verify trailer` | floor | `bin/ocelli.sh gate guards` |
| 125 | `scripts/verify_ledger.py` | `ledger.tree-mismatch`, level 3 | must refuse, output carries `but the commit's tree is` | floor | `bin/ocelli.sh gate guards` |
| 126 | `.githooks/pre-commit` | `hooks.pre-commit.dicom`, level 3 | must refuse, output carries `Commit refused` | floor | `bin/ocelli.sh gate guards` |
| 127 | `.githooks/commit-msg` | `hooks.commit-msg.forged-trailer`, level 3 | must refuse, output carries `written by this hook from the verify ledger` | floor | `bin/ocelli.sh gate guards` |
| 128 | `.githooks/commit-msg` | `hooks.commit-msg.agent-coauthor`, level 3 | must refuse, output carries `no agent co-author trailer` | floor | `bin/ocelli.sh gate guards` |
| 129 | `.githooks/commit-msg` | `hooks.commit-msg.prose`, level 3 | must refuse, output carries `semicolon in prose` | floor | `bin/ocelli.sh gate guards` |
| 130 | `.githooks/pre-push` | `hooks.pre-push.unverified`, level 3 | must refuse, output carries `Push refused` | floor | `bin/ocelli.sh gate guards` |
| 131 | `ci/check-bindgen-isolation.sh` | `bindgen.reaches`, level 3 | must refuse, output carries `reaches wasm-bindgen` | deep | `bin/ocelli.sh gate guards-deep` |
| 132 | `ci/check-bindgen-isolation.sh` | `bindgen.declares`, level 3 | must refuse, output carries `declares wasm-bindgen as a direct dependency` | deep | `bin/ocelli.sh gate guards-deep` |
| 133 | `ci/check-bindgen-isolation.sh` | `bindgen.in-source`, level 3 | must refuse, output carries `names wasm_bindgen in source` | deep | `bin/ocelli.sh gate guards-deep` |
| 134 | `ci/check-device-ownership.sh` | `device.creator`, level 3 | must refuse, output carries `creates a GPU device or surface` | floor | `bin/ocelli.sh gate guards` |
| 135 | `ci/check-device-ownership.sh` | `device.contract-gone`, level 3 | must refuse, output carries `no longer defines GpuContext` | floor | `bin/ocelli.sh gate guards` |
| 136 | `ci/check-device-ownership.sh` | `device.owned-accessor`, level 3 | must refuse, output carries `hands out an owned device` | floor | `bin/ocelli.sh gate guards` |
| 137 | `ci/check-device-ownership.sh` | `device.derives-clone`, level 3 | must refuse, output carries `derives Clone` | floor | `bin/ocelli.sh gate guards` |
| 138 | `scripts/backlog_check.py` | `backlog.done-without-record`, level 3 | must refuse, output carries `is done with no SPRINT_TRACKER.md row` | floor | `bin/ocelli.sh gate guards` |
| 139 | `scripts/backlog_check.py` | `backlog.bad-status`, level 3 | must refuse, output carries `expected one of` | floor | `bin/ocelli.sh gate guards` |
| 140 | `scripts/backlog_check.py` | `backlog.hook-out-of-phase`, level 3 | must refuse, output carries `not P1` | floor | `bin/ocelli.sh gate guards` |
| 141 | `scripts/gen_sprint_plan.py` | `sprint-plan.existing-refuses-write`, level 3 | must refuse, output carries `refuses to overwrite` | floor | `bin/ocelli.sh gate guards` |
| 142 | `scripts/gen_sprint_plan.py` | `sprint-plan.two-sprint-tables`, level 3 | must refuse, output carries `SPRINT_PLAN.md sprint tables` | floor | `bin/ocelli.sh gate guards` |
| 143 | `scripts/gen_sprint_plan.py` | `sprint-plan.milestone-summary-absent`, level 3 | must refuse, output carries `carries no summary line for it` | floor | `bin/ocelli.sh gate guards` |
| 144 | `scripts/gen_sprint_plan.py` | `sprint-plan.milestone-summary-spurious`, level 3 | must refuse, output carries `has no milestone spanning those sprints` | floor | `bin/ocelli.sh gate guards` |
| 145 | `scripts/gen_sprint_plan.py` | `sprint-plan.two-goal-lines`, level 3 | must refuse, output carries `**Goal** lines` | floor | `bin/ocelli.sh gate guards` |
| 146 | `scripts/gen_sprint_plan.py` | `sprint-plan.absent`, level 3 | must refuse, output carries `does not exist` | floor | `bin/ocelli.sh gate guards` |
| 147 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-sprint`, level 3 | must refuse, output carries `is in sprint` | floor | `bin/ocelli.sh gate guards` |
| 148 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-estimate`, level 3 | must refuse, output carries `is estimated` | floor | `bin/ocelli.sh gate guards` |
| 149 | `scripts/gen_sprint_plan.py` | `sprint-plan.wrong-milestone-summary`, level 3 | must refuse, output carries `milestone summary line` | floor | `bin/ocelli.sh gate guards` |
| 150 | `scripts/gen_sprint_plan.py` | `sprint-plan.stale-goal-line`, level 3 | must refuse, output carries `**Goal** line` | floor | `bin/ocelli.sh gate guards` |
| 151 | `scripts/sync_agent_skills.py` | `skills.stale-adapter`, level 3 | must refuse, output carries `is stale, its source changed` | floor | `bin/ocelli.sh gate guards` |
| 152 | `scripts/skill_examples_check.py` | `skill-examples.changed-expected-digit`, level 3 | must refuse, output carries `stdout differs` | floor | `bin/ocelli.sh gate guards` |
| 153 | `scripts/skill_examples_check.py` | `skill-examples.reversed-sigmoid-exponent`, level 3 | must refuse, output carries `stdout differs` | floor | `bin/ocelli.sh gate guards` |
| 154 | `scripts/skill_examples_check.py` | `skill-examples.reversed-sigmoid-width-precondition`, level 3 | must refuse, output carries `exited 1` | floor | `bin/ocelli.sh gate guards` |
| 155 | `scripts/sprint_workflow.py` | `handoff.wrong-branch`, level 3 | must refuse, output carries `does not start with` | floor | `bin/ocelli.sh gate guards` |
| 156 | `scripts/sprint_workflow.py` | `handoff.backticked-branch`, level 3 | must ACCEPT, output carries `validates` | floor | `bin/ocelli.sh gate guards` |
| 157 | `scripts/sprint_workflow.py` | `handoff.missing-files-touched`, level 3 | must refuse, output carries `handoff has no **Files touched** field` | floor | `bin/ocelli.sh gate guards` |
| 158 | `scripts/sprint_workflow.py` | `handoff.multiple-code-spans`, level 3 | must refuse, output carries `**Branch** value must be non-empty plain text or exactly one Markdown code span` | floor | `bin/ocelli.sh gate guards` |
| 159 | `scripts/sprint_workflow.py` | `handoff.unmatched-code-span`, level 3 | must refuse, output carries `**Files touched** value must be non-empty plain text or exactly one Markdown code span` | floor | `bin/ocelli.sh gate guards` |
| 160 | `scripts/sprint_workflow.py` | `handoff.embedded-code-span`, level 3 | must refuse, output carries `**Head** value must be non-empty plain text or exactly one Markdown code span` | floor | `bin/ocelli.sh gate guards` |
| 161 | `scripts/sprint_workflow.py` | `handoff.empty-code-span`, level 3 | must refuse, output carries `**Review** value must be non-empty plain text or exactly one Markdown code span` | floor | `bin/ocelli.sh gate guards` |
| 162 | `scripts/sprint_workflow.py` | `handoff.forged-suffix`, level 3 | must refuse, output carries `**Base** value must be non-empty plain text or exactly one Markdown code span` | floor | `bin/ocelli.sh gate guards` |
| 163 | `scripts/sprint_workflow.py` | `handoff.duplicate-head`, level 3 | must refuse, output carries `handoff has duplicate **Head** fields` | floor | `bin/ocelli.sh gate guards` |
| 164 | `scripts/sprint_workflow.py` | `sprint-lifecycle.not-in-sprint`, level 3 | must refuse, output carries `is not in sprint` | floor | `bin/ocelli.sh gate guards` |
| 165 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-legacy-state`, level 3 | must refuse, output carries `no sprint-scope review recorded` | floor | `bin/ocelli.sh gate guards` |
| 166 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-dirty-review`, level 3 | must refuse, output carries `latest sprint review pass 2 reports 1 defects and 0 smells` | floor | `bin/ocelli.sh gate guards` |
| 167 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-stale-review`, level 3 | must refuse, output carries `latest sprint review tree 000000000000 is stale` | floor | `bin/ocelli.sh gate guards` |
| 168 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-stale-verification`, level 3 | must refuse, output carries `latest sprint-profile verification tree 000000000000 is stale` | floor | `bin/ocelli.sh gate guards` |
| 169 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-latest-verification-failed`, level 3 | must refuse, output carries `latest sprint-profile verification did not pass` | floor | `bin/ocelli.sh gate guards` |
| 170 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-tree-changed`, level 3 | must refuse, output carries `latest sprint review tree` | floor | `bin/ocelli.sh gate guards` |
| 171 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-unrecorded-carry`, level 3 | must refuse, output carries `is carried but has no recorded carry-forward reason` | floor | `bin/ocelli.sh gate guards` |
| 172 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-recorded-carry`, level 3 | must ACCEPT, output carries `is ready to close` | floor | `bin/ocelli.sh gate guards` |
| 173 | `scripts/sprint_workflow.py` | `sprint-lifecycle.close-current-evidence`, level 3 | must ACCEPT, output carries `is ready to close` | floor | `bin/ocelli.sh gate guards` |
| 174 | `scripts/error_code_check.py` | `errors.renumbered`, level 3 | must refuse, output carries `so this is a renumbering` | floor | `bin/ocelli.sh gate guards` |
| 175 | `scripts/bench_check.py` | none in this harness | A benchmark registry whose subject stories do not resolve, a subject whose story has not landed carrying a runner or a recorded number, and a playwright pin that has drifted between the two harnesses. | - | scripts/tests/test_bench_check.py (25 cases, run by the `bench` gate, which is in the floor) |
| 176 | `scripts/corpus_check.py` | `corpus.digest-mismatch`, level 3 | must refuse, output carries `does not match its manifest digest` | floor | `bin/ocelli.sh gate guards` |
| 177 | `scripts/corpus_check.py` | `corpus.absent`, level 3 | must refuse, output carries `corpus cases are absent` | floor | `bin/ocelli.sh gate guards` |
| 178 | `scripts/corpus_check.py` | `corpus.unrecorded-licence`, level 3 | must refuse, output carries `cannot be redistributed or cited` | floor | `bin/ocelli.sh gate guards` |
| 179 | `scripts/corpus_tests.py` | `corpus-tests.skip-is-not-a-pass`, level 3 | must refuse, output carries `FAIL: a prerequisite` | floor | `bin/ocelli.sh gate guards` |
| 180 | `scripts/corpus_synth.py` | none in this harness | A manifest whose header is not the recorded column set. | - | scripts/tests/test_corpus_synth.py (run by the `corpus-tests` gate) |
| 181 | `scripts/target_feature_check.py` | `target-features.cannot-run`, level 3 | must refuse, output carries `could not run` | deep | `bin/ocelli.sh gate guards-deep` |
| 182 | `scripts/package_check.py` | `packages.exports-not-in-tarball`, level 1 | must refuse, output carries `advertises` | floor | `bin/ocelli.sh gate guards` |
| 183 | `scripts/package_check.py` | `packages.version-skew`, level 1 | must refuse, output carries `the Rust workspace is` | floor | `bin/ocelli.sh gate guards` |
| 184 | `bin/ocelli.sh` | `runner.absent-prerequisite`, level 3 | must refuse, output carries `reference stack is not installed` | floor | `bin/ocelli.sh gate guards` |
| 185 | `scripts/source_dir.py` | `source-dir.unconfigured`, level 3 | must refuse, output carries `are not configured` | floor | `bin/ocelli.sh gate guards` |
| 186 | `scripts/split_hld.py` | `split-hld.cannot-run`, level 3 | must refuse, output carries `A check that cannot run is NOT a check that` | floor | `bin/ocelli.sh gate guards` |
| 187 | `scripts/lint_policy_check.py` | `lint-policy.weakened`, level 3 | must refuse, output carries `is 'allow' and HLD 27.1 requires` | deep | `bin/ocelli.sh gate guards-deep` |
| 188 | `scripts/lint_policy_check.py` | `lint-policy.uninherited`, level 3 | must refuse, output carries `does not inherit the workspace lint table` | deep | `bin/ocelli.sh gate guards-deep` |
| 189 | `scripts/lint_policy_check.py` | `lint-policy.group-allow`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 190 | `scripts/lint_policy_check.py` | `lint-policy.expect-attribute`, level 3 | must refuse, output carries `re-allows` | deep | `bin/ocelli.sh gate guards-deep` |
| 191 | `scripts/lint_policy_check.py` | `lint-policy.manifest-not-utf8`, level 3 | must refuse, output carries `cannot be read as UTF-8 text` | floor | `bin/ocelli.sh gate guards` |
| 192 | `scripts/lint_policy_check.py` | `lint-policy.nothing-scanned`, level 3 | must refuse, output carries `not one `.rs` file was read` | deep | `bin/ocelli.sh gate guards-deep` |
| 193 | `scripts/lint_policy_check.py` | `lint-policy.allow-outside-the-crate-root`, level 3 | must refuse, output carries `re-allows `cast_possible_truncation`` | deep | `bin/ocelli.sh gate guards-deep` |
| 194 | `scripts/lint_policy_check.py` | `lint-policy.item-allow-is-permitted`, level 3 | must ACCEPT, output carries `carry no inner allow or expect of a denied lint` | deep | `bin/ocelli.sh gate guards-deep` |
| 195 | `scripts/lint_policy_check.py` | `lint-policy.outer-allow-on-a-module`, level 3 | must refuse, output carries `an outer attribute on a `mod` item covers the whole module tree` | deep | `bin/ocelli.sh gate guards-deep` |
| 196 | `scripts/lint_policy_check.py` | `lint-policy.whitespace-in-the-lint-path`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 197 | `scripts/lint_policy_check.py` | `lint-policy.member-outside-crates-uninherited`, level 3 | must refuse, output carries `does not inherit the workspace lint table` | deep | `bin/ocelli.sh gate guards-deep` |
| 198 | `scripts/lint_policy_check.py` | `lint-policy.member-outside-crates-group-allow`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 199 | `scripts/lint_policy_check.py` | `lint-policy.member-unresolvable`, level 3 | must refuse, output carries `resolves to no directory carrying a Cargo.toml` | deep | `bin/ocelli.sh gate guards-deep` |
| 200 | `scripts/lint_policy_check.py` | `lint-policy.no-members-declared`, level 3 | must refuse, output carries `declares no `members` this parser can read` | deep | `bin/ocelli.sh gate guards-deep` |
| 201 | `scripts/lint_policy_check.py` | `lint-policy.group-row-in-the-workspace-table`, level 3 | must refuse, output carries `carries the lint GROUP` | deep | `bin/ocelli.sh gate guards-deep` |
| 202 | `scripts/lint_policy_check.py` | `lint-policy.group-row-with-a-trailing-comment`, level 3 | must refuse, output carries `carries the lint GROUP` | deep | `bin/ocelli.sh gate guards-deep` |
| 203 | `scripts/lint_policy_check.py` | `lint-policy.quoted-group-row`, level 3 | must refuse, output carries `carries the lint GROUP` | deep | `bin/ocelli.sh gate guards-deep` |
| 204 | `scripts/lint_policy_check.py` | `lint-policy.two-line-group-row`, level 3 | must refuse, output carries `cannot be parsed as TOML` | deep | `bin/ocelli.sh gate guards-deep` |
| 205 | `scripts/lint_policy_check.py` | `lint-policy.unparseable-member-manifest`, level 3 | must refuse, output carries `is a workspace member cargo reports and its Cargo.toml cannot be parsed as TOML` | deep | `bin/ocelli.sh gate guards-deep` |
| 206 | `scripts/lint_policy_check.py` | `lint-policy.quoted-required-row-is-permitted`, level 3 | must ACCEPT, output carries `clippy lint(s) at or above HLD 27.1's level` | deep | `bin/ocelli.sh gate guards-deep` |
| 207 | `scripts/lint_policy_check.py` | `lint-policy.dotted-required-row-is-permitted`, level 3 | must ACCEPT, output carries `clippy lint(s) at or above HLD 27.1's level` | deep | `bin/ocelli.sh gate guards-deep` |
| 208 | `scripts/lint_policy_check.py` | `lint-policy.commented-required-row-is-permitted`, level 3 | must ACCEPT, output carries `clippy lint(s) at or above HLD 27.1's level` | deep | `bin/ocelli.sh gate guards-deep` |
| 209 | `scripts/lint_policy_check.py` | `lint-policy.excluded-named-member`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 210 | `scripts/lint_policy_check.py` | `lint-policy.comment-in-the-lint-path`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 211 | `scripts/lint_policy_check.py` | `lint-policy.unreadable-lint-argument`, level 3 | must refuse, output carries `could not read to its end` | deep | `bin/ocelli.sh gate guards-deep` |
| 212 | `scripts/lint_policy_check.py` | `lint-policy.rustflags-allow`, level 3 | must refuse, output carries `in `rustflags`` | deep | `bin/ocelli.sh gate guards-deep` |
| 213 | `scripts/lint_policy_check.py` | `lint-policy.cargo-config-is-permitted`, level 3 | must ACCEPT, output carries `cargo config(s) lower no denied lint through rustflags` | deep | `bin/ocelli.sh gate guards-deep` |
| 214 | `scripts/lint_policy_check.py` | `lint-policy.path-dependency-member`, level 3 | must refuse, output carries `does not inherit the workspace lint table` | deep | `bin/ocelli.sh gate guards-deep` |
| 215 | `scripts/lint_policy_check.py` | `lint-policy.module-outside-the-member`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 216 | `scripts/lint_policy_check.py` | `lint-policy.clean-module-outside-the-member`, level 3 | must ACCEPT, output carries `1 `#[path]` module(s) followed` | deep | `bin/ocelli.sh gate guards-deep` |
| 217 | `scripts/lint_policy_check.py` | `lint-policy.cfg-attr-module-path`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 218 | `scripts/lint_policy_check.py` | `lint-policy.raw-string-module-path`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 219 | `scripts/lint_policy_check.py` | `lint-policy.clean-cfg-attr-module-path`, level 3 | must ACCEPT, output carries `1 `#[path]` module(s) followed` | deep | `bin/ocelli.sh gate guards-deep` |
| 220 | `scripts/lint_policy_check.py` | `lint-policy.required-row-with-a-tail`, level 3 | must refuse, output carries `cannot be parsed as TOML` | deep | `bin/ocelli.sh gate guards-deep` |
| 221 | `scripts/lint_policy_check.py` | `lint-policy.crate-root-outside-the-member`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 222 | `scripts/lint_policy_check.py` | `lint-policy.clean-crate-root-outside-the-member`, level 3 | must ACCEPT, output carries `cargo target root(s) seeded` | deep | `bin/ocelli.sh gate guards-deep` |
| 223 | `scripts/lint_policy_check.py` | `lint-policy.unreadable-crate-root`, level 3 | must refuse, output carries `as a compilation root of the workspace member` | deep | `bin/ocelli.sh gate guards-deep` |
| 224 | `scripts/lint_policy_check.py` | `lint-policy.cap-lints-allow`, level 3 | must refuse, output carries `caps EVERY lint` | deep | `bin/ocelli.sh gate guards-deep` |
| 225 | `scripts/lint_policy_check.py` | `lint-policy.cap-lints-warn`, level 3 | must refuse, output carries `caps EVERY lint` | deep | `bin/ocelli.sh gate guards-deep` |
| 226 | `scripts/lint_policy_check.py` | `lint-policy.cap-lints-deny-is-permitted`, level 3 | must ACCEPT, output carries `cargo config(s) lower no denied lint through rustflags` | deep | `bin/ocelli.sh gate guards-deep` |
| 227 | `scripts/lint_policy_check.py` | `lint-policy.force-warn-a-denied-lint`, level 3 | must refuse, output carries `in `rustflags`` | deep | `bin/ocelli.sh gate guards-deep` |
| 228 | `scripts/lint_policy_check.py` | `lint-policy.deny-a-group-is-permitted`, level 3 | must ACCEPT, output carries `cargo config(s) lower no denied lint through rustflags` | deep | `bin/ocelli.sh gate guards-deep` |
| 229 | `scripts/lint_policy_check.py` | `lint-policy.empty-rustflags-is-permitted`, level 3 | must ACCEPT, output carries `cargo config(s) lower no denied lint through rustflags` | deep | `bin/ocelli.sh gate guards-deep` |
| 230 | `scripts/lint_policy_check.py` | `lint-policy.dotted-lints-inheritance-is-permitted`, level 3 | must ACCEPT, output carries `workspace member(s) from` | deep | `bin/ocelli.sh gate guards-deep` |
| 231 | `scripts/lint_policy_check.py` | `lint-policy.include-macro`, level 3 | must refuse, output carries `allows the lint group` | deep | `bin/ocelli.sh gate guards-deep` |
| 232 | `scripts/lint_policy_check.py` | `lint-policy.clean-include-macro`, level 3 | must ACCEPT, output carries `1 `include!`(s) followed` | deep | `bin/ocelli.sh gate guards-deep` |
| 233 | `scripts/lint_policy_check.py` | `lint-policy.unresolvable-include-macro`, level 3 | must refuse, output carries `did not read the source it pastes in` | deep | `bin/ocelli.sh gate guards-deep` |
| 234 | `scripts/lint_policy_check.py` | `lint-policy.computed-include-macro`, level 3 | must refuse, output carries `could not read a file name out of it` | deep | `bin/ocelli.sh gate guards-deep` |
| 235 | `scripts/lint_policy_check.py` | `lint-policy.unreadable-member-source`, level 3 | must refuse, output carries `is reached by this check's walk of the workspace member` | deep | `bin/ocelli.sh gate guards-deep` |
| 236 | `scripts/lint_policy_check.py` | `lint-policy.dotted-rustflags-key`, level 3 | must refuse, output carries `in `rustflags`` | deep | `bin/ocelli.sh gate guards-deep` |
| 237 | `scripts/lint_policy_check.py` | `lint-policy.quoted-rustflags-key`, level 3 | must refuse, output carries `in `rustflags`` | deep | `bin/ocelli.sh gate guards-deep` |
| 238 | `scripts/lint_policy_check.py` | `lint-policy.cargo-config-unparseable`, level 3 | must refuse, output carries `does not parse as TOML` | deep | `bin/ocelli.sh gate guards-deep` |
| 239 | `scripts/lint_policy_check.py` | `lint-policy.dotted-cargo-config-is-permitted`, level 3 | must ACCEPT, output carries `cargo config(s) lower no denied lint through rustflags` | deep | `bin/ocelli.sh gate guards-deep` |
| 240 | `scripts/lint_policy_check.py` | `lint-policy.runner-not-utf8`, level 3 | must refuse, output carries `Whether the `unsafe` gate still runs` | deep | `bin/ocelli.sh gate guards-deep` |
| 241 | `scripts/lint_policy_check.py` | `lint-policy.runner-arms-unreadable`, level 3 | must refuse, output carries `cannot be read for its gate arms` | deep | `bin/ocelli.sh gate guards-deep` |
| 242 | `scripts/lint_policy_check.py` | `lint-policy.unsafe-gate-named-only-in-a-comment`, level 3 | must refuse, output carries `neither mechanism is present` | deep | `bin/ocelli.sh gate guards-deep` |
| 243 | `scripts/guards/census.py` | `census.constants-count-shrunk`, level 3 | must refuse, output carries `Narrowing the declared-constant ratchet` | floor | `bin/ocelli.sh gate guards` |
| 244 | `scripts/guards/census.py` | `census.impossible-wall-clock-pair`, level 3 | must refuse, output carries `cannot be the faster of the two` | floor | `bin/ocelli.sh gate guards` |
| 245 | `scripts/guards/census.py` | `census.unclaimed-executable-hook`, level 3 | must refuse, output carries `is executable in a clone that opts in` | floor | `bin/ocelli.sh gate guards` |
| 246 | `scripts/guards/census.py` | `census.oracle-runner-gone`, level 3 | must refuse, output carries `so the `oracle` gate has no runner` | floor | `bin/ocelli.sh gate guards` |
| 247 | `scripts/guards/census.py` | `census.unrecognised-kind`, level 3 | must refuse, output carries `which is not one of` | floor | `bin/ocelli.sh gate guards` |
| 248 | `scripts/guards/census.py` | `census.no-entry-site-count`, level 3 | must refuse, output carries `records no per-entry site count` | floor | `bin/ocelli.sh gate guards` |
| 249 | `scripts/guards/census.py` | `census.refusal-in-a-claimed-file`, level 3 | must refuse, output carries `refusal site(s) to` | floor | `bin/ocelli.sh gate guards` |
| 250 | `scripts/guards/census.py` | `census.unclaimed-site`, level 3 | must refuse, output carries `no catalogue entry claims` | floor | `bin/ocelli.sh gate guards` |
| 251 | `scripts/guards/census.py` | `census.changed-constant`, level 3 | must refuse, output carries `changed without its recorded value` | floor | `bin/ocelli.sh gate guards` |
| 252 | `scripts/guards/census.py` | `census.no-std-set-shrunk`, level 3 | must refuse, output carries `changed without its recorded value` | floor | `bin/ocelli.sh gate guards` |
| 253 | `scripts/guards/census.py` | `census.uncovered-grew`, level 3 | must refuse, output carries `The ratchet may only decrease` | floor | `bin/ocelli.sh gate guards` |
| 254 | `scripts/guards/census.py` | `census.no-ceiling`, level 3 | must refuse, output carries `records no uncovered ceiling` | floor | `bin/ocelli.sh gate guards` |
| 255 | `scripts/guards/census.py` | `census.orphan-recorded-constant`, level 3 | must refuse, output carries `is not declared in the catalogue's CONSTANTS` | floor | `bin/ocelli.sh gate guards` |
| 256 | `scripts/guards/census.py` | `census.gate-without-an-entry`, level 3 | must refuse, output carries `has no catalogue entry and no `delegated` reason` | floor | `bin/ocelli.sh gate guards` |
| 257 | `scripts/guards/census.py` | `census.gate-name-with-a-digit`, level 3 | must refuse, output carries `has no catalogue entry and no `delegated` reason` | floor | `bin/ocelli.sh gate guards` |
| 258 | `scripts/guards/census.py` | `census.floor-needing-a-gpu`, level 1 | must refuse, output carries `no GPU, no browser and no corpus` | floor | `bin/ocelli.sh gate guards` |
| 259 | `scripts/guard_census.py` | `census-runbook.markers-gone`, level 3 | must refuse, output carries `carries no generated-table markers` | floor | `bin/ocelli.sh gate guards` |
| 260 | `scripts/guard_probe.py` | `probe-runner.self-test`, level 3 | must ACCEPT, output carries `OK` | floor | `bin/ocelli.sh gate guards` |
| 261 | `scripts/guard_probe.py` | `probe-runner.a-guard-that-refuses-nothing`, level 3 | must ACCEPT, output carries `did not fire, so it is a guard nobody has watched fail` | floor | `bin/ocelli.sh gate guards` |
| 262 | `scripts/guards/sandbox.py` | none in this harness | A git call outside the sandbox, a git call in a directory this harness did not create, and the `rm --cached` and `checkout --` pair the runbook records as a false-green trap. | - | scripts/guard_probe.py --self-test; scripts/tests/test_guard_catalogue.py |
| 263 | `scripts/guards/discover.py` | none in this harness | Nothing on its own. It is the scanner the census refuses from. | - | scripts/tests/test_guard_catalogue.py |
| 264 | `scripts/guards/catalogue.py` | none in this harness | A probe builder that mutated nothing, and a fixture drawn from a citation that has gone away. | - | scripts/tests/test_guard_catalogue.py |
| 265 | `tools/oracle/run.mjs` | none in this harness | A run that reached no row, a decode that produced nothing, a frame that never presented, a read-back still showing the sentinel, a volume that did not load, and the environment refusals that keep the rasteriser honest. | - | tools/oracle/src/faults.mjs (23 injected faults, replayed by tools/oracle/tests/faults.mjs on every `oracle` gate); tools/oracle/tests/args_test.mjs; tools/oracle/tests/paths_test.mjs; tools/oracle/tests/pins_test.mjs |
| 266 | `tools/oracle/page/app.mjs` | none in this harness | A render that did not present, a read-back that is not comparable, and a frame the page cannot attribute. | - | tools/oracle/src/faults.mjs |
| 267 | `tools/oracle/page/volume.mjs` | none in this harness | A volume that did not load, a geometry that is not a permutation of the declared one, and a reformat that never presented. | - | tools/oracle/src/faults.mjs |
| 268 | `tools/oracle/src/volume.mjs` | none in this harness | A volume subject whose members disagree, a spacing or orientation that does not resolve, and a declared truth the reference does not reproduce. | - | tools/oracle/tests/volume_test.mjs (run inside `bin/ocelli.sh oracle`'s unit pass) |
| 269 | `tools/oracle/src/manifest.mjs` | none in this harness | A manifest the reference half cannot read, and a row it cannot resolve to a case. | - | tools/oracle/tests/manifest_test.mjs |
| 270 | `tools/oracle/src/geometry.mjs` | none in this harness | A geometry the reference cannot express, and one that does not round-trip. | - | tools/oracle/tests/geometry_test.mjs |
| 271 | `tools/oracle/src/voi.mjs` | none in this harness | A VOI declaration the reference cannot apply, and a window the row does not carry. | - | tools/oracle/tests/params_test.mjs |
| 272 | `tools/oracle/src/params.mjs` | none in this harness | A render parameter the page does not implement, and a parameter set that does not resolve for a row. | - | tools/oracle/tests/params_test.mjs |
| 273 | `tools/oracle/src/unsupported.mjs` | none in this harness | A row declared unsupported that renders, and a row that fails without a declaration. | - | tools/oracle/tests/unsupported_test.mjs |
| 274 | `tools/oracle/src/output.mjs` | none in this harness | An output directory that is not the harness's own. | - | tools/oracle/tests/output_test.mjs |
| 275 | `tools/oracle/src/pins.mjs` | none in this harness | A reference stack installed at a version nobody pinned. | - | tools/oracle/tests/pins_test.mjs |
| 276 | `tools/oracle/src/sidecar.mjs` | none in this harness | A sidecar the comparator cannot read. | - | tools/oracle/tests/sidecar_test.mjs |
| 277 | `tools/oracle/check_sidecars.py` | none in this harness | A cross-read mismatch reported without redacting a real row's values, and a sidecar pydicom and the reference disagree about. | - | tools/oracle/check_sidecars.py --self-test, run by tools/oracle/run.mjs under both interpreters |
| 278 | `tools/oracle/src/faults.mjs` | none in this harness | A fault name nothing declares, and a filter that selected nothing reading as success. | - | tools/oracle/tests/faults.mjs |
| 279 | `tools/bench/src/registry.mjs` | none in this harness | A registry entry with no subject story, a duplicate subject, and a runner for a subject whose story has not landed. | - | tools/bench/tests/registry_test.mjs (run by the `bench` gate) |
| 280 | `tools/bench/src/record.mjs` | none in this harness | A record written against a host class it was not measured on, and a malformed baseline. | - | tools/bench/tests/record_test.mjs; tools/bench/tests/hostclass_test.mjs |
| 281 | `tools/bench/src/state.mjs` | none in this harness | A run state the harness cannot resume from. | - | tools/bench/tests/state_test.mjs |
| 282 | `tools/bench/run.mjs` | none in this harness | An argument the harness does not accept, and a runner for a subject whose story is not done. | - | tools/bench/tests/run_test.mjs (run by the `bench` gate) |
| 283 | `tools/bench/src/runners/wasm_cold_start.mjs` | none in this harness | A cold-start measurement taken against a stub, an incomplete artefact copy, and a page that never reported. | - | tools/bench/tests/cold_start_test.mjs |
| 284 | `tools/bench/page/app.mjs` | none in this harness | A page serving an incomplete copy of the wasm artefact, and a mark count that does not match the phase list. | - | tools/bench/tests/cold_start_test.mjs |
| 285 | `scripts/panic_probe.mjs` | none in this harness | A run that measured the probe's stub rather than the module. | - | bin/ocelli.sh gate panic, which builds a second module carrying the panic-probe feature and runs this file on every floor gate |

Known defects this table names, in full:


What a probe above does NOT reach, declared rather than left to be discovered:

- **`scripts/source_provenance_check.py`.** The URL clause has no probe, deliberately. `docs/SOURCE-POLICY.md` names the projects and does not name their addresses, so a probe would have to take its input from the guard's own URL list, which is the R2 failure this catalogue exists to avoid: it would assert the current regex and pass forever once the regex was weakened. The list itself is a declared constant in the ratchet, so a change to it is caught there and lands in front of a reviewer.
- **`scripts/ci_floor_check.py`.** The non-floor rule exempts a gate `bin/ocelli.sh` marks YES in its GPU column, which is `oracle` and deviation D-04's reason for it, and that column is not in the declared-constant ratchet. Marking `guards-deep|YES|` would therefore exempt it without this check noticing. It is left as a limit rather than recorded, because the column is a semantic claim in the runner's own gate table where a false entry reads as false to a person, and because `NOT_IN_FLOOR` and the runner's exclusion list are both watched, so the OTHER routes out of the floor are closed. The second limit is that this file cannot evaluate `github.ref`, so a step behind a condition naming a branch counts on no event and the OK line prints the events it PROVED rather than the events that can happen. No step in `.github/workflows/ci.yml` sits behind such a condition today, so the limit currently drops nothing, and this sentence says so rather than naming a job. It named one until the S03 review's tenth pass, and the job had been deleted a pass earlier: it said what this file proves about `guards-deep` is that CI runs it on `workflow_dispatch`, and that the push-to-main half is unproven, while the check prints `pull_request, push, workflow_dispatch` for that gate because the step moved into the unconditional `guards` job. Both halves were false, in a declared limit, which this project treats as load-bearing. The third limit is the arm-command extractor's own vocabulary. It recognises `python3 `, `npm run `, `cargo ` and `ci/`, so `node`, `wasm-pack` and `"$0"` are invisible and the check cannot demand those commands of CI step by step. That was declared as a limit and taken on trust until the S03 review's sixth pass measured it open: replacing `- run: bin/ocelli.sh gate bench` with its two extractable commands left the check at exit 0 and five node test files out of CI. The vocabulary is unchanged and the CONSEQUENCE is now a rule: `unseen_commands` reports what the extractor cannot see and a gate holding one of those is refused unless a step invokes it by name, per event. So the remaining limit is only that the refusal names the gate rather than the command, and the `all([])` claim still holds, because `panic`, `native` and `oracle` still yield no extractable command and `ci-floor.no-arm-command-gate-behind-a-condition` is what watches `bool(arm)`. The fourth limit STATED THE WRONG CONSEQUENCE until the seventh pass. It said a statement whose first word is in `SHELL_NOISE` is not the work, so an arm that did its work inside an `if` or a `for` "would be read as having none", and that is true only when the WHOLE arm is inside the `if`. The measured shape is narrower and worse: `if node --test ...; then true; fi` is one statement whose head is a keyword, so dropping the head dropped the command and left every OTHER command in the arm visible, which is a gate that reads as fully covered with one command gone. The head is dropped and the remainder RE-SCANNED now, `SHELL_INTRODUCERS` and `SHELL_NOISE` are the two halves of that split, and `ci-floor.work-inside-an-if` watches it. What was left as a limit was the split itself, that a builtin's remainder is treated as arguments, so a command hidden after `command` or `eval` in a form this file does not model would still be invisible. `command` was HALF of that named pair and it was live: MEASURED in the eighth pass, rewriting the `bench` arm's `node --test` line as `command node --test ...` gave `unseen bench: None` and exit 0 with the gate-name step expanded, five node suites out of CI. It has a reader now, `command_builtin_runs`, which treats `-v` and `-V` as a lookup, steps over `-p` and `--`, re-scans what is left and fails CLOSED on an option it does not model. `ci-floor.work-behind-the-command-builtin` and `ci-floor.lookup-with-the-command-builtin` watch both directions. What remains of the split is any OTHER head whose remainder is really a command, and neither `eval` nor `command` is one of them any more. The fifth limit is the arm parser's own fail-opens, and the eighth pass called them two when two was not the count. FIVE are closed and probed now. `NESTED_CASE` carried a `\b` in front of an alternation containing `!`, which needs a word character before it, so `&& ! case` matched nothing. `SHELL_COMMENT` ran AFTER `ARM` had matched, so a `;;` inside a shell comment ended the arm before the comment was stripped. The ninth pass measured three more, each dropping a real trailing command at exit 0 with `sh -n` accepting the file: a `;;` inside a QUOTED STRING, which no amount of stripping reaches, and `while case` and `if case`, which are statement positions `SHELL_INTRODUCERS` already knew about and `NESTED_CASE`'s hand-written `then|do|else|elif|!` did not. The alternation is DERIVED from `SHELL_INTRODUCERS` now, so the two cannot drift again. **The claim that the arm's end, the statement split and the comment strip all use one scan was half true until the S03 review's eleventh pass.** They shared the rule for OPENING a span and the delimiter set. The rule for CLOSING one was written out three times and agreed only because all three were edited in one commit, which is the condition that claim says had been removed. There is one tokenizer now, `ci_floor_check.shell_pieces`, with a span STACK, and its callers consume pieces rather than reimplementing a close loop. It covers the single quote, the double quote, the backtick, a nesting-counted `$( ... )` and `${ ... }`, a backslash escape, a here-document body and the comment itself, so a `#` that opens a comment and a quote that opens a span are decided by one rule rather than by an ordering between two passes. `ci_floor_check.gate_entries` reads the GATES array with the same scanner, which is what removed the second copy of the gate-row regex from `scripts/guards/census.py`. **The seventh limit is what the tokenizer still does not model, and it was written as an ENUMERATION of two constructs until the S03 review's twelfth pass, which measured the enumeration wrong.** It named `$'...'` and a substitution inside a double quote and said the residue was those two. It was not. A here-document body was claimed as covered and was covered for a DELIMITER SUBSET only, which was where the twelfth fail-open lived. Process substitution was not modelled and not mentioned, and a regex pre-pass over line continuations ran BEFORE the tokenizer and appeared nowhere. An enumeration of a grammar's constructs is not a limit, it is a claim that the author thought of all of them, and twelve passes say that shape does not hold. So the limit is stated as what the reader CANNOT SEE and why. **It was then stated as ONE thing, that the scanner does not model compound commands, and one thing was not the count either: the S03 review's thirteenth pass measured the fourteenth route in a production the sentence did not mention.** That claim is accurate about `shell_pieces` and `shell_pieces` is not the whole shell reader. It is ONE tokenizer with FOUR hand-written productions on top of it, `STATEMENT_BREAK`, `NESTED_CASE`, the `SHELL_INTRODUCERS`/`SHELL_NOISE` split and `ARM_LABEL`/`GATE_INVOCATION`, and the route lived in the first. "One tokenizer" was achieved and "one reader" was not, and the limit read as if it had been. Per production, then. THE TOKENIZER models spans and words and does not model bash's COMPOUND COMMANDS, so it cannot tell a `(` that opens a subshell from one that ends a `case` pattern, and it cannot tell `((` arithmetic from `( (` nested subshells the way bash does, which is by attempting the arithmetic parse and backtracking. `$(case y in *) ... esac)` therefore closes at the pattern's `)` and the arm ends at the inner `;;`, which `NESTED_CASE` refuses and `scripts/tests/test_guard_readers.py` asserts is the refusal carrying the weight rather than the scan. `(( a << b ))` reads the `<<` as a here-document whose body then runs to the end of the region, which refuses, and a here-document written inside a substitution is not queued at all, its body being inside the same span. `$'...'` is a span so its EXTENT is right and its C escapes are not decoded, which lands a name outside `GATE_NAME` and refuses, and a continuation INSIDE a double quote is left in place, which reaches `runs_command`'s text comparison and refuses. THE STATEMENT SCANNER, `STATEMENT_BREAK` and `_split_statements`, sees the control operators and nothing else about a list. MEASURED: `coproc case x in *) : ;; esac` is accepted by `bash -n`, is MISSED by `NESTED_CASE`, and fails closed only because the statement scanner reports `coproc case x in *` as a command CI does not run, so the refusal that carries the weight there is not the one the compound-command sentence names. `_tolerated_statements`, which decides whose failure `bash -e` discards, reads the same separators and inherits every one of these blind spots. `NESTED_CASE` derives its alternation from `SHELL_INTRODUCERS`, so `time case` matches and `coproc case` does not, measured both ways. The `SHELL_INTRODUCERS`/`SHELL_NOISE` split is a list rather than a grammar, and what remains of it is any head other than `eval` and `command`, each of which was measured on the wrong side and each of which has a reader now. `ARM_LABEL` and `GATE_INVOCATION` both anchor at a statement head, so both inherit the statement scanner's residue rather than adding one. Every consequence above is fail-closed and each is asserted where it is rather than assumed away. **Was bash asked instead, and it can be.** `declare -f run_gate` is bash's own reparse. It is not the reader, for three measured reasons: `set -n` does not define functions, so `declare -f` needs the audited file EXECUTED, and this catalogue plants adversarial shell into that very file inside a disposable clone. The output is a pretty-printed form with no stability contract, differing between the two bash versions on this machine, `cat <<'EOF'` and `esac` under 5.3.15 against `cat  <<'EOF'` and `esac;` under 3.2.57, and it normalises neither spelling the twelfth pass measured, `<<\\EOF` coming back as `<<'EOF'` and `<<'EOF-1'` unchanged. bash IS asked, in `scripts/tests/test_guard_readers.py`, where every span-table row and the delimiter production is run as a synthetic arm of `echo` markers and what bash PRINTS is compared with what the scanner attributes to the arm. That found the backtick's `opens` flag wrong: `x=`printf '%s' 'a`b'`` is an error to bash, so a quote does not hide a closing backtick, and the scanner had been accepting a file bash refuses. MEASURED over both regions the scanner is used on: after comments are stripped the `run_gate` region carries 0 `$'`, 0 `$(`, 0 `${`, 0 `<<` and 0 backticks against 48 before, and the GATES array carries 0 of all five. What is NOT probed is the unbalanced-quote refusal in `_arm_end`, and it is probed: `ci-floor.unbalanced-quote-in-an-arm` plants the unclosed quote in the LAST arm, which is the only position from which a later quote in the region cannot close it. That dependence on position is the sixth limit, declared here rather than hidden: an unclosed quote in an EARLIER arm is closed by the next quote in the region and read as a very long arm, which the per-command rule then refuses for a different reason, measured at exit 1 with a message naming the wrong thing. **The eighth limit is the workflow's own grammar, and it was not declared at all until the S03 review's twelfth pass, which measured four fail-open routes and five refusals of workflows GitHub Actions runs correctly.** `.github/workflows/ci.yml` is PARSED now, with `yaml.BaseLoader`, which is deviation D-17 and the only third-party import in the CI floor. What remains a limit is what a parse does not decide. The gate-name match is anchored at a STATEMENT HEAD, so a real invocation this file cannot see at a head is not counted: `sh -c 'bin/ocelli.sh gate x'` and `FOO=1 bin/ocelli.sh gate x` are refusals rather than passes, which is fail-CLOSED and names the gate. `_split_statements` splits on the boolean operators without evaluating them, so `false && bin/ocelli.sh gate x` counts as an invocation and never runs, and that is a shape nobody has measured in this repository rather than one that has been shown safe. **That sentence used to cover `|| true` as well and it should not have, which the S03 review's thirteenth pass measured.** A step whose FAILURE is discarded does not run the gate in the sense `--floor` means. `continue-on-error` on the step, the same key on the job, and `|| true` on the run were three plants that each left this check at exit 0 on the gate that watches every other gate, and the first two were covered by nothing at all: the string occurred nowhere in `scripts/`, in `docs/lld/guards.md` or in this runbook. Both keys are read now and `_tolerated_statements` answers the third, MEASURED against `bash -e` rather than read out of the errexit paragraph, whose obvious reading is wrong: `false && true` followed by another line exits 0. **The SHELL the body runs under was the next limit and the S03 review's fourteenth pass measured it open.** It said no step in `.github/workflows/ci.yml` set one, which was true, and that a `shell:` which is not a shell would be read as bash, which named the wrong danger. `python` and `pwsh` fail closed on their own, neither being bash. The value that does not is a CUSTOM TEMPLATE: `shell: bash {0}` is still bash and has no `-e`, MEASURED with a file holding `false` then `echo AFTER` at exit 1 under `bash -e <file>` and 0 under `bash <file>`, so every statement in the body is swallowed and the check exited 0 with the key on the real `bin/ocelli.sh gate guards` step. The same template written once at workflow level under `defaults:` took every `run:` in the file with it, also at exit 0. All three levels are read now and the value is refused BY NAME unless it is in `MEASURED_SHELLS`, which is `bash` and `sh` and is closed against values GitHub adds later. `ci-floor.custom-shell-template-on-a-gate-step` and `ci-floor.custom-shell-template-for-the-workflow` watch the two levels a repair could close separately, and `ci-floor.measured-shell-on-a-gate-step` watches the direction a refusal by name gets wrong, `shell: bash` being `bash --noprofile --norc -eo pipefail {0}` and therefore STRICTER than the default. The same pass measured the other half of the errexit reading, which `_tolerated_statements` could not see because it reads the separators AROUND a statement and these are properties of the statement's position: a `set +e` earlier in the body, an `if`, `elif`, `while` or `until` CONDITION, and a `!` negation, each measured under `bash -ec` at 0 where the plain command exits 1, each leaving the check at 0 with `guards` reported covered. `_errexit_exempt` answers all three and `ci-floor.set-plus-e-before-a-gate-step`, `ci-floor.gate-inside-an-if-condition` and `ci-floor.negated-gate-step` watch them one defence each, against `ci-floor.errexit-restored-before-a-gate-step` and `ci-floor.gate-inside-an-if-body` for the shapes that really do run the gate. What remains a limit there is stated in `_errexit_exempt` itself and both halves are fail-CLOSED, so each costs a refusal naming the gate rather than a pass: a `set +e` inside a `( )` subshell or a function body is scoped to it and this scanner models neither, so the exemption runs to the end of the body or to the next `set -e`, and a `then`, `else` or `do` ends the exemption wherever it appears, so a body using one of those words as a plain argument ends it early. `set -o pipefail` inside a body is not read, and that direction is fail-CLOSED too: pipefail makes a failure this file already treats as discarded reach the step, so the file counts LESS coverage than exists rather than more. A `continue-on-error` whose value is a `${{ }}` expression is treated as tolerating, on `_permits`' own rule: a value this file cannot read as harmless is not read as harmless. `on` is read as the string key, so the YAML 1.1 boolean spelling `true:` reads as no event block at all, which refuses with the top-level keys named. And the shapes the twelfth pass deliberately did NOT claim, because it could not establish them against a real parse, are anchors, the merge key, multi-document files and U+2028. The first three would now be answered by PyYAML rather than by this file, and a multi-document workflow raises a `YAMLError`, which `ci-floor.workflow-unparseable` is the probe for. That is a consequence rather than a claim and it is written here as one.
- **`scripts/verify_ledger.py`.** `assert` with a real record cannot be controlled green in the sandbox without recording one first, so the control for this invoke records a passing entry and then asserts.
- **`scripts/verify_ledger.py`.** The `records a RED corpus` and `records corpus=` branches of check-commit need a commit carrying a trailer this harness would have to forge, and the commit-msg hook refuses exactly that. They are reached instead by ledger.assert's equivalents.
- **`ci/check-device-ownership.sh`.** The trybuild compile-fail cases under crates/ocelli-compute/tests/ui/ are the strong half of section 31 and run in the `test` gate, and they are recorded here rather than as `covered_by`. They assert the same property through the type system and they do not open ci/check-device-ownership.sh, so counting them as coverage of THIS guard's refusals would be the claim check f exists to refuse. The four probes above are what watch those refusals.
- **`scripts/sprint_workflow.py`.** The other twenty-two refusals in this file belong to the sprint lifecycle commands, and the entry below owns them. These probes cover the branch grammar, malformed code spans, duplicate fields and one absent required field. They do not repeat the same shapes independently for all six fields. The six-field tuple is documented in `.claude/commands/complete-feature.md` and is also in the declared-constant ratchet, so adding, removing or renaming a field becomes a reviewed change.
- **`scripts/sprint_workflow.py`.** The remaining lifecycle branches belong to init, feature state transitions and release notes. The close probes build ignored sprint state from allocation.json and use the sandbox's real staged tree, so legacy, missing, dirty, stale, failed, carried and current evidence are exercised without touching the developer's run state.
- **`scripts/bench_check.py`.** No level-3 probe. The suite above is the negative-case set for this guard and it runs on every floor gate, so a level-3 probe would need a second registry fixture that the suite already carries.
- **`scripts/target_feature_check.py`.** The per-target divergence branch needs a dependency whose features differ by target. The locked graph contains no such fixture, and the disposable sandbox has no network authority to fetch a new dependency graph, so that branch has no probe.
- **`scripts/package_check.py`.** The consumer install, node import, two tsc resolutions and publish dry run need an npm install. `node_modules` is ignored and therefore absent from the `git ls-files` sandbox, so these level-3 refusals have no catalogue probe. The `packages` gate runs them against the real install on every push.
- **`scripts/split_hld.py`.** The redaction-map fail-closed branch sits behind pandoc conversion of the private source `.docx`. The document and conversion input are absent from the tracked repository, so a `git ls-files` sandbox cannot reach that branch or the related drift and section-mapping refusals.
- **`scripts/lint_policy_check.py`.** `REFUSED_GROUPS` is a list of nine names that exists only in the guard, and a probe can only ever write one of them, so narrowing the list to `clippy::pedantic` would leave `lint-policy.group-allow` green with eight groups unguarded. That is the shape `device.owned-accessor` has, and the answer is the same: the set is in the declared-constant ratchet, so narrowing it fails the census in the same change. Two of the nine are measured to reach 27.1's table under clippy 1.97.1 and the other seven are refused as blanket allows, which the message says rather than overclaiming. **The second limit was a TOML row this guard's regex could not read, and it is not a limit any more.** It was declared twice and both declarations claimed a division of labour with the declared constant `Cargo.toml:workspace.lints`: a two-line inline table for the sixth pass, a dotted key for the ninth, each measured past `LINT_ROW` at cargo clippy 101 to 0 and each said to be caught on the digest instead. **That sentence was false of the third spelling and the eleventh pass measured it.** A QUOTED key defeated both mechanisms at once: `"pedantic" = { level = "allow", priority = 1 }` as the LAST line of the table gave cargo 0, the guard 0 printing "no group row weaker than deny" and the census 0, because the constant's capture ended at the last line beginning with a bare key. In the middle of the table the ratchet held, `034c52d0054007ea` against `adf2cb2237be28da`, so the boundary was exact. Both halves read the tables with `tomllib` now and the constant records the guard's own parse, so every spelling is one code path and the two mechanisms cannot agree with each other while disagreeing with the grammar. What is refused rather than read is a document `tomllib` cannot parse, which is the two-line row, and `lint-policy.two-line-group-row` watches it. The third limit arrived with the seventh pass's fix and is the PROFILE: this guard reads its member set from `cargo metadata --no-deps`, so every probe here declares `needs="cargo"` and sits in the deep profile rather than in the floor, which is the same rule `nostd` already lives under for `cargo tree`. The guard itself still runs on every pull request in the `guards` gate, and since the ninth pass so does the deep harness, because `gate guards-deep` is a STEP in the unconditional `guards` job rather than the push-to-main job it used to be. This sentence said the harness watching the guard had moved to push-to-main, and it was still saying it a pass after that job was deleted, which the S03 review's tenth pass found. What the profile costs is a second run of the floor set, and what it buys is a member set cargo computes rather than one this file guesses at, after two passes in which the guess was wrong through a different key each time. The eighth pass found a THIRD key to that same defect and it is now closed rather than declared: a manifest's `[lib] path`, `[[bin]] path` or `[[test]] path` puts the crate root outside the member, measured at exit 0 with a group allow in it, so the walk is seeded from `cargo metadata`'s own `targets[].src_path` and the rglob is kept beside it for the modules no target names. **The NINTH pass found the fourth key and stopped calling the result a set the guard knows.** `include!` pastes a file's tokens in and is named by none of the other three, measured at cargo clippy 101 to 0 with the guard at exit 0, and it is followed and refused now like `#[path]` is. What that leaves is the honest statement of the class rather than another closed route: the source list is a RECONSTRUCTION from four keys, every pass since the fifth has found a new one, and `member_sources`'s docstring, this guard's OK line and `docs/lld/guards.md` all say so rather than claiming "every `.rs` file a workspace member compiles", which is what the first two claimed through four passes in which it was false. rustc's own dep-info at `target/<profile>/deps/*.d` IS the set, and it was considered as the authority and rejected with four measurements: it exists only after a build, a stale one narrows in silence, freshness by mtime would refuse after every keystroke, and making it fresh means this guard runs `cargo check --workspace --all-targets` at 10.8s in a clone with no `target/` and again inside every one of these probes, whose sandbox is a fresh copy of `git ls-files`. The fourth limit is what a rustflags scan cannot reach: cargo also reads `$CARGO_HOME/config.toml`, the `RUSTFLAGS` environment variable and `--config` on the command line, none of which is in this repository. That list was stated as if it were exhaustive and was not: `--cap-lints` is in the repository's own `.cargo/config.toml` space, names no lint so nothing in the flag scan saw it, and is MEASURED to take cargo clippy from 101 to 0 at `allow` and at `warn`. It has its own branch and two probes now, which is the whole space of weakening levels rather than a sample of it, so `CAP_LINTS_KEEPING` needs no place in the ratchet where `ALLOWING_FLAGS` does. `ALLOWING_FLAGS` joined the ratchet in the same pass, for the reason `REFUSED_GROUPS` is in it: narrowing it to `{"-A": 1}` was measured to leave both its probes, the census and the guard at exit 0 with `--allow`, `-W` and `--warn` unguarded. The fifth limit is ONE refusal here with no probe, declared rather than left to be found: a workspace member whose Cargo.toml cannot be read. The eighth pass replaced a bare FileNotFoundError traceback with a refusal under the FAIL header, which is the presentation `scripts/ci_floor_check.py` stopped giving in the fifth pass, and the branch cannot be driven from a sandbox: cargo cannot report a member whose manifest it could not read either, so the state is reachable only through the glob fallback and only by a filesystem permission that `Sandbox.reset` cannot restore. A probe that left a sandbox unrecoverable would cost more than the branch is worth.
- **`scripts/guard_probe.py`.** This entry's other refusals are watched by `probe-runner.self-test`, and one class of mutation to this catalogue reaches the harness as `error` rather than as `HARNESS`. A probe builder that resolves its own target through the guard it aims at, which `_expand_a_gate_step_into_its_visible_commands` does through `ci_floor_check.unseen_commands`, raises when that function is broken, and `run_probe` reports a builder failure as `error`. Both are red and both fail the gate, so nothing is lost, and a reader who expects `HARNESS` and sees `error` is reading the builder's refusal rather than the harness's. The builder above avoids the same trap by editing `main`'s first statement rather than the top of the guard's file, so a builder that imports the guard still runs.
- **`tools/oracle/run.mjs`.** Adopted and not re-declared, and not re-run either, because the run needs a browser and the `oracle` gate already does it. A second declaration of the same faults is the same defect as a second copy of the LUT chain, except that it only runs where nobody looks. What the census verifies instead is that the catalogue still carries faults, that each still names the message fragment proving its own boundary, that tools/oracle/run.mjs still reaches the runner, and that the count has not shrunk below the recorded 23.
- **`tools/oracle/check_sidecars.py`.** The self-test drives both new F-013 truth-projection refusals: a malformed source binding raises, and unequal source scopes remain unequal before the main checker reports them.
- **`tools/bench/src/runners/wasm_cold_start.mjs`.** The test that reaches a measurement against a stub, an incomplete artefact copy and a page that never reports launches Chromium. It is opted into with OCELLI_BENCH_BROWSER=1 and cannot run in the floor or a disposable sandbox that has no browser install. The developer browser suite watches it.
- **`tools/bench/page/app.mjs`.** The incomplete-artefact refusal executes in the benchmark page and requires the Chromium path, which the floor and disposable sandbox do not carry. The mark-count refusal does not share that limit and is watched in the floor by the phase-table unit test.
- **`scripts/panic_probe.mjs`.** The stub refusal fires only when the wasm module fails to export what the probe imports. Constructing that state needs a broken wasm-pack build and its generated artefact, neither of which exists in the tracked disposable sandbox.

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
