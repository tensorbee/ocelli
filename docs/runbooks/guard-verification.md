# Runbook, seeing every guard go red

## Why this exists

`docs/sprints/CURRENT_SPRINT.md` states it as a standing expectation of S01:

> **Every guard is seen red before it is claimed.** Remove or revert the thing
> a guard protects, one at a time, and watch it fail. A guard that has never
> been observed failing is a guard nobody has tested, and this repository now
> has seventeen of them written in one sitting by one author. Treat that as a
> liability until each has been seen red once.

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
