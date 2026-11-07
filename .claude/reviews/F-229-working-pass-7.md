# F-229 review, pass 7

**Reviewed**: `git diff 2cf21f8 HEAD` on `work/f-229-xai`, HEAD `afc7232` ("F-229, the jxl-oxide unsafe audit"), one file, `docs/SOURCE-POLICY.md`, 184 insertions. Scope is acceptance row AC25 only. Untracked paths at start and end: `.claude/reviews/F-229-working-pass-1.md` to `-6.md` and `tools/oracle/out`, none touched by this pass apart from adding this file.
**Author / reviewer**: GLM-5.3 in opencode authored the commit. Claude (Opus) reviewed independently and did not author any F-229 material. GLM's evidence scripts under `/private/var/folders/j7/.../T/opencode/` and its log were read and not relied on. Every number below was re-derived.
**Result**: 2 defects, 2 smells, 4 nitpicks

## Acceptance challenged

AC25: a "jxl-oxide 0.12.6, the unsafe audit" section in the openjph-core shape, whole-word `unsafe` counted per file over every `src/` of the closure, split by wasm32 reachability, a malformed-input table from AC16, and extension rows for `jxl-oxide` with its subcrates and `brotli-decompressor`.

- **Package set.** Derived independently: `git archive 4165d1e8` (the merge base) of the manifests, `Cargo.lock`, `crates`, `tools/oracle`, `examples/*` and `vendor` into scratch, then `cargo tree -p ocelli-codec --prefix none --offline --locked` at the base export and at HEAD, for `-e normal`, and again for `-e normal,build` under `--target all` and `--target wasm32-unknown-unknown`. All three give the same 20 added packages, matching the section. `brotli-decompressor`, `alloc-no-stdlib`, `alloc-stdlib` and all 13 `jxl-*` are new to the lock. `tracing` 0.1.44, `tracing-core` 0.1.36, `once_cell` 1.21.4 and `pin-project-lite` 0.2.17 are in the base lock at the same versions.
- **Counting rule, stated.** The openjph-core rule: `scripts/unsafe_allowlist_check.py`'s `strip_noise` (block comments, then the `STRING` regex `"(?:[^"\\]|\\.)*"`, then line comments, each blanked) followed by `UNSAFE = (?<![\w])unsafe(?![\w])`, over every `*.rs` below `src/`. My own driver gives 207 files, 70 with a match, 532 total, and all 70 rows match the table exactly. A second, independent lexer (nested block comments, raw strings, char literals, multi-line strings) gives **534**, which is D2.
- **Reachability.** Re-decided against `cargo tree -p ocelli-codec -e features,normal` for wasm32 and aarch64-apple-darwin (identical feature sets) and a script that maps every match to the `#[cfg]` items enclosing it, then checked by hand against the module declarations. Calls checked, more than ten: every compiled-out row (brotli `ffi/*` behind `#[cfg(feature="ffi-api")] pub mod ffi`, `memory.rs` and `decode.rs` behind `feature="unsafe"`, plus brotli's `forbid(unsafe_code)` under this feature set, `alloc-stdlib/heap_alloc.rs`, the three `src/bin` files, once_cell `imp_pl.rs` and `imp_cs.rs`, every `jxl-color` file, every `jxl-render` x86_64/aarch64 file through `filter/impls.rs` and `vardct/mod.rs`, `tracing-core/metadata.rs`), every `+simd128` split (`jxl-grid/simd.rs` 15/35/15/15 at the cfg boundaries 106, 350, 446, `jxl-modular` `squeeze.rs` 10/24/6 and `rct.rs` 4/4/4 line by line, the four `jxl-render` wasm32 files), and the "yes" rows of `tracing-core`, `tracing`, `once_cell`, `jxl-grid`, `jxl-oxide/fb.rs`, `jxl-bitstream`, `jxl-coding`, `jxl-vardct`. The wrong behaviour this rejects: a construct called reachable that a cfg removes, or the reverse. It found both, D1.
- **Malformed-input table** against `crates/ocelli-codec/tests/jpegxl.rs` as it is: every row's test name, input description and asserted kind read against the test body. All 31 rows hold. The scan's 14 `Case`s, truncation `0..len`, 2,000 LCG flips from seed `0x2290_f229`, the two sentinel buffers (`0xa5`, `0x5a`), `Err` with `out` at `0xa5`, and the collected panic sites are as stated. The test file was run, 29 passed including `malformed_jpegxl_never_panics` (156 s, debug).
- **Extension rows.** Licence fields and packaged licence files read from each package: 13 `jxl-*` with `MIT OR Apache-2.0` and both licence files, so "twelve subcrates" is right. `brotli-decompressor` `BSD-3-Clause/MIT` with `LICENSE`. `alloc-no-stdlib` and `alloc-stdlib` `BSD-3-Clause`, only `alloc-no-stdlib` ships `LICENSE`. Five cells, same column order as the table above. The only script that parses this table, `scripts/guards/catalogue.py:_blocked_project`, reads `cells[3] == "**NO**"`, and the new rows carry `yes` there, so the probe's target is unchanged.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo tree -p ocelli-codec` base vs HEAD, three edge/target variants, `comm -13` | base export of `4165d1e8`, HEAD `afc7232` | now | 20 added, identical in all three |
| scanner-rule count, own driver importing `strip_noise` and `UNSAFE` | registry `index.crates.io-1949cf8c6b5b557f`, locked versions | now | 207 files, 70, 532, 0 row mismatches |
| independent lexer count | same | now | 534. Differs in `tracing-core` `callsite.rs` 1 vs 2 and `metadata.rs` 1 vs 2 |
| scanner with `STRING` compiled `re.S` | same, plus `vendor/openjph-core-0.1.0`, plus all 375 tracked `.rs` | now | 534 here, 104 for openjph, 0 tracked files change |
| cfg enclosure map per match (`gates.py`) | `once_cell`, `tracing-core`, `tracing`, `pin-project-lite`, `alloc-*`, `brotli`, all `jxl-*` | now | `imp_std.rs` 8 ungated + 2 `#[cfg(test)]`, `metadata.rs` 1 `not(debug_assertions)` + 1 `test`, spin and lazy gated at the module |
| `cargo tree -e features,normal` wasm32 and aarch64 | HEAD | now | as the section states. `tracing-core` has `std` |
| `cargo tree -p ocelli-wasm` / `-p ocelli-native` for `ocelli-codec`, `jxl-oxide`; `cargo tree -i ocelli-codec` | HEAD, `--target all` | now | 0 and 0. Reverse: `ocelli-folder-viewer`, `ocelli-oracle` |
| `cargo test -p ocelli-codec --test jpegxl` | HEAD | now | exit 0, 29 passed |
| `python3 scripts/unsafe_allowlist_check.py` | HEAD | now | exit 0, openjph 104, ritk 0, "263 files checked, 2 permitted" |
| `git diff 4165d1e HEAD --stat -- scripts/` | | now | allowlist script untouched |
| `bin/ocelli.sh gate provenance` | HEAD | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate prose` | HEAD | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate licences` | HEAD | now | exit 0, 72 tests OK, ALL GREEN |
| `python3 scripts/source_provenance_check.py` | HEAD | now | exit 0, clean over 1710 files |
| `python3 scripts/prose_check.py` | HEAD | now | exit 0, clean over 864 files |
| `python3 scripts/guard_census.py` | HEAD | now | exit 0, 1108 refusals, 0 watched by nothing |
| GLM log tail and `count_unsafe.py`, `buckets.py` | scratchpad log, opencode temp dir | prior, read only | `buckets.py` `ALWAYS_YES` lists `spin/mutex.rs`, `spin/once.rs`, `lazy.rs` and `imp_std.rs` whole, the source of D1 |

## Defects

### D1, fourteen constructs called reachable on wasm32 are compiled out

**Where**: `docs/SOURCE-POLICY.md`, the table rows `once_cell 1.21.4 | src/imp_std.rs | 10 | yes, std on and parking_lot off`, `tracing-core 0.1.36 | src/spin/once.rs | 7 | yes`, `src/spin/mutex.rs | 4 | yes`, `src/lazy.rs | 1 | yes`, and the prose "150 of the 532", "The other 332", the enumeration of what is compiled out, "No file was left undecided ... a region whose attribute this paragraph quotes", and "The three mixed files".
**What**: `tracing-core-0.1.36/src/lib.rs:273` declares `#[cfg(not(feature = "std"))] pub(crate) mod spin;`, and `lazy.rs:7` puts its one `unsafe impl` (line 37) in `#[cfg(not(feature = "std"))] mod spin`. The section itself says `tracing-core` resolves with `std`, so all 12 are compiled out. `once_cell-1.21.4/src/imp_std.rs:290` opens `#[cfg(test)] mod tests`, which runs to the end of the file and holds the matches at 326 and 337, so 8 of that file's 10 compile. Under the stated rule the split is **136 / 50 / 346**, not 150 / 50 / 332. `imp_std.rs` is a fourth mixed file. The compiled-out enumeration omits `tracing-core`'s no-std spin and lazy code, `once_cell`'s test module, and the 9 constructs in the three `src/bin` files.
**Why it is wrong**: AC25 requires the split by reachability. The section claims the method was "reading every `#[cfg(...)]` attribute on the file and on the module declaration that pulls it in", and the missed gate is exactly a module declaration. A false statement in a record is a defect under the microscope's claim rule.
**Evidence**: `sed -n 268,293p .../tracing-core-0.1.36/src/lib.rs`, `sed -n 1,12p .../lazy.rs`, `sed -n 286,296p .../once_cell-1.21.4/src/imp_std.rs` plus `grep -n -w unsafe`. The table sums to 150/50/332 by its own column, and moving 14 gives 136/50/346.
**Scope and regression**: Present on HEAD. The audit is the only record of this closure, because the per-package ratchet does not see registry packages. Smallest regression: correct the four rows and the three totals, and either name the `not(feature = "std")` and `#[cfg(test)]` gates in the prose or drop the "no file was left undecided" sentence.

### D2, the counted total hides two real `unsafe` blocks, one in every release build

**Where**: `docs/SOURCE-POLICY.md`, "532 `unsafe` constructs sit in 70 of them", the claim that the scanner "strips comments and string literals before matching", and the rows `tracing-core | src/callsite.rs | 1` and `src/metadata.rs | 1 | no, #[cfg(test)] module`. Root cause `scripts/unsafe_allowlist_check.py:71`, not in this diff.
**What**: `STRING` is compiled without `re.S`, so `\\.` cannot match a backslash before a newline, which is a Rust string continuation. The regex stops pairing quotes correctly and blanks code as though it were a string literal: `tracing-core` `metadata.rs` lines 717 to 760 and `callsite.rs` lines 448 to 515. That hides `callsite.rs:469` `while let Some(cs) = unsafe { head.as_ref() }`, which is ungated and reachable, and `metadata.rs:721` `_ => unsafe {`, which is `unreachable_unchecked` under `#[cfg(not(debug_assertions))]`. `[profile.release]` does not turn on debug assertions, so that block is in every release build, the wasm32 ones included. The package holds 534 constructs, not 532. The `metadata.rs` row reports the `#[cfg(test)]` match and says nothing of the release-path block.
**Why it is wrong**: AC25's subject is "whole-word `unsafe` constructs". The audit exists so that nobody adds a dependency without reading what it does unsafely. The section states as fact a total and a per-file count that are false, and one hidden block is reachable.
**Evidence**: the independent lexer gives 534. The same scanner with `re.S` gives 534 here, still 104 on `vendor/openjph-core-0.1.0` and changes 0 of 375 tracked `.rs` files, so openjph and the repository's own allowlist are unaffected today. The span-finding script printed both spans.
**Scope and regression**: Present on HEAD. With D1 the correct split is 138 / 50 / 346 of 534. Smallest regression is in the section: count the two and say the scanner misses continuation lines, or quote the corrected rule. The scanner fix itself is outside AC25. The lead has to decide whether it lands in F-229 or gets its own story, because changing it moves a ratchet input.

## Smells

### S1, the only mention of a shipped module invites the reading that it carries this closure

**Where**: "which the shipped `bin/ocelli.sh wasm` module does not enable and which `bin/ocelli.sh native` builds beside the plain variant".
**Mechanism**: `cargo tree -p ocelli-wasm` and `-p ocelli-native` reach neither `ocelli-codec` nor `jxl-oxide`, and D-28 says so. The section never says it. Its one shipped-module sentence reads as "the shipped module has the 150 plain constructs and not the 50". In fact the plain wasm32 builds that compile this closure are `bin/ocelli.sh native` steps 7 and 10, plus the `ocelli-oracle` and `ocelli-folder-viewer` consumers. The hazard: when a later story routes `ocelli-codec` into `ocelli-wasm`, this record already reads as the shipped audit, so nothing prompts a re-read. One sentence naming which builds compile the closure removes it.

### S2, the scope definition stops reproducing once the story integrates

**Where**: "against the base `git merge-base HEAD sprint/s46`".
**Mechanism**: after `/integrate-feature` merges `work/f-229-xai` into `sprint/s46`, that merge base is HEAD, and the stated derivation prints zero added packages. A reader on `main` cannot re-run the audit's scope. Name the commit (`4165d1e8`) or the closure as it stood before F-229. The openjph section avoids this because it names a package, not a diff.

## Nitpicks

- N1. Within `jxl-render` the table is not in descending count order (`vardct/aarch64/mod.rs` 2 and `vardct/x86_64/mod.rs` 2 come after rows of 1). The openjph table is in descending order.
- N2. `alloc-stdlib` ships no licence file and its row reads a plain `yes`. The openjph precedent for the same situation was "yes, not shippable" until a notice was obtained. Today nothing shipped reaches it, and `scripts/licence_check.py` walks the shipped Rust closure, so it would refuse when that changes. Not blocking.
- N3. "because `jxl-bitstream` depends on tracing": `cargo tree -i tracing` shows every `jxl-*` crate depends on it directly. True as written, but incomplete.
- N4. The `jxl-color` sentence puts all of its unsafe "behind runtime ... dispatch". Some x86_64 blocks are the SSE2 baseline arm with no detection (`convert/gamut_map.rs:29`, `tf/pq.rs:101`, "x86_64 implies SSE2"). The cfg claim, that none of it is compiled on wasm32, holds.

## Verified clean

- The 20-package set, its versions, and the lock-membership sentence, on three edge/target variants.
- All 70 per-file counts under the stated rule, the 207-file total and the 70-file count.
- Every compiled-out row except the ones in D1/D2: brotli's six rows (ffi module gate, `unsafe` feature gates, `forbid(unsafe_code)`, bin target), `alloc-stdlib`'s two, `alloc-no-stdlib`'s bins, once_cell `imp_pl.rs`/`imp_cs.rs` by the `#[path]` selection at `lib.rs:380-390`, all 12 `jxl-color` files, all x86_64/aarch64 `jxl-render` files through the module gates.
- Every `+simd128` call: `jxl-grid/simd.rs` 15/35/15/15, `squeeze.rs` 10/24/6, `rct.rs` 4/4/4, and the four `jxl-render` wasm32 files gated `all(target_family = "wasm", target_feature = "simd128")`. Total 50.
- The "yes" rows for `jxl-grid` subgrids, `jxl-oxide/fb.rs` (both matches before the `#[cfg(test)]` at 187), `jxl-bitstream`, `jxl-coding`, `jxl-vardct`, `jxl-render` shared filter and VarDCT helpers and generic arms (ungated `mod generic`), `once_cell` `lib.rs` and `race.rs` (`std`, `alloc` on), `tracing` three files, `tracing-core` `subscriber.rs`/`dispatcher.rs`/`callsite.rs`, `pin-project-lite` as macro bodies.
- The feature-resolution sentence, the `rayon` sentence (`jxl-oxide` `default = ["rayon"]`, off), the `bin/ocelli.sh native` `+simd128` builds (steps 7 and 10), the pins (`Cargo.toml:176-186`), the no-Rayon check in `scripts/pin_and_size_check.py:360`, the allowlist sentence (script unchanged since the base, ratchet records exactly ritk and openjph).
- The malformed-input paragraph and all 31 rows against the test file, and the test file passing now.
- Extension row content and format, and the only parser of that table.
- Shape: the section follows the openjph one (counting sentence, File/Constructs/Reachable table with an added Package column, bold reachable summary, the "Ocelli adds no unsafe" paragraph), plus the malformed table AC25 adds. It matches.
- No em-dash, en-dash or prose semicolon in the added text (prose gate and `prose_check.py`).

## Limitations and handoff

- No GPU, oracle or browser job was run, as instructed. No wasm build was run, so reachability is decided from cfg and features, not from symbols in a built module.
- `pin-project-lite`'s 9 are macro-body tokens. Whether every one is expanded in this closure was not traced.
- GLM's log says brotli's "`mod memory` [is] behind unsafe+std (off)". It is not: `lib.rs:32-33` declares it ungated, and its macros are gated individually. The table row's reason ("`unsafe` feature off") is right, so this is a log error only.
- Next owner: the implementer, for D1, D2, S1, S2. The lead decides whether the `STRING` regex fix in `scripts/unsafe_allowlist_check.py` is in F-229's scope. AC25 stays unmet until a fresh pass is clean.
