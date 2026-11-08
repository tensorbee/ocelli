# F-238 review, pass 6

**Reviewed**: `ffdddb00..b7201303` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `b7201303`. One commit, "F-238, review pass 5 S1, step 6 runs the binary its build named", touching `bin/ocelli.sh` (step 6), `scripts/guards/catalogue.py` (two probes, the stub build, the runner entry's `refuses` and `limit`), `ci/guard-probe-budget.json` (`runner` 5 to 6) and `docs/runbooks/guard-verification.md` (re-rendered). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`.
**Author / reviewer**: GLM in opencode is the author of the commit. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write passes 1 to 5.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

- **Pass 5 S1, replayed end to end through the real step.** Step 6's own lines were extracted from `bin/ocelli.sh` at `b7201303` and at `ffdddb00` and run in a scratch clone against real cargo, with a scratch `CARGO_TARGET_DIR` whose `release/ocelli-desktop` was replaced by a stale stub that prints a tier A report and touches a marker. With `CARGO_BUILD_TARGET=aarch64-apple-darwin`, the new step built cold into `aarch64-apple-darwin/release/`, ran `.../aarch64-apple-darwin/release/ocelli-desktop`, passed, and the stale marker was never touched. Warm (every artifact `fresh: true`) it did the same. The old step under the same environment ran the stale stub and refused with "did not report Tier Cpu, decided by NoAdapter". Closed.
- **The two new probes, against the old logic and three mutants.** In a scratch clone, `bin/ocelli.sh` was reverted to `ffdddb00` (step 6 is the only hunk in the range), and each mutation below was applied alone and reverted:

  | Step 6 variant | `target-triple-over-a-stale-binary` | `no-named-executable` |
  |---|---|---|
  | tip `b7201303` | green | red, at its refusal |
  | old `target_directory` path | HARNESS, the stale dev report refused | HARNESS, exited 0 |
  | new parse, falling back to the metadata path when nothing is named | green | HARNESS, exited 0 |
  | new parse, `-z` refusal deleted | green | HARNESS, exited 127 at the wrong refusal |
  | new parse, name filter deleted (picks `ocelli-server`) | controls go red, exit 127 | controls go red |

  And the fixture's own faithfulness: deleting the stub build's `CARGO_BUILD_TARGET` branch turns the accept probe red with "the stub build did not honour CARGO_BUILD_TARGET", so the probe cannot go green over a state it never built.
- **The parse, against real cargo streams.** The step's Python was extracted verbatim and fed real `cargo build` output from a scratch crate with two bins named `ocelli-desktop` and `ocelli-server`, a lib, a build script emitting `cargo:warning` and unused-variable warnings. `--message-format=json` (three `compiler-message` lines interleaved on stdout), a warm rerun, and `json-render-diagnostics` with a non-JSON line interleaved before every message all yield exactly `release/ocelli-desktop`. The real `ocelli-native` stream under the triple holds one `ocelli-desktop`, one `ocelli-server`, 16 `build-script-executed` and one `build-finished`, and the parse picks the desktop one. Zero is refused by the step. Two is not, see S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff ffdddb00..b7201303` | worktree | now | 4 files, `bin/ocelli.sh` changes step 6 only |
| `bash -n bin/ocelli.sh` | HEAD | now | 0 |
| `bin/ocelli.sh gate native`, scratch `CARGO_TARGET_DIR`, cold | HEAD | now | 0, `ALL GREEN 1 gate(s)`, step 6 passed |
| `bin/ocelli.sh gate guards`, scratch `CARGO_TARGET_DIR` | HEAD | now | 0, all eight `runner.step6-*` probes as declared, `target-triple-over-a-stale-binary` green, the other seven red |
| `guard_census.py` | HEAD | now | 0, 1174 refusals, 572 probes, 0 watched by nothing |
| `guard_census.py --check-runbook` | HEAD | now | 0 |
| `deviation_check.py` | HEAD | now | 0, 33 deviations |
| `prose_check.py` | HEAD | now | 0, 927 files clean |
| `python3 -m unittest scripts.tests.test_guard_catalogue` | HEAD | now | 0, 57 tests |
| `guard_probe.py --only` the two new probes plus `exits-without-a-report` | scratch clone, tip, old step, three mutants, one fixture mutant | now | as in the table above |
| step 6 extracted, real cargo, `CARGO_BUILD_TARGET=aarch64-apple-darwin`, stale stub at `release/` | scratch clone at `b7201303` and `ffdddb00` | now | new 0 cold and warm, ran the triple binary, stale marker absent. Old 1, ran the stale stub |
| verbatim parser on scratch crate streams | scratchpad crate, cargo 1.97.1 | now | one desktop path for json, warm json, render-diagnostics with junk. Two-target `build.target` stream: prints the first of two, exit 0 |
| `git status`, `git fsck --no-dangling`, `git ls-tree -r b7201303 \| wc -l` against `git ls-files \| wc -l` | worktree | now | clean, fsck silent, 1911 and 1911, `git diff b7201303` empty |

## Defects

None.

## Smells

### S1, step 6 takes the first of several `ocelli-desktop` artifacts and passes

**Where**: `bin/ocelli.sh:657` to `:671`, the `break` at `:670`
**What**: the parse prints the first `compiler-artifact` named `ocelli-desktop` with an `executable` and stops. Cargo emits one such message per requested target, and a `build.target` array in any cargo config (or `--config`) is a multi-target build that cargo accepts on stable. Measured in a scratch crate with `--config 'build.target=["aarch64-apple-darwin","wasm32-unknown-unknown"]'`: the stream names `aarch64-apple-darwin/release/ocelli-desktop` and `wasm32-unknown-unknown/release/ocelli-desktop.wasm`, the verbatim parse prints the first and exits 0, and the step would then run that one and pass with the other unchecked. Which one comes first is the order cargo finishes the units, so which binary step 6 checks is not determined by anything the step controls. If the first is not runnable on the host, the step reports "exited 126 instead of reporting its tier" (126 measured for the `.wasm`), which names the wrong cause, the same wrong-cause shape pass 5 recorded for S1's no-earlier-build case.
**Why it is a smell**: the comment and the commit say step 6 runs "the" binary the build produced, which holds only while exactly one is named. The lead's acceptance for this pass asks for a refusal on two as well as on zero. It needs a multi-target cargo config that the repository and this host do not set, so nothing fails today.
**Scope and regression**: introduced in this range. Fix: collect every matching `executable` and print it only when there is exactly one, refusing otherwise with the count, for example "the shipped build named N ocelli-desktop executables, expected exactly one". Regression: a probe whose stub build's JSON names two `ocelli-desktop` executables under the triple directory and expects that refusal, against the existing triple control. The existing `no-named-executable` probe already covers zero. Not run through `bin/ocelli.sh` with a real multi-target `ocelli-native` build, because only `aarch64-apple-darwin` and `wasm32-unknown-unknown` are installed and `ocelli-native` is native only.

## Nitpicks

### N1, the `stale_report` docstring overstates when the build overwrites the plant

**Where**: `scripts/guards/catalogue.py:7629` to `:7631`
**What**: "only meaningful alongside an exported `CARGO_BUILD_TARGET`, without which the build would overwrite it." With `build_names_desktop=False` the stub build writes no desktop binary, so without a target it would not overwrite the plant. Both probes that use `stale_report` export the target, so nothing is affected. "without which a build that writes the desktop binary would overwrite it" is exact.

## Verified clean

- **Pass 5 S1 is closed** end to end through the real step and real cargo, cold and warm, and both new probes are red on the old path logic and green on the fix, measured rather than read.
- **The new refusal fires**: `runner.step6-no-named-executable` is red at "named no ocelli-desktop executable" on the tip, and deleting the refusal or falling back to the metadata path turns it HARNESS. Its control, the triple build that names the desktop binary over the same stale plant, is green.
- **The accept probe observes the run rather than the exit**: the marker check and the fixture faithfulness check each hold under mutation, so an exit 0 that ran nothing, or a stub that stopped honouring the triple, cannot read as green.
- **The parse** skips non-JSON lines, `compiler-message`, `build-script-executed` and `build-finished`, ignores `ocelli-server` and libs (whose `executable` is null and whose names use underscores), and accepts `fresh: true` artifacts. `json-render-diagnostics` sends rendered warnings and `Compiling` lines to stderr, so a developer still sees them and stdout carries only JSON. A failing `cargo build` still stops the step through `set -euo pipefail` on the assignment, as before.
- **The comment above the step** (`bin/ocelli.sh:653` to `:656`) says what the step does, which binary it takes and that it refuses on none, and nothing about how any target dir spelling resolves. Its "the one" reading is accurate except under S1.
- **No other consumer** of the deleted `shipped_target` variable exists in `bin`, `scripts`, `docs`, `ci` or `.github`.
- **Catalogue prose**: the runner entry's `refuses` names the new refusal, the `limit` describes the stub's triple and message-format behaviour, both probe notes describe their input and control as the code builds them, and the census and runbook agree (`--check-runbook` 0, `entry_sites.runner` 6).
- **The tree is intact after GLM's disclosed `rm -f`**: worktree clean, `git diff b7201303` empty, `git fsck --no-dangling` silent, 1911 tracked paths matching the commit's tree, `ls /tmp | grep -ci f238` is 0.
- No em-dash, en-dash or prose semicolon in the added lines or the commit message. No Rust, dependency, `unsafe`, `wasm-bindgen` or pixel arithmetic in the range. `README.md`, `use.html` and `more.html` are not touched and describe nothing this range changed.

## Limitations and handoff

- S1 was reproduced with the verbatim parse on a scratch crate's real two-target stream, not through step 6 with a multi-target `ocelli-native` build, which this host cannot produce.
- `gate gpu` and the oracle were not run, under this pass's no-GPU rule. AC3, AC4's Linux and Windows halves and AC1 for `x86_64-apple-darwin` remain the lead's carried items, untouched by this range.
- Scratch clones, crates and target directories lived under the session scratchpad and were removed with `shutil.rmtree`. Sandboxes were created under a scratchpad `TMPDIR`. Nothing was written under `/tmp` or the worktree except this file.
- Self-review: GLM's self-review should have caught 1 of 2 findings, S1, because the `break` on first match is visible in the parse it wrote and its own commit message claims the step reads "the" executable.
- Next owner: the lead rules on S1, then a fresh independent pass 7 reviews that change. This is a bounded review of one commit, not acceptance and not sprint readiness.
