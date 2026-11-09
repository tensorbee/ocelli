# F-238 review, pass 5

**Reviewed**: `42b0931a..040e7f7b` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `040e7f7b`. One commit, "F-238, review pass 4, the target dir clause deleted", touching `bin/ocelli.sh` (one clause deleted from the comment above step 6), `.claude/plans/F-238-design.md` (the "Lead ruling after review pass 4") and `.claude/reviews/F-238-working-pass-4.md` (added). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`.
**Author / reviewer**: the sprint lead, a Claude session, is the author. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write passes 1 to 4.
**Result**: 1 defect, 1 smell, 0 nitpicks

## Acceptance challenged

- **Pass 4 D1, replayed.** The clause "a relative value is resolved by cargo against the current directory, which is ROOT after the `cd`" is gone from `bin/ocelli.sh:652` to `:656`. The remaining comment makes no claim about how any spelling resolves, so the configured `build.target-dir` case that falsified it has nothing left to falsify. Closed.
- **Every remaining sentence, measured per spelling.** In a scratch binary crate (cargo 1.97.1), `cargo metadata`'s `target_directory` was read and then `cargo build --release` was run under the same environment, and the binary was looked for at `<target_directory>/release/b`. Unset, absolute `CARGO_TARGET_DIR`, relative `CARGO_TARGET_DIR`, relative `CARGO_BUILD_TARGET_DIR` and a configured `build.target-dir` all agree: metadata names the directory the build wrote to. So "the target directory cargo itself resolved for the build above", "not from a fixed relative path" and "just as live" are true for all five.
- **The ruling's resolution claim, measured.** See D1.
- **An adjacent cargo knob the comment does not name.** A configured `build.target` leaves `target_directory` unchanged but moves the binary to `<target_directory>/<triple>/release`. See S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 42b0931a..040e7f7b` | worktree | now | 3 files, `bin/ocelli.sh` is a comment-only change, 1 clause deleted |
| `bash -n bin/ocelli.sh` | HEAD `040e7f7b` | now | 0 |
| `bin/ocelli.sh gate native` from ROOT | HEAD, `CARGO_TARGET_DIR` unset | now | 0, `ALL GREEN 1 gate(s)`, step 6 passes in 0.07s |
| `bin/ocelli.sh gate guards` | HEAD | now | 0, all six `runner.step6-*` probes red, controls green |
| `guard_census.py` | HEAD | now | 0, 749 + 395 + 29 = 1173 refusals, 570 probes, 0 watched by nothing |
| `guard_census.py --check-runbook` | HEAD | now | 0 |
| `prose_check.py` | HEAD | now | 0, 926 files clean |
| `deviation_check.py` | HEAD, it scans `.claude/plans` and `.claude/reviews` | now | 0, 33 deviations |
| scratch lib crate, `.cargo/config.toml` `target-dir = "cfgrel"`, `cargo metadata` from `proj/sub` and `proj` | scratchpad, no repository input | now | `proj/cfgrel` both times |
| same, `CARGO_HOME=h/.cargohome` holding `target-dir = "homerel"`, no project config | scratchpad | now | `h/homerel` |
| same, env overrides: `CARGO_TARGET_DIR=rel`, `CARGO_BUILD_TARGET_DIR=rel2`, both set, absolute | scratchpad | now | `proj/sub/rel`, `proj/sub/rel2`, `CARGO_TARGET_DIR` wins, absolute verbatim |
| scratch bin crate, metadata then `cargo build --release`, five spellings plus `build.target` | scratchpad | now | as under Acceptance challenged and S1 |
| `ls /tmp` for `f238` | host | now | 0 matches |

## Defects

### D1, the ruling names the wrong base directory for a configured `build.target-dir`

**Where**: `.claude/plans/F-238-design.md:159`
**What**: "A configured `build.target-dir` resolves against its config file's parent." The parent of `proj/.cargo/config.toml` is `proj/.cargo`, and of `h/.cargohome/config.toml` is `h/.cargohome`. Cargo resolves against the directory above those, `proj` and `h`. Measured: `target-dir = "cfgrel"` in `proj/.cargo/config.toml` answers `proj/cfgrel`, not `proj/.cargo/cfgrel`, and `target-dir = "homerel"` in `$CARGO_HOME/config.toml` with `CARGO_HOME=h/.cargohome` answers `h/homerel`, not `h/.cargohome/homerel`. Pass 4 stated the rule correctly ("the parent of the directory holding that file"), and the ruling paraphrased it one level short.
**Why it is wrong**: microscope section 3, a false factual sentence in a plan. It is the same clause that drew findings in passes 3 and 4, now restated in the ruling that removes it.
**Evidence**: the two scratch `cargo metadata` runs in the table.
**Scope and regression**: plan prose only, HEAD. No behaviour depends on it. The smallest fix is to delete the sentence's first half and keep "The step reads `target_directory` from `cargo metadata`, which is right for each", which matches how this clause was retired in `bin/ocelli.sh`. The alternative is "against the directory above the one holding its config file".

