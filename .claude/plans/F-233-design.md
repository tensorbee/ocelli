# F-233, SIMD wasm builds compile SIMD code

**Status**: approved
**Epic ref**: E38.36
**Sprint**: S46
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "SIMD wasm builds compile SIMD code: the SIMD codec verification variants pass +simd128 to every crate through a target-scoped RUSTFLAGS, a native arm step refuses a SIMD variant whose dependency rlibs match the plain build, and D-22's F-X013 claim is rechecked on that build". Layer Build, depends on F-229 and F-232, milestone M20.

`allocation.json` notes: "Found by F-229's eighth review, defect D2. Answered by the operator on 2026-11-06: fixed in S46, every crate built with +simd128 for the SIMD variants and F-X013's claim rechecked."

## Normative source, transcribed

### HLD 15.2, `docs/hld/12-workspace-and-build.md` line 93

"**DICOM-RS FEATURES** Disable default features. The defaults are rayon and simd, and the gdcm feature is native-only. On wasm you want: default-features = false, then jpeg, rle, deflate and openjp2 selected explicitly."

The HLD says nothing else about wasm SIMD. The normative sources for this story are the deviation row and the story whose claim it rechecks.

### D-22, `docs/hld/DEVIATIONS.md` line 39, the evidence clause

"F-X013 then priced the one candidate that uses the same Rust implementation on browser, desktop and server, and measured `.201` and `.202` exact against an encoder-independent synthetic ramp on native, plain wasm and `+simd128`, with `.203` identical across all three builds and differing from OpenJPH 0.31.0 at 41 of 6,144 samples by one, which is a D14 measured divergence rather than a bit-exactness claim."

### The standing check, `bin/ocelli.sh` native arm

Lines 615 to 617: "9 to 11 are the standing version of F-X013's central claim, that openjph-core produces identical samples on native, plain wasm and +simd128 wasm. The spike that measured it once is deleted. D-22."

Steps 7 and 10, lines 603 to 608 and 621 to 626:

```bash
cargo build -p ocelli-codec --release --example verify_jpeg2000 \
  --target wasm32-unknown-unknown --target-dir target/codec-wasm-plain
cargo rustc -p ocelli-codec --release --example verify_jpeg2000 \
  --target wasm32-unknown-unknown --target-dir target/codec-wasm-simd -- \
  -C target-feature=+simd128
```

Step 10 is the same pair with `--example verify_htj2k`. Steps 8 and 11 run both modules under `node scripts/run_codec_wasm.mjs`, which calls each module's `main` and prints `executed <path>`. `crates/ocelli-codec/examples/verify_htj2k.rs` aborts when the `.201` or `.202` decode differs from `jpegls_corpus_reference_u16le.raw` or the `.203` byte sum differs from `LOSSY_CHECKSUM: u64 = 1_559_962`.

### The finding, `../ocelli-wt-f-229/.claude/reviews/F-229-working-pass-8.md`

D2, line 62: "steps 7 and 10 (`bin/ocelli.sh:603-607`, `621-625`) pass `-C target-feature=+simd128` after `cargo rustc --`. Cargo hands those arguments to the final crate only, the example, and compiles every dependency without them. `jxl-grid`, `jxl-modular` and `jxl-render` are therefore compiled with `target_feature = "simd128"` false, and their 50 regions are absent from the "+simd128" build exactly as from the plain one."

Line 41: "`shasum` of `libjxl_{grid,render,modular}-*.rlib` in `target/codec-wasm-plain` vs `target/codec-wasm-simd` ... byte-identical, with the same metadata hash in both directories".

