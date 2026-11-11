# F-239 review, pass 5, codec part

**Reviewed**: `work/f-239-codec`, range `ea3e5f6b..925b546f` (one commit, 5 files: `.github/workflows/ci.yml`, `bin/ocelli.sh`, `ci/guard-probe-budget.json`, `docs/runbooks/guard-verification.md`, `scripts/guards/catalogue.py`), in the worktree `ocelli-wt-f-239-codec`, tip `925b546f05a263f0ca18797c29d8a93c37ab90e6`. No staged or unstaged tracked change at review time. Untracked: the `.venv` symlink and the `tools/oracle/out` symlink only.
**Author / reviewer**: remediation by GLM in opencode under the lead ruling after pass 4 (`.claude/plans/F-239-design.md:574` to `:578`). Reviewed by Claude as an independent `/microscope --working` pass 5. The reviewer wrote no part of this range. OpenJPEG source was not read.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

- **Pass 4 S1, the vendored suites in a gate.** Driven through the real runner, `bin/ocelli.sh gate vendor-tests`, not through hand-run cargo. Both directions measured: green on the tip, red under the openjph mutation inside the probe, red under a ritk 1e-2 mutation applied by hand, and the probe itself reported HARNESS FAIL when the gate's openjph `cargo test` was replaced by `true`. So the probe discriminates the gate and is not satisfied by any red.
- **CI dependency supply.** A fresh `CARGO_HOME` with network, as a GitHub runner has, reproduces the CI condition: the arm's `cargo fetch --locked` downloads each crate's lock set, and the `--offline --locked` test then resolves from that cache. The commit's claim that the workspace lock cannot supply the dev-dependencies was checked against the three lock files: `criterion` is in neither workspace lock row (0) but in both vendored locks, `base64` and `image` only in ritk's, `jpeg-decoder` in the root and ritk.
- **The lint-policy notes against `sprint/s46`.** Block-by-block comparison of both probes, and a `git merge-tree` of the tip and of the base `ea3e5f6b` against `sprint/s46`, to see whether this range adds a conflict. It does, see S1.
- **The builder precondition.** The pre-fix StopIteration was reproduced by deleting the new two-line check in scratch, and the three other callers of `_a_gate_with_an_unextractable_arm_command` were run green on the tip.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate vendor-tests` | scratch clone at 925b546f, clone's own `target/` | now | 0. openjph 154 passed, ritk 321 passed, 17 s warm |
| same, `CARGO_HOME` set to an empty scratch directory | scratch clone | now | 0. `Downloaded thiserror`, `quick-error` and the rest, then both suites green offline |
| `shasum -a 256 vendor/*/Cargo.lock` before, after each run, after both floors | scratch clone and worktree | now | `5613681a...` and `0741dfb2...` throughout, equal to `ea3e5f6b` and to each `PACKAGE-INVENTORY.sha256` row |
| `find vendor -name target` | scratch clone after runs | now | nothing. Artefacts land in `target/vendor-tests` |
| `bin/ocelli.sh gate --list` | worktree | now | `vendor-tests no both vendored crates' library tests, offline and locked (F-239, D-31)` |
| `guard_probe.py --profile deep --only vendor-tests.openjph-ict-coefficient` | scratch clone | now | OK, red for its declared reason, control green, 25.8 s |
| same probe with the gate's openjph `cargo test` replaced by `true` | second scratch clone | now | FAIL, "exited 0. The guard it aims at did not fire". Restored |
| ritk `0.34413 * cb` to `0.35413 * cb` in `reader.rs`, then the gate | second scratch clone | now | exit 1, `inverse_ict_9_7_is_the_annex_g_transform ... FAILED`, 320 passed 1 failed. Restored |
| `--only` the two lint-policy accept probes and `ci-floor.gate-with-an-unextractable-arm-command`, `ci-floor.work-inside-an-if`, `ci-floor.work-behind-the-command-builtin` | second scratch clone | now | each OK |
| `ci-floor.gate-with-an-unextractable-arm-command` with the new `if not any(...)` check deleted | second scratch clone | now | FAIL, "could not build its rejected state: StopIteration". Restored |
| `guard_probe.py --profile deep` | second scratch clone, no `node_modules` | now | 0. 492 refusal probes, 59 accept, 105 controls, 180.5 s |
| `bin/ocelli.sh gate --floor`, first run | scratch clone after `npm ci`, no `tools/oracle/out` or `corpus/data` | now | 1. 28 passed 1 failed: `test`, two `ocelli-oracle` tests panicking on a missing `tools/oracle/out/run.json`. See Limitations |
| `bin/ocelli.sh gate --floor`, second run | same clone with the worktree's two links, `tools/oracle/out` and `corpus/data`, mirrored | now | 0. ALL GREEN 29 gates, 0 skipped. `ci` gate: "all 29 floor gate(s) are invoked by CI". Census: 1131 refusals all claimed |
| block comparison of the two lint-policy probes, worktree against `git show sprint/s46:` and `git show work/f-237-xai:` | refs | now | expects equal (`3`), notes differ from s46. s46 and F-237 equal to each other |
| `git merge-tree --write-tree ea3e5f6b sprint/s46` and `925b546f sprint/s46` | refs | now | 8 conflicted files before, 9 after. The new one is `scripts/guards/catalogue.py`, two hunks, both the notes |
| `git diff --stat ea3e5f6b 925b546f -- vendor crates`, `git show 925b546f -- '*.rs' '*.wgsl'` | worktree | now | empty and 0 lines |
| added lines and commit message scanned for em-dash, en-dash and `; ` | worktree | now | none. `prose_check.py` green inside the floor |

## Defects

None.

## Smells

### S1, the two lint-policy notes now conflict with `sprint/s46`
**Where**: `scripts/guards/catalogue.py:12049` to `:12051` (`lint-policy.clean-module-outside-the-member`) and `:12093` to `:12095` (`lint-policy.clean-cfg-attr-module-path`).
**What**: the range changes each probe's expect from `2` to `3` and rewrites its note. The expect change is the same bytes `sprint/s46` already carries (`d514da30`, then `1a52c504`), so it merges clean. The notes are new wording, "`produce` and the folder viewer's `held` each follow one path module already, so the planted module is the third.", where `sprint/s46` and `work/f-237-xai` both carry, byte-identical, "The tree follows two `#[path]` modules, `produce` and `held.rs`, so the planted module is the third. The count is the one measured on the F-232 tree." Both sides edited the same lines from the base, so the merge conflicts.
**Why it is wrong**: the branch carries a conflicting change to text `sprint/s46` had already settled. Both wordings are true (`tools/oracle/src/produce.rs:40` and `examples/folder-viewer/src/held.rs:23` are the two `#[path]` sites), so no claim is false, but `/integrate-feature` gets a new conflict in a 12000-line generated-adjacent file, and a resolver choosing "ours" drops the s46 wording that F-237 was aligned to, which re-opens a divergence the lead closed hours earlier.
**Evidence**: `git merge-tree --write-tree` against `sprint/s46` lists `CONFLICT (content): Merge conflict in scripts/guards/catalogue.py` at the tip and not at `ea3e5f6b`. The merged blob holds two conflict hunks, both the note lines and nothing else.
**Scope and regression**: integration only. No guard verdict depends on a note. Fix: replace both notes with the `sprint/s46` bytes verbatim (copy from `git show sprint/s46:scripts/guards/catalogue.py`, do not retype), keep the `3` expects, and confirm `git merge-tree --write-tree HEAD sprint/s46` no longer names `catalogue.py`. Then re-render the runbook if the census asks.

## Nitpicks

- N1. `scripts/guards/catalogue.py:631`, "`script()`'s 300 seconds is the only bound". The 300 is `Sandbox.run`'s default `timeout` (`scripts/guards/sandbox.py:266`), which `script()` reaches. True in effect, but names the wrong owner.

## Pass 4 findings replayed

- **S1, closed.** All five ruling conditions hold, each executed: both crates' `--lib` tests run `--offline --locked` (`bin/ocelli.sh:128` to `:139`), `CARGO_TARGET_DIR` is `$ROOT/target/vendor-tests` and nothing lands under `vendor/`, `ci.yml:69` to `:70` runs `bin/ocelli.sh gate vendor-tests` in the `rust` job, `gate --list` shows it, and both lock files are byte-identical by sha256 after every run. The probe is red under its 1e-2 `GAMMA_CB2G` mutation, green as a control, and stops passing when the gate stops running the openjph suite. The ritk patch is covered by the same step, measured by hand.
- **N1, closed.** `catalogue.py:8375`, "The recorded D-31 patch covers the files its refusal names.", and the `pins.openjph-inventory` probe is green inside the floor.

## Answers

- **Would CI pass this step?** Yes. GitHub runners have network, the arm's `cargo fetch --locked` supplies each vendored lock set, and the offline test then resolves, as the fresh-`CARGO_HOME` run shows. `--offline` was kept, as the ruling required. `Swatinem/rust-cache` caches `target/`, which now includes `target/vendor-tests`. No `RUSTFLAGS` is set in the workflow, so vendored warnings do not fail it. There is no `.cargo/config.toml` and no `cargo vendor` source replacement anywhere, so nothing else is in play.
- **Would the CI job pass?** Not established, and not because of this range. `cargo test --workspace`, the step before it, failed in a clean clone on `produce::tests::dx_window_snap_matches_the_reference_bound` and `jpeg_extended_12_level_matches_the_reference`, which need the untracked `tools/oracle/out/run.json`. Those tests came in with `d198eed9` (F-X021), are on `ea3e5f6b` and on `sprint/s46`, and are external to F-239.
- **lint-policy probes.** Expects equal to `sprint/s46`, so that part is a change s46 already has. Notes are a conflicting change, S1.
- **The builder precondition.** Correct and the narrowest repair. `vendor-tests` entered the class (its `CARGO_TARGET_DIR=` prefix makes the two tests unseen to the extractor while its four commands are extracted joined from wrapped lines, none of them verbatim), the builder picked it first, and the caller's `next(... if command in runner)` raised StopIteration. The new check uses the same `command in runner` test as the caller, so the builder now returns `bench` again, which its docstring names.

## Verified clean

- **No arithmetic changed.** The range touches no `.rs`, `.wgsl` or `vendor/` file. The coefficient sources hash as at `ea3e5f6b`, and the pins gate is green in the floor.
- **Every factual sentence in the new comments executed**: the dependency-supply sentence (lock contents and the fresh-home run), the lock byte-identity and the inventory rows, the target-directory claim, the sandbox `clean -fdxq` wiping the target (`sandbox.py:258`), the `silent=` and census registration (census green, 0 watched by nothing), the budget's `gates_declared` 36.
- **Full floor**: 29 of 29 green, 0 skipped, with `node_modules` from `npm ci` in a scratch clone. GLM's "26 passed and 3 skipped" was a missing-`node_modules` artefact, the three are green here.
- **Deep profile**: green, including both changed lint-policy probes.

## Limitations and handoff

- No real CI run. CI was reasoned from `ci.yml` and the fresh-`CARGO_HOME` simulation.
- The second floor run mirrored the worktree's two data links. The first run without them is the evidence for the pre-existing `ocelli-oracle` CI hazard above, which the lead may want filed separately against F-X021.
- No oracle, GPU or browser job was run. The deep profile's `viewer-browser.*` probes are `node -e` calls into the check functions and launch no Chromium, by their own `limit` text at `catalogue.py:13450`.
- Both scratch clones and the fresh `CARGO_HOME` are deleted with Python `shutil` after this report.
- Next owner: implementation takes S1's verbatim note copy, then a fresh pass 6 reviews that change.

## Self-review catch figure

GLM should have caught **1 of 1** blocking findings. `sprint/s46` had both the `3` expects (`d514da30`, 2026-11-07 00:01) and the settled notes (`1a52c504`, 23:14) before GLM's commit at 2026-11-08 01:47, and one `git show sprint/s46:` of the two blocks shows them.
