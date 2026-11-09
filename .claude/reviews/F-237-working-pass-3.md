# F-237 review, pass 3

**Reviewed**: `d29a7884..b0677cb8` on `work/f-237-xai`, three commits (d500f857, 15e4c636, b0677cb8). The worktree was clean apart from untracked `.venv` and `tools/oracle/out`, which are not part of the review. Only this review file was written in the worktree, and it is uncommitted.
**Author / reviewer**: remediation and self-review (`.claude/scratch/F-237-self-review-1.md`, `-2.md`, `-3.md`) by GLM in opencode. Reviewer is Claude, independent of GLM, and authored no F-237 material.
**Result**: 1 defect, 0 smells, 3 nitpicks

F-237 pass 3: 1 defect, 0 smells, 3 nitpicks.

## Acceptance challenged

Pass 2 findings, replayed through the real caller:

- **Pass 2 D1, a staged tree could not pass `gate packages`. Closed.** In scratch clone `m` at b0677cb8, a staged README edit (index tree `27ffed8e...`) made the real `gate packages` exit 0, and the packed build id recorded `git_sha` b0677cb8, `tree` `27ffed8e...`, `dirty` false. An unstaged README edit on top of that made the same gate exit 1 with "does not record a tree without unstaged tracked changes". In the whole `gate --floor` run on the same staged tree, `packages` passed and the index tree did not change. The run's only failure and its only skip are environment results from the fresh clone (see Evidence).
- **Pass 2 D2, the comparison ran after the tag and crates.io. Closed.** `.claude/commands/release.md:47-54` runs `release_wasm.py HEAD` at preflight 8, under "Refuse before any tag or push", and publish step 5 (`:84-88`) publishes that verified file. `docs/RELEASE.md` item 6 and both LLDs say the same. The adapters carry SHA-256 `3718b9c9...`, `shasum` reproduces it, and `sync_agent_skills.py --check` exits 0. Through the real script in clone `r`: clean HEAD exit 0, with digests `9d2c7927...` and `07cf6db9...`, the same pair as pass 1 and pass 2, and recorded tree `3ebd6bb4...` equal to `HEAD^{tree}`. A staged README edit exits 1 with the tree refusal only. An unstaged one exits 1 with the dirty refusal only. Neither refused run created its `--out` directory.
- **Pass 2 D3, the CI comment. Closed.** `env PATH=/usr/bin:/bin:/usr/sbin bin/ocelli.sh wasm` and `wasm-decode` both print "wasm-pack is not installed. See docs/DEVELOPER_SETUP.md" and exit 1, as `.github/workflows/ci.yml:312-317` now says. The comment no longer claims a CI run.
- **Pass 2 S1, the call-in-main probe. Closed.** `packages.build-id-call-in-main` is now behavioural. It goes HARNESS red (the probe exited 0) under five mutations of `package_check.main()`: the two lines commented out, the result discarded, the branch keyed on `MODULE_PACKAGE + 'x'`, the call deleted, and the call removed from the loop. Its control is green on the clean tree.
- **Pass 2 N1. Closed.** The python pin comment now sits above `setup-python`.
- **Pass 2 N2. Closed.** `is not False` to `is True` in `release_wasm.py` turns `test_a_build_id_without_the_dirty_field_is_refused` red. The same mutation in `package_check.py` turns `packages.build-id-dirty` red.
- **Pass 2 N3. Closed.** `ci_floor_check.py` exits 1 naming the `frontend` job in three cases: the wasm-pack step deleted, the `targets:` line deleted, and the gate step rewritten to `python3 scripts/package_check.py` with wasm-pack deleted. An `echo` that only mentions the gate in the `guards` job is not matched. `bin/ocelli.sh gate lint packages` in that job is matched, which is consistent with `runs_command`'s equality rule. Deleting the target-half `problems.append` turns `guard_census.py` red with "ci-floor shrank from 27 refusal site(s) to 26", which is what the probe note means by "watched by the recorded count".
- **Pass 2 N4. Open, see D1.**

Fresh material:

