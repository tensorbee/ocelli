# F-238 review, pass 4

**Reviewed**: `4d65d665..42b0931a` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `42b0931a`. Two commits, `7385690a` "F-238, review pass 3 remediation" (5 files) and `42b0931a` "F-238, self-review cycle 2 fixes" (`scripts/guards/catalogue.py` only). Working tree clean apart from the untracked `.venv` and `tools/oracle/out`. Pass 3's findings in `.claude/reviews/F-238-working-pass-3.md` and the "Lead ruling after review pass 3" in `.claude/plans/F-238-design.md` were replayed against this tree.
**Author / reviewer**: GLM in opencode is the author, with self-review cycles 1 to 3 in `.claude/scratch/F-238-self-review-1.md` to `-3.md`. Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write passes 1 to 3.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 3 D1, through the real caller and from a subdirectory.** From `crates/ocelli-core`, `CARGO_TARGET_DIR=target ../../bin/ocelli.sh native` is exit 0, step 6 passes against the existing `ROOT/target` build in 0.06s, and no `crates/ocelli-core/target` is created. So the `cd` at `bin/ocelli.sh:13` does put cargo at ROOT and a relative environment value lands there. Run directly from `crates/ocelli-core`, `cargo metadata` answers `crates/ocelli-core/rel` for `CARGO_TARGET_DIR=rel` and `crates/ocelli-core/rel2` for `CARGO_BUILD_TARGET_DIR=rel2`, so the environment spellings resolve against the current directory. The sentence also covers the configured `build.target-dir` it names one clause earlier, and that spelling does not resolve against the current directory. That is D1 below.
- **Pass 3 S1, mutated.** Scratch copy (`git archive HEAD`, git-initialised locally), the `exit 1` after the non-zero-exit refusal deleted: `runner.step6-exits-without-a-report` HARNESS, "exited 0. The guard it aims at did not fire", harness exit 1. On the intact scratch and in the worktree's `gate guards` it is red for its declared reason and its control is green. Closed.
- **The other three refusal exits, one at a time.** Deleting the Apple tier refusal's `exit 1` turns the new `runner.step6-wrong-tier-over-the-cause-line` HARNESS while `runner.step6-wrong-tier-on-apple` stays red, which is exactly the gap the new probe's note says it closes. Deleting the cause-line exit turns `runner.step6-cause-line-missing` HARNESS. Deleting the off-Apple exit turns `runner.step6-no-tier-off-apple` HARNESS. All four exits at `bin/ocelli.sh:672`, `:680`, `:686` and `:693` are now watched.
- **N1, the floor.** Scratch, `metal` added to `ocelli-compute`'s native wgpu table: `decode_graph_check.py` exit 0. `vulkan` removed instead: exit 1, "ocelli-compute native wgpu drops a default backend". A floor, as the comment and D-33 now say.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh gate native` from ROOT | worktree HEAD `42b0931a`, `CARGO_TARGET_DIR` unset | now | 0, `ALL GREEN 1 gate(s)`, step 6 passes |
| `CARGO_TARGET_DIR=target ../../bin/ocelli.sh native` from `crates/ocelli-core` | same | now | 0, step 6 passes, no `crates/ocelli-core/target` |
| `cargo metadata --no-deps` from ROOT and `crates/ocelli-core`, relative env values | worktree, read only | now | resolved against the current directory |
| `cargo metadata --no-deps` in a scratch crate under a `.cargo/config.toml` with `target-dir = "cfgrel"`, run from a subdirectory | scratchpad, no repository input | now | `<project>/cfgrel`, the config's parent, not the current directory |
| same with `CARGO_HOME=<s>/home3` holding `config.toml` with `target-dir = "homerel"` | scratchpad | now | `<s>/homerel`, the parent of `CARGO_HOME`, not the current directory |
| `bin/ocelli.sh gate guards` | worktree HEAD | now | 0, census 1173 refusals in 81 files, 570 probes, 0 watched by nothing, all six step 6 probes red, controls green |
| `guard_probe.py --only runner.*` (7 probes), intact and four exit mutations | scratch export of HEAD's tracked files | now | as under Acceptance challenged |
| `decode_graph_check.py`, compute table plus `metal` and minus `vulkan` | same scratch | now | 0 and 1 |
| `guard_census.py --check-runbook`, `prose_check.py`, `deviation_check.py`, `test_guard_catalogue.py`, `decode_graph_check.py` | worktree HEAD | now | 0 each, 925 files clean, 33 deviations |
| `wgpu-30.0.1/build.rs:13` to `:35` | registry `index.crates.io-1949cf8c6b5b557f` | now | see Verified clean |
| GLM self-review cycles 1 to 3 | `.claude/scratch/` | prior, not relied on | read for the catch figure and the incident only |

