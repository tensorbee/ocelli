# F-233 review, pass 2

**Reviewed**: `git diff 56caf03c..d57539ba` on `work/f-233-xai`, HEAD `d57539ba` ("F-233, structural pin on the SIMD build shape"), tree `f3e307431863f44a38ca4608c38b43d68555ce52`, worktree clean before the pass. Two commits, `90f9557c` (the pass 1 repair, the check reads the linked modules) and `d57539ba` (the structural pin). Every pass 1 finding was replayed against this tree.
**Author / reviewer**: repair by a Claude implementer after the lead rulings in `.claude/plans/F-233-design.md` ("Lead rulings after review pass 1" and "Lead ruling after the pass 1 repair"). Reviewer is a Claude subagent run by the sprint lead as `/microscope F-233 --working`, pass 2. The reviewer wrote none of the reviewed material and none of the rulings. The rulings themselves are the lead's and are not independently approved here, only the implementation against them and every measured claim the range adds.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The decoder against the WebAssembly Core Specification**, with an independent implementation as the oracle rather than the script's own table. Every 0xFD sub-opcode 0 to 255 and the relaxed range 0x100 to 0x113 was hand-assembled into a module, with immediates written from the specification text and not from the script's sets, and decoded by both the script and binaryen `wasm-dis --all-features` version 125. Every assigned name agrees. The 20 reserved slots are refused by both. The six relaxed entries where the names differ are binaryen's own spelling (`i8x16.laneselect` for the specification's `i8x16.relaxed_laneselect`, `i16x8.dot_i8x16_i7x16_s` for `i16x8.relaxed_dot_i8x16_i7x16_s`), and the script uses the specification's.
- **Byte patterns a naive scan misreads**, each decoded by the script and by binaryen: 0xFD inside `i64.const`, `f32.const` and `f64.const` immediates, a memarg offset of 253 (`0xFD 0x01`), `local.get 253`, a block type index of 125 (`0xFD 0x00`), `br_table`, typed `select` over `v128`, `memory.fill` after an `i32.const 125`, and a data section of ten `0xFD 0x0C` pairs. No SIMD instruction is counted in any. A relaxed `i8x16.relaxed_swizzle` (`0xFD 0x80 0x02`), a five-byte non-canonical `v128.const` sub-opcode, a `v128.load` with offset 253, an `i8x16.shuffle` whose lane bytes are all `0xFD 0x0C` and a `v128.load8_lane` whose lane byte is `0x0B` (`end`) each count exactly the instructions present. binaryen agrees on all but the shuffle, which it rejects as an illegal lane index, a validation rule a counter need not apply.
- **Pass 1 D1 and D2**, replayed through `bin/ocelli.sh native` in a scratch copy with its own target dirs. Each is now refused at step 8.
- **The case the lead ruled cannot be refused**, the flag on the example only. Replayed, it passes as the LLD says, and its modules are byte-identical to the pre-F-233 `cargo rustc --` modules.
- **The structural pin** over the pre-F-233 runner and over the tracked one, and under mutation.
- **D-22, the LLD's measured section and `SOURCE-POLICY.md`**. The per-module counts, the attribution and the executed counts were re-measured in full for both modules with a trap-insertion rewrite of my own, not sampled.
- **Shipped modules**: `pkg` and `pkg-decode` are unchanged against pass 1's recorded hashes.

## Evidence

Scratch work under `<wt>/target/review-p2/`. The scratch tree is `git archive d57539ba`. Reviewer gates ran with `CARGO_TARGET_DIR=<wt>/target`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `python3 -B -m unittest discover -s scripts/tests -p test_simd_variant_check.py` | HEAD | now | 0, 25 tests |
| Sub-opcode cross-check, 276 hand-assembled modules, script against `wasm-dis` 125 | own assembler | now | 0 name mismatches on assigned codes. Reserved 154, 162, 165, 166, 175, 176, 178 to 180, 187, 194, 197, 198, 207, 208, 210 to 212, 226, 238 refused by both |
| Naive-scan trap modules, 16 cases listed above | own assembler | now | the script's count equals binaryen's in every case binaryen accepts |
| `bin/ocelli.sh native`, fresh scratch copy, fresh dirs | `d57539ba` | now | 0. Step 8 `156 SIMD instructions ... v128.load x67, v128.store x62, v128.const x9, i8x16.eq x8, v128.load64_lane x1, v128.load64_zero x1, i8x16.all_true x8`, step 12 `43 ... v128.load x18, v128.store x23, v128.const x2`, steps 9 and 13 four `executed` lines |
| `wasm-dis --all-features`, per-opcode count of the four fresh modules | fresh dirs | now | plain modules 0 each. SIMD modules equal to the script's counts, opcode by opcode |
| **Pass 1 mutation 2 and beyond**: `RUSTC_WRAPPER` stripping `+simd128` from every wasm32 crate, wrapper log names 47 crates including `verify_jpeg2000` | scratch arm | now | arm 1 at step 8, "holds no SIMD instruction the plain module lacks". SIMD `verify_jpeg2000.wasm` is `aaa7df84...`, byte-identical to plain |
| **The wrong flag**: runner copy with `+bulk-memory` in place of `+simd128` | scratch arm | now | arm 1 at step 8. Module `a4eb3491...`, not plain's bytes, and no SIMD instruction |
| **The flag on the example only**: the same wrapper sparing `verify_*` | scratch arm | now | arm 0, both OK lines as the fresh run. HTJ2K SIMD module `2f9c0c7c...`, the hash pass 1 recorded for the pre-F-233 module. JPEG 2000 SIMD module `cbf9a6ca...`, equal to a pre-F-233 `cargo rustc -- -C target-feature=+simd128` build I made into a separate dir, whose inventory is the same 156 |
| **Pass 1 D2, reused SIMD dir**: runner copy without the variable, then the real arm, then the copy again | scratch arm | now | 1, 0, 1. The real arm recompiled 0 units, the third run 0 units, and its step 8 refuses with the SIMD module at plain's hash `aaa7df84...` |
| The same three builds for HTJ2K with the arm's commands | scratch tree | now | units compiled 0, 44, 0. Module hash plain, `a7514168...`, plain. Check 1 |
| `build_shape_problems` over `git show 0d812e0b:bin/ocelli.sh` and the tracked runner | both | now | 4 refusals (two prefix, two `cargo rustc --`) and 0. 28 arm commands parsed, two SIMD build steps selected |
| 26 scratch-copy mutations of `simd_variant_check.py`, unit suite each | copies | now | 14 red. The 12 survivors were then run against the real modules, see the nitpick |
| Closure grep: `cargo tree -p ocelli-codec --target wasm32-unknown-unknown -e normal` and `-e normal,dev`, `grep -rln simd128` over each package's `src` | scratch tree | now | 42 and 49 packages. Exactly glam 0.30.10, jpeg-decoder 0.3.2, jxl-grid 0.6.2, jxl-modular 0.11.3, jxl-render 0.12.4, memchr 2.8.3 in both. openjph-core absent. The seven dev-only packages add none |
| `cargo rustc -p <crate> --lib ... -- --print cfg`, glam and jxl-modular, with and without the variable | scratch dirs | now | `target_feature="simd128"` 1 and 0 for each |
| glam dep-info, arm's plain and SIMD dirs | scratch arm | now | 14 `wasm32` sources SIMD only, 13 `scalar` sources plain only, as the LLD says |
| rlib member `/0` (magic `42 43 c0 de`) byte counts, plain and SIMD | scratch arm | now | `inverse_h_i16_wasm32_simd128` 0 / 104, `epf_row_wasm32_simd128` 0 / 57, `4wasm24color_convert_line_ycbcr` 0 / 5, `4wasm29dequantize_and_idct_block_8x8` 0 / 4 |
| `CARGO_PROFILE_RELEASE_STRIP=false` SIMD build, per-function count from `wasm-dis` | scratch dir | now | inventory equals the stripped build's. HTJ2K: 30 openjph-core, 5 `Htj2kDecoder::decode`, 6 in core and std functions instantiated in std, 2 dlmalloc. JPEG 2000: none in glam, jpeg-decoder, memchr or a jxl crate. No glam function in either module and one jxl function in HTJ2K, holding no SIMD |
| Own trap-insertion rewrite: `unreachable` after one SIMD instruction at a time, body and section sizes re-encoded, run `main` under Node | unstripped and stripped SIMD modules | now | identity rewrite byte-identical and runs. HTJ2K 25 executed, `v128.store` 14, `v128.load` 9, `v128.const` 2, 8 `Tile::read_tile_data`, 4 `Codestream::read_headers`, 4 `Subband::new`, 2 `rev53_atk`, 2 `irv97_inverse_1d`, 5 `Htj2kDecoder::decode`. Stripped module also 25. JPEG 2000 14 executed, `v128.load` 7, `v128.store` 7, 6 in `decode_j2k_fragment` and 8 in core and alloc generics over ritk-codecs types. No `i8x16.eq`, `i8x16.all_true` or lane load executes |
| `crates/ocelli-codec/examples/verify_htj2k.rs` read | HEAD | now | `.201` and `.202` exact, `.203` against `LOSSY_CHECKSUM`, as D-22 says |
| `bin/ocelli.sh wasm`, `wasm-decode`, `pin_and_size_check.py --with-size` | HEAD | now | 0, 0, 0. `pkg` `008aac80ef24f8b0`, 2,179,571 bytes, `pkg-decode` `07cf6db9ba05bd09`, 16,455 bytes, before and after, equal to pass 1's record |
| `bin/ocelli.sh gate native` in the worktree, whose `target/codec-wasm-simd` holds the stale pre-F-233 pair | HEAD | now | 0, both OK lines, 25 tests |
| `bin/ocelli.sh gate guards`, `deviation_check.py`, `guard_census.py`, `prose_check.py`, `source_provenance_check.py` | HEAD | now | all 0. `gate guards`: 439 refusal probes red for their declared reason, 103 controls green. Both `simd-variant.no-simd-instruction` and `simd-variant.cargo-rustc-shape` red with green controls. 28 deviations, prose clean over 891 files, provenance clean over 1753 files |

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the unit suite does not pin several of the decoder's immediate rules

Twelve of the 26 mutations leave `test_simd_variant_check.py` green. Against the real F-233 modules they split three ways.

- Red at the arm through the real modules, so `gate native` still catches them: `v128.load32_zero` and `v128.load64_zero` read without a memarg (the JPEG 2000 module holds one `v128.load64_zero`), `br_table` without its default label, `f32.const` as three bytes, local declarations ignored, a five-byte LEB limit, and a memarg offset read as one byte.
- Equivalent on every input rustc emits today: the sub-opcode read as one byte (for a canonical encoding of 128 to 255 the first byte is the value and the `0x01` after it decodes as `nop`), `memory.copy` with one immediate (the second `0x00` decodes as `unreachable`), and `v128` dropped from the block value types (a one-byte LEB read gives the same position).
- Unreached by both modules: the relaxed table moved away, the multi-memory memarg flag, typed `select`, and `if` (neither module holds a `0x04`).

None of these turns a module into a wrong count without a refusal on the modules measured, and a wrong rule on a future toolchain more likely refuses than miscounts, so this does not block. The cheapest additions are a relaxed instruction (`0xFD 0x80 0x02`, red under the one-byte read and under the moved table) and a `v128.load64_zero` with a memarg, so the unit suite catches what only the arm catches today.

### N2, "the only lane-wise operations" leaves out the lane load

`docs/lld/build-targets.md`, the sentence after the module table: "The only lane-wise operations are `i8x16.eq` and `i8x16.all_true` in the JPEG 2000 module, and neither executes." The same table lists `v128.load64_lane`, which writes one lane, and `i8x16.all_true` is a reduction rather than a lane-wise operation. The conclusion holds, because only `v128.load` and `v128.store` execute in that module, measured above. Something like "every instruction other than a `v128` load, store or constant is in the JPEG 2000 module, and none of them executes" says the same with nothing to argue over.

## Verified clean

- Pass 1 D1 is closed. The rlib comparison is gone and the module check refuses both of pass 1's false passes through the real arm, the stripped build and the `+bulk-memory` build.
- Pass 1 D2 is closed. The newest-rlib rule is gone. The three-build stale sequence is refused through the real arm for JPEG 2000 and with the arm's commands for HTJ2K, and the LLD's sentence that cargo relinks the fresh unit's module into `examples/` holds (0 units recompiled, module at plain's hash).
- Pass 1 D3 is closed. D-22 now says what was measured, and every number in its new sentences was re-measured above: 43, the opcode mix, 30 in openjph-core, 25 executed with 20 and 5, six closure crates with openjph-core not among them, and the `.201`, `.202` and `.203` checks. The catalogue note now names `gate native` as the runner of the unit suite, which `bin/ocelli.sh:216-217` confirms. The test docstring's step numbers are 7 and 11. The step 5 comment now says step 6 runs natively.
- Pass 1's nitpick on digest names is gone with the code it described.
- The lead's ruling after the repair is implemented as written. `build_shape_problems` refuses the pre-F-233 runner four times and passes the tracked one. Removing the prefix rule, the `cargo rustc --` rule or the continuation join, or renaming the target dir, each turns the suite red. Its docstring and catalogue note call it a spelling pin, which is what it is: it reads only lines naming `--target-dir target/codec-wasm-simd` and does not see a flag removed some other way, for example a `--config` override or an exported `RUSTFLAGS`, which takes precedence over the target variable. Each of those removes the flag from the example too, so step 8 refuses it, as the wrong-flag row shows for the same module state.
- The decoder: the walker's structure (`end` before the no-immediate set, `else` not changing depth, body size and code section tail checked, unknown core, 0xFC and 0xFD codes refused), the SIMD immediate sets (memarg 0 to 11, 92 and 93, memarg plus lane 84 to 91, lane 21 to 34, sixteen bytes for 12 and 13) and the 0xFC immediate counts, against section 5.4 and against binaryen. Only function bodies are counted, which the docstring says.
- The six crates' cfg facts: the closure grep, `--print cfg` for two crates, and the dep-info and bitcode rows for glam, jpeg-decoder, jxl-modular and jxl-render. jxl-grid's rmeta row and memchr's dep-info row were not re-measured.
- `SOURCE-POLICY.md`'s new sentence: no function of the three jxl crates holds a SIMD instruction in either module, measured. The only jxl function linked anywhere is one in the HTJ2K module with none.
- AC7: shipped `pkg` and `pkg-decode` unchanged, size check 0.
- Probe budget `"simd-variant": 19` under `entry_sites`: `guard_census.py` exits 0 against it.
- README, `use.html`, `more.html`: the range touches none of them and none mentions SIMD.

## Limitations and handoff

- The range changes no Rust, so `cargo fmt` and `clippy` were not rerun. Pass 1 ran both on the parent tree.
- CI's `native` job not observed (no push). No GPU, oracle or browser job, as instructed.
- The trap-insertion rewrite locates instructions with the script's own walker. Its independence rests on binaryen agreeing on every count and on Node accepting all 199 rewritten modules, which a mispositioned insert would usually fail.
- jxl-grid's `lib.rmeta` row, memchr's dep-info row and `--print cfg` for four of the six crates were not re-measured.
- Wrapper runs stop at step 8 by `set -e`, so step 12 was reached through the arm only in the fresh and example-only runs. HTJ2K's refusals were replayed with the arm's commands.
- Scratch under `<wt>/target/review-p2/` is deleted at the end of the pass with `shutil`.
- Next owner: the lead. This pass is clean on defects and smells. The two nitpicks are optional.
