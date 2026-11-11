# F-238 review, pass 3

**Reviewed**: `d1814bb3..56d00f15` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `56d00f15`. Two commits, `c8947ed1` "F-238, review pass 2 remediation" (6 files) and `56d00f15` "F-238, pass 2 self-review note fixes" (`scripts/guards/catalogue.py` only). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`. The pass 2 findings in `.claude/reviews/F-238-working-pass-2.md` were replayed against this tree.
**Author / reviewer**: GLM in opencode is the author, with self-review cycles 5 to 8 in `.claude/scratch/F-238-self-review-5.md` to `-8.md`. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write pass 1 or pass 2.
**Result**: 2 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **Pass 2 S2, the override, through the real caller.** `OCELLI_TIER=cpu bin/ocelli.sh native` on HEAD is exit 0 and step 6 passes. The shipped binary itself still prints `Tier Cpu, decided by Override` under that export, so the green comes from `env -u` and not from a changed binary. In a scratch copy with `env -u OCELLI_TIER ` removed from step 6, the override probe's builder refuses and its control goes red with the Override report, so the shield is watched.
- **Pass 2 S3, the producer, through the real caller.** In a scratch copy with an APFS clone of the target, `resolution.evidence.no_backend_compiled = true;` deleted (and the `mut` with it, so no warning is left): `bin/ocelli.sh native` exit 1 at step 6 with `FAIL: the shipped ocelli-desktop reported NoAdapter without its no-backend cause line`. Closed.
- **Pass 2 D1, the dev restore sentence.** Scratch, `metal` removed from `ocelli-render`'s native dev table: `cargo test --offline -p ocelli-render --lib a_development_build_compiles_at_least_one_backend` FAILED, and passed with the table restored. The narrowed sentence is exact.
- **Pass 2 D2 and N3, the binary path.** The "prescribed environment" sentence is gone. `cargo metadata --no-deps` answers the directory cargo builds into for unset, `CARGO_TARGET_DIR` and `CARGO_BUILD_TARGET_DIR`. Run from `crates/ocelli-core`, a relative `CARGO_TARGET_DIR=rel` comes back as `crates/ocelli-core/rel`, which is D1 below.
- **The five new probes, each mutated in a scratch copy of the tracked tree** (`git archive HEAD`, git-initialised locally, `scripts/guard_probe.py --only` over all five):
  - Apple tier grep weakened to `Tier `: `wrong-tier-on-apple` and `override-cannot-reach-the-binary` reported HARNESS, "not at the declared refusal". Red.
  - `env -u OCELLI_TIER ` removed: override control red. Red.
  - Cause-line grep emptied: `cause-line-missing` HARNESS, "did not fire". Red.
  - Off-Apple grep emptied: `no-tier-off-apple` HARNESS, "did not fire". Red.
  - `shipped_bin` reverted to a fixed `$ROOT/target/release` path: every control red with exit 127. Red.
  - Step banner renamed `6/16`: the builder raised and the harness exited 1. Fails closed, as the section comment says.
  - The `exit 1` after the non-zero-status refusal replaced by `true`: harness exit 0, all five still red for their declared reason. This is S1 below.
- **Stub reports against the real binary.** `_HONEST_APPLE_REPORT` and `_OVERRIDDEN_APPLE_REPORT` equal the shipped `target/release/ocelli-desktop` output from its `Tier ` line on, with and without `OCELLI_TIER=cpu`, compared in Python. `_APPLE_REPORT_WITHOUT_CAUSE` is the honest report minus exactly the cause line.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate native` | worktree HEAD `56d00f15`, `CARGO_TARGET_DIR=$PWD/target` | now | 0, `ALL GREEN 1 gate(s)` |
| `OCELLI_TIER=cpu bin/ocelli.sh native` | same | now | 0, step 6 passes |
| `bin/ocelli.sh gate guards` | same | now | 0, 450 refusal probes red, 44 accept green, 112 controls green, census 1173 refusals in 81 files, 569 probes, 0 watched by nothing |
| `target/release/ocelli-desktop` with no override, `cpu`, `a` | binary from the run above | now | exit 0 each, `NoAdapter` with the cause line, `Override` without it, `RefusedUnconstructible(A)` with it |
| `guard_census.py --check-runbook`, `prose_check.py`, `deviation_check.py`, `test_guard_catalogue.py` | worktree HEAD | now | 0 each, 924 files clean, 33 deviations |
| Five step 6 probes, seven mutations of step 6, one catalogue mutation | scratch export of HEAD's tracked files | now | as under Acceptance challenged |
| `bin/ocelli.sh native` with the flag line deleted | scratch export plus cloned target | now | 1 at step 6, cause-line refusal |
| `cargo test -p ocelli-render --lib` with render dev `metal` removed | same scratch | now | FAILED, then ok when restored |
| `cargo metadata --no-deps` from `crates/ocelli-core` with relative values | worktree, read only | now | resolved against the current directory |
| `wgpu-30.0.1/build.rs`, `src/api/instance.rs:58` to `:70`, `:108` | registry `index.crates.io-1949cf8c6b5b557f` | now | see Verified clean |
| GLM self-review cycles 5 to 8 | `.claude/scratch/` | prior, not relied on | read for the catch figure and the incidents only |