## Defects

### D1, a configured relative `build.target-dir` is said to resolve against the current directory

**Where**: `bin/ocelli.sh:654` to `:658`
**What**: the comment lists `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` and "a configured `build.target-dir`", then says "a relative value is resolved by cargo against the current directory, which is ROOT after the `cd` at the top of this script." That is true of the two environment spellings and false of the config spelling. Cargo resolves a relative path in a config file against the parent of the directory holding that file. Measured in a scratch crate: `.cargo/config.toml` with `target-dir = "cfgrel"`, `cargo metadata` run from a subdirectory answers `<project>/cfgrel`, not `<project>/sub/cfgrel`. A `$CARGO_HOME/config.toml` with `target-dir = "homerel"` answers `<parent of CARGO_HOME>/homerel`, which for the default `~/.cargo` is `~/homerel` and not ROOT.
**Why it is wrong**: microscope section 3, a false factual sentence. Pass 3 rated the earlier wording of this clause a defect for the same reason, naming the wrong mechanism. The step's behaviour is right because it reads `target_directory` from `cargo metadata`, so the error is in the comment only. But it is the remediated line itself, and a maintainer who trusts it would assume a developer's `~/.cargo/config.toml` relative `target-dir` lands under ROOT. GLM's cycle 1 measured both environment spellings from two subdirectories and never measured the third spelling the sentence covers.
**Evidence**: the two scratch `cargo metadata` runs above. The repository has no `.cargo/config.toml`, and this host has no `~/.cargo/config.toml`, so the step is unaffected here.
**Scope and regression**: comment only, HEAD. The lead ruling allowed deleting the clause, and that is the smallest fix: end the sentence at "are just as live", because `cargo metadata` answers every spelling and the step relies on nothing else. The narrower alternative is "a relative `CARGO_TARGET_DIR` or `CARGO_BUILD_TARGET_DIR` resolves against the current directory, which is ROOT after the `cd` at the top, and a relative `build.target-dir` against the directory above the `.cargo` or `CARGO_HOME` that holds it". That adds a claim where deleting removes one. This clause has now drawn a finding in passes 3 and 4.

## Smells

None.

## Nitpicks

- **N1, outside this range.** Pass 3's tree-integrity section says "no stray file was found under `/tmp`". Six `/tmp/f238-*` files exist: `f238-census.out`, `f238-deviation.out`, `f238-prose.out` and `f238-runbook.out` from 20:46, and `f238-meta.json` and `f238-meta.err` from 14:03 (the `.err` holds a `cargo metadata -p` usage error). All predate `4d65d665` (21:32), so none came from this range. Pass 3's glob must have missed them. They are untracked scratch outside the worktree, and I left them in place for the lead to clear.

## Verified clean

