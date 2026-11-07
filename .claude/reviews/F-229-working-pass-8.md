# F-229 review, pass 8

**Reviewed**: `git diff f85ef70 72824ed` on `work/f-229-xai`, HEAD `72824eddea79d05222b72234dc6ee3eae66971e0` ("F-229, the audit recounted with the scanner fixed"). Three files: `docs/SOURCE-POLICY.md`, `scripts/unsafe_allowlist_check.py`, `scripts/tests/test_unsafe_allowlist_check.py`. Scope is AC25 and the pass 7 lead rulings. Untracked paths at start and end: `.claude/reviews/F-229-working-pass-1.md` to `-7.md` and `tools/oracle/out`. This pass added only this file.
**Author / reviewer**: GLM-5.3 in opencode authored 72824ed. Claude (Opus) reviewed independently and authored no F-229 material. GLM's log `glm-F-229-fix7b.log` was read and not relied on. Every number below was re-derived.
**Result**: 3 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The scanner fix against its ruling.** The ruling asks that a Rust string continuation no longer ends a string, so the code after it is scanned, that a test pins it, and that no tracked file's count moves. The fix is `re.S` on `STRING`. Since `.` appears only in the `\\.` alternative, the flag changes behaviour for a backslash before a newline and nothing else. The test is green on the new pattern and red on the old one. No tracked count moves.
- **Adjacent triggers of the same mechanism**, replayed with old and new patterns on synthetic inputs, each with a planted `unsafe` between two lines of code. The continuation case went from hidden to found, and so did a continuation inside a byte string. Escaped quotes, a trailing escaped backslash, plain byte strings, lifetimes, block and line comments holding `unsafe`, a URL in a string and a CRLF continuation were all correct before and still are. **Hidden under both patterns**: a `'"'` char literal, a `'\"'` char literal, a raw string `r"a\"`, a raw string `r#"a " b"#`, and a line comment with an unpaired `"`. A nested block comment over-counts, which is the safe direction. This is D1, and the shape this rejects is "the guard is green, so no tracked file holds an unsafe".
- **Closure recount.** I derived the closure myself: `git archive 4165d1e8` into scratch, then `cargo tree -p ocelli-codec -e normal --prefix none --offline --locked` at that base and at HEAD, then `comm -13`. That gives 20 packages. With the fixed `strip_noise` and `UNSAFE`, imported from the worktree module, they hold 207 files, 70 with a match, 534 in total. All 70 rows match the table exactly. My own lexer, which handles nested comments, raw and byte strings, char literals and lifetimes, also gives 534 with no per-file difference. Summing the table's reachability column gives 138 yes, 50 `+simd128`-only and 346 compiled out.
- **Named files from pass 7, re-read at the cfg lines.** `tracing-core` `lib.rs:272-273` gives `#[cfg(not(feature = "std"))] pub(crate) mod spin`, so the 7 in `once.rs` and the 4 in `mutex.rs` are out. `lazy.rs:7` gives `#[cfg(not(feature = "std"))] mod spin`, which encloses line 37. `callsite.rs` 239 and 469 sit in ungated items, and the file's only cfgs are at 515 and 577. `metadata.rs:720` is `#[cfg(not(debug_assertions))]` over the `_ => unsafe {` at 721, and the `#[cfg(test)]` at 1055 encloses 1112. In once_cell, `imp_std.rs` is selected by `lib.rs:388-390`, with 8 matches before `#[cfg(test)] mod tests` at 290-291 and 2 after it. The `src/bin` files: `alloc-no-stdlib` and `alloc-stdlib` set `autobins = false` with `[[bin]]`, and brotli's `[[bin]]` is at `Cargo.toml:57-59`. A dependency's bins are never built. The features `tracing-core` `std` and `once_cell`, and `once_cell` `std`, `alloc`, `race`, come from `cargo tree -e features,normal`, identical on wasm32 and aarch64-apple-darwin.
- **Ten fresh reachability calls**, decided from the cfg lines:
  1. once_cell `race.rs`: 8 ungated plus 6 under `#[cfg(feature = "alloc")] mod once_box` at 353-354, under `#[cfg(feature = "race")] pub mod race` at `lib.rs:1421`, so 14 yes.
  2. once_cell's `#[path]` selection, which rules out `imp_pl.rs` and `imp_cs.rs`.
  3. `jxl-render` `filter/impls.rs:3-9`, which leaves `generic` ungated and gates `wasm32` behind `all(target_family = "wasm", target_feature = "simd128")`.
  4. `jxl-render` `vardct/mod.rs:25-40`, which has the same shape.
  5. `jxl-oxide` `fb.rs`: 164 and 181 come before `#[cfg(test)]` at 187, and `mod fb` is ungated.
  6. `jxl-modular` `rct.rs`: 13, 96, 100 and 131 ungated, 26, 60, 142 and 189 `x86_64`, 39, 73, 149 and 196 `aarch64`, so 4/4/4.
  7. `jxl-modular` `squeeze.rs`, line by line: 10 wasm (50, 613 to 728, 794, 1047), 24 `x86_64` and 6 `aarch64`.
  8. `jxl-grid` `simd.rs`, split at 106, 350 and 446, giving 15/35/15/15.
  9. `alloc-stdlib` `heap_alloc.rs:131` `#[cfg(feature="unsafe")]`.
  10. brotli `lib.rs:12` `forbid(unsafe_code)` unless `unsafe` or `ffi-api`, `lib.rs:50` `#[cfg(feature="ffi-api")] pub mod ffi`, and `decode.rs:2128` `#[cfg(feature="unsafe")]`.

  Two extras: `jxl-color` `powf.rs`, whose 12 matches each sit after an `x86_64` or `aarch64` cfg, and `jxl-render`'s `#![allow(unsafe_op_in_unsafe_fn)]`, which the whole-word rule correctly does not count. All of these agree with the table.