Line 65: "D-22's F-X013 "+simd128" leg was built the same way. openjph-core has no `simd128` cfg, so no gated code is missed, but its own codegen in that build did not get the feature either."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e`.

1. `simd128` appears in `bin/ocelli.sh` at lines 608 and 626, both in the `native)` arm, and nowhere else in the runner. There is no `.cargo/` directory. No `wasm-pack` invocation passes a SIMD flag, so the shipped module and the decode module are plain wasm and stay out of scope.
2. `grep -rn 'simd128\|wasm32' vendor/openjph-core-0.1.0/src` finds nothing. Its SIMD files are `coding/simd/{neon,x86}.rs` and `transform/simd/{neon,neon_colour,x86,x86_colour}.rs`, all native.
3. jxl-render 0.12.4 and jxl-grid 0.6.2 in the local registry carry `simd128` gated code, for example `jxl-render-0.12.4/src/filter/impls/wasm32.rs` and `jxl-grid-0.6.2/src/simd.rs`. They reach the tree only through F-229, which is in flight on `work/f-229-xai`.
4. `crates/ocelli-render/src/caps.rs:497` reads `cfg!(target_feature = "simd128")` inside `SimdSupport::build_target()`. It is the only `target_feature` read in Ocelli source.
5. `docs/lld/build-targets.md` says "Eight steps" and its table lists eight, while the arm prints `1/11` to `11/11`.
6. The CI `native` job runs `bin/ocelli.sh gate native` on `ubuntu-latest` (`.github/workflows/ci.yml:74`).

## What the specification does not cover

Every item is a lead decision under the operator's delegation of 2026-11-06.

1. **The flag reaches every crate through `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+simd128"` on the SIMD build commands only**, and those commands become `cargo build`, not `cargo rustc`. With an explicit `--target`, cargo applies a target's rustflags to the target's crates and not to build scripts or proc macros, which run on the host. The SIMD variants keep their own `--target-dir target/codec-wasm-simd`, so the flag change never invalidates the plain or default caches.
2. **No cargo config file.** A `.cargo/config.toml` entry would reach every wasm32 build, including `wasm-pack` for the shipped module, and `lint_policy_check.py` reads every cargo config's rustflags. A variable on two commands is scoped to exactly what the operator asked for.
3. **The proof is an rlib comparison run as a native arm step.** A new `scripts/simd_variant_check.py` takes the plain and SIMD target directories and a list of crate names, and refuses when any listed crate's release rlib is byte-identical between the two, or is missing from either. The list is `ocelli_codec` and `openjph_core`, plus `jxl_grid`, `jxl_render` and `jxl_modular` once F-229 has landed. It runs after steps 7 and 10. Its unit test plants identical and differing rlibs.
4. **The recheck is the existing execution.** Steps 8 and 11 already abort on a wrong decode. Built correctly, they now exercise SIMD codegen of the decoders. If either aborts, that is a finding against D-22, and the story stops and reports rather than relaxing the check.
5. **D-22 gets one sentence**, saying that F-X013's `+simd128` leg was compiled with the flag on the example crate only, that F-233 rebuilt it with the flag on every crate, and the result. The row's other clauses are untouched.

## Problem and reproducer

```bash
cargo rustc -p ocelli-codec --release --example verify_htj2k \
  --target wasm32-unknown-unknown --target-dir "$SCRATCH/t" -v -- -C target-feature=+simd128 2>&1 \
  | grep -c 'target-feature=+simd128'
```

Pass 8's line 40 records the result of this command: `simd128` on the `verify_htj2k` invocation only. Expected after this story: every wasm32 rustc invocation of the SIMD variant carries the flag.

## Scope, API and compatibility

In scope: steps 7 and 10 of the `native)` arm and a new step after each, `scripts/simd_variant_check.py` and its test, the D-22 sentence, `docs/lld/build-targets.md`, and the `docs/SOURCE-POLICY.md` sentence F-229 adds about which builds compile the `simd128` regions.

Non-goals: a SIMD build of the shipped module. D-07's A7.1b note in `docs/hld/DEVIATIONS.md` says "wasm SIMD128 becomes a requirement rather than a detection detail", and that belongs to the tier C stories, not to this verification build. Native SIMD. The vendored counts.

No public API change.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `gate native`, CI `native` job | `native)` arm steps 7 and 10 | release, wasm32, separate target dirs | plain and SIMD `.wasm` per example |
| new steps 7b and 10b | `scripts/simd_variant_check.py` | the two target dirs, the crate list | OK naming each crate, or a refusal naming the identical rlib |
| steps 8 and 11 | `run_codec_wasm.mjs` | both modules | `executed`, or a trap from `abort` |
| D-22 reader | `docs/hld/DEVIATIONS.md` | evidence | the recheck sentence |

## State transitions and invariants

Not applicable. Builds and checks are functions of the tree and the toolchain.

## Ownership, commit and recovery

Not applicable. No runtime resource.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Every crate in the SIMD variant is compiled with `+simd128`. `cargo build -v` of the SIMD `verify_htj2k` variant into a fresh scratch target dir shows the flag on every wasm32 rustc invocation, `openjph_core` and `ocelli_codec` included, and on no host invocation | recorded in the handoff with the counts | `cargo rustc --` kept, which reaches the example only | Implementation, recorded |
| AC2 | The plain variant is unchanged. The same `-v` build of the plain variant shows the flag on no invocation | recorded | A config that leaks the flag into the plain build | Implementation, recorded |
| AC3 | `simd_variant_check.py` refuses when a listed crate's rlib is byte-identical in the two dirs, naming it, and when one is missing. It passes when all differ | `scripts/tests/test_simd_variant_check.py` and probe `simd-variant.identical-rlib` with a control | A check that compares the final `.wasm` only, which the example's own SIMD codegen always changes | guards |
| AC4 | Old-source RED. On the unchanged steps the new check refuses, naming `openjph_core` | recorded in the handoff | A check never run against today's build | Implementation, recorded |
| AC5 | After F-229 lands, the list includes `jxl_grid`, `jxl_render` and `jxl_modular`, and all three differ | `gate native` | A list fixed at today's closure | Implementation |
| AC6 | The recheck. Steps 8 and 11 print `executed` for both modules of both examples on the corrected build, on this host and in the CI `native` job | `gate native`, CI | Reporting the recheck from the plain module | Implementation and CI |
| AC7 | No other build sees the flag. `bin/ocelli.sh wasm` and `bin/ocelli.sh wasm-decode` produce modules whose size check in `gate wasm` passes against the unchanged budgets | `gate wasm` | A cargo config or exported variable that reaches `wasm-pack` | Implementation |
| AC8 | D-22 carries the recheck sentence with the result, and `python3 scripts/deviation_check.py` passes | deviation check | Rewording D-22's measured clauses | Implementation |
| AC9 | `docs/lld/build-targets.md` lists the arm's real steps, the two new ones included, and stops saying eight | review | Leaving fact 5's stale count | Implementation |
| AC10 | `lint_policy_check.py` and `gate guards` pass with the variable in the runner | `gate guards` | A rustflags spelling the lint policy refuses | Implementation |