- **Pass 3 D1**: the "current directory, which is ROOT after the `cd`" mechanism is right for both environment spellings, from ROOT and from a subdirectory, through `bin/ocelli.sh native` itself. Its remaining overreach is D1 above.
- **Pass 3 D2 closed.** `catalogue.py:12170` names the `Step 6 of the native arm` section above `GUARDS`. The header is at `:7517`, `GUARDS` at `:7696`, and every step 6 builder and fixture sits between them.
- **Pass 3 S1 closed**, mutated as above. The new note's claims hold: `_HONEST_APPLE_REPORT` passes both Apple greps, so with the exit removed the fragment exits 0.
- **The new probe `runner.step6-wrong-tier-over-the-cause-line`** goes red for its declared reason intact and HARNESS with its refusal's exit deleted, while the older wrong-tier probe stays red under that mutation, as its note says. `NoDevice` is a real `DecidedBy` variant (`crates/ocelli-render/src/caps.rs:646`). The `_WRONG_TIER_APPLE_REPORT` comment is accurate.
- **N1 closed**: the floor wording in `decode_graph_check.py:109` to `:114` and `:137` to `:140` and in D-33, mutated both directions.
- **N2 closed**: `wgpu-30.0.1/build.rs:13` gives `webgpu` and `:14` gives `webgl`, both `all(not(native), not(Emscripten), feature)`. With `:15` `dx12` windows only, `:16` `metal` feature-gated, `:17` to `:23` `vulkan` needing `vulkan-portability` on Apple and `:29` to `:35` `gles` needing `angle` on Apple, D-33's "no backend cfg is satisfied there" holds for the shipped macOS set.
- **N4 closed**, as amended by `42b0931a`: the `runner` entry's `refuses` has four phrases for the four refusal exits, and "does not report the NoAdapter tier on an Apple host" covers both the empty and the wrong-tier report, which the same grep refuses.
- The runbook diff is a re-render with one new row and renumbering. `--check-runbook` is 0 and the census matches at 570 probes, one more than pass 3's 569.
- `7385690a`'s message claim that the wrong-tier exit was watched by nothing one step away from S1 is consistent with the mutation above, where the older probe stayed red.
- No em-dash, en-dash or prose semicolon in added lines or either commit message. `prose_check.py` clean. No Rust changed, no `unsafe`, no `wasm-bindgen`, no pixel arithmetic, no dependency change.
- `README.md`, `use.html` and `more.html` are not in the range, and none names step 6, a target directory, the cause line or the floor wording.

## Tree integrity and the disclosed incident

- `git status --short` shows only `?? .venv` and `?? tools/oracle/out`. `git diff HEAD --quiet` is 0. `git fsck --connectivity-only` is 0.
- The reflog from `4d65d665` holds exactly two `commit:` entries, `7385690a` at 22:52:45 and `42b0931a` at 23:10:05, with no reset, amend or rebase. `git log 4d65d665..42b0931a` lists those two and nothing else.
- The `rm -d` aimed at a doubled `target/target` path: `target/target` does not exist. `target/` holds `codec-wasm-plain`, `codec-wasm-simd`, `debug`, `f216-runs`, `release`, `tests`, `tmp` and `wasm32-unknown-unknown`, with `release/ocelli-desktop` present, and step 6 still finds its binary. No `target/f238*`, `stub-target`, `probe-f238-step6.sh`, `rel` or `rel2` exists at ROOT or under `crates/`. Nothing tracked changed. `rm -d` cannot remove a non-empty directory, so the most it could have removed was an empty untracked directory, and nothing shows that it did.
- The canonical checkout's `git status --short` is empty.

## Self-review catch figure

1 of 1 in-range finding was within reach of GLM's self-review. D1: cycle 1 measured the sentence for two of the three spellings it names. One scratch `.cargo/config.toml` would have shown the third resolving elsewhere. N1 predates the range and was not counted. GLM's cycles did find and fix one real issue of their own, the N4 wording, in cycle 2.

## Limitations and handoff

- `gate gpu` was not run, under this pass's no-GPU rule. **AC3** is still owed on the M4 Max with a GPU job allowed. **AC4**'s Linux half (an observed `ubuntu-latest` CI run of step 6) and Windows half are still owed. **AC1** for `x86_64-apple-darwin` beyond `cargo tree` is still owed or a named gap. These are the lead's carried items and this range does not touch them.
- The deep guard profile was not run, because this range touches only floor probes.
- The config-file resolution was measured in a scratch crate and not in the worktree, so that nothing was written under the repository.
- One rule slip by this reviewer: a scratch `config.toml` I had just written under the scratchpad was deleted with `rm -f` and not with `shutil`. Every scratch copy and directory was removed with `shutil.rmtree`: `scratchpad/f238-p4`, `cfgproj`, `home2` and `home3`. Logs are in `scratchpad/p4logs`. Three `ocelli-guard-*` sandboxes from this pass's runs remain under the system temp directory, and the harness's `sweep_stale` removes them.
- Next owner: the implementer fixes D1, preferably by deleting the clause, and then a fresh independent pass 5 reviews that change. This is a bounded review of two commits. It is not acceptance and not sprint readiness.
