# F-233 review, pass 1

**Reviewed**: `git diff 0d812e0b HEAD` on `work/f-233-xai`, HEAD `1f380905` ("F-233, SIMD wasm builds compile SIMD code"), tree `3df2a8c4f3b0a9bc7d8f1fe3058c8bd0aac1fb02`, worktree clean before the pass. Stacked on F-232 `0d812e0b`, which is under its own review and is not reviewed here.
**Author / reviewer**: implementation by GLM-5.3 in opencode (its report, the end of the session's `glm-F-233.log`). Reviewer is a Claude subagent run by the sprint lead as `/microscope F-233 --working`, pass 1. The reviewer wrote none of the reviewed material. The lead wrote the design plan, so this pass does not independently approve the plan's decisions, only the implementation against them and the plan's premises where the implementation inherits them.
**Result**: 3 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **AC1 and AC2**, reproduced independently with `cargo build -v` into fresh scratch target dirs. Correct.
- **AC3**, challenged with a build mutation rather than planted bytes, and with a wrong flag. The check passes both. It cannot tell a crate that got `+simd128` from one that did not (D1).
- **AC4**, reproduced. GLM's explanation of why `openjph_core` is not named holds, and it generalises into D1.
- **Decision 3**, the newest-rlib rule, challenged with the cargo sequence a developer's reused SIMD dir actually goes through. It false-passes (D2).
- **AC6 and the D-22 sentence**, challenged by counting the SIMD instructions in the final modules before and after F-233. The count is the same, so the "recheck" exercises no SIMD code the pre-F-233 build lacked (D3).
- **AC7**, shipped module hash and size before and after `bin/ocelli.sh native`. Unchanged.

## Evidence

All scratch builds below used target dirs under the session scratchpad `f233/`, deleted at the end. Gate runs used `CARGO_TARGET_DIR=<wt>/target`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo build -v` plain and `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS=... cargo build -v` SIMD, `verify_htj2k` then `verify_jpeg2000`, fresh dirs `plain` and `simd` | HEAD | now | all 0. htj2k plain: 45 wasm32 rustc invocations, 0 with the flag, 15 host, 0 with it. htj2k SIMD: 45 wasm32, 45 with the flag, 15 host, 0 with it. jpeg2000 then rebuilt only the example (1 invocation, flag present in SIMD only) |
| `simd_variant_check.py plain simd <five>` | fresh dirs | now | 0, OK for all five |
| Same check against `cargo rustc ... -- -C target-feature=+simd128` (the pre-F-233 command), dir `simd-old` | fresh dir | now | 1, names `jxl_grid`, `jxl_render`, `jxl_modular` only. AC4 and GLM's report reproduced |
| `strings libthiserror-*.rmeta` in `plain` | fresh dir | now | embeds `<plain>/wasm32-unknown-unknown/release/build/thiserror-.../out/private.rs`, the OUT_DIR path. GLM's leak explanation confirmed |
| rlib members, `plain` vs `simd-old`, `openjph_core` and `ocelli_codec` | fresh dirs | now | `lib.rmeta` differs, the `/0` code member is byte-identical (775,004 and 652,584 bytes). That member starts `42 43 c0 de`, LLVM bitcode, because the release profile is `lto = "fat"` and cargo passes `-C linker-plugin-lto` to every dependency. There is no wasm object code in any listed rlib |
| rlib file names, `plain` vs `simd` | fresh dirs | now | every listed crate has a different metadata hash in its file name (`openjph_core-02998e3d…` vs `-dd31055e…`), so `-C metadata` differs and every rlib's bytes differ whenever the SIMD dir's RUSTFLAGS differ at all |
| **Mutation 1**: `RUSTC_WRAPPER` stripping `-C target-feature=+simd128` from `jxl_grid` and `openjph_core` | HEAD, dir `simd-mut` | now | build 101: `jxl_render`'s wasm32 SIMD impls fail to compile against a non-SIMD `jxl_grid`. That crate is self-detecting |
| **Mutation 2**: the same wrapper stripping the flag from `openjph_core`, `ocelli_codec`, `jxl_render`, `jxl_modular`, wrapper log confirms each stripped once | HEAD, dir `simd-mut` | now | build 0, **check 0, "OK: simd variant rlibs differ" for all five**. The final module is byte-identical to the pre-F-233 `simd-old` module (`2f9c0c7c…` both) |
| **Wrong flag**: `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+bulk-memory"` | HEAD, dir `simd-wrongflag` | now | build 0, **check 0 for all five**, module has 0 SIMD instructions |
| **Stale dir**: `simd-old` (pre-F-233 rlibs), then the F-233 SIMD build, then the pre-F-233 `cargo rustc --` command again | HEAD | now | after F-233 build: check 0. Regressed build: "Finished in 0.08s", nothing recompiled, it links the old `libjxl_grid-b96ecb…` (identical to plain). **Check 0** because the newest mtime is the flagged `-576bf6…` |
| `wasm-dis --all-features`, SIMD opcode count per final module | dirs `plain`, `simd`, `simd-old`, `simd-mut` | now | `verify_htj2k`: plain 0, F-233 SIMD 43, pre-F-233 SIMD 43, mutated 43. Mix identical in both SIMD builds: 2 `v128.const`, 18 `v128.load`, 23 `v128.store`, no lane arithmetic. `verify_jpeg2000`: F-233 and pre-F-233 both 9 `v128.const`, 67 `v128.load`, 62 `v128.store`, 8 `i8x16.eq`, 8 `i8x16.all_true`, 1 `v128.load64_lane`, 1 `v128.load64_zero`. No `f32x4` op in any module, so none of jxl's wasm32 SIMD regions is linked into either example |
| Scratch-copy mutations of `simd_variant_check.py`: `max`→`min` on mtime, identity comparison disabled, missing-side `continue` disabled | copies | now | unit suite red each time (2 failures, 2 failures, 2 errors). Tracked file untouched |
| `cargo fmt --all --check` | HEAD | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| shipped module hash/size before, after `bin/ocelli.sh wasm`, after `bin/ocelli.sh native` and a second `wasm` | HEAD | now | `pkg` `008aac80ef24f8b0`, 2,179,571 bytes, `pkg-decode` `07cf6db9ba05bd09`, 16,455 bytes, at all three points |
| `pin_and_size_check.py --with-size`, after each `wasm` | HEAD | now | 0 both times, decode 16,455 against baseline 16,455 |
| `bin/ocelli.sh native` | HEAD | now | 0, 13 steps, both "OK: simd variant rlibs differ" lines, four `executed` lines |
| `python3 -m unittest scripts/tests/test_simd_variant_check.py` | HEAD | now | 0, 7 tests |
| `bin/ocelli.sh gate native` | HEAD | now | 0 |
| `gate guards`, `gate unsafe`, `gate ci`, `deviation_check.py`, `guard_census.py`, `--check-runbook`, `prose_check.py`, `source_provenance_check.py` | HEAD | now | `gate unsafe` 0, `gate ci` 0, deviation 0 (28 deviations), census 0, runbook 0, prose 0 (890 files), provenance 0 (1752 files). `gate guards` first run 1 because the reviewer wrote this file while the harness ran ("untracked changed while the harness ran", the reviewer's fault and not the diff's). Rerun with the file already present: 0, 438 refusal probes, `simd-variant.identical-rlib` red, 102 controls green |

## Defects

### D1, the rlib comparison cannot detect a crate the flag did not reach

**Where**: `scripts/simd_variant_check.py:34-85`, run at `bin/ocelli.sh` steps 8 and 12, documented at `docs/lld/build-targets.md` rows 8 and 12, `docs/SOURCE-POLICY.md:410-413`, the D-22 sentence, and catalogue guard `simd-variant`.
**What**: the check hashes whole rlibs. Two causes make those bytes differ whether or not `+simd128` reached a crate. Cargo folds the target RUSTFLAGS into `-C metadata`, so any value at all renames and rehashes every rlib in the SIMD dir. And thiserror's OUT_DIR path is embedded in its rmeta, which changes the SVH of everything downstream of it, so `openjph_core` and `ocelli_codec` differ even under the pre-F-233 command. The code member it would need to read is LLVM bitcode, not object code, because of `lto = "fat"`.
**Why it is wrong**: the story row says the step "refuses a SIMD variant whose dependency rlibs match the plain build" as the proof that every crate got the flag (design decision 3, AC3, and the step 8 and 12 echo "compiles every listed crate with the flag"). A check that passes when its property is false is microscope section 4's defect class.
**Evidence**: mutation 2 above takes the flag away from four of the five listed crates and the check prints OK for all five. The wrong-flag build has zero SIMD instructions and the check prints OK for all five. The only state it rejects is the exact pre-F-233 command, and only for the three jxl crates. Mutation 1 shows `jxl_grid` going without the flag already breaks the build.
**Scope and regression**: present on HEAD, and every consumer that cites steps 8 and 12 as proof inherits it. Smallest meaningful regression: a test that builds, or plants from a real build, a SIMD dir where one listed crate lacked the flag and asserts the check refuses that crate. The planted-bytes unit tests and the level-1 probe can never be that test, because they make the bytes identical by construction. Possible discriminating sources are the per-unit rustc command line (`cargo build -v`, or the unit's fingerprint) or the target-features carried by the crate's code. Measure which one works before writing it.

### D2, the newest-rlib rule false-passes on a reused SIMD dir

**Where**: `scripts/simd_variant_check.py:34-54` (`newest_rlib`), its docstring, and `test_a_stale_rlib_does_not_mask_a_fresh_identical_one`.
**What**: the docstring says a regression that lost the flag "would write plain's own rlib beside a stale differing one", so the newest file is "the one the build just wrote". Cargo does not do that. A regressed command maps back to the pre-F-233 metadata hash, finds that unit's fingerprint fresh, and recompiles and touches nothing. The rlib that gets linked is then the older one, and the newest by mtime is the stale flagged one.
**Why it is wrong**: decision 3, as GLM records it, exists to stop exactly this false pass. The unit test models a cargo behaviour that was never measured, which breaks the rule that tests come from the specification or from observation and not from the implementation's model.
**Evidence**: the stale-dir row above. After the regressed build, "Finished in 0.08s" and check exit 0, while the example linked `libjxl_grid-b96ecb…`, byte-identical to plain's. The worktree's own `target/codec-wasm-simd` holds both hash pairs today, so this is the state of any machine that ran the pre-F-233 arm.
**Scope and regression**: developer machines with a reused `target/codec-wasm-simd`. A fresh CI runner has no stale pair. Whatever replaces D1's comparison should name the unit the final link actually used, for example from the example's dep-info or fingerprint, not the newest file. The regression test is this three-build sequence.

### D3, the D-22 recheck sentence implies new SIMD evidence the measurement does not show, and two new sentences are false

**Where**: `docs/hld/DEVIATIONS.md:39` (D-22, F-233 sentence), `docs/SOURCE-POLICY.md:410-415`, `scripts/guards/catalogue.py:11276-11279`, `scripts/tests/test_simd_variant_check.py:15`, `bin/ocelli.sh` step 5 comment ("Steps 6 and 9 run that exact source as plain and SIMD-enabled wasm under Node").
**What**:
1. D-22 now says F-X013's leg "was compiled with the flag on the example crate only" and that F-233 "rebuilt it with the flag on every crate", and offers that as the recheck. Under `lto = "fat"` every dependency rustc emits bitcode only, and codegen for all of them runs once, inside the example's rustc, which had `+simd128` before F-233 too. Measured: the F-233 HTJ2K SIMD module and the pre-F-233 one hold the same 43 SIMD instructions in the same mix, all `v128` loads, stores and constants, with no lane arithmetic. JPEG 2000 matches too. The only thing the flag changes before LTO is `cfg(target_feature = "simd128")` source selection, which `openjph-core` does not have (design fact 2), and jxl's gated regions are not linked into either example. So the recheck exercises no SIMD code F-X013's leg lacked, and the sentence leaves that out. It also cites the rlib comparison as part of the recheck, and D1 shows that comparison proves nothing.
2. The catalogue note says the missing-side refusals are driven by `test_simd_variant_check.py`, "run by the guards gate". The guards arm does not run it. Only `gate native` does, at `bin/ocelli.sh:216-217` (`git grep test_simd_variant_check`).
3. `test_simd_variant_check.py:15` says the layout is the one "steps 7 and 10 produce". After the renumbering, step 10 is the native HTJ2K run and the HTJ2K build is step 11.
4. The step 5 comment says steps 6 and 9 run the source "as plain and SIMD-enabled wasm under Node". Step 6 runs it natively.
**Why it is wrong**: microscope section 3. D-22 is the evidence clause of a standing deviation, and a reader takes from it that F-X013's SIMD leg was weaker than it was and that F-233 strengthened it. The last three are false sentences in new material.
**Evidence**: the `wasm-dis` rows above and the `git grep` and gate-arm reads.
**Scope and regression**: present on HEAD. Prefer deleting to explaining. The smallest correct D-22 sentence states what was measured: the flag now reaches every crate, the SIMD modules carry the same SIMD instructions as before because fat LTO had already code-generated every crate with the example's flag, and `.201` and `.202` stay exact with `.203` unchanged. The same premise sits in F-229 pass 8's line 65 and the design plan's decision 4. Those are review and plan text outside this diff, and the lead should know their premise is false.

## Smells

None.

## Nitpicks

- `simd_variant_check.py:60-62` says "the same name with the same digest on both sides is the byte-identical state". Measured, the names always differ once any target RUSTFLAGS is set, so the only identical state left is "no RUSTFLAGS at all". This goes away with D1's fix.

## Verified clean

- AC1 and AC2: the flag reaches every wasm32 rustc invocation of the SIMD variant (45 of 45, `openjph_core` and `ocelli_codec` included), no host invocation (0 of 15), and no plain invocation (0 of 45). Measured with `-v` into fresh dirs, matching GLM's counts.
- AC4 contradiction: GLM's explanation reproduced. The OUT_DIR path is in thiserror's rmeta, and `openjph_core` and `ocelli_codec` differ only in `lib.rmeta` under the old command. GLM called the identical member "`.rcgu.o` object code". It is LLVM bitcode, which is the observation D1 and D3 build on.
- AC7: the variable is a command prefix on two lines and is not exported. No `.cargo/` directory. Shipped and decode modules have the same hash and size before and after `bin/ocelli.sh native`. The size check passes.
- The script's own logic: identical, missing-side and missing-dir refusals and the mtime choice each go red under a scratch-copy mutation. The unit suite is wired into `gate native` with `&&`.
- `bin/ocelli.sh` arm: 13 echo lines numbered consistently. Steps 9 and 13 print `executed` for all four modules on this host.
- `docs/lld/build-targets.md` table matches the arm's 13 steps. `docs/lld/codecs.md` "steps 10 to 13" matches.
- README, `use.html`, `more.html`: `grep -i simd` finds nothing, and nothing in the diff makes them stale.
- fmt, clippy and the gates and scripts in the evidence table.

## Limitations and handoff

- CI `native` job not observed (no push). No GPU, oracle or browser job, as instructed.
- I counted the SIMD instructions in each module but did not measure whether any of them executes at run time. The argument in D3 rests on old and new modules carrying the same inventory, which does not depend on that.
- The `SOURCE-POLICY.md` counts 138 and 137 were checked for consistency with the table above them, and were not re-derived construct by construct.
- F-232 (`0d812e0b`) is not reviewed here.
- Next owner: implementation remediates D1 to D3. Then a fresh pass. D1's replacement needs a red run against mutation 2 and the wrong-flag build, and D2's against the three-build stale sequence, both reproduced through the real arm.