## Defects

### D1, a relative target directory is said to resolve against the workspace root

**Where**: `bin/ocelli.sh:656` to `:657`
**What**: "a relative value is resolved by cargo against the workspace root, which is where it runs from here." Cargo resolves a relative `CARGO_TARGET_DIR` or `CARGO_BUILD_TARGET_DIR` against the current directory. Run from `crates/ocelli-core`, `CARGO_TARGET_DIR=rel cargo metadata --no-deps` answers `.../crates/ocelli-core/rel` and `CARGO_BUILD_TARGET_DIR=rel2` answers `.../crates/ocelli-core/rel2`, not the workspace root.
**Why it is wrong**: microscope section 3. The step's behaviour is correct only because the `cd` at `bin/ocelli.sh:13` makes the current directory equal the root. The comment names the wrong mechanism, and a later edit that runs step 6 from another directory would trust it. GLM's cycle 5 measured the claim from the root only, where the two cannot be told apart.
**Evidence**: the two commands above, run read only in the worktree.
**Scope and regression**: comment only, HEAD. Say "a relative value is resolved by cargo against the current directory, which is ROOT after the `cd` at the top", or delete the clause.

### D2, the step 6 builders are said to sit directly above the runner entry

**Where**: `scripts/guards/catalogue.py:12159`
**What**: "The builders for that live directly above the entry." The `runner` entry is at `:12162`. The builders are in the `Step 6 of the native arm` section at `:7517` to `:7680`, before `# The catalogue` at `:7682`, about 4,480 lines away.
**Why it is wrong**: microscope section 3, a false factual sentence in a guard note.
**Evidence**: `grep -n` for the section header, the four builders and `id="runner"`.
**Scope and regression**: comment only. Name the section: "The builders are in the `Step 6 of the native arm` section above `GUARDS`."

## Smells

### S1, the non-zero-exit probe does not watch the refusal's exit

**Where**: `scripts/guards/catalogue.py:12185` to `:12187`, guarding `bin/ocelli.sh:669` to `:671`
**What**: `runner.step6-exits-without-a-report` drives a stub that prints nothing and exits 3. With the refusal's `exit 1` at `bin/ocelli.sh:671` replaced by `true`, the echo still prints "exited 3 instead of reporting its tier" and the empty report is then refused by the Apple `NoAdapter` grep, so the probe stays red for its declared reason and the harness is exit 0. On a Linux runner the off-Apple `^Tier ` grep does the same. The probe watches the message and not the refusal it names.
**Why it matters**: microscope section 2, a test that passes both ways. With that exit gone, a shipped binary that prints a correct report and then exits non-zero, for example a panic while tearing down, passes step 6 and every gate stays green. GLM's cycle 6 P1 changed `exit 1` to `exit 0`, which stops the run and so cannot tell these apart.
**Smallest regression**: give the probe `report=_HONEST_APPLE_REPORT, exit_code=3`. Measured in a scratch copy: with that change and the `exit 1` removed the probe reports HARNESS, "exited 0. The guard it aims at did not fire". With the step intact it is red for its declared reason and its control is green.

## Nitpicks

- **N1, carried from pass 2 N1.** `scripts/decode_graph_check.py:131` to `:134` and D-33 still say compute's table is "held to the same shipped set". The check is a floor. Not in this range.
- **N2.** D-33's sentence now reads as a complete list of the unsatisfied backend cfgs and leaves out `webgl`. `ocelli-render`'s native table names it, and its cfg needs `not(native)` (`wgpu-30.0.1/build.rs:14`). `c8947ed1`'s message says N2 "completes" the list. No consumer is affected.
- **N3.** Step 6 assumes no `build.target` or `CARGO_BUILD_TARGET`. Under either, cargo writes `<target_directory>/<triple>/release/ocelli-desktop` and the step would run a missing binary (fails closed) or a stale one. Unconfirmed, because nothing in the repository sets either.
- **N4.** The `runner` entry's `refuses` lists "prints no tier" twice, once with no host and once as "on any other host". Step 6 has four refusal sites, and the sentence reads as five.

## Verified clean