- **The dirty rule refuses only unstaged changes.** `stage.mjs` now reads `git diff --name-only` (working tree against index). Mutation M1 puts back `git status --porcelain`, and the staged stage test goes red. M2 forces dirty to false, and the unstaged test goes red. M3 records `HEAD^{tree}` for the tree, and the staged test goes red. M5 makes the gate compare against `HEAD^{tree}`, and the staged test goes red. M10 removes the gate's dirty check, and the unstaged test goes red. M4 removes the gate's tree check. The unit suite stays OK, and `packages.build-id-tree` goes red. M6 removes the release tree check, and `test_a_staged_index_tree_is_refused` goes red.
- **The LLD count.** `guard_probe.py --list` shows eleven L1 `packages.build-id*` probes: nine over the parsed id and two over the wiring, as `docs/lld/typescript-packaging.md` says.
- **README:170-176, README:454-461, use.html:238** match the build id the stage writes and what the gate refuses. more.html names no build id and is not stale.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 -m unittest scripts/tests/test_release_wasm.py -v` | worktree b0677cb8 | now | 0, 12 tests OK |
| ten mutations M1 to M10 of `stage.mjs`, `package_check.py`, `release_wasm.py`, unit suite | scratch clone `mut` | now | M1 M2 M3 M5 M6 M7 M10 red. M4 M8 green in the suite and red in their probes. M9 green everywhere (N1) |
| `guard_probe.py --only` under M4, M8 and the five S1 unwirings | `mut` | now | each HARNESS red, "exited 0. The guard it aims at did not fire" |
| `guard_probe.py --only packages.build-id-tree` | worktree | now | red for its reason, control green |
| `ci_floor_check.py`, clean and under N3a to N3c and three matching cases | worktree, `mut` | now | 0 clean. 1 for each N3 case. The echo and flagged cases 0, the multi-gate case 1 |
| `guard_census.py` with the target-half refusal deleted | `mut` | now | 1, ci-floor 27 to 26 |
| `npm ci`, `bin/ocelli.sh wasm`, `npm run typecheck`, `gate packages` | clone `m`, clean | now | 0, 0, 0, 0 `OK: 3 package(s) at 0.1.0` |
| `gate packages`, staged README edit | `m`, index 27ffed8e | now | 0, build id tree 27ffed8e, dirty False |
| `gate packages`, staged plus unstaged edit | `m` | now | 1, unstaged refusal |
| `bin/ocelli.sh gate --floor`, staged README edit | `m`, index 27ffed8e | now | 1, 26 passed, 1 failed, 1 skipped. `packages`, `ci` and `guards` passed, and the index tree was 27ffed8e before and after. `test` failed in two `produce::tests` because untracked `tools/oracle/out/run.json` is absent in a fresh clone. `corpus-tests` skipped because pydicom and numpy are absent without `.venv`. Both are environment results from the clone, not F-237 |
| `release_wasm.py HEAD`, clean | clone `r` | now | 0 in 96 s, digests 9d2c7927 and 07cf6db9, tree 3ebd6bb4 |
| `release_wasm.py HEAD`, staged README edit | `r` | now | 1, tree refusal only, no `--out` |
| `release_wasm.py HEAD`, unstaged README edit | `r` | now | 1, dirty refusal only, no `--out` |
| `bin/ocelli.sh wasm` and `wasm-decode` without wasm-pack on PATH | `r` | now | 1, the message the CI comment names |
| `sync_agent_skills.py --check`, `prose_check.py`, `shasum` of release.md | worktree | now | 0, 0, `3718b9c9...` equal to both adapters |
| lint-policy note text, HEAD against `sprint/s46` | `git show` of both | now | differs (D1) |
| `npm publish --dry-run <verified file>` | | prior receipt (pass 2, GLM cycle 1), not re-run | no prepack, shasum equal |
| `gate guards-deep` | | prior receipt (GLM cycle 3), not re-run | recorded 0 |

Environment: macOS arm64, rustc 1.97.1, wasm-pack 0.15.0, node v24.13.0. Scratch clones `m`, `r` and `mut` lived under the session scratchpad `f237-p3`, each with its own `CARGO_TARGET_DIR`. They are deleted with `shutil.rmtree` after the pass.

## Defects

### D1, the two lint-policy notes are not the sprint's text, and they say the planted module is the second when it is the third

**Where**: `scripts/guards/catalogue.py`, notes of `lint-policy.clean-module-outside-the-member` (around line 12743) and `lint-policy.clean-cfg-attr-module-path` (around line 12792)
**What**: Both notes read "`produce` already follows one path module, so the planted module is the second. The count is the one measured on the F-232 tree, where a third `#[path]` module had landed since the probe was written." The expect fragment beside each one is "3 `#[path]` module(s) followed". The workspace follows two path modules, `tools/oracle/src/produce.rs:40` and `examples/folder-viewer/src/held.rs:23`. The real `lint_policy_check.py` in this pass's floor run printed "2 `#[path]` module(s) followed". So the planted module is the third, and "the second" is false. `sprint/s46` carries different text: "The tree follows two `#[path]` modules, `produce` and `held.rs` (F-210), so the planted module is the third. The count is the one measured on the F-232 tree." HEAD has the intermediate wording from d514da30, which F-232 later replaced.
**Why it is wrong**: it is a false sentence in a guard note. It also does not follow the lead ruling after pass 2, "For N4 the sprint's F-232 note text wins". The sprint text was not taken, so integration will still conflict on both notes, which is what N4 asked to avoid. GLM's cycle 3 records the notes as "the sprint text verbatim", and that is also false.
**Evidence**: a Python `difflib` of both probe blocks at `HEAD` and `sprint/s46`, and the `guards` gate output in the floor run.
**Scope and regression**: prose only, present on HEAD. Fix: copy the two note strings from `sprint/s46:scripts/guards/catalogue.py` exactly. Regression: diff the two probe blocks against `sprint/s46` and require them to be equal.

