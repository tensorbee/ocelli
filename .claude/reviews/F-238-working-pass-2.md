# F-238 review, pass 2

**Reviewed**: `a512a3af..92c5bcd7` on `work/f-238-xai` in `ocelli-wt-f-238`, HEAD `92c5bcd7`, the single commit "F-238, review pass 1 remediation", 15 files. Working tree clean apart from the untracked `.venv` and `tools/oracle/out`. The pass 1 findings in `.claude/reviews/F-238-working-pass-1.md` were replayed against this tree.
**Author / reviewer**: GLM in opencode is the author of the remediation, with three self-review cycles in `.claude/scratch/F-238-self-review-1.md` to `-3.md` (remediation round). Reviewer is Claude Opus 5.5 as an independent subagent of the sprint lead. The reviewer wrote none of the reviewed material and did not write pass 1.
**Result**: 2 defects, 3 smells, 4 nitpicks

## Acceptance challenged

- **Pass 1 S1, build edges.** In a scratch copy, objc2 planted under `[build-dependencies]` of `ocelli-native` with the lock resolved offline: `shipped_native_check.py` exit 1, naming `objc2 v0.6.4` and `objc2-encode v4.1.0` on both Apple triples. The same plant with the walk reverted to `-e normal` exits 0, and the new unit test `test_the_walk_reads_normal_and_build_edges` goes red on that revert. The new deep probe `shipped-native.build-dep-planted` is red with its control green on HEAD, and in a git-initialised scratch copy with the walk reverted to `-e normal` the harness reports `HARNESS ... exited 0. The guard it aims at did not fire`. So the probe discriminates the edge kinds and not only the plant.
- **GLM's dev-edge boundary.** objc2 planted under `[dev-dependencies]` with the lock not updated: exit 1, refused as "did not resolve" because the walk runs `--locked`. Fails closed, as GLM noted. The same plant with the lock updated (one `"objc2"` edge added to `Cargo.lock`) is exit 0, which is the documented dev-edge exclusion. The build-dependency plant with the lock not updated is also refused as unresolved, so the probe's offline `cargo update` is what makes it a test of the edge kinds.
- **Pass 1 S2, step 6 runs the binary.** Scratch copy with its own `CARGO_TARGET_DIR` (an APFS clone of the worktree's target), `probe.rs:282` guard disabled with `if false &&`: `bin/ocelli.sh native` ends at step 6 with `FAIL: the shipped ocelli-desktop exited 134 instead of reporting its tier`, the panic at `wgpu-30.0.1/src/api/instance.rs:65:13`, arm exit 1. `gate native` runs `"$0" native` first (`bin/ocelli.sh:233`), so it goes red with it.
- **Adjacent case for S2.** The step inherits the caller's environment. With `OCELLI_TIER=cpu` exported, `bin/ocelli.sh native` on HEAD exits 1 at step 6 with `Tier Cpu, decided by Override. OCELLI_TIER override: Applied(Cpu).` (S2 below). `OCELLI_TIER=b`, `c` and `bogus` still pass, as each resolves `NoAdapter`.
- **Pass 1 S3.** `crates/ocelli-compute/Cargo.toml` declares no `[features]`, and `decode_graph_check.py` is exit 0 on HEAD, so nothing refuses the removal. Dropping `dx12` from compute's normal table is still refused (`ocelli-compute native wgpu drops a default backend`).
- **Pass 1 N1, the cause line.** The shipped desktop and server binaries print `backends  none compiled for this target`. The dev binary prints no such line. The producer half was then mutated (S3 below).
- **Budget re-record.** Catch-all site counts computed with `discover.discover` and `census.match_sites` over a git-initialised export of each tree. Base `a512a3af`: `catalogue` 82, `runner` 1, `shipped-native` 4. HEAD: 83, 4 and 4. The `catalogue` delta is the one new `raise SandboxError(` in `_plant_objc2_build_dependency`, and the `runner` delta is the three new `echo "FAIL` sites in step 6 at `bin/ocelli.sh:662`, `:670` and `:677`. Both recorded numbers are correct counts. What the `runner` move means is S1 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh native` | worktree HEAD `92c5bcd7`, `CARGO_TARGET_DIR` unset | now | 0, fifteen steps |
| `bin/ocelli.sh gate native` | same | now | 0, `ALL GREEN 1 gate(s)` |
| `bin/ocelli.sh gate licences` | same | now | 0 |
| `bin/ocelli.sh gate guards` | same | now | 0, 445 refusal probes red, 44 accept green, 109 controls green |
| `guard_census.py`, `--check-runbook` | same | now | both 0, 1170 refusals in 81 files, 0 watched by nothing |
| `guard_probe.py --profile deep --only shipped-native.build-dep-planted --only shipped-native.metal-planted` | same | now | 0, both red, control green |
| `prose_check.py`, `deviation_check.py`, `decode_graph_check.py` | same | now | 0, 923 files clean, 33 deviations |
| `cargo test -p ocelli-native` | same | now | 9 passed |
| `target/release/ocelli-desktop`, also with `OCELLI_TIER=a`, `ocelli-server` | binaries from the native run above | now | exit 0, `Tier Cpu, decided by NoAdapter`, `backends  none compiled for this target`, `RefusedUnconstructible(A)` under `a` |
| `target/debug/ocelli-desktop` | same | now | exit 0, `Tier A, decided by Benchmark`, `"Apple M4 Max" on Metal`, no backends line |
| `otool -L`, `strings` | same | now | shipped links `libiconv` and `libSystem` only, 0 `objc2` strings. Dev links `Metal.framework` |
| `OCELLI_TIER=cpu bin/ocelli.sh native` | worktree HEAD | now | 1 at step 6, `decided by Override` |
| Scratch mutations: build-dep plant, dev-dep plant both lock states, walk reverted, guard disabled, flag set removed, compute dev `metal` removed, compute normal `metal` added, compute `dx12` dropped | scratch copies of HEAD's tracked files | now | as described in each section |
| `cargo tree` for D1, D4, D5, D6 and wgpu 30.0.1 `build.rs`, `instance.rs`, `wgpu-core` manifest | registry `index.crates.io-1949cf8c6b5b557f`, workspace pin `=30.0.1` | now | see Verified clean |
| GLM remediation self-review | `.claude/scratch/F-238-self-review-1.md` to `-3.md` | prior, not relied on | read for the catch figure only |

## Defects

### D1, the dev test is said to turn the removal of both restores red, and it watches only one

**Where**: `docs/lld/tier-resolution.md:376` to `:380`
**What**: "The native dev-dependency tables of `ocelli-render` and `ocelli-compute` are the test suite's restore, ... and `probe::tests::a_development_build_compiles_at_least_one_backend` is the test that turns the removal of that restore red." The test lives in `ocelli-render` and watches only render's table. With `metal` removed from compute's dev table in a scratch copy, `cargo tree -p ocelli-compute -e features --target aarch64-apple-darwin -i wgpu-hal` shows `wgpu-hal` with only `default` and `portable-atomic`, so no backend at all, and `cargo test -p ocelli-compute` is green (0, 1 and 4 passed across three binaries). No compute test creates a wgpu instance (`git grep` finds no `Instance::` in `crates/ocelli-compute`).
**Why it is wrong**: microscope section 3 and section 4. The sentence is new in this remediation, it replaces pass 1's D1, and it claims a guard covers a removal that nothing observes.
**Evidence**: the scratch mutation above, on HEAD's tracked files.
**Scope and regression**: prose only, present on HEAD. Name render's table as the one the test watches, or drop the claim for compute. Deleting the clause is smaller than explaining it.

### D2, a "prescribed environment" nothing prescribes

**Where**: `bin/ocelli.sh:649` to `:651`
**What**: "`CARGO_TARGET_DIR` is `$PWD/target` in the prescribed environment, unset in CI". No tracked file sets or prescribes it (`git grep CARGO_TARGET_DIR` outside `.claude/reviews` finds only this comment, `docs/spikes/A1-htj2k-route.md` and `scripts/tests/test_licence_check.py`), `AGENTS.md` names no target directory, `~/.config/opencode` sets none, and this reviewer's shell has it unset. GLM's cycle 2 records the sentence as executed "per `AGENTS.md`", which does not say it.
**Why it is wrong**: a false factual sentence in a comment that explains a guard (microscope section 3).
**Evidence**: the greps above. The `case` statement under it is correct for all three shapes, so only the sentence is wrong.
**Scope and regression**: comment only. Say that the variable may be unset, absolute or relative, and drop "prescribed".

## Smells

### S1, three new refusals re-recorded under an entry whose probe watches none of them

**Where**: `bin/ocelli.sh:662`, `:670`, `:677`, `ci/guard-probe-budget.json:325`, `scripts/guards/catalogue.py` entry `runner`
**What**: the `runner` entry claims `bin/ocelli.sh` with `"*"`. Its `refuses` is "A command whose prerequisite is absent" and its only probe is `runner.absent-prerequisite`, which drives `bin/ocelli.sh oracle`. The three step 6 refusals moved its count from 1 to 4 and the budget was re-recorded, with no probe for them and no `limit=` declaring why they need none. The census now prints them in the bucket "belong to an entry carrying one of this harness's 564 probe(s)" and reports "0 watched by nothing", which the harness has not observed.
**Why it matters**: the census's own refusal text for this move says "Add a probe for the new refusal, or declare why it needs none, and re-record in this diff". The re-record was done and the other half was not. Other catch-all entries declare it (`handoff`: "The other twenty-two refusals in this file belong to ..."). The manual mutation above is the only observation of step 6 going red, and it is not repeatable by any gate.
**Smallest regression**: a `limit=` on `runner` naming the step 6 refusals and the reason a sandbox cannot drive them (a release build of `ocelli-native`), or a separate entry with explicit claims and a probe that runs step 6's matching against a stub binary.

### S2, step 6 reads the caller's `OCELLI_TIER`

**Where**: `bin/ocelli.sh:660`
**What**: the binary runs with the caller's environment. `OCELLI_TIER=cpu bin/ocelli.sh native` on HEAD exits 1 at step 6 with `FAIL: the shipped ocelli-desktop did not report Tier Cpu, decided by NoAdapter on an Apple host`, because the report is `Tier Cpu, decided by Override`. Under that override `detect` returns at `probe.rs:263` before the empty-backend guard at `:282`, so the run would not exercise the guard at all even where the grep accepted it (the off-Apple `^Tier ` branch does).
**Why it matters**: `OCELLI_TIER` is the documented operator override (`docs/lld/tier-resolution.md`, "The override's named user"). A developer who exports it gets a red `gate native` with no defect in the tree, and on Linux CI an exported override would turn the step into a check that skips the code it exists for.
**Smallest regression**: run the binary under `env -u OCELLI_TIER`.

### S3, the producer of the new cause line is watched by nothing

**Where**: `crates/ocelli-render/src/probe.rs:283` to `:284`, `crates/ocelli-native/src/lib.rs:380` to `:395`, `bin/ocelli.sh:669`
**What**: with `resolution.evidence.no_backend_compiled = true;` deleted in a scratch copy, `bin/ocelli.sh native` is exit 0 and the shipped binary prints no `backends` line. The new unit test sets the flag by hand, so it pins the report and not the guard. Step 6 greps only `Tier Cpu, decided by NoAdapter`. The one-line deletion leaves an `unused_mut` warning, and deleting the `mut` with it leaves nothing.
**Why it matters**: `docs/lld/tier-resolution.md:366` to `:371` says "only this line tells the macOS user of the shipped build that their hardware is not the problem" and pastes the line into the measured report. A refactor that returns `classify(...)` inline again makes both false with every gate green.
**Smallest regression**: have step 6's Apple branch also require `backends  none compiled for this target`. That extends a refusal S1 already asks to declare.

## Nitpicks

- **N1.** `decode_graph_check.py:135` and D-33 say compute's table "is held to the same shipped set". The check is a floor (`required <= native_features`). With `metal` put back in compute's normal table, `decode_graph_check.py` and `shipped_native_check.py` are both exit 0, measured. No consumer is harmed today because compute is outside the shipped graph and the graph walk would see it the day it joins, so this does not block. "Holds it to at least the shipped set" would be exact.
- **N2.** D-33's "no backend cfg is satisfied there: `metal` is omitted, the `gles` cfg needs `angle` and the `vulkan` cfg needs `vulkan-portability`" reads as complete. The D-27 set also names `dx12` and `webgpu`, unsatisfied on macOS because `dx12` needs `target_os = "windows"` and `webgpu` needs `not(native)` (`wgpu-30.0.1/build.rs`). Trivial, and the three named are the ones that matter.
- **N3.** Step 6 derives the binary path from `CARGO_TARGET_DIR` only. Cargo also honours `CARGO_BUILD_TARGET_DIR` and `build.target-dir`, and under either the step would run `$ROOT/target/release/ocelli-desktop`, missing (fail closed) or stale (could pass). Unconfirmed, because nothing in the repository sets either today.
- **N4.** Six GLM logs remain in the worktree's ignored `target/` (`f238-final-gate-native.log`, `f238-final-guards.log`, `f238-final-lic.log`, `f238-final-native.log`, `f238-gate-native.log`, `f238-native-run.log`). The self-review says its scratch was deleted, which is true of the directories and not of these.

## Verified clean

- **Pass 1 D1 closed.** `cargo tree -p ocelli-native -e features,normal --target aarch64-apple-darwin -i wgpu` shows `wgpu feature "metal"` from `ocelli-render feature "metal"` from `ocelli-native feature "metal"`, and the binary build carries no dev edges. The new mechanism sentence is true. Its last clause is D1 above.
- **Pass 1 D2 closed.** The browser paragraph sits at the end of `### The override's named user` again, and the no-backend section has no dangling referent.
- **Pass 1 D3 closed.** `sysctl -n machdep.cpu.brand_string` prints `Apple M4 Max`, and the sentence names it directly.
- **Pass 1 D4 closed.** `git grep '"metal"' -- '*Cargo.toml'` finds exactly the sites the four-place enumeration groups: render dev (`:65`), compute dev (`:46`), native `default` (`:49`), wasm (`:124`) and oracle (`:37`), and native dev (`:59`). Same text in D-33, `build-targets.md` and `crates/ocelli-render/Cargo.toml`.
- **Pass 1 D5 closed.** `cargo tree -i ocelli-compute --workspace -e all` prints no dependent, and the shipped `ocelli-native` graph's `ocelli-*` members do not include compute. The D-33 command spelling executes.
- **Pass 1 D6 closed.** `wgpu-core-30.0.1/Cargo.toml:60` is `metal = ["wgpu-core-deps-apple/metal"]`, `:227` gates the crate on `target_vendor = "apple"`. The Apple graph holds it with default features (1) and not without (0). `THIRD_PARTY_LICENSES` holds 0.
- **Pass 1 S1, S2, S3, N1, N2 closed** as described under Acceptance challenged. N2's three conditions match `wgpu-30.0.1/build.rs`.
- wgpu API names against the pinned source: `Instance::enabled_backend_features` is `pub const fn` at `wgpu-30.0.1/src/api/instance.rs:108`, and the panic message at `:65` is the one the mutated run printed. Workspace pin is `wgpu = { version = "=30.0.1" }`.
- `TierEvidence::no_backend_compiled` is set false at both `classify` constructors (`caps.rs:778`, `:946`) and true only at `probe.rs:284`. No other crate constructs `TierEvidence`. The `OCELLI_TIER=c` short-circuit precedes the guard and reports `Override`, so it never prints a false cause.
- Step 6's comment sentences other than D2: the `case` handles unset, absolute and relative values, a relative one against `ROOT=$(pwd)` after the `cd` at the top, and CI's `env` sets only `CARGO_TERM_COLOR`.
- The `build-targets.md` rows 5 and 6 match the arm's commands. The regenerated runbook passes `--check-runbook`.
- No em-dash or en-dash in added lines. No `unsafe`, no `wasm-bindgen` outside `ocelli-wasm`, no pixel arithmetic, no new dependency or version.
- `README.md:475` to `:477` is true with build edges now read. `use.html` and `more.html` make no claim this diff touches.

## Self-review catch figure

4 of this pass's 9 findings were within reach of GLM's remediation self-review, and all 4 are blocking (of 5 blocking).

- D1: cycle 3 records D1 as "verified against the manifests and `cargo tree -e features,normal,dev`". One removal of compute's dev `metal`, the same mutation cycle 1 ran for render, shows nothing goes red.
- D2: cycle 2 records the sentence as executed "per `AGENTS.md`", and `AGENTS.md` does not say it.
- S1: the census's own refusal text, which GLM had to clear to re-record the budget, names the missing probe or declaration.
- S3: cycle 1 mutated the report line two ways and never the line in `probe.rs` that sets the flag.
- S2 needs the override's interaction with a gate step, and the nitpicks are judgement calls. A self-review would not be expected to raise them.

## Limitations and handoff

- `gate gpu` was not run. AC3 rests on the dev desktop binary's tier report, not on the `#[ignore]` GPU suite.
- No Linux or Windows host. Step 6 now RUNS the shipped binary in CI on `ubuntu-latest` (`.github/workflows/ci.yml:74`), where it has Vulkan and GL compiled and no GPU. That run has never been observed by this pass, and whether it exits 0 there is unverified. The off-Apple claim that such a machine "prints the same `NoAdapter` and the same zero adapters" is also unverified, because a software adapter such as llvmpipe would be counted.
- No Intel Mac, so `x86_64-apple-darwin` is resolved by `cargo tree` and never linked or run.
- My `OCELLI_TIER=cpu bin/ocelli.sh native` and `cargo test -p ocelli-native` ran in the worktree and wrote only its ignored `target/`. The scratch copies, the cloned target and the export of `a512a3af` were under `scratchpad/f238-p2` and removed with `shutil.rmtree`. Run logs are in `scratchpad/p2logs`, and `scratchpad/sites.py` is this pass's site counter.
- Next owner: the implementer remediates D1, D2 and S1 to S3, then a fresh independent pass 3 reviews the changed material. This is a bounded review of one remediation commit, not acceptance and not sprint readiness.
