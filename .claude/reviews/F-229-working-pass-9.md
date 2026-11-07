# F-229 review, pass 9

**Reviewed**: `git diff 72824ed 3f8fba7` on `work/f-229-xai`, HEAD `3f8fba75d7ec53cb7086c1c3e2cb48da3dc1e730` ("F-229, the build sentence removed and the scanner gaps ruled out of scope"), `git write-tree` `4d3b01e01364eb07e27aca002c95588d208b42c5`. Two files: `.claude/plans/F-229-design.md` (the pass 8 lead rulings) and `docs/SOURCE-POLICY.md` (one sentence deleted). The operator record was read at `sprint/s46:docs/sprints/CURRENT_SPRINT.md` (`cd04300`). Scope is AC25 and the scanner. Untracked paths at start and end: `.claude/reviews/F-229-working-pass-1.md` to `-8.md` and `tools/oracle/out`. This pass added only this file.
**Author / reviewer**: the sprint lead (Claude) authored 3f8fba7. Claude (Opus) reviewed independently, as a separate agent that authored no F-229 material.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 8 D2 and D3 against the deletion.** Both findings were about one sentence, "The closure's wasm32 compilations today are ...". `grep -n 'compilations today\|steps 7 and 10\|fifty more'` over `docs/SOURCE-POLICY.md` finds nothing, so the sentence is gone and the text is not duplicated anywhere. D2 said no build compiles the fifty `+simd128` constructs. D3 said the list was presented as exhaustive and left out step 2. With the sentence deleted, the section no longer names any build that compiles the closure for wasm32, so neither claim survives. **Both are closed.**
- **The ruling's own facts.** "no build passes `+simd128` to the closure's crates": `simd128` appears in `bin/ocelli.sh` only at 607 and 625, after `cargo rustc ... --`, there is no `.cargo/` directory, and nothing in `bin/`, `.github/` or `scripts/` sets `RUSTFLAGS` or `target-feature`. Pass 8 already showed that the trailing arguments reach only the example crate. "step 2 of `bin/ocelli.sh native` also compiles it": `bin/ocelli.sh:576-577` runs `cargo check --workspace --exclude ocelli-native --target wasm32-unknown-unknown`, and `Cargo.toml:3` puts `crates/*`, `tools/oracle` and `examples/folder-viewer` in the workspace. "predate this story and change no audit count" agrees with pass 8's lexer comparison, 534 and 104 under both.
- **The paragraph around the cut.** It now reads: 138 plain, fifty more "only when the build enables `+simd128`", the shipped wasm module carrying none of this because neither `ocelli-wasm` nor `ocelli-native` reaches `ocelli-codec`, the reverse tree, then "The other **346**". 138 + 50 + 346 = 534. "The other" still refers to the 138 and the fifty, with one sentence about shipped modules in between, and it still reads correctly. The `+simd128` sentence is conditional and names no build, so after the deletion it no longer implies that any build compiles the fifty. The rest of the section is unchanged since pass 8, which executed it, and I did not recount it.
- **D1, the ruling and the completeness question.** The ruling is in `.claude/plans/F-229-design.md` "Lead rulings after review pass 8", and the operator record is `CURRENT_SPRINT.md` lines 153-159 on `sprint/s46`, "The unsafe guard has blind spots", which leaves the operator a choice between a lexer story and a recorded limit. I read every sentence in the jxl-oxide audit section, the openjph-core audit section, the rest of `SOURCE-POLICY.md` that mentions `unsafe`, and the scanner's comments, looking for a claim that the guard is complete. The only candidate is the method sentence at `SOURCE-POLICY.md:315-317`, "which strips comments and string literals before matching the keyword, string continuations included". It says what the scanner does. It does not say the scanner strips only those, or that the guard catches every `unsafe`. The counts it produced were corroborated by an independent lexer in pass 8, 534 for the closure and 104 for openjph-core, so as a description of how the audited numbers were made it is true. The openjph sentence at 271-272 is the same description, and its "qualified by a number the build holds" refers to the recorded per-package count. The scanner's comment at 71-79 describes only the continuation mechanism. The docstring at line 61 states intent and predates this branch. **No sentence claims the guard is complete.**
- **Discriminating replay of the out-of-scope triggers**, run only to check that the record matches them. Each case has one `unsafe` block, and a control case without a trigger finds it (1). The scanner finds 0 after `'"'`, after `'\"'`, after `r"a\"`, after `r#"a " b"#` and after `// say "`. That is five triggers. The record names three, see N2.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 72824ed 3f8fba7` | HEAD 3f8fba7 | now | 2 files, +5 -4, as described above |
| `grep` for the deleted sentence's phrases | `docs/SOURCE-POLICY.md` at HEAD | now | no match |
| `grep -rn simd128\|RUSTFLAGS\|target-feature` over `bin .github scripts Cargo.toml crates/*/Cargo.toml`, `ls .cargo` | HEAD | now | only `bin/ocelli.sh:607,625` after `--`. No `.cargo` |
| `sed -n 570,630p bin/ocelli.sh`, `Cargo.toml:3` | HEAD | now | step 2 is a workspace wasm32 `cargo check`. Steps 7 and 10 are as pass 8 read them |
| `tracing-core-0.1.36/src/metadata.rs:720-721` | registry source | now | `#[cfg(not(debug_assertions))]` over `_ => unsafe {`, see N1 |
| `git show sprint/s46:docs/sprints/CURRENT_SPRINT.md` | `cd04300` | now | both pass 8 items recorded at 153-164 |
| `strip_noise` replay, five triggers plus a control | the worktree module, imported | now | 0,0,0,0,0, control 1 |
| `python3 -B -m unittest discover -s scripts/tests -p test_unsafe_allowlist_check.py` | HEAD | now | exit 0, 6 tests OK |
| `python3 scripts/unsafe_allowlist_check.py` and `--staged` | HEAD | now | exit 0, 263 files, 2 permitted, openjph-core 104, ritk 0 |
| `bin/ocelli.sh gate unsafe` / `provenance` / `prose` | HEAD | now | exit 0 each, ALL GREEN |
| `python3 scripts/guard_census.py` | HEAD | now | exit 0, 1108 refusals, 0 watched by nothing |
| `python3 scripts/prose_check.py` | HEAD | now | exit 0, 864 files |
| `python3 scripts/source_provenance_check.py` | HEAD | now | exit 0, 1710 files |
| pass 8's closure recount, lexer comparison and `cargo rustc -v` | 72824ed | prior receipt, `F-229-working-pass-8.md` | relied on for the unchanged sentences and the counts |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1. "138 of the 534 compile into a plain wasm32 build" does not say "release".** Pass 8 measured that step 2, the closure's routine wasm32 build, is a dev-profile check, and in that profile `debug_assertions` removes `metadata.rs:721`, so it compiles 137. The table cell "1 yes in release" states that, so the prose and the table agree when read together, and 138 is the upper bound in the safe direction. Writing "a plain wasm32 release build" would make the bold sentence hold on its own. This sits next to the deleted sentence and is not new.
- **N2. The ruling and the operator record name three of the five triggers pass 8 measured.** The record has "a `'"'` character literal, a raw string holding a quote and a line comment with an unpaired quote". `'\"'` is covered by "a character literal" if that is read loosely, but `r"a\"`, a raw string that ends in a backslash and holds no quote, is not covered by any of the three, and it still hides the code after it (replayed above, 0 found). A lexer story would close it anyway. Naming it gives the story's regression test the full set. It does not touch the audit section or any count.
- **N3. The ruling says the blind spots "go to their own story with a real lexer", but the operator record leaves a choice** between that story and "a recorded limit on the guard". The record is the authority and the ruling reads as if the choice is already made. The wording fits a lead recommendation, not a decision.
- Carried from pass 8, not re-raised: N1 ("the four `src/bin` files with their nine constructs") and N2 (the `jxl-render` row order).

## Verified clean

- The deletion is complete, it leaves no duplicate or dangling reference, and the paragraph around it still adds up (138 + 50 + 346 = 534) and reads correctly.
- Every fact in the pass 8 ruling: no `+simd128` reaches a dependency, step 2 compiles the closure for wasm32, and the blind spots predate the story and change no count.
- Pass 8 D2 and D3 are closed. D1 is ruled out of scope and recorded for the operator on `sprint/s46`.
- No sentence in the audit section, elsewhere in `SOURCE-POLICY.md` or in the scanner's comments claims the guard is complete. The method sentence is true as a description of how the audited counts were made, and an independent lexer corroborated those counts.
- The scanner and its test are unchanged in this diff. The test is green, the guard is green, and the five triggers still behave as pass 8 reported, which is the expected state for an out-of-scope gap.
- Every requested gate and script, all exit 0.
- New prose in both files is clean under `prose_check.py`.

## Limitations and handoff

- No GPU, oracle, browser or wasm build was run. The `+simd128` reach rests on pass 8's `cargo rustc -v` receipt plus this pass's grep. I did not rebuild it.
- The 534, 207, 70 and 138/50/346 counts and the per-file reachability calls were not recounted. The request said they need not be, and pass 8 did it at 72824ed. This diff does not change the scanner or the table.
- `README.md`, `use.html` and `more.html` were not re-read, because the diff changes nothing they describe.
- The D-22 F-X013 `+simd128` concern and the guard's blind spots are operator items on `sprint/s46`. They are not F-229 findings.
- The worktree is unchanged except for this file. HEAD is still 3f8fba7.
- This is a clean bounded review of AC25 and the scanner change only. It does not mean F-229's other ACs are accepted or that the sprint is ready.