## Smells

None.

## Nitpicks

- N1, `scripts/release_wasm.py:235`: `tag_tree` comes from `rev-parse <sha>^{tree}`. Replacing that with `git write-tree` passes every test and probe (M9), because `main()` is not exercised outside `/release`. That mutant would accept a staged index whose wasm bytes match the tag. Precondition 2 (clean tree) covers that case, and the real staged run in this pass refused correctly. A small test of `main()` with `cold_digests` and `pack` stubbed would pin it.
- N2, `scripts/release_wasm.py:79`: the message says "the release revision's git write-tree is". A revision has a tree, not a write-tree, and the value is `<sha>^{tree}`. "the release revision's tree is" would be accurate.
- N3, release.md preflight 8 and step 5: a refused re-run does not remove an `ocelli-wasm-X.Y.Z.tgz` left in `target/release-wasm` by an earlier successful run at another HEAD. The release stops at the refusal, so this is not reachable through the procedure as written. Clearing `--out`, or that one file, before the run would make "the file in that directory is the verified one" hold without relying on the procedure.

## Verified clean

- All of pass 2's D1, D2, D3, S1, N1, N2 and N3, through the real gate, the real release script and the real CI checker, each with an adjacent case (see Acceptance challenged).
- The staged and unstaged split in `stage.mjs`, the gate and the release script, each pinned by at least one test or probe that a single mutation turns red. The one exception is N1.
- Release reproducibility held again: a cold build in `/var/folders` produced the same two digests as passes 1 and 2.
- The provisioning rule matches by statement head or by exact arm command, not by substring, as its docstring says.
- The "watched by the recorded count" claim in the ci-floor probe note holds under deletion.
- The LLD probe count, the README and use.html sentences, and the release.md, RELEASE.md and release-policy timing sentences, each against the file or command.
- No em-dash, en-dash or prose semicolon in the range or its commit messages. `prose_check.py` exits 0.
- Tree intact: `git status --porcelain` shows only the two untracked paths, `git diff HEAD` is empty, and the branch reflog is linear from 0490062e through b0677cb8 with no reset, amend or rebase. GLM's `target/f237-*` artefacts are logs only, and its scratch clones are gone.
- GLM's self-review 3 reports one `git checkout -q HEAD -- .` inside its own scratch clone. It touched nothing in the worktree, which the clean status and reflog confirm.

## Limitations and handoff

- No GitHub Actions run of this branch exists. The `frontend` job's provisioning is shown only by `ci_floor_check.py` and by local emulation on macOS.
- `corpus/data` is absent in the scratch clones, so the feature profile's corpus half was not run. The floor run on the staged tree stands in for `/verify`'s gate set without corpus, and no ledger receipt was recorded.
- `gate guards-deep` and `npm publish --dry-run` were not re-run this pass. Prior receipts are cited.
- No publish, no tag, no push and no registry write.
- D1 is GLM's to remediate, and a pass 4 must review the change.

Acceptance rows still owed, with the host each one needs:

- AC4 and the ruling's CI half: the `frontend` job green on GitHub Actions `ubuntu-latest`, after integration with `sprint/s46`, which now carries F-217's fresh-clone build fix. Host: GitHub Actions, lead-owned integration.
- `/verify` with a recorded receipt on F-237's final staged tree, feature profile including `corpus`. Host: the operator's machine with `corpus/data`.
- AC3 end to end in a release: `/release` preflight 8 (`release_wasm.py HEAD`) at the real release HEAD, after `gate --all` including `oracle` and `gpu`. Host: the release machine with GPU and corpus.
- AC5 against the registry: `@ocelli/core` and `@ocelli/wasm` installed at one version from npm by a consumer outside the workspace. Host: the release, after publish. Only the local tarball consumer proof has run.
- The integration merge itself, where both lint-policy notes (D1) and the regenerated runbook table in `docs/runbooks/guard-verification.md` will conflict with `sprint/s46`. Host: the canonical sprint worktree, `/integrate-feature`.

## Self-review catch figure

Of the 1 blocking finding, GLM's recorded checks should have caught 1. Its cycle 1 writes that `held.rs:23` "is the second production module, which is what the expect fragment's count 3 measures with the planted one", which contradicts the note it kept. Its cycle 3 claims "the sprint text verbatim" without diffing against `sprint/s46`. Of the 3 nitpicks, N1 was within reach, because GLM mutated the comparison and not `main()`'s tree derivation. So 2 of 4 findings overall.