- **Scope and shipped modules.** `git merge-base HEAD sprint/s46` prints `4165d1e8b4b63b3d87d271cfc2a6c16c10497744`. `cargo tree -p ocelli-wasm -i ocelli-codec --target all`, the same for `-i jxl-oxide`, and both again for `ocelli-native`, each with and without `--all-features` under `-e all`, all answer "did not match any packages". `cargo tree -i ocelli-codec --target all -e normal` names `ocelli-folder-viewer` and `ocelli-oracle`. Those claims hold. The sentence that names the closure's wasm32 compilations does not hold, which is D2 and D3.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| old vs new `STRING` over all 375 tracked `.rs`, the new one imported from the worktree module | HEAD 72824ed | now | 0 count changes, total 104 both, openjph-core 104 both. Stripped text differs in 27 files, none with a count change |
| independent lexer vs fixed scanner, 375 tracked `.rs` | HEAD | now | counts agree (104). The scanner blanks real code in 9 files: `ocelli-dicom/src/dicomweb.rs` 412-425, `ocelli-render/src/caps.rs` 2486-2487, `ocelli-wasm/src/canvas.rs` 558, `example-server` `reply.rs` 138-226 and `series.rs` 208-325, `folder-viewer/src/preview.rs` 1334-1371, and three test or example files |
| plant `let _p = unsafe { core::ptr::read(&0u8) };` at `dicomweb.rs:418` in a `git archive HEAD` copy, `--staged` | scratch copy | now | exit 0, "OK: no unsafe outside the allow-list". The control plant in `ocelli-pixel/src/lib.rs` gives FAIL naming line 50 |
| synthetic edge cases, old vs new | `edge.py` | now | listed under Acceptance challenged |
| `python3 -B -m unittest discover -s scripts/tests -p test_unsafe_allowlist_check.py` | HEAD | now | exit 0, 6 tests OK. `pytest` is not installed, and the repo's runner is unittest |
| the same test file in a scratch copy with `re.S` removed | scratch copy | now | FAILED, `test_string_continuation_does_not_blank_following_code` |
| `python3 scripts/unsafe_allowlist_check.py` and `--staged` | HEAD | now | exit 0, 263 files checked, 2 permitted, openjph 104, ritk 0 |
| closure count, fixed scanner and own lexer | registry `index.crates.io-1949cf8c6b5b557f`, 20 packages from base `4165d1e8` vs HEAD | now | 207 files, 70, 534 both. 70 of 70 rows match. Column sums 138/50/346 |
| `cargo rustc -p ocelli-codec --release --example verify_htj2k --target wasm32-unknown-unknown --target-dir <scratch> -v -- -C target-feature=+simd128` | HEAD | now | exit 0. `simd128` appears in the rustc command line of `verify_htj2k` only, not in those of `jxl_grid`, `jxl_modular`, `jxl_render` or `openjph_core` |
| `shasum` of `libjxl_{grid,render,modular}-*.rlib` in `target/codec-wasm-plain` vs `target/codec-wasm-simd` | worktree build products from 2026-11-05 | prior artefacts, hashed now | byte-identical, with the same metadata hash in both directories |
| `ls target/wasm32-unknown-unknown/debug/.fingerprint` | worktree | prior artefacts | `jxl-oxide`, `jxl-grid`, `jxl-oxide-common`, `tracing-core` present, i.e. the closure checked for wasm32 in the dev profile |
| `cargo tree` reverse and forward queries listed above | HEAD | now | as stated |
| `bin/ocelli.sh gate guards` | HEAD | now | exit 0, 431 refusal probes, 37 guards, 0 known defects, ALL GREEN |
| `bin/ocelli.sh gate unsafe` / `provenance` / `prose` | HEAD | now | exit 0, ALL GREEN each |
| `python3 scripts/guard_census.py` and `--check-runbook` | HEAD | now | exit 0 both, 1108 refusals, 0 watched by nothing |
| `python3 scripts/source_provenance_check.py` / `prose_check.py` | HEAD | now | exit 0, 1710 files / 864 files |