## Approach

1. Start after F-229 and F-232 are integrated.
2. Write `simd_variant_check.py` and its test, and record AC4 red against the unchanged steps.
3. Change steps 7 and 10 to `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-C target-feature=+simd128" cargo build ...`, add the two check steps, renumber the arm's echo lines.
4. Run AC1, AC2, AC6 and AC7, then the D-22 sentence and the LLD.

Rejected alternatives:

- Plain `RUSTFLAGS`. With `--target` cargo scopes it the same way, but the name carries no target and reads as a general setting to the next editor of the runner.
- A `#[cfg(not(target_feature = "simd128"))] compile_error!` in the example. The example already gets the flag today, so it proves nothing about dependencies.
- Counting SIMD opcodes in the `.wasm`. It needs a wasm decoder in the repository, and the example's own SIMD codegen would satisfy it.

## Dependencies, write ownership and resources

Declared: F-229, which writes the `native)` arm's neighbourhood, `docs/hld/DEVIATIONS.md` (D-28) and the `docs/SOURCE-POLICY.md` sentence this story corrects, and F-232, which writes the catalogue, the probe budget and the runbook first. **Order: F-229, then F-232, then F-233.**

Write set:

```text
bin/ocelli.sh                                    native) arm, steps 7 and 10 and two new steps
scripts/simd_variant_check.py, scripts/tests/test_simd_variant_check.py   new
scripts/guards/catalogue.py, ci/guard-probe-budget.json, docs/runbooks/guard-verification.md
docs/hld/DEVIATIONS.md                           D-22, one sentence
docs/SOURCE-POLICY.md, docs/lld/build-targets.md
```

`bin/ocelli.sh` is written by most S46 stories, in the gate list and arms other than `native)`. No GPU, browser or corpus. Node runs the modules as today.

## Verification and evidence reuse

```bash
bin/ocelli.sh gate native wasm guards
python3 scripts/deviation_check.py
```

Final profile: `bin/ocelli.sh gate --sprint`. Evidence is invalidated by a change to steps 7 to 11, the crate list, the toolchain or the codec closure.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a, codec build verification
- Tier B (WebGL2): n/a, codec build verification
- Tier C (CPU): n/a, codec build verification. The decoders themselves are CPU code on every tier and are exercised by steps 8 and 11

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC3, identical and missing rlibs refused | `scripts/tests/test_simd_variant_check.py` |
| guard probe | AC3 in the floor sandbox | `scripts/guards/catalogue.py` |
| integration | AC5 to AC7, AC10 through the real arm | `gate native`, `gate wasm`, `gate guards`, CI |
| recorded | AC1, AC2, AC4 | handoff |

No `fixture` row is owed. The decoders' sample arithmetic is checked by the existing examples against their recorded truth.

## Parity surface covered

None.

## Deviations

D-22, a sentence recording the recheck. No new row.

## LLD impact

`docs/lld/build-targets.md`, the native arm's step table and the sentence on steps 6 to 8.

## User-facing documentation impact

No change to `README.md`, `examples/folder-viewer/src/use.html` or `examples/folder-viewer/src/more.html`. None of them mentions SIMD, measured by `grep -n -i simd` over the three files.

## Open questions

None open.

- Q1, how the flag reaches every crate. Answered: lead decision under the operator's delegation of 2026-11-06. A target-scoped rustflags variable on the SIMD commands, decisions 1 and 2.
- Q2, the proof. Answered: lead decision under the operator's delegation of 2026-11-06. An rlib difference per named crate as a gate step, decision 3.
- Q3, what happens if the recheck fails. Answered: lead decision under the operator's delegation of 2026-11-06. The story stops and reports against D-22, decision 4.