## Smells

### S1, step 6 runs a stale binary when `build.target` is configured

**Where**: `bin/ocelli.sh:657` to `:659`, unchanged in this range, F-238's own step
**What**: `shipped_bin` is `<target_directory>/release/ocelli-desktop`. With `CARGO_BUILD_TARGET` set or `build.target` in any cargo config, `cargo build --release` writes to `<target_directory>/<triple>/release/` and `target_directory` does not change. Measured in a scratch bin crate: after an unset build, `build.target = "aarch64-apple-darwin"` produced `target/aarch64-apple-darwin/release/b`, and the step's path shape still found the older `target/release/b` and reported OK. In this worktree `target/release/ocelli-desktop` exists, so step 6 would run and pass on whatever binary was last built without a target, not the one it just built. With no earlier build it would exit 127 and report "exited 127 instead of reporting its tier", which names the wrong cause.
**Why it is a smell**: the step exists to check the binary the shipped configuration just produced (its own comment, "The shipped configuration LINKS and RUNS"). A stale pass is the quietly-wrong shape CLAUDE.md names. It needs a developer-side `build.target` or `CARGO_BUILD_TARGET`, which the repository and this host do not set, so nothing fails today.
**Scope and regression**: not introduced by this range, present on HEAD. Earlier passes covered the target-dir spellings and not the target-triple one. Fix, either of: take the path from the build itself, `cargo build ... --message-format=json-render-diagnostics` and read the `executable` of the `compiler-artifact` whose target name is `ocelli-desktop`, which is right for every spelling and needs no comment about resolution, or refuse step 6 when `cargo metadata`'s host build would not land in `release/` (for example when `CARGO_BUILD_TARGET` is set). Regression: a probe in `catalogue.py` that sets `CARGO_BUILD_TARGET` to the host triple over a stale `release/ocelli-desktop` stub and expects step 6 to run the fresh one or refuse. Unconfirmed through `bin/ocelli.sh` itself, because that would rebuild `ocelli-native` into the worktree's target directory.

## Nitpicks

None.

## Verified clean

- **Pass 4 D1 closed** in `bin/ocelli.sh`. The remaining comment is two sentences, and each was measured for unset, absolute and relative `CARGO_TARGET_DIR`, relative `CARGO_BUILD_TARGET_DIR` and a configured `build.target-dir` in a scratch crate, with metadata and build agreeing every time.
- **Nothing else changed behaviour.** The only `bin/ocelli.sh` hunk is inside a `#` comment. The other two files are Markdown under `.claude/`, read by `prose_check.py`, `deviation_check.py` and the guard discovery, all of which pass. `bash -n` is 0, `gate native` and `gate guards` are green, and the census still counts 570 probes with 0 watched by nothing, the same as pass 4.
- **The ruling's other claims hold.** Pass 3's ruling did allow "or the clause goes" (`.claude/plans/F-238-design.md:151`). The step does read `target_directory` from `cargo metadata`. The `/tmp/f238-*` files pass 4 found are gone, `ls /tmp | grep -c f238` is 0.
- **Pass 4's review file** is added verbatim as the reviewer's record and makes no new claim about the tree beyond what pass 4 measured.
- No em-dash, en-dash or prose semicolon in the diff or the commit message. No Rust, dependency, `unsafe`, `wasm-bindgen` or pixel arithmetic in the range. `README.md`, `use.html` and `more.html` are not touched and name nothing this range changed.

## Limitations and handoff

- S1 was reproduced in a scratch crate and not through `bin/ocelli.sh native` with `CARGO_BUILD_TARGET` set, to avoid writing a second release build under the worktree.
- `gate gpu` and the oracle were not run, under this pass's no-GPU rule. AC3, AC4's Linux and Windows halves and AC1 for `x86_64-apple-darwin` remain the lead's carried items, untouched by this range.
- Scratch crates were removed with `shutil.rmtree`. Logs are in the session scratchpad `p5logs`. No file under `/tmp` was written by this pass outside the session scratchpad.
- Next owner: the lead fixes D1 and rules on S1, then a fresh independent pass 6 reviews that change. This is a bounded review of one commit, not acceptance and not sprint readiness.