## Defects

### D1, the R5 guard is blind after a `'"'` char literal, and five shipped-crate regions sit in that blind spot today

**Where**: `scripts/unsafe_allowlist_check.py:80` (`STRING`) and `:83-89` (`strip_noise`). The claims are the new comment at 71-79 and `docs/SOURCE-POLICY.md`'s "strips comments and string literals before matching the keyword, string continuations included".
**What**: `STRING` begins a match at any `"`. A char literal `'"'` or `'\"'`, a raw string holding a `"` or ending in `\`, and a line comment with an unpaired `"` each open a false string. That string runs to the next real literal's opening quote, and everything between is blanked as string. This is the pass 7 D2 mechanism with a different trigger. The fix closed the continuation trigger, and these triggers predate it and remain, unchanged by `re.S`.
**Why it is wrong**: HLD 27.2 R5 makes this the guard that keeps the unsafe audit surface to two files. A guard that goes green with an `unsafe` in shipped code is the microscope's class 4. The new sentence and comment present the stripping as complete.
**Evidence**: I planted an unsafe block at `crates/ocelli-dicom/src/dicomweb.rs:418`, inside `media_type_parameter`, between the `trim_matches('"')` calls at 412 and 425, in a full `git archive HEAD` copy. `--staged` printed OK with exit 0, while the control plant in `ocelli-pixel` was refused. The lexer comparison finds the same blind span in `ocelli-render/src/caps.rs` 2486-2487, `ocelli-wasm/src/canvas.rs` 558, `examples/example-server/src/reply.rs` 138-226, `series.rs` 208-325 and `examples/folder-viewer/src/preview.rs` 1334-1371. In the jxl closure no count is affected: my lexer and the scanner agree on all 207 files, so the 534 stands.
**Scope and regression**: Present on HEAD and before this branch. It is adjacent to the ruled fix, not introduced by it. The smallest meaningful regression is a test planting `unsafe` after `'"'` and after `r#"a " b"#`, which is red today. The fix is to lex char literals, raw strings and line comments in one left-to-right pass rather than three regex passes, with the tracked-count-unchanged check repeated. The lead decides whether that lands in F-229, since the ruling named only continuations, or goes to its own story with the SOURCE-POLICY sentence qualified meanwhile.

### D2, no build in the repository compiles the fifty `+simd128` constructs

**Where**: `docs/SOURCE-POLICY.md`, the new sentence "The closure's wasm32 compilations today are the verification examples of `bin/ocelli.sh native` steps 7 and 10, whose plain builds compile the 138 and whose `+simd128` builds the fifty more".
**What**: steps 7 and 10 (`bin/ocelli.sh:603-607`, `621-625`) pass `-C target-feature=+simd128` after `cargo rustc --`. Cargo hands those arguments to the final crate only, the example, and compiles every dependency without them. `jxl-grid`, `jxl-modular` and `jxl-render` are therefore compiled with `target_feature = "simd128"` false, and their 50 regions are absent from the "+simd128" build exactly as from the plain one.
**Why it is wrong**: AC25 requires the split by reachability. This sentence was added to answer pass 7's S1 by saying which builds compile what, and it names a build that does not compile the fifty. A reader auditing a SIMD wasm module would take this as the record that the fifty have been compiled and exercised. Pass 7 also accepted the earlier form of this claim ("which `bin/ocelli.sh native` builds beside the plain variant"), so it is a miss carried from pass 7, not new to GLM's round.
**Evidence**: a fresh `cargo rustc ... -v -- -C target-feature=+simd128` into a scratch target dir shows `simd128` on the `verify_htj2k` invocation only. The existing `libjxl_grid`, `libjxl_render` and `libjxl_modular` rlibs in `target/codec-wasm-plain` and `target/codec-wasm-simd` are byte-identical.
**Scope and regression**: Present on HEAD. The smallest fix is in the prose: say that no build compiles the fifty today, because the `+simd128` flag reaches only the example crate. The counts and the `+simd128 only` column are right as a description of a build that sets the feature for dependencies, for example through `RUSTFLAGS`. A related concern for the lead, not reproduced as a defect: D-22's F-X013 "+simd128" leg was built the same way. openjph-core has no `simd128` cfg, so no gated code is missed, but its own codegen in that build did not get the feature either.