- **Pass 2 D1 closed.** The sentence names render's table only, and that table's removal is red, measured.
- **Pass 2 D2 closed.** The false sentence is deleted and the path comes from `cargo metadata`. Its replacement carries D1 above.
- **Pass 2 S1 closed in substance.** Five probes, a `limit=`, and the census re-recorded. `gate guards` shows all five red with controls green. `runner` 5 and `catalogue` 85 match the census matcher (census OK, `--check-runbook` 0). Four of the five probes discriminate their refusal under mutation. The fifth is S1 above.
- **Pass 2 S2 closed** through `bin/ocelli.sh native` with `OCELLI_TIER=cpu` exported.
- **Pass 2 S3 closed** through `bin/ocelli.sh native` with the producer line deleted.
- **Pass 2 N2** partly closed (N2 above). **N3** closed for the three target-dir spellings. **N4** closed: no `f238*` path under `target/`.
- Step 6 comment claims: the cause-line sentence (measured), `NoAdapter` with backends and no GPU (the same claim `probe.rs:278` and `ocelli-native/src/lib.rs:380` already make, not new), "Under an override `detect` returns before the no-backend guard" (`probe.rs:263` returns before `:282`), and `CARGO_BUILD_TARGET_DIR` being as live as the others (measured).
- wgpu against the pinned source: `Instance::new` panics when `enabled_backend_features()` is empty (`src/api/instance.rs:64` to `:69`). `enabled_backend_features` is `pub const fn` at `:108`. `build.rs:13` to `:15` gives `webgpu` as `all(not(native), not(Emscripten), feature)` and `dx12` as `all(target_os = "windows", feature)`, so D-33's new clauses are true.
- The stub script runs inside the sandbox and writes only `target/f238-step6-stub` under it. No `stub-target`, `probe-f238-step6.sh` or `f238*` path exists in the worktree, the canonical checkout or the parent directory after `gate guards`.
- The guard section comment's fail-closed claim (banner rename raises, harness exit 1).
- `README.md`, `use.html` and `more.html` are not in the range, and none of them names step 6, `cargo metadata` or the cause line. `use.html:764`'s `OCELLI_TIER` sentence is unaffected.
- No em-dash or en-dash in added lines, `prose_check.py` clean. No Rust changed, no `unsafe`, no `wasm-bindgen`, no pixel arithmetic, no dependency change.

## Tree integrity and the disclosed incidents

- `git status` shows only the untracked `.venv` and `tools/oracle/out`. `git diff HEAD --quiet` is 0. `git fsck --connectivity-only` is 0.
- The reflog from `d1814bb3` to `56d00f15` is two `commit:` entries, `c8947ed1` at 20:46:59 and `56d00f15` at 21:16:08, with no reset, amend or rebase. Both commits are in `git log d1814bb3..56d00f15`, and nothing else is.
- Incident 1, the five `rm` calls: the cycle 7 note names five files inside `target/f238-sr-native`, an ignored scratch under the worktree's `target/`. That scratch is gone and no tracked file changed. It breaks the no-`rm` rule and did no harm I can find.
- Incident 2, `stub-target/` at the worktree root: absent now, untracked, and not in either commit.
- Outside the worktree: the canonical checkout's `git status --short` is empty and no stray file was found under `/tmp` or the parent directory. This cannot prove nothing else was touched. It shows no sign that anything was.
- One `ocelli-guard-*` sandbox from 21:27 remains under the system temp directory. It comes from this pass's `gate guards` run, and the harness's hourly `sweep_stale` removes it.

## Self-review catch figure

3 of this pass's 7 findings were within reach of GLM's self-review, and all 3 blocking findings are among them.

- D1: cycle 5 executed the claim from the workspace root, the one directory where cwd and root coincide. One run from a subdirectory tells them apart.
- D2: a location claim checked by one `grep -n`.
- S1: cycle 6 mutated this refusal's `exit 1` to `exit 0`. Removing the exit is the plainer mutation, and the cycle ran every other probe's one-step-away case.
- N2 is arguably a fourth, since `build.rs:14` sits between the two lines cycle 5 cites. N1 is carried, and N3 and N4 are judgement calls.

## Limitations and handoff

- `gate gpu` was not run, under this pass's no-GPU rule. **AC3** (development `gate gpu` resolves tier A through Metal on the M4 Max) is still owed and needs the macOS M4 Max host with a GPU job allowed. The dev binary's tier report is not the `#[ignore]` suite.
- **AC4** has its macOS half here (the shipped binary resolves and reports tier C, exit 0). Its Linux and Windows halves are owed. Step 6 runs the shipped binary on `ubuntu-latest` in CI and that run has not been observed by any pass. It needs a Linux host or a CI run, and Windows needs a Windows host.
- **AC1** for `x86_64-apple-darwin` is resolved by `cargo tree` and never linked or run. It needs an Intel Mac, or a cross-link, to go further.
- **AC5** rests on `gate licences` from pass 2 and on `gate native` now. This range does not touch the notices.
- **AC2** passes on `gate guards` (floor). The deep probes `shipped-native.*` were not re-run, because this range does not touch them.
- Scratch copies were under `scratchpad/f238-p3` and were removed with `shutil.rmtree`. Logs are in `scratchpad/p3logs`. The relative `cargo metadata` runs wrote nothing.
- Next owner: the implementer remediates D1, D2 and S1, then a fresh independent pass 4 reviews the changed material. This is a bounded review of two commits. It is not acceptance and not sprint readiness.