### D3, the same sentence omits the closure's one routine wasm32 compilation, which counts 137, not 138

**Where**: the same sentence, "The closure's wasm32 compilations today are ...", and "the oracle and folder-viewer consumers compile it natively".
**What**: `bin/ocelli.sh native` step 2 (`bin/ocelli.sh:576-578`) runs `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`. The workspace includes `ocelli-codec`, `ocelli-folder-viewer` and `ocelli-oracle`, so every closure crate is compiled for wasm32 there. It runs on CI as "Cross-target proof" (`.github/workflows/ci.yml:73-74`), and it runs in the dev profile, where `debug_assertions` is on and `tracing-core` `metadata.rs:721` is compiled out. That build sees 137 of the plain constructs. The two consumers are also compiled for wasm32 in that step, not only natively.
**Why it is wrong**: the sentence is an exhaustive list, and that is false. Its purpose, per the pass 7 ruling, is to say where the closure is compiled. The only build that type-checks the closure for wasm32 on every CI run is the one it leaves out.
**Evidence**: `sed -n 576,578p bin/ocelli.sh`, the workspace `members` at `Cargo.toml:3`, and fingerprints for `jxl-oxide`, `jxl-grid`, `jxl-oxide-common` and `tracing-core` under `target/wasm32-unknown-unknown/debug/.fingerprint`.
**Scope and regression**: Present on HEAD. Add step 2 to the list, with its dev-profile count, or replace the list with the release builds alone and say so.

## Smells

None.

## Nitpicks

- N1. "the four `src/bin` files with their nine constructs": the closure has nine `.rs` files under `src/bin`, and four of them hold matches. "the four `src/bin` files that hold any" would be exact.
- N2 (pass 7 N1, carried). The `jxl-render` rows are still not in descending count order.

## Verified clean

- The scanner change is exactly `re.S` plus a comment. The comment's mechanism (failed match at the opening quote, re-pairing from the closing quote) is correct, as I traced by hand on the test input. The test is green now and red with `re.S` removed.
- All 375 tracked `.rs` files keep their count, and the openjph-core total stays 104 under both patterns. The allowlist check passes and goes red on a plant in a non-allowed file (D1 is the exception).
- The 20-package closure from base `4165d1e8`, 207 files, 70 rows, 534, every per-file count, and the 138/50/346 column sums, which a second lexer confirms.
- Every pass 7 D1 and D2 row correction, read at the cfg lines. Ten fresh reachability calls and two extras, all agreeing.
- The S2 base commit and the S1 `cargo tree` claims about `ocelli-wasm`, `ocelli-native` and the reverse tree, including `--all-features`.
- The tracing-dependency sentence: 12 of 13 `jxl-*` manifests name `tracing`, and `jxl-oxide-common` names only `jxl-bitstream`. The `yes, not shippable` cell. The SSE2 sentence was not re-traced beyond pass 7.
- The release profile of steps 7 and 10, so the plain release builds do compile the 138.
- The five-mixed-files sentence: `simd.rs`, `squeeze.rs`, `rct.rs`, `imp_std.rs` and `metadata.rs` are the only files whose rows split.
- Every gate and script named in the request, all exit 0.

## Limitations and handoff

- No GPU, oracle or browser job was run. No wasm module was disassembled. D2 rests on the rustc command lines and the identical rlibs, not on symbol inspection.
- `README.md`, `use.html` and `more.html` were not re-read, because the diff changes nothing they describe.
- The `jxl-color` per-match attribution used the nearest preceding cfg as a heuristic on one file. Pass 7 checked the whole package.
- I wrote the scratch build into the session scratchpad, not the worktree `target/`. The worktree is unchanged apart from this file: `git status --short` matches the start, plus this report, and HEAD is still 72824ed.
- Next owner: the implementer, for D2 and D3, which are prose. The lead decides D1's scope. The same D1 mechanism surviving one more trigger after pass 7 is worth knowing before ruling.
